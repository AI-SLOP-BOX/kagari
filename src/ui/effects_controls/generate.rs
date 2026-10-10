use crate::core::timeline::EffectType;
use crate::ui::inspector_property::draw_property_ui;
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
        EffectType::FractalNoise {
            fractal_type,
            contrast,
            brightness,
            complexity,
            evolution,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Fractal Type",
                fractal_type,
                |ui, v| {
                    let options = [
                        (0.0_f32, "Fractal (fBm)"),
                        (1.0, "Turbulent"),
                        (2.0, "Dynamic"),
                        (3.0, "Ridge"),
                    ];
                    let mut selected = v.round().clamp(0.0, 3.0);
                    if crate::ui::kui::combo(ui, "fractal_noise_type", &mut selected, &options) {
                        *v = selected;
                    }
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Contrast",
                contrast,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=200.0, "%");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Brightness",
                brightness,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, -100.0..=100.0, "%");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Complexity",
                complexity,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 1.0..=10.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Evolution",
                evolution,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=360.0, "°");
                },
            );
        }
        EffectType::GodRays {
            sun_x,
            sun_y,
            samples,
            decay,
            weight,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Sun X",
                sun_x,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=1.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Sun Y",
                sun_y,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=1.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Samples",
                samples,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 1.0..=64.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Decay",
                decay,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.5..=1.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Weight",
                weight,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=2.0, "");
                },
            );
        }
        EffectType::StarField {
            num_stars,
            depth_speed,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Stars",
                num_stars,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 1.0..=2000.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Depth Speed",
                depth_speed,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=10.0, "");
                },
            );
        }
        EffectType::LightningArc {
            start_x,
            start_y,
            end_x,
            end_y,
            seed,
            glow,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Start X",
                start_x,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=1.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Start Y",
                start_y,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=1.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "End X",
                end_x,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=1.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "End Y",
                end_y,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=1.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Seed",
                seed,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=9999.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Glow",
                glow,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=5.0, "");
                },
            );
        }
        EffectType::LaserBeam {
            start_x,
            start_y,
            end_x,
            end_y,
            progress,
            length,
            starting_thickness,
            ending_thickness,
            core_color,
            glow_color,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Start X",
                start_x,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=1.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Start Y",
                start_y,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=1.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "End X",
                end_x,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=1.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "End Y",
                end_y,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=1.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Progress",
                progress,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=1.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Length %",
                length,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 1.0..=100.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Start Thickness",
                starting_thickness,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.5..=50.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "End Thickness",
                ending_thickness,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.5..=50.0, "");
                },
            );
            let c_before = core_color.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Core Color", core_color, |ui, val| {
                    crate::ui::kui::color_rgba(ui, val);
                })
            {
                *next_frame = Some(nf);
            }
            if c_before != *core_color {
                *project_changed = true;
            }

            let g_before = glow_color.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Glow Color", glow_color, |ui, val| {
                    crate::ui::kui::color_rgba(ui, val);
                })
            {
                *next_frame = Some(nf);
            }
            if g_before != *glow_color {
                *project_changed = true;
            }
        }
        EffectType::FireAutomaton { intensity } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Intensity",
                intensity,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=10.0, "");
                },
            );
        }
        EffectType::LightLeak {
            pos_x,
            pos_y,
            intensity,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Position X",
                pos_x,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=1.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Position Y",
                pos_y,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=1.0, "");
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
                    crate::ui::kui::slider(ui, v, 0.0..=3.0, "");
                },
            );
        }
        EffectType::PerlinFlow { scale } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Scale",
                scale,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.5..=20.0, "");
                },
            );
        }
        EffectType::FbmTurbulence { octaves, amplitude } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Octaves",
                octaves,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 1.0..=8.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Amplitude",
                amplitude,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=255.0, "");
                },
            );
        }
        _ => {}
    }
}
