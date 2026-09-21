use eframe::egui;

pub struct PanelAnimation {
    pub progress: f32,
    pub target: f32,
    pub speed: f32,
    pub is_open: bool,
}

impl PanelAnimation {
    pub fn new(open: bool) -> Self {
        Self {
            progress: if open { 1.0 } else { 0.0 },
            target: if open { 1.0 } else { 0.0 },
            speed: 8.0,
            is_open: open,
        }
    }

    pub fn new_opening() -> Self {
        let mut animation = Self::new(false);
        animation.target = 1.0;
        animation.speed = 2.2;
        animation
    }

    pub fn is_animating(&self) -> bool {
        (self.progress - self.target).abs() >= 0.001
    }

    pub fn update(&mut self, dt: f32) {
        if !self.target.is_finite() {
            self.target = 0.0;
        }
        if !self.progress.is_finite() {
            self.progress = 0.0;
        }
        if !self.speed.is_finite() || self.speed < 0.0 {
            self.speed = 0.0;
        }
        self.target = self.target.clamp(0.0, 1.0);
        self.progress = self.progress.clamp(0.0, 1.0);
        let dt = if dt.is_finite() {
            dt.clamp(0.0, 0.25)
        } else {
            0.0
        };
        if (self.progress - self.target).abs() < 0.001 {
            self.progress = self.target;
            self.is_open = self.target >= 1.0;
            return;
        }
        let step = dt * self.speed;
        if self.progress < self.target {
            self.progress = (self.progress + step).min(self.target);
        } else {
            self.progress = (self.progress - step).max(self.target);
        }
        self.is_open = self.target >= 1.0 && self.progress >= 1.0;
    }

    pub fn set_open(&mut self, open: bool) {
        self.target = if open { 1.0 } else { 0.0 };
        self.is_open = open;
    }

    pub fn toggle(&mut self) {
        self.set_open(self.target < 0.5);
    }

    pub fn ease(t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        1.0 - (1.0 - t).powi(3)
    }
}

pub fn animate_panel_width(
    _ctx: &egui::Context,
    animation: &PanelAnimation,
    max_width: f32,
) -> f32 {
    max_width * PanelAnimation::ease(animation.progress)
}

