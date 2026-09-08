use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub fn draw(app: &mut KagariApp, ctx: &egui::Context) {
    let project_empty = app.history.current().compositions.is_empty();
    if !project_empty && !app.show_welcome {
        return;
    }

    let mut open = app.show_welcome || project_empty;
    egui::Window::new("Welcome")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(320.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.heading("Kagari VFX");
            ui.label(
                egui::RichText::new("GPU-accelerated motion graphics in Rust")
                    .color(colors::TEXT_SECONDARY),
            );
            ui.add_space(12.0);

            if ui
                .add(egui::Button::new("Load Demo Scene").min_size(egui::vec2(200.0, 32.0)))
                .clicked()
            {
                crate::ui::demo_scene::build(app);
                app.show_welcome = false;
            }
            ui.add_space(4.0);
            if ui
                .add(egui::Button::new("New Composition").min_size(egui::vec2(200.0, 32.0)))
                .clicked()
            {
                app.show_welcome = false;
                app.show_new_comp_dialog = true;
            }

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("Drop files anywhere to import")
                    .small()
                    .color(colors::TEXT_MUTED),
            );
        });
    app.show_welcome = open && !project_empty;
}
