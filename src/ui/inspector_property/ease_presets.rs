use crate::core::keyframe::{BezierControlPoint, InterpolationType};
use crate::core::property::Animatable;
use crate::ui::custom_widgets;
use crate::ui::kui;
use crate::ui::theme::colors;
use eframe::egui;

/// Draw keyframe ease presets & smart curve selector for properties
pub fn draw_easy_ease_button<T: Clone>(
    ui: &mut egui::Ui,
    property: &mut Animatable<T>,
    project_changed: &mut bool,
) {
    if ui.available_width() < 180.0 {
        return;
    }
    ui.horizontal_wrapped(|ui| {
        ui.add_space(8.0);
        if custom_widgets::ae_button(ui, "Easy Ease (F9)")
            .on_hover_text("Symmetrical Bezier Ease (F9)")
            .clicked()
        {
            if let Animatable::Animated(ref mut keyframes) = property {
                let coords = crate::core::keyframe::EasePreset::Standard.control_points();
                for kf in keyframes {
                    kf.interpolation = InterpolationType::Bezier {
                        outgoing: BezierControlPoint {
                            influence: 0.333,
                            speed: 0.0,
                        },
                        incoming: BezierControlPoint {
                            influence: 0.333,
                            speed: 0.0,
                        },
                        custom_bezier: Some(coords),
                    };
                }
                *project_changed = true;
            }
        }

        let combo_id = ui.make_persistent_id(("smart_ease_combo", ui.next_auto_id()));
        kui::combo_popup(ui, combo_id, "Smart Presets...", None, |ui| {
            for (preset, label, desc) in [
                (
                    crate::core::keyframe::EasePreset::Standard,
                    "Standard Ease",
                    "Symmetrical Smooth Ease",
                ),
                (
                    crate::core::keyframe::EasePreset::FastIn,
                    "Fast Acceleration",
                    "Sudden Speed Up",
                ),
                (
                    crate::core::keyframe::EasePreset::SmoothOut,
                    "Smooth Deceleration",
                    "Gentle Slow Down",
                ),
                (
                    crate::core::keyframe::EasePreset::Overshoot,
                    "Spring Overshoot",
                    "Bounce Back Effect",
                ),
                (
                    crate::core::keyframe::EasePreset::Sine,
                    "Sine Wave",
                    "Ultra Smooth Harmonic Ease",
                ),
                (
                    crate::core::keyframe::EasePreset::FastOut,
                    "Fast Out",
                    "Explosive Start, Quick Settle",
                ),
                (
                    crate::core::keyframe::EasePreset::SlowIn,
                    "Slow In",
                    "Gradual Gentle Acceleration",
                ),
                (
                    crate::core::keyframe::EasePreset::Elastic,
                    "Elastic",
                    "Rubber Band Elastic Motion",
                ),
                (
                    crate::core::keyframe::EasePreset::Bounce,
                    "Bounce",
                    "Ball Bounce Impact",
                ),
                (
                    crate::core::keyframe::EasePreset::Cycle,
                    "Cycle",
                    "Looping Rhythmic Ease",
                ),
                (
                    crate::core::keyframe::EasePreset::MirrorEase2,
                    "Mirror",
                    "Symmetrical Back and Forth",
                ),
                (
                    crate::core::keyframe::EasePreset::EaseIn,
                    "Quadratic In",
                    "Classic Quadratic Acceleration",
                ),
                (
                    crate::core::keyframe::EasePreset::EaseOut,
                    "Quadratic Out",
                    "Classic Quadratic Deceleration",
                ),
            ] {
                ui.horizontal(|ui| {
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(24.0, 16.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 2.0, colors::BG_DEEPEST);

                    let pts = preset.control_points();
                    let p0 = egui::pos2(rect.left() + 2.0, rect.bottom() - 2.0);
                    let p3 = egui::pos2(rect.right() - 2.0, rect.top() + 2.0);
                    let p1 = egui::pos2(
                        rect.left() + 2.0 + pts[0] * (rect.width() - 4.0),
                        rect.bottom() - 2.0 - pts[1] * (rect.height() - 4.0),
                    );
                    let p2 = egui::pos2(
                        rect.left() + 2.0 + pts[2] * (rect.width() - 4.0),
                        rect.bottom() - 2.0 - pts[3] * (rect.height() - 4.0),
                    );

                    let mut curve_pts = Vec::with_capacity(11);
                    for step in 0..=10 {
                        let t = step as f32 / 10.0;
                        let inv_t = 1.0 - t;
                        let x = inv_t.powi(3) * p0.x
                            + 3.0 * inv_t.powi(2) * t * p1.x
                            + 3.0 * inv_t * t.powi(2) * p2.x
                            + t.powi(3) * p3.x;
                        let y = inv_t.powi(3) * p0.y
                            + 3.0 * inv_t.powi(2) * t * p1.y
                            + 3.0 * inv_t * t.powi(2) * p2.y
                            + t.powi(3) * p3.y;
                        curve_pts.push(egui::pos2(x, y));
                    }
                    for window in curve_pts.windows(2) {
                        ui.painter().line_segment(
                            [window[0], window[1]],
                            egui::Stroke::new(1.5_f32, colors::ACCENT_CYAN),
                        );
                    }

                    if kui::menu_item(ui, label).on_hover_text(desc).clicked() {
                        if let Animatable::Animated(ref mut keyframes) = property {
                            for kf in keyframes {
                                kf.interpolation = InterpolationType::Bezier {
                                    outgoing: BezierControlPoint {
                                        influence: 0.333,
                                        speed: 0.0,
                                    },
                                    incoming: BezierControlPoint {
                                        influence: 0.333,
                                        speed: 0.0,
                                    },
                                    custom_bezier: Some(pts),
                                };
                            }
                            *project_changed = true;
                            ui.close_menu();
                        }
                    }
                });
            }
        });

        if custom_widgets::ae_button(ui, "Physics Spring")
            .on_hover_text("Apply Physics-based Overshoot & Spring Dynamics")
            .clicked()
        {
            if let Animatable::Animated(ref mut keyframes) = property {
                let spring_bezier = [0.175, 0.885, 0.32, 1.275];
                for kf in keyframes {
                    kf.interpolation = InterpolationType::Bezier {
                        outgoing: BezierControlPoint {
                            influence: 0.25,
                            speed: 0.0,
                        },
                        incoming: BezierControlPoint {
                            influence: 0.75,
                            speed: 0.0,
                        },
                        custom_bezier: Some(spring_bezier),
                    };
                }
                *project_changed = true;
            }
        }

        if custom_widgets::ae_button(ui, "⏸ Hold")
            .on_hover_text(
                "Toggle Toggle Hold Keyframe (Cmd+Opt+H): Values step discretely at keyframes",
            )
            .clicked()
        {
            if let Animatable::Animated(ref mut keyframes) = property {
                for kf in keyframes {
                    kf.interpolation = InterpolationType::Hold;
                }
                *project_changed = true;
            }
        }

        if custom_widgets::ae_button(ui, "Linear")
            .on_hover_text("Linear Keyframe: Values interpolate smoothly at constant speed")
            .clicked()
        {
            if let Animatable::Animated(ref mut keyframes) = property {
                for kf in keyframes {
                    kf.interpolation = InterpolationType::Linear;
                }
                *project_changed = true;
            }
        }

        if custom_widgets::ae_button(ui, "⏩ 2x Speed")
            .on_hover_text("Compress keyframe duration by 50%")
            .clicked()
        {
            if let Animatable::Animated(ref mut keyframes) = property {
                if let Some(first_kf) = keyframes.first() {
                    let start_f = first_kf.frame;
                    for kf in keyframes {
                        let offset = kf.frame - start_f;
                        kf.frame = start_f + (offset as f32 * 0.5).round() as u32;
                    }
                    *project_changed = true;
                }
            }
        }

        if custom_widgets::ae_button(ui, "⏪ 0.5x Speed")
            .on_hover_text("Stretch keyframe duration by 200%")
            .clicked()
        {
            if let Animatable::Animated(ref mut keyframes) = property {
                if let Some(first_kf) = keyframes.first() {
                    let start_f = first_kf.frame;
                    for kf in keyframes {
                        let offset = kf.frame - start_f;
                        kf.frame = start_f + (offset as f32 * 2.0).round() as u32;
                    }
                    *project_changed = true;
                }
            }
        }
    });
}
