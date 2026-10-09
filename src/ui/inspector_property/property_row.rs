use crate::core::keyframe::{BezierControlPoint, InterpolationType, Keyframe};
use crate::core::property::Animatable;
use crate::ui::custom_widgets;
use crate::ui::kui;
use crate::ui::theme::colors;
use eframe::egui;

pub fn draw_property_ui<
    T: Clone + crate::core::property::Interpolate + PartialEq + std::fmt::Debug + 'static,
>(
    current_frame: u32,
    ui: &mut egui::Ui,
    label: &str,
    property: &mut Animatable<T>,
    draw_value_widget: impl FnOnce(&mut egui::Ui, &mut T),
) -> Option<u32> {
    let mut next_frame = None;
    let compact = ui.available_width() < 360.0;
    if compact {
        ui.label(label);
    }
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        if !compact {
            ui.label(label);
        }

        let has_keyframes = property.keyframes().is_some();

        let mut temp_val = property.evaluate(current_frame);
        draw_value_widget(ui, &mut temp_val);
        match property {
            Animatable::Constant(val) => {
                if *val != temp_val {
                    *val = temp_val.clone();
                }
            }
            Animatable::Animated(keyframes) => {
                let existing_idx = keyframes.iter().position(|kf| kf.frame == current_frame);
                if let Some(idx) = existing_idx {
                    keyframes[idx].value = temp_val.clone();
                } else {
                    let evaluated = property.evaluate(current_frame);
                    if temp_val != evaluated {
                        property.add_keyframe(Keyframe::new(
                            current_frame,
                            temp_val.clone(),
                            InterpolationType::Linear,
                        ));
                    }
                }
            }
        }

        if has_keyframes
            && custom_widgets::ae_svg_icon_button(
                ui,
                "previous-keyframe",
                crate::ui::icons::SVG_STEP_BACK,
                "Jump to Previous Keyframe (J)",
                colors::TEXT_SECONDARY,
            )
            .clicked()
        {
            if let Some(kfs) = property.keyframes() {
                if let Some(target) = kfs.iter().rev().find(|k| k.frame < current_frame) {
                    next_frame = Some(target.frame);
                }
            }
        }

        let has_keyframe_at_current = property
            .keyframes()
            .is_some_and(|kfs| kfs.iter().any(|k| k.frame == current_frame));

        let kf_tooltip = if !has_keyframes {
            "Enable Keyframes (Stopwatch)"
        } else if has_keyframe_at_current {
            "Remove Keyframe at Current Frame (Click) / Alt+Click to Remove All"
        } else {
            "Add Keyframe at Current Frame"
        };

        let kf_color = if has_keyframe_at_current {
            colors::ACCENT_BRAND
        } else if has_keyframes {
            colors::TEXT_SECONDARY
        } else {
            colors::TEXT_MUTED
        };

        if custom_widgets::ae_svg_icon_button(
            ui,
            "keyframe-toggle",
            crate::ui::icons::SVG_KEYFRAME,
            kf_tooltip,
            kf_color,
        )
        .clicked()
        {
            let alt_pressed = ui.input(|i| i.modifiers.alt);
            if !has_keyframes {
                let current_val = property.evaluate(current_frame);
                *property = Animatable::Animated(vec![Keyframe::new(
                    current_frame,
                    current_val,
                    InterpolationType::Linear,
                )]);
            } else if alt_pressed {
                let current_val = property.evaluate(current_frame);
                *property = Animatable::Constant(current_val);
            } else if has_keyframe_at_current {
                if let Animatable::Animated(ref mut kfs) = property {
                    kfs.retain(|k| k.frame != current_frame);
                    if kfs.is_empty() {
                        let current_val = temp_val;
                        *property = Animatable::Constant(current_val);
                    }
                }
            } else {
                property.add_keyframe(Keyframe::new(
                    current_frame,
                    temp_val,
                    InterpolationType::Linear,
                ));
            }
        }

        if has_keyframes {
            if custom_widgets::ae_svg_icon_button(
                ui,
                "next-keyframe",
                crate::ui::icons::SVG_STEP_FORWARD,
                "Jump to Next Keyframe (K)",
                colors::TEXT_SECONDARY,
            )
            .clicked()
            {
                if let Some(kfs) = property.keyframes() {
                    if let Some(target) = kfs.iter().find(|k| k.frame > current_frame) {
                        next_frame = Some(target.frame);
                    }
                }
            }
            kui::menu_button(ui, "Ease", |ui| {
                if kui::menu_item(ui, "Easy Ease (F9)").clicked() {
                    if let Animatable::Animated(ref mut kfs) = property {
                        let has_curr = kfs.iter().any(|k| k.frame == current_frame);
                        for kf in kfs {
                            if !has_curr || kf.frame == current_frame {
                                kf.interpolation = InterpolationType::Bezier {
                                    outgoing: BezierControlPoint {
                                        influence: 0.333,
                                        speed: 0.0,
                                    },
                                    incoming: BezierControlPoint {
                                        influence: 0.333,
                                        speed: 0.0,
                                    },
                                    custom_bezier: Some([0.333, 0.0, 0.333, 1.0]),
                                };
                            }
                        }
                    }
                    ui.close_menu();
                }
                if kui::menu_item(ui, "Ease In (Shift+F9)").clicked() {
                    if let Animatable::Animated(ref mut kfs) = property {
                        let has_curr = kfs.iter().any(|k| k.frame == current_frame);
                        for kf in kfs {
                            if !has_curr || kf.frame == current_frame {
                                kf.interpolation = InterpolationType::Bezier {
                                    outgoing: BezierControlPoint {
                                        influence: 0.1,
                                        speed: 0.0,
                                    },
                                    incoming: BezierControlPoint {
                                        influence: 0.75,
                                        speed: 0.0,
                                    },
                                    custom_bezier: Some([0.75, 0.0, 1.0, 1.0]),
                                };
                            }
                        }
                    }
                    ui.close_menu();
                }
                if kui::menu_item(ui, "Ease Out (Cmd+Shift+F9)").clicked() {
                    if let Animatable::Animated(ref mut kfs) = property {
                        let has_curr = kfs.iter().any(|k| k.frame == current_frame);
                        for kf in kfs {
                            if !has_curr || kf.frame == current_frame {
                                kf.interpolation = InterpolationType::Bezier {
                                    outgoing: BezierControlPoint {
                                        influence: 0.75,
                                        speed: 0.0,
                                    },
                                    incoming: BezierControlPoint {
                                        influence: 0.1,
                                        speed: 0.0,
                                    },
                                    custom_bezier: Some([0.0, 0.0, 0.25, 1.0]),
                                };
                            }
                        }
                    }
                    ui.close_menu();
                }
                if kui::menu_item(ui, "Hold (Cmd+Opt+H)").clicked() {
                    if let Animatable::Animated(ref mut kfs) = property {
                        let has_curr = kfs.iter().any(|k| k.frame == current_frame);
                        for kf in kfs {
                            if !has_curr || kf.frame == current_frame {
                                kf.interpolation = InterpolationType::Hold;
                            }
                        }
                    }
                    ui.close_menu();
                }
                if kui::menu_item(ui, "Linear").clicked() {
                    if let Animatable::Animated(ref mut kfs) = property {
                        let has_curr = kfs.iter().any(|k| k.frame == current_frame);
                        for kf in kfs {
                            if !has_curr || kf.frame == current_frame {
                                kf.interpolation = InterpolationType::Linear;
                            }
                        }
                    }
                    ui.close_menu();
                }
            });
        }

        kui::menu_button(ui, "...", |ui| {
            ui.label(egui::RichText::new("Advanced property actions").strong());
            if kui::menu_item(ui, "Easy Ease (F9)").clicked() {
                property.easy_ease();
                ui.close_menu();
            }
            if kui::menu_item(ui, "Sine Wave Ease").clicked() {
                if let Animatable::Animated(ref mut kfs) = property {
                    for kf in kfs {
                        kf.interpolation = InterpolationType::Bezier {
                            outgoing: BezierControlPoint {
                                influence: 0.5,
                                speed: 0.0,
                            },
                            incoming: BezierControlPoint {
                                influence: 0.5,
                                speed: 0.0,
                            },
                            custom_bezier: Some([0.37, 0.0, 0.63, 1.0]),
                        };
                    }
                }
                ui.close_menu();
            }
        });
    });

    next_frame
}
