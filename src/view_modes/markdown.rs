use crate::app::state::TabState;
use octa::data::MarkdownLayout;

use eframe::egui;
use egui::RichText;

/// Cheap content hash so we can invalidate `tab.markdown_render_cache` only
/// when the buffer actually changes. Uses `DefaultHasher` for simplicity —
/// collisions are harmless (worst case is one stale render before the next
/// keystroke triggers another rebuild).
fn content_hash(s: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

/// Render the Markdown view with a Preview / Split / Edit segmented toggle.
/// In Split mode the left pane is a TextEdit bound to `tab.raw_content`;
/// edits are reflected in the right-pane preview every frame.
pub fn render_markdown_view(ui: &mut egui::Ui, tab: &mut TabState, readonly: bool) {
    let Some(content_owned) = tab.raw_content.clone() else {
        ui.centered_and_justified(|ui| {
            ui.label(
                RichText::new("Markdown content not available")
                    .size(16.0)
                    .color(ui.visuals().weak_text_color()),
            );
        });
        return;
    };

    // Layout toggle bar (Preview / Split / Edit).
    ui.horizontal(|ui| {
        let mut layout = tab.markdown_layout;
        ui.label(RichText::new("Layout:").size(12.0));
        ui.selectable_value(&mut layout, MarkdownLayout::Preview, "\u{1f441} Preview");
        ui.selectable_value(&mut layout, MarkdownLayout::Split, "\u{21cb} Split");
        ui.selectable_value(&mut layout, MarkdownLayout::Edit, "\u{270e} Edit");
        if layout != tab.markdown_layout {
            tab.markdown_layout = layout;
        }
    });
    ui.add_space(4.0);

    match tab.markdown_layout {
        MarkdownLayout::Preview => {
            render_preview_pane(ui, tab, &content_owned);
        }
        MarkdownLayout::Edit => {
            render_editor_pane(ui, tab, readonly, ui.available_width());
        }
        MarkdownLayout::Split => {
            // 50/50 split. The left SidePanel hosts the editor; the central
            // area receives the rendered preview.
            let editor_width = (ui.available_width() * 0.5).max(200.0);
            egui::SidePanel::left("md_editor_pane")
                .resizable(true)
                .min_width(150.0)
                .default_width(editor_width)
                .show_inside(ui, |ui| {
                    render_editor_pane(ui, tab, readonly, ui.available_width());
                });
            render_preview_pane(ui, tab, &tab.raw_content.clone().unwrap_or_default());
        }
    }
}

fn render_editor_pane(ui: &mut egui::Ui, tab: &mut TabState, readonly: bool, _width: f32) {
    let Some(buffer) = tab.raw_content.as_mut() else {
        return;
    };
    // `desired_width(f32::INFINITY)` disables auto-wrap so long lines extend
    // beyond the visible pane; the surrounding `ScrollArea::both` then
    // provides horizontal scrolling instead of clipping or word-wrapping.
    let response = egui::ScrollArea::both()
        .id_salt("markdown_editor_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(buffer)
                    .id(egui::Id::new("markdown_editor"))
                    .font(egui::FontId::new(13.0, egui::FontFamily::Monospace))
                    .desired_width(f32::INFINITY)
                    .desired_rows(20)
                    .interactive(!readonly),
            )
        })
        .inner;
    if response.changed() && !readonly {
        tab.raw_content_modified = true;
        // Invalidate the pre-render cache so the preview pane re-renders
        // with the latest content. Also drop stale link hooks the
        // commonmark cache may have registered for fragment links the
        // user just deleted.
        tab.markdown_render_cache = None;
        tab.commonmark_cache.link_hooks_mut().clear();
    }
}

fn render_preview_pane(ui: &mut egui::Ui, tab: &mut TabState, raw_content: &str) {
    // CRLF normalization for consistent line handling — pulldown_cmark
    // accepts both, but `\r`-only line endings interact poorly with our
    // event-driven renderer's break heuristics.
    let raw_normalized = if raw_content.contains('\r') {
        raw_content.replace("\r\n", "\n").replace('\r', "\n")
    } else {
        raw_content.to_string()
    };
    let hash = content_hash(&raw_normalized);
    if !matches!(&tab.markdown_render_cache, Some((h, _)) if *h == hash) {
        // Even though we no longer pre-render to HTML-translated CommonMark,
        // keep the cache pointer fresh so other code (e.g. raw editor change
        // handler) can still invalidate it.
        tab.markdown_render_cache = Some((hash, raw_normalized.clone()));
    }

    let bg_response = ui.interact(
        ui.available_rect_before_wrap(),
        ui.id().with("markdown_bg"),
        egui::Sense::click(),
    );
    let raw_for_copy = raw_content.to_string();
    bg_response.context_menu(|ui| {
        if ui.button("Copy Markdown").clicked() {
            ui.ctx().copy_text(raw_for_copy.clone());
            ui.close_menu();
        }
    });

    let pending_offset = tab.markdown_scroll_target.take();
    let mut scroll_area = egui::ScrollArea::both()
        .id_salt("markdown_scroll")
        .auto_shrink([false, false]);
    if let Some(offset) = pending_offset {
        scroll_area = scroll_area.vertical_scroll_offset(offset);
    }
    scroll_area.show(ui, |ui| {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            ui.vertical(|ui| {
                let cap = ui.available_width().clamp(200.0, 900.0);
                ui.set_max_width(cap);
                render_pulldown(ui, &raw_normalized);
            });
        });
    });
}

