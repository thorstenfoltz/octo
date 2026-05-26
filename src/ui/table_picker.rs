use std::path::PathBuf;

use eframe::egui;

use crate::formats::TableInfo;

/// Modal state for picking which table to open from a multi-table source
/// (DuckDB / SQLite). When `Some`, the main app renders a blocking dialog.
pub struct TablePickerState {
    pub path: PathBuf,
    pub format_name: String,
    pub tables: Vec<TableInfo>,
    pub selected: usize,
    /// How many table rows the dialog should fit vertically at its default
    /// size, sourced from `AppSettings::table_picker_visible_rows`. The user
    /// can still drag the window taller after it opens.
    pub visible_rows: usize,
}

/// What the user did with the picker on this frame.
#[derive(Debug, Clone)]
pub enum TablePickerAction {
    /// Still showing — leave state untouched.
    None,
    /// User confirmed; load `(path, table_name)`.
    Open(PathBuf, String),
    /// User cancelled.
    Cancel,
}

/// Render the modal picker. Returns the user's action for this frame.
pub fn render_table_picker(ctx: &egui::Context, state: &mut TablePickerState) -> TablePickerAction {
    let mut action = TablePickerAction::None;
    let mut open_flag = true;

    // Fit-to-content up to `visible_rows`: with 5 tables and a cap of 10 the
    // dialog only reserves height for 5 rows. The user can still drag the
    // window taller after it opens.
    let row_height_approx = 22.0_f32;
    let rows_to_show = state.tables.len().min(state.visible_rows.max(1)).max(1) as f32;
    // 56 px ≈ "contains N tables…" line + spacing.
    // 56 px ≈ footer separator + Cancel/Open row + padding.
    let chrome_h = 56.0 + 56.0;
    let raw_default_h = chrome_h + rows_to_show * row_height_approx;
    let screen_h = ctx.content_rect().height();
    let default_h = raw_default_h.min((screen_h - 80.0).max(220.0));
    let default_w = 480.0_f32;
    let min_h = 200.0_f32.min(default_h);
    let center = ctx.content_rect().center();

    // Use an explicit, stable Window id rather than the title-derived default.
    // Bumping the suffix here forces egui to discard any size/position
    // persisted under the old key (yesterday's WIP saved a much larger box)
    // and start from the new `default_*` numbers on first open.
    let window_id = egui::Id::new("octa_table_picker_dialog_v2");

    egui::Window::new(format!("Open table — {}", state.format_name))
        .id(window_id)
        .collapsible(false)
        .resizable([true, true])
        .default_width(default_w)
        .default_height(default_h)
        .min_width(380.0)
        .min_height(min_h)
        .open(&mut open_flag)
        .default_pos(center - egui::vec2(default_w / 2.0, default_h / 2.0))
        .show(ctx, |ui| {
            // Footer goes in a bottom panel so egui — not us — works out the
            // exact pixel split between body and footer. Doing the math by
            // hand (a fixed `footer_h` literal) was off by a few pixels per
            // frame; the `Resize` widget then auto-grew the window until it
            // filled the screen.
            egui::Panel::bottom("table_picker_footer")
                .resizable(false)
                .show_inside(ui, |ui| {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            action = TablePickerAction::Cancel;
                        }
                        let can_open = state.selected < state.tables.len();
                        let open_resp = ui.add_enabled(can_open, egui::Button::new("Open table"));
                        if open_resp.clicked() && can_open {
                            let name = state.tables[state.selected].name.clone();
                            action = TablePickerAction::Open(state.path.clone(), name);
                        }
                    });
                    ui.add_space(2.0);
                });

            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show_inside(ui, |ui| {
                    render_picker_body(ui, state);
                });
        });

    if !open_flag {
        action = TablePickerAction::Cancel;
    }
    action
}

fn render_picker_body(ui: &mut egui::Ui, state: &mut TablePickerState) {
    ui.label(format!(
        "{} contains {} table{}. Pick one to open:",
        state
            .path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| state.path.display().to_string()),
        state.tables.len(),
        if state.tables.len() == 1 { "" } else { "s" },
    ));
    ui.add_space(6.0);

    let body_h = ui.available_height();
    // Track the user-draggable split between the table list (left)
    // and the schema preview (right). Kept as a per-window memory
    // value so resizing the dialog doesn't reset it.
    let split_id = ui.id().with("table_picker_split");
    let initial_split = 240.0_f32;
    let mut split_w = ui
        .ctx()
        .data_mut(|d| *d.get_persisted_mut_or(split_id, initial_split));
    let min_left = 160.0;
    let min_right = 200.0;
    let max_left = (ui.available_width() - min_right - 16.0).max(min_left);
    split_w = split_w.clamp(min_left, max_left);

    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(split_w, body_h),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("table_picker_list")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for (idx, t) in state.tables.iter().enumerate() {
                            let label = match t.row_count {
                                Some(n) => format!("{}  ({})", t.name, n),
                                None => t.name.clone(),
                            };
                            if ui.selectable_label(state.selected == idx, label).clicked() {
                                state.selected = idx;
                            }
                        }
                    });
            },
        );
        // Draggable splitter — thin vertical strip the user can grab
        // to widen the left list when table names are long.
        let splitter = ui.allocate_response(egui::vec2(6.0, body_h), egui::Sense::click_and_drag());
        if splitter.hovered() || splitter.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        if splitter.dragged() {
            split_w = (split_w + splitter.drag_delta().x).clamp(min_left, max_left);
        }
        let stroke_col = if splitter.hovered() || splitter.dragged() {
            ui.visuals().widgets.hovered.bg_stroke.color
        } else {
            ui.visuals().widgets.noninteractive.bg_stroke.color
        };
        let mid_x = splitter.rect.center().x;
        ui.painter().line_segment(
            [
                egui::pos2(mid_x, splitter.rect.top() + 4.0),
                egui::pos2(mid_x, splitter.rect.bottom() - 4.0),
            ],
            egui::Stroke::new(1.0, stroke_col),
        );
        ui.ctx().data_mut(|d| d.insert_persisted(split_id, split_w));
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), body_h),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                if let Some(t) = state.tables.get(state.selected) {
                    ui.heading(&t.name);
                    ui.add_space(4.0);
                    ui.label(format!("{} columns", t.columns.len()));
                    ui.add_space(4.0);
                    egui::ScrollArea::vertical()
                        .id_salt("table_picker_schema")
                        .show(ui, |ui| {
                            egui::Grid::new("schema_grid")
                                .striped(true)
                                .spacing(egui::vec2(12.0, 4.0))
                                .show(ui, |ui| {
                                    ui.strong("Column");
                                    ui.strong("Type");
                                    ui.end_row();
                                    for col in &t.columns {
                                        ui.label(&col.name);
                                        ui.label(&col.data_type);
                                        ui.end_row();
                                    }
                                });
                        });
                }
            },
        );
    });
}
