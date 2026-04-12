use crate::TabState;
use crate::ui;
use octa::data;

use eframe::egui;
use egui::{Align, Color32, Layout, RichText, Stroke};
use ui::theme::ThemeMode;

/// Render the PDF page view. Returns early if there are no textures.
pub fn render_pdf_view(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    tab: &mut TabState,
    theme_mode: ThemeMode,
) {
    // Lazily create textures from rendered images
    if tab.pdf_textures.len() != tab.pdf_page_images.len() {
        tab.pdf_textures.clear();
        for (i, image) in tab.pdf_page_images.iter().enumerate() {
            let texture = ctx.load_texture(
                format!("pdf_page_{}", i),
                image.clone(),
                egui::TextureOptions::LINEAR,
            );
            tab.pdf_textures.push(texture);
        }
    }

    if tab.pdf_textures.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label(
                RichText::new("No PDF pages to display")
                    .size(16.0)
                    .color(ui.visuals().weak_text_color()),
            );
        });
        return;
    }

    let colors = ui::theme::ThemeColors::for_mode(theme_mode);
    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                let page_count = tab.pdf_textures.len();
                for (page_idx, texture) in tab.pdf_textures.iter().enumerate() {
                    let size = texture.size_vec2();
                    let page_text = tab
                        .pdf_page_texts
                        .get(page_idx)
                        .cloned()
                        .unwrap_or_default();
                    // Page header
                    ui.label(
                        RichText::new(format!("Page {} of {}", page_idx + 1, page_count))
                            .size(11.0)
                            .color(colors.text_muted),
                    );
                    ui.add_space(4.0);
                    // Rendered page image
                    egui::Frame::new()
                        .fill(egui::Color32::WHITE)
                        .shadow(egui::epaint::Shadow {
                            offset: [2, 2],
                            blur: 8,
                            spread: 0,
                            color: colors.border.gamma_multiply(0.5),
                        })
                        .show(ui, |ui| {
                            ui.image(egui::load::SizedTexture::new(texture.id(), size));
                        });
                    // Selectable text below the page image
                    if !page_text.is_empty() {
                        ui.add_space(4.0);
                        egui::Frame::new()
                            .fill(colors.bg_secondary)
                            .stroke(Stroke::new(1.0, colors.border_subtle))
                            .corner_radius(4.0)
                            .inner_margin(8.0)
                            .show(ui, |ui| {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(&page_text)
                                            .font(egui::FontId::new(
                                                12.0,
                                                egui::FontFamily::Monospace,
                                            ))
                                            .color(colors.text_primary),
                                    )
                                    .selectable(true),
                                );
                            });
                    }
                    ui.add_space(16.0);
                    ui.separator();
                    ui.add_space(8.0);
                }
            });
        });
}

