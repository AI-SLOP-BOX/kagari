use crate::core::timeline::EffectType;
use eframe::egui;

use super::common::draw_prop;

pub fn draw(
    effect_type: &mut EffectType,
    ui: &mut egui::Ui,
    current_frame: u32,
    project_changed: &mut bool,
    next_frame: &mut Option<u32>,
) {
    match effect_type {
        EffectType::LinearWipe { completion, angle } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Completion",
                completion,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=100.0, "%");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Wipe Angle",
                angle,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=360.0, "°");
                },
            );
        }
        EffectType::IrisWipe { completion } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Completion",
                completion,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=100.0, "%");
                },
            );
        }
        EffectType::RadialWipe { completion } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Completion",
                completion,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=100.0, "%");
                },
            );
        }
        EffectType::VenetianBlinds { completion, width } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Completion",
                completion,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=100.0, "%");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Width",
                width,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 2.0..=100.0, " px");
                },
            );
        }
        EffectType::PageTurn {
            fold_position,
            fold_radius,
            fold_direction_deg,
            light_direction_deg,
            back_opacity,
            back_color,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Fold Position",
                fold_position,
                |ui, value| {
                    let _ = crate::ui::kui::vector_axes(
                        ui,
                        value,
                        -10000.0..=10000.0,
                        1.0,
                        " px",
                        ["X", "Y"],
                    );
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Fold Radius",
                fold_radius,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 10.0..=500.0, " px");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Fold Angle",
                fold_direction_deg,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, -180.0..=180.0, " °");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Light Direction",
                light_direction_deg,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, -180.0..=180.0, " °");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Back Opacity",
                back_opacity,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=100.0, " %");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Back Color",
                back_color,
                |ui, value| {
                    let _ = crate::ui::kui::color_rgba(ui, value);
                },
            );
        }
        EffectType::LightSweep {
            direction_deg,
            sweep_intensity,
            edge_intensity,
            ..
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Direction",
                direction_deg,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=360.0, "°");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Sweep Intensity",
                sweep_intensity,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=1.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Edge Intensity",
                edge_intensity,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=1.0, "");
                },
            );
        }
        _ => {}
    }
}
