use super::helpers::kf;
use crate::core::keyframe::{InterpolationType, Keyframe};
use crate::core::property::Animatable;
use crate::core::text_animator::{RangeSelector, SelectorShape, TextAnimatorSettings};
use crate::core::timeline::Layer;

/// Typewriter: characters appear left-to-right via animated selector offset.
pub fn text_typewriter(l: &mut Layer, cf: u32) -> bool {
    let sel = RangeSelector {
        shape: SelectorShape::RampUp,
        offset_anim: Some(Animatable::new_animated(vec![
            kf(cf, -100.0),
            kf(cf + 30, 100.0),
        ])),
        ..RangeSelector::default()
    };
    l.text_animator = Some(TextAnimatorSettings {
        enabled: true,
        selector: sel,
        position_offset: [0.0, 0.0],
        scale: [1.0, 1.0],
        opacity: 100.0,
        tracking: 0.0,
        rotation: 0.0,
        blur_amount: 0.0,
    });
    true
}

/// Bounce In: characters drop in from above with a bounce expression.
pub fn text_bounce_in(l: &mut Layer, cf: u32) -> bool {
    let sel = RangeSelector {
        shape: SelectorShape::RampUp,
        offset_anim: Some(Animatable::new_animated(vec![
            kf(cf, -100.0),
            kf(cf + 24, 100.0),
        ])),
        ..RangeSelector::default()
    };
    l.text_animator = Some(TextAnimatorSettings {
        enabled: true,
        selector: sel,
        position_offset: [0.0, -50.0],
        scale: [1.0, 1.0],
        opacity: 0.0,
        tracking: 0.0,
        rotation: 0.0,
        blur_amount: 0.0,
    });
    // Layer-level bounce expression on position Y
    l.transform.position_expression = Some(crate::core::timeline::Expression::Raw(
        "[value[0], value[1] + 30 * abs(sin(time * 6.0 * 3.14159265)) * exp(-time * 3.5)]".into(),
    ));
    true
}

/// Scale Up: characters scale from 0% to 100% with overshoot.
pub fn text_scale_up(l: &mut Layer, cf: u32) -> bool {
    let sel = RangeSelector {
        shape: SelectorShape::RampUp,
        offset_anim: Some(Animatable::new_animated(vec![
            kf(cf, -100.0),
            kf(cf + 20, 100.0),
        ])),
        ..RangeSelector::default()
    };
    l.text_animator = Some(TextAnimatorSettings {
        enabled: true,
        selector: sel,
        position_offset: [0.0, 0.0],
        scale: [0.0, 0.0],
        opacity: 0.0,
        tracking: 0.0,
        rotation: 0.0,
        blur_amount: 0.0,
    });
    true
}

/// Fade Up Words: characters fade in while drifting upward (10 at a time).
pub fn text_fade_up(l: &mut Layer, cf: u32) -> bool {
    let sel = RangeSelector {
        shape: SelectorShape::RampUp,
        ease_high: 50.0,
        ease_low: 50.0,
        offset_anim: Some(Animatable::new_animated(vec![
            kf(cf, -100.0),
            kf(cf + 36, 100.0),
        ])),
        ..RangeSelector::default()
    };
    l.text_animator = Some(TextAnimatorSettings {
        enabled: true,
        selector: sel,
        position_offset: [0.0, 30.0],
        scale: [1.0, 1.0],
        opacity: 0.0,
        tracking: 0.0,
        rotation: 0.0,
        blur_amount: 4.0,
    });
    true
}

/// Freeze the layer at the current source frame for its entire duration.
pub fn freeze_frame(l: &mut Layer, cf: u32) -> bool {
    let src = l
        .time_remap
        .as_ref()
        .map(|r| r.evaluate(cf))
        .unwrap_or(cf as f32 - l.in_frame as f32);
    l.time_remap = Some(Animatable::new_animated(vec![
        Keyframe::new(l.in_frame, src, InterpolationType::Linear),
        Keyframe::new(
            l.out_frame.saturating_sub(1),
            src,
            InterpolationType::Linear,
        ),
    ]));
    true
}

/// Reverse the layer's playback direction.
pub fn reverse_time(l: &mut Layer) -> bool {
    let span = l.out_frame.saturating_sub(l.in_frame).max(1);
    l.time_remap = Some(Animatable::new_animated(vec![
        Keyframe::new(l.in_frame, span as f32, InterpolationType::Linear),
        Keyframe::new(
            l.out_frame.saturating_sub(1),
            0.0,
            InterpolationType::Linear,
        ),
    ]));
    true
}

/// Scale the layer's playback speed by `factor` (0.5 = half speed, 2.0 = double).
pub fn time_scale(l: &mut Layer, factor: f32) -> bool {
    let span = l.out_frame.saturating_sub(l.in_frame).max(1) as f32;
    let mapped = span * factor;
    l.time_remap = Some(Animatable::new_animated(vec![
        Keyframe::new(l.in_frame, 0.0, InterpolationType::Linear),
        Keyframe::new(
            l.out_frame.saturating_sub(1),
            mapped,
            InterpolationType::Linear,
        ),
    ]));
    true
}

/// Piecewise speed ramp around the playhead.
pub fn speed_ramp(l: &mut Layer, cf: u32, factor: f32) -> bool {
    const RAMP: u32 = 20;
    let in_f = l.in_frame;
    let out_f = l.out_frame;
    if cf <= in_f || cf >= out_f {
        return false;
    }
    let b = cf;
    let c = (cf + RAMP).min(out_f.saturating_sub(1));
    if c <= b {
        return false;
    }

    if l.time_remap.is_none() {
        let span = out_f.saturating_sub(in_f).max(1);
        l.time_remap = Some(Animatable::new_animated(vec![
            Keyframe::new(0, 0.0f32, InterpolationType::Linear),
            Keyframe::new(out_f.max(1), span as f32, InterpolationType::Linear),
        ]));
    }

    let v_at = |t: u32| l.time_remap.as_ref().unwrap().evaluate(t);
    let va = v_at(in_f.min(b));
    let vb = v_at(b);
    let vd = v_at(out_f.saturating_sub(1));

    let _seg1 = vb - va;
    let r = (c - b) as f32;
    let vc = vb + r * factor;
    let tail = (out_f.saturating_sub(1) - c) as f32;
    let vd2 = vc + tail;

    let mut kfs = vec![
        Keyframe::new(in_f, va, InterpolationType::Linear),
        Keyframe::new(b, vb, InterpolationType::Linear),
        Keyframe::new(c, vc, InterpolationType::Linear),
        Keyframe::new(
            out_f.saturating_sub(1),
            vd2.max(vc),
            InterpolationType::Linear,
        ),
    ];
    let _ = vd;
    kfs.dedup_by(|a2, b2| a2.frame == b2.frame);
    l.time_remap = Some(Animatable::Animated(kfs));
    true
}
