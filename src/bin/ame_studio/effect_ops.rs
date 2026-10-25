use super::*;

impl AfterEffectsStudio {
    pub(crate) fn add_effect(&mut self, kind: &str) {
        let Some(layer_id) = self.selected_layer_id.clone() else {
            return;
        };
        use kagari_vfx::core::timeline::{Animatable, EffectType};
        let (name, effect_type) = match kind {
            "blur" => (
                "Gaussian Blur",
                EffectType::GaussianBlur {
                    blur_radius: Animatable::new_constant(12.0),
                },
            ),
            "tint" => (
                "Color Tint",
                EffectType::ColorTint {
                    color: Animatable::new_constant([1.0, 0.45, 0.12, 1.0]),
                    intensity: Animatable::new_constant(50.0),
                },
            ),
            "shadow" => (
                "Drop Shadow",
                EffectType::DropShadow {
                    color: Animatable::new_constant([0.0, 0.0, 0.0, 1.0]),
                    opacity: Animatable::new_constant(65.0),
                    direction: Animatable::new_constant(135.0),
                    distance: Animatable::new_constant(12.0),
                    softness: Animatable::new_constant(8.0),
                },
            ),
            "glow" => (
                "Glow",
                EffectType::Glow {
                    threshold: Animatable::new_constant(65.0),
                    radius: Animatable::new_constant(12.0),
                    intensity: Animatable::new_constant(60.0),
                    color: Animatable::new_constant([1.0, 0.52, 0.2, 1.0]),
                },
            ),
            "vignette" => (
                "Vignette",
                EffectType::Vignette {
                    intensity: Animatable::new_constant(40.0),
                    roundness: Animatable::new_constant(1.0),
                    feather: Animatable::new_constant(70.0),
                    color: Animatable::new_constant([0.0, 0.0, 0.0, 1.0]),
                },
            ),
            "hue_saturation" => (
                "Hue / Saturation",
                EffectType::HueSaturation {
                    hue_shift: Animatable::new_constant(0.0),
                    saturation: Animatable::new_constant(0.0),
                    lightness: Animatable::new_constant(0.0),
                },
            ),
            "levels" => (
                "Levels",
                EffectType::Levels {
                    input_black: Animatable::new_constant(0.0),
                    input_white: Animatable::new_constant(1.0),
                    gamma: Animatable::new_constant(1.0),
                    output_black: Animatable::new_constant(0.0),
                    output_white: Animatable::new_constant(1.0),
                },
            ),
            "motion_blur" => (
                "Motion Blur",
                EffectType::MotionBlur {
                    shutter_angle: Animatable::new_constant(180.0),
                    samples: 8,
                },
            ),
            "film_grain" => (
                "Film Grain",
                EffectType::FilmGrain {
                    intensity: Animatable::new_constant(20.0),
                    grain_size: 1.0,
                    color_film: false,
                },
            ),
            "posterize" => (
                "Posterize",
                EffectType::Posterize {
                    levels: Animatable::new_constant(6.0),
                },
            ),
            "directional_blur" => (
                "Directional Blur",
                EffectType::DirectionalBlur {
                    angle: Animatable::new_constant(0.0),
                    length: Animatable::new_constant(12.0),
                },
            ),
            "radial_blur" => (
                "Radial Blur",
                EffectType::RadialBlur {
                    amount: Animatable::new_constant(0.2),
                },
            ),
            "sharpen" => (
                "Sharpen",
                EffectType::Sharpen {
                    amount: Animatable::new_constant(0.5),
                },
            ),
            "twirl" => (
                "Twirl",
                EffectType::Twirl {
                    angle: Animatable::new_constant(90.0),
                    radius: Animatable::new_constant(0.5),
                },
            ),
            "bulge" => (
                "Bulge",
                EffectType::Bulge {
                    amount: Animatable::new_constant(0.5),
                    radius: Animatable::new_constant(0.5),
                },
            ),
            "invert" => (
                "Invert",
                EffectType::Invert {
                    invert_alpha: false,
                },
            ),
            "offset" => (
                "Offset",
                EffectType::Offset {
                    shift_x: Animatable::new_constant(0.0),
                    shift_y: Animatable::new_constant(0.0),
                },
            ),
            "threshold" => (
                "Threshold",
                EffectType::Threshold {
                    threshold: Animatable::new_constant(0.5),
                },
            ),
            "linear_wipe" => (
                "Linear Wipe",
                EffectType::LinearWipe {
                    completion: Animatable::new_constant(0.0),
                    angle: Animatable::new_constant(0.0),
                },
            ),
            "spherize" => (
                "Spherize",
                EffectType::Spherize {
                    radius: Animatable::new_constant(0.5),
                    refractive_index: Animatable::new_constant(1.0),
                },
            ),
            "vibrance" => (
                "Vibrance",
                EffectType::Vibrance {
                    amount: Animatable::new_constant(25.0),
                },
            ),
            "white_balance" => (
                "White Balance",
                EffectType::WhiteBalance {
                    temperature: Animatable::new_constant(0.0),
                    tint: Animatable::new_constant(0.0),
                },
            ),
            _ => return,
        };
        let id = format!(
            "ameui_effect_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or_default()
        );
        let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == layer_id && !layer.locked)
        else {
            return;
        };
        layer.effects.push(kagari_vfx::core::timeline::Effect {
            id: id.clone(),
            name: name.to_owned(),
            effect_type,
            enabled: true,
        });
        self.selected_effect_id = Some(id);
        self.mark_project_changed();
    }

    pub(crate) fn move_effect(&mut self, effect_id: &str, direction: isize) {
        let Some(layer_id) = self.selected_layer_id.as_deref() else {
            return;
        };
        let composition = self.project_model.active_composition_mut();
        let Some(layer) = composition
            .layers
            .iter_mut()
            .find(|layer| layer.id == layer_id && !layer.locked)
        else {
            return;
        };
        if layer.locked {
            return;
        }
        let Some(index) = layer
            .effects
            .iter()
            .position(|effect| effect.id == effect_id)
        else {
            return;
        };
        let next = (index as isize + direction).clamp(0, layer.effects.len() as isize - 1) as usize;
        if index == next {
            return;
        }
        layer.effects.swap(index, next);
        self.selected_effect_id = Some(effect_id.to_owned());
        self.mark_project_changed();
        self.sync_bindings_logged();
    }

    pub(crate) fn update_selected_effect_parameter(&mut self, parameter: &str, input: &str) {
        let Some(value) = input
            .trim()
            .parse::<f32>()
            .ok()
            .filter(|value| value.is_finite())
        else {
            return;
        };
        let (Some(layer_id), Some(effect_id)) = (
            self.selected_layer_id.clone(),
            self.selected_effect_id.clone(),
        ) else {
            return;
        };
        let frame = self.current_frame;
        let changed =
            self.project_model
                .active_composition_mut()
                .layers
                .iter_mut()
                .find(|layer| layer.id == layer_id && !layer.locked)
                .and_then(|layer| {
                    layer
                        .effects
                        .iter_mut()
                        .find(|effect| effect.id == effect_id)
                })
                .and_then(|effect| {
                    let explicit =
                        match &mut effect.effect_type {
                            kagari_vfx::core::timeline::EffectType::GaussianBlur {
                                blur_radius,
                            } if parameter == "blur_radius" => Some(set_animatable_at_frame(
                                blur_radius,
                                frame,
                                value.clamp(0.0, 256.0),
                            )),
                            kagari_vfx::core::timeline::EffectType::ColorTint {
                                intensity, ..
                            } if parameter == "intensity" => Some(set_animatable_at_frame(
                                intensity,
                                frame,
                                value.clamp(0.0, 100.0),
                            )),
                            kagari_vfx::core::timeline::EffectType::DropShadow {
                                opacity,
                                direction,
                                distance,
                                softness,
                                ..
                            } => match parameter {
                                "opacity" => Some(set_animatable_at_frame(
                                    opacity,
                                    frame,
                                    value.clamp(0.0, 100.0),
                                )),
                                "direction" => Some(set_animatable_at_frame(
                                    direction,
                                    frame,
                                    value.clamp(-360.0, 360.0),
                                )),
                                "distance" => Some(set_animatable_at_frame(
                                    distance,
                                    frame,
                                    value.clamp(0.0, 1000.0),
                                )),
                                "softness" => Some(set_animatable_at_frame(
                                    softness,
                                    frame,
                                    value.clamp(0.0, 256.0),
                                )),
                                _ => None,
                            },
                            kagari_vfx::core::timeline::EffectType::Vignette {
                                intensity,
                                roundness,
                                feather,
                                ..
                            } => match parameter {
                                "intensity" => Some(set_animatable_at_frame(
                                    intensity,
                                    frame,
                                    value.clamp(0.0, 100.0),
                                )),
                                "roundness" => Some(set_animatable_at_frame(
                                    roundness,
                                    frame,
                                    value.clamp(0.1, 10.0),
                                )),
                                "feather" => Some(set_animatable_at_frame(
                                    feather,
                                    frame,
                                    value.clamp(0.0, 100.0),
                                )),
                                _ => None,
                            },
                            kagari_vfx::core::timeline::EffectType::HueSaturation {
                                hue_shift,
                                saturation,
                                lightness,
                            } => match parameter {
                                "hue_shift" => Some(set_animatable_at_frame(
                                    hue_shift,
                                    frame,
                                    value.clamp(-180.0, 180.0),
                                )),
                                "saturation" => Some(set_animatable_at_frame(
                                    saturation,
                                    frame,
                                    value.clamp(-100.0, 100.0),
                                )),
                                "lightness" => Some(set_animatable_at_frame(
                                    lightness,
                                    frame,
                                    value.clamp(-100.0, 100.0),
                                )),
                                _ => None,
                            },
                            kagari_vfx::core::timeline::EffectType::Levels {
                                input_black,
                                input_white,
                                gamma,
                                output_black,
                                output_white,
                            } => match parameter {
                                "input_black" => Some(set_animatable_at_frame(
                                    input_black,
                                    frame,
                                    value.clamp(0.0, input_white.evaluate(frame).clamp(0.0, 1.0)),
                                )),
                                "input_white" => Some(set_animatable_at_frame(
                                    input_white,
                                    frame,
                                    value.clamp(input_black.evaluate(frame).clamp(0.0, 1.0), 1.0),
                                )),
                                "gamma" => Some(set_animatable_at_frame(
                                    gamma,
                                    frame,
                                    value.clamp(0.01, 10.0),
                                )),
                                "output_black" => Some(set_animatable_at_frame(
                                    output_black,
                                    frame,
                                    value.clamp(0.0, output_white.evaluate(frame).clamp(0.0, 1.0)),
                                )),
                                "output_white" => Some(set_animatable_at_frame(
                                    output_white,
                                    frame,
                                    value.clamp(output_black.evaluate(frame).clamp(0.0, 1.0), 1.0),
                                )),
                                _ => None,
                            },
                            kagari_vfx::core::timeline::EffectType::Glow {
                                threshold,
                                radius,
                                intensity,
                                ..
                            } => match parameter {
                                "threshold" => Some(set_animatable_at_frame(
                                    threshold,
                                    frame,
                                    value.clamp(0.0, 100.0),
                                )),
                                "radius" => Some(set_animatable_at_frame(
                                    radius,
                                    frame,
                                    value.clamp(0.0, 256.0),
                                )),
                                "intensity" => Some(set_animatable_at_frame(
                                    intensity,
                                    frame,
                                    value.clamp(0.0, 100.0),
                                )),
                                _ => None,
                            },
                            kagari_vfx::core::timeline::EffectType::MotionBlur {
                                shutter_angle,
                                ..
                            } if parameter == "shutter_angle" => Some(set_animatable_at_frame(
                                shutter_angle,
                                frame,
                                value.clamp(0.0, 720.0),
                            )),
                            kagari_vfx::core::timeline::EffectType::FilmGrain {
                                intensity, ..
                            } if parameter == "intensity" => Some(set_animatable_at_frame(
                                intensity,
                                frame,
                                value.clamp(0.0, 100.0),
                            )),
                            kagari_vfx::core::timeline::EffectType::Posterize { levels }
                                if parameter == "levels" =>
                            {
                                Some(set_animatable_at_frame(
                                    levels,
                                    frame,
                                    value.clamp(2.0, 256.0).round(),
                                ))
                            }
                            kagari_vfx::core::timeline::EffectType::DirectionalBlur {
                                angle,
                                length,
                            } => match parameter {
                                "angle" => Some(set_animatable_at_frame(
                                    angle,
                                    frame,
                                    value.clamp(-360.0, 360.0),
                                )),
                                "length" => Some(set_animatable_at_frame(
                                    length,
                                    frame,
                                    value.clamp(0.0, 256.0),
                                )),
                                _ => None,
                            },
                            kagari_vfx::core::timeline::EffectType::RadialBlur { amount }
                                if parameter == "amount" =>
                            {
                                Some(set_animatable_at_frame(
                                    amount,
                                    frame,
                                    value.clamp(0.0, 1.0),
                                ))
                            }
                            kagari_vfx::core::timeline::EffectType::Sharpen { amount }
                                if parameter == "amount" =>
                            {
                                Some(set_animatable_at_frame(
                                    amount,
                                    frame,
                                    value.clamp(0.0, 5.0),
                                ))
                            }
                            _ => None,
                        };
                    explicit.or_else(|| {
                        use kagari_vfx::core::effect_params::ParamRef;
                        effect.effect_type.animatable_params().into_iter().find_map(
                            |(id, param_ref)| {
                                if id != parameter {
                                    None
                                } else {
                                    match param_ref {
                                        ParamRef::Scalar(track) => {
                                            Some(set_animatable_at_frame(track, frame, value))
                                        }
                                        _ => None,
                                    }
                                }
                            },
                        )
                    })
                })
                .unwrap_or(false);
        if changed {
            self.mark_project_changed();
        }
    }

    pub(crate) fn update_selected_effect_color(&mut self, input: &str) {
        let (Some(layer_id), Some(effect_id)) = (
            self.selected_layer_id.clone(),
            self.selected_effect_id.clone(),
        ) else {
            return;
        };
        let frame = self.current_frame;
        let changed = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == layer_id && !layer.locked)
            .and_then(|layer| {
                layer
                    .effects
                    .iter_mut()
                    .find(|effect| effect.id == effect_id)
            })
            .and_then(|effect| {
                use kagari_vfx::core::timeline::EffectType;
                let track = match &mut effect.effect_type {
                    EffectType::ColorTint { color, .. }
                    | EffectType::DropShadow { color, .. }
                    | EffectType::Vignette { color, .. }
                    | EffectType::Glow { color, .. }
                    | EffectType::LensFlare { color, .. } => color,
                    _ => return None,
                };
                let current = track.evaluate(frame);
                let next = parse_hex_rgb(input, current[3])?;
                (current != next).then(|| set_animatable_at_frame(track, frame, next))
            })
            .unwrap_or(false);
        if changed {
            self.mark_project_changed();
        }
    }

    pub(crate) fn keyframe_selected_effect_parameter(&mut self, parameter_id: &str) {
        let (Some(layer_id), Some(effect_id)) = (
            self.selected_layer_id.as_deref(),
            self.selected_effect_id.as_deref(),
        ) else {
            return;
        };
        let frame = self.current_frame;
        let changed = self.project_model.active_composition_mut().layers.iter_mut()
            .find(|layer| layer.id == layer_id && !layer.locked)
            .and_then(|layer| layer.effects.iter_mut().find(|effect| effect.id == effect_id))
            .is_some_and(|effect| {
                use kagari_vfx::core::effect_params::ParamRef;
                let Some((_, parameter)) = effect.effect_type.animatable_params().into_iter()
                    .find(|(id, _)| *id == parameter_id) else {
                    return false;
                };
                match parameter {
                    ParamRef::Scalar(track) => add_current_keyframe(track, frame),
                    ParamRef::Vec2(track) => add_current_keyframe(track, frame),
                    ParamRef::Vec3(track) => add_current_keyframe(track, frame),
                    ParamRef::Vec4Color(track) => add_current_keyframe(track, frame),
                }
                true
            });
        if changed {
            self.mark_project_changed();
        }
    }
}
