use crate::core::keyframe::{EasePreset, InterpolationType, Keyframe};

pub(crate) fn kf(frame: u32, v: f32) -> Keyframe<f32> {
    Keyframe::new(frame, v, InterpolationType::Linear)
}

pub(crate) fn kfv2(frame: u32, v: [f32; 2]) -> Keyframe<[f32; 2]> {
    Keyframe::new(frame, v, InterpolationType::Linear)
}

pub(crate) fn ease_all<T>(kfs: &mut [Keyframe<T>]) {
    let coords = EasePreset::Standard.control_points();
    for kf in kfs.iter_mut() {
        kf.interpolation = InterpolationType::Bezier {
            outgoing: crate::core::keyframe::BezierControlPoint {
                influence: 0.333,
                speed: 0.0,
            },
            incoming: crate::core::keyframe::BezierControlPoint {
                influence: 0.333,
                speed: 0.0,
            },
            custom_bezier: Some(coords),
        };
    }
}

pub(crate) fn make_effect(
    id_seed: &str,
    name: &str,
    et: crate::core::timeline::EffectType,
) -> crate::core::timeline::Effect {
    crate::core::timeline::Effect {
        id: format!("{}_{}", id_seed, rand_suffix()),
        name: name.to_string(),
        effect_type: et,
        enabled: true,
    }
}

fn counter() -> u32 {
    use std::sync::atomic::{AtomicU32, Ordering};
    static C: AtomicU32 = AtomicU32::new(0);
    C.fetch_add(1, Ordering::Relaxed)
}

pub(crate) fn rand_suffix() -> u32 {
    counter()
}
