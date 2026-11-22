use super::types::{AnimationPreset, PresetCategory, PresetKeyframe, PresetPropertyType};

pub(crate) fn text_presets() -> Vec<AnimationPreset> {
    vec![
        AnimationPreset {
            name: "Typewriter".into(),
            category: PresetCategory::Text,
            description: "Character-by-character reveal".into(),
            property_type: PresetPropertyType::Opacity,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 0.0,
                    ease: 0.0,
                    value_y: None,
                },
                PresetKeyframe {
                    time: 0.05,
                    value: 100.0,
                    ease: 0.0,
                    value_y: None,
                },
            ],
        },
        AnimationPreset {
            name: "Text Wave".into(),
            category: PresetCategory::Text,
            description: "Per-character wave animation".into(),
            property_type: PresetPropertyType::Position,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(10.0),
                },
                PresetKeyframe {
                    time: 0.5,
                    value: 0.0,
                    ease: 0.5,
                    value_y: Some(-10.0),
                },
                PresetKeyframe {
                    time: 1.0,
                    value: 0.0,
                    ease: 0.5,
                    value_y: Some(10.0),
                },
            ],
        },
        AnimationPreset {
            name: "Text Scale Pop".into(),
            category: PresetCategory::Text,
            description: "Per-character scale pop".into(),
            property_type: PresetPropertyType::Scale,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(0.0),
                },
                PresetKeyframe {
                    time: 0.3,
                    value: 120.0,
                    ease: 0.7,
                    value_y: Some(120.0),
                },
                PresetKeyframe {
                    time: 0.5,
                    value: 100.0,
                    ease: 0.5,
                    value_y: Some(100.0),
                },
            ],
        },
    ]
}
