use super::*;
use crate::core::keyframe::{InterpolationType, Keyframe};
use crate::core::property::Animatable;
use crate::core::timeline::{Layer, LayerType};

pub fn speed_ramp_init_for_test(l: &mut Layer, out_frame: u32) {
    l.out_frame = out_frame;
    l.time_remap = Some(Animatable::new_animated(vec![
        Keyframe::new(0, 0.0, InterpolationType::Linear),
        Keyframe::new(out_frame, out_frame as f32, InterpolationType::Linear),
    ]));
}

fn mk() -> Layer {
    let mut l = Layer::new(
        "p".into(),
        "P".into(),
        LayerType::Solid { color: [1.0; 4] },
        200,
    );
    l.in_frame = 10;
    l.out_frame = 190;
    l.transform.opacity = Animatable::new_constant(100.0);
    l.transform.scale = Animatable::new_constant([100.0, 100.0]);
    l.transform.position = Animatable::new_constant([960.0, 540.0]);
    l
}

#[test]
fn test_fade_in_creates_two_keyframes() {
    let mut l = mk();
    assert!(apply_by_name("Fade In", &mut l, 30, 1920.0, 1080.0));
    let kfs = l.transform.opacity.keyframes().unwrap();
    assert_eq!(kfs.len(), 2);
    assert_eq!(kfs[0].value, 0.0);
    assert_eq!(kfs[1].value, 100.0);
}

#[test]
fn test_fade_out_fails_when_window_too_small() {
    let mut l = mk();
    l.out_frame = 12; // in=10 → start would be < in
    assert!(!apply_by_name("Fade Out", &mut l, 30, 1920.0, 1080.0));
}

#[test]
fn test_pop_in_overshoot_and_settle() {
    let mut l = mk();
    apply_by_name("Pop In", &mut l, 50, 1920.0, 1080.0);
    let kfs = l.transform.scale.keyframes().unwrap();
    assert_eq!(kfs.len(), 3);
    assert_eq!(kfs[0].value, [0.0, 0.0]);
    assert!((kfs[1].value[0] - 112.0).abs() < 0.01, "overshoot 112%");
    assert_eq!(kfs[2].value, [100.0, 100.0]);
    assert_eq!(l.transform.opacity.keyframes().unwrap().len(), 2);
}

#[test]
fn test_slide_direction_offset() {
    let mut left = mk();
    apply_by_name("Slide In ←", &mut left, 40, 1920.0, 1080.0);
    let kl = left.transform.position.keyframes().unwrap();
    assert!((kl[0].value[0] - (960.0 - 672.0)).abs() < 0.01);

    let mut right = mk();
    apply_by_name("Slide In →", &mut right, 40, 1920.0, 1080.0);
    let kr = right.transform.position.keyframes().unwrap();
    assert!((kr[0].value[0] - (960.0 + 672.0)).abs() < 0.01);
}

#[test]
fn test_unknown_preset_is_noop_false() {
    let mut l = mk();
    assert!(!apply_by_name("Nope", &mut l, 30, 1920.0, 1080.0));
    assert!(l.transform.opacity.keyframes().is_none());
}

#[test]
fn test_speed_ramp_slow_mo_reduces_consumed_source() {
    let mut l = mk();
    speed_ramp_init_for_test(&mut l, 200);
    let lin_at_60 = l.time_remap.as_ref().unwrap().evaluate(60);

    let mut l2 = mk();
    speed_ramp_init_for_test(&mut l2, 200);
    assert!(speed_ramp(&mut l2, 40, 0.25));
    let v = l2.time_remap.as_ref().unwrap().evaluate(60);
    assert!(
        (v - (lin_at_60 - 15.0)).abs() < 0.01,
        "v={} lin={}",
        v,
        lin_at_60
    );
}

