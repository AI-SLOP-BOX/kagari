use crate::core::timeline::EffectType;
use crate::ui::inspector_property::draw_property_ui;
use eframe::egui;

use super::super::common::draw_prop;

pub fn draw_glitch_displace(
    effect_type: &mut EffectType,
    ui: &mut egui::Ui,
    current_frame: u32,
    project_changed: &mut bool,
    next_frame: &mut Option<u32>,
) -> bool {
    match effect_type {
        EffectType::Tiler { scale_percent, .. } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Scale",
                scale_percent,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 10.0..=500.0, "%");
                },
            );
            true
        }
        EffectType::Vortex { radius, angle_deg } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Radius",
                radius,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 1.0..=2000.0, " px");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Angle",
                angle_deg,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, -720.0..=720.0, "°");
                },
            );
            true
        }
        EffectType::HeatDistortion { strength, speed } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Strength",
                strength,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=30.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Speed",
                speed,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=5.0, "×");
                },
            );
            true
        }
        EffectType::RainRipples {
            drop_count,
            wave_strength,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Drop Count",
                drop_count,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=100.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Wave Strength",
                wave_strength,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=20.0, "");
                },
            );
            true
        }
        EffectType::Fisheye { strength } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Strength (− = pincushion)",
                strength,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, -1.0..=1.0, "");
                },
            );
            true
        }
        EffectType::LensCorrection { k1, k2 } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "K1 (Barrel + / Pincushion −)",
                k1,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, -0.5..=0.5, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "K2",
                k2,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, -0.5..=0.5, "");
                },
            );
            true
        }
        EffectType::GlitchDisplacement { seed, amount } => {
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
                "Amount",
                amount,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=10.0, "");
                },
            );
            true
        }
        EffectType::RGBSplit {
            red_offset,
            green_offset,
            blue_offset,
        } => {
            for (label, track) in [
                ("Red Offset", red_offset),
                ("Green Offset", green_offset),
                ("Blue Offset", blue_offset),
            ] {
                let before = track.clone();
                if let Some(nf) =
                    draw_property_ui(current_frame, ui, label, track, |ui, val: &mut [f32; 2]| {
                        crate::ui::kui::vector(ui, val, f32::MIN..=f32::MAX, 0.5, "");
                    })
                {
                    *next_frame = Some(nf);
                }
                if before != *track {
                    *project_changed = true;
                }
            }
            true
        }
        EffectType::Flicker {
            amount,
            speed,
            seed,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Amount",
                amount,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=1.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Speed",
                speed,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=60.0, "");
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
            true
        }
        EffectType::BlockGlitch {
            block_size,
            amount,
            seed,
            corruption,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Block Size",
                block_size,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 1.0..=256.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Amount",
                amount,
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
                "Corruption",
                corruption,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=1.0, "");
                },
            );
            true
        }
        EffectType::SliceTear {
            slices,
            max_offset,
            seed,
            ..
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Slices",
                slices,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=32.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Max Offset",
                max_offset,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=400.0, "");
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
            true
        }
        EffectType::PinchPunch { radius, amount } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Radius",
                radius,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 1.0..=2000.0, " px");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Amount (+pinch / −punch)",
                amount,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, -2.0..=2.0, "");
                },
            );
            true
        }
        EffectType::ScanlineGlitch {
            jitter_amount,
            seed,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Jitter",
                jitter_amount,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=50.0, " px");
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
            true
        }
        EffectType::GlassEdgeBevel {
            bevel_size,
            refraction,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Bevel Size",
                bevel_size,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 1.0..=64.0, " px");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Refraction",
                refraction,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=3.0, "");
                },
            );
            true
        }
        EffectType::RefractionLens { radius, ior } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Radius",
                radius,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 1.0..=2000.0, " px");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "IOR",
                ior,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 1.0..=3.0, "");
                },
            );
            true
        }
        _ => false,
    }
}
