//! Settings dialog UI rendering. The full `impl SettingsDialog` lives here;
//! the struct definition + supporting `AppSettings` plus enums stay in
//! [`super`]. Split out purely for navigability - no behaviour change.

use egui;

use super::*;
use crate::data::{BinaryDisplayMode, MapMode, MarkColor, SearchMode};
use crate::ui::shortcuts::{KeyCombo, ShortcutAction};
use crate::ui::theme::{BodyFont, ThemeMode};

impl SettingsDialog {
    /// Open the dialog, seeding the draft from current settings.
    pub fn open(&mut self, current: &AppSettings) {
        self.draft = current.clone();
        self.icon_changed = false;
        self.font_changed = false;
        self.theme_changed = false;
        self.sql_row_limit_buf = current.sql_default_row_limit.to_string();
        // Pick the most natural unit for the current bytes value so the
        // user sees "1 MB" rather than "1,048,576 Bytes" when the setting
        // is at the default.
        self.syntax_highlight_size_unit =
            SyntaxSizeUnit::best_fit(current.syntax_highlight_max_bytes);
        // `SyntaxSizeUnit::factor` is always >= 1, so the division is safe.
        let unit_factor = self.syntax_highlight_size_unit.factor();
        self.syntax_highlight_max_bytes_buf =
            crate::ui::status_bar::format_number(current.syntax_highlight_max_bytes / unit_factor);
        self.initial_load_rows_buf =
            crate::ui::status_bar::format_number(current.initial_load_rows);
        self.text_mode_extensions_buf = current.text_mode_extensions.join(", ");
        // MCP buffers seed from the live settings.
        self.mcp_unlimited_rows = current.mcp_default_row_limit.is_none();
        self.mcp_row_limit_buf =
            crate::ui::status_bar::format_number(current.mcp_default_row_limit.unwrap_or(1000));
        self.mcp_cell_bytes_buf =
            crate::ui::status_bar::format_number(current.mcp_default_cell_bytes);
        self.grep_max_file_size_buf =
            crate::ui::status_bar::format_number(current.grep_max_file_size_mb as usize);
        self.chart_max_points_buf = crate::ui::status_bar::format_number(current.chart_max_points);
        self.chart_max_categories_buf =
            crate::ui::status_bar::format_number(current.chart_max_categories);
        self.table_picker_visible_rows_buf =
            crate::ui::status_bar::format_number(current.table_picker_visible_rows);
        self.excel_max_auto_sheets_buf =
            crate::ui::status_bar::format_number(current.excel_max_auto_sheets);
        self.recording = None;
        self.shortcut_conflict = None;
        self.show_reset_confirm = false;
        self.open = true;
    }

