#![allow(clippy::too_many_arguments)]
use super::*;

pub(crate) fn add_graph_keyframe(
    layer: &mut kagari_vfx::core::timeline::Layer,
    camera: Option<&mut kagari_vfx::core::timeline::Camera3D>,
    light: Option<&mut kagari_vfx::core::timeline::Light3D>,
    mask_id: Option<&str>,
    effect_id: Option<&str>,
    effect_parameter_id: Option<&str>,
    channel: GraphChannel,
    frame: u32,
) {
    match channel {
        GraphChannel::PositionX | GraphChannel::PositionY => {
            add_current_keyframe(&mut layer.transform.position, frame);
        }
        GraphChannel::AnchorX | GraphChannel::AnchorY => {
            add_current_keyframe(&mut layer.transform.anchor_point, frame)
        }
        GraphChannel::ScaleX | GraphChannel::ScaleY => {
            add_current_keyframe(&mut layer.transform.scale, frame);
        }
        GraphChannel::Rotation => {
            add_current_keyframe(&mut layer.transform.rotation, frame);
        }
        GraphChannel::Opacity => {
            add_current_keyframe(&mut layer.transform.opacity, frame);
        }
        GraphChannel::Position3DX | GraphChannel::Position3DY | GraphChannel::Position3DZ => {
            add_current_keyframe(&mut layer.transform_3d.position, frame)
        }
        GraphChannel::Rotation3DX | GraphChannel::Rotation3DY | GraphChannel::Rotation3DZ => {
            add_current_keyframe(&mut layer.transform_3d.rotation, frame)
        }
        GraphChannel::Scale3DX | GraphChannel::Scale3DY | GraphChannel::Scale3DZ => {
            add_current_keyframe(&mut layer.transform_3d.scale, frame)
        }
        GraphChannel::AudioVolume => match &mut layer.layer_type {
            kagari_vfx::core::timeline::LayerType::Audio { volume, .. } => {
                add_current_keyframe(volume, frame)
            }
            kagari_vfx::core::timeline::LayerType::Video {
                audio_wav: Some(_),
                audio_volume,
                ..
            } => add_current_keyframe(audio_volume, frame),
            _ => {}
        },
        GraphChannel::TimeRemap => {
            if let Some(track) = &mut layer.time_remap {
                add_current_keyframe(track, frame);
            }
        }
        GraphChannel::CameraFov => {
            if let Some(camera) = camera {
                let value = camera.fov_degrees;
                add_current_keyframe(
                    camera.fov_animation.get_or_insert_with(|| {
                        kagari_vfx::core::timeline::Animatable::new_constant(value)
                    }),
                    frame,
                );
            }
        }
        GraphChannel::CameraFocus => {
            if let Some(camera) = camera {
                let value = camera.focus_distance;
                add_current_keyframe(
                    camera.focus_distance_animation.get_or_insert_with(|| {
                        kagari_vfx::core::timeline::Animatable::new_constant(value)
                    }),
                    frame,
                );
            }
        }
        GraphChannel::CameraAperture => {
            if let Some(camera) = camera {
                let value = camera.aperture;
                add_current_keyframe(
                    camera.aperture_animation.get_or_insert_with(|| {
                        kagari_vfx::core::timeline::Animatable::new_constant(value)
                    }),
                    frame,
                );
            }
        }
        GraphChannel::CameraDofBlur => {
            if let Some(camera) = camera {
                let value = camera.dof_max_blur;
                add_current_keyframe(
                    camera.dof_max_blur_animation.get_or_insert_with(|| {
                        kagari_vfx::core::timeline::Animatable::new_constant(value)
                    }),
                    frame,
                );
            }
        }
        GraphChannel::CameraDofEnabled => {
            if let Some(camera) = camera {
                let value = if camera.dof_enabled_at(frame) {
                    1.0
                } else {
                    0.0
                };
                add_current_keyframe(
                    camera.dof_enabled_animation.get_or_insert_with(|| {
                        kagari_vfx::core::timeline::Animatable::new_constant(value)
                    }),
                    frame,
                );
            }
        }
        GraphChannel::LightIntensity => {
            if let Some(light) = light {
                add_current_keyframe(
                    light.intensity_animation.get_or_insert_with(|| {
                        kagari_vfx::core::timeline::Animatable::new_constant(light.intensity)
                    }),
                    frame,
                );
            }
        }
        GraphChannel::MaskFeather => {
            if let Some(mask) =
                mask_id.and_then(|id| layer.masks.iter_mut().find(|mask| mask.id == id))
            {
                add_current_keyframe(&mut mask.feather, frame);
            }
        }
        GraphChannel::MaskOpacity => {
            if let Some(mask) =
                mask_id.and_then(|id| layer.masks.iter_mut().find(|mask| mask.id == id))
            {
                add_current_keyframe(&mut mask.opacity, frame);
            }
        }
        GraphChannel::MaskExpansion => {
            if let Some(mask) =
                mask_id.and_then(|id| layer.masks.iter_mut().find(|mask| mask.id == id))
            {
                add_current_keyframe(&mut mask.expansion, frame);
            }
        }
        GraphChannel::ShapeWidth
        | GraphChannel::ShapeHeight
        | GraphChannel::ShapeCornerRadius
        | GraphChannel::ShapePoints
        | GraphChannel::ShapeInnerRadius
        | GraphChannel::ShapeOuterRadius
        | GraphChannel::ShapeSides
        | GraphChannel::ShapeRadius => {
            if let Some(track) = shape_parameter_track_mut(layer, channel) {
                add_current_keyframe(track, frame);
            }
        }
        GraphChannel::EffectParameter => {
            if let Some(track) = scalar_effect_track_mut(layer, effect_id, effect_parameter_id) {
                add_current_keyframe(track, frame);
            }
        }
        GraphChannel::EffectColorRed
        | GraphChannel::EffectColorGreen
        | GraphChannel::EffectColorBlue
        | GraphChannel::EffectColorAlpha => {
            if let Some(track) = effect_color_track_mut(layer, effect_id, effect_parameter_id) {
                add_current_keyframe(track, frame);
            }
        }
    }
}