/// Render the Jupyter Notebook view. Handles Ctrl+X for copying all cells.
pub fn render_notebook_view(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    tab: &TabState,
    theme_mode: ThemeMode,
) {
    let colors = ui::theme::ThemeColors::for_mode(theme_mode);
    let is_dark = theme_mode == ThemeMode::Dark;

    if tab.table.row_count() == 0 {
        ui.centered_and_justified(|ui| {
            ui.label(
                RichText::new("Empty notebook")
                    .size(16.0)
                    .color(ui.visuals().weak_text_color()),
            );
        });
        return;
    }

    // Helper: build a LayoutJob for line number gutter (non-selectable)
    let build_line_numbers = |line_count: usize, line_num_color: Color32| {
        let mono = egui::FontId::new(13.0, egui::FontFamily::Monospace);
        let gutter_width = line_count.max(1).to_string().len();
        let mut job = egui::text::LayoutJob::default();
        for i in 0..line_count.max(1) {
            let num_str = format!("{:>width$}", i + 1, width = gutter_width);
            let suffix = if i + 1 < line_count.max(1) { "\n" } else { "" };
            job.append(
                &format!("{}{}", num_str, suffix),
                0.0,
                egui::text::TextFormat {
                    font_id: mono.clone(),
                    color: line_num_color,
                    ..Default::default()
                },
            );
        }
        job
    };

    // Collect all cell text for Ctrl+C on the whole notebook
    let mut all_notebook_text = String::new();

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.vertical(|ui| {
                    for row_idx in 0..tab.table.row_count() {
                        let cell_num = match tab.table.get(row_idx, 0) {
                            Some(data::CellValue::Int(n)) => Some(n),
                            _ => None,
                        };
                        let cell_type = match tab.table.get(row_idx, 1) {
                            Some(data::CellValue::String(s)) => s.clone(),
                            _ => "code".to_string(),
                        };
                        let source = match tab.table.get(row_idx, 2) {
                            Some(data::CellValue::String(s)) => s.clone(),
                            Some(v) => v.to_string(),
                            None => String::new(),
                        };
                        let output = match tab.table.get(row_idx, 3) {
                            Some(data::CellValue::String(s)) => s.clone(),
                            Some(v) => v.to_string(),
                            None => String::new(),
                        };

                        // Accumulate for whole-notebook copy
                        if !all_notebook_text.is_empty() {
                            all_notebook_text.push_str("\n\n");
                        }
                        all_notebook_text.push_str(&source);
                        if !output.is_empty() {
                            all_notebook_text.push('\n');
                            all_notebook_text.push_str(&output);
                        }

                        let is_code = cell_type == "code";
                        let is_markdown = cell_type == "markdown";

                        // Cell container
                        let cell_bg = if is_code {
                            if is_dark {
                                Color32::from_rgb(30, 34, 42)
                            } else {
                                Color32::from_rgb(248, 249, 250)
                            }
                        } else if is_dark {
                            Color32::from_rgb(35, 38, 45)
                        } else {
                            Color32::from_rgb(252, 252, 254)
                        };

                        let border_color = if is_code {
                            if is_dark {
                                Color32::from_rgb(60, 70, 90)
                            } else {
                                Color32::from_rgb(200, 210, 220)
                            }
                        } else {
                            colors.border_subtle
                        };

                        let line_num_color = if is_dark {
                            Color32::from_rgb(100, 110, 130)
                        } else {
                            Color32::from_rgb(150, 160, 175)
                        };

                        let text_color = if is_markdown {
                            colors.text_secondary
                        } else {
                            colors.text_primary
                        };

                        let line_count = source.lines().count();

                        // Cell label (e.g. "In [1]:" or nothing for markdown)
                        ui.horizontal(|ui| {
                            // Left label area
                            let label_width = 80.0;
                            ui.allocate_ui_with_layout(
                                egui::vec2(label_width, 20.0),
                                Layout::right_to_left(Align::TOP),
                                |ui| {
                                    if is_code {
                                        let label = if let Some(n) = cell_num {
                                            format!("In [{}]:", n)
                                        } else {
                                            "In [ ]:".to_string()
                                        };
                                        ui.label(
                                            RichText::new(label)
                                                .font(egui::FontId::new(
                                                    12.0,
                                                    egui::FontFamily::Monospace,
                                                ))
                                                .color(colors.accent),
                                        );
                                    }
                                },
                            );

                            // Cell content area with separate gutter + source
                            let frame_response = egui::Frame::new()
                                .fill(cell_bg)
                                .stroke(Stroke::new(1.0, border_color))
                                .corner_radius(4.0)
                                .inner_margin(8.0)
                                .show(ui, |ui| {
                                    ui.horizontal_top(|ui| {
                                        // Line number gutter (not selectable)
                                        let gutter_job =
                                            build_line_numbers(line_count, line_num_color);
                                        ui.add(egui::Label::new(gutter_job).selectable(false));
                                        ui.add_space(8.0);
                                        // Source text (selectable — no line numbers)
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(&source)
                                                    .font(egui::FontId::new(
                                                        13.0,
                                                        egui::FontFamily::Monospace,
                                                    ))
                                                    .color(text_color),
                                            )
                                            .selectable(true),
                                        );
                                    });
                                });
                            let copy_source = source.clone();
                            let all_text = all_notebook_text.clone();
                            frame_response.response.context_menu(|ui| {
                                if ui.button("Copy cell").clicked() {
                                    ui.ctx().copy_text(copy_source.clone());
                                    ui.close_menu();
                                }
                                if ui.button("Copy all cells").clicked() {
                                    ui.ctx().copy_text(all_text.clone());
                                    ui.close_menu();
                                }
                            });

                            // Output area (code cells only)
                            if is_code && !output.is_empty() {
                                let out_bg = if is_dark {
                                    Color32::from_rgb(25, 28, 35)
                                } else {
                                    Color32::from_rgb(255, 255, 255)
                                };
                                let out_frame = egui::Frame::new()
                                    .fill(out_bg)
                                    .stroke(Stroke::new(1.0, border_color))
                                    .corner_radius(4.0)
                                    .inner_margin(8.0)
                                    .show(ui, |ui| {
                                        let out_label = if let Some(n) = cell_num {
                                            format!("Out[{}]:", n)
                                        } else {
                                            "Out[ ]:".to_string()
                                        };
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                RichText::new(out_label)
                                                    .font(egui::FontId::new(
                                                        12.0,
                                                        egui::FontFamily::Monospace,
                                                    ))
                                                    .color(colors.error),
                                            );
                                        });
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(&output)
                                                    .font(egui::FontId::new(
                                                        13.0,
                                                        egui::FontFamily::Monospace,
                                                    ))
                                                    .color(colors.text_secondary),
                                            )
                                            .selectable(true),
                                        );
                                    });
                                let copy_output = output.clone();
                                out_frame.response.context_menu(|ui| {
                                    if ui.button("Copy output").clicked() {
                                        ui.ctx().copy_text(copy_output.clone());
                                        ui.close_menu();
                                    }
                                });
                            }
                        });

                        // Separator between cells
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(4.0);
                    }
                });
            });
        });

    // Ctrl+X: cut (copy all notebook content — notebook view is read-only)
    if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::X)) {
        ctx.copy_text(all_notebook_text);
    }
}