#[test]
fn test_speed_ramp_fast_increases_and_auto_inits() {
    let mut l = mk();
    assert!(speed_ramp(&mut l, 80, 4.0));
    assert!(l.time_remap.is_some());
    let kfs = l.time_remap.as_ref().unwrap().keyframes().unwrap();
    assert!(kfs.len() >= 3);
    let b = kfs.iter().find(|k| k.frame == 80).unwrap();
    let c = kfs.iter().find(|k| k.frame == 100).unwrap();
    assert!((c.value - b.value - (100.0 - 80.0) * 4.0).abs() < 0.01);
}

#[test]
fn test_ken_burns_in_zooms_down_and_pans() {
    let mut l = mk();
    assert!(apply_by_name("🎥 Ken Burns In", &mut l, 50, 1920.0, 1080.0));
    let sk = l.transform.scale.keyframes().unwrap();
    assert_eq!(sk.len(), 2);
    assert!(
        (sk[0].value[0] - 130.0).abs() < 0.01,
        "start 130%: {}",
        sk[0].value[0]
    );
    assert_eq!(sk[1].value, [100.0, 100.0]);
    let pk = l.transform.position.keyframes().unwrap();
    assert!((pk[0].value[0] - (960.0 - 153.6)).abs() < 1.0);
}

#[test]
fn test_fade_to_black_reduces_opacity_at_out() {
    let mut l = mk();
    assert!(apply_by_name("Fade to Black", &mut l, 100, 1920.0, 1080.0));
    let kfs = l.transform.opacity.keyframes().unwrap();
    assert_eq!(kfs.len(), 2);
    assert_eq!(kfs[0].value, 100.0);
    assert_eq!(kfs[1].value, 0.0);
    assert_eq!(kfs[1].frame, 189);
}

#[test]
fn test_fade_from_black_at_in() {
    let mut l = mk();
    assert!(apply_by_name("Fade from Black", &mut l, 50, 1920.0, 1080.0));
    let kfs = l.transform.opacity.keyframes().unwrap();
    assert_eq!(kfs[0].frame, 10);
    assert_eq!(kfs[0].value, 0.0);
    assert_eq!(kfs[1].value, 100.0);
}

#[test]
fn test_flash_cut_pushes_glow_effect() {
    let mut l = mk();
    assert!(apply_by_name("⚡ Flash Cut", &mut l, 80, 1920.0, 1080.0));
    assert!(l.effects.iter().any(|e| e.name == "Flash Glow"));
    assert!(l.transform.position.keyframes().unwrap().len() >= 3);
}

#[test]
fn test_expr_bounce_sets_position_expression() {
    let mut l = mk();
    assert!(apply_by_name(
        "\u{1f9f2} Bounce",
        &mut l,
        30,
        1920.0,
        1080.0
    ));
    let expr = l.transform.position_expression.as_ref().unwrap();
    match expr {
        crate::core::timeline::Expression::Raw(s) => {
            assert!(s.contains("sin"), "bounce expr missing sin: {}", s);
            assert!(s.contains("exp"), "bounce expr missing exp: {}", s);
        }
        _ => panic!("expected Raw expression"),
    }
}

#[test]
fn test_expr_elastic_sets_scale_expression() {
    let mut l = mk();
    assert!(apply_by_name(
        "\u{1f300} Elastic",
        &mut l,
        30,
        1920.0,
        1080.0
    ));
    assert!(l.transform.scale_expression.is_some());
}

#[test]
fn test_expr_sine_wave_sets_position() {
    let mut l = mk();
    assert!(apply_by_name(
        "\u{1f30a} Sine Wave",
        &mut l,
        30,
        1920.0,
        1080.0
    ));
    let expr = l.transform.position_expression.as_ref().unwrap();
    match expr {
        crate::core::timeline::Expression::Raw(s) => assert!(s.contains("sin")),
        _ => panic!("expected Raw"),
    }
}

#[test]
fn test_expr_strobe_sets_opacity_expression() {
    let mut l = mk();
    assert!(apply_by_name(
        "\u{1f4a1} Strobe",
        &mut l,
        30,
        1920.0,
        1080.0
    ));
    assert!(l.transform.opacity_expression.is_some());
}

