pub(crate) fn color_control_value(layer: &crate::core::timeline::Layer, key: &str, frame: u32) -> f32 {
    let Some(effect) = layer
        .effects
        .iter()
        .find(|effect| effect.id == format!("ui_color_{key}"))
    else {
        return match key {
            "contrast" => 0.10,
            "highlights" => -0.20,
            "shadows" => 0.30,
            "saturation" => 1.0,
            _ => 0.0,
        };
    };
    match (&effect.effect_type, key) {
        (crate::core::timeline::EffectType::Levels { gamma, .. }, "exposure") => {
            gamma.evaluate(frame).max(0.0001).log2()
        }
        (crate::core::timeline::EffectType::Levels { input_black, .. }, "contrast") => {
            input_black.evaluate(frame) * 4.0
        }
        (crate::core::timeline::EffectType::ColorBalance { highlights, .. }, "highlights") => {
            highlights[0] / 100.0
        }
        (crate::core::timeline::EffectType::ColorBalance { shadows, .. }, "shadows") => {
            shadows[0] / 100.0
        }
        (crate::core::timeline::EffectType::HueSaturation { saturation, .. }, "saturation") => {
            saturation.evaluate(frame) + 1.0
        }
        _ => 0.0,
    }
}

pub(crate) fn set_color_control(layer: &mut crate::core::timeline::Layer, key: &str, frame: u32, value: f32) {
    use crate::core::property::Animatable;
    use crate::core::timeline::{Effect, EffectType};

    let id = format!("ui_color_{key}");
    if !layer.effects.iter().any(|effect| effect.id == id) {
        let effect_type = match key {
            "exposure" | "contrast" => EffectType::Levels {
                input_black: Animatable::new_constant(0.0),
                input_white: Animatable::new_constant(1.0),
                gamma: Animatable::new_constant(1.0),
                output_black: Animatable::new_constant(0.0),
                output_white: Animatable::new_constant(1.0),
            },
            "highlights" | "shadows" => EffectType::ColorBalance {
                shadows: [0.0; 3],
                midtones: [0.0; 3],
                highlights: [0.0; 3],
                preserve_luminosity: false,
            },
            _ => EffectType::HueSaturation {
                hue_shift: Animatable::new_constant(0.0),
                saturation: Animatable::new_constant(0.0),
                lightness: Animatable::new_constant(0.0),
            },
        };
        layer.effects.push(Effect {
            id: id.clone(),
            name: format!("Color: {}", key),
            effect_type,
            enabled: true,
        });
    }
    let Some(effect) = layer.effects.iter_mut().find(|effect| effect.id == id) else {
        return;
    };
    match (&mut effect.effect_type, key) {
        (EffectType::Levels { gamma, .. }, "exposure") => {
            gamma.set_value_at_frame(frame, 2.0_f32.powf(value))
        }
        (
            EffectType::Levels {
                input_black,
                input_white,
                ..
            },
            "contrast",
        ) => {
            let black = value * 0.25;
            let white = 1.0 - value * 0.25;
            input_black.set_value_at_frame(frame, black);
            input_white.set_value_at_frame(frame, white);
        }
        (EffectType::ColorBalance { highlights, .. }, "highlights") => {
            *highlights = [value * 100.0; 3]
        }
        (EffectType::ColorBalance { shadows, .. }, "shadows") => *shadows = [value * 100.0; 3],
        (EffectType::HueSaturation { saturation, .. }, "saturation") => {
            saturation.set_value_at_frame(frame, value - 1.0)
        }
        _ => {}
    }
}