/// Render the Markdown view using commonmark.
pub fn render_markdown_view(ui: &mut egui::Ui, tab: &mut TabState) {
    if let Some(ref content) = tab.raw_content {
        let md_content = content.clone();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.vertical(|ui| {
                        ui.set_max_width(900.0);
                        egui_commonmark::CommonMarkViewer::new().show(
                            ui,
                            &mut tab.commonmark_cache,
                            &md_content,
                        );
                    });
                });
            });
    } else {
        ui.centered_and_justified(|ui| {
            ui.label(
                RichText::new("Markdown content not available")
                    .size(16.0)
                    .color(ui.visuals().weak_text_color()),
            );
        });
    }
}

/// Render the raw text editor view with line numbers and optional column alignment.
pub fn render_raw_view(ui: &mut egui::Ui, tab: &mut TabState, theme_mode: ThemeMode) {
    if let Some(ref mut content) = tab.raw_content {
        let colors = ui::theme::ThemeColors::for_mode(theme_mode);

        // Toolbar for CSV/TSV: align columns + delimiter selector
        let is_csv = tab.table.format_name.as_deref() == Some("CSV");
        let is_tsv = tab.table.format_name.as_deref() == Some("TSV");
        if is_csv || is_tsv {
            ui.horizontal(|ui| {
                if ui
                    .checkbox(&mut tab.raw_view_formatted, "Align Columns")
                    .changed()
                    && tab.raw_view_formatted
                {
                    let delim = tab.csv_delimiter as char;
                    *content = format_delimited_text(content, delim);
                    tab.raw_content_modified = true;
                }
                ui.add_space(16.0);
                if is_csv {
                    ui.label("Delimiter:");
                    let delim_label = match tab.csv_delimiter {
                        b',' => "Comma (,)",
                        b';' => "Semicolon (;)",
                        b'|' => "Pipe (|)",
                        b'\t' => "Tab (\\t)",
                        _ => "Comma (,)",
                    };
                    egui::ComboBox::from_id_salt("csv_delimiter_combo")
                        .selected_text(delim_label)
                        .show_ui(ui, |ui| {
                            let options: &[(u8, &str)] = &[
                                (b',', "Comma (,)"),
                                (b';', "Semicolon (;)"),
                                (b'|', "Pipe (|)"),
                                (b'\t', "Tab (\\t)"),
                            ];
                            for &(delim, label) in options {
                                if ui
                                    .selectable_value(&mut tab.csv_delimiter, delim, label)
                                    .clicked()
                                {
                                    tab.raw_content_modified = true;
                                }
                            }
                        });
                }
            });
            ui.add_space(2.0);
        }

        // Line numbers + text editor side by side
        let line_count = content.lines().count().max(1);
        let line_num_text: String = (1..=line_count)
            .map(|n| format!("{:>width$}", n, width = line_count.to_string().len()))
            .collect::<Vec<_>>()
            .join("\n");
        let line_num_width = line_count.to_string().len() as f32 * 8.0 + 16.0;

        let mono_font = egui::FontId::new(13.0, egui::FontFamily::Monospace);
        let nowrap_layouter = |ui: &egui::Ui, text: &str, _wrap_width: f32| {
            let mut job = egui::text::LayoutJob::simple(
                text.to_owned(),
                egui::FontId::new(13.0, egui::FontFamily::Monospace),
                ui.visuals().text_color(),
                f32::INFINITY,
            );
            job.wrap.max_width = f32::INFINITY;
            ui.fonts(|f| f.layout_job(job))
        };

        let content_for_copy = content.clone();

        // Allocate full available rect for right-click detection
        let full_rect = ui.max_rect();
        let raw_area = ui.interact(
            full_rect,
            ui.id().with("raw_view_ctx"),
            egui::Sense::click(),
        );

        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    // Line numbers column (non-editable)
                    ui.add_sized(
                        [line_num_width, ui.available_height()],
                        egui::TextEdit::multiline(&mut line_num_text.clone())
                            .font(mono_font.clone())
                            .interactive(false)
                            .desired_width(line_num_width)
                            .text_color(colors.text_muted)
                            .frame(false)
                            .layouter(&mut nowrap_layouter.clone()),
                    );
                    // Separator line
                    ui.add_space(2.0);
                    let sep_rect = egui::Rect::from_min_size(
                        ui.cursor().left_top(),
                        egui::vec2(1.0, ui.available_height()),
                    );
                    ui.painter().rect_filled(sep_rect, 0.0, colors.border);
                    ui.add_space(4.0);
                    // Text editor (no wrapping — scroll horizontally)
                    let response = ui.add(
                        egui::TextEdit::multiline(content)
                            .font(mono_font)
                            .desired_width(f32::INFINITY)
                            .text_color(colors.text_primary)
                            .layouter(&mut nowrap_layouter.clone()),
                    );
                    if response.changed() {
                        tab.raw_content_modified = true;
                    }
                });
            });

        // Right-click context menu
        raw_area.context_menu(|ui| {
            if ui.button("Copy All").clicked() {
                ui.ctx().copy_text(content_for_copy.clone());
                ui.close_menu();
            }
        });
    } else {
        ui.centered_and_justified(|ui| {
            ui.label(
                RichText::new("Raw text view is not available for binary formats")
                    .size(16.0)
                    .color(ui.visuals().weak_text_color()),
            );
        });
    }
}

