#![allow(clippy::too_many_arguments)]
use super::*;

pub(crate) fn cycle_track_interpolation<T: Clone>(
    track: &mut kagari_vfx::core::timeline::Animatable<T>,
    frame: u32,
) -> bool {
    use kagari_vfx::core::keyframe::{BezierControlPoint, InterpolationType};
    let Some(key) = track
        .keyframes_mut()
        .and_then(|keys| keys.iter_mut().find(|key| key.frame == frame))
    else {
        return false;
    };
    key.interpolation = match key.interpolation {
        InterpolationType::Linear => InterpolationType::Bezier {
            outgoing: BezierControlPoint {
                influence: 0.333,
                speed: 0.0,
            },
            incoming: BezierControlPoint {
                influence: 0.333,
                speed: 0.0,
            },
            custom_bezier: Some([0.25, 0.1, 0.25, 1.0]),
        },
        InterpolationType::Bezier { .. } => InterpolationType::Hold,
        InterpolationType::Hold => InterpolationType::Linear,
    };
    true
}

pub(crate) fn set_track_interpolation_preset<T: Clone>(
    track: &mut kagari_vfx::core::timeline::Animatable<T>,
    frame: u32,
    preset: &str,
) -> bool {
    use kagari_vfx::core::keyframe::{BezierControlPoint, InterpolationType};
    let Some(key) = track
        .keyframes_mut()
        .and_then(|keys| keys.iter_mut().find(|key| key.frame == frame))
    else {
        return false;
    };
    key.interpolation = match preset {
        "linear" => InterpolationType::Linear,
        "hold" => InterpolationType::Hold,
        "easy_ease" => InterpolationType::Bezier {
            outgoing: BezierControlPoint {
                influence: 0.333,
                speed: 0.0,
            },
            incoming: BezierControlPoint {
                influence: 0.333,
                speed: 0.0,
            },
            custom_bezier: Some([0.25, 0.1, 0.25, 1.0]),
        },
        "ease_in" => InterpolationType::Bezier {
            outgoing: BezierControlPoint {
                influence: 0.166,
                speed: 0.0,
            },
            incoming: BezierControlPoint {
                influence: 0.666,
                speed: 0.0,
            },
            custom_bezier: Some([0.42, 0.0, 1.0, 1.0]),
        },
        "ease_out" => InterpolationType::Bezier {
            outgoing: BezierControlPoint {
                influence: 0.666,
                speed: 0.0,
            },
            incoming: BezierControlPoint {
                influence: 0.166,
                speed: 0.0,
            },
            custom_bezier: Some([0.0, 0.0, 0.58, 1.0]),
        },
        "overshoot" => InterpolationType::Bezier {
            outgoing: BezierControlPoint {
                influence: 0.68,
                speed: 0.0,
            },
            incoming: BezierControlPoint {
                influence: 0.265,
                speed: 0.0,
            },
            custom_bezier: Some([0.68, -0.4, 0.265, 1.4]),
        },
        "bounce" => InterpolationType::Bezier {
            outgoing: BezierControlPoint {
                influence: 0.5,
                speed: 0.3,
            },
            incoming: BezierControlPoint {
                influence: 0.7,
                speed: 0.1,
            },
            custom_bezier: Some([0.5, 0.3, 0.7, 0.1]),
        },
        "sine" => InterpolationType::Bezier {
            outgoing: BezierControlPoint {
                influence: 0.37,
                speed: 0.0,
            },
            incoming: BezierControlPoint {
                influence: 0.63,
                speed: 0.0,
            },
            custom_bezier: Some([0.37, 0.0, 0.63, 1.0]),
        },
        _ => return false,
    };
    true
}