#[test]
fn test_slide_out_moves_right_and_fades() {
    let mut l = mk();
    assert!(apply_by_name(
        "\u{1f3ac} Slide Out \u{2192}",
        &mut l,
        100,
        1920.0,
        1080.0
    ));
    let pk = l.transform.position.keyframes().unwrap();
    assert_eq!(pk.len(), 2);
    assert!(
        (pk[1].value[0] - (960.0 + 960.0)).abs() < 1.0,
        "slide right: {}",
        pk[1].value[0]
    );
    let op = l.transform.opacity.keyframes().unwrap();
    assert_eq!(op[1].value, 0.0);
}

#[test]
fn test_zoom_out_scales_up_and_fades() {
    let mut l = mk();
    assert!(apply_by_name(
        "\u{1f3ac} Zoom Out",
        &mut l,
        100,
        1920.0,
        1080.0
    ));
    let sk = l.transform.scale.keyframes().unwrap();
    assert!(
        (sk[1].value[0] - 200.0).abs() < 0.01,
        "zoom 200%: {}",
        sk[1].value[0]
    );
    let op = l.transform.opacity.keyframes().unwrap();
    assert_eq!(op[1].value, 0.0);
}

#[test]
fn test_text_typewriter_sets_animator_with_offset_anim() {
    let mut l = mk();
    assert!(apply_by_name("Typewriter", &mut l, 30, 1920.0, 1080.0));
    let anim = l.text_animator.as_ref().unwrap();
    assert!(anim.enabled);
    assert!(anim.selector.offset_anim.is_some());
    let oa = anim.selector.offset_anim.as_ref().unwrap();
    assert!((oa.evaluate(30) - (-100.0)).abs() < 0.01);
    assert!((oa.evaluate(60) - 100.0).abs() < 0.01);
}

#[test]
fn test_text_bounce_in_sets_position_expression() {
    let mut l = mk();
    assert!(apply_by_name("Bounce In Text", &mut l, 30, 1920.0, 1080.0));
    assert!(l.transform.position_expression.is_some());
    let anim = l.text_animator.as_ref().unwrap();
    assert_eq!(anim.position_offset, [0.0, -50.0]);
}

#[test]
fn test_text_scale_up_zeroes_scale() {
    let mut l = mk();
    assert!(apply_by_name("Scale Up Text", &mut l, 30, 1920.0, 1080.0));
    let anim = l.text_animator.as_ref().unwrap();
    assert_eq!(anim.scale, [0.0, 0.0]);
    assert_eq!(anim.opacity, 0.0);
}

#[test]
fn test_text_fade_up_has_blur_and_offset() {
    let mut l = mk();
    assert!(apply_by_name("Fade Up Words", &mut l, 30, 1920.0, 1080.0));
    let anim = l.text_animator.as_ref().unwrap();
    assert!((anim.blur_amount - 4.0).abs() < 0.01);
    assert_eq!(anim.position_offset, [0.0, 30.0]);
    assert!(anim.selector.ease_high > 0.0);
}

#[test]
fn test_freeze_frame_locks_to_single_source() {
    let mut l = mk();
    assert!(apply_by_name("Freeze Frame", &mut l, 50, 1920.0, 1080.0));
    let rm = l.time_remap.as_ref().unwrap();
    let kfs = rm.keyframes().unwrap();
    assert_eq!(kfs.len(), 2);
    assert!(
        (kfs[0].value - kfs[1].value).abs() < 0.01,
        "freeze: {} vs {}",
        kfs[0].value,
        kfs[1].value
    );
}

#[test]
fn test_reverse_time_maps_start_to_end() {
    let mut l = mk();
    assert!(apply_by_name("Reverse", &mut l, 50, 1920.0, 1080.0));
    let kfs = l.time_remap.as_ref().unwrap().keyframes().unwrap();
    assert!(
        (kfs[0].value - 180.0).abs() < 0.01,
        "reverse start: {}",
        kfs[0].value
    );
    assert!((kfs[1].value).abs() < 0.01, "reverse end: {}", kfs[1].value);
}

