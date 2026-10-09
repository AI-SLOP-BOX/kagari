mod layer_features;
mod media;
mod other;
mod shape;
mod text;

use crate::core::timeline::Layer;
use eframe::egui;

pub fn draw_layer_type_specs(
    ui: &mut egui::Ui,
    layer: &mut Layer,
    current_frame: u32,
    project_changed: &mut bool,
    next_frame: &mut Option<u32>,
    comp_fps: f32,
) {
    egui::Frame::none()
        .inner_margin(egui::Margin::symmetric(0.0, 4.0))
        .show(ui, |ui| {
            ui.label("Layer Specs");

            // Dispatch to specialized submodules
            if !media::draw_media_specs(ui, layer, current_frame, project_changed, next_frame, comp_fps)
                && !text::draw_text_specs(ui, layer, current_frame, project_changed, next_frame)
                && !shape::draw_shape_specs(ui, layer, current_frame, project_changed, next_frame)
            {
                other::draw_other_specs(ui, layer, current_frame, project_changed, next_frame);
            }

            ui.separator();

            // Layer-wide composite features (Shape repeater, puppet, paint, trim, styles)
            layer_features::draw_layer_features(ui, layer, current_frame, project_changed, next_frame);
        });
}