/// Render the interactive JSON tree view (Firefox-style collapsible tree).
pub fn render_json_tree_view(ui: &mut egui::Ui, tab: &mut TabState, theme_mode: ThemeMode) {
    let Some(ref value) = tab.json_value else {
        ui.centered_and_justified(|ui| {
            ui.label(
                RichText::new("JSON tree view is not available")
                    .size(16.0)
                    .color(ui.visuals().weak_text_color()),
            );
        });
        return;
    };

    let colors = ui::theme::ThemeColors::for_mode(theme_mode);
    let value = value.clone();
    let json_text = tab
        .raw_content
        .clone()
        .unwrap_or_else(|| serde_json::to_string_pretty(&value).unwrap_or_default());

    // Allocate full available rect for right-click detection
    let full_rect = ui.max_rect();
    let json_area = ui.interact(
        full_rect,
        ui.id().with("json_tree_ctx"),
        egui::Sense::click(),
    );

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.vertical(|ui| {
                    render_json_node(
                        ui,
                        &value,
                        "",
                        None,
                        false,
                        0,
                        &colors,
                        &mut tab.json_tree_expanded,
                        true,
                    );
                });
            });
        });

    // Right-click context menu on the whole JSON tree area
    json_area.context_menu(|ui| {
        if ui.button("Copy JSON").clicked() {
            ui.ctx().copy_text(json_text.clone());
            ui.close_menu();
        }
    });

    // Ctrl+C / Ctrl+X: copy full JSON
    if ui.input(|i| {
        i.modifiers.command && (i.key_pressed(egui::Key::C) || i.key_pressed(egui::Key::X))
    }) {
        ui.ctx().copy_text(json_text);
    }
}

