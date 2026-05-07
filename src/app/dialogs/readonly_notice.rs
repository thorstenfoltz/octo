//! Modal that announces a read-only-mode toggle. Contains a "Don't show
//! this again" checkbox that writes through to `AppSettings.show_readonly_notice`,
//! disabling future notices globally.

use eframe::egui;
use egui::RichText;

use super::super::state::OctaApp;

pub(crate) fn render_readonly_notice_dialog(app: &mut OctaApp, ctx: &egui::Context) {
    let Some(notice) = app.pending_readonly_notice.as_ref() else {
        return;
    };
    let is_active = notice.is_active;

    // Pull the persisted checkbox state out of the notice so the dialog
    // can mutate it across frames. Without this round-trip the box would
    // flicker — re-deriving the initial value from settings on every frame
    // overwrites the user's click in the same frame they made it.
    let mut suppress_future = notice.suppress_future;
    let mut close = false;

    let title = if is_active {
        "Read-only mode enabled"
    } else {
        "Read-only mode disabled"
    };
    let body = if is_active {
        "Editing is now disabled across the app. Cells, structural changes, \
         marks, undo/redo, and the raw text editor will all decline to mutate. \
         Saving and copying still work. Toggle off with F8 (or your custom \
         shortcut) when you want to edit again."
    } else {
        "Editing is enabled again. The previous read-only toggle has been \
         lifted; cell edits, marks, undo/redo, and structural changes will \
         apply normally."
    };

    egui::Window::new(title)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.set_min_width(360.0);
            ui.label(body);
            ui.add_space(8.0);
            ui.checkbox(&mut suppress_future, "Don't show this again");
            ui.add_space(4.0);
            ui.label(
                RichText::new("(Re-enable in Settings → Read-only notice.)")
                    .weak()
                    .size(11.0),
            );
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("OK").clicked() {
                    close = true;
                }
            });
        });

    // Mirror the checkbox change back into the notice so the next frame
    // shows the same checked/unchecked state the user clicked.
    if let Some(n) = app.pending_readonly_notice.as_mut() {
        n.suppress_future = suppress_future;
    }

    if close {
        app.settings.show_readonly_notice = !suppress_future;
        app.settings.save();
        app.pending_readonly_notice = None;
    }
}
