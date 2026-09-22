use crate::core::timeline::Layer;
use crate::ui::theme::colors;
use eframe::egui;

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
    if prop.ends_with('Z') {
        2
    } else if prop.ends_with('Y') {
        1
    } else {
        0
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
    let rest = property.strip_prefix("fxid:")?;
    let (base, component) = rest
        .rsplit_once('|')
        .map(|(base, channel)| (base, channel.parse::<usize>().ok()))
        .unwrap_or((rest, None));
    let (effect_id, parameter) = base.split_once("::")?;
    Some((effect_id, parameter, component))
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
    match property {
        "Position X" | "Position Y" => keyframe_frame(layer.transform.position.keyframes(), index),
        "Scale X" | "Scale Y" => keyframe_frame(layer.transform.scale.keyframes(), index),
        "Rotation" => keyframe_frame(layer.transform.rotation.keyframes(), index),
        "Opacity" => keyframe_frame(layer.transform.opacity.keyframes(), index),
        p if p.starts_with("3D Position") => {
            keyframe_frame(layer.transform_3d.position.keyframes(), index)
        }
        p if p.starts_with("3D Rotation") => {
            keyframe_frame(layer.transform_3d.rotation.keyframes(), index)
        }
        p if p.starts_with("3D Scale") => {
            keyframe_frame(layer.transform_3d.scale.keyframes(), index)
        }
        p if p.starts_with("PinX:") || p.starts_with("PinY:") => {
            let id = p.split(':').nth(1)?;
            layer
                .puppet_pins
                .iter()
                .find(|pin| pin.id == id)
                .and_then(|pin| keyframe_frame(pin.position.keyframes(), index))
        }
        p if is_effect_property(p) => {
            let (effect_id, parameter, _) = parse_effect_property(p)?;
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
        _ => None,
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
    match property {
        "Position X" | "Position Y" => layer
            .transform
            .position
            .keyframes_mut()
            .is_some_and(|keys| rove_keyframes(keys, |a, b| (b[0] - a[0]).hypot(b[1] - a[1]))),
        p if p.starts_with("3D Position") => layer
            .transform_3d
            .position
            .keyframes_mut()
            .is_some_and(|keys| {
                rove_keyframes(keys, |a, b| {
                    ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2) + (b[2] - a[2]).powi(2)).sqrt()
                })
            }),
        p if p.starts_with("PinX:") || p.starts_with("PinY:") => pin_anim_mut(layer, p)
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
    let id = prop
        .strip_prefix("PinX:")
        .or_else(|| prop.strip_prefix("PinY:"))?;
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

    match property {
        "Position X" | "Position Y" => apply(&mut layer.transform.position, interpolation),
        "Scale X" | "Scale Y" => apply(&mut layer.transform.scale, interpolation),
        "Rotation" => apply(&mut layer.transform.rotation, interpolation),
        "Opacity" => apply(&mut layer.transform.opacity, interpolation),
        p if p.starts_with("3D Position") => apply(&mut layer.transform_3d.position, interpolation),
        p if p.starts_with("3D Rotation") => apply(&mut layer.transform_3d.rotation, interpolation),
        p if p.starts_with("3D Scale") => apply(&mut layer.transform_3d.scale, interpolation),
        p if p.starts_with("Pin") => pin_anim_mut(layer, p)
            .map(|track| apply(track, interpolation))
            .unwrap_or(false),
        p if is_effect_property(p) => {
            let Some((effect_id, parameter_name, _)) = parse_effect_property(p) else {
                return false;
            };
            layer
                .effects
                .iter_mut()
                .find(|effect| effect.id == effect_id)
                .map(|effect| {
                    effect
                        .effect_type
                        .set_parameter_keyframe_interpolation(Some(parameter_name), interpolation)
                })
                .unwrap_or(false)
        }
        _ => false,
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

    match property {
        "Position X" | "Position Y" => {
            apply(&mut layer.transform.position, selected_frame, &mut map)
        }
        "Scale X" | "Scale Y" => apply(&mut layer.transform.scale, selected_frame, &mut map),
        "Rotation" => apply(&mut layer.transform.rotation, selected_frame, &mut map),
        "Opacity" => apply(&mut layer.transform.opacity, selected_frame, &mut map),
        p if p.starts_with("3D Position") => {
            apply(&mut layer.transform_3d.position, selected_frame, &mut map)
        }
        p if p.starts_with("3D Rotation") => {
            apply(&mut layer.transform_3d.rotation, selected_frame, &mut map)
        }
        p if p.starts_with("3D Scale") => {
            apply(&mut layer.transform_3d.scale, selected_frame, &mut map)
        }
        p if p.starts_with("Pin") => pin_anim_mut(layer, p)
            .map(|track| apply(track, selected_frame, &mut map))
            .unwrap_or(false),
        p if is_effect_property(p) => {
            let Some((effect_id, parameter_name, _)) = parse_effect_property(p) else {
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
        _ => false,
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
    let want = egui::vec2(48.0, 28.0);
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
    ui.painter().rect_filled(rect, 3.0, bg);
    ui.painter()
        .rect_stroke(rect, 3.0, egui::Stroke::new(1.0_f32, colors::BORDER_MEDIUM));
    let pad = 4.0;
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
        egui::Stroke::new(1.0_f32, colors::BORDER_MEDIUM),
    );
    let stroke_col = if resp.hovered() {
        colors::TEXT_ACCENT
    } else {
        colors::ACCENT_BLUE
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
            ui.painter()
                .line_segment([pr, p], egui::Stroke::new(1.5_f32, stroke_col));
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

pub fn draw_graph_editor(
    selected_property: &mut Option<String>,
    ui: &mut egui::Ui,
    duration_frames: u32,
    fps: u32,
    layer: &mut Layer,
    project_changed: &mut bool,
    linked_tangent: &mut bool,
) {
    let graph_height = 120.0f32;

    ui.group(|ui| {
        let graph_prop = selected_property
            .clone()
            .unwrap_or_else(|| "Position X".to_string());
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Graph Editor").strong());
            let prop_name = selected_property
                .clone()
                .unwrap_or_else(|| "Position X".to_string());
            egui::ComboBox::from_id_salt("graph_prop_select_module")
                .selected_text(&prop_name)
                .show_ui(ui, |ui| {
                    let mut props: Vec<(String, String)> = [
                        ("Position X", "Position X"),
                        ("Position Y", "Position Y"),
                        ("Scale X", "Scale X"),
                        ("Scale Y", "Scale Y"),
                        ("Rotation", "Rotation"),
                        ("Opacity", "Opacity"),
                        ("3D Position X", "3D Position X"),
                        ("3D Position Y", "3D Position Y"),
                        ("3D Position Z", "3D Position Z"),
                        ("3D Rotation X", "3D Rotation X"),
                        ("3D Rotation Y", "3D Rotation Y"),
                        ("3D Rotation Z", "3D Rotation Z"),
                        ("3D Scale X", "3D Scale X"),
                        ("3D Scale Y", "3D Scale Y"),
                        ("3D Scale Z", "3D Scale Z"),
                    ]
                    .iter()
                    .map(|(a, b)| (a.to_string(), b.to_string()))
                    .collect();
                    for pin in &layer.puppet_pins {
                        props.push((
                            format!("PinX:{}", pin.id),
                            format!("\u{1f9f7} {} X", pin.name),
                        ));
                        props.push((
                            format!("PinY:{}", pin.id),
                            format!("\u{1f9f7} {} Y", pin.name),
                        ));
                    }
                    for effect in &layer.effects {
                        for (label, parameter) in effect.effect_type.animatable_params_ref() {
                            let channels: &[(&str, usize)] = match parameter {
                                crate::core::effect_params::ParamRefRef::Scalar(_) => &[("", 0)],
                                crate::core::effect_params::ParamRefRef::Vec2(_) => {
                                    &[(" X", 0), (" Y", 1)]
                                }
                                crate::core::effect_params::ParamRefRef::Vec3(_) => {
                                    &[(" X", 0), (" Y", 1), (" Z", 2)]
                                }
                                crate::core::effect_params::ParamRefRef::Vec4Color(_) => {
                                    &[(" R", 0), (" G", 1), (" B", 2), (" A", 3)]
                                }
                            };
                            for (suffix, channel) in channels {
                                let key = if suffix.is_empty() {
                                    format!("fxid:{}::{}", effect.id, label)
                                } else {
                                    format!("fxid:{}::{}|{}", effect.id, label, channel)
                                };
                                props.push((
                                    key,
                                    format!("⚙ {} / {}{}", effect.name, label, suffix),
                                ));
                            }
                        }
                    }
                    let label_of = |key: &str| -> String {
                        props
                            .iter()
                            .find(|(k, _)| k == key)
                            .map(|(_, l)| l.clone())
                            .unwrap_or_else(|| key.to_string())
                    };
                    let sel_label = label_of(&prop_name);
                    ui.label(egui::RichText::new(&sel_label).weak());
                    for (key, lbl) in &props {
                        if ui.selectable_label(prop_name == *key, lbl).clicked() {
                            *selected_property = Some(key.clone());
                        }
                    }
                });

            ui.add_space(8.0);
            ui.checkbox(linked_tangent, "🔗 Link");

            // ── Visual Ease Presets Palette ──
            fn apply_preset_to_layer(
                layer: &mut Layer,
                prop: &str,
                preset: crate::core::keyframe::EasePreset,
                selected_frame: Option<u32>,
            ) {
                use crate::core::keyframe::{BezierControlPoint, InterpolationType};
                use crate::core::property::Animatable;
                let pts = preset.control_points();
                fn apply<T>(
                    kfs: &mut [crate::core::keyframe::Keyframe<T>],
                    pts: [f32; 4],
                    selected_frame: Option<u32>,
                ) {
                    for kf in kfs.iter_mut() {
                        if selected_frame.is_some_and(|frame| frame != kf.frame) {
                            continue;
                        }
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
                }

                match prop {
                    "Position X" | "Position Y" => {
                        if let Animatable::Animated(ref mut kfs) = layer.transform.position {
                            apply(kfs, pts, selected_frame);
                        }
                    }
                    "Scale X" | "Scale Y" => {
                        if let Animatable::Animated(ref mut kfs) = layer.transform.scale {
                            apply(kfs, pts, selected_frame);
                        }
                    }
                    "Rotation" => {
                        if let Animatable::Animated(ref mut kfs) = layer.transform.rotation {
                            apply(kfs, pts, selected_frame);
                        }
                    }
                    "Opacity" => {
                        if let Animatable::Animated(ref mut kfs) = layer.transform.opacity {
                            apply(kfs, pts, selected_frame);
                        }
                    }
                    p if p.starts_with("3D Position") => {
                        if let Animatable::Animated(ref mut kfs) = layer.transform_3d.position {
                            apply(kfs, pts, selected_frame);
                        }
                    }
                    p if p.starts_with("3D Rotation") => {
                        if let Animatable::Animated(ref mut kfs) = layer.transform_3d.rotation {
                            apply(kfs, pts, selected_frame);
                        }
                    }
                    p if p.starts_with("3D Scale") => {
                        if let Animatable::Animated(ref mut kfs) = layer.transform_3d.scale {
                            apply(kfs, pts, selected_frame);
                        }
                    }
                    p if p.starts_with("Pin") => {
                        if let Some(Animatable::Animated(ref mut kfs)) = pin_anim_mut(layer, p) {
                            apply(kfs, pts, selected_frame);
                        }
                    }
                    p if is_effect_property(p) => {
                        if let Some((effect_id, parameter_name, _)) = parse_effect_property(p) {
                            if let Some(effect) = layer
                                .effects
                                .iter_mut()
                                .find(|effect| effect.id == effect_id)
                            {
                                let interpolation = InterpolationType::Bezier {
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
                                if let Some(frame) = selected_frame {
                                    effect
                                        .effect_type
                                        .set_parameter_keyframe_interpolation_at_frame(
                                            parameter_name,
                                            frame,
                                            interpolation,
                                        );
                                } else {
                                    effect.effect_type.set_parameter_keyframe_interpolation(
                                        Some(parameter_name),
                                        interpolation,
                                    );
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }

            let active_prop = selected_property
                .clone()
                .unwrap_or_else(|| "Position X".to_string());
            let ease_scope_id = egui::Id::new(("graph_ease_scope", &layer.id, &active_prop));
            let mut apply_all_keys = ui.ctx().data(|d| d.get_temp(ease_scope_id).unwrap_or(true));
            ui.horizontal(|ui| {
                ui.checkbox(&mut apply_all_keys, "All keys");
                ui.label(egui::RichText::new("off = hovered key only").small().weak());
            });
            ui.ctx()
                .data_mut(|d| d.insert_temp(ease_scope_id, apply_all_keys));
            let hovered_index: Option<usize> = ui.ctx().data(|d| {
                d.get_temp(egui::Id::new((
                    "ae_graph_hovered_kf",
                    &layer.id,
                    &active_prop,
                )))
            });
            let ease_target_frame = if apply_all_keys {
                None
            } else {
                hovered_index.and_then(|index| graph_keyframe_frame(layer, &active_prop, index))
            };

            ui.horizontal_wrapped(|ui| {
                for (lbl, short, preset, tip) in [
                    (
                        "⚡ Easy Ease (F9)",
                        "Easy Ease",
                        crate::core::keyframe::EasePreset::Standard,
                        "Standard symmetric ease [0.25, 0.1, 0.25, 1.0]",
                    ),
                    (
                        "↗ In",
                        "In",
                        crate::core::keyframe::EasePreset::EaseIn,
                        "Ease In (slow start, fast end)",
                    ),
                    (
                        "↘ Out",
                        "Out",
                        crate::core::keyframe::EasePreset::EaseOut,
                        "Ease Out (fast start, slow end)",
                    ),
                    (
                        "🌊 Sine",
                        "Sine",
                        crate::core::keyframe::EasePreset::Sine,
                        "Ultra smooth Sine ease",
                    ),
                    (
                        "🚀 Fast Out",
                        "Fast Out",
                        crate::core::keyframe::EasePreset::FastOut,
                        "Quick initial burst then smooth decelerate",
                    ),
                    (
                        "🎯 Overshoot",
                        "Overshoot",
                        crate::core::keyframe::EasePreset::Overshoot,
                        "Spring overshoot past target value",
                    ),
                    (
                        "🏀 Bounce",
                        "Bounce",
                        crate::core::keyframe::EasePreset::Bounce,
                        "Physical single bounce easing",
                    ),
                    (
                        "🪀 Elastic",
                        "Elastic",
                        crate::core::keyframe::EasePreset::Elastic,
                        "Elastic spring recoil easing",
                    ),
                ] {
                    ui.vertical(|ui| {
                        ui.set_min_width(52.0);
                        let thumb =
                            draw_ease_thumbnail(ui, preset).on_hover_text(format!("{lbl}\n{tip}"));
                        let label = ui
                            .small_button(short)
                            .on_hover_text(format!("{lbl}\n{tip}"));
                        if thumb.clicked() || label.clicked() {
                            apply_preset_to_layer(layer, &active_prop, preset, ease_target_frame);
                            *project_changed = true;
                        }
                    });
                }
            });

            ui.add_space(4.0);
            let rove_supported = matches!(active_prop.as_str(), "Position X" | "Position Y")
                || active_prop.starts_with("3D Position")
                || active_prop.starts_with("PinX:")
                || active_prop.starts_with("PinY:");
            if ui
                .add_enabled(rove_supported, egui::Button::new("〰 Rove Across Time"))
                .on_hover_text(if rove_supported {
                    "Redistribute position keyframes by cumulative spatial distance"
                } else {
                    "Rove is available for position and puppet-pin tracks"
                })
                .clicked()
            {
                *project_changed |= rove_across_time(layer, &active_prop);
            }

            if ui
                .button("⇄ Reverse Keys")
                .on_hover_text("Reverse keyframe order in time (values stay, timing flips)")
                .clicked()
            {
                let changed = match selected_property
                    .clone()
                    .unwrap_or_else(|| "Position X".to_string())
                    .as_str()
                {
                    "Position X" | "Position Y" => reverse_keyframes(&mut layer.transform.position),
                    "Scale X" | "Scale Y" => reverse_keyframes(&mut layer.transform.scale),
                    "Rotation" => reverse_keyframes(&mut layer.transform.rotation),
                    "Opacity" => reverse_keyframes(&mut layer.transform.opacity),
                    p if p.starts_with("3D Position") => {
                        reverse_keyframes(&mut layer.transform_3d.position)
                    }
                    p if p.starts_with("3D Rotation") => {
                        reverse_keyframes(&mut layer.transform_3d.rotation)
                    }
                    p if p.starts_with("3D Scale") => {
                        reverse_keyframes(&mut layer.transform_3d.scale)
                    }
                    p if p.starts_with("Pin") => {
                        pin_anim_mut(layer, p).is_some_and(reverse_keyframes)
                    }
                    p if is_effect_property(p) => reverse_effect_property(layer, p),
                    _ => false,
                };
                *project_changed |= changed;
            }

            ui.add_space(4.0);
            if ui
                .button("⚡ Mirror Ease")
                .on_hover_text("Symmetrically mirror Ease In / Ease Out handles")
                .clicked()
            {
                use crate::core::keyframe::InterpolationType;

                let mut mirror_custom_bezier = |interpolation: &mut InterpolationType| {
                    if let InterpolationType::Bezier {
                        custom_bezier: Some(ref mut pts),
                        ..
                    } = interpolation
                    {
                        let mirrored = [1.0 - pts[2], 1.0 - pts[3], 1.0 - pts[0], 1.0 - pts[1]];
                        *pts = mirrored;
                    }
                };
                *project_changed |= map_layer_interpolation(
                    layer,
                    &active_prop,
                    ease_target_frame,
                    &mut mirror_custom_bezier,
                );
            }

            ui.add_space(4.0);
            if ui
                .button("↘ Ease In")
                .on_hover_text("Flatten incoming tangent — keyframe eases into its value")
                .clicked()
            {
                use crate::core::keyframe::InterpolationType;
                let mut ease_in = |interpolation: &mut InterpolationType| {
                    if let InterpolationType::Bezier {
                        ref mut outgoing,
                        ref mut incoming,
                        ref mut custom_bezier,
                        ..
                    } = *interpolation
                    {
                        outgoing.influence = 0.0;
                        outgoing.speed = 0.0;
                        incoming.influence = 0.333;
                        incoming.speed = 0.0;
                        *custom_bezier = Some([0.0, 0.0, 0.33, 1.0]);
                    }
                };
                *project_changed |=
                    map_layer_interpolation(layer, &active_prop, ease_target_frame, &mut ease_in);
            }
            if ui
                .button("↗ Ease Out")
                .on_hover_text("Flatten outgoing tangent — keyframe eases out of its value")
                .clicked()
            {
                use crate::core::keyframe::InterpolationType;
                let mut ease_out = |interpolation: &mut InterpolationType| {
                    if let InterpolationType::Bezier {
                        ref mut outgoing,
                        ref mut incoming,
                        ref mut custom_bezier,
                        ..
                    } = *interpolation
                    {
                        outgoing.influence = 0.333;
                        outgoing.speed = 0.0;
                        incoming.influence = 0.0;
                        incoming.speed = 0.0;
                        *custom_bezier = Some([0.67, 0.0, 1.0, 1.0]);
                    }
                };
                *project_changed |=
                    map_layer_interpolation(layer, &active_prop, ease_target_frame, &mut ease_out);
            }

            ui.add_space(8.0);
            // ── Speed Graph vs Value Graph Mode Switcher ──
            let mode_id = egui::Id::new("ae_graph_mode_select");
            let mut current_mode = ui.ctx().data(|d| d.get_temp::<i32>(mode_id).unwrap_or(0));
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Mode:")
                        .small()
                        .color(colors::TEXT_SECONDARY),
                );
                if ui
                    .selectable_label(current_mode == 0, "⚡ Speed Graph")
                    .clicked()
                {
                    current_mode = 0;
                    ui.ctx().data_mut(|d| d.insert_temp(mode_id, 0));
                }
                if ui
                    .selectable_label(current_mode == 1, "📈 Value Graph")
                    .clicked()
                {
                    current_mode = 1;
                    ui.ctx().data_mut(|d| d.insert_temp(mode_id, 1));
                }
            });

            ui.collapsing("🎯 Keyframe Velocity / Influence", |ui| {
                let prop = selected_property.as_deref().unwrap_or("Position X");
                let target_id = egui::Id::new(("ae_graph_hovered_kf", &layer.id, prop));
                let index: Option<usize> = ui.ctx().data(|d| d.get_temp(target_id));
                if let Some((index, mut values)) = index.and_then(|index| {
                    layer_velocity(layer, prop, index, fps as f32, None)
                        .map(|values| (index, values))
                }) {
                    let mut changed = false;
                    ui.horizontal(|ui| {
                        ui.label("Incoming:");
                        changed |= ui
                            .add(
                                egui::DragValue::new(&mut values[0])
                                    .range(0.1..=100.0)
                                    .speed(0.5)
                                    .prefix("Inf: ")
                                    .suffix("%"),
                            )
                            .changed();
                        changed |= ui
                            .add(
                                egui::DragValue::new(&mut values[2])
                                    .speed(1.0)
                                    .prefix("Spd: ")
                                    .suffix(" units/s"),
                            )
                            .changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Outgoing:");
                        changed |= ui
                            .add(
                                egui::DragValue::new(&mut values[1])
                                    .range(0.1..=100.0)
                                    .speed(0.5)
                                    .prefix("Inf: ")
                                    .suffix("%"),
                            )
                            .changed();
                        changed |= ui
                            .add(
                                egui::DragValue::new(&mut values[3])
                                    .speed(1.0)
                                    .prefix("Spd: ")
                                    .suffix(" units/s"),
                            )
                            .changed();
                    });
                    if changed {
                        *project_changed |=
                            layer_velocity(layer, prop, index, fps as f32, Some(values)).is_some();
                    }
                } else {
                    ui.label("Hover a keyframe with a following segment to edit velocity.");
                }

                ui.separator();
                ui.horizontal(|ui| {
                    if ui
                        .small_button("📐 Linear")
                        .on_hover_text("Convert keyframes to linear interpolation")
                        .clicked()
                    {
                        *project_changed |= set_layer_interpolation(
                            layer,
                            selected_property.as_deref().unwrap_or("Position X"),
                            crate::core::keyframe::InterpolationType::Linear,
                        );
                    }
                    if ui
                        .small_button("🌊 Auto Bezier")
                        .on_hover_text("Smooth keyframe tangents automatically")
                        .clicked()
                    {
                        *project_changed |= set_layer_interpolation(
                            layer,
                            selected_property.as_deref().unwrap_or("Position X"),
                            crate::core::keyframe::InterpolationType::Bezier {
                                outgoing: crate::core::keyframe::BezierControlPoint::default(),
                                incoming: crate::core::keyframe::BezierControlPoint::default(),
                                custom_bezier: None,
                            },
                        );
                    }
                    if ui
                        .small_button("🛑 Hold")
                        .on_hover_text("Hold keyframe value until next keyframe")
                        .clicked()
                    {
                        *project_changed |= set_layer_interpolation(
                            layer,
                            selected_property.as_deref().unwrap_or("Position X"),
                            crate::core::keyframe::InterpolationType::Hold,
                        );
                    }
                });
            });
        });

        let total_f = duration_frames.max(1);

        // Detect speed graph vs value graph mode (0 = speed graph, 1 = value graph)
        let speed_graph_mode = ui.ctx().data(|d| {
            d.get_temp::<i32>(egui::Id::new("ae_graph_mode_select"))
                .unwrap_or(0)
        }) == 0;

        // Sample values along timeline duration for drawing curve (screen-adaptive step)
        let max_samples = 2000usize;
        let step = (total_f as usize / max_samples).max(1) as u32;
        let mut samples = Vec::with_capacity((total_f / step) as usize + 2);
        let mut f = 0u32;
        while f <= total_f {
            let raw_val = match graph_prop.as_str() {
                "Position X" => layer.transform.position.evaluate(f)[0],
                "Position Y" => layer.transform.position.evaluate(f)[1],
                "Scale X" => layer.transform.scale.evaluate(f)[0],
                "Scale Y" => layer.transform.scale.evaluate(f)[1],
                "Rotation" => layer.transform.rotation.evaluate(f),
                "Opacity" => layer.transform.opacity.evaluate(f),
                p if p.starts_with("3D Position") => {
                    layer.transform_3d.position.evaluate(f)[axis_3d(p)]
                }
                p if p.starts_with("3D Rotation") => {
                    layer.transform_3d.rotation.evaluate(f)[axis_3d(p)]
                }
                p if p.starts_with("3D Scale") => layer.transform_3d.scale.evaluate(f)[axis_3d(p)],
                p if p.starts_with("PinX:") || p.starts_with("PinY:") => {
                    let ci = usize::from(p.starts_with("PinY:"));
                    let pid = p.split(':').nth(1).unwrap_or("");
                    layer
                        .puppet_pins
                        .iter()
                        .find(|pp| pp.id == pid)
                        .map(|pp| pp.position.evaluate(f)[ci])
                        .unwrap_or(0.0)
                }
                p if is_effect_property(p) => effect_parameter_channel_value(layer, p, f),
                _ => layer.transform.position.evaluate(f)[0],
            };
            let val = if raw_val.is_nan() { 0.0 } else { raw_val };
            samples.push((f, val));
            if f < total_f && f + step > total_f {
                f = total_f;
            } else {
                f += step;
            }
        }

        // Compute per-keyframe velocity when in speed graph mode
        let keyframes_ref: Vec<(u32, f32)> = match graph_prop.as_str() {
            "Position X" => layer
                .transform
                .position
                .keyframes()
                .map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.value[0])).collect())
                .unwrap_or_default(),
            "Position Y" => layer
                .transform
                .position
                .keyframes()
                .map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.value[1])).collect())
                .unwrap_or_default(),
            "Scale X" => layer
                .transform
                .scale
                .keyframes()
                .map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.value[0])).collect())
                .unwrap_or_default(),
            "Scale Y" => layer
                .transform
                .scale
                .keyframes()
                .map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.value[1])).collect())
                .unwrap_or_default(),
            "Rotation" => layer
                .transform
                .rotation
                .keyframes()
                .map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.value)).collect())
                .unwrap_or_default(),
            "Opacity" => layer
                .transform
                .opacity
                .keyframes()
                .map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.value)).collect())
                .unwrap_or_default(),
            p if p.starts_with("3D Position") => layer
                .transform_3d
                .position
                .keyframes()
                .map(|kfs| {
                    kfs.iter()
                        .map(|kf| (kf.frame, kf.value[axis_3d(p)]))
                        .collect()
                })
                .unwrap_or_default(),
            p if p.starts_with("3D Rotation") => layer
                .transform_3d
                .rotation
                .keyframes()
                .map(|kfs| {
                    kfs.iter()
                        .map(|kf| (kf.frame, kf.value[axis_3d(p)]))
                        .collect()
                })
                .unwrap_or_default(),
            p if p.starts_with("3D Scale") => layer
                .transform_3d
                .scale
                .keyframes()
                .map(|kfs| {
                    kfs.iter()
                        .map(|kf| (kf.frame, kf.value[axis_3d(p)]))
                        .collect()
                })
                .unwrap_or_default(),
            p if p.starts_with("PinX:") || p.starts_with("PinY:") => {
                let ci = usize::from(p.starts_with("PinY:"));
                let pid = p.split(':').nth(1).unwrap_or("");
                layer
                    .puppet_pins
                    .iter()
                    .find(|pp| pp.id == pid)
                    .and_then(|pp| pp.position.keyframes())
                    .map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.value[ci])).collect())
                    .unwrap_or_default()
            }
            p if is_effect_property(p) => effect_parameter_channel_keyframes(layer, p),
            _ => vec![],
        };

        // Allocate drawing region
        let (rect, graph_response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), graph_height),
            egui::Sense::click_and_drag(),
        );
        #[cfg(test)]
        ui.ctx().data_mut(|d| {
            d.insert_temp(
                egui::Id::new(("ae_graph_canvas_rect", &layer.id, &graph_prop)),
                rect,
            );
        });
        ui.painter()
            .rect_filled(rect, 4.0, egui::Color32::from_gray(25));
        ui.painter().rect_stroke(
            rect,
            4.0,
            egui::Stroke::new(1.0_f32, egui::Color32::from_gray(50)),
        );

        let min_val = samples
            .iter()
            .map(|(_, v)| *v)
            .fold(f32::INFINITY, f32::min);
        let max_val = samples
            .iter()
            .map(|(_, v)| *v)
            .fold(f32::NEG_INFINITY, f32::max);
        let val_range = (max_val - min_val).max(0.001);

        // Min/max value readouts on the right edge (curve readability)
        if !speed_graph_mode {
            let mono = egui::FontId::monospace(9.0);
            ui.painter().text(
                egui::pos2(rect.right() - 4.0, rect.top() + 3.0),
                egui::Align2::RIGHT_TOP,
                format!("{:.1}", max_val),
                mono.clone(),
                colors::TEXT_MUTED,
            );
            ui.painter().text(
                egui::pos2(rect.right() - 4.0, rect.bottom() - 3.0),
                egui::Align2::RIGHT_BOTTOM,
                format!("{:.1}", min_val),
                mono,
                colors::TEXT_MUTED,
            );
        }

        // Convert keyframe time/value to screen space coordinates inside the allocated rect
        let display_fps = fps.max(1);
        let points: Vec<egui::Pos2> = if speed_graph_mode {
            // Speed Graph: compute velocity curve and map to screen
            let vel_curve = compute_velocity_curve(&keyframes_ref, display_fps);
            let vel_min = vel_curve
                .iter()
                .map(|(_, v)| *v)
                .fold(f32::INFINITY, f32::min);
            let vel_max = vel_curve
                .iter()
                .map(|(_, v)| *v)
                .fold(f32::NEG_INFINITY, f32::max);
            let vel_range = (vel_max - vel_min).abs().max(0.001);

            // Update readout labels for velocity range
            {
                let mono = egui::FontId::monospace(9.0);
                ui.painter().text(
                    egui::pos2(rect.right() - 4.0, rect.top() + 3.0),
                    egui::Align2::RIGHT_TOP,
                    format!("{:.1} v/s", vel_max),
                    mono.clone(),
                    colors::TEXT_MUTED,
                );
                ui.painter().text(
                    egui::pos2(rect.right() - 4.0, rect.bottom() - 3.0),
                    egui::Align2::RIGHT_BOTTOM,
                    format!("{:.1} v/s", vel_min),
                    mono,
                    colors::TEXT_MUTED,
                );
            }

            vel_curve
                .iter()
                .map(|&(frame, vel)| {
                    let x = rect.left() + (frame / total_f as f32) * rect.width();
                    let y =
                        rect.bottom() - 4.0 - ((vel - vel_min) / vel_range) * (rect.height() - 8.0);
                    egui::pos2(x, y)
                })
                .collect()
        } else {
            // Value Graph: original value curve
            samples
                .iter()
                .map(|&(f, v)| {
                    let x = rect.left() + (f as f32 / total_f as f32) * rect.width();
                    let y =
                        rect.bottom() - 4.0 - ((v - min_val) / val_range) * (rect.height() - 8.0);
                    egui::pos2(x, y)
                })
                .collect()
        };

        // Draw graph spline segments (Value Curve)
        for window in points.windows(2) {
            ui.painter().line_segment(
                [window[0], window[1]],
                egui::Stroke::new(2.0_f32, colors::TIMELINE_KEYFRAME),
            );
        }

        // ── Click on empty graph area → create a keyframe at that frame/value ──
        {
            use crate::core::keyframe::{InterpolationType as GInterp, Keyframe as GKeyframe};

            let x_of = |f: u32| rect.left() + (f as f32 / total_f as f32) * rect.width();
            let y_of =
                |v: f32| rect.bottom() - 4.0 - ((v - min_val) / val_range) * (rect.height() - 8.0);

            if graph_response.clicked() {
                if let Some(pos) = graph_response.interact_pointer_pos() {
                    if rect.contains(pos) {
                        // Existing anchor proximity guard: clicks on anchors belong to the anchor drag
                        let chan_kfs = |p: &crate::core::property::Animatable<[f32; 2]>,
                                        ci: usize|
                         -> Vec<(u32, f32)> {
                            p.keyframes()
                                .map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.value[ci])).collect())
                                .unwrap_or_default()
                        };
                        let scalar_kfs =
                            |p: &crate::core::property::Animatable<f32>| -> Vec<(u32, f32)> {
                                p.keyframes()
                                    .map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.value)).collect())
                                    .unwrap_or_default()
                            };
                        let anchor_pts: Vec<(u32, f32)> = match graph_prop.as_str() {
                            "Position X" => chan_kfs(&layer.transform.position, 0),
                            "Position Y" => chan_kfs(&layer.transform.position, 1),
                            "Scale X" => chan_kfs(&layer.transform.scale, 0),
                            "Scale Y" => chan_kfs(&layer.transform.scale, 1),
                            "Rotation" => scalar_kfs(&layer.transform.rotation),
                            "Opacity" => scalar_kfs(&layer.transform.opacity),
                            p if p.starts_with("3D Position") => layer
                                .transform_3d
                                .position
                                .keyframes()
                                .map(|k| {
                                    k.iter()
                                        .map(|kf| (kf.frame, kf.value[axis_3d(p)]))
                                        .collect()
                                })
                                .unwrap_or_default(),
                            p if p.starts_with("3D Rotation") => layer
                                .transform_3d
                                .rotation
                                .keyframes()
                                .map(|k| {
                                    k.iter()
                                        .map(|kf| (kf.frame, kf.value[axis_3d(p)]))
                                        .collect()
                                })
                                .unwrap_or_default(),
                            p if p.starts_with("3D Scale") => layer
                                .transform_3d
                                .scale
                                .keyframes()
                                .map(|k| {
                                    k.iter()
                                        .map(|kf| (kf.frame, kf.value[axis_3d(p)]))
                                        .collect()
                                })
                                .unwrap_or_default(),
                            p if p.starts_with("PinX:") || p.starts_with("PinY:") => {
                                let ci = usize::from(p.starts_with("PinY:"));
                                let pid = p.split(':').nth(1).unwrap_or("");
                                layer
                                    .puppet_pins
                                    .iter()
                                    .find(|pp| pp.id == pid)
                                    .map(|pp| chan_kfs(&pp.position, ci))
                                    .unwrap_or_default()
                            }
                            p if is_effect_property(p) => {
                                effect_parameter_channel_keyframes(layer, p)
                            }
                            _ => vec![],
                        };
                        let near_anchor = anchor_pts
                            .iter()
                            .any(|&(f, v)| egui::pos2(x_of(f), y_of(v)).distance(pos) < 8.0);

                        if !near_anchor {
                            let new_frame = (((pos.x - rect.left()) / rect.width())
                                * total_f as f32)
                                .round()
                                .clamp(0.0, total_f as f32)
                                as u32;
                            let new_val = min_val
                                + ((rect.bottom() - 4.0 - pos.y) / (rect.height() - 8.0))
                                    * val_range;
                            match graph_prop.as_str() {
                                "Position X" => {
                                    let mut v = layer.transform.position.evaluate(new_frame);
                                    v[0] = new_val;
                                    layer.transform.position.add_keyframe(GKeyframe::new(
                                        new_frame,
                                        v,
                                        GInterp::Linear,
                                    ));
                                }
                                "Position Y" => {
                                    let mut v = layer.transform.position.evaluate(new_frame);
                                    v[1] = new_val;
                                    layer.transform.position.add_keyframe(GKeyframe::new(
                                        new_frame,
                                        v,
                                        GInterp::Linear,
                                    ));
                                }
                                "Scale X" => {
                                    let mut v = layer.transform.scale.evaluate(new_frame);
                                    v[0] = new_val;
                                    layer.transform.scale.add_keyframe(GKeyframe::new(
                                        new_frame,
                                        v,
                                        GInterp::Linear,
                                    ));
                                }
                                "Scale Y" => {
                                    let mut v = layer.transform.scale.evaluate(new_frame);
                                    v[1] = new_val;
                                    layer.transform.scale.add_keyframe(GKeyframe::new(
                                        new_frame,
                                        v,
                                        GInterp::Linear,
                                    ));
                                }
                                "Rotation" => {
                                    layer.transform.rotation.add_keyframe(GKeyframe::new(
                                        new_frame,
                                        new_val,
                                        GInterp::Linear,
                                    ));
                                }
                                "Opacity" => {
                                    layer.transform.opacity.add_keyframe(GKeyframe::new(
                                        new_frame,
                                        new_val.clamp(0.0, 100.0),
                                        GInterp::Linear,
                                    ));
                                }
                                p if p.starts_with("3D Position") => {
                                    let mut v = layer.transform_3d.position.evaluate(new_frame);
                                    v[axis_3d(p)] = new_val;
                                    layer.transform_3d.position.add_keyframe(GKeyframe::new(
                                        new_frame,
                                        v,
                                        GInterp::Linear,
                                    ));
                                }
                                p if p.starts_with("3D Rotation") => {
                                    let mut v = layer.transform_3d.rotation.evaluate(new_frame);
                                    v[axis_3d(p)] = new_val;
                                    layer.transform_3d.rotation.add_keyframe(GKeyframe::new(
                                        new_frame,
                                        v,
                                        GInterp::Linear,
                                    ));
                                }
                                p if p.starts_with("3D Scale") => {
                                    let mut v = layer.transform_3d.scale.evaluate(new_frame);
                                    v[axis_3d(p)] = new_val;
                                    layer.transform_3d.scale.add_keyframe(GKeyframe::new(
                                        new_frame,
                                        v,
                                        GInterp::Linear,
                                    ));
                                }
                                p if p.starts_with("PinX:") || p.starts_with("PinY:") => {
                                    if let Some(pin) = pin_anim_mut(layer, p) {
                                        let ci = usize::from(p.starts_with("PinY:"));
                                        let mut v = pin.evaluate(new_frame);
                                        v[ci] = new_val;
                                        pin.add_keyframe(GKeyframe::new(
                                            new_frame,
                                            v,
                                            GInterp::Linear,
                                        ));
                                    }
                                }
                                p if is_effect_property(p) => {
                                    let Some((effect_id, parameter_name, component)) =
                                        parse_effect_property(p)
                                    else {
                                        return;
                                    };
                                    if let Some(effect) = layer
                                        .effects
                                        .iter_mut()
                                        .find(|effect| effect.id == effect_id)
                                    {
                                        if let Some(component) = component {
                                            effect.effect_type.set_parameter_component_keyframe(
                                                parameter_name,
                                                component,
                                                new_frame,
                                                new_val,
                                            );
                                        } else {
                                            effect.effect_type.set_scalar_parameter_keyframe(
                                                parameter_name,
                                                new_frame,
                                                new_val,
                                            );
                                        }
                                    }
                                }
                                _ => {}
                            }
                            *project_changed = true;
                        }
                    }
                }
            }
        }

        // Draw Speed Graph Velocity Line (First Derivative v(t) = dy/dt)
        // Skip overlay when in speed graph mode (main curve already shows velocity)
        if !speed_graph_mode {
            let mut max_speed = 0.0f32;
            let mut speed_pts = Vec::with_capacity(points.len());
            for win in samples.windows(2) {
                let dt = (win[1].0 as f32 - win[0].0 as f32).max(1.0);
                let speed = ((win[1].1 - win[0].1) / dt).abs();
                max_speed = max_speed.max(speed);
                speed_pts.push(speed);
            }

            if max_speed > 0.001 {
                for i in 0..speed_pts.len() {
                    let sx = rect.left() + (i as f32 / total_f as f32) * rect.width();
                    let sy =
                        rect.bottom() - 4.0 - (speed_pts[i] / max_speed) * (rect.height() - 16.0);
                    let p1 = egui::pos2(sx, sy);

                    let next_i = (i + 1).min(speed_pts.len() - 1);
                    let nsx = rect.left() + (next_i as f32 / total_f as f32) * rect.width();
                    let nsy = rect.bottom()
                        - 4.0
                        - (speed_pts[next_i] / max_speed) * (rect.height() - 16.0);
                    let p2 = egui::pos2(nsx, nsy);

                    ui.painter()
                        .line_segment([p1, p2], egui::Stroke::new(1.2_f32, colors::MOTION_PATH));
                }

                // Peak Speed Badge HUD
                let speed_badge_pos = egui::pos2(rect.right() - 110.0, rect.top() + 6.0);
                ui.painter().text(
                    speed_badge_pos,
                    egui::Align2::LEFT_TOP,
                    format!("⚡ Peak: {:.0} px/s", max_speed * display_fps as f32),
                    egui::FontId::monospace(10.0),
                    colors::MOTION_PATH,
                );
            }
        }

        // Render interactive keyframe anchor points & tangent handles (real editing)
        // Anchors are drawn at actual keyframe positions and can be dragged in time;
        // tangent handles edit the keyframe's custom bezier control points.
        {
            use crate::core::keyframe::{BezierControlPoint, InterpolationType};

            // Mutable access to Vec2-typed keyframe tracks (Position / Scale)
            fn keyframes_of_vec2<'a>(
                layer: &'a mut Layer,
                prop: &str,
            ) -> Option<&'a mut Vec<crate::core::keyframe::Keyframe<[f32; 2]>>> {
                use crate::core::property::Animatable;
                let animated = |a: &'a mut Animatable<[f32; 2]>| match a {
                    Animatable::Animated(kfs) => Some(kfs),
                    _ => None,
                };
                match prop {
                    "Position X" | "Position Y" => animated(&mut layer.transform.position),
                    "Scale X" | "Scale Y" => animated(&mut layer.transform.scale),
                    _ => None,
                }
            }

            // Mutable access to scalar keyframe tracks (Rotation / Opacity)
            fn keyframes_of_f32<'a>(
                layer: &'a mut Layer,
                prop: &str,
            ) -> Option<&'a mut Vec<crate::core::keyframe::Keyframe<f32>>> {
                use crate::core::property::Animatable;
                let animated = |a: &'a mut Animatable<f32>| match a {
                    Animatable::Animated(kfs) => Some(kfs),
                    _ => None,
                };
                match prop {
                    "Rotation" => animated(&mut layer.transform.rotation),
                    "Opacity" => animated(&mut layer.transform.opacity),
                    _ => None,
                }
            }

            fn effect_keyframes_of_f32<'a>(
                layer: &'a mut Layer,
                prop: &str,
            ) -> Option<&'a mut Vec<crate::core::keyframe::Keyframe<f32>>> {
                let (effect_id, label, component) = parse_effect_property(prop)?;
                if component.is_some() {
                    return None;
                }
                let effect = layer
                    .effects
                    .iter_mut()
                    .find(|effect| effect.id == effect_id)?;
                for (name, parameter) in effect.effect_type.animatable_params() {
                    if name == label {
                        if let crate::core::effect_params::ParamRef::Scalar(track) = parameter {
                            return track.keyframes_mut();
                        }
                    }
                }
                None
            }

            fn keyframes_of_vec3<'a>(
                layer: &'a mut Layer,
                prop: &str,
            ) -> Option<&'a mut Vec<crate::core::keyframe::Keyframe<[f32; 3]>>> {
                use crate::core::property::Animatable;
                let animated = |a: &'a mut Animatable<[f32; 3]>| match a {
                    Animatable::Animated(kfs) => Some(kfs),
                    _ => None,
                };
                match prop {
                    p if p.starts_with("3D Position") => animated(&mut layer.transform_3d.position),
                    p if p.starts_with("3D Rotation") => animated(&mut layer.transform_3d.rotation),
                    p if p.starts_with("3D Scale") => animated(&mut layer.transform_3d.scale),
                    _ => None,
                }
            }

            macro_rules! with_keyframes {
                ($layer:expr, $prop:expr, $kfs:ident => $body:expr) => {{
                    let prop: String = $prop.clone();
                    if prop.starts_with("3D ") {
                        if let Some($kfs) = keyframes_of_vec3($layer, &prop) {
                            Some({ $body })
                        } else {
                            None
                        }
                    } else if matches!(
                        prop.as_str(),
                        "Position X" | "Position Y" | "Scale X" | "Scale Y"
                    ) {
                        if let Some($kfs) = keyframes_of_vec2($layer, &prop) {
                            Some({ $body })
                        } else {
                            None
                        }
                    } else if is_effect_property(&prop) {
                        if let Some($kfs) = effect_keyframes_of_f32($layer, &prop) {
                            Some({ $body })
                        } else {
                            None
                        }
                    } else {
                        if let Some($kfs) = keyframes_of_f32($layer, &prop) {
                            Some({ $body })
                        } else {
                            None
                        }
                    }
                }};
            }

            let frame_to_x = |f: u32| rect.left() + (f as f32 / total_f as f32) * rect.width();
            let val_to_y =
                |v: f32| rect.bottom() - 4.0 - ((v - min_val) / val_range) * (rect.height() - 8.0);

            // Snapshot keyframe positions first (immutable), then edit mutably on drag
            let kf_positions: Vec<(usize, u32, f32)> = if graph_prop.starts_with("3D ") {
                let ci = axis_3d(&graph_prop);
                keyframes_of_vec3(layer, &graph_prop)
                    .map(|kfs| {
                        kfs.iter()
                            .enumerate()
                            .map(|(i, kf)| (i, kf.frame, kf.value[ci]))
                            .collect()
                    })
                    .unwrap_or_default()
            } else if matches!(
                graph_prop.as_str(),
                "Position X" | "Position Y" | "Scale X" | "Scale Y"
            ) {
                let comp_idx = if graph_prop.ends_with('Y') {
                    1usize
                } else {
                    0usize
                };
                keyframes_of_vec2(layer, &graph_prop)
                    .map(|kfs| {
                        kfs.iter()
                            .enumerate()
                            .map(|(i, kf)| (i, kf.frame, kf.value[comp_idx]))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            } else if is_effect_property(&graph_prop) {
                effect_keyframes_of_f32(layer, &graph_prop)
                    .map(|kfs| {
                        kfs.iter()
                            .enumerate()
                            .map(|(i, kf)| (i, kf.frame, kf.value))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            } else {
                keyframes_of_f32(layer, &graph_prop)
                    .map(|kfs| {
                        kfs.iter()
                            .enumerate()
                            .map(|(i, kf)| (i, kf.frame, kf.value))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            };

            if is_effect_property(&graph_prop) && graph_prop.contains('|') {
                let channel_keys = effect_parameter_channel_keyframes(layer, &graph_prop);
                let channel_drag_id =
                    egui::Id::new(("effect_channel_drag", &layer.id, &graph_prop));
                let active_channel_drag: Option<GraphKeyframeDrag> =
                    ui.ctx().data(|d| d.get_temp(channel_drag_id));
                for (frame, value) in &channel_keys {
                    let drag_display = active_channel_drag.filter(|drag| drag.current_frame == *frame);
                    let display_frame = drag_display
                        .map(|drag| drag.current_frame)
                        .unwrap_or(*frame);
                    let display_value = drag_display
                        .map(|drag| drag.current_value)
                        .unwrap_or(*value);
                    // Use the frame identity for egui IDs. An array index is
                    // not stable after a retime sorts the keyframes and can
                    // make the drag state attach to a neighbouring key.
                    let key_token = drag_display
                        .map(|drag| drag.original_frame as usize)
                        .unwrap_or(*frame as usize);
                    let key_pos = egui::pos2(frame_to_x(display_frame), val_to_y(display_value));
                    let key_rect = egui::Rect::from_center_size(key_pos, egui::vec2(14.0, 14.0));
                    let key_response = ui.interact(
                        key_rect,
                        egui::Id::new(("effect_channel_key", &layer.id, &graph_prop, key_token)),
                        egui::Sense::click_and_drag(),
                    );
                    ui.painter()
                        .circle_filled(key_pos, 4.0, colors::TIMELINE_KEYFRAME);
                    if let Some(points) =
                        effect_channel_bezier_points(layer, &graph_prop, display_frame)
                    {
                        let out =
                            egui::pos2(key_pos.x + points[2] * 44.0, key_pos.y - points[3] * 24.0);
                        let incoming =
                            egui::pos2(key_pos.x - points[0] * 44.0, key_pos.y + points[1] * 24.0);
                        let out_drag_id = egui::Id::new((
                            "effect_bezier_drag_out",
                            &layer.id,
                            &graph_prop,
                            key_token,
                        ));
                        let in_drag_id = egui::Id::new((
                            "effect_bezier_drag_in",
                            &layer.id,
                            &graph_prop,
                            key_token,
                        ));
                        let out_resp = ui.interact(
                            egui::Rect::from_center_size(out, egui::vec2(14.0, 14.0)),
                            egui::Id::new(("effect_bezier_out", &layer.id, &graph_prop, key_token)),
                            egui::Sense::drag(),
                        );
                        let in_resp = ui.interact(
                            egui::Rect::from_center_size(incoming, egui::vec2(14.0, 14.0)),
                            egui::Id::new(("effect_bezier_in", &layer.id, &graph_prop, key_token)),
                            egui::Sense::drag(),
                        );
                        ui.painter().line_segment(
                            [key_pos, out],
                            egui::Stroke::new(1.0_f32, colors::MOTION_PATH),
                        );
                        ui.painter().line_segment(
                            [key_pos, incoming],
                            egui::Stroke::new(1.0_f32, colors::MOTION_PATH),
                        );
                        ui.painter().circle_filled(out, 3.0, colors::HANDLE_NORMAL);
                        ui.painter()
                            .circle_filled(incoming, 3.0, colors::HANDLE_NORMAL);
                        if out_resp.drag_started() {
                            ui.ctx()
                                .data_mut(|data| data.insert_temp(out_drag_id, points));
                        }
                        if in_resp.drag_started() {
                            ui.ctx()
                                .data_mut(|data| data.insert_temp(in_drag_id, points));
                        }
                        let out_base = ui
                            .ctx()
                            .data(|data| data.get_temp::<[f32; 4]>(out_drag_id))
                            .unwrap_or(points);
                        let in_base = ui
                            .ctx()
                            .data(|data| data.get_temp::<[f32; 4]>(in_drag_id))
                            .unwrap_or(points);
                        let mut next = points;
                        if out_resp.dragged() {
                            next[2] = clamp_bezier_x(
                                out_base[2] + out_resp.drag_delta().x / 44.0,
                                out_base[0] + 0.01,
                                1.0,
                            );
                            next[3] =
                                (out_base[3] - out_resp.drag_delta().y / 24.0).clamp(-1.5, 2.5);
                        }
                        if in_resp.dragged() {
                            next[0] = clamp_bezier_x(
                                in_base[0] - in_resp.drag_delta().x / 44.0,
                                0.0,
                                in_base[2] - 0.01,
                            );
                            next[1] = (in_base[1] + in_resp.drag_delta().y / 24.0).clamp(-1.5, 2.5);
                        }
                        if (out_resp.dragged() || in_resp.dragged())
                            && set_effect_channel_bezier(layer, &graph_prop, display_frame, next)
                        {
                            *project_changed = true;
                        }
                        if out_resp.drag_stopped() {
                            ui.ctx()
                                .data_mut(|data| data.remove::<[f32; 4]>(out_drag_id));
                        }
                        if in_resp.drag_stopped() {
                            ui.ctx()
                                .data_mut(|data| data.remove::<[f32; 4]>(in_drag_id));
                        }
                    }
                    if key_response.drag_started() {
                        ui.ctx().data_mut(|d| {
                            d.insert_temp(
                                channel_drag_id,
                                GraphKeyframeDrag {
                                    original_frame: *frame,
                                    current_frame: *frame,
                                    original_value: *value,
                                    current_value: *value,
                                },
                            )
                        });
                    }
                    if key_response.secondary_clicked() {
                        if remove_effect_channel_at_frame(layer, &graph_prop, display_frame) {
                            *project_changed = true;
                        }
                        continue;
                    }
                    if key_response.dragged() {
                        let state = ui
                            .ctx()
                            .data(|d| d.get_temp::<GraphKeyframeDrag>(channel_drag_id))
                            .unwrap_or(GraphKeyframeDrag {
                                original_frame: *frame,
                                current_frame: *frame,
                                original_value: *value,
                                current_value: *value,
                            });
                        let next_frame = (state.original_frame as i32
                            + (key_response.drag_delta().x / rect.width() * total_f as f32).round()
                                as i32)
                            .clamp(0, total_f as i32)
                            as u32;
                        let next_value = state.original_value
                            - key_response.drag_delta().y / (rect.height() - 8.0) * val_range;
                        let moved = next_frame != state.current_frame
                            && move_effect_channel_keyframe(
                                layer,
                                &graph_prop,
                                state.current_frame,
                                next_frame,
                            );
                        let value_changed =
                            set_effect_channel_at_frame(layer, &graph_prop, next_frame, next_value);
                        *project_changed |= moved || value_changed;
                        ui.ctx().data_mut(|d| {
                            d.insert_temp(
                                channel_drag_id,
                                GraphKeyframeDrag {
                                    current_frame: next_frame,
                                    current_value: next_value,
                                    ..state
                                },
                            )
                        });
                    }
                    if key_response.drag_stopped() {
                        ui.ctx()
                            .data_mut(|d| d.remove::<GraphKeyframeDrag>(channel_drag_id));
                    }
                }
            }

            let drag_state_id = egui::Id::new(("graph_keyframe_drag", &layer.id, &graph_prop));
            let active_drag: Option<GraphKeyframeDrag> =
                ui.ctx().data(|d| d.get_temp(drag_state_id));

            for (kf_idx, kf_frame, kf_val) in &kf_positions {
                let drag_display = active_drag.filter(|drag| drag.current_frame == *kf_frame);
                let display_frame = drag_display
                    .map(|drag| drag.current_frame)
                    .unwrap_or(*kf_frame);
                let display_value = drag_display
                    .map(|drag| drag.current_value)
                    .unwrap_or(*kf_val);
                let pt = egui::pos2(frame_to_x(display_frame), val_to_y(display_value));

                // --- Anchor point: drag horizontally to retime, vertically to change value ---
                let anchor_rect = egui::Rect::from_center_size(pt, egui::vec2(14.0, 14.0));
                #[cfg(test)]
                ui.ctx().data_mut(|d| {
                    d.insert_temp(
                        egui::Id::new(("ae_graph_anchor_rect", &layer.id, &graph_prop, *kf_frame)),
                        anchor_rect,
                    );
                });
                let anchor_token = drag_display
                    .map(|drag| drag.original_frame as usize)
                    .unwrap_or(*kf_frame as usize);
                let anchor_resp = ui.interact(
                    anchor_rect,
                    egui::Id::new(("graph_anchor", &layer.id, &graph_prop, anchor_token)),
                    egui::Sense::click_and_drag(),
                );
                if anchor_resp.hovered() {
                    ui.ctx().data_mut(|d| {
                        d.insert_temp(
                            egui::Id::new(("ae_graph_hovered_kf", &layer.id, &graph_prop)),
                            *kf_idx,
                        )
                    });
                }
                if anchor_resp.secondary_clicked() {
                    with_keyframes!(layer, graph_prop, kfs => {
                        if let Some(index) = kfs.iter().position(|key| key.frame == *kf_frame) {
                            kfs.remove(index);
                            *project_changed = true;
                        }
                    });
                    continue;
                }
                if anchor_resp.drag_started() {
                    let state = GraphKeyframeDrag {
                        original_frame: *kf_frame,
                        current_frame: *kf_frame,
                        original_value: *kf_val,
                        current_value: *kf_val,
                    };
                    ui.ctx().data_mut(|d| d.insert_temp(drag_state_id, state));
                }
                if anchor_resp.dragged() {
                    let state = ui
                        .ctx()
                        .data(|d| d.get_temp::<GraphKeyframeDrag>(drag_state_id))
                        .unwrap_or(GraphKeyframeDrag {
                            original_frame: *kf_frame,
                            current_frame: *kf_frame,
                            original_value: *kf_val,
                            current_value: *kf_val,
                        });
                    let delta_frames =
                        (anchor_resp.drag_delta().x / rect.width() * total_f as f32).round() as i32;
                    let new_frame = (state.original_frame as i32 + delta_frames)
                        .clamp(0, total_f as i32) as u32;
                    let delta_val = -anchor_resp.drag_delta().y / (rect.height() - 8.0) * val_range;
                    let new_value = state.original_value + delta_val;
                    let axis = if graph_prop.starts_with("3D ") {
                        axis_3d(&graph_prop)
                    } else {
                        usize::from(graph_prop.ends_with('Y'))
                    };

                    let changed = if is_effect_property(&graph_prop) && !graph_prop.contains('|') {
                        let moved = state.current_frame != new_frame
                            && move_effect_scalar_keyframe(
                                layer,
                                &graph_prop,
                                state.current_frame,
                                new_frame,
                            );
                        let updated =
                            set_effect_channel_at_frame(layer, &graph_prop, new_frame, new_value);
                        moved || updated
                    } else if graph_prop == "Rotation" {
                        move_and_set_channel(
                            &mut layer.transform.rotation,
                            state.current_frame,
                            new_frame,
                            0,
                            new_value,
                        )
                    } else if graph_prop == "Opacity" {
                        move_and_set_channel(
                            &mut layer.transform.opacity,
                            state.current_frame,
                            new_frame,
                            0,
                            new_value.clamp(0.0, 100.0),
                        )
                    } else if graph_prop.starts_with("3D Position") {
                        move_and_set_channel(
                            &mut layer.transform_3d.position,
                            state.current_frame,
                            new_frame,
                            axis,
                            new_value,
                        )
                    } else if graph_prop.starts_with("3D Rotation") {
                        move_and_set_channel(
                            &mut layer.transform_3d.rotation,
                            state.current_frame,
                            new_frame,
                            axis,
                            new_value,
                        )
                    } else if graph_prop.starts_with("3D Scale") {
                        move_and_set_channel(
                            &mut layer.transform_3d.scale,
                            state.current_frame,
                            new_frame,
                            axis,
                            new_value,
                        )
                    } else if graph_prop.starts_with("Position") {
                        move_and_set_channel(
                            &mut layer.transform.position,
                            state.current_frame,
                            new_frame,
                            axis,
                            new_value,
                        )
                    } else if graph_prop.starts_with("Scale") {
                        move_and_set_channel(
                            &mut layer.transform.scale,
                            state.current_frame,
                            new_frame,
                            axis,
                            new_value,
                        )
                    } else if graph_prop.starts_with("PinX:") || graph_prop.starts_with("PinY:") {
                        pin_anim_mut(layer, &graph_prop)
                            .map(|pin| {
                                move_and_set_channel(
                                    pin,
                                    state.current_frame,
                                    new_frame,
                                    axis,
                                    new_value,
                                )
                            })
                            .unwrap_or(false)
                    } else {
                        false
                    };
                    if changed {
                        *project_changed = true;
                    }
                    ui.ctx().data_mut(|d| {
                        d.insert_temp(
                            drag_state_id,
                            GraphKeyframeDrag {
                                current_frame: new_frame,
                                current_value: new_value,
                                ..state
                            },
                        )
                    });
                }
                if anchor_resp.drag_stopped() {
                    ui.ctx()
                        .data_mut(|d| d.remove::<GraphKeyframeDrag>(drag_state_id));
                }
                let anchor_color = if anchor_resp.dragged() {
                    colors::HANDLE_NORMAL
                } else if anchor_resp.hovered() {
                    colors::HANDLE_HOVER_FILL
                } else {
                    colors::TIMELINE_KEYFRAME
                };
                ui.painter().circle_filled(pt, 4.0, anchor_color);

                // ── Double-click anchor → numeric value popup ──
                let dbl_id =
                    ui.make_persistent_id(("graph_kf_popup", &layer.id, &graph_prop, anchor_token));
                let mut show_popup: bool = ui
                    .ctx()
                    .data_mut(|d| *d.get_temp_mut_or_insert_with(dbl_id, || false));
                if anchor_resp.double_clicked() {
                    show_popup = true;
                    let frame_edit_id = egui::Id::new((
                        "graph_kf_frame_text",
                        &layer.id,
                        &graph_prop,
                        anchor_token,
                    ));
                    let value_edit_id = egui::Id::new((
                        "graph_kf_value_text",
                        &layer.id,
                        &graph_prop,
                        anchor_token,
                    ));
                    ui.ctx().data_mut(|data| {
                        data.insert_temp(frame_edit_id, kf_frame.to_string());
                        data.insert_temp(value_edit_id, format!("{:.2}", kf_val));
                    });
                }
                if show_popup {
                    let popup_id =
                        egui::Id::new(("graph_kf_val_popup", &layer.id, &graph_prop, anchor_token));
                    let resp = egui::Area::new(popup_id)
                        .fixed_pos(pt + egui::vec2(12.0, -20.0))
                        .order(egui::Order::Foreground)
                        .show(ui.ctx(), |ui| {
                            ui.group(|ui| {
                                ui.set_min_width(140.0);
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("Frame:").small());
                                    let frame_edit_id = egui::Id::new((
                                        "graph_kf_frame_text",
                                        &layer.id,
                                        &graph_prop,
                                        anchor_token,
                                    ));
                                    let mut frame_str = ui.ctx().data(|data| {
                                        data.get_temp::<String>(frame_edit_id)
                                            .unwrap_or_else(|| kf_frame.to_string())
                                    });
                                    let frame_response = ui.add(
                                        egui::TextEdit::singleline(&mut frame_str)
                                            .desired_width(50.0),
                                    );
                                    if frame_response.changed() {
                                        ui.ctx().data_mut(|data| {
                                            data.insert_temp(frame_edit_id, frame_str.clone())
                                        });
                                    }
                                    if frame_response.lost_focus()
                                        || ui.input(|input| input.key_pressed(egui::Key::Enter))
                                    {
                                        if let Ok(f) = frame_str.parse::<u32>() {
                                            let new_f = f.min(total_f);
                                            let handled_effect_move =
                                                if is_effect_property(&graph_prop)
                                                    && new_f != *kf_frame
                                                {
                                                    if graph_prop.contains('|') {
                                                        move_effect_channel_keyframe(
                                                            layer,
                                                            &graph_prop,
                                                            *kf_frame,
                                                            new_f,
                                                        )
                                                    } else {
                                                        move_effect_scalar_keyframe(
                                                            layer,
                                                            &graph_prop,
                                                            *kf_frame,
                                                            new_f,
                                                        )
                                                    }
                                                } else if graph_prop == "Rotation"
                                                    && new_f != *kf_frame
                                                {
                                                    layer
                                                        .transform
                                                        .rotation
                                                        .move_keyframe(*kf_frame, new_f)
                                                } else if graph_prop == "Opacity"
                                                    && new_f != *kf_frame
                                                {
                                                    layer
                                                        .transform
                                                        .opacity
                                                        .move_keyframe(*kf_frame, new_f)
                                                } else if graph_prop == "Position X"
                                                    || graph_prop == "Position Y"
                                                {
                                                    layer
                                                        .transform
                                                        .position
                                                        .move_keyframe(*kf_frame, new_f)
                                                } else if graph_prop == "Scale X"
                                                    || graph_prop == "Scale Y"
                                                {
                                                    layer
                                                        .transform
                                                        .scale
                                                        .move_keyframe(*kf_frame, new_f)
                                                } else if graph_prop.starts_with("3D Position") {
                                                    layer
                                                        .transform_3d
                                                        .position
                                                        .move_keyframe(*kf_frame, new_f)
                                                } else if graph_prop.starts_with("3D Rotation") {
                                                    layer
                                                        .transform_3d
                                                        .rotation
                                                        .move_keyframe(*kf_frame, new_f)
                                                } else if graph_prop.starts_with("3D Scale") {
                                                    layer
                                                        .transform_3d
                                                        .scale
                                                        .move_keyframe(*kf_frame, new_f)
                                                } else if graph_prop.starts_with("PinX:")
                                                    || graph_prop.starts_with("PinY:")
                                                {
                                                    pin_anim_mut(layer, &graph_prop).is_some_and(
                                                        |pin| pin.move_keyframe(*kf_frame, new_f),
                                                    )
                                                } else {
                                                    false
                                                };
                                            if handled_effect_move {
                                                *project_changed = true;
                                            }
                                            ui.ctx().data_mut(|data| {
                                                data.insert_temp(frame_edit_id, new_f.to_string())
                                            });
                                        }
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("Value:").small());
                                    let value_edit_id = egui::Id::new((
                                        "graph_kf_value_text",
                                        &layer.id,
                                        &graph_prop,
                                        anchor_token,
                                    ));
                                    let mut val_str = ui.ctx().data(|data| {
                                        data.get_temp::<String>(value_edit_id)
                                            .unwrap_or_else(|| format!("{:.2}", kf_val))
                                    });
                                    let value_response = ui.add(
                                        egui::TextEdit::singleline(&mut val_str)
                                            .desired_width(70.0),
                                    );
                                    if value_response.changed() {
                                        ui.ctx().data_mut(|data| {
                                            data.insert_temp(value_edit_id, val_str.clone())
                                        });
                                    }
                                    if value_response.lost_focus()
                                        || ui.input(|input| input.key_pressed(egui::Key::Enter))
                                    {
                                        if let Ok(v) = val_str.parse::<f32>() {
                                            if graph_prop.starts_with("Position") {
                                                let ci =
                                                    if graph_prop.ends_with('Y') { 1 } else { 0 };
                                                if let Some(kfs) =
                                                    layer.transform.position.keyframes_mut()
                                                {
                                                    if let Some(kf) = kfs
                                                        .iter_mut()
                                                        .find(|kf| kf.frame == *kf_frame)
                                                    {
                                                        kf.value[ci] = v;
                                                        *project_changed = true;
                                                    }
                                                }
                                            } else if graph_prop.starts_with("Scale") {
                                                let ci =
                                                    if graph_prop.ends_with('Y') { 1 } else { 0 };
                                                if let Some(kfs) =
                                                    layer.transform.scale.keyframes_mut()
                                                {
                                                    if let Some(kf) = kfs
                                                        .iter_mut()
                                                        .find(|kf| kf.frame == *kf_frame)
                                                    {
                                                        kf.value[ci] = v;
                                                        *project_changed = true;
                                                    }
                                                }
                                            } else if graph_prop == "Rotation" {
                                                if let Some(kfs) =
                                                    layer.transform.rotation.keyframes_mut()
                                                {
                                                    if let Some(kf) = kfs
                                                        .iter_mut()
                                                        .find(|kf| kf.frame == *kf_frame)
                                                    {
                                                        kf.value = v;
                                                        *project_changed = true;
                                                    }
                                                }
                                            } else if graph_prop == "Opacity" {
                                                if let Some(kfs) =
                                                    layer.transform.opacity.keyframes_mut()
                                                {
                                                    if let Some(kf) = kfs
                                                        .iter_mut()
                                                        .find(|kf| kf.frame == *kf_frame)
                                                    {
                                                        kf.value = v.clamp(0.0, 100.0);
                                                        *project_changed = true;
                                                    }
                                                }
                                            } else if graph_prop.starts_with("3D Position") {
                                                if let Some(kfs) =
                                                    layer.transform_3d.position.keyframes_mut()
                                                {
                                                    if let Some(kf) = kfs
                                                        .iter_mut()
                                                        .find(|kf| kf.frame == *kf_frame)
                                                    {
                                                        kf.value[axis_3d(&graph_prop)] = v;
                                                        *project_changed = true;
                                                    }
                                                }
                                            } else if graph_prop.starts_with("3D Rotation") {
                                                if let Some(kfs) =
                                                    layer.transform_3d.rotation.keyframes_mut()
                                                {
                                                    if let Some(kf) = kfs
                                                        .iter_mut()
                                                        .find(|kf| kf.frame == *kf_frame)
                                                    {
                                                        kf.value[axis_3d(&graph_prop)] = v;
                                                        *project_changed = true;
                                                    }
                                                }
                                            } else if graph_prop.starts_with("3D Scale") {
                                                if let Some(kfs) =
                                                    layer.transform_3d.scale.keyframes_mut()
                                                {
                                                    if let Some(kf) = kfs
                                                        .iter_mut()
                                                        .find(|kf| kf.frame == *kf_frame)
                                                    {
                                                        kf.value[axis_3d(&graph_prop)] = v;
                                                        *project_changed = true;
                                                    }
                                                }
                                            } else if is_effect_property(&graph_prop) {
                                                if set_effect_channel_at_frame(
                                                    layer,
                                                    &graph_prop,
                                                    *kf_frame,
                                                    v,
                                                ) {
                                                    *project_changed = true;
                                                }
                                            }
                                        }
                                    }
                                });
                                if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
                                    show_popup = false;
                                }
                                if ui.button("Done").clicked() {
                                    show_popup = false;
                                }
                            });
                        });
                    // Click outside popup → dismiss
                    if ui.input(|i| i.pointer.any_click()) && !resp.response.contains_pointer() {
                        show_popup = false;
                    }
                }
                ui.ctx().data_mut(|d| d.insert_temp(dbl_id, show_popup));
                if anchor_resp.hovered() {
                    ui.painter().circle_stroke(
                        pt,
                        7.0,
                        egui::Stroke::new(1.0_f32, colors::TIMELINE_KEYFRAME),
                    );
                }

                // --- Tangent handles: drag to edit custom bezier control points ---
                // Extract current bezier points (default Easy Ease if linear/hold)
                fn bezier_pts<T>(kf: &crate::core::keyframe::Keyframe<T>) -> (f32, f32, f32, f32) {
                    match &kf.interpolation {
                        InterpolationType::Bezier {
                            custom_bezier: Some(pts),
                            ..
                        } => (pts[0], pts[1], pts[2], pts[3]),
                        _ => (0.33, 0.0, 0.67, 1.0),
                    }
                }
                let (bx1, by1, bx2, by2): (f32, f32, f32, f32) =
                    with_keyframes!(layer, graph_prop, kfs => {
                        kfs.iter()
                            .find(|key| key.frame == display_frame)
                            .map(bezier_pts)
                            .unwrap_or((0.33, 0.0, 0.67, 1.0))
                    })
                    .unwrap_or((0.33, 0.0, 0.67, 1.0));

                let h_out = egui::pos2(pt.x + bx2 * 44.0, pt.y - by2 * 24.0);
                let h_in = egui::pos2(pt.x - bx1 * 44.0, pt.y + by1 * 24.0);

                let h_out_rect = egui::Rect::from_center_size(h_out, egui::vec2(14.0, 14.0));
                let h_in_rect = egui::Rect::from_center_size(h_in, egui::vec2(14.0, 14.0));
                let h_out_id = egui::Id::new(("graph_h_out", &layer.id, &graph_prop, anchor_token));
                let h_in_id = egui::Id::new(("graph_h_in", &layer.id, &graph_prop, anchor_token));
                let h_out_drag_id = egui::Id::new((
                    "graph_bezier_drag_out",
                    &layer.id,
                    &graph_prop,
                    anchor_token,
                ));
                let h_in_drag_id =
                    egui::Id::new(("graph_bezier_drag_in", &layer.id, &graph_prop, anchor_token));
                let h_out_resp = ui.interact(h_out_rect, h_out_id, egui::Sense::drag());
                let h_in_resp = ui.interact(h_in_rect, h_in_id, egui::Sense::drag());

                let mut new_pts: Option<[f32; 4]> = None;
                if h_out_resp.drag_started() {
                    ui.ctx()
                        .data_mut(|data| data.insert_temp(h_out_drag_id, [bx1, by1, bx2, by2]));
                }
                if h_in_resp.drag_started() {
                    ui.ctx()
                        .data_mut(|data| data.insert_temp(h_in_drag_id, [bx1, by1, bx2, by2]));
                }
                let out_base = ui
                    .ctx()
                    .data(|data| data.get_temp::<[f32; 4]>(h_out_drag_id))
                    .unwrap_or([bx1, by1, bx2, by2]);
                let in_base = ui
                    .ctx()
                    .data(|data| data.get_temp::<[f32; 4]>(h_in_drag_id))
                    .unwrap_or([bx1, by1, bx2, by2]);
                if h_out_resp.dragged() {
                    let d = h_out_resp.drag_delta();
                    let nx2 = clamp_bezier_x(out_base[2] + d.x / 44.0, out_base[0] + 0.01, 1.0);
                    let ny2 = (out_base[3] - d.y / 24.0).clamp(-1.5, 2.5);
                    if *linked_tangent {
                        let mir_x = (1.0 - nx2).clamp(0.0, nx2 - 0.01);
                        new_pts = Some([mir_x, -ny2, nx2, ny2]);
                    } else {
                        new_pts = Some([out_base[0], out_base[1], nx2, ny2]);
                    }
                }
                if h_in_resp.dragged() {
                    let d = h_in_resp.drag_delta();
                    let nx1 = clamp_bezier_x(in_base[0] - d.x / 44.0, 0.0, in_base[2] - 0.01);
                    let ny1 = (in_base[1] + d.y / 24.0).clamp(-1.5, 2.5);
                    if *linked_tangent {
                        let mir_x = (1.0 - nx1).clamp(nx1 + 0.01, 1.0);
                        new_pts = Some([nx1, ny1, mir_x, -ny1]);
                    } else {
                        new_pts = Some([nx1, ny1, in_base[2], in_base[3]]);
                    }
                }
                if let Some(pts) = new_pts {
                    with_keyframes!(layer, graph_prop, kfs => {
                        if let Some(kf) = kfs.iter_mut().find(|key| key.frame == display_frame) {
                            kf.interpolation = InterpolationType::Bezier {
                                outgoing: BezierControlPoint { influence: 0.333, speed: 0.0 },
                                incoming: BezierControlPoint { influence: 0.333, speed: 0.0 },
                                custom_bezier: Some(pts),
                            };
                            *project_changed = true;
                        }
                    });
                }
                if h_out_resp.drag_stopped() {
                    ui.ctx()
                        .data_mut(|data| data.remove::<[f32; 4]>(h_out_drag_id));
                }
                if h_in_resp.drag_stopped() {
                    ui.ctx()
                        .data_mut(|data| data.remove::<[f32; 4]>(h_in_drag_id));
                }

                let any_hover = h_out_resp.hovered()
                    || h_in_resp.dragged()
                    || h_in_resp.hovered()
                    || h_out_resp.dragged();
                let stroke_color = if any_hover {
                    colors::ACCENT_ORANGE
                } else {
                    colors::MOTION_PATH
                };
                ui.painter()
                    .line_segment([pt, h_out], egui::Stroke::new(1.2_f32, stroke_color));
                ui.painter()
                    .line_segment([pt, h_in], egui::Stroke::new(1.2_f32, stroke_color));

                let h_out_color = if h_out_resp.hovered() || h_out_resp.dragged() {
                    colors::HANDLE_NORMAL
                } else {
                    colors::MOTION_PATH
                };
                let h_in_color = if h_in_resp.hovered() || h_in_resp.dragged() {
                    colors::HANDLE_NORMAL
                } else {
                    colors::MOTION_PATH
                };
                ui.painter().circle_filled(h_out, 4.0, h_out_color);
                ui.painter().circle_filled(h_in, 4.0, h_in_color);
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
    ui.label(egui::RichText::new("🎚 Automation Channel").strong());
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

fn camera_animation_mut<'a>(
    camera: &'a mut crate::core::timeline::Camera3D,
    property: &str,
) -> &'a mut Option<crate::core::property::Animatable<f32>> {
    match property {
        "FOV" => &mut camera.fov_animation,
        "Focus Distance" => &mut camera.focus_distance_animation,
        "Aperture" => &mut camera.aperture_animation,
        "DOF Enabled" => &mut camera.dof_enabled_animation,
        _ => &mut camera.dof_max_blur_animation,
    }
}

pub fn draw_camera_lens_graph(
    ui: &mut egui::Ui,
    camera: &mut crate::core::timeline::Camera3D,
    duration_frames: u32,
    current_frame: u32,
    project_changed: &mut bool,
) {
    let id = egui::Id::new("camera_lens_graph_property");
    let mut property = ui
        .ctx()
        .data(|d| d.get_temp::<String>(id))
        .unwrap_or_else(|| "FOV".into());
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("📈 Camera Lens Graph").strong());
        egui::ComboBox::from_id_salt(id)
            .selected_text(&property)
            .show_ui(ui, |ui| {
                for name in [
                    "FOV",
                    "Focus Distance",
                    "Aperture",
                    "DOF Blur",
                    "DOF Enabled",
                ] {
                    if ui.selectable_label(property == name, name).clicked() {
                        property = name.into();
                    }
                }
            });
    });
    ui.ctx().data_mut(|d| d.insert_temp(id, property.clone()));
    ui.horizontal(|ui| {
        if ui.small_button("× Remove Key").clicked() {
            let removed = match property.as_str() {
                "FOV" => remove_camera_key(&mut camera.fov_animation, current_frame),
                "Focus Distance" => {
                    remove_camera_key(&mut camera.focus_distance_animation, current_frame)
                }
                "Aperture" => remove_camera_key(&mut camera.aperture_animation, current_frame),
                "DOF Enabled" => {
                    remove_camera_key(&mut camera.dof_enabled_animation, current_frame)
                }
                _ => remove_camera_key(&mut camera.dof_max_blur_animation, current_frame),
            };
            *project_changed |= removed;
        }
    });
    ui.horizontal(|ui| {
        ui.label("Interpolation:");
        for (label, interpolation) in [
            ("Linear", crate::core::keyframe::InterpolationType::Linear),
            ("Hold", crate::core::keyframe::InterpolationType::Hold),
        ] {
            if ui.small_button(label).clicked() {
                let changed = match property.as_str() {
                    "FOV" => set_camera_key_interpolation(
                        &mut camera.fov_animation,
                        current_frame,
                        interpolation.clone(),
                    ),
                    "Focus Distance" => set_camera_key_interpolation(
                        &mut camera.focus_distance_animation,
                        current_frame,
                        interpolation.clone(),
                    ),
                    "Aperture" => set_camera_key_interpolation(
                        &mut camera.aperture_animation,
                        current_frame,
                        interpolation.clone(),
                    ),
                    "DOF Enabled" => set_camera_key_interpolation(
                        &mut camera.dof_enabled_animation,
                        current_frame,
                        interpolation.clone(),
                    ),
                    _ => set_camera_key_interpolation(
                        &mut camera.dof_max_blur_animation,
                        current_frame,
                        interpolation.clone(),
                    ),
                };
                *project_changed |= changed;
            }
        }
    });
    ui.horizontal(|ui| {
        ui.label("Ease:");
        for (label, preset) in [
            ("F9", crate::core::keyframe::EasePreset::Standard),
            ("Ease In", crate::core::keyframe::EasePreset::EaseIn),
            ("Ease Out", crate::core::keyframe::EasePreset::EaseOut),
        ] {
            if ui.small_button(label).clicked() {
                let changed = match property.as_str() {
                    "FOV" => set_camera_key_ease(&mut camera.fov_animation, current_frame, preset),
                    "Focus Distance" => set_camera_key_ease(
                        &mut camera.focus_distance_animation,
                        current_frame,
                        preset,
                    ),
                    "Aperture" => {
                        set_camera_key_ease(&mut camera.aperture_animation, current_frame, preset)
                    }
                    "DOF Enabled" => set_camera_key_ease(
                        &mut camera.dof_enabled_animation,
                        current_frame,
                        preset,
                    ),
                    _ => set_camera_key_ease(
                        &mut camera.dof_max_blur_animation,
                        current_frame,
                        preset,
                    ),
                };
                *project_changed |= changed;
            }
        }
    });
    let track = match property.as_str() {
        "FOV" => camera.fov_animation.as_ref(),
        "Focus Distance" => camera.focus_distance_animation.as_ref(),
        "Aperture" => camera.aperture_animation.as_ref(),
        "DOF Enabled" => camera.dof_enabled_animation.as_ref(),
        _ => camera.dof_max_blur_animation.as_ref(),
    };
    let Some(track) = track else {
        ui.label("No keyframes yet — use ◆ in Camera Settings.");
        return;
    };
    let Some(keys) = track.keyframes().map(|keys| keys.to_vec()) else {
        return;
    };
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 110.0),
        egui::Sense::click_and_drag(),
    );
    let min = keys.iter().map(|k| k.value).fold(f32::INFINITY, f32::min);
    let max = keys
        .iter()
        .map(|k| k.value)
        .fold(f32::NEG_INFINITY, f32::max);
    let range = (max - min).max(0.001);
    let end = duration_frames
        .max(keys.last().map(|k| k.frame).unwrap_or(0))
        .max(1);
    let point = |frame: u32, value: f32| {
        egui::pos2(
            rect.left() + frame as f32 / end as f32 * rect.width(),
            rect.bottom() - 6.0 - (value - min) / range * (rect.height() - 12.0),
        )
    };
    let clamp_value = |value: f32| match property.as_str() {
        "FOV" => value.clamp(1.0, 179.0),
        "Focus Distance" | "Aperture" => value.max(0.0),
        "DOF Enabled" => value.clamp(0.0, 1.0),
        _ => value.clamp(1.0, 64.0),
    };
    for pair in keys.windows(2) {
        let start = &pair[0];
        let end_key = &pair[1];
        let bezier = match start.interpolation {
            crate::core::keyframe::InterpolationType::Bezier {
                custom_bezier: Some(points),
                ..
            } => Some(points),
            _ => None,
        };
        let mut previous = point(start.frame, start.value);
        for step in 1..=24 {
            let normalized = step as f32 / 24.0;
            let eased = bezier
                .map(|[x1, y1, x2, y2]| {
                    let t =
                        crate::core::keyframe::solve_bezier_eased_time(normalized, x1, y1, x2, y2);
                    let omt = 1.0 - t;
                    3.0 * omt * omt * t * y1 + 3.0 * omt * t * t * y2 + t * t * t
                })
                .unwrap_or_else(|| match start.interpolation {
                    crate::core::keyframe::InterpolationType::Hold => 0.0,
                    _ => normalized,
                });
            let frame = start.frame as f32
                + (end_key.frame.saturating_sub(start.frame) as f32) * normalized;
            let value = start.value + (end_key.value - start.value) * eased;
            let next = point(frame.round() as u32, value);
            ui.painter().line_segment(
                [previous, next],
                egui::Stroke::new(2.0_f32, colors::ACCENT_CYAN),
            );
            previous = next;
        }
    }
    for key in &keys {
        ui.painter()
            .circle_filled(point(key.frame, key.value), 4.0, colors::TIMELINE_KEYFRAME);
    }
    let x = rect.left() + current_frame.min(end) as f32 / end as f32 * rect.width();
    ui.painter().line_segment(
        [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
        egui::Stroke::new(1.0_f32, colors::HANDLE_HOVER_FILL),
    );
    let drag_id = egui::Id::new(("camera_lens_graph_drag", property.as_str()));
    if response.drag_started() {
        if let Some(pointer) = response.interact_pointer_pos() {
            let nearest = keys
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    point(a.frame, a.value)
                        .distance(pointer)
                        .partial_cmp(&point(b.frame, b.value).distance(pointer))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .filter(|(_, key)| point(key.frame, key.value).distance(pointer) <= 12.0)
                .map(|(index, _)| index);
                let drag = nearest.and_then(|index| {
                    keys.get(index).map(|key| GraphKeyframeDrag {
                    original_frame: key.frame,
                    current_frame: key.frame,
                    original_value: key.value,
                    current_value: key.value,
                })
            });
            ui.ctx().data_mut(|data| data.insert_temp(drag_id, drag));
        }
    }
    if response.dragged() {
        if let Some(pointer) = response.interact_pointer_pos() {
            let drag = ui
                .ctx()
                .data(|data| data.get_temp::<GraphKeyframeDrag>(drag_id));
            if let Some(state) = drag {
                let frame = ((pointer.x - rect.left()) / rect.width() * end as f32)
                    .round()
                    .clamp(0.0, end as f32) as u32;
                let value = clamp_value(
                    min + ((rect.bottom() - 6.0 - pointer.y) / (rect.height() - 12.0) * range),
                );
                let track = camera_animation_mut(camera, &property);
                let moved = track
                    .as_mut()
                    .is_some_and(|track| track.move_keyframe(state.current_frame, frame));
                let updated = track.as_mut().is_some_and(|track| {
                    let before = track.evaluate(frame);
                    track.set_value_at_frame(frame, value);
                    before != value
                });
                *project_changed |= moved || updated;
                ui.ctx().data_mut(|data| {
                    data.insert_temp(
                        drag_id,
                        GraphKeyframeDrag {
                            current_frame: frame,
                            current_value: value,
                            ..state
                        },
                    )
                });
            }
        }
    }
    if response.drag_stopped() {
        ui.ctx()
            .data_mut(|data| data.remove::<GraphKeyframeDrag>(drag_id));
    }
    if response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            let frame = ((pos.x - rect.left()) / rect.width() * end as f32)
                .round()
                .clamp(0.0, end as f32) as u32;
            let value =
                clamp_value(min + ((rect.bottom() - 6.0 - pos.y) / (rect.height() - 12.0) * range));
            match property.as_str() {
                "FOV" => camera.set_fov_at(frame, value),
                "Focus Distance" => camera.set_focus_distance_at(frame, value),
                "Aperture" => camera.set_aperture_at(frame, value),
                "DOF Enabled" => camera.set_dof_enabled_at(frame, value >= 0.5),
                _ => camera.set_dof_max_blur_at(frame, value),
            }
            *project_changed = true;
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
