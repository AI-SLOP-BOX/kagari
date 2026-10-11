//! Stress and consistency tests for cache modules and effect parameter safety.
//! Catches data races, memory accounting errors, and cross-module parameter drift.

use kagari_vfx::core::frame_cache::{self, FrameCache, PixelBufferPool};
use kagari_vfx::core::merkle_frame_cache::MerkleFrameCache;
use kagari_vfx::core::parallel_render::{ParallelRenderQueue, RenderQueueItem, RenderStats};
use kagari_vfx::core::property::Animatable;
use kagari_vfx::core::software_renderer;
use kagari_vfx::core::tile_cache::{self, TileCache};
use kagari_vfx::core::timeline::{Composition, Effect, EffectType, Layer, LayerType};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Barrier, Mutex};

fn fx(effect_type: EffectType) -> Effect {
    Effect {
        id: "test".into(),
        name: "Test".into(),
        effect_type,
        enabled: true,
    }
}

fn c32(v: f32) -> Animatable<f32> {
    Animatable::new_constant(v)
}

fn c32a4(v: [f32; 4]) -> Animatable<[f32; 4]> {
    Animatable::new_constant(v)
}

// ─────────────────────────────────────────────────────────────────────────────
// §1  FrameCache Stress Tests
// ─────────────────────────────────────────────────────────────────────────────
include!("cache_and_effect_stress_tests/cases_01.rs");
include!("cache_and_effect_stress_tests/cases_02.rs");
include!("cache_and_effect_stress_tests/cases_03.rs");
