use super::channel_ops::parse_effect_property;
use super::property::{GraphProperty, Scalar2dTrack, Vector2Track, Vector3Track};
use crate::core::timeline::Layer;

pub fn set_layer_interpolation(
    layer: &mut Layer,
    property: &str,
    interpolation: crate::core::keyframe::InterpolationType,
) -> bool {
    use crate::core::property::Animatable;

    fn apply<T: Clone>(
        track: &mut Animatable<T>,
        interpolation: crate::core::keyframe::InterpolationType,
    ) -> bool {
        if let Animatable::Animated(keyframes) = track {
            let changed = keyframes
                .iter()
                .any(|key| key.interpolation != interpolation);
            for key in keyframes {
                key.interpolation = interpolation;
            }
            return changed;
        }
        false
    }

    match GraphProperty::parse(property) {
        GraphProperty::Vector2 { track, .. } => match track {
            Vector2Track::Position => apply(&mut layer.transform.position, interpolation),
            Vector2Track::Scale => apply(&mut layer.transform.scale, interpolation),
        },
        GraphProperty::Scalar2d(track) => match track {
            Scalar2dTrack::Rotation => apply(&mut layer.transform.rotation, interpolation),
            Scalar2dTrack::Opacity => apply(&mut layer.transform.opacity, interpolation),
        },
        GraphProperty::Vector3 { track, .. } => match track {
            Vector3Track::Position => apply(&mut layer.transform_3d.position, interpolation),
            Vector3Track::Rotation => apply(&mut layer.transform_3d.rotation, interpolation),
            Vector3Track::Scale => apply(&mut layer.transform_3d.scale, interpolation),
        },
        GraphProperty::PuppetPin { id, .. } => layer
            .puppet_pins
            .iter_mut()
            .find(|pin| pin.id == id)
            .map(|pin| apply(&mut pin.position, interpolation))
            .unwrap_or(false),
        GraphProperty::Effect {
            effect_id,
            parameter: parameter_name,
            ..
        } => layer
            .effects
            .iter_mut()
            .find(|effect| effect.id == effect_id)
            .map(|effect| {
                effect
                    .effect_type
                    .set_parameter_keyframe_interpolation(Some(parameter_name), interpolation)
            })
            .unwrap_or(false),
        GraphProperty::Unknown(_) => false,
    }
}

pub fn map_layer_interpolation(
    layer: &mut Layer,
    property: &str,
    selected_frame: Option<u32>,
    mut map: impl FnMut(&mut crate::core::keyframe::InterpolationType),
) -> bool {
    use crate::core::property::Animatable;

    fn apply<T: Clone>(
        track: &mut Animatable<T>,
        selected_frame: Option<u32>,
        map: &mut impl FnMut(&mut crate::core::keyframe::InterpolationType),
    ) -> bool {
        let Some(keyframes) = track.keyframes_mut() else {
            return false;
        };
        let mut changed = false;
        for keyframe in keyframes {
            if selected_frame.is_some_and(|frame| frame != keyframe.frame) {
                continue;
            }
            let before = keyframe.interpolation;
            map(&mut keyframe.interpolation);
            changed |= before != keyframe.interpolation;
        }
        changed
    }

    match GraphProperty::parse(property) {
        GraphProperty::Vector2 { track, .. } => match track {
            Vector2Track::Position => {
                apply(&mut layer.transform.position, selected_frame, &mut map)
            }
            Vector2Track::Scale => apply(&mut layer.transform.scale, selected_frame, &mut map),
        },
        GraphProperty::Scalar2d(track) => match track {
            Scalar2dTrack::Rotation => {
                apply(&mut layer.transform.rotation, selected_frame, &mut map)
            }
            Scalar2dTrack::Opacity => apply(&mut layer.transform.opacity, selected_frame, &mut map),
        },
        GraphProperty::Vector3 { track, .. } => match track {
            Vector3Track::Position => {
                apply(&mut layer.transform_3d.position, selected_frame, &mut map)
            }
            Vector3Track::Rotation => {
                apply(&mut layer.transform_3d.rotation, selected_frame, &mut map)
            }
            Vector3Track::Scale => apply(&mut layer.transform_3d.scale, selected_frame, &mut map),
        },
        GraphProperty::PuppetPin { id, .. } => layer
            .puppet_pins
            .iter_mut()
            .find(|pin| pin.id == id)
            .map(|pin| apply(&mut pin.position, selected_frame, &mut map))
            .unwrap_or(false),
        GraphProperty::Effect {
            effect_id,
            parameter: parameter_name,
            ..
        } => {
            let Some(effect) = layer
                .effects
                .iter_mut()
                .find(|effect| effect.id == effect_id)
            else {
                return false;
            };
            effect
                .effect_type
                .animatable_params()
                .into_iter()
                .find_map(|(name, parameter)| {
                    if name != parameter_name {
                        return None;
                    }
                    Some(match parameter {
                        crate::core::effect_params::ParamRef::Scalar(track) => {
                            apply(track, selected_frame, &mut map)
                        }
                        crate::core::effect_params::ParamRef::Vec2(track) => {
                            apply(track, selected_frame, &mut map)
                        }
                        crate::core::effect_params::ParamRef::Vec3(track) => {
                            apply(track, selected_frame, &mut map)
                        }
                        crate::core::effect_params::ParamRef::Vec4Color(track) => {
                            apply(track, selected_frame, &mut map)
                        }
                    })
                })
                .unwrap_or(false)
        }
        GraphProperty::Unknown(_) => false,
    }
}

