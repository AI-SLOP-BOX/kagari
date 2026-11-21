//! Hardened regression tests — catch structural bugs, layout mismatches,
//! boundary-value errors, and cross-module inconsistencies.

use kagari_vfx::core::expression_engine;
use kagari_vfx::core::keyframe::InterpolationType;
use kagari_vfx::core::property::Animatable;
use kagari_vfx::core::software_renderer;
use kagari_vfx::core::timeline::{
    BlendMode, Composition, Effect, EffectType, Layer, LayerType, TrackMatteMode,
};

fn fx(effect_type: EffectType) -> Effect {
    Effect {
        id: "test".into(),
        name: "Test".into(),
        effect_type,
        enabled: true,
    }
}

fn fx_disabled(effect_type: EffectType) -> Effect {
    Effect {
        id: "test_disabled".into(),
        name: "TestDisabled".into(),
        effect_type,
        enabled: false,
    }
}

fn c32(v: f32) -> Animatable<f32> {
    Animatable::new_constant(v)
}

fn c32a4(v: [f32; 4]) -> Animatable<[f32; 4]> {
    Animatable::new_constant(v)
}

fn make_gradient(w: u32, h: u32) -> Vec<u8> {
    let mut p = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let idx = ((y * w + x) * 4) as usize;
            let val = (x as f32 / (w - 1).max(1) as f32 * 255.0) as u8;
            p[idx] = val;
            p[idx + 1] = val;
            p[idx + 2] = val;
            p[idx + 3] = 255;
        }
    }
    p
}

fn effects_boundary_sweep(params: Vec<(&str, Vec<Effect>)>) {
    let w = 32u32;
    let h = 32u32;
    for (_name, effects) in &params {
        let mut pixels = make_gradient(w, h);
        kagari_vfx::core::cpu_effects::apply_layer_effects(
            None,
            None,
            &mut pixels,
            w,
            h,
            effects,
            0,
            30,
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// §1  Shader validation
// ─────────────────────────────────────────────────────────────────────────────
include!("hardened_tests/cases_01.rs");
include!("hardened_tests/cases_02.rs");
include!("hardened_tests/cases_03.rs");