    /// Draw the dialog. Returns `Some(settings)` when the user clicks Apply.
    /// `logo` is an optional texture (the app icon) rendered as a header; passing
    /// `None` omits it and shows just the title.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        logo: Option<&egui::TextureHandle>,
    ) -> Option<AppSettings> {
        if !self.open {
            return None;
        }

        let mut applied: Option<AppSettings> = None;

        // Render the reset-confirm modal first so it sits above the Settings
        // window in the same frame.
        self.draw_reset_confirm(ctx);

        // Custom title bar (egui's is disabled below) - we render Min /
        // Max / Close buttons inline next to the title, like a typical
        // desktop window. Dragging works because the title text is a
        // non-interactive area inside the window's drag region.
        let screen_center = ctx.content_rect().center();
        let default_pos = screen_center - egui::vec2(340.0, 290.0);
        let mut window = egui::Window::new("Settings")
            .title_bar(false)
            .collapsible(false);
        window = match self.size {
            DialogSize::Maximized => window.fixed_rect(ctx.content_rect().shrink(8.0)),
            // Minimized: no min sizing - let egui auto-shrink to the header.
            DialogSize::Minimized => window.resizable(false).default_pos(default_pos),
            DialogSize::Normal => window
                .resizable(true)
                .default_pos(default_pos)
                .min_width(640.0)
                .default_width(680.0)
                .default_height(580.0)
                .min_height(360.0),
        };
        let minimized = self.size == DialogSize::Minimized;
        window.show(ctx, |ui| {
            // Custom title bar: logo + "Octa Settings" + three control
            // buttons. Stays rendered when minimized so the user can
            // restore from there.
            egui::Panel::top("settings_header")
                .frame(egui::Frame::default().inner_margin(egui::Margin::symmetric(0, 6)))
                .show_inside(ui, |ui| {
                    ui.horizontal(|ui| {
                        if let Some(tex) = logo {
                            let size = egui::vec2(28.0, 28.0);
                            ui.add(egui::Image::new(tex).fit_to_exact_size(size));
                            ui.add_space(8.0);
                        }
                        ui.label(egui::RichText::new("Octa Settings").strong().size(16.0));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if draw_window_controls(ui, &mut self.size) {
                                self.open = false;
                            }
                        });
                    });
                });

            if minimized {
                return;
            }

            // Pin Apply/Cancel to the bottom so they're always reachable
            // regardless of how much content the scroll area holds.
            egui::Panel::bottom("settings_buttons")
                .frame(egui::Frame::default().inner_margin(egui::Margin::symmetric(0, 8)))
                .show_inside(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("Apply").clicked() {
                            if let Ok(n) = parse_comma_number(&self.sql_row_limit_buf)
                                && n >= 1
                            {
                                self.draft.sql_default_row_limit = n;
                            }
                            if let Ok(n) = parse_comma_number(&self.syntax_highlight_max_bytes_buf)
                            {
                                // 0 is a valid input meaning "disable highlighting"
                                // - anything <= 0 trips the size guard immediately.
                                let unit_factor = self.syntax_highlight_size_unit.factor();
                                self.draft.syntax_highlight_max_bytes =
                                    n.saturating_mul(unit_factor);
                            }
                            if let Ok(n) = parse_comma_number(&self.initial_load_rows_buf)
                                && n >= 1
                            {
                                self.draft.initial_load_rows = n;
                            }
                            self.draft.text_mode_extensions = self
                                .text_mode_extensions_buf
                                .split([',', ' ', '\t', '\n'])
                                .map(|s| s.trim().trim_start_matches('.').to_lowercase())
                                .filter(|s| !s.is_empty())
                                .collect();
                            // MCP row cap: "Unlimited" overrides the text
                            // input, otherwise parse the comma-separated
                            // number. Invalid input falls back to the
                            // existing draft value so the user doesn't
                            // silently lose their previous setting.
                            if self.mcp_unlimited_rows {
                                self.draft.mcp_default_row_limit = None;
                            } else if let Ok(n) = parse_comma_number(&self.mcp_row_limit_buf)
                                && n >= 1
                            {
                                self.draft.mcp_default_row_limit = Some(n);
                            }
                            if let Ok(n) = parse_comma_number(&self.mcp_cell_bytes_buf) {
                                self.draft.mcp_default_cell_bytes = n;
                            }
                            if let Ok(n) = parse_comma_number(&self.grep_max_file_size_buf) {
                                // Multi-search per-file size cap. Stored as u32
                                // because mb >= 4 GB is nonsense for this knob.
                                self.draft.grep_max_file_size_mb = n.min(u32::MAX as usize) as u32;
                            }
                            if let Ok(n) = parse_comma_number(&self.chart_max_points_buf) {
                                self.draft.chart_max_points = n;
                            }
                            if let Ok(n) = parse_comma_number(&self.chart_max_categories_buf) {
                                self.draft.chart_max_categories = n.max(1);
                            }
                            if let Ok(n) = parse_comma_number(&self.table_picker_visible_rows_buf) {
                                self.draft.table_picker_visible_rows = n.max(1);
                            }
                            if let Ok(n) = parse_comma_number(&self.excel_max_auto_sheets_buf) {
                                self.draft.excel_max_auto_sheets = n.max(1);
                            }
                            applied = Some(self.draft.clone());
                            self.open = false;
                        }
                        if ui.button("Cancel").clicked() {
                            self.open = false;
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let label = egui::RichText::new("Reset to defaults")
                                .color(ui.visuals().error_fg_color);
                            if ui.button(label).clicked() {
                                self.show_reset_confirm = true;
                            }
                        });
                    });
                });

            egui::CentralPanel::default()
                .frame(egui::Frame::default())
                .show_inside(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            self.draw_sections(ui);
                        });
                });
        });

        applied
    }

    /// Render the "Reset to defaults?" confirmation modal. On confirm, the
    /// draft is replaced with `AppSettings::default()` and the icon/font/theme
    /// changed flags are set so the existing Apply path re-applies them.
    /// Nothing is written to disk and the Settings window stays open - the
    /// user still has to click Apply (or Cancel) to commit / discard.
    fn draw_reset_confirm(&mut self, ctx: &egui::Context) {
        if !self.show_reset_confirm {
            return;
        }
        let mut confirm = false;
        let mut cancel = false;
        egui::Window::new("Reset all settings to defaults?")
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(
                    "This replaces every value in the Settings dialog with its default.\n\
                     Nothing is saved until you click Apply - Cancel still reverts.",
                );
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Reset").clicked() {
                        confirm = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });
        if confirm {
            self.draft = AppSettings::default();
            self.sql_row_limit_buf = self.draft.sql_default_row_limit.to_string();
            self.syntax_highlight_size_unit =
                SyntaxSizeUnit::best_fit(self.draft.syntax_highlight_max_bytes);
            // `SyntaxSizeUnit::factor` is always >= 1, so the division is safe.
            let factor = self.syntax_highlight_size_unit.factor();
            self.syntax_highlight_max_bytes_buf = crate::ui::status_bar::format_number(
                self.draft.syntax_highlight_max_bytes / factor,
            );
            self.initial_load_rows_buf =
                crate::ui::status_bar::format_number(self.draft.initial_load_rows);
            self.text_mode_extensions_buf = self.draft.text_mode_extensions.join(", ");
            self.mcp_unlimited_rows = self.draft.mcp_default_row_limit.is_none();
            self.mcp_row_limit_buf = crate::ui::status_bar::format_number(
                self.draft.mcp_default_row_limit.unwrap_or(1000),
            );
            self.mcp_cell_bytes_buf =
                crate::ui::status_bar::format_number(self.draft.mcp_default_cell_bytes);
            self.icon_changed = true;
            self.font_changed = true;
            self.theme_changed = true;
            self.show_reset_confirm = false;
        } else if cancel {
            self.show_reset_confirm = false;
        }
    }

    /// Render the collapsible setting groups inside the scroll area.
    fn draw_sections(&mut self, ui: &mut egui::Ui) {
        // ── Appearance ──
        egui::CollapsingHeader::new(egui::RichText::new("Appearance").strong().size(13.0))
            .id_salt("settings_section_appearance")
            .default_open(false)
            .show(ui, |ui| {
                egui::Grid::new("settings_appearance")
                    .num_columns(2)
                    .spacing([16.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Font size:")
                            .on_hover_text("Base font size for all text in the application");
                        let old_size = self.draft.font_size;
                        let current_pt = self.draft.font_size.round() as i32;
                        egui::ComboBox::from_id_salt("font_size_combo")
                            .selected_text(format!("{} pt", current_pt))
                            .show_ui(ui, |ui| {
                                for sz in 8..=32 {
                                    ui.selectable_value(
                                        &mut self.draft.font_size,
                                        sz as f32,
                                        format!("{} pt", sz),
                                    );
                                }
                            });
                        if self.draft.font_size != old_size {
                            self.font_changed = true;
                        }
                        ui.end_row();

                        ui.label("Default theme:")
                            .on_hover_text("Theme applied when the application starts");
                        let old_theme = self.draft.default_theme;
                        egui::ComboBox::from_id_salt("theme_combo")
                            .selected_text(self.draft.default_theme.label())
                            .show_ui(ui, |ui| {
                                for &preset in ThemeMode::ALL {
                                    ui.selectable_value(
                                        &mut self.draft.default_theme,
                                        preset,
                                        preset.label(),
                                    );
                                }
                            });
                        if self.draft.default_theme != old_theme {
                            self.theme_changed = true;
                        }
                        ui.end_row();

                        ui.label("Body font:").on_hover_text(
                            "Font family used for body, button and heading text.\n\
                                 Monospace gives every character the same width.",
                        );
                        let old_body_font = self.draft.body_font;
                        egui::ComboBox::from_id_salt("body_font_combo")
                            .selected_text(self.draft.body_font.label())
                            .show_ui(ui, |ui| {
                                for &choice in BodyFont::ALL {
                                    ui.selectable_value(
                                        &mut self.draft.body_font,
                                        choice,
                                        choice.label(),
                                    );
                                }
                            });
                        if self.draft.body_font != old_body_font {
                            self.font_changed = true;
                        }
                        ui.end_row();

                        ui.label("Custom font (.ttf, .otf, .ttc):").on_hover_text(
                            "Optional path to a TrueType (.ttf), OpenType (.otf),\n\
                                 or TrueType Collection (.ttc) font file. When set and\n\
                                 readable, overrides the body font choice for proportional text.\n\
                                 WOFF/WOFF2 are not supported.",
                        );
                        let old_path = self.draft.custom_font_path.clone();
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.draft.custom_font_path)
                                    .hint_text("(none - .ttf, .otf, or .ttc)")
                                    .desired_width(220.0),
                            );
                            if ui.button("Browse...").clicked()
                                && let Some(p) = rfd::FileDialog::new()
                                    .add_filter("Font (.ttf, .otf, .ttc)", &["ttf", "otf", "ttc"])
                                    .pick_file()
                            {
                                self.draft.custom_font_path = p.to_string_lossy().into_owned();
                            }
                            if !self.draft.custom_font_path.is_empty()
                                && ui.button("Clear").clicked()
                            {
                                self.draft.custom_font_path.clear();
                            }
                        });
                        if self.draft.custom_font_path != old_path {
                            self.font_changed = true;
                        }
                        ui.end_row();

                        ui.label("Icon color:")
                            .on_hover_text("Color variant for the application icon");
                        let old_icon = self.draft.icon_variant;
                        ui.horizontal(|ui| {
                            paint_icon_swatch(ui, self.draft.icon_variant.preview_color());
                            egui::ComboBox::from_id_salt("icon_combo")
                                .selected_text(self.draft.icon_variant.label())
                                .show_ui(ui, |ui| {
                                    for &variant in IconVariant::ALL {
                                        ui.horizontal(|ui| {
                                            paint_icon_swatch(ui, variant.preview_color());
                                            ui.selectable_value(
                                                &mut self.draft.icon_variant,
                                                variant,
                                                variant.label(),
                                            );
                                        });
                                    }
                                });
                        });
                        if self.draft.icon_variant != old_icon {
                            self.icon_changed = true;
                        }
                        ui.end_row();

                        ui.label("Window controls in toolbar:").on_hover_text(
                            "Disable the system window decorations and let Octa\n\
                             draw close / minimize / maximize buttons at the\n\
                             right edge of the main toolbar. Useful on tiling\n\
                             WMs that don't provide controls. Takes effect\n\
                             after restart.",
                        );
                        ui.checkbox(&mut self.draft.use_custom_title_bar, "");
                        ui.end_row();
                    });
            });

        // ── Table View ──
        egui::CollapsingHeader::new(egui::RichText::new("Table View").strong().size(13.0))
            .id_salt("settings_section_table")
            .default_open(false)
            .show(ui, |ui| {
                egui::Grid::new("settings_table")
                    .num_columns(2)
                    .spacing([16.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Show row numbers:")
                            .on_hover_text("Display row numbers in the leftmost column");
                        ui.checkbox(&mut self.draft.show_row_numbers, "");
                        ui.end_row();

                        ui.label("Alternating row colors:")
                            .on_hover_text("Alternate row background colors for readability");
                        ui.checkbox(&mut self.draft.alternating_row_colors, "");
                        ui.end_row();

                        ui.label("Negative numbers in red:")
                            .on_hover_text("Highlight negative numeric values with red text");
                        ui.checkbox(&mut self.draft.negative_numbers_red, "");
                        ui.end_row();

                        ui.label("Thousand separators:").on_hover_text(
                            "Show thousand separators in numeric cells\n\
                             (e.g. 1,234,567.89). Display only - saved /\n\
                             exported data is never changed.",
                        );
                        ui.checkbox(&mut self.draft.thousands_separators_in_cells, "");
                        ui.end_row();

                        ui.label("Number style:").on_hover_text(
                            "Grouping and decimal marks for numeric cells.\n\
                             English: 1,234.56   European: 1.234,56\n\
                             The decimal mark follows this even with\n\
                             thousand separators off.",
                        );
                        egui::ComboBox::from_id_salt("settings_number_separator_style")
                            .selected_text(self.draft.number_separator_style.label())
                            .show_ui(ui, |ui| {
                                for style in
                                    crate::data::num_format::SeparatorStyle::ALL.iter().copied()
                                {
                                    ui.selectable_value(
                                        &mut self.draft.number_separator_style,
                                        style,
                                        style.label(),
                                    );
                                }
                            });
                        ui.end_row();

                        ui.label("Highlight edited cells:")
                            .on_hover_text("Show background color on modified cells");
                        ui.checkbox(&mut self.draft.highlight_edits, "");
                        ui.end_row();

                        ui.label("Cell line breaks:").on_hover_text(
                            "Allow long text to wrap onto multiple lines\n\
                             within a table cell instead of clipping",
                        );
                        ui.checkbox(&mut self.draft.cell_line_breaks, "");
                        ui.end_row();

                        ui.label("Binary display:").on_hover_text(
                            "How to show binary data columns\n\
                             Binary: raw bits (01000001)\n\
                             Hex: hexadecimal (41)\n\
                             Text: decode as UTF-8 when possible",
                        );
                        egui::ComboBox::from_id_salt("binary_display_combo")
                            .selected_text(self.draft.binary_display_mode.label())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.draft.binary_display_mode,
                                    BinaryDisplayMode::Binary,
                                    BinaryDisplayMode::Binary.label(),
                                );
                                ui.selectable_value(
                                    &mut self.draft.binary_display_mode,
                                    BinaryDisplayMode::Hex,
                                    BinaryDisplayMode::Hex.label(),
                                );
                                ui.selectable_value(
                                    &mut self.draft.binary_display_mode,
                                    BinaryDisplayMode::Text,
                                    BinaryDisplayMode::Text.label(),
                                );
                            });
                        ui.end_row();

                        ui.label("Default mark color:").on_hover_text(
                            "Color applied by the Mark shortcut (Ctrl+M by default).\n\
                             The toolbar / context menu still let you pick any color.",
                        );
                        egui::ComboBox::from_id_salt("default_mark_color_combo")
                            .selected_text(self.draft.default_mark_color.label())
                            .show_ui(ui, |ui| {
                                for &color in MarkColor::ALL {
                                    ui.selectable_value(
                                        &mut self.draft.default_mark_color,
                                        color,
                                        color.label(),
                                    );
                                }
                            });
                        ui.end_row();
                    });
            });

        // ── Search & Editor ──
        egui::CollapsingHeader::new(egui::RichText::new("Search & Editor").strong().size(13.0))
            .id_salt("settings_section_search_editor")
            .default_open(false)
            .show(ui, |ui| {
                egui::Grid::new("settings_search_editor")
                    .num_columns(2)
                    .spacing([16.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Default search mode:")
                            .on_hover_text("Default search/filter mode for new tabs");
                        egui::ComboBox::from_id_salt("search_mode_combo")
                            .selected_text(self.draft.default_search_mode.label())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.draft.default_search_mode,
                                    SearchMode::Plain,
                                    "Plain",
                                );
                                ui.selectable_value(
                                    &mut self.draft.default_search_mode,
                                    SearchMode::Wildcard,
                                    "Wildcard",
                                );
                                ui.selectable_value(
                                    &mut self.draft.default_search_mode,
                                    SearchMode::Regex,
                                    "Regex",
                                );
                            });
                        ui.end_row();

                        ui.label("Tab size:")
                            .on_hover_text("Spaces inserted when pressing Tab in the text editor");
                        egui::ComboBox::from_id_salt("tab_size_combo")
                            .selected_text(self.draft.tab_size.to_string())
                            .width(40.0)
                            .show_ui(ui, |ui| {
                                for n in 1..=16 {
                                    ui.selectable_value(&mut self.draft.tab_size, n, n.to_string());
                                }
                            });
                        ui.end_row();
                    });
            });

        // ── File-Specific ──
        egui::CollapsingHeader::new(egui::RichText::new("File-Specific").strong().size(13.0))
            .id_salt("settings_section_format")
            .default_open(false)
            .show(ui, |ui| {
                egui::Grid::new("settings_format")
                    .num_columns(2)
                    .spacing([16.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Color aligned columns:").on_hover_text(
                            "Color columns in CSV/TSV Raw Text view\n\
                             (only applies when 'Align Columns' is enabled)",
                        );
                        ui.checkbox(&mut self.draft.color_aligned_columns, "");
                        ui.end_row();

                        ui.label("Warn before un-aligning CSV:").on_hover_text(
                            "Confirm before turning 'Align Columns' off in the raw\n\
                             CSV/TSV view - un-aligning reloads the file from disk\n\
                             and discards in-buffer edits.",
                        );
                        ui.checkbox(&mut self.draft.warn_raw_align_reload, "");
                        ui.end_row();

                        ui.label("Warn on date format change:").on_hover_text(
                            "Show a banner when date inference rewrites a column\n\
                             into ISO display form (e.g. stored as 02.05.2026,\n\
                             shown as 2026-05-02). Disable to silence the warning.",
                        );
                        ui.checkbox(&mut self.draft.warn_on_date_format_change, "");
                        ui.end_row();

                        ui.label("Trim whitespace on load:").on_hover_text(
                            "Strip leading/trailing whitespace from string cells\n\
                             when a file is opened. Interior spaces are kept.",
                        );
                        ui.checkbox(&mut self.draft.trim_whitespace_on_load, "");
                        ui.end_row();

                        ui.label("Warn on whitespace trim:").on_hover_text(
                            "Show a banner listing which columns had whitespace\n\
                             trimmed on load. Independent of the trim setting.",
                        );
                        ui.checkbox(&mut self.draft.warn_on_whitespace_trim, "");
                        ui.end_row();

                        ui.label("Read-only mode notice:").on_hover_text(
                            "Pop a confirmation modal each time read-only mode\n\
                             is toggled (via F8 or the View menu). Turn off to\n\
                             flip the state silently.",
                        );
                        ui.checkbox(&mut self.draft.show_readonly_notice, "");
                        ui.end_row();

                        ui.label("Notebook output:").on_hover_text(
                            "Code output position in Jupyter Notebook view\n\
                             (only applies to .ipynb files in Notebook view mode)",
                        );
                        egui::ComboBox::from_id_salt("notebook_layout_combo")
                            .selected_text(self.draft.notebook_output_layout.label())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.draft.notebook_output_layout,
                                    NotebookOutputLayout::Beside,
                                    "Beside",
                                );
                                ui.selectable_value(
                                    &mut self.draft.notebook_output_layout,
                                    NotebookOutputLayout::Beneath,
                                    "Beneath",
                                );
                            });
                        ui.end_row();
                    });
            });

        // ── SQL ──
        egui::CollapsingHeader::new(egui::RichText::new("SQL").strong().size(13.0))
            .id_salt("settings_section_sql")
            .default_open(false)
            .show(ui, |ui| {
                egui::Grid::new("settings_sql")
                    .num_columns(2)
                    .spacing([16.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Open SQL panel by default:").on_hover_text(
                            "When opening a tabular file, automatically show the\n\
                             SQL editor alongside the table view.",
                        );
                        ui.checkbox(&mut self.draft.sql_panel_default_open, "");
                        ui.end_row();

                        ui.label("SQL panel position:")
                            .on_hover_text("Where the SQL editor docks relative to the table");
                        egui::ComboBox::from_id_salt("sql_panel_position_combo")
                            .selected_text(self.draft.sql_panel_position.label())
                            .show_ui(ui, |ui| {
                                for &pos in SqlPanelPosition::ALL {
                                    ui.selectable_value(
                                        &mut self.draft.sql_panel_position,
                                        pos,
                                        pos.label(),
                                    );
                                }
                            });
                        ui.end_row();

                        ui.label("Default row limit:").on_hover_text(
                            "LIMIT used in the placeholder query when a tab is opened\n\
                             (e.g. 100 -> SELECT * FROM data LIMIT 100).\n\
                             Type a number - applied on Apply.",
                        );
                        ui.add(
                            egui::TextEdit::singleline(&mut self.sql_row_limit_buf)
                                .desired_width(80.0)
                                .hint_text("100"),
                        );
                        ui.end_row();

                        ui.label("Autocomplete:").on_hover_text(
                            "Offer SQL keyword and column-name suggestions\n\
                             beneath the SQL editor.",
                        );
                        ui.checkbox(&mut self.draft.sql_autocomplete, "");
                        ui.end_row();

                        ui.label("Editor font:").on_hover_text(
                            "Font face used by the SQL editor and its line-number\n\
                             gutter. JetBrains Mono is bundled with Octa.\n\
                             \n\
                             Note: programming ligatures (e.g. != -> ≠, -> -> ->)\n\
                             are not applied - egui's text renderer does not\n\
                             process OpenType GSUB substitutions yet.",
                        );
                        egui::ComboBox::from_id_salt("sql_editor_font_combo")
                            .selected_text(self.draft.sql_editor_font.label())
                            .show_ui(ui, |ui| {
                                for &font in SqlEditorFont::ALL {
                                    ui.selectable_value(
                                        &mut self.draft.sql_editor_font,
                                        font,
                                        font.label(),
                                    );
                                }
                            });
                        ui.end_row();
                    });
            });

        // ── MCP server ──
        egui::CollapsingHeader::new(egui::RichText::new("MCP").strong().size(13.0))
            .id_salt("settings_section_mcp")
            .default_open(false)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(
                        "Defaults for the MCP server (`octa --mcp`). The server reads these \
                         once at startup; changing them while a server is running needs an \
                         `octa --mcp` restart.",
                    )
                    .weak()
                    .size(11.0),
                );
                ui.add_space(6.0);
                egui::Grid::new("settings_mcp")
                    .num_columns(2)
                    .spacing([16.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Default row limit:").on_hover_text(
                            "Maximum rows returned by `read_table` / `run_sql` when the \
                             caller omits `limit`. The tool schema advertises this default \
                             to the model so it can ask for more (or unlimited) when needed.\n\
                             \n\
                             Default: 1,000. Setting this high - or checking Unlimited - \
                             can push very large responses through stdio and slow down the \
                             MCP client.",
                        );
                        ui.horizontal(|ui| {
                            let edit = egui::TextEdit::singleline(&mut self.mcp_row_limit_buf)
                                .desired_width(100.0)
                                .hint_text("1,000");
                            ui.add_enabled(!self.mcp_unlimited_rows, edit);
                            ui.checkbox(&mut self.mcp_unlimited_rows, "Unlimited");
                        });
                        ui.end_row();

                        ui.label("Cell byte cap:").on_hover_text(
                            "Per-cell on-wire size cap. Cells whose textual form exceeds \
                             this are replaced with a `[truncated: ...]` marker and the \
                             response flags `cell_truncated: true`. Set to 0 to disable \
                             the cap.\n\
                             \n\
                             Default: 65,536 (64 KiB).",
                        );
                        ui.add(
                            egui::TextEdit::singleline(&mut self.mcp_cell_bytes_buf)
                                .desired_width(120.0)
                                .hint_text("65,536"),
                        );
                        ui.end_row();
                    });
            });

        // ── Map ──
        egui::CollapsingHeader::new(egui::RichText::new("Map").strong().size(13.0))
            .id_salt("settings_section_map")
            .default_open(false)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(
                        "Defaults for the Map view (used by GeoJSON files). The active tab \
                         can flip between Tiles and Geometry-only via the Map toolbar.",
                    )
                    .weak()
                    .size(11.0),
                );
                ui.add_space(6.0);
                egui::Grid::new("settings_map")
                    .num_columns(2)
                    .spacing([16.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Default mode:").on_hover_text(
                            "Tiles: fetch a slippy map from the configured tile URL.\n\
                             Geometry only: paint just the features on a blank canvas.",
                        );
                        egui::ComboBox::from_id_salt("map_default_mode_combo")
                            .selected_text(self.draft.map_default_mode.label())
                            .show_ui(ui, |ui| {
                                for &m in MapMode::ALL {
                                    ui.selectable_value(
                                        &mut self.draft.map_default_mode,
                                        m,
                                        m.label(),
                                    );
                                }
                            });
                        ui.end_row();

                        ui.label("Fallback to geometry:").on_hover_text(
                            "When tile fetch fails (offline, blocked), automatically \
                             switch to geometry-only rendering for that tab.",
                        );
                        ui.checkbox(&mut self.draft.map_fallback_to_geometry, "");
                        ui.end_row();

                        ui.label("Tile URL template:").on_hover_text(
                            "URL pattern for raster tiles. `{z}/{x}/{y}` are substituted \
                             with the zoom level and tile coordinates. Default points at \
                             the OSM tile server - for production / heavy use, point at \
                             a self-hosted or commercial provider.",
                        );
                        ui.add(
                            egui::TextEdit::singleline(&mut self.draft.map_tile_url_template)
                                .desired_width(380.0)
                                .hint_text("https://tile.openstreetmap.org/{z}/{x}/{y}.png"),
                        );
                        ui.end_row();
                    });
            });

        // ── Directory Tree ──
        egui::CollapsingHeader::new(egui::RichText::new("Directory Tree").strong().size(13.0))
            .id_salt("settings_section_directory_tree")
            .default_open(false)
            .show(ui, |ui| {
                egui::Grid::new("settings_directory_tree")
                    .num_columns(2)
                    .spacing([16.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Sidebar position:").on_hover_text(
                            "Which side the directory tree is docked on when a\n\
                         folder is opened via File > Open Directory.",
                        );
                        egui::ComboBox::from_id_salt("directory_tree_position_combo")
                            .selected_text(self.draft.directory_tree_position.label())
                            .show_ui(ui, |ui| {
                                for &pos in DirectoryTreePosition::ALL {
                                    ui.selectable_value(
                                        &mut self.draft.directory_tree_position,
                                        pos,
                                        pos.label(),
                                    );
                                }
                            });
                        ui.end_row();
                    });
            });

        // ── Shortcuts ──
        egui::CollapsingHeader::new(egui::RichText::new("Shortcuts").strong().size(13.0))
            .id_salt("settings_section_shortcuts")
            .default_open(false)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(
                        "Click 'Record' then press a key combo (with Ctrl / Shift / Alt).\n\
                         Press Esc to cancel recording. 'Clear' leaves the action unbound.",
                    )
                    .weak()
                    .size(11.0),
                );
                ui.add_space(6.0);
                self.draw_shortcuts_grid(ui);
            });

        // ── Performance ──
        egui::CollapsingHeader::new(egui::RichText::new("Performance").strong().size(13.0))
            .id_salt("settings_section_performance")
            .default_open(false)
            .show(ui, |ui| {
                egui::Grid::new("settings_performance")
                    .num_columns(2)
                    .spacing([16.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Initial-load row cap:").on_hover_text(
                            "Maximum rows loaded into the table on first open for\n\
                             streaming formats (Parquet, CSV, TSV). Remaining rows\n\
                             stream in the background as you scroll.\n\
                             \n\
                             Default: 5,000,000. Raising it improves first-paint\n\
                             completeness but uses more memory. Lowering it makes\n\
                             the initial open faster but pushes more work onto\n\
                             the background loader. Tick \"Unlimited\" to disable\n\
                             the cap entirely and load every row up front.\n\
                             \n\
                             Type a number - applied on Apply.",
                        );
                        ui.horizontal(|ui| {
                            ui.add_enabled(
                                !self.draft.initial_load_rows_unlimited,
                                egui::TextEdit::singleline(&mut self.initial_load_rows_buf)
                                    .desired_width(120.0)
                                    .hint_text("5,000,000"),
                            );
                            ui.checkbox(&mut self.draft.initial_load_rows_unlimited, "Unlimited")
                                .on_hover_text(
                                    "Load every row in the file up front. Recommended only\n\
                                 when you have RAM to spare - a 100 M-row parquet eats\n\
                                 several GB.",
                                );
                        });
                        ui.end_row();

                        ui.label("Syntax-highlight size cap:").on_hover_text(
                            "Files larger than this fall back to plain monospace in\n\
                             the raw editor. Set to 0 to disable syntax highlighting\n\
                             entirely; set a very large number to opt out of the\n\
                             guard. JSON/YAML/XML/Markdown/TOML are never highlighted\n\
                             - they use their dedicated tree/preview views.\n\
                             \n\
                             Default: 1 MB. Type a number, pick a unit - applied on Apply.",
                        );
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(
                                    &mut self.syntax_highlight_max_bytes_buf,
                                )
                                .desired_width(100.0)
                                .hint_text("1"),
                            );
                            egui::ComboBox::from_id_salt("syntax_size_unit_combo")
                                .selected_text(self.syntax_highlight_size_unit.label())
                                .width(70.0)
                                .show_ui(ui, |ui| {
                                    for &unit in SyntaxSizeUnit::ALL {
                                        ui.selectable_value(
                                            &mut self.syntax_highlight_size_unit,
                                            unit,
                                            unit.label(),
                                        );
                                    }
                                });
                        });
                        ui.end_row();

                        ui.label("Open as text:").on_hover_text(
                            "Extra file extensions to treat as plain text on open\n\
                             (overrides whatever reader would normally claim them).\n\
                             Comma- or space-separated, no leading dot, lowercase.\n\
                             Example: log4j  myproj  rawdata\n\
                             \n\
                             Applied on Apply; takes effect for subsequent file opens.",
                        );
                        ui.add(
                            egui::TextEdit::singleline(&mut self.text_mode_extensions_buf)
                                .desired_width(280.0)
                                .hint_text("log4j, myproj, rawdata"),
                        );
                        ui.end_row();

                        ui.label("Multi-search file cap (MB):").on_hover_text(
                            "Per-file size cap for the Multi-search panel's directory\n\
                             scope. Files larger than this are skipped and listed in\n\
                             the skipped-files chip. Set to 0 to disable the cap.\n\
                             \n\
                             Default: 50 MB.",
                        );
                        ui.add(
                            egui::TextEdit::singleline(&mut self.grep_max_file_size_buf)
                                .desired_width(120.0)
                                .hint_text("50"),
                        );
                        ui.end_row();

                        ui.label("Chart max points:").on_hover_text(
                            "Maximum rows the Chart tab will plot before evenly-spaced\n\
                             downsampling kicks in (Histogram, Line, Scatter). Bar charts\n\
                             always aggregate the full input; Box plots compute the\n\
                             5-number summary over the full input. Set to 0 to disable\n\
                             sampling - only safe for moderately-sized tables.\n\
                             \n\
                             Default: 100,000. Numeric input accepts comma separators.",
                        );
                        ui.add(
                            egui::TextEdit::singleline(&mut self.chart_max_points_buf)
                                .desired_width(120.0)
                                .hint_text("100,000"),
                        );
                        ui.end_row();

                        ui.label("Chart max categories:").on_hover_text(
                            "Maximum distinct X categories a Bar chart will accept before\n\
                             refusing to draw. Above this the renderer surfaces an error\n\
                             rather than producing an unreadable wall of bars - filter or\n\
                             aggregate the table first.\n\
                             \n\
                             Default: 200. Numeric input accepts comma separators.",
                        );
                        ui.add(
                            egui::TextEdit::singleline(&mut self.chart_max_categories_buf)
                                .desired_width(120.0)
                                .hint_text("200"),
                        );
                        ui.end_row();

                        ui.label("Tables visible in picker:").on_hover_text(
                            "How many table rows the multi-table picker dialog (SQLite,\n\
                             DuckDB, ...) should fit vertically at its default size. The\n\
                             dialog is still user-resizable - drag the corner to make\n\
                             it bigger when a database has many tables.\n\
                             \n\
                             Default: 10. Minimum 1.",
                        );
                        ui.add(
                            egui::TextEdit::singleline(&mut self.table_picker_visible_rows_buf)
                                .desired_width(120.0)
                                .hint_text("10"),
                        );
                        ui.end_row();

                        ui.label("Excel sheets to auto-open:").on_hover_text(
                            "How many sheets of a multi-sheet Excel workbook to open\n\
                             automatically (each in its own tab). Workbooks with more\n\
                             sheets than this show a picker so you choose which to open\n\
                             (you can still pick more than this number, or all).\n\
                             \n\
                             Default: 5. Minimum 1.",
                        );
                        ui.add(
                            egui::TextEdit::singleline(&mut self.excel_max_auto_sheets_buf)
                                .desired_width(120.0)
                                .hint_text("5"),
                        );
                        ui.end_row();
                    });
            });

        // ── Files ──
        egui::CollapsingHeader::new(egui::RichText::new("Files").strong().size(13.0))
            .id_salt("settings_section_files")
            .default_open(false)
            .show(ui, |ui| {
                egui::Grid::new("settings_files")
                    .num_columns(2)
                    .spacing([16.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Max recent files:").on_hover_text(
                            "Number of recently opened files shown in the File menu",
                        );
                        egui::ComboBox::from_id_salt("max_recent_combo")
                            .selected_text(self.draft.max_recent_files.to_string())
                            .width(50.0)
                            .show_ui(ui, |ui| {
                                for n in 1..=30 {
                                    ui.selectable_value(
                                        &mut self.draft.max_recent_files,
                                        n,
                                        n.to_string(),
                                    );
                                }
                            });
                        ui.end_row();
                    });
            });

        // ── Window ──
        egui::CollapsingHeader::new(egui::RichText::new("Window").strong().size(13.0))
            .id_salt("settings_section_window")
            .default_open(false)
            .show(ui, |ui| {
                egui::Grid::new("settings_window")
                    .num_columns(2)
                    .spacing([16.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Start maximised:").on_hover_text(
                            "When on, the window launches maximised and the size below\n\
                             is used as the restore-from-maximize size.\n\
                             When off, the window launches at the chosen size.",
                        );
                        ui.checkbox(&mut self.draft.start_maximized, "");
                        ui.end_row();

                        ui.label("Initial window size:").on_hover_text(
                            "Window size used at startup (when \"Start maximised\" is off),\n\
                             or the restore-from-maximize size when it is on.",
                        );
                        ui.add_enabled_ui(!self.draft.start_maximized, |ui| {
                            egui::ComboBox::from_id_salt("window_size_combo")
                                .selected_text(self.draft.window_size.label())
                                .show_ui(ui, |ui| {
                                    for &size in WindowSize::ALL {
                                        ui.selectable_value(
                                            &mut self.draft.window_size,
                                            size,
                                            size.label(),
                                        );
                                    }
                                });
                        });
                        ui.end_row();
                    });
            });
    }

    /// One grid row per [`ShortcutAction`]: name, current combo, Record/Clear/Reset.
    fn draw_shortcuts_grid(&mut self, ui: &mut egui::Ui) {
        use strum::IntoEnumIterator;
        // If the user is recording a binding, capture the next real key press.
        if let Some(action) = self.recording {
            let captured = ui.input(capture_combo);
            if let Some(CaptureResult::Cancel) = captured {
                self.recording = None;
            } else if let Some(CaptureResult::Combo(combo)) = captured {
                // Reject combos already bound to another action so two
                // functions can never share a shortcut.
                let conflict = self
                    .draft
                    .shortcuts
                    .bindings
                    .iter()
                    .find(|(other, existing)| **other != action && **existing == combo)
                    .map(|(other, _)| *other);
                if let Some(other) = conflict {
                    self.shortcut_conflict = Some(format!(
                        "{} is already bound to \"{}\". Clear that binding first or pick a different key.",
                        combo.label(),
                        other.label(),
                    ));
                } else {
                    self.draft.shortcuts.set(action, combo);
                    self.shortcut_conflict = None;
                }
                self.recording = None;
            }
        }

        if let Some(msg) = &self.shortcut_conflict {
            ui.colored_label(egui::Color32::from_rgb(0xd9, 0x53, 0x4f), msg);
            ui.add_space(4.0);
        }

        egui::Grid::new("settings_shortcuts_grid")
            .num_columns(4)
            .spacing([12.0, 4.0])
            .show(ui, |ui| {
                for action in ShortcutAction::iter() {
                    ui.label(action.label());
                    let combo = self.draft.shortcuts.combo(action);
                    let label_text = if self.recording == Some(action) {
                        egui::RichText::new("Press any key...").italics()
                    } else {
                        egui::RichText::new(combo.label()).monospace()
                    };
                    ui.label(label_text);
                    if self.recording == Some(action) {
                        if ui.button("Stop").clicked() {
                            self.recording = None;
                        }
                    } else if ui.button("Record").clicked() {
                        self.recording = Some(action);
                    }
                    ui.horizontal(|ui| {
                        if ui.button("Clear").clicked() {
                            self.draft.shortcuts.set(action, KeyCombo::UNBOUND);
                        }
                        if ui.button("Reset").clicked() {
                            self.draft.shortcuts.reset(action);
                        }
                    });
                    ui.end_row();
                }
            });
    }
}
/// Result of a single-frame shortcut capture.
enum CaptureResult {
    Cancel,
    Combo(KeyCombo),
}

/// While recording, watch for a non-modifier key press and return it with the
/// current modifier state. Esc cancels.
fn capture_combo(input: &egui::InputState) -> Option<CaptureResult> {
    if input.key_pressed(egui::Key::Escape) {
        return Some(CaptureResult::Cancel);
    }
    let mods = input.modifiers;
    for ev in &input.events {
        if let egui::Event::Key {
            key,
            pressed: true,
            repeat: false,
            ..
        } = ev
        {
            if matches!(key, egui::Key::Escape) {
                return Some(CaptureResult::Cancel);
            }
            return Some(CaptureResult::Combo(KeyCombo {
                key: Some(*key),
                ctrl: mods.command,
                shift: mods.shift,
                alt: mods.alt,
            }));
        }
    }
    None
}