pub fn remove_effect_channel_at_frame(layer: &mut Layer, property: &str, frame: u32) -> bool {
    let Some((effect_id, parameter_name, component)) = parse_effect_property(property) else {
        return false;
    };
    if let Some(effect) = layer
        .effects
        .iter_mut()
        .find(|effect| effect.id == effect_id)
    {
        return match component {
            Some(_) => effect
                .effect_type
                .remove_parameter_component_keyframe(parameter_name, frame),
            None => effect
                .effect_type
                .remove_scalar_parameter_keyframe(parameter_name, frame),
        };
    }
    false
}

pub fn effect_parameter_channel_value(layer: &Layer, property: &str, frame: u32) -> f32 {
    let Some((effect_id, label, channel)) = parse_effect_property(property) else {
        return 0.0;
    };
    if let Some(effect) = layer.effects.iter().find(|effect| effect.id == effect_id) {
        for (name, parameter) in effect.effect_type.animatable_params_ref() {
            if name != label {
                continue;
            }
            return match parameter {
                crate::core::effect_params::ParamRefRef::Scalar(track) => track.evaluate(frame),
                crate::core::effect_params::ParamRefRef::Vec2(track) => {
                    track.evaluate(frame)[channel.unwrap_or(0).min(1)]
                }
                crate::core::effect_params::ParamRefRef::Vec3(track) => {
                    track.evaluate(frame)[channel.unwrap_or(0).min(2)]
                }
                crate::core::effect_params::ParamRefRef::Vec4Color(track) => {
                    track.evaluate(frame)[channel.unwrap_or(0).min(3)]
                }
            };
        }
    }
    0.0
}

pub fn effect_parameter_channel_keyframes(layer: &Layer, property: &str) -> Vec<(u32, f32)> {
    let Some((effect_id, label, channel)) = parse_effect_property(property) else {
        return vec![];
    };
    let channel = channel.unwrap_or(0);
    if let Some(effect) = layer.effects.iter().find(|effect| effect.id == effect_id) {
        for (name, parameter) in effect.effect_type.animatable_params_ref() {
            if name != label {
                continue;
            }
            return match parameter {
                crate::core::effect_params::ParamRefRef::Scalar(track) => track
                    .keyframes()
                    .map(|keys| keys.iter().map(|key| (key.frame, key.value)).collect())
                    .unwrap_or_default(),
                crate::core::effect_params::ParamRefRef::Vec2(track) => track
                    .keyframes()
                    .map(|keys| {
                        keys.iter()
                            .map(|key| (key.frame, key.value[channel.min(1)]))
                            .collect()
                    })
                    .unwrap_or_default(),
                crate::core::effect_params::ParamRefRef::Vec3(track) => track
                    .keyframes()
                    .map(|keys| {
                        keys.iter()
                            .map(|key| (key.frame, key.value[channel.min(2)]))
                            .collect()
                    })
                    .unwrap_or_default(),
                crate::core::effect_params::ParamRefRef::Vec4Color(track) => track
                    .keyframes()
                    .map(|keys| {
                        keys.iter()
                            .map(|key| (key.frame, key.value[channel.min(3)]))
                            .collect()
                    })
                    .unwrap_or_default(),
            };
        }
    }
    vec![]
}

