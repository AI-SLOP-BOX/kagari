//! Mister Horse "Animation Composer" & AEJuice-style Non-Destructive Motion Rig.
//!
//! Evaluates IN/OUT transition markers dynamically relative to layer `in_frame`
//! and `out_frame` (no broken baked keyframes when trimming layer duration), with
//! overshoot, elastic, bounce, whip-slide, pop-rotate, and cyber-glitch curves.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EasingCurve {
    Linear,
    ExpoOut,
    BackOut { overshoot: i32 }, // scaled x100, e.g. 170 => 1.70158
    ElasticOut,
    BounceOut,
}

impl EasingCurve {
    pub fn sample(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            EasingCurve::Linear => t,
            EasingCurve::ExpoOut => {
                if t >= 1.0 {
                    1.0
                } else {
                    1.0 - 2.0_f32.powf(-10.0 * t)
                }
            }
            EasingCurve::BackOut { overshoot } => {
                let s = (overshoot as f32) / 100.0;
                let u = t - 1.0;
                1.0 + (s + 1.0) * u * u * u + s * u * u
            }
            EasingCurve::ElasticOut => {
                if t <= 0.0 {
                    0.0
                } else if t >= 1.0 {
                    1.0
                } else {
                    let c4 = (2.0 * std::f32::consts::PI) / 3.0;
                    2.0_f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * c4).sin() + 1.0
                }
            }
            EasingCurve::BounceOut => {
                let n1 = 7.5625_f32;
                let d1 = 2.75_f32;
                if t < 1.0 / d1 {
                    n1 * t * t
                } else if t < 2.0 / d1 {
                    let u = t - 1.5 / d1;
                    n1 * u * u + 0.75
                } else if t < 2.5 / d1 {
                    let u = t - 2.25 / d1;
                    n1 * u * u + 0.9375
                } else {
                    let u = t - 2.625 / d1;
                    n1 * u * u + 0.984375
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComposerCategory {
    EssentialsInOut,
    KineticTitles,
    SocialPopTransitions,
    WhipAndZoom,
    CyberGlitch,
    CinematicReveal,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComposerPreset {
    pub id: &'static str,
    pub display_name: &'static str,
    pub category: ComposerCategory,
    pub curve: EasingCurve,
    pub offset_px: [f32; 2],
    pub start_scale: [f32; 2],
    pub start_rotation_deg: f32,
    pub fade_opacity: bool,
}

/// Curated Animation Composer + AEJuice-style preset catalog.
pub const COMPOSER_BUNDLES: &[ComposerPreset] = &[
    ComposerPreset {
        id: "ac_overshoot_pop",
        display_name: "Overshoot Pop",
        category: ComposerCategory::SocialPopTransitions,
        curve: EasingCurve::BackOut { overshoot: 210 },
        offset_px: [0.0, 0.0],
        start_scale: [0.0, 0.0],
        start_rotation_deg: 0.0,
        fade_opacity: true,
    },
    ComposerPreset {
        id: "ac_elastic_snap",
        display_name: "Elastic Snap",
        category: ComposerCategory::EssentialsInOut,
        curve: EasingCurve::ElasticOut,
        offset_px: [0.0, -40.0],
        start_scale: [0.65, 0.65],
        start_rotation_deg: -8.0,
        fade_opacity: true,
    },
    ComposerPreset {
        id: "ac_whip_slide_left",
        display_name: "Whip Slide Left",
        category: ComposerCategory::WhipAndZoom,
        curve: EasingCurve::ExpoOut,
        offset_px: [480.0, 0.0],
        start_scale: [1.0, 1.0],
        start_rotation_deg: 0.0,
        fade_opacity: false,
    },
    ComposerPreset {
        id: "ac_kinetic_rise",
        display_name: "Kinetic Title Rise",
        category: ComposerCategory::KineticTitles,
        curve: EasingCurve::BackOut { overshoot: 145 },
        offset_px: [0.0, 90.0],
        start_scale: [0.92, 0.92],
        start_rotation_deg: 0.0,
        fade_opacity: true,
    },
    ComposerPreset {
        id: "ac_bounce_drop",
        display_name: "Bounce Drop",
        category: ComposerCategory::SocialPopTransitions,
        curve: EasingCurve::BounceOut,
        offset_px: [0.0, -180.0],
        start_scale: [1.0, 1.0],
        start_rotation_deg: 0.0,
        fade_opacity: true,
    },
    ComposerPreset {
        id: "aej_zoom_punch_rotate",
        display_name: "Zoom Punch + Twist",
        category: ComposerCategory::WhipAndZoom,
        curve: EasingCurve::ExpoOut,
        offset_px: [0.0, 0.0],
        start_scale: [2.4, 2.4],
        start_rotation_deg: 18.0,
        fade_opacity: true,
    },
    ComposerPreset {
        id: "aej_cinematic_push",
        display_name: "Cinematic Reveal Push",
        category: ComposerCategory::CinematicReveal,
        curve: EasingCurve::ExpoOut,
        offset_px: [0.0, 24.0],
        start_scale: [1.18, 1.18],
        start_rotation_deg: 0.0,
        fade_opacity: true,
    },
    ComposerPreset {
        id: "aej_cyber_snap",
        display_name: "Cyber Snap In",
        category: ComposerCategory::CyberGlitch,
        curve: EasingCurve::BackOut { overshoot: 170 },
        offset_px: [-60.0, 0.0],
        start_scale: [1.35, 0.75],
        start_rotation_deg: -4.0,
        fade_opacity: true,
    },
];

pub fn find_composer_preset(id: &str) -> Option<&'static ComposerPreset> {
    COMPOSER_BUNDLES.iter().find(|p| p.id == id)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransitionDelta {
    pub offset_px: [f32; 2],
    pub scale_mul: [f32; 2],
    pub rotation_deg: f32,
    pub opacity_mul: f32,
}

impl Default for TransitionDelta {
    fn default() -> Self {
        Self {
            offset_px: [0.0, 0.0],
            scale_mul: [1.0, 1.0],
            rotation_deg: 0.0,
            opacity_mul: 1.0,
        }
    }
}

/// Non-destructive IN/OUT marker rig attached to a layer.
#[derive(Debug, Clone, Copy)]
pub struct TransitionRig {
    pub in_preset: Option<&'static str>,
    pub out_preset: Option<&'static str>,
    pub in_duration_frames: u32,
    pub out_duration_frames: u32,
}

impl TransitionRig {
    /// Evaluates the non-destructive delta at `frame` for a layer spanning `[layer_in, layer_out]`.
    pub fn evaluate(&self, frame: f32, layer_in: f32, layer_out: f32) -> TransitionDelta {
        let mut delta = TransitionDelta::default();

        if let Some(id) = self.in_preset {
            if let Some(p) = find_composer_preset(id) {
                let dur = (self.in_duration_frames.max(1)) as f32;
                let t = ((frame - layer_in) / dur).clamp(0.0, 1.0);
                if t < 1.0 {
                    let e = p.curve.sample(t);
                    let inv = 1.0 - e;
                    delta.offset_px[0] += p.offset_px[0] * inv;
                    delta.offset_px[1] += p.offset_px[1] * inv;
                    delta.scale_mul[0] *= p.start_scale[0] + (1.0 - p.start_scale[0]) * e;
                    delta.scale_mul[1] *= p.start_scale[1] + (1.0 - p.start_scale[1]) * e;
                    delta.rotation_deg += p.start_rotation_deg * inv;
                    if p.fade_opacity {
                        delta.opacity_mul *= t.clamp(0.0, 1.0);
                    }
                }
            }
        }

        if let Some(id) = self.out_preset {
            if let Some(p) = find_composer_preset(id) {
                let dur = (self.out_duration_frames.max(1)) as f32;
                let out_start = layer_out - dur;
                let t_out = ((frame - out_start) / dur).clamp(0.0, 1.0);
                if frame > out_start {
                    // Reverse progress from 1.0 -> 0.0 at layer_out
                    let e = p.curve.sample(1.0 - t_out);
                    let inv = 1.0 - e;
                    delta.offset_px[0] -= p.offset_px[0] * inv;
                    delta.offset_px[1] -= p.offset_px[1] * inv;
                    delta.scale_mul[0] *= p.start_scale[0] + (1.0 - p.start_scale[0]) * e;
                    delta.scale_mul[1] *= p.start_scale[1] + (1.0 - p.start_scale[1]) * e;
                    delta.rotation_deg -= p.start_rotation_deg * inv;
                    if p.fade_opacity {
                        delta.opacity_mul *= (1.0 - t_out).clamp(0.0, 1.0);
                    }
                }
            }
        }

        delta
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transition_rig_overshoots_in_holds_mid_and_exits_out() {
        let rig = TransitionRig {
            in_preset: Some("ac_overshoot_pop"),
            out_preset: Some("ac_whip_slide_left"),
            in_duration_frames: 15,
            out_duration_frames: 10,
        };

        // At start frame: scale is 0, opacity is 0
        let d_start = rig.evaluate(0.0, 0.0, 100.0);
        assert!(d_start.scale_mul[0].abs() < 1e-5);
        assert!(d_start.opacity_mul.abs() < 1e-5);

        // Halfway through overshoot IN: BackOut overshoots > 1.0
        let d_over = rig.evaluate(10.0, 0.0, 100.0);
        assert!(d_over.scale_mul[0] > 1.0, "expected overshoot > 1.0, got {}", d_over.scale_mul[0]);

        // Mid-layer hold: exact identity
        let d_mid = rig.evaluate(50.0, 0.0, 100.0);
        assert_eq!(d_mid, TransitionDelta::default());

        // At layer_out: slides out by -480px
        let d_end = rig.evaluate(100.0, 0.0, 100.0);
        assert!((d_end.offset_px[0] - (-480.0)).abs() < 1e-3);
    }
}