pub(crate) fn set_graph_interpolation_preset(
    layer: &mut kagari_vfx::core::timeline::Layer,
    camera: Option<&mut kagari_vfx::core::timeline::Camera3D>,
    light: Option<&mut kagari_vfx::core::timeline::Light3D>,
    mask_id: Option<&str>,
    effect_id: Option<&str>,
    effect_parameter_id: Option<&str>,
    channel: GraphChannel,
    frame: u32,
    preset: &str,
) -> bool {
    match channel {
        GraphChannel::PositionX | GraphChannel::PositionY => {
            set_track_interpolation_preset(&mut layer.transform.position, frame, preset)
        }
        GraphChannel::AnchorX | GraphChannel::AnchorY => {
            set_track_interpolation_preset(&mut layer.transform.anchor_point, frame, preset)
        }
        GraphChannel::ScaleX | GraphChannel::ScaleY => {
            set_track_interpolation_preset(&mut layer.transform.scale, frame, preset)
        }
        GraphChannel::Rotation => {
            set_track_interpolation_preset(&mut layer.transform.rotation, frame, preset)
        }
        GraphChannel::Opacity => {
            set_track_interpolation_preset(&mut layer.transform.opacity, frame, preset)
        }
        GraphChannel::Position3DX | GraphChannel::Position3DY | GraphChannel::Position3DZ => {
            set_track_interpolation_preset(&mut layer.transform_3d.position, frame, preset)
        }
        GraphChannel::Rotation3DX | GraphChannel::Rotation3DY | GraphChannel::Rotation3DZ => {
            set_track_interpolation_preset(&mut layer.transform_3d.rotation, frame, preset)
        }
        GraphChannel::Scale3DX | GraphChannel::Scale3DY | GraphChannel::Scale3DZ => {
            set_track_interpolation_preset(&mut layer.transform_3d.scale, frame, preset)
        }
        GraphChannel::AudioVolume => match &mut layer.layer_type {
            kagari_vfx::core::timeline::LayerType::Audio { volume, .. } => {
                set_track_interpolation_preset(volume, frame, preset)
            }
            kagari_vfx::core::timeline::LayerType::Video {
                audio_wav: Some(_),
                audio_volume,
                ..
            } => set_track_interpolation_preset(audio_volume, frame, preset),
            _ => false,
        },
        GraphChannel::TimeRemap => layer
            .time_remap
            .as_mut()
            .is_some_and(|track| set_track_interpolation_preset(track, frame, preset)),
        GraphChannel::CameraFov => camera
            .and_then(|camera| camera.fov_animation.as_mut())
            .is_some_and(|track| set_track_interpolation_preset(track, frame, preset)),
        GraphChannel::CameraFocus => camera
            .and_then(|camera| camera.focus_distance_animation.as_mut())
            .is_some_and(|track| set_track_interpolation_preset(track, frame, preset)),
        GraphChannel::CameraAperture => camera
            .and_then(|camera| camera.aperture_animation.as_mut())
            .is_some_and(|track| set_track_interpolation_preset(track, frame, preset)),
        GraphChannel::CameraDofBlur => camera
            .and_then(|camera| camera.dof_max_blur_animation.as_mut())
            .is_some_and(|track| set_track_interpolation_preset(track, frame, preset)),
        GraphChannel::CameraDofEnabled => camera
            .and_then(|camera| camera.dof_enabled_animation.as_mut())
            .is_some_and(|track| set_track_interpolation_preset(track, frame, preset)),
        GraphChannel::LightIntensity => light
            .and_then(|light| light.intensity_animation.as_mut())
            .is_some_and(|track| set_track_interpolation_preset(track, frame, preset)),
        GraphChannel::MaskFeather => mask_id
            .and_then(|id| layer.masks.iter_mut().find(|mask| mask.id == id))
            .is_some_and(|mask| set_track_interpolation_preset(&mut mask.feather, frame, preset)),
        GraphChannel::MaskOpacity => mask_id
            .and_then(|id| layer.masks.iter_mut().find(|mask| mask.id == id))
            .is_some_and(|mask| set_track_interpolation_preset(&mut mask.opacity, frame, preset)),
        GraphChannel::MaskExpansion => mask_id
            .and_then(|id| layer.masks.iter_mut().find(|mask| mask.id == id))
            .is_some_and(|mask| set_track_interpolation_preset(&mut mask.expansion, frame, preset)),
        GraphChannel::ShapeWidth
        | GraphChannel::ShapeHeight
        | GraphChannel::ShapeCornerRadius
        | GraphChannel::ShapePoints
        | GraphChannel::ShapeInnerRadius
        | GraphChannel::ShapeOuterRadius
        | GraphChannel::ShapeSides
        | GraphChannel::ShapeRadius => shape_parameter_track_mut(layer, channel)
            .is_some_and(|track| set_track_interpolation_preset(track, frame, preset)),
        GraphChannel::EffectParameter => {
            scalar_effect_track_mut(layer, effect_id, effect_parameter_id)
                .is_some_and(|track| set_track_interpolation_preset(track, frame, preset))
        }
        GraphChannel::EffectColorRed
        | GraphChannel::EffectColorGreen
        | GraphChannel::EffectColorBlue
        | GraphChannel::EffectColorAlpha => {
            effect_color_track_mut(layer, effect_id, effect_parameter_id)
                .is_some_and(|track| set_track_interpolation_preset(track, frame, preset))
        }
    }
}

