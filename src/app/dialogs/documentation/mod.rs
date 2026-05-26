//! In-app documentation. Categorized into sections so the dialog can offer a
//! sidebar nav (left) + content pane (right), mirroring the structure of the
//! Settings dialog. The shortcut table is generated from the user's current
//! bindings each time the dialog opens, so it never drifts from behavior.

mod content;

use eframe::egui;

use octa::ui;
use octa::ui::settings::{DialogSize, draw_window_controls};

use content::*;

use super::super::state::OctaApp;
use crate::view_modes::markdown::render_pulldown;

const SIDEBAR_WIDTH: f32 = 180.0;

/// Build the Markdown shortcut table rendered in the Shortcuts section.
fn build_shortcut_doc_table(shortcuts: &ui::shortcuts::Shortcuts) -> String {
    use strum::IntoEnumIterator;
    let mut s = String::from("| Shortcut | Action |\n|----------|--------|\n");
    for action in ui::shortcuts::ShortcutAction::iter() {
        let combo = shortcuts.combo(action);
        s.push_str(&format!("| {} | {} |\n", combo.label(), action.label()));
    }
    s
}

/// Returns the ordered list of documentation sections. The Shortcuts section
/// embeds the live key-binding table; all other sections are static.
fn sections(shortcuts: &ui::shortcuts::Shortcuts) -> Vec<(&'static str, String)> {
    let shortcut_table = build_shortcut_doc_table(shortcuts);
    vec![
        ("Getting Started", GETTING_STARTED.to_string()),
        ("Navigation & Selection", NAVIGATION.to_string()),
        ("Editing & Undo/Redo", EDITING.to_string()),
        ("Formulas", FORMULAS.to_string()),
        ("Search & Replace", SEARCH.to_string()),
        ("Multi-search", MULTI_SEARCH.to_string()),
        ("Column Filter", COLUMN_FILTER.to_string()),
        ("Column Tools", COLUMN_TOOLS.to_string()),
        ("Value Frequency", VALUE_FREQUENCY.to_string()),
        ("Find Duplicates", FIND_DUPLICATES.to_string()),
        ("Schema Export", SCHEMA_EXPORT.to_string()),
        ("Archive Viewer", ARCHIVE_VIEWER.to_string()),
        ("Selection Stats", SELECTION_STATS.to_string()),
        ("Pinned Tabs", PINNED_TABS.to_string()),
        ("Color Marking", MARKING.to_string()),
        ("View Modes", VIEW_MODES.to_string()),
        ("Compare View", COMPARE_VIEW.to_string()),
        ("EPUB Reader", EPUB_VIEW.to_string()),
        ("Map View", MAP_VIEW.to_string()),
        ("Chart", CHART_VIEW.to_string()),
        ("Tabs & Folder Sidebar", TABS.to_string()),
        ("SQL View", SQL_VIEW.to_string()),
        ("Command-line & MCP", CLI_AND_MCP.to_string()),
        ("Saving", SAVING.to_string()),
        ("Settings Reference", SETTINGS_REFERENCE.to_string()),
        (
            "Shortcuts",
            format!("{}\n\n{}", SHORTCUTS_INTRO, shortcut_table),
        ),
    ]
}

pub(crate) fn render_documentation_dialog(app: &mut OctaApp, ctx: &egui::Context) {
    if !app.show_documentation_dialog {
        return;
    }
    let mut window = egui::Window::new("Documentation")
        .title_bar(false)
        .collapsible(false);
    window = match app.documentation_size {
        DialogSize::Maximized => window.fixed_rect(ctx.content_rect().shrink(8.0)),
        DialogSize::Minimized => window.resizable(false),
        DialogSize::Normal => window.resizable(true).default_size([900.0, 600.0]),
    };
    let minimized = app.documentation_size == DialogSize::Minimized;
    window.show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Documentation").strong().size(16.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if draw_window_controls(ui, &mut app.documentation_size) {
                    app.show_documentation_dialog = false;
                }
            });
        });
        ui.separator();

        if minimized {
            return;
        }

        let entries = sections(&app.settings.shortcuts);
        if app.docs_active_section >= entries.len() {
            app.docs_active_section = 0;
        }

        ui.horizontal_top(|ui| {
            // --- Sidebar nav ---
            ui.allocate_ui_with_layout(
                egui::vec2(SIDEBAR_WIDTH, ui.available_height()),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_width(SIDEBAR_WIDTH);
                    egui::ScrollArea::vertical()
                        .id_salt("docs_sidebar_scroll")
                        .show(ui, |ui| {
                            for (idx, (title, _)) in entries.iter().enumerate() {
                                let is_active = idx == app.docs_active_section;
                                let resp = ui.selectable_label(is_active, *title);
                                if resp.clicked() {
                                    app.docs_active_section = idx;
                                }
                            }
                        });
                },
            );
            ui.separator();
            // --- Content pane ---
            ui.vertical(|ui| {
                egui::ScrollArea::vertical()
                    .id_salt("docs_content_scroll")
                    .show(ui, |ui| {
                        let body = &entries[app.docs_active_section].1;
                        let cap = ui.available_width().clamp(200.0, 900.0);
                        ui.set_max_width(cap);
                        render_pulldown(ui, body);
                    });
            });
        });
    });
}
