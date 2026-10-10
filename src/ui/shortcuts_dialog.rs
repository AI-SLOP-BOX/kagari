use crate::KagariApp;
use eframe::egui;

pub fn draw_shortcuts_dialog(app: &mut KagariApp, ctx: &egui::Context) {
    let mut show = app.show_shortcuts_dialog;
    if !show {
        return;
    }

    crate::ui::modal::dialog("Keyboard Shortcuts")
        .open(&mut show)
        .resizable(true)
        .default_size(egui::vec2(520.0, 420.0))
        .show(ctx, |ui| {
            ui.heading("Keybindings");
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                crate::ui::shortcuts::draw_shortcuts_reference(ui);
            });
        });

    app.show_shortcuts_dialog = show;
}

pub fn draw_shortcuts_reference_dialog(ctx: &egui::Context, open: &mut bool) {
    crate::ui::modal::window("Keyboard Shortcuts Reference")
        .open(open)
        .resizable(true)
        .show(ctx, |ui| {
            ui.heading("Kagari VFX — Shortcuts Reference");
            ui.separator();
            crate::ui::shortcuts::draw_shortcuts_reference(ui);
        });
}
