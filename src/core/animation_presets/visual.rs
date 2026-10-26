use super::types::{AnimationPreset, PresetCategory, PresetKeyframe, PresetPropertyType};

pub(crate) fn opacity_presets() -> Vec<AnimationPreset> {
    vec![
        AnimationPreset {
            name: "Fade In".into(),
            category: PresetCategory::Opacity,
            description: "Simple fade from transparent to opaque".into(),
            property_type: PresetPropertyType::Opacity,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 0.0,
                    ease: 0.5,
                    value_y: None,
                },
                PresetKeyframe {
                    time: 0.5,
                    value: 100.0,
                    ease: 0.5,
                    value_y: None,
                },
            ],
        },
        AnimationPreset {
            name: "Fade Out".into(),
            category: PresetCategory::Opacity,
            description: "Simple fade from opaque to transparent".into(),
            property_type: PresetPropertyType::Opacity,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 100.0,
                    ease: 0.5,
                    value_y: None,
                },
                PresetKeyframe {
                    time: 0.5,
                    value: 0.0,
                    ease: 0.5,
                    value_y: None,
                },
            ],
        },
        AnimationPreset {
            name: "Fade In + Hold + Fade Out".into(),
            category: PresetCategory::Opacity,
            description: "Fade in, hold, then fade out".into(),
            property_type: PresetPropertyType::Opacity,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 0.0,
                    ease: 0.5,
                    value_y: None,
                },
                PresetKeyframe {
                    time: 0.3,
                    value: 100.0,
                    ease: 0.5,
                    value_y: None,
                },
                PresetKeyframe {
                    time: 0.7,
                    value: 100.0,
                    ease: 0.5,
                    value_y: None,
                },
                PresetKeyframe {
                    time: 1.0,
                    value: 0.0,
                    ease: 0.5,
                    value_y: None,
                },
            ],
        },
        AnimationPreset {
            name: "Flash".into(),
            category: PresetCategory::Opacity,
            description: "Quick flash: appear, hold, disappear".into(),
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
                PresetKeyframe {
                    time: 0.15,
                    value: 100.0,
                    ease: 0.0,
                    value_y: None,
                },
                PresetKeyframe {
                    time: 0.2,
                    value: 0.0,
                    ease: 0.0,
                    value_y: None,
                },
            ],
        },
    ]
}

pub(crate) fn blur_presets() -> Vec<AnimationPreset> {
    vec![
        AnimationPreset {
            name: "Focus In".into(),
            category: PresetCategory::Focus,
            description: "Blur from 20px to 0px (rack focus)".into(),
            property_type: PresetPropertyType::Blur,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 20.0,
                    ease: 0.5,
                    value_y: None,
                },
                PresetKeyframe {
                    time: 0.5,
                    value: 0.0,
                    ease: 0.5,
                    value_y: None,
                },
            ],
        },
        AnimationPreset {
            name: "Blur Out".into(),
            category: PresetCategory::Focus,
            description: "Blur from 0px to 20px".into(),
            property_type: PresetPropertyType::Blur,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 0.0,
                    ease: 0.5,
                    value_y: None,
                },
                PresetKeyframe {
                    time: 0.5,
                    value: 20.0,
                    ease: 0.5,
                    value_y: None,
                },
            ],
        },
    ]
}

pub(crate) fn combo_presets() -> Vec<AnimationPreset> {
    vec![
        AnimationPreset {
            name: "Pop and Fade In".into(),
            category: PresetCategory::Combo,
            description: "Scale pop + fade in combined".into(),
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
                    value: 115.0,
                    ease: 0.7,
                    value_y: Some(115.0),
                },
                PresetKeyframe {
                    time: 0.5,
                    value: 100.0,
                    ease: 0.5,
                    value_y: Some(100.0),
                },
            ],
        },
        AnimationPreset {
            name: "Shrink and Fade Out".into(),
            category: PresetCategory::Combo,
            description: "Shrink to 0% while fading out".into(),
            property_type: PresetPropertyType::Scale,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 100.0,
                    ease: 0.5,
                    value_y: Some(100.0),
                },
                PresetKeyframe {
                    time: 0.5,
                    value: 0.0,
                    ease: 0.7,
                    value_y: Some(0.0),
                },
            ],
        },
        AnimationPreset {
            name: "Spin and Scale In".into(),
            category: PresetCategory::Combo,
            description: "Spin 360 + scale from 0% combined".into(),
            property_type: PresetPropertyType::Rotation,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(0.0),
                },
                PresetKeyframe {
                    time: 0.5,
                    value: 360.0,
                    ease: 0.7,
                    value_y: Some(110.0),
                },
                PresetKeyframe {
                    time: 0.7,
                    value: 360.0,
                    ease: 0.5,
                    value_y: Some(100.0),
                },
            ],
        },
        AnimationPreset {
            name: "Drop and Bounce".into(),
            category: PresetCategory::Combo,
            description: "Drop from above + bounce scale".into(),
            property_type: PresetPropertyType::Position,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(-80.0),
                },
                PresetKeyframe {
                    time: 0.4,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(5.0),
                },
                PresetKeyframe {
                    time: 0.6,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(-2.0),
                },
                PresetKeyframe {
                    time: 0.8,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(0.0),
                },
            ],
        },
        AnimationPreset {
            name: "Slide and Rotate In".into(),
            category: PresetCategory::Combo,
            description: "Slide from left + rotate 90 degrees".into(),
            property_type: PresetPropertyType::Position,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: -100.0,
                    ease: 0.0,
                    value_y: Some(0.0),
                },
                PresetKeyframe {
                    time: 0.5,
                    value: 0.0,
                    ease: 0.7,
                    value_y: Some(0.0),
                },
                PresetKeyframe {
                    time: 0.7,
                    value: 0.0,
                    ease: 0.5,
                    value_y: Some(0.0),
                },
            ],
        },
    ]
}
