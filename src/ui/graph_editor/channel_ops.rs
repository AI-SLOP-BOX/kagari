use super::property::{GraphProperty, Scalar2dTrack, Vector2Track, Vector3Track};
use crate::core::timeline::Layer;

pub trait GraphChannelValue {
    fn component(&self, axis: usize) -> f32;
    fn set_component(&mut self, axis: usize, value: f32);
}

impl GraphChannelValue for f32 {
    fn component(&self, _axis: usize) -> f32 {
        *self
    }

    fn set_component(&mut self, _axis: usize, value: f32) {
        *self = value;
    }
}

impl GraphChannelValue for [f32; 2] {
    fn component(&self, axis: usize) -> f32 {
        self[axis.min(1)]
    }

    fn set_component(&mut self, axis: usize, value: f32) {
        self[axis.min(1)] = value;
    }
}

impl GraphChannelValue for [f32; 3] {
    fn component(&self, axis: usize) -> f32 {
        self[axis.min(2)]
    }

    fn set_component(&mut self, axis: usize, value: f32) {
        self[axis.min(2)] = value;
    }
}

/// Move the keyed item by its frame identity, then re-find it after sorting.
pub fn move_and_set_channel<T: GraphChannelValue + Clone>(
    track: &mut crate::core::property::Animatable<T>,
    from_frame: u32,
    to_frame: u32,
    axis: usize,
    value: f32,
) -> bool {
    let mut changed = false;
    if from_frame != to_frame {
        changed |= track.move_keyframe(from_frame, to_frame);
    }
    if let Some(keyframes) = track.keyframes_mut() {
        if let Some(keyframe) = keyframes.iter_mut().find(|key| key.frame == to_frame) {
            if (keyframe.value.component(axis) - value).abs() > f32::EPSILON {
                keyframe.value.set_component(axis, value);
                changed = true;
            }
        }
    }
    changed
}

pub fn axis_3d(prop: &str) -> usize {
    match GraphProperty::parse(prop) {
        GraphProperty::Vector3 { axis, .. } => axis.index(),
        _ => 0,
    }
}

pub fn pin_component(property: &str) -> usize {
    match GraphProperty::parse(property) {
        GraphProperty::PuppetPin { axis, .. } => axis.index(),
        _ => 0,
    }
}

pub fn clamp_bezier_x(value: f32, first_bound: f32, second_bound: f32) -> f32 {
    let lower = first_bound.min(second_bound).clamp(0.0, 1.0);
    let upper = first_bound.max(second_bound).clamp(0.0, 1.0);
    value.clamp(lower, upper)
}

pub fn parse_effect_property(property: &str) -> Option<(&str, &str, Option<usize>)> {
    match GraphProperty::parse(property) {
        GraphProperty::Effect {
            effect_id,
            parameter,
            component,
        } => Some((effect_id, parameter, component)),
        _ => None,
    }
}

pub fn is_effect_property(property: &str) -> bool {
    property.starts_with("fxid:")
}

fn keyframe_frame<T>(
    keys: Option<&[crate::core::keyframe::Keyframe<T>]>,
    index: usize,
) -> Option<u32> {
    keys.and_then(|keys| keys.get(index)).map(|key| key.frame)
}

pub fn graph_keyframe_frame(layer: &Layer, property: &str, index: usize) -> Option<u32> {
    match GraphProperty::parse(property) {
        GraphProperty::Vector2 { track, .. } => match track {
            Vector2Track::Position => keyframe_frame(layer.transform.position.keyframes(), index),
            Vector2Track::Scale => keyframe_frame(layer.transform.scale.keyframes(), index),
        },
        GraphProperty::Scalar2d(track) => match track {
            Scalar2dTrack::Rotation => keyframe_frame(layer.transform.rotation.keyframes(), index),
            Scalar2dTrack::Opacity => keyframe_frame(layer.transform.opacity.keyframes(), index),
        },
        GraphProperty::Vector3 { track, .. } => match track {
            Vector3Track::Position => {
                keyframe_frame(layer.transform_3d.position.keyframes(), index)
            }
            Vector3Track::Rotation => {
                keyframe_frame(layer.transform_3d.rotation.keyframes(), index)
            }
            Vector3Track::Scale => keyframe_frame(layer.transform_3d.scale.keyframes(), index),
        },
        GraphProperty::PuppetPin { id, .. } => layer
            .puppet_pins
            .iter()
            .find(|pin| pin.id == id)
            .and_then(|pin| keyframe_frame(pin.position.keyframes(), index)),
        GraphProperty::Effect {
            effect_id,
            parameter,
            ..
        } => {
            let effect = layer.effects.iter().find(|effect| effect.id == effect_id)?;
            effect
                .effect_type
                .animatable_params_ref()
                .into_iter()
                .find(|(name, _)| *name == parameter)
                .and_then(|(_, parameter)| match parameter {
                    crate::core::effect_params::ParamRefRef::Scalar(track) => {
                        keyframe_frame(track.keyframes(), index)
                    }
                    crate::core::effect_params::ParamRefRef::Vec2(track) => {
                        keyframe_frame(track.keyframes(), index)
                    }
                    crate::core::effect_params::ParamRefRef::Vec3(track) => {
                        keyframe_frame(track.keyframes(), index)
                    }
                    crate::core::effect_params::ParamRefRef::Vec4Color(track) => {
                        keyframe_frame(track.keyframes(), index)
                    }
                })
        }
        GraphProperty::Unknown(_) => None,
    }
}