/// Colors for different JSON value types.
fn json_value_color(value: &serde_json::Value, colors: &ui::theme::ThemeColors) -> Color32 {
    match value {
        serde_json::Value::String(_) => Color32::from_rgb(10, 140, 70),
        serde_json::Value::Number(_) => Color32::from_rgb(30, 100, 200),
        serde_json::Value::Bool(_) => Color32::from_rgb(180, 80, 180),
        serde_json::Value::Null => colors.text_muted,
        _ => colors.text_primary,
    }
}

/// Render a single JSON node recursively.
fn render_json_node(
    ui: &mut egui::Ui,
    value: &serde_json::Value,
    path: &str,
    key: Option<&str>,
    is_index: bool,
    depth: usize,
    colors: &ui::theme::ThemeColors,
    expanded: &mut std::collections::HashSet<String>,
    is_last: bool,
) {
    let indent = depth as f32 * 20.0;
    let mono = egui::FontId::new(13.0, egui::FontFamily::Monospace);
    let comma = if is_last { "" } else { "," };

    // Render the key label: "key": for object keys, index: for array items
    let show_key = |ui: &mut egui::Ui| {
        if let Some(k) = key {
            let label = if is_index {
                format!("{k}:")
            } else {
                format!("\"{k}\":")
            };
            let key_color = if is_index {
                colors.text_muted
            } else {
                colors.accent
            };
            ui.add(
                egui::Label::new(RichText::new(label).font(mono.clone()).color(key_color))
                    .selectable(true),
            );
            ui.add_space(4.0);
        }
    };

    match value {
        serde_json::Value::Object(map) => {
            let is_expanded = expanded.contains(path);
            let count = map.len();

            ui.horizontal(|ui| {
                ui.add_space(indent);
                let arrow = if is_expanded { "\u{25BC}" } else { "\u{25B6}" };
                if ui
                    .add(
                        egui::Label::new(
                            RichText::new(arrow)
                                .font(mono.clone())
                                .color(colors.text_muted),
                        )
                        .selectable(false)
                        .sense(egui::Sense::click()),
                    )
                    .clicked()
                {
                    if is_expanded {
                        expanded.remove(path);
                    } else {
                        expanded.insert(path.to_string());
                    }
                }
                show_key(ui);
                if is_expanded {
                    ui.label(
                        RichText::new("{")
                            .font(mono.clone())
                            .color(colors.text_primary),
                    );
                } else {
                    ui.label(
                        RichText::new(format!("{{...}} ({count} keys){comma}"))
                            .font(mono.clone())
                            .color(colors.text_muted),
                    );
                }
            });

            if is_expanded {
                let entries: Vec<_> = map.iter().collect();
                for (i, (k, v)) in entries.iter().enumerate() {
                    let child_path = if path.is_empty() {
                        k.to_string()
                    } else {
                        format!("{path}.{k}")
                    };
                    render_json_node(
                        ui,
                        v,
                        &child_path,
                        Some(k),
                        false,
                        depth + 1,
                        colors,
                        expanded,
                        i == entries.len() - 1,
                    );
                }
                ui.horizontal(|ui| {
                    ui.add_space(indent);
                    ui.label(
                        RichText::new(format!("}}{comma}"))
                            .font(mono.clone())
                            .color(colors.text_primary),
                    );
                });
            }
        }
        serde_json::Value::Array(arr) => {
            let is_expanded = expanded.contains(path);
            let count = arr.len();

            ui.horizontal(|ui| {
                ui.add_space(indent);
                let arrow = if is_expanded { "\u{25BC}" } else { "\u{25B6}" };
                if ui
                    .add(
                        egui::Label::new(
                            RichText::new(arrow)
                                .font(mono.clone())
                                .color(colors.text_muted),
                        )
                        .selectable(false)
                        .sense(egui::Sense::click()),
                    )
                    .clicked()
                {
                    if is_expanded {
                        expanded.remove(path);
                    } else {
                        expanded.insert(path.to_string());
                    }
                }
                show_key(ui);
                if is_expanded {
                    ui.label(
                        RichText::new("[")
                            .font(mono.clone())
                            .color(colors.text_primary),
                    );
                } else {
                    ui.label(
                        RichText::new(format!("[...] ({count} items){comma}"))
                            .font(mono.clone())
                            .color(colors.text_muted),
                    );
                }
            });

            if is_expanded {
                for (i, v) in arr.iter().enumerate() {
                    let child_path = if path.is_empty() {
                        format!("[{i}]")
                    } else {
                        format!("{path}[{i}]")
                    };
                    render_json_node(
                        ui,
                        v,
                        &child_path,
                        Some(&i.to_string()),
                        true,
                        depth + 1,
                        colors,
                        expanded,
                        i == arr.len() - 1,
                    );
                }
                ui.horizontal(|ui| {
                    ui.add_space(indent);
                    ui.label(
                        RichText::new(format!("]{comma}"))
                            .font(mono.clone())
                            .color(colors.text_primary),
                    );
                });
            }
        }
        // Leaf values (string, number, bool, null)
        _ => {
            let display = match value {
                serde_json::Value::String(s) => format!("\"{s}\"{comma}"),
                serde_json::Value::Number(n) => format!("{n}{comma}"),
                serde_json::Value::Bool(b) => format!("{b}{comma}"),
                serde_json::Value::Null => format!("null{comma}"),
                _ => unreachable!(),
            };
            let color = json_value_color(value, colors);
            ui.horizontal(|ui| {
                ui.add_space(indent);
                ui.add_space(18.0); // Align with content after arrows
                show_key(ui);
                ui.add(
                    egui::Label::new(RichText::new(display).font(mono.clone()).color(color))
                        .selectable(true),
                );
            });
        }
    }
}

/// Align columns in delimited text for display.
fn format_delimited_text(content: &str, delimiter: char) -> String {
    let lines: Vec<Vec<&str>> = content
        .lines()
        .map(|line| line.split(delimiter).collect())
        .collect();
    if lines.is_empty() {
        return content.to_string();
    }
    let max_cols = lines.iter().map(|l| l.len()).max().unwrap_or(0);
    let mut widths = vec![0usize; max_cols];
    for line in &lines {
        for (i, cell) in line.iter().enumerate() {
            widths[i] = widths[i].max(cell.trim().len());
        }
    }
    lines
        .iter()
        .map(|line| {
            line.iter()
                .enumerate()
                .map(|(i, cell)| {
                    let trimmed = cell.trim();
                    if i < line.len() - 1 {
                        format!("{:<width$}", trimmed, width = widths[i])
                    } else {
                        trimmed.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join(&format!("{} ", delimiter))
        })
        .collect::<Vec<_>>()
        .join("\n")
}
