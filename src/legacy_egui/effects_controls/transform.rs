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
        EffectType::Offset { shift_x, shift_y } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Offset X",
                shift_x,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, -300.0..=300.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Offset Y",
                shift_y,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, -300.0..=300.0, "");
                },
            );
        }
        EffectType::MotionTile {
            tile_center,
            tile_width,
            tile_height,
            output_width,
            output_height,
            mirror_edges,
            phase,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Tile Center",
                tile_center,
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
                "Tile Width",
                tile_width,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 1.0..=500.0, " %");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Tile Height",
                tile_height,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 1.0..=500.0, " %");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Output Width",
                output_width,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 100.0..=1000.0, " %");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Output Height",
                output_height,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 100.0..=1000.0, " %");
                },
            );
            if crate::ui::kui::toggle(ui, mirror_edges, "Mirror Edges").changed() {
                *project_changed = true;
            }
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Phase",
                phase,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, -360.0..=360.0, " °");
                },
            );
        }
        EffectType::Transform {
            anchor_point,
            position,
            scale_width,
            scale_height,
            uniform_scale,
            skew_deg,
            skew_axis_deg,
            rotation_deg,
            opacity,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Anchor Point",
                anchor_point,
                |ui, val| {
                    let _ = crate::ui::kui::vector_axes(
                        ui,
                        val,
                        f32::MIN..=f32::MAX,
                        1.0,
                        "",
                        ["X", "Y"],
                    );
                },
            );

            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Position",
                position,
                |ui, val| {
                    let _ = crate::ui::kui::vector_axes(
                        ui,
                        val,
                        f32::MIN..=f32::MAX,
                        1.0,
                        "",
                        ["X", "Y"],
                    );
                },
            );

            if crate::ui::kui::toggle(ui, uniform_scale, "Uniform Scale").changed() {
                *project_changed = true;
            }
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                if *uniform_scale {
                    "Scale"
                } else {
                    "Scale Width"
                },
                scale_width,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=1000.0, "%");
                },
            );
            if !*uniform_scale {
                draw_prop(
                    ui,
                    current_frame,
                    project_changed,
                    next_frame,
                    "Scale Height",
                    scale_height,
                    |ui, v| {
                        crate::ui::kui::slider(ui, v, 0.0..=1000.0, "%");
                    },
                );
            }
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Skew",
                skew_deg,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, -85.0..=85.0, "°");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Skew Axis",
                skew_axis_deg,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=360.0, "°");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Rotation",
                rotation_deg,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, -3600.0..=3600.0, "°");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Opacity",
                opacity,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=100.0, "%");
                },
            );
        }
        EffectType::DisplacementMap {
            source_layer,
            max_horizontal,
            max_vertical,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Source Layer ID",
                source_layer,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=10.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Max Horizontal",
                max_horizontal,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, -200.0..=200.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Max Vertical",
                max_vertical,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, -200.0..=200.0, "");
                },
            );
        }
        _ => {}
    }
}
