use crate::core::timeline::{Effect, EffectType};
use crate::KagariApp;

pub const CB_EFFECT: &str = "Lumetri Color Balance";
pub const VIG_EFFECT: &str = "Lumetri Vignette";
pub const WB_EFFECT: &str = "Lumetri White Balance";
pub const VIB_EFFECT: &str = "Lumetri Vibrance";
pub const HSL_EFFECT: &str = "Lumetri HSL Adjust";

pub type ThreeWay = ([f32; 3], [f32; 3], [f32; 3], bool);
pub type LookPreset<'n> = (&'n str, [f32; 3], [f32; 3], [f32; 3]);
pub type WbPair = (f32, f32);
pub type HslTriple = (f32, f32, f32);
pub(crate) fn read_single_f32<F>(app: &KagariApp, effect_name: &str, field: F) -> Option<f32>
where
    F: Fn(&EffectType, u32) -> Option<f32>,
{
    let idx = app.selection.selected_layer_idx?;
    let comp = app.history.current().active_composition();
    let layer = comp.layers.get(idx)?;
    let e = layer.effects.iter().find(|e| e.name == effect_name)?;
    field(&e.effect_type, app.playback.current_frame)
}

pub(crate) fn read_wb(app: &KagariApp) -> Option<WbPair> {
    let idx = app.selection.selected_layer_idx?;
    let comp = app.history.current().active_composition();
    let layer = comp.layers.get(idx)?;
    let cur_frame = app.playback.current_frame;
    match &layer
        .effects
        .iter()
        .find(|e| e.name == WB_EFFECT)?
        .effect_type
    {
        EffectType::WhiteBalance { temperature, tint } => {
            Some((temperature.evaluate(cur_frame), tint.evaluate(cur_frame)))
        }
        _ => None,
    }
}

pub(crate) fn read_hsl(app: &KagariApp) -> Option<HslTriple> {
    let idx = app.selection.selected_layer_idx?;
    let comp = app.history.current().active_composition();
    let layer = comp.layers.get(idx)?;
    let cur_frame = app.playback.current_frame;
    match &layer
        .effects
        .iter()
        .find(|e| e.name == HSL_EFFECT)?
        .effect_type
    {
        EffectType::HslAdjust {
            hue_deg,
            saturation,
            lightness,
        } => Some((
            hue_deg.evaluate(cur_frame),
            saturation.evaluate(cur_frame),
            lightness.evaluate(cur_frame),
        )),
        _ => None,
    }
}

/// Inserts or updates a single-`Animatable<f32>` Lumetri effect on the selected layer.
pub(crate) fn write_single_f32(
    app: &mut KagariApp,
    effect_name: &'static str,
    id_prefix: &'static str,
    value: f32,
    make: fn(crate::core::property::Animatable<f32>) -> EffectType,
) {
    let Some(idx) = app.selection.selected_layer_idx else {
        return;
    };
    let cur_frame = app.playback.current_frame;
    app.modify_project(move |p| {
        let comp = p.active_composition_mut();
        let Some(layer) = comp.layers.get_mut(idx) else {
            return;
        };
        if let Some(e) = layer.effects.iter_mut().find(|e| e.name == effect_name) {
            e.enabled = true;
            if let EffectType::Vibrance { amount } = &mut e.effect_type {
                amount.set_value_at_frame(cur_frame, value);
            } else {
                e.effect_type = make(crate::core::property::Animatable::new_constant(value));
            }
            return;
        }
        let build = || make(crate::core::property::Animatable::new_constant(value));
        layer.effects.push(Effect {
            id: format!("{id_prefix}_{}", layer.effects.len()),
            name: effect_name.into(),
            effect_type: build(),
            enabled: true,
        });
    });
}

pub(crate) fn write_wb(app: &mut KagariApp, new_temperature: f32, new_tint: f32) {
    let Some(idx) = app.selection.selected_layer_idx else {
        return;
    };
    let cur_frame = app.playback.current_frame;
    app.modify_project(move |p| {
        let comp = p.active_composition_mut();
        let Some(layer) = comp.layers.get_mut(idx) else {
            return;
        };
        if let Some(e) = layer.effects.iter_mut().find(|e| e.name == WB_EFFECT) {
            e.enabled = true;
            if let EffectType::WhiteBalance {
                ref mut temperature,
                ref mut tint,
            } = e.effect_type
            {
                temperature.set_value_at_frame(cur_frame, new_temperature);
                tint.set_value_at_frame(cur_frame, new_tint);
                return;
            }
        }
        let make = || EffectType::WhiteBalance {
            temperature: crate::core::property::Animatable::new_constant(new_temperature),
            tint: crate::core::property::Animatable::new_constant(new_tint),
        };
        layer.effects.push(Effect {
            id: format!("lumetri_wb_{}", layer.effects.len()),
            name: WB_EFFECT.into(),
            effect_type: make(),
            enabled: true,
        });
    });
}