/// Custom markdown renderer using `pulldown_cmark` for parsing. Replaces
/// `egui_commonmark` so that `**bold**` runs use a real bold font face
/// (registered as `FontFamily::Name("bold")` in `apply_fonts`) instead of
/// egui's color-only `RichText::strong()` which produced visually
/// indistinguishable output.
fn render_pulldown(ui: &mut egui::Ui, src: &str) {
    use pulldown_cmark::{Event, Parser, Tag, TagEnd};

    let parser = Parser::new(src);
    let body_size = 13.0;
    let mut state = InlineState::default();

    // Buffer pending inline runs for the current block. Flushed when the
    // block closes (paragraph/heading/list-item end).
    let mut buf: Vec<(String, RunStyle)> = Vec::new();
    let mut block_kind = BlockKind::Paragraph;
    let mut list_stack: Vec<ListInfo> = Vec::new();
    let mut code_block_buf = String::new();

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => {
                    block_kind = BlockKind::Paragraph;
                }
                Tag::Heading { level, .. } => {
                    block_kind = BlockKind::Heading(heading_level_u8(level));
                }
                Tag::BlockQuote(_) => {
                    block_kind = BlockKind::Quote;
                }
                Tag::CodeBlock(_) => {
                    block_kind = BlockKind::CodeBlock;
                    code_block_buf.clear();
                }
                Tag::List(start) => {
                    list_stack.push(ListInfo {
                        ordered: start.is_some(),
                        next_num: start.unwrap_or(1),
                    });
                }
                Tag::Item => {
                    block_kind = BlockKind::ListItem;
                }
                Tag::Emphasis => state.italic = true,
                Tag::Strong => state.strong = true,
                Tag::Strikethrough => state.strikethrough = true,
                Tag::Link { dest_url, .. } => {
                    state.link = Some(dest_url.to_string());
                }
                _ => {}
            },
            Event::End(end) => match end {
                TagEnd::Paragraph => {
                    flush_block(ui, &mut buf, block_kind, &list_stack, body_size);
                    ui.add_space(6.0);
                }
                TagEnd::Heading(_) => {
                    flush_block(ui, &mut buf, block_kind, &list_stack, body_size);
                    ui.add_space(8.0);
                    block_kind = BlockKind::Paragraph;
                }
                TagEnd::BlockQuote(_) => {
                    flush_block(ui, &mut buf, block_kind, &list_stack, body_size);
                    ui.add_space(4.0);
                    block_kind = BlockKind::Paragraph;
                }
                TagEnd::CodeBlock => {
                    render_code_block(ui, &code_block_buf, body_size);
                    code_block_buf.clear();
                    ui.add_space(6.0);
                    block_kind = BlockKind::Paragraph;
                }
                TagEnd::List(_) => {
                    list_stack.pop();
                }
                TagEnd::Item => {
                    flush_block(ui, &mut buf, BlockKind::ListItem, &list_stack, body_size);
                    if let Some(top) = list_stack.last_mut() {
                        top.next_num += 1;
                    }
                }
                TagEnd::Emphasis => state.italic = false,
                TagEnd::Strong => state.strong = false,
                TagEnd::Strikethrough => state.strikethrough = false,
                TagEnd::Link => state.link = None,
                _ => {}
            },
            Event::Text(text) => {
                if matches!(block_kind, BlockKind::CodeBlock) {
                    code_block_buf.push_str(&text);
                } else {
                    buf.push((text.into_string(), state.style()));
                }
            }
            Event::Code(text) => {
                let mut s = state.style();
                s.code = true;
                buf.push((text.into_string(), s));
            }
            Event::SoftBreak => {
                buf.push((" ".to_string(), state.style()));
            }
            Event::HardBreak => {
                buf.push(("\n".to_string(), state.style()));
            }
            Event::Rule => {
                flush_block(ui, &mut buf, block_kind, &list_stack, body_size);
                ui.separator();
                ui.add_space(4.0);
            }
            _ => {}
        }
    }
    flush_block(ui, &mut buf, block_kind, &list_stack, body_size);
}

#[derive(Default, Clone)]
struct InlineState {
    italic: bool,
    strong: bool,
    strikethrough: bool,
    link: Option<String>,
}