pub(crate) fn remove_graph_keyframe(
    layer: &mut kagari_vfx::core::timeline::Layer,
    camera: Option<&mut kagari_vfx::core::timeline::Camera3D>,
    light: Option<&mut kagari_vfx::core::timeline::Light3D>,
    mask_id: Option<&str>,
    effect_id: Option<&str>,
    effect_parameter_id: Option<&str>,
    channel: GraphChannel,
    frame: u32,
) -> bool {
    match channel {
        GraphChannel::PositionX | GraphChannel::PositionY => {
            layer.transform.position.remove_keyframe(frame)
        }
        GraphChannel::AnchorX | GraphChannel::AnchorY => {
            layer.transform.anchor_point.remove_keyframe(frame)
        }
        GraphChannel::ScaleX | GraphChannel::ScaleY => layer.transform.scale.remove_keyframe(frame),
        GraphChannel::Rotation => layer.transform.rotation.remove_keyframe(frame),
        GraphChannel::Opacity => layer.transform.opacity.remove_keyframe(frame),
        GraphChannel::Position3DX | GraphChannel::Position3DY | GraphChannel::Position3DZ => {
            layer.transform_3d.position.remove_keyframe(frame)
        }
        GraphChannel::Rotation3DX | GraphChannel::Rotation3DY | GraphChannel::Rotation3DZ => {
            layer.transform_3d.rotation.remove_keyframe(frame)
        }
        GraphChannel::Scale3DX | GraphChannel::Scale3DY | GraphChannel::Scale3DZ => {
            layer.transform_3d.scale.remove_keyframe(frame)
        }
        GraphChannel::AudioVolume => match &mut layer.layer_type {
            kagari_vfx::core::timeline::LayerType::Audio { volume, .. } => {
                volume.remove_keyframe(frame)
            }
            kagari_vfx::core::timeline::LayerType::Video {
                audio_wav: Some(_),
                audio_volume,
                ..
            } => audio_volume.remove_keyframe(frame),
            _ => false,
        },
        GraphChannel::TimeRemap => layer
            .time_remap
            .as_mut()
            .is_some_and(|track| track.remove_keyframe(frame)),
        GraphChannel::CameraFov => camera.is_some_and(|camera| {
            let changed = camera
                .fov_animation
                .as_mut()
                .is_some_and(|track| track.remove_keyframe(frame));
            if changed {
                camera.fov_degrees = camera.fov_at(frame);
            }
            changed
        }),
        GraphChannel::CameraFocus => camera.is_some_and(|camera| {
            let changed = camera
                .focus_distance_animation
                .as_mut()
                .is_some_and(|track| track.remove_keyframe(frame));
            if changed {
                camera.focus_distance = camera.focus_distance_at(frame);
            }
            changed
        }),
        GraphChannel::CameraAperture => camera.is_some_and(|camera| {
            let changed = camera
                .aperture_animation
                .as_mut()
                .is_some_and(|track| track.remove_keyframe(frame));
            if changed {
                camera.aperture = camera.aperture_at(frame);
            }
            changed
        }),
        GraphChannel::CameraDofBlur => camera.is_some_and(|camera| {
            let changed = camera
                .dof_max_blur_animation
                .as_mut()
                .is_some_and(|track| track.remove_keyframe(frame));
            if changed {
                camera.dof_max_blur = camera.dof_max_blur_at(frame);
            }
            changed
        }),
        GraphChannel::CameraDofEnabled => camera.is_some_and(|camera| {
            let changed = camera
                .dof_enabled_animation
                .as_mut()
                .is_some_and(|track| track.remove_keyframe(frame));
            camera.dof_enabled = camera.dof_enabled_at(frame);
            changed
        }),
        GraphChannel::LightIntensity => light.is_some_and(|light| {
            let changed = light
                .intensity_animation
                .as_mut()
                .is_some_and(|track| track.remove_keyframe(frame));
            if changed {
                light.intensity = light.intensity_at(frame);
            }
            changed
        }),
        GraphChannel::MaskFeather => mask_id
            .and_then(|id| layer.masks.iter_mut().find(|mask| mask.id == id))
            .is_some_and(|mask| mask.feather.remove_keyframe(frame)),
        GraphChannel::MaskOpacity => mask_id
            .and_then(|id| layer.masks.iter_mut().find(|mask| mask.id == id))
            .is_some_and(|mask| mask.opacity.remove_keyframe(frame)),
        GraphChannel::MaskExpansion => mask_id
            .and_then(|id| layer.masks.iter_mut().find(|mask| mask.id == id))
            .is_some_and(|mask| mask.expansion.remove_keyframe(frame)),
        GraphChannel::ShapeWidth
        | GraphChannel::ShapeHeight
        | GraphChannel::ShapeCornerRadius
        | GraphChannel::ShapePoints
        | GraphChannel::ShapeInnerRadius
        | GraphChannel::ShapeOuterRadius
        | GraphChannel::ShapeSides
        | GraphChannel::ShapeRadius => shape_parameter_track_mut(layer, channel)
            .is_some_and(|track| track.remove_keyframe(frame)),
        GraphChannel::EffectParameter => {
            scalar_effect_track_mut(layer, effect_id, effect_parameter_id)
                .is_some_and(|track| track.remove_keyframe(frame))
        }
        GraphChannel::EffectColorRed
        | GraphChannel::EffectColorGreen
        | GraphChannel::EffectColorBlue
        | GraphChannel::EffectColorAlpha => {
            effect_color_track_mut(layer, effect_id, effect_parameter_id)
                .is_some_and(|track| track.remove_keyframe(frame))
        }
    }
}

