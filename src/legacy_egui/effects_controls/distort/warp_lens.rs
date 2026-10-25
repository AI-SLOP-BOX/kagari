use crate::core::timeline::EffectType;
use crate::ui::inspector_property::draw_property_ui;
use eframe::egui;

use super::super::common::draw_prop;

pub fn draw_warp_lens(
    effect_type: &mut EffectType,
    ui: &mut egui::Ui,
    current_frame: u32,
    project_changed: &mut bool,
    next_frame: &mut Option<u32>,
) -> bool {
    match effect_type {
        EffectType::MeshWarp {
            top_left,
            top_right,
            bottom_left,
            bottom_right,
        } => {
            let tl_before = top_left.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Top Left Corner", top_left, |ui, val| {
                    crate::ui::kui::vector(ui, val, f32::MIN..=f32::MAX, 1.0, "");
                })
            {
                *next_frame = Some(nf);
            }
            if tl_before != *top_left {
                *project_changed = true;
            }

            let tr_before = top_right.clone();
            if let Some(nf) = draw_property_ui(
                current_frame,
                ui,
                "Top Right Corner",
                top_right,
                |ui, val| {
                    crate::ui::kui::vector(ui, val, f32::MIN..=f32::MAX, 1.0, "");
                },
            ) {
                *next_frame = Some(nf);
            }
            if tr_before != *top_right {
                *project_changed = true;
            }

            let bl_before = bottom_left.clone();
            if let Some(nf) = draw_property_ui(
                current_frame,
                ui,
                "Bottom Left Corner",
                bottom_left,
                |ui, val| {
                    crate::ui::kui::vector(ui, val, f32::MIN..=f32::MAX, 1.0, "");
                },
            ) {
                *next_frame = Some(nf);
            }
            if bl_before != *bottom_left {
                *project_changed = true;
            }

            let br_before = bottom_right.clone();
            if let Some(nf) = draw_property_ui(
                current_frame,
                ui,
                "Bottom Right Corner",
                bottom_right,
                |ui, val| {
                    crate::ui::kui::vector(ui, val, f32::MIN..=f32::MAX, 1.0, "");
                },
            ) {
                *next_frame = Some(nf);
            }
            if br_before != *bottom_right {
                *project_changed = true;
            }
            true
        }
        EffectType::CornerPin {
            top_left,
            top_right,
            bottom_right,
            bottom_left,
        } => {
            let tl_before = top_left.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Top Left Pin", top_left, |ui, val| {
                    crate::ui::kui::vector(ui, val, f32::MIN..=f32::MAX, 1.0, "");
                })
            {
                *next_frame = Some(nf);
            }
            if tl_before != *top_left {
                *project_changed = true;
            }

            let tr_before = top_right.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Top Right Pin", top_right, |ui, val| {
                    crate::ui::kui::vector(ui, val, f32::MIN..=f32::MAX, 1.0, "");
                })
            {
                *next_frame = Some(nf);
            }
            if tr_before != *top_right {
                *project_changed = true;
            }

            let br_before = bottom_right.clone();
            if let Some(nf) = draw_property_ui(
                current_frame,
                ui,
                "Bottom Right Pin",
                bottom_right,
                |ui, val| {
                    crate::ui::kui::vector(ui, val, f32::MIN..=f32::MAX, 1.0, "");
                },
            ) {
                *next_frame = Some(nf);
            }
            if br_before != *bottom_right {
                *project_changed = true;
            }

            let bl_before = bottom_left.clone();
            if let Some(nf) = draw_property_ui(
                current_frame,
                ui,
                "Bottom Left Pin",
                bottom_left,
                |ui, val| {
                    crate::ui::kui::vector(ui, val, f32::MIN..=f32::MAX, 1.0, "");
                },
            ) {
                *next_frame = Some(nf);
            }
            if bl_before != *bottom_left {
                *project_changed = true;
            }

            ui.label(
                egui::RichText::new("Pin corners in layer pixel space (animatable)")
                    .small()
                    .color(crate::ui::theme::colors::TEXT_SECONDARY),
            );
            true
        }
        EffectType::Twirl { angle, radius } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Twirl Angle",
                angle,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=360.0, "°");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Twirl Radius",
                radius,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 1.0..=300.0, "");
                },
            );
            true
        }
        EffectType::Bulge { amount, radius } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Bulge Amount",
                amount,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, -100.0..=100.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Bulge Radius",
                radius,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 1.0..=300.0, "");
                },
            );
            true
        }
        EffectType::Spherize {
            radius,
            refractive_index,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Radius",
                radius,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 1.0..=500.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Refractive Index",
                refractive_index,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.5..=2.0, "");
                },
            );
            true
        }
        EffectType::TurbulentDisplace {
            amount,
            size,
            evolution,
            complexity,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Amount",
                amount,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, -200.0..=200.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Size",
                size,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 2.0..=500.0, "");
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
            true
        }
        EffectType::WaveWarp {
            wave_height,
            wave_width,
            speed,
            direction_deg,
            ..
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Wave Height",
                wave_height,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=200.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Wave Width",
                wave_width,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 1.0..=500.0, "");
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
                    crate::ui::kui::slider(ui, v, 0.0..=20.0, "");
                },
            );
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
            true
        }
        EffectType::CcLens { convergence, zoom } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Convergence",
                convergence,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=200.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Zoom",
                zoom,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=5.0, "");
                },
            );
            true
        }
        EffectType::PolarCoordinates { interpolation, .. } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Interpolation",
                interpolation,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=100.0, "%");
                },
            );
            true
        }
        EffectType::OpticsCompensation {
            field_of_view_deg,
            zoom,
            ..
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "FOV",
                field_of_view_deg,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=180.0, "°");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Zoom",
                zoom,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=5.0, "");
                },
            );
            true
        }
        EffectType::RadialFastBlur { amount, .. } => {
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
            true
        }
        EffectType::BendIt {
            top_offset,
            bottom_offset,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Top Offset",
                top_offset,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, -50.0..=50.0, " px");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Bottom Offset",
                bottom_offset,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, -50.0..=50.0, " px");
                },
            );
            true
        }
        EffectType::VRHorizon {
            pitch_deg,
            yaw_deg,
            roll_deg,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Pitch (°)",
                pitch_deg,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, -180.0..=180.0, "°");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Yaw (°)",
                yaw_deg,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, -180.0..=180.0, "°");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Roll (°)",
                roll_deg,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, -180.0..=180.0, "°");
                },
            );
            true
        }
        EffectType::VRSphericalBlur { radius } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Blur Radius",
                radius,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=100.0, " px");
                },
            );
            true
        }
        EffectType::CCKaleida { sides } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Sides",
                sides,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 2.0..=64.0, "");
                },
            );
            true
        }
        EffectType::CCPixelPolly { shatter_progress } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Shatter Progress",
                shatter_progress,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=100.0, "%");
                },
            );
            true
        }
        _ => false,
    }
}
