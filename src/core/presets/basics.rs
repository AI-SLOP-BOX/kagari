use super::helpers::{ease_all, kf, kfv2};
use crate::core::property::Animatable;
use crate::core::timeline::Layer;

/// Opacity 0→100 over the first `dur` frames of the playhead window.
pub fn fade_in(l: &mut Layer, cf: u32, dur: u32) -> bool {
    let end = cf + dur.max(2);
    let mut kfs = vec![kf(cf, 0.0), kf(end, 100.0)];
    ease_all(&mut kfs);
    l.transform.opacity = Animatable::Animated(kfs);
    true
}

/// Opacity 100→0 across the last `dur` frames before the layer out-point.
pub fn fade_out(l: &mut Layer, _cf: u32, dur: u32) -> bool {
    let out = l.out_frame;
    let start = out.saturating_sub(dur.max(2));
    if start <= l.in_frame {
        return false;
    }
    let mut kfs = vec![kf(start, 100.0), kf(out.saturating_sub(1), 0.0)];
    ease_all(&mut kfs);
    l.transform.opacity = Animatable::Animated(kfs);
    true
}

/// Scale pop: 0% → 112% → 100% with overshoot settle.
pub fn pop_in(l: &mut Layer, cf: u32) -> bool {
    let base = l.transform.scale.evaluate(cf);
    let mut kfs = vec![
        kfv2(cf, [0.0, 0.0]),
        kfv2(cf + 12, [base[0] * 1.12, base[1] * 1.12]),
        kfv2(cf + 20, base),
    ];
    ease_all(&mut kfs);
    l.transform.scale = Animatable::Animated(kfs);
    // Pair with a quick opacity snap so it doesn't pop from nothing.
    let mut op = vec![kf(cf, 0.0), kf(cf + 6, 100.0)];
    ease_all(&mut op);
    l.transform.opacity = Animatable::Animated(op);
    true
}

/// Slide in from off-screen left/right while fading up.
pub fn slide_in(l: &mut Layer, cf: u32, comp_w: f32, from_right: bool) -> bool {
    let base = l.transform.position.evaluate(cf);
    let dir: f32 = if from_right { 1.0 } else { -1.0 };
    let start_x = base[0] + dir * comp_w * 0.35;
    let mut pos_kfs = vec![kfv2(cf, [start_x, base[1]]), kfv2(cf + 24, base)];
    ease_all(&mut pos_kfs);
    l.transform.position = Animatable::Animated(pos_kfs);

    let mut op = vec![kf(cf, 0.0), kf(cf + 16, 100.0)];
    ease_all(&mut op);
    l.transform.opacity = Animatable::Animated(op);
    true
}

/// Impact punch: quick scale spike then settle (use on beat hits).
pub fn zoom_punch(l: &mut Layer, cf: u32) -> bool {
    let base = l.transform.scale.evaluate(cf);
    let mut kfs = vec![
        kfv2(cf, base),
        kfv2(cf + 3, [base[0] * 1.18, base[1] * 1.18]),
        kfv2(cf + 8, [base[0] * 0.97, base[1] * 0.97]),
        kfv2(cf + 14, base),
    ];
    ease_all(&mut kfs);
    l.transform.scale = Animatable::Animated(kfs);
    true
}

/// Drop-in shadow emphasis: subtle scale-down + opacity dim then restore,
/// used to make a title land heavier.
pub fn slam_in(l: &mut Layer, cf: u32) -> bool {
    let base_s = l.transform.scale.evaluate(cf);
    let base_p = l.transform.position.evaluate(cf);
    let mut s = vec![
        kfv2(cf, [base_s[0] * 1.6, base_s[1] * 1.6]),
        kfv2(cf + 10, base_s),
    ];
    ease_all(&mut s);
    l.transform.scale = Animatable::Animated(s);

    let mut p = vec![
        kfv2(cf, [base_p[0], base_p[1] - 40.0]),
        kfv2(cf + 10, base_p),
    ];
    ease_all(&mut p);
    l.transform.position = Animatable::Animated(p);
    true
}
