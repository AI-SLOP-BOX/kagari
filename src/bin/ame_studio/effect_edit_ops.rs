use super::*;

impl AfterEffectsStudio {
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
            .is_some_and(|effect| {
                use kagari_vfx::core::effect_params::ParamRef;
                let Some((_, parameter)) = effect
                    .effect_type
                    .animatable_params()
                    .into_iter()
                    .find(|(id, _)| *id == parameter_id)
                else {
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
