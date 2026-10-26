pub mod text;
pub mod transform;
pub mod types;
pub mod visual;

pub use types::*;

use text::*;
use transform::*;
use visual::*;

use crate::core::timeline::Layer;

pub fn all_presets() -> Vec<AnimationPreset> {
    let mut p = Vec::new();
    p.extend(position_presets());
    p.extend(opacity_presets());
    p.extend(scale_presets());
    p.extend(rotation_presets());
    p.extend(blur_presets());
    p.extend(text_presets());
    p.extend(combo_presets());
    p
}

pub fn presets_by_category(cat: PresetCategory) -> Vec<AnimationPreset> {
    all_presets()
        .into_iter()
        .filter(|p| p.category == cat)
        .collect()
}

pub fn find_preset(name: &str) -> Option<AnimationPreset> {
    all_presets().into_iter().find(|p| p.name == name)
}

/// Apply a preset by name; returns false if not found or not applicable.
pub fn apply_by_name(name: &str, layer: &mut Layer, start_frame: u32) -> bool {
    match find_preset(name) {
        Some(p) => apply_preset_to_layer(&p, layer, start_frame),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::property::Animatable;

    #[test]
    fn test_all_presets_non_empty() {
        let presets = all_presets();
        assert!(
            presets.len() >= 20,
            "Expected at least 20 presets, got {}",
            presets.len()
        );
    }

    #[test]
    fn test_presets_by_category() {
        let pos = presets_by_category(PresetCategory::Position);
        assert!(pos.len() >= 4, "Expected at least 4 position presets");
        for p in &pos {
            assert_eq!(p.category, PresetCategory::Position);
        }
    }

    #[test]
    fn test_find_preset() {
        let p = find_preset("Fade In");
        assert!(p.is_some());
        assert_eq!(p.unwrap().category, PresetCategory::Opacity);
    }

    #[test]
    fn test_find_preset_not_found() {
        assert!(find_preset("Nonexistent").is_none());
    }

    #[test]
    fn test_preset_keyframe_serialization() {
        let kf = PresetKeyframe {
            time: 0.5,
            value: 100.0,
            ease: 0.8,
            value_y: None,
        };
        let json = serde_json::to_string(&kf).unwrap();
        let decoded: PresetKeyframe = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.time, 0.5);
        assert_eq!(decoded.value, 100.0);
    }

    #[test]
    fn test_preset_keyframe_with_y() {
        let kf = PresetKeyframe {
            time: 0.0,
            value: 50.0,
            ease: 0.0,
            value_y: Some(75.0),
        };
        let json = serde_json::to_string(&kf).unwrap();
        let decoded: PresetKeyframe = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.value_y, Some(75.0));
    }

    fn test_layer() -> Layer {
        Layer::new(
            "t1".into(),
            "Test".into(),
            crate::core::timeline::LayerType::Solid {
                color: [1.0, 1.0, 1.0, 1.0],
            },
            100,
        )
    }

    #[test]
    fn test_apply_fade_in() {
        let mut layer = test_layer();
        assert!(apply_by_name("Fade In", &mut layer, 10));
        match &layer.transform.opacity {
            Animatable::Animated(kfs) => {
                assert_eq!(kfs.len(), 2);
                assert_eq!(kfs[0].frame, 10);
                assert_eq!(kfs[1].frame, 25);
            }
            _ => panic!("expected animated opacity"),
        }
    }

    #[test]
    fn test_apply_pop_in_preserves_rest_scale() {
        let mut layer = test_layer();
        layer.transform.scale = Animatable::Constant([50.0, 50.0]);
        assert!(apply_by_name("Pop In", &mut layer, 0));
        match &layer.transform.scale {
            Animatable::Animated(kfs) => {
                let last = kfs.last().unwrap();
                assert!((last.value[0] - 50.0).abs() < 0.01);
            }
            _ => panic!("expected animated scale"),
        }
    }

    #[test]
    fn test_apply_position_preset_offsets_from_rest() {
        let mut layer = test_layer();
        layer.transform.position = Animatable::Constant([500.0, 300.0]);
        assert!(apply_by_name("Slide In from Left", &mut layer, 5));
        match &layer.transform.position {
            Animatable::Animated(kfs) => {
                assert!((kfs[0].value[0] - 400.0).abs() < 0.01);
                assert!((kfs[0].value[1] - 300.0).abs() < 0.01);
            }
            _ => panic!("expected animated position"),
        }
    }

    #[test]
    fn test_apply_unknown_returns_false() {
        let mut layer = test_layer();
        assert!(!apply_by_name("No Such Preset", &mut layer, 0));
    }

    #[test]
    fn test_apply_rotation_preset() {
        let mut layer = test_layer();
        layer.transform.rotation = Animatable::Constant(45.0);
        assert!(apply_by_name("Spin Clockwise", &mut layer, 0));
        match &layer.transform.rotation {
            Animatable::Animated(kfs) => {
                let last = kfs.last().unwrap();
                assert!((last.value - 405.0).abs() < 0.01);
            }
            _ => panic!("expected animated rotation"),
        }
    }
}