pub(crate) fn cycle_graph_interpolation(
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
            cycle_track_interpolation(&mut layer.transform.position, frame)
        }
        GraphChannel::AnchorX | GraphChannel::AnchorY => {
            cycle_track_interpolation(&mut layer.transform.anchor_point, frame)
        }
        GraphChannel::ScaleX | GraphChannel::ScaleY => {
            cycle_track_interpolation(&mut layer.transform.scale, frame)
        }
        GraphChannel::Rotation => cycle_track_interpolation(&mut layer.transform.rotation, frame),
        GraphChannel::Opacity => cycle_track_interpolation(&mut layer.transform.opacity, frame),
        GraphChannel::Position3DX | GraphChannel::Position3DY | GraphChannel::Position3DZ => {
            cycle_track_interpolation(&mut layer.transform_3d.position, frame)
        }
        GraphChannel::Rotation3DX | GraphChannel::Rotation3DY | GraphChannel::Rotation3DZ => {
            cycle_track_interpolation(&mut layer.transform_3d.rotation, frame)
        }
        GraphChannel::Scale3DX | GraphChannel::Scale3DY | GraphChannel::Scale3DZ => {
            cycle_track_interpolation(&mut layer.transform_3d.scale, frame)
        }
        GraphChannel::AudioVolume => match &mut layer.layer_type {
            kagari_vfx::core::timeline::LayerType::Audio { volume, .. } => {
                cycle_track_interpolation(volume, frame)
            }
            kagari_vfx::core::timeline::LayerType::Video {
                audio_wav: Some(_),
                audio_volume,
                ..
            } => cycle_track_interpolation(audio_volume, frame),
            _ => false,
        },
        GraphChannel::TimeRemap => layer
            .time_remap
            .as_mut()
            .is_some_and(|track| cycle_track_interpolation(track, frame)),
        GraphChannel::CameraFov => camera
            .and_then(|camera| camera.fov_animation.as_mut())
            .is_some_and(|track| cycle_track_interpolation(track, frame)),
        GraphChannel::CameraFocus => camera
            .and_then(|camera| camera.focus_distance_animation.as_mut())
            .is_some_and(|track| cycle_track_interpolation(track, frame)),
        GraphChannel::CameraAperture => camera
            .and_then(|camera| camera.aperture_animation.as_mut())
            .is_some_and(|track| cycle_track_interpolation(track, frame)),
        GraphChannel::CameraDofBlur => camera
            .and_then(|camera| camera.dof_max_blur_animation.as_mut())
            .is_some_and(|track| cycle_track_interpolation(track, frame)),
        GraphChannel::CameraDofEnabled => camera
            .and_then(|camera| camera.dof_enabled_animation.as_mut())
            .is_some_and(|track| cycle_track_interpolation(track, frame)),
        GraphChannel::LightIntensity => light
            .and_then(|light| light.intensity_animation.as_mut())
            .is_some_and(|track| cycle_track_interpolation(track, frame)),
        GraphChannel::MaskFeather => mask_id
            .and_then(|id| layer.masks.iter_mut().find(|mask| mask.id == id))
            .is_some_and(|mask| cycle_track_interpolation(&mut mask.feather, frame)),
        GraphChannel::MaskOpacity => mask_id
            .and_then(|id| layer.masks.iter_mut().find(|mask| mask.id == id))
            .is_some_and(|mask| cycle_track_interpolation(&mut mask.opacity, frame)),
        GraphChannel::MaskExpansion => mask_id
            .and_then(|id| layer.masks.iter_mut().find(|mask| mask.id == id))
            .is_some_and(|mask| cycle_track_interpolation(&mut mask.expansion, frame)),
        GraphChannel::ShapeWidth
        | GraphChannel::ShapeHeight
        | GraphChannel::ShapeCornerRadius
        | GraphChannel::ShapePoints
        | GraphChannel::ShapeInnerRadius
        | GraphChannel::ShapeOuterRadius
        | GraphChannel::ShapeSides
        | GraphChannel::ShapeRadius => shape_parameter_track_mut(layer, channel)
            .is_some_and(|track| cycle_track_interpolation(track, frame)),
        GraphChannel::EffectParameter => {
            scalar_effect_track_mut(layer, effect_id, effect_parameter_id)
                .is_some_and(|track| cycle_track_interpolation(track, frame))
        }
        GraphChannel::EffectColorRed
        | GraphChannel::EffectColorGreen
        | GraphChannel::EffectColorBlue
        | GraphChannel::EffectColorAlpha => {
            effect_color_track_mut(layer, effect_id, effect_parameter_id)
                .is_some_and(|track| cycle_track_interpolation(track, frame))
        }
    }
}