pub(crate) fn update_graph_keyframe(
    layer: &mut kagari_vfx::core::timeline::Layer,
    camera: Option<&mut kagari_vfx::core::timeline::Camera3D>,
    light: Option<&mut kagari_vfx::core::timeline::Light3D>,
    mask_id: Option<&str>,
    effect_id: Option<&str>,
    effect_parameter_id: Option<&str>,
    channel: GraphChannel,
    source_frame: u32,
    target_frame: u32,
    value: f32,
) -> bool {
    if !value.is_finite() {
        return false;
    }
    let value = graph_clamp_value(channel, value);
    match channel {
        GraphChannel::PositionX | GraphChannel::PositionY => {
            let track = &mut layer.transform.position;
            if track.keyframes().is_none() {
                return false;
            }
            if source_frame != target_frame && !track.move_keyframe(source_frame, target_frame) {
                return false;
            }
            let mut position = track.evaluate(target_frame);
            position[if channel == GraphChannel::PositionX {
                0
            } else {
                1
            }] = value;
            track.set_value_at_frame(target_frame, position);
        }
        GraphChannel::AnchorX | GraphChannel::AnchorY => {
            let track = &mut layer.transform.anchor_point;
            if track.keyframes().is_none() {
                return false;
            }
            if source_frame != target_frame && !track.move_keyframe(source_frame, target_frame) {
                return false;
            }
            let mut anchor = track.evaluate(target_frame);
            anchor[if channel == GraphChannel::AnchorX {
                0
            } else {
                1
            }] = value;
            track.set_value_at_frame(target_frame, anchor);
        }
        GraphChannel::ScaleX | GraphChannel::ScaleY => {
            let track = &mut layer.transform.scale;
            if track.keyframes().is_none() {
                return false;
            }
            if source_frame != target_frame && !track.move_keyframe(source_frame, target_frame) {
                return false;
            }
            let mut scale = track.evaluate(target_frame);
            scale[if channel == GraphChannel::ScaleX {
                0
            } else {
                1
            }] = value;
            track.set_value_at_frame(target_frame, scale);
        }
        GraphChannel::Rotation => {
            let track = &mut layer.transform.rotation;
            if track.keyframes().is_none() {
                return false;
            }
            if source_frame != target_frame && !track.move_keyframe(source_frame, target_frame) {
                return false;
            }
            track.set_value_at_frame(target_frame, value);
        }
        GraphChannel::Opacity => {
            let track = &mut layer.transform.opacity;
            if track.keyframes().is_none() {
                return false;
            }
            if source_frame != target_frame && !track.move_keyframe(source_frame, target_frame) {
                return false;
            }
            track.set_value_at_frame(target_frame, value);
        }
        GraphChannel::Position3DX | GraphChannel::Position3DY | GraphChannel::Position3DZ => {
            let track = &mut layer.transform_3d.position;
            if track.keyframes().is_none() {
                return false;
            }
            if source_frame != target_frame && !track.move_keyframe(source_frame, target_frame) {
                return false;
            }
            let mut position = track.evaluate(target_frame);
            let axis = match channel {
                GraphChannel::Position3DX => 0,
                GraphChannel::Position3DY => 1,
                _ => 2,
            };
            position[axis] = value;
            track.set_value_at_frame(target_frame, position);
        }
        GraphChannel::Rotation3DX | GraphChannel::Rotation3DY | GraphChannel::Rotation3DZ => {
            let track = &mut layer.transform_3d.rotation;
            if track.keyframes().is_none() {
                return false;
            }
            if source_frame != target_frame && !track.move_keyframe(source_frame, target_frame) {
                return false;
            }
            let mut rotation = track.evaluate(target_frame);
            let axis = match channel {
                GraphChannel::Rotation3DX => 0,
                GraphChannel::Rotation3DY => 1,
                _ => 2,
            };
            rotation[axis] = value;
            track.set_value_at_frame(target_frame, rotation);
        }
        GraphChannel::Scale3DX | GraphChannel::Scale3DY | GraphChannel::Scale3DZ => {
            let track = &mut layer.transform_3d.scale;
            if track.keyframes().is_none() {
                return false;
            }
            if source_frame != target_frame && !track.move_keyframe(source_frame, target_frame) {
                return false;
            }
            let mut scale = track.evaluate(target_frame);
            let axis = match channel {
                GraphChannel::Scale3DX => 0,
                GraphChannel::Scale3DY => 1,
                _ => 2,
            };
            scale[axis] = value;
            track.set_value_at_frame(target_frame, scale);
        }
        GraphChannel::AudioVolume => {
            let volume = match &mut layer.layer_type {
                kagari_vfx::core::timeline::LayerType::Audio { volume, .. } => volume,
                kagari_vfx::core::timeline::LayerType::Video {
                    audio_wav: Some(_),
                    audio_volume,
                    ..
                } => audio_volume,
                _ => return false,
            };
            if update_scalar_graph_track(volume, source_frame, target_frame, value).is_none() {
                return false;
            }
        }
        GraphChannel::TimeRemap => {
            let Some(track) = &mut layer.time_remap else {
                return false;
            };
            if update_scalar_graph_track(track, source_frame, target_frame, value).is_none() {
                return false;
            }
        }
        GraphChannel::CameraFov => {
            let Some(camera) = camera else { return false };
            let Some(track) = &mut camera.fov_animation else {
                return false;
            };
            if update_scalar_graph_track(track, source_frame, target_frame, value).is_none() {
                return false;
            }
            camera.fov_degrees = track.evaluate(target_frame);
        }
        GraphChannel::CameraFocus => {
            let Some(camera) = camera else { return false };
            let Some(track) = &mut camera.focus_distance_animation else {
                return false;
            };
            if update_scalar_graph_track(track, source_frame, target_frame, value).is_none() {
                return false;
            }
            camera.focus_distance = track.evaluate(target_frame);
        }
        GraphChannel::CameraAperture => {
            let Some(camera) = camera else { return false };
            let Some(track) = &mut camera.aperture_animation else {
                return false;
            };
            if update_scalar_graph_track(track, source_frame, target_frame, value).is_none() {
                return false;
            }
            camera.aperture = track.evaluate(target_frame);
        }
        GraphChannel::CameraDofBlur => {
            let Some(camera) = camera else { return false };
            let Some(track) = &mut camera.dof_max_blur_animation else {
                return false;
            };
            if update_scalar_graph_track(track, source_frame, target_frame, value).is_none() {
                return false;
            }
            camera.dof_max_blur = track.evaluate(target_frame);
        }
        GraphChannel::CameraDofEnabled => {
            let Some(camera) = camera else { return false };
            let Some(track) = &mut camera.dof_enabled_animation else {
                return false;
            };
            if update_scalar_graph_track(track, source_frame, target_frame, value).is_none() {
                return false;
            }
            camera.dof_enabled = track.evaluate(target_frame) >= 0.5;
        }
        GraphChannel::LightIntensity => {
            let Some(light) = light else { return false };
            let Some(track) = &mut light.intensity_animation else {
                return false;
            };
            if update_scalar_graph_track(track, source_frame, target_frame, value).is_none() {
                return false;
            }
            light.intensity = track.evaluate(target_frame);
        }
        GraphChannel::MaskFeather | GraphChannel::MaskOpacity | GraphChannel::MaskExpansion => {
            let Some(mask) =
                mask_id.and_then(|id| layer.masks.iter_mut().find(|mask| mask.id == id))
            else {
                return false;
            };
            let track = match channel {
                GraphChannel::MaskFeather => &mut mask.feather,
                GraphChannel::MaskOpacity => &mut mask.opacity,
                _ => &mut mask.expansion,
            };
            if update_scalar_graph_track(track, source_frame, target_frame, value).is_none() {
                return false;
            }
        }
        GraphChannel::ShapeWidth
        | GraphChannel::ShapeHeight
        | GraphChannel::ShapeCornerRadius
        | GraphChannel::ShapePoints
        | GraphChannel::ShapeInnerRadius
        | GraphChannel::ShapeOuterRadius
        | GraphChannel::ShapeSides
        | GraphChannel::ShapeRadius => {
            let Some(track) = shape_parameter_track_mut(layer, channel) else {
                return false;
            };
            if update_scalar_graph_track(track, source_frame, target_frame, value).is_none() {
                return false;
            }
        }
        GraphChannel::EffectParameter => {
            let Some(track) = scalar_effect_track_mut(layer, effect_id, effect_parameter_id) else {
                return false;
            };
            if update_scalar_graph_track(track, source_frame, target_frame, value).is_none() {
                return false;
            }
        }
        GraphChannel::EffectColorRed
        | GraphChannel::EffectColorGreen
        | GraphChannel::EffectColorBlue
        | GraphChannel::EffectColorAlpha => {
            let Some(component) = effect_color_component(channel) else {
                return false;
            };
            let Some(track) = effect_color_track_mut(layer, effect_id, effect_parameter_id) else {
                return false;
            };
            if !update_color_component_graph_track(
                track,
                component,
                source_frame,
                target_frame,
                value,
            ) {
                return false;
            }
        }
    }
    true
}

