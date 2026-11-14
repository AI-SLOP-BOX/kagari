use super::*;

pub(super) fn return_to_home(app: &mut KagariApp, ctx: &egui::Context) {
    app.show_home = true;
    crate::ui::home_screen::set_home_nav(ctx, crate::ui::home_screen::HomeNav::Home);
}

pub(super) fn settings_card<R>(
    ui: &mut egui::Ui,
    title: &str,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let card_width = ui.available_width();
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(14, 21, 24))
        .stroke(egui::Stroke::new(
            1.0_f32,
            egui::Color32::from_rgb(35, 44, 50),
        ))
        .rounding(egui::Rounding::same(6.0))
        .inner_margin(egui::Margin::symmetric(10.0, 2.0))
        .show(ui, |ui| {
            ui.set_width((card_width - 20.0).max(0.0));
            ui.vertical(|ui| {
                if !title.is_empty() {
                    crate::ui::kui::section(ui, &crate::ui::localization::tr(title));
                    ui.add_space(4.0);
                }
                add_contents(ui)
            })
            .inner
        })
        .inner
}

pub(super) fn preference_toggle_row(
    ui: &mut egui::Ui,
    label: &str,
    description: &str,
    value: &mut bool,
) {
    ui.horizontal(|ui| {
        let label_width = (ui.available_width() - 60.0).max(100.0);
        ui.allocate_ui_with_layout(
            egui::vec2(label_width, 36.0),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.label(
                    egui::RichText::new(crate::ui::localization::tr(label))
                        .size(14.0)
                        .color(colors::TEXT_PRIMARY),
                );
                ui.label(
                    egui::RichText::new(crate::ui::localization::tr(description))
                        .size(10.5)
                        .color(colors::TEXT_SECONDARY),
                );
            },
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            kui::switch(ui, value, "");
        });
    });
    ui.add_space(1.0);
    kui::separator(ui);
    ui.add_space(1.0);
}

pub(super) fn preference_property_row<R>(
    ui: &mut egui::Ui,
    label: &str,
    description: &str,
    control: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let result = ui
        .horizontal(|ui| {
            let label_width = (ui.available_width() - 250.0).max(180.0);
            ui.allocate_ui_with_layout(
                egui::vec2(label_width, 38.0),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.label(
                        egui::RichText::new(crate::ui::localization::tr(label))
                            .size(14.0)
                            .color(colors::TEXT_PRIMARY),
                    );
                    ui.label(
                        egui::RichText::new(crate::ui::localization::tr(description))
                            .size(10.5)
                            .color(colors::TEXT_SECONDARY),
                    );
                },
            );
            let field_width = 240.0_f32.min(ui.available_width());
            ui.add_space((ui.available_width() - field_width).max(0.0));
            ui.allocate_ui_with_layout(
                egui::vec2(field_width, 38.0),
                egui::Layout::left_to_right(egui::Align::Center),
                control,
            )
            .inner
        })
        .inner;
    ui.add_space(1.0);
    kui::separator(ui);
    ui.add_space(1.0);
    result
}

pub(super) fn status_chip(ui: &mut egui::Ui, label: &str, value: &str) {
    egui::Frame::none()
        .fill(colors::BG_MID)
        .stroke(egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE))
        .rounding(egui::Rounding::same(6.0))
        .inner_margin(egui::Margin::symmetric(10.0, 6.0))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                kui::caption(ui, label);
                ui.label(
                    egui::RichText::new(value)
                        .strong()
                        .color(colors::TEXT_PRIMARY),
                );
            });
        });
}

pub(super) fn format_bytes(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.1}", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.0} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.0} KB", bytes as f64 / 1024.0)
    }
}