impl InlineState {
    fn style(&self) -> RunStyle {
        RunStyle {
            italic: self.italic,
            strong: self.strong,
            strikethrough: self.strikethrough,
            code: false,
            link: self.link.clone(),
        }
    }
}

#[derive(Default, Clone)]
struct RunStyle {
    italic: bool,
    strong: bool,
    strikethrough: bool,
    code: bool,
    link: Option<String>,
}

#[derive(Clone, Copy)]
enum BlockKind {
    Paragraph,
    Heading(u8),
    Quote,
    CodeBlock,
    ListItem,
}

struct ListInfo {
    ordered: bool,
    next_num: u64,
}

fn heading_level_u8(level: pulldown_cmark::HeadingLevel) -> u8 {
    use pulldown_cmark::HeadingLevel as H;
    match level {
        H::H1 => 1,
        H::H2 => 2,
        H::H3 => 3,
        H::H4 => 4,
        H::H5 => 5,
        H::H6 => 6,
    }
}

fn flush_block(
    ui: &mut egui::Ui,
    buf: &mut Vec<(String, RunStyle)>,
    kind: BlockKind,
    list_stack: &[ListInfo],
    body_size: f32,
) {
    if buf.is_empty() {
        return;
    }
    let runs = std::mem::take(buf);

    match kind {
        BlockKind::Heading(level) => {
            let size = match level {
                1 => body_size * 1.8,
                2 => body_size * 1.5,
                3 => body_size * 1.3,
                4 => body_size * 1.15,
                _ => body_size * 1.05,
            };
            render_runs(ui, &runs, size, /* heading */ true);
        }
        BlockKind::Paragraph => {
            render_runs(ui, &runs, body_size, false);
        }
        BlockKind::Quote => {
            ui.horizontal_wrapped(|ui| {
                ui.add_space(12.0);
                let muted = ui.visuals().weak_text_color();
                ui.label(RichText::new("\u{2503}").color(muted));
                ui.add_space(6.0);
                render_runs(ui, &runs, body_size, false);
            });
        }
        BlockKind::ListItem => {
            ui.horizontal_wrapped(|ui| {
                let depth = list_stack.len().saturating_sub(1);
                ui.add_space(8.0 + depth as f32 * 16.0);
                let bullet = match list_stack.last() {
                    Some(li) if li.ordered => format!("{}. ", li.next_num),
                    _ => "\u{2022} ".to_string(),
                };
                ui.label(
                    RichText::new(bullet)
                        .font(egui::FontId::new(body_size, egui::FontFamily::Proportional)),
                );
                render_runs(ui, &runs, body_size, false);
            });
        }
        BlockKind::CodeBlock => { /* handled separately */ }
    }
}

/// Build a `LayoutJob` from a list of styled runs and emit it as a wrapping
/// `Label`. Bold runs use the bundled `FontFamily::Name("bold")` family;
/// italics use egui's runtime skew; code uses Monospace + a tinted bg.
fn render_runs(ui: &mut egui::Ui, runs: &[(String, RunStyle)], size: f32, heading: bool) {
    use egui::text::{LayoutJob, TextFormat};
    let mut job = LayoutJob::default();
    job.wrap.max_width = ui.available_width();

    let body_color = ui.visuals().text_color();
    let link_color = ui.visuals().hyperlink_color;
    let bold_family = egui::FontFamily::Name(std::sync::Arc::from("bold"));

    for (text, style) in runs {
        let mut fmt = TextFormat::default();
        let want_bold = style.strong || heading;
        let family = if style.code {
            egui::FontFamily::Monospace
        } else if want_bold {
            bold_family.clone()
        } else {
            egui::FontFamily::Proportional
        };
        fmt.font_id = egui::FontId::new(size, family);
        fmt.color = if style.link.is_some() {
            link_color
        } else {
            body_color
        };
        fmt.italics = style.italic;
        if style.strikethrough {
            fmt.strikethrough = egui::Stroke::new(1.0, body_color);
        }
        if style.link.is_some() {
            fmt.underline = egui::Stroke::new(1.0, link_color);
        }
        if style.code {
            fmt.background = ui.visuals().code_bg_color;
        }
        job.append(text, 0.0, fmt);
    }

    ui.add(egui::Label::new(job).wrap());
}

fn render_code_block(ui: &mut egui::Ui, content: &str, size: f32) {
    let bg = ui.visuals().code_bg_color;
    let stroke = ui.visuals().widgets.noninteractive.bg_stroke;
    egui::Frame::new()
        .fill(bg)
        .stroke(stroke)
        .corner_radius(4.0)
        .inner_margin(egui::Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.add(
                egui::Label::new(
                    RichText::new(content.trim_end_matches('\n'))
                        .font(egui::FontId::new(size, egui::FontFamily::Monospace)),
                )
                .selectable(true),
            );
        });
}
