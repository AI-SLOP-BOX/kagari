use super::helpers::{ease_all, kf, kfv2};
use crate::core::property::Animatable;
use crate::core::timeline::Layer;

/// Bounce settle on position Y: overshoots downward then damps.
pub fn expr_bounce(l: &mut Layer) -> bool {
    l.transform.position_expression = Some(crate::core::timeline::Expression::Raw(
        "[value[0], value[1] + 40 * abs(sin(time * 8.0 * 3.14159265)) * exp(-time * 4.0)]".into(),
    ));
    true
}

/// Elastic spring on scale: wobbles like a rubber band after a pull.
pub fn expr_elastic(l: &mut Layer) -> bool {
    l.transform.scale_expression = Some(crate::core::timeline::Expression::Raw(
        "let s = value[0]; [s + s * 0.25 * sin(time * 10.0 * 3.14159265) * exp(-time * 3.0), s + s * 0.25 * sin(time * 10.0 * 3.14159265) * exp(-time * 3.0)]".into(),
    ));
    true
}

/// Continuous sinusoidal oscillation on position Y (breathing / bob effect).
pub fn expr_sine_wave(l: &mut Layer) -> bool {
    l.transform.position_expression = Some(crate::core::timeline::Expression::Raw(
        "[value[0], value[1] + 12 * sin(time * 1.5 * 3.14159265)]".into(),
    ));
    true
}

/// Strobe / flicker: rapid opacity on/off at a given rate (useful for beat hits).
pub fn expr_strobe(l: &mut Layer, _comp_w: f32) -> bool {
    l.transform.opacity_expression = Some(crate::core::timeline::Expression::Raw(
        "if (floor(time * 12.0) % 2 == 0) { 100.0 } else { 0.0 }".into(),
    ));
    true
}

/// Slide the layer off-screen to the right + fade at the out-point.
pub fn slide_out(l: &mut Layer, comp_w: f32) -> bool {
    let out = l.out_frame;
    let start = out.saturating_sub(20).max(l.in_frame + 1);
    let base_p = l.transform.position.evaluate(start);
    let mut pk = vec![
        kfv2(start, base_p),
        kfv2(out.saturating_sub(1), [base_p[0] + comp_w * 0.5, base_p[1]]),
    ];
    let mut op = vec![kf(start, 100.0), kf(out.saturating_sub(1), 0.0)];
    ease_all(&mut pk);
    ease_all(&mut op);
    l.transform.position = Animatable::Animated(pk);
    l.transform.opacity = Animatable::Animated(op);
    true
}

/// Scale up to 200% + fade at the out-point (zoom-through transition).
pub fn zoom_out(l: &mut Layer) -> bool {
    let out = l.out_frame;
    let start = out.saturating_sub(20).max(l.in_frame + 1);
    let base_s = l.transform.scale.evaluate(start);
    let mut sk = vec![
        kfv2(start, base_s),
        kfv2(out.saturating_sub(1), [base_s[0] * 2.0, base_s[1] * 2.0]),
    ];
    let mut op = vec![kf(start, 100.0), kf(out.saturating_sub(1), 0.0)];
    ease_all(&mut sk);
    ease_all(&mut op);
    l.transform.scale = Animatable::Animated(sk);
    l.transform.opacity = Animatable::Animated(op);
    true
}
