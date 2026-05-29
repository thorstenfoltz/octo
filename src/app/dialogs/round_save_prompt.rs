//! "Round on save?" prompt. Shown when the user saves a tab that has
//! per-column rounding formats (which are otherwise display-only). Lets them
//! choose whether the written file carries the rounded values or full
//! precision. Set up by `do_save_tab`; resolved here.

use eframe::egui;

use super::super::state::OctaApp;

pub(crate) fn render_round_save_prompt_dialog(app: &mut OctaApp, ctx: &egui::Context) {
    let Some(prompt) = app.pending_round_save.clone() else {
        return;
    };

    let mut decision: Option<bool> = None;
    let mut cancel = false;
    let mut open = true;

    egui::Window::new("Save rounded values?")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.set_min_width(360.0);
            ui.label(
                "This table has columns with a display rounding format. \
                 Rounding is normally display-only - how should the \
                 file be written?",
            );
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui
                    .button("Save rounded values")
                    .on_hover_text("Write the rounded numbers shown in the table")
                    .clicked()
                {
                    decision = Some(true);
                }
                if ui
                    .button("Save full precision")
                    .on_hover_text("Write the original, un-rounded numbers")
                    .clicked()
                {
                    decision = Some(false);
                }
                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
            });
        });

    if let Some(round) = decision {
        app.pending_round_save = None;
        app.do_save_tab_inner(
            prompt.tab_idx,
            prompt.path,
            prompt.save_filtered_view,
            Some(round),
        );
    } else if cancel || !open {
        app.pending_round_save = None;
    }
}
