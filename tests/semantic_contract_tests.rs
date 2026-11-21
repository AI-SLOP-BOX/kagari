use kagari_vfx::core::cpu_effects::apply_layer_effects;
use kagari_vfx::core::expression_engine::{build_engine, eval_f32};
use kagari_vfx::core::keyframe::{InterpolationType, Keyframe};
use kagari_vfx::core::parallel_render::{
    ParallelRenderQueue, RenderFailure, RenderQueueItem, RenderStatus,
};
use kagari_vfx::core::production_document::ProductionDocument;
use kagari_vfx::core::property::Animatable;
use kagari_vfx::core::software_renderer::render_frame_to_pixels;
use kagari_vfx::core::timeline::{Composition, Effect, EffectType, Layer, LayerType, Project};
use serde_json::Value;
use std::collections::HashSet;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc, Mutex};

fn constant<T: Clone>(value: T) -> Animatable<T> {
    Animatable::new_constant(value)
}

fn semantic_project() -> Project {
    let mut composition =
        Composition::new("semantic-comp".into(), "Semantic".into(), 16, 16, 24, 24);
    let mut layer = Layer::new(
        "solid".into(),
        "Semantic Solid".into(),
        LayerType::Solid {
            color: [0.2, 0.45, 0.8, 1.0],
        },
        24,
    );
    layer.transform.position = Animatable::new_animated(vec![
        Keyframe::new(0, [2.0, 2.0], InterpolationType::Linear),
        Keyframe::new(23, [8.0, 6.0], InterpolationType::Linear),
    ]);
    layer.effects.push(Effect {
        id: "tint".into(),
        name: "Color Tint".into(),
        effect_type: EffectType::ColorTint {
            color: constant([1.0, 0.2, 0.1, 1.0]),
            intensity: constant(35.0),
        },
        enabled: true,
    });
    composition.layers.push(layer);

    Project {
        use_gpu_compute: false,
        compositions: vec![composition],
        active_composition_idx: 0,
        assets: Vec::new(),
    }
}

fn patterned_pixels(width: u32, height: u32) -> Vec<u8> {
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    for (index, pixel) in pixels.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let x = (index as u32) % width;
        let y = (index as u32) / width;
        pixel[0] = ((x * 31 + y * 7) % 256) as u8;
        pixel[1] = ((x * 11 + y * 29) % 256) as u8;
        pixel[2] = ((x * 17 + y * 13 + 41) % 256) as u8;
        pixel[3] = 255;
    }
    pixels
}

fn distinct_rgb_count(pixels: &[u8]) -> usize {
    pixels
        .as_chunks::<4>()
        .0
        .iter()
        .map(|pixel| (pixel[0], pixel[1], pixel[2]))
        .collect::<HashSet<_>>()
        .len()
}

struct MutationRng(u64);

impl MutationRng {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }

    fn choose<T: Copy>(&mut self, values: &[T]) -> T {
        values[(self.next() as usize) % values.len()]
    }
}
include!("semantic_contract_tests/cases_01.rs");
include!("semantic_contract_tests/cases_02.rs");
