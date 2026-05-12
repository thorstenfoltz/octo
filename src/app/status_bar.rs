//! Render the bottom status bar and handle its navigation action
//! ("Go to Cell" input).

use eframe::egui;

use octa::ui;

use super::state::{OctaApp, UpdateState};

impl OctaApp {
    pub(crate) fn render_status_bar(
        &mut self,
        ctx: &egui::Context,
        filtered_count: usize,
        search_active: bool,
    ) {
        let status_colors = ui::theme::ThemeColors::for_mode(self.theme_mode);
        let status_frame = egui::Frame::new()
            .fill(status_colors.bg_header)
            .inner_margin(egui::Margin::symmetric(4, 2))
            .stroke(egui::Stroke::new(1.0, status_colors.border_subtle));

        // Busy indicator state: a long-running operation is either a
        // background row-load draining into the active tab or an
        // update-check / install in flight. We surface a small spinner +
        // one-word reason so the user knows the app is intentionally
        // doing work (and so the WM's startup cursor, which we disabled
        // in octa.desktop, isn't replaced by a different mystery).
        let bg_loading = !self.tabs[self.active_tab]
            .bg_loading_done
            .load(std::sync::atomic::Ordering::Relaxed);
        let update_busy = matches!(
            *self.update_state.lock().unwrap(),
            UpdateState::Checking | UpdateState::Updating
        );
        let busy = bg_loading || update_busy;
        let busy_hint = if update_busy {
            Some("Updating…")
        } else if bg_loading {
            Some("Loading rows…")
        } else {
            None
        };

        let status_action = egui::TopBottomPanel::bottom("status_bar")
            .exact_height(28.0)
            .frame(status_frame)
            .show(ctx, |ui| {
                ui::status_bar::draw_status_bar(
                    ui,
                    &self.tabs[self.active_tab].table,
                    &self.tabs[self.active_tab].table_state,
                    self.theme_mode,
                    filtered_count,
                    search_active,
                    &mut self.nav_input,
                    std::mem::take(&mut self.nav_focus_requested),
                    self.zoom_percent,
                    self.readonly_mode,
                    busy,
                    busy_hint,
                )
            })
            .inner;

        if let Some((row, col)) = status_action.navigate_to {
            let tab = &mut self.tabs[self.active_tab];
            tab.table_state.selected_cell = Some((row, col));
            tab.table_state.selected_rows.clear();
            tab.table_state.selected_cols.clear();
            // Auto-scroll to the target cell
            let row_height =
                (self.settings.font_size * self.zoom_percent as f32 / 100.0 * 2.0).max(26.0);
            tab.table_state.set_scroll_y(row as f32 * row_height);
            let col_left: f32 = tab.table_state.col_widths[..col].iter().sum();
            tab.table_state.set_scroll_x(col_left);
        }

        if status_action.kraken_summoned {
            // Easter egg: typing "kraken" into the nav input wakes the beast.
            // Prefixed with "\u{1f419}" so the central-panel renderer paints
            // the message in the accent color instead of error-red.
            self.status_message = Some((
                "\u{1f419} The kraken stirs from the depths\u{2026}".to_string(),
                std::time::Instant::now(),
            ));
        }
    }
}
