use super::*;

#[test]
fn test_hover_anim_new() {
    let anim = HoverAnim::new();
    assert_eq!(anim.progress, 0.0);
    assert_eq!(anim.target, 0.0);
}

#[test]
fn test_hover_anim_update() {
    let mut anim = HoverAnim::new();
    anim.update(0.1, true);
    assert!(anim.progress > 0.0);
    assert!(anim.progress <= 1.0);
}

#[test]
fn test_hover_anim_settle() {
    let mut anim = HoverAnim::new();
    for _ in 0..100 {
        anim.update(0.016, true);
    }
    assert!((anim.progress - 1.0).abs() < 0.01);
}

#[test]
fn test_color_anim_new() {
    let anim = ColorAnim::new(eframe::egui::Color32::RED);
    assert_eq!(anim.current, eframe::egui::Color32::RED);
    assert_eq!(anim.target, eframe::egui::Color32::RED);
}

#[test]
fn test_color_anim_transition() {
    let mut anim = ColorAnim::new(eframe::egui::Color32::RED);
    anim.set_target(eframe::egui::Color32::BLUE);
    for _ in 0..100 {
        anim.update(0.016);
    }
    assert!(anim.current.b() > anim.current.r());
}

#[test]
fn test_pulse_anim_new() {
    let anim = PulseAnim::new();
    assert_eq!(anim.phase, 0.0);
    assert_eq!(anim.min_alpha, 180);
    assert_eq!(anim.max_alpha, 255);
}

#[test]
fn test_pulse_anim_alpha_range() {
    let mut anim = PulseAnim::new();
    let mut min = 255u8;
    let mut max = 0u8;
    for _ in 0..100 {
        anim.update(0.05);
        let a = anim.alpha();
        min = min.min(a);
        max = max.max(a);
    }
    assert!(min >= anim.min_alpha);
    assert!(max <= anim.max_alpha);
}
