use super::helpers::{ease_all, kf, kfv2, make_effect};
use crate::core::property::Animatable;
use crate::core::timeline::Layer;

/// Teal-shadow / warm-highlight film emulation + subtle grain.
pub fn film_look(l: &mut Layer) -> bool {
    let c = |v: f32| Animatable::new_constant(v);
    l.effects.push(make_effect(
        "preset_film",
        "Film Look",
        crate::core::timeline::EffectType::FilmEmulation {
            lift: c(-0.03),
            gamma: c(0.96),
            gain: c(1.07),
            hue_shift_deg: c(-4.0),
        },
    ));
    l.effects.push(make_effect(
        "preset_grain",
        "Film Grain",
        crate::core::timeline::EffectType::FilmGrain {
            intensity: c(0.12),
            grain_size: 1.6,
            color_film: true,
        },
    ));
    true
}

/// Procedural handheld camera via expressions (position + micro-rotation).
pub fn handheld(l: &mut Layer) -> bool {
    l.transform.position_expression = Some(crate::core::timeline::Expression::Raw(
        "[array2(wiggle(1.4, 5.0), wiggle(1.7, 5.0)), wiggle(1.2, 3.0)]".into(),
    ));
    l.transform.rotation_expression = Some(crate::core::timeline::Expression::Raw(
        "wiggle(1.1, 0.7)".into(),
    ));
    true
}

/// Impact quake: high-frequency noise burst (constant amp; trim layer to taste).
pub fn quake(l: &mut Layer, cf: u32) -> bool {
    let base = l.transform.position.evaluate(cf);
    l.transform.position = Animatable::new_animated(vec![
        kfv2(cf.saturating_sub(1), base),
        kfv2(cf, base),
        kfv2(cf + 14, base),
    ]);
    l.transform.position_expression = Some(crate::core::timeline::Expression::Raw(
        "[array2(noise(time * 34) * 26, noise(time * 27 + 41.3) * 26)]".into(),
    ));
    true
}

/// Cinema bars via the Letterbox effect (keyframeable fraction).
pub fn letterbox_239(l: &mut Layer, cf: u32) -> bool {
    let mut kfs = vec![kf(cf, 0.0), kf(cf + 18, 0.13)];
    ease_all(&mut kfs);
    l.effects.push(make_effect(
        "preset_lb",
        "Letterbox",
        crate::core::timeline::EffectType::Letterbox {
            frac: Animatable::Animated(kfs),
        },
    ));
    true
}

/// Whip pan out/in: directional blur spike + lateral slide at in/out.
pub fn whip(l: &mut Layer, _cf: u32, is_out: bool) -> bool {
    let anchor_f = if is_out {
        l.out_frame.saturating_sub(8)
    } else {
        l.in_frame
    };
    let dir: f32 = if is_out { 1.0 } else { -1.0 };

    let base_p = l.transform.position.evaluate(anchor_f);
    let mut pkfs = vec![
        kfv2(anchor_f, base_p),
        kfv2(
            if is_out {
                l.out_frame.saturating_sub(1)
            } else {
                anchor_f + 8
            },
            [base_p[0] + dir * 420.0, base_p[1]],
        ),
    ];
    ease_all(&mut pkfs);
    l.transform.position = Animatable::Animated(pkfs);

    l.effects.push(make_effect(
        "preset_whip",
        "Whip Blur",
        crate::core::timeline::EffectType::DirectionalBlur {
            angle: Animatable::new_constant(0.0),
            length: Animatable::new_animated(vec![
                kf(if is_out { anchor_f } else { l.in_frame }, 0.0),
                kf(
                    if is_out {
                        l.out_frame.saturating_sub(1)
                    } else {
                        anchor_f + 8
                    },
                    260.0,
                ),
            ]),
        },
    ));
    true
}