pub(crate) fn write_hsl(app: &mut KagariApp, new_hue: f32, new_sat: f32, new_light: f32) {
    let Some(idx) = app.selection.selected_layer_idx else {
        return;
    };
    let cur_frame = app.playback.current_frame;
    app.modify_project(move |p| {
        let comp = p.active_composition_mut();
        let Some(layer) = comp.layers.get_mut(idx) else {
            return;
        };
        if let Some(e) = layer.effects.iter_mut().find(|e| e.name == HSL_EFFECT) {
            e.enabled = true;
            if let EffectType::HslAdjust {
                ref mut hue_deg,
                ref mut saturation,
                ref mut lightness,
            } = e.effect_type
            {
                hue_deg.set_value_at_frame(cur_frame, new_hue);
                saturation.set_value_at_frame(cur_frame, new_sat);
                lightness.set_value_at_frame(cur_frame, new_light);
                return;
            }
        }
        let make = || EffectType::HslAdjust {
            hue_deg: crate::core::property::Animatable::new_constant(new_hue),
            saturation: crate::core::property::Animatable::new_constant(new_sat),
            lightness: crate::core::property::Animatable::new_constant(new_light),
        };
        layer.effects.push(Effect {
            id: format!("lumetri_hsl_{}", layer.effects.len()),
            name: HSL_EFFECT.into(),
            effect_type: make(),
            enabled: true,
        });
    });
}

/// Reads the live three-way values from the selected layer, if present.
pub(crate) fn read_cb(app: &KagariApp) -> Option<ThreeWay> {
    let idx = app.selection.selected_layer_idx?;
    let comp = app.history.current().active_composition();
    let layer = comp.layers.get(idx)?;
    match &layer
        .effects
        .iter()
        .find(|e| e.name == CB_EFFECT)?
        .effect_type
    {
        EffectType::ColorBalance {
            shadows,
            midtones,
            highlights,
            preserve_luminosity,
        } => Some((*shadows, *midtones, *highlights, *preserve_luminosity)),
        _ => None,
    }
}

/// Inserts or updates the Lumetri Color Balance effect on the selected layer.
pub(crate) fn write_cb(app: &mut KagariApp, s: [f32; 3], m: [f32; 3], h: [f32; 3], pl: bool) {
    let Some(idx) = app.selection.selected_layer_idx else {
        return;
    };
    app.modify_project(move |p| {
        let comp = p.active_composition_mut();
        let Some(layer) = comp.layers.get_mut(idx) else {
            return;
        };
        if let Some(e) = layer.effects.iter_mut().find(|e| e.name == CB_EFFECT) {
            e.enabled = true;
            if let EffectType::ColorBalance {
                shadows,
                midtones,
                highlights,
                preserve_luminosity,
            } = &mut e.effect_type
            {
                *shadows = s;
                *midtones = m;
                *highlights = h;
                *preserve_luminosity = pl;
            }
            return;
        }
        layer.effects.push(Effect {
            id: format!("lumetri_cb_{}", layer.effects.len()),
            name: CB_EFFECT.into(),
            effect_type: EffectType::ColorBalance {
                shadows: s,
                midtones: m,
                highlights: h,
                preserve_luminosity: pl,
            },
            enabled: true,
        });
    });
}

pub(crate) fn remove_effect(app: &mut KagariApp, layer_idx: usize, effect_name: &str) {
    app.modify_project(move |p| {
        let comp = p.active_composition_mut();
        if let Some(layer) = comp.layers.get_mut(layer_idx) {
            layer.effects.retain(|e| e.name != effect_name);
        }
    });
}

/// Reads the live vignette parameters from the selected layer.
pub(crate) fn read_vignette(app: &KagariApp) -> Option<(f32, f32, f32, [f32; 4])> {
    let idx = app.selection.selected_layer_idx?;
    let comp = app.history.current().active_composition();
    let layer = comp.layers.get(idx)?;
    match &layer
        .effects
        .iter()
        .find(|e| e.name == VIG_EFFECT)?
        .effect_type
    {
        EffectType::Vignette {
            intensity,
            roundness,
            feather,
            color,
        } => Some((
            intensity.evaluate(0),
            roundness.evaluate(0),
            feather.evaluate(0),
            color.evaluate(0),
        )),
        _ => None,
    }
}

/// Inserts or updates the Lumetri Vignette effect on the selected layer.
pub(crate) fn write_vignette(
    app: &mut KagariApp,
    intensity: f32,
    roundness: f32,
    feather: f32,
    color: [f32; 4],
) {
    let Some(idx) = app.selection.selected_layer_idx else {
        return;
    };
    app.modify_project(move |p| {
        let comp = p.active_composition_mut();
        let Some(layer) = comp.layers.get_mut(idx) else {
            return;
        };
        let make = || EffectType::Vignette {
            intensity: crate::core::property::Animatable::new_constant(intensity),
            roundness: crate::core::property::Animatable::new_constant(roundness),
            feather: crate::core::property::Animatable::new_constant(feather),
            color: crate::core::property::Animatable::new_constant(color),
        };
        if let Some(e) = layer.effects.iter_mut().find(|e| e.name == VIG_EFFECT) {
            e.enabled = true;
            e.effect_type = make();
            return;
        }
        layer.effects.push(Effect {
            id: format!("lumetri_vig_{}", layer.effects.len()),
            name: VIG_EFFECT.into(),
            effect_type: make(),
            enabled: true,
        });
    });
}
