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
        EffectType::DropShadow {
            color,
            opacity,
            direction,
            distance,
            softness,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Shadow Color",
                color,
                |ui, val| {
                    crate::ui::kui::color_rgba(ui, val);
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Opacity",
                opacity,
                |ui, val| {
                    crate::ui::kui::slider(ui, val, 0.0..=100.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Direction",
                direction,
                |ui, val| {
                    crate::ui::kui::slider(ui, val, 0.0..=360.0, "°");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Distance",
                distance,
                |ui, val| {
                    crate::ui::kui::slider(ui, val, 0.0..=100.0, " px");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Softness",
                softness,
                |ui, val| {
                    crate::ui::kui::slider(ui, val, 0.0..=100.0, "");
                },
            );
        }
        EffectType::ChromaticAberration {
            shift_r,
            shift_b,
            edge_falloff,
            iris_linked: _,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Red Shift",
                shift_r,
                |ui, val| {
                    crate::ui::kui::slider(ui, val, 0.0..=20.0, " px");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Blue Shift",
                shift_b,
                |ui, val| {
                    crate::ui::kui::slider(ui, val, 0.0..=20.0, " px");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Edge Falloff",
                edge_falloff,
                |ui, val| {
                    crate::ui::kui::slider(ui, val, 0.0..=1.0, "");
                },
            );
        }
        EffectType::Vignette {
            intensity,
            roundness,
            feather,
            color,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Intensity",
                intensity,
                |ui, val| {
                    crate::ui::kui::slider(ui, val, 0.0..=100.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Roundness",
                roundness,
                |ui, val| {
                    crate::ui::kui::slider(ui, val, 0.0..=1.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Feather",
                feather,
                |ui, val| {
                    crate::ui::kui::slider(ui, val, 0.0..=100.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Color",
                color,
                |ui, val| {
                    crate::ui::kui::color_rgba(ui, val);
                },
            );
        }
        EffectType::Glow {
            threshold,
            radius,
            intensity,
            color,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Threshold",
                threshold,
                |ui, val| {
                    crate::ui::kui::slider(ui, val, 0.0..=100.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Radius",
                radius,
                |ui, val| {
                    crate::ui::kui::slider(ui, val, 0.0..=200.0, " px");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Intensity",
                intensity,
                |ui, val| {
                    crate::ui::kui::slider(ui, val, 0.0..=100.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Glow Color",
                color,
                |ui, val| {
                    crate::ui::kui::color_rgba(ui, val);
                },
            );
        }
        EffectType::LensFlare {
            enabled,
            position_x,
            position_y,
            intensity,
            threshold,
            color,
            ..
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Enabled",
                enabled,
                |ui, val| {
                    crate::ui::kui::slider_without_value(ui, val, 0.0..=1.0);
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Position X",
                position_x,
                |ui, val| {
                    crate::ui::kui::slider(ui, val, 0.0..=1.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Position Y",
                position_y,
                |ui, val| {
                    crate::ui::kui::slider(ui, val, 0.0..=1.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Intensity",
                intensity,
                |ui, val| {
                    crate::ui::kui::slider(ui, val, 0.0..=5.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Threshold",
                threshold,
                |ui, val| {
                    crate::ui::kui::slider(ui, val, 0.0..=2.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Flare Color",
                color,
                |ui, val| {
                    crate::ui::kui::color_rgba(ui, val);
                },
            );
        }
        EffectType::GlowPro {
            threshold,
            radius,
            intensity,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Threshold",
                threshold,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=1.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Radius",
                radius,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=128.0, " px");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Intensity",
                intensity,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=4.0, "");
                },
            );
        }
        EffectType::CrtScanlines {
            line_spacing,
            intensity,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Line Spacing",
                line_spacing,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 1.0..=50.0, " px");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Intensity",
                intensity,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=1.0, "");
                },
            );
        }
        EffectType::NightVision { amplification } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Amplification",
                amplification,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 1.0..=8.0, "×");
                },
            );
        }
        EffectType::Emboss { angle_deg, depth } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Angle",
                angle_deg,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, -180.0..=180.0, "°");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Depth",
                depth,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=10.0, "");
                },
            );
        }
        EffectType::BevelAlpha {
            depth,
            light_angle_deg,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Depth",
                depth,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 1.0..=32.0, " px");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Light Angle",
                light_angle_deg,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=360.0, "°");
                },
            );
        }
        EffectType::ReflectionMap {
            reflect_y,
            fade_dist,
            opacity,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Horizon Y",
                reflect_y,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=2000.0, " px");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Fade Distance",
                fade_dist,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 1.0..=1000.0, " px");
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
                    crate::ui::kui::slider(ui, v, 0.0..=1.0, "");
                },
            );
        }
        EffectType::OpticalFlares {
            position: _,
            brightness,
            scale,
        } => {
            ui.label("Optical Flares");
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Brightness",
                brightness,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=5.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Scale",
                scale,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.1..=5.0, "");
                },
            );
        }
        EffectType::StrobeLight {
            interval_frames,
            strobe_color,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Interval (frames)",
                interval_frames,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 1.0..=60.0, " frames");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Strobe Color",
                strobe_color,
                |ui, v| {
                    ui.color_edit_button_rgba_unmultiplied(v);
                },
            );
        }
        _ => {}
    }
}
