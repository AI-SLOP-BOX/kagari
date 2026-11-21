use super::basics::*;
use super::cinematic::*;
use super::expressions_transitions::*;
use super::helpers::{kf, make_effect};
use crate::core::property::Animatable;
use crate::core::timeline::Layer;

/// YouTube Vlog: Handheld + Film Look + Letterbox + subtle zoom punch on beat.
pub fn youtube_vlog(l: &mut Layer, cf: u32) -> bool {
    let _ = handheld(l);
    let _ = film_look(l);
    let _ = letterbox_239(l, cf);
    let _ = zoom_punch(l, cf);
    true
}

/// Music Video: Strobe + Quake + high-energy shake for music video cuts.
pub fn music_video(l: &mut Layer, cf: u32) -> bool {
    let _ = quake(l, cf);
    let _ = expr_strobe(l, 1920.0);
    l.transform.rotation_expression = Some(crate::core::timeline::Expression::Raw(
        "noise(time * 40.0) * 3.0".into(),
    ));
    true
}

/// Cinematic Reveal: Fade from black + Ken Burns in + film look: dramatic scene opener.
pub fn cinematic_reveal(l: &mut Layer, cf: u32, comp_w: f32) -> bool {
    let _ = fade_from_color(l, cf, [0.0; 4]);
    let _ = ken_burns(l, cf, comp_w, true);
    let _ = film_look(l);
    let _ = letterbox_239(l, cf);
    true
}

/// Documentary Opener: Slow Ken Burns + fade from black + grain + handheld.
pub fn documentary_opener(l: &mut Layer, cf: u32) -> bool {
    let _ = fade_from_color(l, cf, [0.0; 4]);
    let _ = ken_burns(l, cf, 1920.0, true);
    let c = |v: f32| Animatable::new_constant(v);
    l.effects.push(make_effect(
        "preset_doc_grain",
        "Film Grain",
        crate::core::timeline::EffectType::FilmGrain {
            intensity: c(0.18),
            grain_size: 2.0,
            color_film: false,
        },
    ));
    l.effects.push(make_effect(
        "preset_doc_lb",
        "Letterbox",
        crate::core::timeline::EffectType::Letterbox { frac: c(0.13) },
    ));
    let _ = handheld(l);
    true
}

/// Film Reel Intro: Compound cinematic opener: Film Look + Grain + Letterbox + Fade from Black + Handheld.
pub fn film_reel_intro(l: &mut Layer, cf: u32) -> bool {
    let c = |v: f32| Animatable::new_constant(v);
    l.effects.push(make_effect(
        "preset_fri_film",
        "Film Look",
        crate::core::timeline::EffectType::FilmEmulation {
            lift: c(-0.03),
            gamma: c(0.94),
            gain: c(1.08),
            hue_shift_deg: c(-5.0),
        },
    ));
    l.effects.push(make_effect(
        "preset_fri_grain",
        "Film Grain",
        crate::core::timeline::EffectType::FilmGrain {
            intensity: c(0.15),
            grain_size: 1.8,
            color_film: true,
        },
    ));
    l.effects.push(make_effect(
        "preset_fri_lb",
        "Letterbox",
        crate::core::timeline::EffectType::Letterbox {
            frac: Animatable::new_animated(vec![kf(cf, 0.0), kf(cf + 20, 0.13)]),
        },
    ));
    let _ = fade_from_color(l, cf, [0.0; 4]);
    let _ = handheld(l);
    true
}

/// Clear all effects, expressions, and time remap; reset transforms to defaults.
pub fn reset_layer(l: &mut Layer) -> bool {
    l.effects.clear();
    l.transform.position_expression = None;
    l.transform.scale_expression = None;
    l.transform.rotation_expression = None;
    l.transform.opacity_expression = None;
    l.time_remap = None;
    l.text_animator = None;
    l.transform.position = Animatable::new_constant([960.0, 540.0]);
    l.transform.scale = Animatable::new_constant([100.0, 100.0]);
    l.transform.rotation = Animatable::new_constant(0.0);
    l.transform.opacity = Animatable::new_constant(100.0);
    true
}
