use super::*;
use crate::core::keyframe::InterpolationType;

// Mutable access to Vec2-typed keyframe tracks (Position / Scale)
pub(super) fn layer_keyframes_of_vec2<'a>(
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
pub(super) fn layer_keyframes_of_f32<'a>(
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

pub(super) fn effect_keyframes_of_f32<'a>(
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

pub(super) fn layer_keyframes_of_vec3<'a>(
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

pub(super) fn bezier_control_points<T>(
    kf: &crate::core::keyframe::Keyframe<T>,
) -> (f32, f32, f32, f32) {
    match &kf.interpolation {
        InterpolationType::Bezier {
            custom_bezier: Some(pts),
            ..
        } => (pts[0], pts[1], pts[2], pts[3]),
        _ => (0.33, 0.0, 0.67, 1.0),
    }
}