pub(crate) fn graph_clamp_value(channel: GraphChannel, value: f32) -> f32 {
    match channel {
        GraphChannel::Opacity => value.clamp(0.0, 100.0),
        GraphChannel::CameraDofEnabled => value.clamp(0.0, 1.0),
        GraphChannel::LightIntensity => value.clamp(0.0, 500.0),
        GraphChannel::ScaleX
        | GraphChannel::ScaleY
        | GraphChannel::Scale3DX
        | GraphChannel::Scale3DY
        | GraphChannel::Scale3DZ => value.clamp(0.1, 1000.0),
        GraphChannel::Rotation => value.clamp(-36_000.0, 36_000.0),
        GraphChannel::Position3DX | GraphChannel::Position3DY | GraphChannel::Position3DZ => {
            value.clamp(-100_000.0, 100_000.0)
        }
        GraphChannel::Rotation3DX | GraphChannel::Rotation3DY | GraphChannel::Rotation3DZ => {
            value.clamp(-36_000.0, 36_000.0)
        }
        GraphChannel::AudioVolume => value.clamp(-48.0, 12.0),
        GraphChannel::TimeRemap => value.max(0.0),
        GraphChannel::CameraFov => value.clamp(1.0, 179.0),
        GraphChannel::CameraFocus | GraphChannel::CameraAperture => value.max(0.0),
        GraphChannel::CameraDofBlur => value.clamp(1.0, 64.0),
        GraphChannel::MaskFeather => value.clamp(0.0, 5000.0),
        GraphChannel::MaskOpacity => value.clamp(0.0, 100.0),
        GraphChannel::MaskExpansion => value.clamp(-5000.0, 5000.0),
        GraphChannel::ShapePoints => value.clamp(2.0, 100.0),
        GraphChannel::ShapeSides => value.clamp(3.0, 100.0),
        GraphChannel::ShapeWidth
        | GraphChannel::ShapeHeight
        | GraphChannel::ShapeCornerRadius
        | GraphChannel::ShapeInnerRadius
        | GraphChannel::ShapeOuterRadius
        | GraphChannel::ShapeRadius => value.clamp(0.0, 1_000_000.0),
        GraphChannel::EffectParameter => value.clamp(-1_000_000.0, 1_000_000.0),
        GraphChannel::EffectColorRed
        | GraphChannel::EffectColorGreen
        | GraphChannel::EffectColorBlue
        | GraphChannel::EffectColorAlpha => value.clamp(0.0, 1.0),
        GraphChannel::PositionX
        | GraphChannel::PositionY
        | GraphChannel::AnchorX
        | GraphChannel::AnchorY => value.clamp(-100_000.0, 100_000.0),
    }
}

pub(crate) fn update_scalar_graph_track(
    track: &mut kagari_vfx::core::timeline::Animatable<f32>,
    source_frame: u32,
    target_frame: u32,
    value: f32,
) -> Option<()> {
    track.keyframes()?;
    if source_frame != target_frame && !track.move_keyframe(source_frame, target_frame) {
        return None;
    }
    track.set_value_at_frame(target_frame, value);
    Some(())
}