pub fn effect_channel_bezier_points(layer: &Layer, property: &str, frame: u32) -> Option<[f32; 4]> {
    let (effect_id, label, _) = parse_effect_property(property)?;
    let effect = layer.effects.iter().find(|effect| effect.id == effect_id)?;
    for (name, parameter) in effect.effect_type.animatable_params_ref() {
        if name != label {
            continue;
        }
        let interpolation = match parameter {
            crate::core::effect_params::ParamRefRef::Vec2(track) => {
                track
                    .keyframes()?
                    .iter()
                    .find(|key| key.frame == frame)?
                    .interpolation
            }
            crate::core::effect_params::ParamRefRef::Vec3(track) => {
                track
                    .keyframes()?
                    .iter()
                    .find(|key| key.frame == frame)?
                    .interpolation
            }
            crate::core::effect_params::ParamRefRef::Vec4Color(track) => {
                track
                    .keyframes()?
                    .iter()
                    .find(|key| key.frame == frame)?
                    .interpolation
            }
            _ => return None,
        };
        return match interpolation {
            crate::core::keyframe::InterpolationType::Bezier {
                custom_bezier: Some(points),
                ..
            } => Some(points),
            _ => Some([0.33, 0.0, 0.67, 1.0]),
        };
    }
    None
}

pub fn set_effect_channel_bezier(
    layer: &mut Layer,
    property: &str,
    frame: u32,
    points: [f32; 4],
) -> bool {
    let Some((effect_id, label, _)) = parse_effect_property(property) else {
        return false;
    };
    if let Some(effect) = layer
        .effects
        .iter_mut()
        .find(|effect| effect.id == effect_id)
    {
        return effect
            .effect_type
            .set_parameter_keyframe_bezier_at_frame(label, frame, points);
    }
    false
}

pub fn set_effect_channel_at_frame(layer: &mut Layer, property: &str, frame: u32, value: f32) -> bool {
    let Some((effect_id, parameter_name, component)) = parse_effect_property(property) else {
        return false;
    };
    if let Some(effect) = layer
        .effects
        .iter_mut()
        .find(|effect| effect.id == effect_id)
    {
        return match component {
            Some(component) => effect.effect_type.set_parameter_component_keyframe(
                parameter_name,
                component,
                frame,
                value,
            ),
            None => effect
                .effect_type
                .set_scalar_parameter_keyframe(parameter_name, frame, value),
        };
    }
    false
}

pub fn move_effect_channel_keyframe(
    layer: &mut Layer,
    property: &str,
    from_frame: u32,
    to_frame: u32,
) -> bool {
    let Some((effect_id, parameter_name, component)) = parse_effect_property(property) else {
        return false;
    };
    let Some(component) = component else {
        return false;
    };
    if let Some(effect) = layer
        .effects
        .iter_mut()
        .find(|effect| effect.id == effect_id)
    {
        return effect.effect_type.move_parameter_component_keyframe(
            parameter_name,
            component,
            from_frame,
            to_frame,
        );
    }
    false
}

pub fn move_effect_scalar_keyframe(
    layer: &mut Layer,
    property: &str,
    from_frame: u32,
    to_frame: u32,
) -> bool {
    let Some((effect_id, parameter_name, component)) = parse_effect_property(property) else {
        return false;
    };
    if component.is_some() {
        return false;
    }
    if let Some(effect) = layer
        .effects
        .iter_mut()
        .find(|effect| effect.id == effect_id)
    {
        return effect.effect_type.move_scalar_parameter_keyframe(
            parameter_name,
            from_frame,
            to_frame,
        );
    }
    false
}