pub fn animate_panel_height(
    _ctx: &egui::Context,
    animation: &PanelAnimation,
    max_height: f32,
) -> f32 {
    max_height * PanelAnimation::ease(animation.progress)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_open() {
        let a = PanelAnimation::new(true);
        assert_eq!(a.progress, 1.0);
        assert_eq!(a.target, 1.0);
        assert!(a.is_open);
    }

    #[test]
    fn test_new_closed() {
        let a = PanelAnimation::new(false);
        assert_eq!(a.progress, 0.0);
        assert_eq!(a.target, 0.0);
        assert!(!a.is_open);
    }

    #[test]
    fn test_new_opening_starts_closed_and_targets_open() {
        let animation = PanelAnimation::new_opening();
        assert_eq!(animation.progress, 0.0);
        assert_eq!(animation.target, 1.0);
        assert!(!animation.is_open);
        assert!(animation.is_animating());
    }

    #[test]
    fn test_update_sanitizes_invalid_state_and_frame_delta() {
        let mut animation = PanelAnimation::new_opening();
        animation.progress = f32::NAN;
        animation.target = f32::INFINITY;
        animation.speed = f32::NEG_INFINITY;
        animation.update(f32::NAN);
        assert!(animation.progress.is_finite());
        assert!(animation.target.is_finite());
        assert!(animation.speed.is_finite());
        assert!((0.0..=1.0).contains(&animation.progress));
        assert!((0.0..=1.0).contains(&animation.target));
    }

    #[test]
    fn test_irregular_frame_deltas_never_overshoot() {
        let mut animation = PanelAnimation::new_opening();
        for dt in [0.0, 0.001, 0.25, -1.0, 0.016, 0.033, 10.0, 0.008] {
            animation.update(dt);
            assert!(animation.progress.is_finite());
            assert!((0.0..=1.0).contains(&animation.progress));
        }
        animation.set_open(false);
        for dt in [f32::INFINITY, 0.016, 0.25, 0.004, 0.032] {
            animation.update(dt);
            assert!(animation.progress.is_finite());
            assert!((0.0..=1.0).contains(&animation.progress));
        }
    }

    #[test]
    fn test_reversing_mid_flight_converges_to_latest_target() {
        let mut animation = PanelAnimation::new_opening();
        animation.update(0.08);
        assert!(animation.progress > 0.0 && animation.progress < 1.0);
        animation.set_open(false);
        for _ in 0..120 {
            animation.update(1.0 / 60.0);
        }
        assert_eq!(animation.progress, 0.0);
        assert!(!animation.is_open);
        animation.set_open(true);
        for _ in 0..120 {
            animation.update(1.0 / 60.0);
        }
        assert_eq!(animation.progress, 1.0);
        assert!(animation.is_open);
    }

    #[test]
    fn test_long_adversarial_trace_preserves_animation_invariants() {
        let mut animation = PanelAnimation::new_opening();
        let mut entropy = 0x9e37_79b9_u32;

        for frame in 0..20_000 {
            entropy = entropy
                .wrapping_mul(1_664_525)
                .wrapping_add(1_013_904_223);
            if frame % 113 == 0 {
                animation.set_open(entropy & 1 == 0);
            }
            if frame % 997 == 0 {
                animation.progress = if entropy & 2 == 0 {
                    f32::NAN
                } else {
                    f32::INFINITY
                };
            }
            let dt = match frame % 19 {
                0 => f32::NAN,
                1 => f32::NEG_INFINITY,
                2 => f32::INFINITY,
                _ => ((entropy >> 8) % 5_000) as f32 / 1_000.0 - 1.0,
            };
            animation.update(dt);

            assert!(animation.progress.is_finite());
            assert!(animation.target.is_finite());
            assert!(animation.speed.is_finite());
            assert!((0.0..=1.0).contains(&animation.progress));
            assert!((0.0..=1.0).contains(&animation.target));
            if !animation.is_animating() {
                assert_eq!(animation.progress, animation.target);
            }
        }

        animation.speed = 2.2;
        animation.set_open(true);
        for _ in 0..300 {
            animation.update(1.0 / 60.0);
        }
        assert_eq!(animation.progress, 1.0);
        assert!(animation.is_open);
    }

    #[test]
    fn test_update_converges() {
        let mut a = PanelAnimation::new(false);
        a.set_open(true);
        for _ in 0..200 {
            a.update(0.016);
        }
        assert!((a.progress - 1.0).abs() < 0.001);
        assert!(a.is_open);
    }

    #[test]
    fn test_toggle() {
        let mut a = PanelAnimation::new(false);
        a.toggle();
        assert!(a.target >= 1.0);
        a.toggle();
        assert!(a.target < 0.5);
    }

    #[test]
    fn test_ease_bounds() {
        assert_eq!(PanelAnimation::ease(0.0), 0.0);
        assert_eq!(PanelAnimation::ease(1.0), 1.0);
        assert!(PanelAnimation::ease(0.5) > 0.5);
    }

    #[test]
    fn test_animate_width_closed() {
        let ctx = egui::Context::default();
        let a = PanelAnimation::new(false);
        assert_eq!(animate_panel_width(&ctx, &a, 300.0), 0.0);
        assert_eq!(animate_panel_height(&ctx, &a, 200.0), 0.0);
    }

    #[test]
    fn test_animate_width_open() {
        let ctx = egui::Context::default();
        let a = PanelAnimation::new(true);
        assert_eq!(animate_panel_width(&ctx, &a, 300.0), 300.0);
        assert_eq!(animate_panel_height(&ctx, &a, 200.0), 200.0);
    }

    #[test]
    fn test_animate_mid_progress_uses_ease() {
        let ctx = egui::Context::default();
        let mut a = PanelAnimation::new(false);
        a.progress = 0.5;
        // ease(0.5) = 1 - 0.5^3 = 0.875 > linear 0.5: proves easing applies.
        assert_eq!(
            animate_panel_width(&ctx, &a, 300.0),
            300.0 * PanelAnimation::ease(0.5)
        );
        assert!(animate_panel_width(&ctx, &a, 300.0) > 150.0);
    }
}
