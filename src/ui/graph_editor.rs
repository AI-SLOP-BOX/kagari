use crate::core::timeline::Layer;
use crate::ui::theme::colors;
use eframe::egui;
mod camera;
mod canvas;
pub(crate) mod property;
pub use camera::draw_camera_lens_graph;
pub use canvas::draw_graph_editor;
use property::{GraphProperty, Scalar2dTrack, Vector2Track, Vector3Track};

#[derive(Clone, Copy, Debug, PartialEq)]
struct GraphKeyframeDrag {
    original_frame: u32,
    current_frame: u32,
    original_value: f32,
    current_value: f32,
}

trait GraphChannelValue {
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
/// The old graph editor mutated an index after sorting, which could edit the
/// neighbour that took that index. Keeping the lookup frame-based also makes
/// this helper safe for all vector/scalar transform tracks.
fn move_and_set_channel<T: GraphChannelValue + Clone>(
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

fn axis_3d(prop: &str) -> usize {
    match GraphProperty::parse(prop) {
        GraphProperty::Vector3 { axis, .. } => axis.index(),
        _ => 0,
    }
}

fn pin_component(property: &str) -> usize {
    match GraphProperty::parse(property) {
        GraphProperty::PuppetPin { axis, .. } => axis.index(),
        _ => 0,
    }
}

fn clamp_bezier_x(value: f32, first_bound: f32, second_bound: f32) -> f32 {
    let lower = first_bound.min(second_bound).clamp(0.0, 1.0);
    let upper = first_bound.max(second_bound).clamp(0.0, 1.0);
    value.clamp(lower, upper)
}

/// Graph properties for effects use the effect instance id, not its display
/// name. Two instances may have the same name, and names can also be
/// localized or edited without changing which parameter is selected.
pub(crate) fn parse_effect_property(property: &str) -> Option<(&str, &str, Option<usize>)> {
    match GraphProperty::parse(property) {
        GraphProperty::Effect {
            effect_id,
            parameter,
            component,
        } => Some((effect_id, parameter, component)),
        _ => None,
    }
}

pub(crate) fn is_effect_property(property: &str) -> bool {
    property.starts_with("fxid:")
}

fn keyframe_frame<T>(
    keys: Option<&[crate::core::keyframe::Keyframe<T>]>,
    index: usize,
) -> Option<u32> {
    keys.and_then(|keys| keys.get(index)).map(|key| key.frame)
}

fn graph_keyframe_frame(layer: &Layer, property: &str, index: usize) -> Option<u32> {
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

fn rove_keyframes<T: Clone>(
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

fn rove_across_time(layer: &mut Layer, property: &str) -> bool {
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

fn reverse_keyframes<T: Clone>(track: &mut crate::core::property::Animatable<T>) -> bool {
    track.reverse_keyframes()
}

fn reverse_effect_property(layer: &mut Layer, property: &str) -> bool {
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

/// A reusable module for rendering the keyframe Graph Editor.
///
/// Visualizes animatable property value curves over time, drawing interactive control
/// points and Bezier tangent handles.
/// Resolve "PinX:<id>" / "PinY:<id>" graph properties to the pin's track.
fn pin_anim_mut<'a>(
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

fn set_layer_interpolation(
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

/// Mutate interpolation handles on the active graph property without losing
/// the user's selection scope. `None` means all keys; `Some(frame)` means the
/// one key under the graph cursor. The legacy ease buttons used to always
/// mutate the whole track, which made a single-key edit surprisingly global.
fn map_layer_interpolation(
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

fn remove_effect_channel_at_frame(layer: &mut Layer, property: &str, frame: u32) -> bool {
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

#[allow(dead_code)] // graph-editor interpolation infrastructure — wired in future pass
fn effect_parameter_channel_interpolation(
    layer: &Layer,
    property: &str,
    frame: u32,
) -> Option<crate::core::keyframe::InterpolationType> {
    let (effect_id, label, _) = parse_effect_property(property)?;
    let effect = layer.effects.iter().find(|effect| effect.id == effect_id)?;
    for (name, parameter) in effect.effect_type.animatable_params_ref() {
        if name != label {
            continue;
        }
        return match parameter {
            crate::core::effect_params::ParamRefRef::Scalar(track) => track
                .keyframes()?
                .iter()
                .find(|key| key.frame == frame)
                .map(|key| key.interpolation),
            crate::core::effect_params::ParamRefRef::Vec2(track) => track
                .keyframes()?
                .iter()
                .find(|key| key.frame == frame)
                .map(|key| key.interpolation),
            crate::core::effect_params::ParamRefRef::Vec3(track) => track
                .keyframes()?
                .iter()
                .find(|key| key.frame == frame)
                .map(|key| key.interpolation),
            crate::core::effect_params::ParamRefRef::Vec4Color(track) => track
                .keyframes()?
                .iter()
                .find(|key| key.frame == frame)
                .map(|key| key.interpolation),
        };
    }
    None
}

#[allow(dead_code)] // graph-editor interpolation infrastructure — wired in future pass
fn set_effect_channel_interpolation_at_frame(
    layer: &mut Layer,
    property: &str,
    frame: u32,
    interpolation: crate::core::keyframe::InterpolationType,
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
            .set_parameter_keyframe_interpolation_at_frame(label, frame, interpolation);
    }
    false
}

fn effect_parameter_channel_value(layer: &Layer, property: &str, frame: u32) -> f32 {
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

fn effect_parameter_channel_keyframes(layer: &Layer, property: &str) -> Vec<(u32, f32)> {
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

fn effect_channel_bezier_points(layer: &Layer, property: &str, frame: u32) -> Option<[f32; 4]> {
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

fn set_effect_channel_bezier(
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

fn set_effect_channel_at_frame(layer: &mut Layer, property: &str, frame: u32, value: f32) -> bool {
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

fn move_effect_channel_keyframe(
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

fn move_effect_scalar_keyframe(
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

fn track_velocity<T: Clone>(
    track: &mut crate::core::property::Animatable<T>,
    index: usize,
    fps: f32,
    component: impl Fn(&T) -> f32,
    replacement: Option<[f32; 4]>,
) -> Option<[f32; 4]> {
    use crate::core::keyframe::{
        compute_ae_bezier_control_points, BezierControlPoint, InterpolationType,
    };
    let keys = track.keyframes_mut()?;
    let key = keys.get(index)?;
    let next = keys.get(index + 1)?;
    let span = next.frame.checked_sub(key.frame)? as f32;
    let delta = component(&next.value) - component(&key.value);
    let values = replacement.unwrap_or(match key.interpolation {
        InterpolationType::Bezier {
            incoming, outgoing, ..
        } => [
            incoming.influence * 100.0,
            outgoing.influence * 100.0,
            incoming.speed,
            outgoing.speed,
        ],
        _ => [33.3, 33.3, 0.0, 0.0],
    });
    if replacement.is_some() {
        let incoming = BezierControlPoint {
            influence: values[0] / 100.0,
            speed: values[2],
        };
        let outgoing = BezierControlPoint {
            influence: values[1] / 100.0,
            speed: values[3],
        };
        let control = compute_ae_bezier_control_points(&outgoing, &incoming, span, delta, fps);
        keys[index].interpolation = InterpolationType::Bezier {
            incoming,
            outgoing,
            custom_bezier: Some(control),
        };
    }
    Some(values)
}

fn layer_velocity(
    layer: &mut Layer,
    property: &str,
    index: usize,
    fps: f32,
    replacement: Option<[f32; 4]>,
) -> Option<[f32; 4]> {
    let axis = axis_3d(property);
    match property {
        "Position X" | "Position Y" => track_velocity(
            &mut layer.transform.position,
            index,
            fps,
            |v| v[axis],
            replacement,
        ),
        "Scale X" | "Scale Y" => track_velocity(
            &mut layer.transform.scale,
            index,
            fps,
            |v| v[axis],
            replacement,
        ),
        "Rotation" => track_velocity(
            &mut layer.transform.rotation,
            index,
            fps,
            |v| *v,
            replacement,
        ),
        "Opacity" => track_velocity(
            &mut layer.transform.opacity,
            index,
            fps,
            |v| *v,
            replacement,
        ),
        p if p.starts_with("3D Position") => track_velocity(
            &mut layer.transform_3d.position,
            index,
            fps,
            |v| v[axis],
            replacement,
        ),
        p if p.starts_with("3D Rotation") => track_velocity(
            &mut layer.transform_3d.rotation,
            index,
            fps,
            |v| v[axis],
            replacement,
        ),
        p if p.starts_with("3D Scale") => track_velocity(
            &mut layer.transform_3d.scale,
            index,
            fps,
            |v| v[axis],
            replacement,
        ),
        _ => None,
    }
}

fn draw_ease_thumbnail(
    ui: &mut egui::Ui,
    preset: crate::core::keyframe::EasePreset,
) -> egui::Response {
    let want = egui::vec2(52.0, 30.0);
    let (rect, resp) = ui.allocate_exact_size(want, egui::Sense::click());
    if !ui.is_rect_visible(rect) {
        return resp;
    }
    let pts = preset.control_points();
    let bg = if resp.hovered() {
        colors::BG_HOVER
    } else {
        colors::BG_SURFACE
    };
    ui.painter().rect_filled(rect, 6.0, bg);
    let border_stroke = if resp.hovered() {
        egui::Stroke::new(1.0_f32, colors::ACCENT_SELECTION)
    } else {
        egui::Stroke::new(1.0_f32, colors::BORDER_MEDIUM)
    };
    ui.painter().rect_stroke(rect, 6.0, border_stroke);
    let pad = 5.0;
    let inner = egui::Rect::from_min_max(
        rect.min + egui::vec2(pad, pad),
        rect.max - egui::vec2(pad, pad),
    );
    let w = inner.width().max(0.01);
    let h = inner.height().max(0.01);
    // Faint diagonal reference (linear) so the easing shape reads instantly.
    ui.painter().line_segment(
        [
            egui::pos2(inner.min.x, inner.max.y),
            egui::pos2(inner.max.x, inner.min.y),
        ],
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
    let stroke_col = if resp.hovered() {
        colors::ACCENT_BRAND_HOVER
    } else {
        colors::ACCENT_BRAND
    };
    let mut prev: Option<egui::Pos2> = None;
    for step in 0..=16 {
        let x = step as f32 / 16.0;
        let t = crate::core::keyframe::solve_bezier_eased_time(x, pts[0], pts[1], pts[2], pts[3]);
        if !t.is_finite() {
            prev = None;
            continue;
        }
        let omt = 1.0 - t;
        let y = 3.0 * omt * omt * t * pts[1] + 3.0 * omt * t * t * pts[3] + t * t * t;
        if !y.is_finite() {
            prev = None;
            continue;
        }
        let y_clamped = y.clamp(-0.35, 1.35);
        let norm = (y_clamped + 0.35) / 1.7;
        let p = egui::pos2(inner.min.x + x * w, inner.max.y - norm * h);
        if let Some(pr) = prev {
            // Subtle glow/emphasis
            if resp.hovered() {
                ui.painter().line_segment(
                    [pr, p],
                    egui::Stroke::new(3.5_f32, stroke_col.gamma_multiply(0.25)),
                );
            }
            ui.painter()
                .line_segment([pr, p], egui::Stroke::new(1.8_f32, stroke_col));
        }
        prev = Some(p);
    }
    resp
}

/// Timeline graph workspace: animation curves only, never a dependency/node graph.
/// Each property has its own value range so Position, Rotation and Opacity stay readable.
pub fn draw_animation_graph_editor(
    selected_property: &mut Option<String>,
    ui: &mut egui::Ui,
    duration_frames: u32,
    layer: &Layer,
    current_frame: &mut u32,
) {
    struct Track {
        key: String,
        label: &'static str,
        color: egui::Color32,
        values: Vec<(u32, f32)>,
        animated: bool,
    }
    let total = duration_frames.max(1);
    let mut tracks = Vec::with_capacity(5);
    let mut add = |key: &str,
                   label: &'static str,
                   color: egui::Color32,
                   mut values: Vec<(u32, f32)>,
                   animated: bool| {
        if values.is_empty() {
            values.push((0, 0.0));
            values.push((total, 0.0));
        }
        tracks.push(Track {
            key: key.into(),
            label,
            color,
            values,
            animated,
        });
    };
    let v2 = |a: &crate::core::property::Animatable<[f32; 2]>, axis: usize| -> Vec<(u32, f32)> {
        a.keyframes()
            .map(|k| {
                k.iter()
                    .map(|x| (x.frame.min(total), x.value[axis]))
                    .collect()
            })
            .unwrap_or_default()
    };
    let s = |a: &crate::core::property::Animatable<f32>| -> Vec<(u32, f32)> {
        a.keyframes()
            .map(|k| k.iter().map(|x| (x.frame.min(total), x.value)).collect())
            .unwrap_or_default()
    };
    let px = v2(&layer.transform.position, 0);
    let py = v2(&layer.transform.position, 1);
    let sx = v2(&layer.transform.scale, 0);
    let rot = s(&layer.transform.rotation);
    let op = s(&layer.transform.opacity);
    add(
        "Position X",
        "Position X",
        colors::ACCENT_BLUE,
        if px.is_empty() {
            vec![
                (0, layer.transform.position.evaluate(0)[0]),
                (total, layer.transform.position.evaluate(total)[0]),
            ]
        } else {
            px
        },
        !v2(&layer.transform.position, 0).is_empty(),
    );
    add(
        "Position Y",
        "Position Y",
        egui::Color32::from_rgb(92, 200, 180),
        if py.is_empty() {
            vec![
                (0, layer.transform.position.evaluate(0)[1]),
                (total, layer.transform.position.evaluate(total)[1]),
            ]
        } else {
            py
        },
        !v2(&layer.transform.position, 1).is_empty(),
    );
    add(
        "Scale X",
        "Scale",
        egui::Color32::from_rgb(172, 132, 255),
        if sx.is_empty() {
            vec![
                (0, layer.transform.scale.evaluate(0)[0]),
                (total, layer.transform.scale.evaluate(total)[0]),
            ]
        } else {
            sx
        },
        !v2(&layer.transform.scale, 0).is_empty(),
    );
    add(
        "Rotation",
        "Rotation",
        egui::Color32::from_rgb(255, 173, 92),
        if rot.is_empty() {
            vec![
                (0, layer.transform.rotation.evaluate(0)),
                (total, layer.transform.rotation.evaluate(total)),
            ]
        } else {
            rot
        },
        !s(&layer.transform.rotation).is_empty(),
    );
    add(
        "Opacity",
        "Opacity",
        egui::Color32::from_rgb(242, 112, 148),
        if op.is_empty() {
            vec![
                (0, layer.transform.opacity.evaluate(0)),
                (total, layer.transform.opacity.evaluate(total)),
            ]
        } else {
            op
        },
        !s(&layer.transform.opacity).is_empty(),
    );

    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Graph Editor").size(13.0).strong());
            ui.label(
                egui::RichText::new("Animation Curves")
                    .small()
                    .color(colors::TEXT_MUTED),
            );
            ui.separator();
            ui.label(
                egui::RichText::new("X: time   Y: value")
                    .small()
                    .color(colors::TEXT_MUTED),
            );
        });
        ui.add_space(3.0);
        let (toolbar, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 25.0), egui::Sense::hover());
        ui.painter().line_segment(
            [toolbar.left_bottom(), toolbar.right_bottom()],
            egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
        );
        ui.painter().text(
            toolbar.left_center() + egui::vec2(8.0, 0.0),
            egui::Align2::LEFT_CENTER,
            "Value Graph   ·   Bezier",
            egui::FontId::proportional(11.0),
            colors::TEXT_SECONDARY,
        );
        ui.painter().text(
            toolbar.right_center() - egui::vec2(8.0, 0.0),
            egui::Align2::RIGHT_CENTER,
            format!(
                "{} animated tracks",
                tracks.iter().filter(|x| x.animated).count()
            ),
            egui::FontId::proportional(10.0),
            colors::TEXT_MUTED,
        );

        let row_h = ((ui.available_height() - 8.0) / tracks.len().max(1) as f32).clamp(48.0, 76.0);
        let graph = egui::Rect::from_min_size(
            ui.cursor().min,
            egui::vec2(ui.available_width(), row_h * tracks.len() as f32),
        );
        ui.allocate_rect(graph, egui::Sense::hover());
        let left = graph.left() + 116.0;
        let width = (graph.width() - 124.0).max(40.0);
        let x_of = |frame: u32| left + frame.min(total) as f32 / total as f32 * width;
        let playhead = x_of(*current_frame);
        ui.painter().line_segment(
            [
                egui::pos2(playhead, graph.top()),
                egui::pos2(playhead, graph.bottom()),
            ],
            egui::Stroke::new(1.0_f32, colors::ACCENT_ORANGE),
        );
        for (index, track) in tracks.iter().enumerate() {
            let row = egui::Rect::from_min_size(
                egui::pos2(graph.left(), graph.top() + index as f32 * row_h),
                egui::vec2(graph.width(), row_h),
            );
            let min = track
                .values
                .iter()
                .map(|(_, v)| *v)
                .fold(f32::INFINITY, f32::min);
            let max = track
                .values
                .iter()
                .map(|(_, v)| *v)
                .fold(f32::NEG_INFINITY, f32::max);
            let pad = (max - min).abs().max(1.0) * 0.12;
            let lo = min - pad;
            let hi = max + pad;
            let y_of = |v: f32| {
                row.bottom()
                    - 10.0
                    - ((v - lo) / (hi - lo).max(0.01)).clamp(0.0, 1.0) * (row.height() - 20.0)
            };
            if index > 0 {
                ui.painter().line_segment(
                    [row.left_top(), row.right_top()],
                    egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
                );
            }
            ui.painter().text(
                egui::pos2(row.left() + 8.0, row.center().y - 5.0),
                egui::Align2::LEFT_CENTER,
                track.label,
                egui::FontId::proportional(11.0),
                if selected_property.as_deref() == Some(track.key.as_str()) {
                    colors::TEXT_PRIMARY
                } else {
                    colors::TEXT_SECONDARY
                },
            );
            ui.painter().text(
                egui::pos2(row.left() + 8.0, row.center().y + 11.0),
                egui::Align2::LEFT_CENTER,
                if track.animated {
                    "animated"
                } else {
                    "constant"
                },
                egui::FontId::proportional(9.0),
                if track.animated {
                    track.color
                } else {
                    colors::TEXT_MUTED
                },
            );
            for division in 0..=4 {
                let x = left + division as f32 / 4.0 * width;
                ui.painter().line_segment(
                    [egui::pos2(x, row.top()), egui::pos2(x, row.bottom())],
                    egui::Stroke::new(0.5_f32, colors::GRID_LINE),
                );
                if index == tracks.len() - 1 {
                    ui.painter().text(
                        egui::pos2(x + 2.0, row.bottom() - 2.0),
                        egui::Align2::LEFT_BOTTOM,
                        format!("{}f", total * division / 4),
                        egui::FontId::proportional(9.0),
                        colors::TEXT_MUTED,
                    );
                }
            }
            for division in 0..=2 {
                let y = row.bottom() - 10.0 - division as f32 / 2.0 * (row.height() - 20.0);
                ui.painter().line_segment(
                    [egui::pos2(left, y), egui::pos2(graph.right(), y)],
                    egui::Stroke::new(0.5_f32, colors::GRID_LINE),
                );
            }
            let points: Vec<_> = track
                .values
                .iter()
                .map(|(f, v)| egui::pos2(x_of(*f), y_of(*v)))
                .collect();
            for pair in points.windows(2) {
                let p0 = pair[0];
                let p3 = pair[1];
                let span = (p3.x - p0.x).max(8.0);
                let p1 = egui::pos2(p0.x + span * 0.36, p0.y);
                let p2 = egui::pos2(p3.x - span * 0.36, p3.y);
                let mut previous = p0;
                for step in 1..=18 {
                    let t = step as f32 / 18.0;
                    let q = 1.0 - t;
                    let next = egui::pos2(
                        q.powi(3) * p0.x
                            + 3.0 * q.powi(2) * t * p1.x
                            + 3.0 * q * t.powi(2) * p2.x
                            + t.powi(3) * p3.x,
                        q.powi(3) * p0.y
                            + 3.0 * q.powi(2) * t * p1.y
                            + 3.0 * q * t.powi(2) * p2.y
                            + t.powi(3) * p3.y,
                    );
                    ui.painter().line_segment(
                        [previous, next],
                        egui::Stroke::new(
                            if track.animated { 1.8_f32 } else { 1.0_f32 },
                            track.color,
                        ),
                    );
                    previous = next;
                }
            }
            for (i, point) in points.iter().enumerate() {
                if track.animated {
                    let value = track.values[i].1;
                    let prev = track
                        .values
                        .get(i.saturating_sub(1))
                        .map(|x| x.1)
                        .unwrap_or(value);
                    let next = track
                        .values
                        .get((i + 1).min(track.values.len() - 1))
                        .map(|x| x.1)
                        .unwrap_or(value);
                    let span = 14.0_f32.min(width * 0.08);
                    let hout = egui::pos2(point.x + span, y_of((value + next) * 0.5));
                    let hin = egui::pos2(point.x - span, y_of((value + prev) * 0.5));
                    ui.painter().line_segment(
                        [*point, hout],
                        egui::Stroke::new(0.8_f32, track.color.linear_multiply(0.65)),
                    );
                    ui.painter().line_segment(
                        [*point, hin],
                        egui::Stroke::new(0.8_f32, track.color.linear_multiply(0.65)),
                    );
                    ui.painter()
                        .circle_filled(hout, 2.5, track.color.linear_multiply(0.75));
                    ui.painter()
                        .circle_filled(hin, 2.5, track.color.linear_multiply(0.75));
                    ui.painter().circle_filled(
                        *point,
                        4.0,
                        if selected_property.as_deref() == Some(track.key.as_str()) {
                            colors::ACCENT_ORANGE
                        } else {
                            track.color
                        },
                    );
                }
            }
        }
    });
}

pub fn draw_automation_curve(
    ui: &mut egui::Ui,
    curve: &mut crate::core::automation_binding::AutomationCurve,
    changed: &mut bool,
) {
    if curve.points.is_empty() {
        return;
    }
    ui.separator();
    ui.label(egui::RichText::new("Automation Channel").strong());
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 100.0),
        egui::Sense::hover(),
    );
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, crate::ui::theme::colors::BG_DEEPEST);
    let min_time = curve
        .points
        .first()
        .map(|point| point.time.numerator as f32 / point.time.denominator as f32)
        .unwrap_or(0.0);
    let max_time = curve
        .points
        .last()
        .map(|point| point.time.numerator as f32 / point.time.denominator as f32)
        .unwrap_or(min_time + 1.0)
        .max(min_time + f32::EPSILON);
    let min_value = curve
        .points
        .iter()
        .map(|point| point.value as f32)
        .fold(f32::INFINITY, f32::min);
    let max_value = curve
        .points
        .iter()
        .map(|point| point.value as f32)
        .fold(f32::NEG_INFINITY, f32::max)
        .max(min_value + f32::EPSILON);
    let to_screen = |time: f32, value: f32| {
        egui::pos2(
            rect.left()
                + ((time - min_time) / (max_time - min_time)).clamp(0.0, 1.0) * rect.width(),
            rect.bottom()
                - ((value - min_value) / (max_value - min_value)).clamp(0.0, 1.0) * rect.height(),
        )
    };
    let points: Vec<_> = (0..=64)
        .filter_map(|index| {
            let ratio = index as f32 / 64.0;
            let time = min_time + ratio * (max_time - min_time);
            let sample_time = crate::core::unified_time::Time::new(
                (time * 1_000_000.0).round() as i64,
                1_000_000,
            );
            curve
                .sample(sample_time)
                .map(|value| to_screen(time, value as f32))
        })
        .collect();
    for segment in points.windows(2) {
        painter.line_segment(
            [segment[0], segment[1]],
            egui::Stroke::new(2.0_f32, crate::ui::theme::colors::ACCENT_CYAN),
        );
    }
    for point in &curve.points {
        let time = point.time.numerator as f32 / point.time.denominator as f32;
        painter.circle_filled(
            to_screen(time, point.value as f32),
            3.0,
            crate::ui::theme::colors::ACCENT_ORANGE,
        );
    }
    if response.double_clicked() {
        if let Some(pointer) = response.interact_pointer_pos() {
            let normalized_time =
                ((pointer.x - rect.left()) / rect.width().max(1.0)).clamp(0.0, 1.0);
            let normalized_value =
                ((rect.bottom() - pointer.y) / rect.height().max(1.0)).clamp(0.0, 1.0);
            let time = min_time + normalized_time * (max_time - min_time);
            let value = min_value + normalized_value * (max_value - min_value);
            if curve
                .upsert_point(
                    crate::core::unified_time::Time::new(
                        (time * 1_000_000.0).round() as i64,
                        1_000_000,
                    ),
                    value as f64,
                )
                .is_ok()
            {
                *changed = true;
            }
        }
    }
}

fn remove_camera_key<T>(
    track: &mut Option<crate::core::property::Animatable<T>>,
    frame: u32,
) -> bool {
    if let Some(crate::core::property::Animatable::Animated(keys)) = track {
        let original_len = keys.len();
        keys.retain(|key| key.frame != frame);
        let removed = keys.len() != original_len;
        let became_empty = keys.is_empty();
        if became_empty {
            *track = None;
        }
        return removed;
    }
    false
}

fn set_camera_key_interpolation<T>(
    track: &mut Option<crate::core::property::Animatable<T>>,
    frame: u32,
    interpolation: crate::core::keyframe::InterpolationType,
) -> bool {
    if let Some(crate::core::property::Animatable::Animated(keys)) = track {
        if let Some(key) = keys.iter_mut().find(|key| key.frame == frame) {
            if key.interpolation != interpolation {
                key.interpolation = interpolation;
                return true;
            }
        }
    }
    false
}

fn set_camera_key_ease<T>(
    track: &mut Option<crate::core::property::Animatable<T>>,
    frame: u32,
    preset: crate::core::keyframe::EasePreset,
) -> bool {
    use crate::core::keyframe::{BezierControlPoint, InterpolationType};

    if let Some(crate::core::property::Animatable::Animated(keys)) = track {
        if let Some(key) = keys.iter_mut().find(|key| key.frame == frame) {
            let next = InterpolationType::Bezier {
                outgoing: BezierControlPoint {
                    influence: 0.333,
                    speed: 0.0,
                },
                incoming: BezierControlPoint {
                    influence: 0.333,
                    speed: 0.0,
                },
                custom_bezier: Some(preset.control_points()),
            };
            if key.interpolation != next {
                key.interpolation = next;
                return true;
            }
        }
    }
    false
}

/// Compute velocity (derivative) at each keyframe from value keyframes.
/// Returns Vec of (frame, velocity) pairs.
fn compute_velocity_curve(keyframes: &[(u32, f32)], fps: u32) -> Vec<(f32, f32)> {
    if keyframes.len() < 2 {
        return keyframes
            .iter()
            .map(|&(frame, _)| (frame as f32, 0.0))
            .collect();
    }
    let mut velocities = Vec::with_capacity(keyframes.len());
    for i in 0..keyframes.len() {
        let vel = if i == 0 {
            let dt = (keyframes[1].0 as f32 - keyframes[0].0 as f32) / fps as f32;
            if dt > 0.0 {
                (keyframes[1].1 - keyframes[0].1) / dt
            } else {
                0.0
            }
        } else if i == keyframes.len() - 1 {
            let dt = (keyframes[i].0 as f32 - keyframes[i - 1].0 as f32) / fps as f32;
            if dt > 0.0 {
                (keyframes[i].1 - keyframes[i - 1].1) / dt
            } else {
                0.0
            }
        } else {
            let dt = (keyframes[i + 1].0 as f32 - keyframes[i - 1].0 as f32) / fps as f32;
            if dt > 0.0 {
                (keyframes[i + 1].1 - keyframes[i - 1].1) / dt
            } else {
                0.0
            }
        };
        velocities.push((keyframes[i].0 as f32, vel));
    }
    velocities
}

#[cfg(test)]
mod tests {
    use super::{
        axis_3d, clamp_bezier_x, draw_graph_editor, map_layer_interpolation, move_and_set_channel,
        parse_effect_property, remove_camera_key, rove_keyframes, set_camera_key_ease,
        set_camera_key_interpolation, GraphKeyframeDrag,
    };
    use crate::core::keyframe::{InterpolationType, Keyframe};
    use crate::core::property::Animatable;
    use crate::core::timeline::{Layer, LayerType};

    #[test]
    fn bezier_x_clamp_handles_collapsed_bounds_without_panicking() {
        assert_eq!(clamp_bezier_x(0.5, 1.01, 1.0), 1.0);
        assert_eq!(clamp_bezier_x(0.5, 0.0, -0.01), 0.0);
        assert_eq!(clamp_bezier_x(0.5, 0.2, 0.8), 0.5);
    }

    #[test]
    fn active_drag_visual_follows_keyframe_after_sorting() {
        let mut layer = Layer::new(
            "layer".into(),
            "Layer".into(),
            LayerType::Solid {
                color: [1.0, 1.0, 1.0, 1.0],
            },
            120,
        );
        layer.transform.position = Animatable::new_animated(vec![
            Keyframe::new(0, [100.0, 540.0], InterpolationType::Linear),
            Keyframe::new(60, [600.0, 540.0], InterpolationType::Linear),
            Keyframe::new(90, [350.0, 540.0], InterpolationType::Linear),
        ]);
        let drag_id = eframe::egui::Id::new(("graph_keyframe_drag", &layer.id, "Position X"));
        let context = eframe::egui::Context::default();
        context.data_mut(|data| {
            data.insert_temp(
                drag_id,
                GraphKeyframeDrag {
                    original_frame: 30,
                    current_frame: 90,
                    original_value: 300.0,
                    current_value: 350.0,
                },
            )
        });
        let mut selected_property = Some("Position X".to_owned());
        let mut changed = false;
        let mut linked_tangent = true;
        let screen_rect = eframe::egui::Rect::from_min_size(
            eframe::egui::Pos2::ZERO,
            eframe::egui::vec2(900.0, 600.0),
        );
        let _ = context.run(
            eframe::egui::RawInput {
                screen_rect: Some(screen_rect),
                ..Default::default()
            },
            |context| {
                eframe::egui::CentralPanel::default().show(context, |ui| {
                    draw_graph_editor(
                        &mut selected_property,
                        ui,
                        120,
                        30,
                        &mut layer,
                        &mut changed,
                        &mut linked_tangent,
                    );
                });
            },
        );

        let rect_at = |frame| {
            context
                .data(|data| {
                    data.get_temp::<eframe::egui::Rect>(eframe::egui::Id::new((
                        "ae_graph_anchor_rect",
                        &layer.id,
                        "Position X",
                        frame,
                    )))
                })
                .expect("each visible keyframe has a rendered graph anchor")
        };
        let frame_60 = rect_at(60);
        let frame_90 = rect_at(90);
        assert!(
            frame_90.center().x - frame_60.center().x > 10.0,
            "the drag state must stay attached to its retimed key, not the key now at its old index"
        );
    }

    #[test]
    fn interpolation_commands_respect_selected_key_scope() {
        let mut layer = Layer::new(
            "layer".into(),
            "Layer".into(),
            LayerType::Solid {
                color: [1.0, 1.0, 1.0, 1.0],
            },
            60,
        );
        layer.transform.position = Animatable::new_animated(vec![
            Keyframe::new(0, [0.0, 0.0], InterpolationType::Linear),
            Keyframe::new(20, [20.0, 0.0], InterpolationType::Linear),
        ]);

        let mut make_hold = |interpolation: &mut InterpolationType| {
            *interpolation = InterpolationType::Hold;
        };
        assert!(map_layer_interpolation(
            &mut layer,
            "Position X",
            Some(20),
            &mut make_hold,
        ));
        let keys = layer.transform.position.keyframes().unwrap();
        assert_eq!(keys[0].interpolation, InterpolationType::Linear);
        assert_eq!(keys[1].interpolation, InterpolationType::Hold);

        let mut make_linear = |interpolation: &mut InterpolationType| {
            *interpolation = InterpolationType::Linear;
        };
        assert!(map_layer_interpolation(
            &mut layer,
            "Position X",
            None,
            &mut make_linear,
        ));
        assert!(layer
            .transform
            .position
            .keyframes()
            .unwrap()
            .iter()
            .all(|key| key.interpolation == InterpolationType::Linear));
    }

    #[test]
    fn move_and_set_channel_reacquires_keyframe_after_sorting() {
        let mut track = Animatable::new_animated(vec![
            Keyframe::new(10, [1.0, 2.0], InterpolationType::Linear),
            Keyframe::new(20, [3.0, 4.0], InterpolationType::Linear),
            Keyframe::new(30, [5.0, 6.0], InterpolationType::Linear),
        ]);

        assert!(move_and_set_channel(&mut track, 20, 40, 0, 9.0));
        let keyframes = track.keyframes().unwrap();
        assert_eq!(
            keyframes.iter().map(|key| key.frame).collect::<Vec<_>>(),
            vec![10, 30, 40]
        );
        assert_eq!(
            keyframes.iter().find(|key| key.frame == 30).unwrap().value,
            [5.0, 6.0]
        );
        assert_eq!(
            keyframes.iter().find(|key| key.frame == 40).unwrap().value,
            [9.0, 4.0]
        );
    }

    #[test]
    fn effect_property_uses_instance_id_and_component() {
        assert_eq!(
            parse_effect_property("fxid:effect-2::Glow Radius|1"),
            Some(("effect-2", "Glow Radius", Some(1)))
        );
        assert_eq!(
            parse_effect_property("fxid:effect-2::Opacity"),
            Some(("effect-2", "Opacity", None))
        );
        assert!(parse_effect_property("fx_Glow_Opacity").is_none());
    }

    #[test]
    fn rove_keyframes_uses_cumulative_spatial_distance() {
        let mut keys = vec![
            Keyframe::new(0, [0.0_f32, 0.0], InterpolationType::Linear),
            Keyframe::new(10, [100.0_f32, 0.0], InterpolationType::Linear),
            Keyframe::new(20, [900.0_f32, 0.0], InterpolationType::Linear),
        ];
        assert!(rove_keyframes(&mut keys, |a, b| (b[0] - a[0]).abs()));
        assert_eq!(
            keys.iter().map(|key| key.frame).collect::<Vec<_>>(),
            vec![0, 2, 20]
        );
    }

    #[test]
    fn velocity_preserves_fields_and_uses_segment_units() {
        use super::track_velocity;
        let mut track = Animatable::new_animated(vec![
            Keyframe::new(10, 100.0, InterpolationType::Linear),
            Keyframe::new(130, 500.0, InterpolationType::Linear),
        ]);
        let values = [25.0, 40.0, 50.0, 100.0];
        track_velocity(&mut track, 0, 60.0, |v| *v, Some(values)).unwrap();
        assert_eq!(
            track_velocity(&mut track, 0, 60.0, |v| *v, None),
            Some(values)
        );
        let mut edited = values;
        edited[0] = 30.0;
        track_velocity(&mut track, 0, 60.0, |v| *v, Some(edited)).unwrap();
        for (actual, expected) in track_velocity(&mut track, 0, 60.0, |v| *v, None)
            .unwrap()
            .into_iter()
            .zip(edited)
        {
            assert!((actual - expected).abs() < 1e-4);
        }
        let InterpolationType::Bezier {
            custom_bezier: Some(cp),
            ..
        } = track.keyframes().unwrap()[0].interpolation
        else {
            panic!("Expected Bezier");
        };
        for (actual, expected) in cp.into_iter().zip([0.4, 0.2, 0.7, 0.925]) {
            assert!((actual - expected).abs() < 1e-6);
        }
        assert!(track_velocity(&mut track, 1, 60.0, |v| *v, Some(values)).is_none());
    }

    #[test]
    fn velocity_updates_each_three_d_track_and_axis() {
        use super::layer_velocity;
        use crate::core::timeline::{Layer, LayerType};
        for property in ["3D Position Z", "3D Rotation Y", "3D Scale X"] {
            let mut layer = Layer::new(
                "layer".into(),
                "Layer".into(),
                LayerType::Solid { color: [1.0; 4] },
                120,
            );
            let track = Animatable::new_animated(vec![
                Keyframe::new(0, [0.0; 3], InterpolationType::Linear),
                Keyframe::new(60, [100.0, 200.0, 400.0], InterpolationType::Linear),
            ]);
            layer.transform_3d.position = track.clone();
            layer.transform_3d.rotation = track.clone();
            layer.transform_3d.scale = track;
            let values = [30.0, 40.0, 50.0, 100.0];
            assert_eq!(
                layer_velocity(&mut layer, property, 0, 60.0, Some(values)),
                Some(values)
            );
            for (actual, expected) in layer_velocity(&mut layer, property, 0, 60.0, None)
                .unwrap()
                .into_iter()
                .zip(values)
            {
                assert!((actual - expected).abs() < 1e-4);
            }
            let changed = if property.starts_with("3D Position") {
                &layer.transform_3d.position
            } else if property.starts_with("3D Rotation") {
                &layer.transform_3d.rotation
            } else {
                &layer.transform_3d.scale
            };
            let InterpolationType::Bezier {
                custom_bezier: Some(cp),
                ..
            } = changed.keyframes().unwrap()[0].interpolation
            else {
                panic!("Expected Bezier");
            };
            let expected = if property.ends_with('Z') {
                0.1
            } else if property.ends_with('Y') {
                0.2
            } else {
                0.4
            };
            assert!((cp[1] - expected).abs() < 1e-6);
        }
    }

    #[test]
    fn three_d_property_axis_selection_is_stable() {
        assert_eq!(axis_3d("3D Position X"), 0);
        assert_eq!(axis_3d("3D Rotation Y"), 1);
        assert_eq!(axis_3d("3D Scale Z"), 2);
    }

    #[test]
    fn unknown_axis_defaults_to_x_for_backward_compatibility() {
        assert_eq!(axis_3d("Position X"), 0);
        assert_eq!(axis_3d("3D Position"), 0);
    }

    #[test]
    fn removing_missing_camera_key_is_a_noop() {
        let mut track = Some(Animatable::new_animated(vec![Keyframe::new(
            10,
            50.0,
            InterpolationType::Linear,
        )]));

        assert!(!remove_camera_key(&mut track, 11));
        assert_eq!(
            track
                .as_ref()
                .and_then(|value| value.keyframes())
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn removing_last_camera_key_clears_empty_track() {
        let mut track = Some(Animatable::new_animated(vec![Keyframe::new(
            10,
            50.0,
            InterpolationType::Linear,
        )]));

        assert!(remove_camera_key(&mut track, 10));
        assert!(track.is_none());
    }

    #[test]
    fn camera_key_interpolation_changes_only_existing_key() {
        let mut track = Some(Animatable::new_animated(vec![Keyframe::new(
            10,
            50.0,
            InterpolationType::Linear,
        )]));

        assert!(set_camera_key_interpolation(
            &mut track,
            10,
            InterpolationType::Hold
        ));
        assert!(!set_camera_key_interpolation(
            &mut track,
            11,
            InterpolationType::Linear
        ));
        assert_eq!(
            track.as_ref().and_then(|value| value.keyframes()).unwrap()[0].interpolation,
            InterpolationType::Hold
        );
    }

    #[test]
    fn camera_key_ease_creates_bezier_only_at_current_frame() {
        let mut track = Some(Animatable::new_animated(vec![
            Keyframe::new(10, 50.0, InterpolationType::Linear),
            Keyframe::new(20, 60.0, InterpolationType::Linear),
        ]));

        assert!(set_camera_key_ease(
            &mut track,
            10,
            crate::core::keyframe::EasePreset::EaseIn
        ));
        assert!(matches!(
            track.as_ref().and_then(|value| value.keyframes()).unwrap()[0].interpolation,
            InterpolationType::Bezier { .. }
        ));
        assert!(matches!(
            track.as_ref().and_then(|value| value.keyframes()).unwrap()[1].interpolation,
            InterpolationType::Linear
        ));
        assert!(!set_camera_key_ease(
            &mut track,
            99,
            crate::core::keyframe::EasePreset::EaseOut
        ));
    }
}
