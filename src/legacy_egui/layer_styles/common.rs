//! Color and slider UI helpers for layer style controls.

use crate::ui::theme::colors;
use eframe::egui;

pub fn color_row(ui: &mut egui::Ui, label: &str, color: &mut [f32; 4], changed: &mut bool) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(label)
                .small()
                .color(colors::TEXT_SECONDARY),
        );
        let mut col = egui::Color32::from_rgba_premultiplied(
            (color[0] * 255.0) as u8,
            (color[1] * 255.0) as u8,
            (color[2] * 255.0) as u8,
            (color[3] * 255.0) as u8,
        );
        if ui.color_edit_button_srgba(&mut col).changed() {
            let [r, g, b, a] = col.to_array();
            *color = [
                r as f32 / 255.0,
                g as f32 / 255.0,
                b as f32 / 255.0,
                a as f32 / 255.0,
            ];
            *changed = true;
        }
    });
}