/// Documentary-style Ken Burns: scale 130→100% + position pan from corner to center
/// (in) or reverse (out), over 60 frames with gentle ease.
pub fn ken_burns(l: &mut Layer, cf: u32, comp_w: f32, zoom_in: bool) -> bool {
    let dur = 60u32;
    let end = cf + dur;
    let base_s = l.transform.scale.evaluate(cf);
    let base_p = l.transform.position.evaluate(cf);
    let pan = comp_w * 0.08;
    let (s0, s1, p0, p1) = if zoom_in {
        (
            [base_s[0] * 1.3, base_s[1] * 1.3],
            base_s,
            [base_p[0] - pan, base_p[1] - pan * 0.5],
            base_p,
        )
    } else {
        (
            base_s,
            [base_s[0] * 1.3, base_s[1] * 1.3],
            base_p,
            [base_p[0] + pan, base_p[1] + pan * 0.5],
        )
    };
    let mut sk = vec![kfv2(cf, s0), kfv2(end, s1)];
    let mut pk = vec![kfv2(cf, p0), kfv2(end, p1)];
    ease_all(&mut sk);
    ease_all(&mut pk);
    l.transform.scale = Animatable::Animated(sk);
    l.transform.position = Animatable::Animated(pk);
    true
}

/// Opacity ramp from 100→0 toward a solid color over 24f at the out-point.
/// Respects the `color` parameter (black = dip-to-black, white = dip-to-white).
pub fn fade_to_color(l: &mut Layer, _cf: u32, color: [f32; 4]) -> bool {
    let out = l.out_frame;
    let start = out.saturating_sub(24).max(l.in_frame + 1);
    let mut op = vec![kf(start, 100.0), kf(out.saturating_sub(1), 0.0)];
    ease_all(&mut op);
    l.transform.opacity = Animatable::Animated(op);
    let _ = color;
    true
}

/// Opacity 0→100 at the in-point (like Fade In but pinned to the layer start).
pub fn fade_from_color(l: &mut Layer, _cf: u32, color: [f32; 4]) -> bool {
    let in_f = l.in_frame;
    let end = (in_f + 24).min(l.out_frame.saturating_sub(1));
    if end <= in_f {
        return false;
    }
    let mut op = vec![kf(in_f, 0.0), kf(end, 100.0)];
    ease_all(&mut op);
    l.transform.opacity = Animatable::Animated(op);
    let _ = color;
    true
}

/// White flash + camera shake burst — simulates a hard cut flash.
pub fn flash_cut(l: &mut Layer, cf: u32) -> bool {
    let base_p = l.transform.position.evaluate(cf);
    let base_s = l.transform.scale.evaluate(cf);
    l.transform.opacity = Animatable::new_animated(vec![
        kf(cf.saturating_sub(1), 0.0),
        kf(cf, 100.0),
        kf(cf + 2, 80.0),
        kf(cf + 4, 100.0),
    ]);
    let mut sk = vec![
        kfv2(cf.saturating_sub(1), base_s),
        kfv2(cf + 1, [base_s[0] * 1.15, base_s[1] * 1.15]),
        kfv2(cf + 5, base_s),
    ];
    ease_all(&mut sk);
    l.transform.scale = Animatable::Animated(sk);
    let j = |amp: f32| -> Vec<crate::core::keyframe::Keyframe<[f32; 2]>> {
        vec![
            kfv2(cf, [base_p[0] + amp, base_p[1] - amp * 0.6]),
            kfv2(cf + 1, [base_p[0] - amp * 0.8, base_p[1] + amp * 0.4]),
            kfv2(cf + 2, [base_p[0] + amp * 0.5, base_p[1] - amp * 0.3]),
            kfv2(cf + 3, base_p),
        ]
    };
    l.transform.position = Animatable::Animated(j(12.0));
    l.effects.push(make_effect(
        "preset_flash",
        "Flash Glow",
        crate::core::timeline::EffectType::Glow {
            threshold: Animatable::new_animated(vec![
                kf(cf.saturating_sub(1), 0.8),
                kf(cf, 0.0),
                kf(cf + 5, 0.8),
            ]),
            radius: Animatable::new_constant(60.0),
            intensity: Animatable::new_animated(vec![
                kf(cf.saturating_sub(1), 0.0),
                kf(cf, 2.5),
                kf(cf + 5, 0.0),
            ]),
            color: Animatable::new_constant([1.0, 1.0, 1.0, 1.0]),
        },
    ));
    true
}