pub fn rove_keyframes<T: Clone>(
    keys: &mut [crate::core::keyframe::Keyframe<T>],
    distance: impl Fn(&T, &T) -> f32,
) -> bool {
    if keys.len() < 3 {
        return false;
    }
    let first_frame = keys[0].frame;
    let last_frame = keys[keys.len() - 1].frame;
    let span = last_frame.saturating_sub(first_frame);
    if span < (keys.len() - 1) as u32 {
        return false;
    }
    let mut cumulative = vec![0.0_f32; keys.len()];
    for index in 1..keys.len() {
        cumulative[index] =
            cumulative[index - 1] + distance(&keys[index - 1].value, &keys[index].value).max(0.0);
    }
    let total_distance = *cumulative.last().unwrap_or(&0.0);
    if total_distance <= f32::EPSILON {
        return false;
    }

    let mut changed = false;
    let mut previous_frame = first_frame;
    let last_index = keys.len() - 1;
    for (index, key) in keys.iter_mut().enumerate().skip(1).take(last_index - 1) {
        let minimum = previous_frame.saturating_add(1);
        let remaining = (last_index - index) as u32;
        let maximum = last_frame.saturating_sub(remaining);
        let proposed = first_frame as f32 + span as f32 * cumulative[index] / total_distance;
        let next_frame = (proposed.round() as u32).clamp(minimum, maximum);
        changed |= key.frame != next_frame;
        key.frame = next_frame;
        previous_frame = next_frame;
    }
    changed
}

pub fn rove_across_time(layer: &mut Layer, property: &str) -> bool {
    match GraphProperty::parse(property) {
        GraphProperty::Vector2 {
            track: Vector2Track::Position,
            ..
        } => layer
            .transform
            .position
            .keyframes_mut()
            .is_some_and(|keys| rove_keyframes(keys, |a, b| (b[0] - a[0]).hypot(b[1] - a[1]))),
        GraphProperty::Vector3 {
            track: Vector3Track::Position,
            ..
        } => layer
            .transform_3d
            .position
            .keyframes_mut()
            .is_some_and(|keys| {
                rove_keyframes(keys, |a, b| {
                    ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2) + (b[2] - a[2]).powi(2)).sqrt()
                })
            }),
        GraphProperty::PuppetPin { id, axis: _ } => layer
            .puppet_pins
            .iter_mut()
            .find(|pin| pin.id == id)
            .map(|pin| &mut pin.position)
            .and_then(|track| track.keyframes_mut())
            .is_some_and(|keys| rove_keyframes(keys, |a, b| (b[0] - a[0]).hypot(b[1] - a[1]))),
        _ => false,
    }
}

pub fn reverse_keyframes<T: Clone>(track: &mut crate::core::property::Animatable<T>) -> bool {
    track.reverse_keyframes()
}

pub fn reverse_effect_property(layer: &mut Layer, property: &str) -> bool {
    let Some((effect_id, parameter, _)) = parse_effect_property(property) else {
        return false;
    };
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
        .find_map(|(name, parameter_ref)| {
            if name != parameter {
                return None;
            }
            Some(match parameter_ref {
                crate::core::effect_params::ParamRef::Scalar(track) => reverse_keyframes(track),
                crate::core::effect_params::ParamRef::Vec2(track) => reverse_keyframes(track),
                crate::core::effect_params::ParamRef::Vec3(track) => reverse_keyframes(track),
                crate::core::effect_params::ParamRef::Vec4Color(track) => reverse_keyframes(track),
            })
        })
        .unwrap_or(false)
}

pub fn pin_anim_mut<'a>(
    layer: &'a mut Layer,
    prop: &str,
) -> Option<&'a mut crate::core::property::Animatable<[f32; 2]>> {
    let GraphProperty::PuppetPin { id, .. } = GraphProperty::parse(prop) else {
        return None;
    };
    layer
        .puppet_pins
        .iter_mut()
        .find(|p| p.id == id)
        .map(|p| &mut p.position)
}
