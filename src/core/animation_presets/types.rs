use serde::{Deserialize, Serialize};

use crate::core::keyframe::{EasePreset, InterpolationType, Keyframe};
use crate::core::property::Animatable;
use crate::core::timeline::Layer;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationPreset {
    pub name: String,
    pub category: PresetCategory,
    pub description: String,
    pub property_type: PresetPropertyType,
    pub keyframes: Vec<PresetKeyframe>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PresetCategory {
    Position,
    Opacity,
    Scale,
    Rotation,
    Focus,
    Color,
    Text,
    Combo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PresetPropertyType {
    Position,
    Opacity,
    Scale,
    Rotation,
    Blur,
    ColorRGB,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetKeyframe {
    pub time: f32,
    pub value: f32,
    #[serde(default)]
    pub ease: f32,
    #[serde(default)]
    pub value_y: Option<f32>,
}

pub(crate) fn eased_kf(frame: u32, v: f32, ease: f32) -> Keyframe<f32> {
    let mut kf = Keyframe::new(frame, v, InterpolationType::Linear);
    if ease > 0.001 {
        let coords = EasePreset::Standard.control_points();
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
    kf
}

pub(crate) fn eased_kfv2(frame: u32, v: [f32; 2], ease: f32) -> Keyframe<[f32; 2]> {
    let mut kf = Keyframe::new(frame, v, InterpolationType::Linear);
    if ease > 0.001 {
        let coords = EasePreset::Standard.control_points();
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
    kf
}

/// Apply a preset's keyframes to a layer at the given start frame.
/// Position/Scale presets preserve the layer's current value as the rest point.
pub fn apply_preset_to_layer(
    preset: &AnimationPreset,
    layer: &mut Layer,
    start_frame: u32,
) -> bool {
    let to_frame = |t: f32| start_frame + (t * 30.0).round() as u32;

    match preset.property_type {
        PresetPropertyType::Opacity | PresetPropertyType::Blur => {
            let kfs: Vec<Keyframe<f32>> = preset
                .keyframes
                .iter()
                .map(|k| eased_kf(to_frame(k.time), k.value, k.ease))
                .collect();
            if kfs.is_empty() {
                return false;
            }
            if preset.property_type == PresetPropertyType::Opacity {
                layer.transform.opacity = Animatable::Animated(kfs);
            }
            true
        }
        PresetPropertyType::Scale => {
            let rest = layer.transform.scale.evaluate(start_frame);
            let kfs: Vec<Keyframe<[f32; 2]>> = preset
                .keyframes
                .iter()
                .map(|k| {
                    let v = k.value.max(0.0) / 100.0;
                    let vx = rest[0] * v;
                    let vy = k
                        .value_y
                        .map(|y| rest[1] * (y.max(0.0) / 100.0))
                        .unwrap_or(vx);
                    eased_kfv2(to_frame(k.time), [vx, vy], k.ease)
                })
                .collect();
            if kfs.is_empty() {
                return false;
            }
            layer.transform.scale = Animatable::Animated(kfs);
            true
        }
        PresetPropertyType::Position => {
            let rest = layer.transform.position.evaluate(start_frame);
            let kfs: Vec<Keyframe<[f32; 2]>> = preset
                .keyframes
                .iter()
                .map(|k| {
                    let x = rest[0] + k.value;
                    let y = rest[1] + k.value_y.unwrap_or(0.0);
                    eased_kfv2(to_frame(k.time), [x, y], k.ease)
                })
                .collect();
            if kfs.is_empty() {
                return false;
            }
            layer.transform.position = Animatable::Animated(kfs);
            true
        }
        PresetPropertyType::Rotation => {
            let rest = layer.transform.rotation.evaluate(start_frame);
            let kfs: Vec<Keyframe<f32>> = preset
                .keyframes
                .iter()
                .map(|k| eased_kf(to_frame(k.time), rest + k.value, k.ease))
                .collect();
            if kfs.is_empty() {
                return false;
            }
            layer.transform.rotation = Animatable::Animated(kfs);
            true
        }
        PresetPropertyType::ColorRGB => false,
    }
}
