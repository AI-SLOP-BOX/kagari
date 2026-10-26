use super::types::{AnimationPreset, PresetCategory, PresetKeyframe, PresetPropertyType};

pub(crate) fn position_presets() -> Vec<AnimationPreset> {
    vec![
        AnimationPreset {
            name: "Slide In from Left".into(),
            category: PresetCategory::Position,
            description: "Slides in from the left with overshoot".into(),
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
                    value: 5.0,
                    ease: 0.8,
                    value_y: Some(0.0),
                },
                PresetKeyframe {
                    time: 0.7,
                    value: 0.0,
                    ease: 0.6,
                    value_y: Some(0.0),
                },
            ],
        },
        AnimationPreset {
            name: "Slide In from Right".into(),
            category: PresetCategory::Position,
            description: "Slides in from the right with overshoot".into(),
            property_type: PresetPropertyType::Position,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 100.0,
                    ease: 0.0,
                    value_y: Some(0.0),
                },
                PresetKeyframe {
                    time: 0.5,
                    value: -5.0,
                    ease: 0.8,
                    value_y: Some(0.0),
                },
                PresetKeyframe {
                    time: 0.7,
                    value: 0.0,
                    ease: 0.6,
                    value_y: Some(0.0),
                },
            ],
        },
        AnimationPreset {
            name: "Slide In from Top".into(),
            category: PresetCategory::Position,
            description: "Slides in from above with overshoot".into(),
            property_type: PresetPropertyType::Position,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(-100.0),
                },
                PresetKeyframe {
                    time: 0.5,
                    value: 0.0,
                    ease: 0.8,
                    value_y: Some(5.0),
                },
                PresetKeyframe {
                    time: 0.7,
                    value: 0.0,
                    ease: 0.6,
                    value_y: Some(0.0),
                },
            ],
        },
        AnimationPreset {
            name: "Slide In from Bottom".into(),
            category: PresetCategory::Position,
            description: "Slides in from below with overshoot".into(),
            property_type: PresetPropertyType::Position,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(100.0),
                },
                PresetKeyframe {
                    time: 0.5,
                    value: 0.0,
                    ease: 0.8,
                    value_y: Some(-5.0),
                },
                PresetKeyframe {
                    time: 0.7,
                    value: 0.0,
                    ease: 0.6,
                    value_y: Some(0.0),
                },
            ],
        },
        AnimationPreset {
            name: "Bounce In".into(),
            category: PresetCategory::Position,
            description: "Bounces in from below with decreasing amplitude".into(),
            property_type: PresetPropertyType::Position,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(80.0),
                },
                PresetKeyframe {
                    time: 0.4,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(-15.0),
                },
                PresetKeyframe {
                    time: 0.6,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(8.0),
                },
                PresetKeyframe {
                    time: 0.8,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(-3.0),
                },
                PresetKeyframe {
                    time: 1.0,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(0.0),
                },
            ],
        },
        AnimationPreset {
            name: "Elastic Pop".into(),
            category: PresetCategory::Position,
            description: "Elastic overshoot with rapid oscillation".into(),
            property_type: PresetPropertyType::Position,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(60.0),
                },
                PresetKeyframe {
                    time: 0.3,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(-20.0),
                },
                PresetKeyframe {
                    time: 0.5,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(10.0),
                },
                PresetKeyframe {
                    time: 0.65,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(-5.0),
                },
                PresetKeyframe {
                    time: 0.8,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(2.0),
                },
                PresetKeyframe {
                    time: 1.0,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(0.0),
                },
            ],
        },
    ]
}

pub(crate) fn scale_presets() -> Vec<AnimationPreset> {
    vec![
        AnimationPreset {
            name: "Pop In".into(),
            category: PresetCategory::Scale,
            description: "Scale from 0% to 110% then settle to 100%".into(),
            property_type: PresetPropertyType::Scale,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(0.0),
                },
                PresetKeyframe {
                    time: 0.4,
                    value: 110.0,
                    ease: 0.7,
                    value_y: Some(110.0),
                },
                PresetKeyframe {
                    time: 0.6,
                    value: 100.0,
                    ease: 0.5,
                    value_y: Some(100.0),
                },
            ],
        },
        AnimationPreset {
            name: "Scale Down".into(),
            category: PresetCategory::Scale,
            description: "Scale from 100% to 0%".into(),
            property_type: PresetPropertyType::Scale,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 100.0,
                    ease: 0.5,
                    value_y: Some(100.0),
                },
                PresetKeyframe {
                    time: 0.4,
                    value: -5.0,
                    ease: 0.7,
                    value_y: Some(-5.0),
                },
                PresetKeyframe {
                    time: 0.6,
                    value: 0.0,
                    ease: 0.5,
                    value_y: Some(0.0),
                },
            ],
        },
        AnimationPreset {
            name: "Breathing".into(),
            category: PresetCategory::Scale,
            description: "Continuous scale oscillation between 95% and 105%".into(),
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
                    value: 105.0,
                    ease: 0.5,
                    value_y: Some(105.0),
                },
                PresetKeyframe {
                    time: 1.0,
                    value: 100.0,
                    ease: 0.5,
                    value_y: Some(100.0),
                },
            ],
        },
        AnimationPreset {
            name: "Squash and Stretch".into(),
            category: PresetCategory::Scale,
            description: "Classic cartoon squash-stretch".into(),
            property_type: PresetPropertyType::Scale,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 100.0,
                    ease: 0.0,
                    value_y: Some(100.0),
                },
                PresetKeyframe {
                    time: 0.15,
                    value: 120.0,
                    ease: 0.0,
                    value_y: Some(80.0),
                },
                PresetKeyframe {
                    time: 0.3,
                    value: 85.0,
                    ease: 0.0,
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
    ]
}

pub(crate) fn rotation_presets() -> Vec<AnimationPreset> {
    vec![
        AnimationPreset {
            name: "Spin Clockwise".into(),
            category: PresetCategory::Rotation,
            description: "360 degree clockwise rotation".into(),
            property_type: PresetPropertyType::Rotation,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 0.0,
                    ease: 0.5,
                    value_y: None,
                },
                PresetKeyframe {
                    time: 1.0,
                    value: 360.0,
                    ease: 0.5,
                    value_y: None,
                },
            ],
        },
        AnimationPreset {
            name: "Spin Counter-Clockwise".into(),
            category: PresetCategory::Rotation,
            description: "360 degree counter-clockwise rotation".into(),
            property_type: PresetPropertyType::Rotation,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 0.0,
                    ease: 0.5,
                    value_y: None,
                },
                PresetKeyframe {
                    time: 1.0,
                    value: -360.0,
                    ease: 0.5,
                    value_y: None,
                },
            ],
        },
        AnimationPreset {
            name: "Wobble".into(),
            category: PresetCategory::Rotation,
            description: "Decaying wobble oscillation".into(),
            property_type: PresetPropertyType::Rotation,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 0.0,
                    ease: 0.0,
                    value_y: None,
                },
                PresetKeyframe {
                    time: 0.15,
                    value: 15.0,
                    ease: 0.0,
                    value_y: None,
                },
                PresetKeyframe {
                    time: 0.35,
                    value: -10.0,
                    ease: 0.0,
                    value_y: None,
                },
                PresetKeyframe {
                    time: 0.55,
                    value: 5.0,
                    ease: 0.0,
                    value_y: None,
                },
                PresetKeyframe {
                    time: 0.75,
                    value: -2.0,
                    ease: 0.0,
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
    ]
}