#[test]
fn test_slow_motion_doubles_mapped_span() {
    let mut l = mk();
    assert!(apply_by_name(
        "Slow Motion 0.5\u{00d7}",
        &mut l,
        50,
        1920.0,
        1080.0
    ));
    let kfs = l.time_remap.as_ref().unwrap().keyframes().unwrap();
    assert!((kfs[1].value - 90.0).abs() < 0.01, "slow: {}", kfs[1].value);
}

#[test]
fn test_fast_forward_halves_mapped_span() {
    let mut l = mk();
    assert!(apply_by_name(
        "Fast Forward 2\u{00d7}",
        &mut l,
        50,
        1920.0,
        1080.0
    ));
    let kfs = l.time_remap.as_ref().unwrap().keyframes().unwrap();
    assert!(
        (kfs[1].value - 360.0).abs() < 0.01,
        "fast: {}",
        kfs[1].value
    );
}

#[test]
fn test_youtube_vlog_stacks_four_effects() {
    let mut l = mk();
    assert!(apply_by_name("YouTube Vlog", &mut l, 30, 1920.0, 1080.0));
    assert!(l.effects.len() >= 3, "vlog effects: {}", l.effects.len());
    assert!(l.transform.position_expression.is_some(), "handheld expr");
}

#[test]
fn test_music_video_sets_rotation_expression() {
    let mut l = mk();
    assert!(apply_by_name("Music Video", &mut l, 30, 1920.0, 1080.0));
    assert!(l.transform.rotation_expression.is_some());
    assert!(l.transform.opacity_expression.is_some(), "strobe expr");
}

#[test]
fn test_cinematic_reveal_fades_and_letterboxes() {
    let mut l = mk();
    assert!(apply_by_name(
        "Cinematic Reveal",
        &mut l,
        30,
        1920.0,
        1080.0
    ));
    let op = l.transform.opacity.keyframes().unwrap();
    assert_eq!(op[0].value, 0.0, "fade from black");
    assert!(l.effects.iter().any(|e| e.name == "Letterbox"));
    assert!(l.effects.iter().any(|e| e.name == "Film Look"));
}

#[test]
fn test_documentary_opener_has_grain_and_letterbox() {
    let mut l = mk();
    assert!(apply_by_name(
        "Documentary Opener",
        &mut l,
        30,
        1920.0,
        1080.0
    ));
    let names: Vec<_> = l.effects.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"Film Grain"));
    assert!(names.contains(&"Letterbox"));
    assert!(l.transform.position_expression.is_some());
}

#[test]
fn test_reset_layer_clears_everything() {
    let mut l = mk();
    let _ = apply_by_name("YouTube Vlog", &mut l, 30, 1920.0, 1080.0);
    assert!(!l.effects.is_empty());
    assert!(l.transform.position_expression.is_some());
    assert!(apply_by_name("Reset Layer", &mut l, 30, 1920.0, 1080.0));
    assert!(l.effects.is_empty());
    assert!(l.transform.position_expression.is_none());
    assert!(l.transform.scale_expression.is_none());
    assert!(l.time_remap.is_none());
    assert!(l.text_animator.is_none());
}

#[test]
fn test_film_reel_intro_stacks_four_effects() {
    let mut l = mk();
    assert!(apply_by_name(
        "🎞 Film Reel Intro",
        &mut l,
        30,
        1920.0,
        1080.0
    ));
    let names: Vec<_> = l.effects.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"Film Look"));
    assert!(names.contains(&"Film Grain"));
    assert!(names.contains(&"Letterbox"));
    let op = l.transform.opacity.keyframes().unwrap();
    assert_eq!(op[0].value, 0.0);
    assert!(l.transform.position_expression.is_some());
}
