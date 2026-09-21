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
    for (index, pixel) in pixels.chunks_exact_mut(4).enumerate() {
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
        .chunks_exact(4)
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

#[test]
fn accepted_project_round_trip_preserves_render_semantics() {
    let project = semantic_project();
    let json = serde_json::to_string(&project).expect("semantic project must serialize");

    let mut variants = Vec::new();
    variants.push(("canonical", serde_json::from_str::<Value>(&json).unwrap()));

    let mut unknown_field = variants[0].1.clone();
    unknown_field["future_metadata"] = serde_json::json!({
        "owner": "test",
        "flags": [true, false, null]
    });
    variants.push(("unknown fields", unknown_field));

    let mut changed_keyframe = variants[0].1.clone();
    changed_keyframe["compositions"][0]["layers"][0]["transform"]["position"]["value"][1]
        ["value"] = serde_json::json!([12.0, 3.0]);
    variants.push(("changed animation", changed_keyframe));

    for (name, value) in variants {
        let mutated_json = serde_json::to_string(&value).unwrap();
        let loaded = kagari_vfx::core::project_migration::load_project_migrated(&mutated_json)
            .unwrap_or_else(|error| panic!("{name} must be accepted: {error}"));
        ProductionDocument::new(loaded.clone())
            .validate()
            .unwrap_or_else(|error| panic!("{name} must remain valid: {error}"));

        for frame in [0, 7, 23] {
            let first = render_frame_to_pixels(&loaded.compositions[0], frame, 16, 16, 0.0, 0);
            let second = render_frame_to_pixels(&loaded.compositions[0], frame, 16, 16, 0.0, 0);
            assert_eq!(
                first, second,
                "{name} is not deterministic at frame {frame}"
            );

            let round_trip = serde_json::to_string(&loaded).unwrap();
            let reloaded = kagari_vfx::core::project_migration::load_project_migrated(&round_trip)
                .unwrap_or_else(|error| panic!("{name} round trip failed: {error}"));
            let after_round_trip =
                render_frame_to_pixels(&reloaded.compositions[0], frame, 16, 16, 0.0, 0);
            assert_eq!(
                first, after_round_trip,
                "{name} changed render semantics after serialization at frame {frame}"
            );
        }
    }
}

#[test]
fn structured_ast_mutations_validate_and_render_deterministically() {
    let seed: Value = serde_json::to_value(semantic_project()).unwrap();
    let mut rng = MutationRng(0xA57_5EED);

    for case in 0..256 {
        let mut mutated = seed.clone();
        match case % 8 {
            0 => {
                mutated["compositions"][0]["width"] =
                    serde_json::json!(rng.choose(&[0u32, 1, 2, 16, 32, 16_384, 16_385, u32::MAX,]))
            }
            1 => {
                mutated["compositions"][0]["height"] =
                    serde_json::json!(rng.choose(&[0u32, 1, 2, 16, 32, 16_384, 16_385, u32::MAX,]))
            }
            2 => {
                mutated["compositions"][0]["fps"] =
                    serde_json::json!(rng.choose(&[0u32, 1, 12, 24, 120, u32::MAX,]))
            }
            3 => {
                mutated["compositions"][0]["duration_frames"] =
                    serde_json::json!(rng.choose(&[0u32, 1, 2, 24, 10_000_001, u32::MAX,]))
            }
            4 => {
                mutated["active_composition_idx"] =
                    serde_json::json!(rng.choose(&[0usize, 1, usize::MAX,]))
            }
            5 => {
                mutated["compositions"][0]["layers"][0]["transform"]["position"]["value"][1]
                    ["frame"] = serde_json::json!(rng.choose(&[0u32, 1, 7, 23, 24, u32::MAX]))
            }
            6 => {
                mutated["compositions"][0]["layers"][0]["effects"][0]["effect_type"]["ColorTint"]
                    ["intensity"]["value"] =
                    serde_json::json!(rng.choose(&[-100.0_f32, 0.0, 35.0, 100.0, 10_000.0]))
            }
            _ => {
                mutated["future"][format!("field_{case}")] = serde_json::json!({
                    "nested": [null, true, {"number": case}]
                })
            }
        }

        let json = serde_json::to_string(&mutated).unwrap();
        let result = catch_unwind(AssertUnwindSafe(|| {
            kagari_vfx::core::project_migration::load_project_migrated(&json)
        }));
        let result = result.unwrap_or_else(|_| panic!("AST mutation {case} must not panic"));

        match result {
            Err(error) => assert!(
                !error.trim().is_empty(),
                "AST mutation {case} hid its error"
            ),
            Ok(project) => {
                if let Err(error) = ProductionDocument::new(project.clone()).validate() {
                    assert!(
                        !error.trim().is_empty(),
                        "AST mutation {case} hid validation error"
                    );
                    continue;
                }

                let composition = &project.compositions[0];
                if composition.width > 64
                    || composition.height > 64
                    || composition.duration_frames == 0
                {
                    continue;
                }
                let frame = composition.duration_frames.saturating_sub(1).min(7);
                let first = render_frame_to_pixels(
                    composition,
                    frame,
                    composition.width,
                    composition.height,
                    0.0,
                    0,
                );
                let second = render_frame_to_pixels(
                    composition,
                    frame,
                    composition.width,
                    composition.height,
                    0.0,
                    0,
                );
                assert_eq!(first, second, "AST mutation {case} was nondeterministic");
                assert_eq!(
                    first.len(),
                    (composition.width * composition.height * 4) as usize,
                    "AST mutation {case} returned the wrong pixel contract"
                );
            }
        }
    }
}

#[test]
fn valid_expression_contracts_are_checked_semantically() {
    let engine = build_engine();
    let cases = [
        ("value + 2", 3.0, 5.0),
        ("frame / fps", 0.0, 2.0),
        ("clamp(value, 0.0, 1.0)", 4.0, 1.0),
        ("linear(value, 0.0, 10.0, 0.0, 100.0)", 5.0, 50.0),
    ];

    for (script, value, expected) in cases {
        let actual = eval_f32(&engine, script, value, 48, 24);
        assert!(actual.is_finite(), "{script} returned a non-finite value");
        assert!(
            (actual - expected).abs() < 1e-5,
            "{script}: {actual} != {expected}"
        );
    }

    for script in ["1 / 0", "sqrt(-1)", "value + missing_variable"] {
        let fallback = eval_f32(&engine, script, 37.0, 48, 24);
        assert!(
            fallback.is_finite(),
            "invalid expression {script} escaped fallback"
        );
        assert_eq!(
            fallback, 37.0,
            "invalid expression {script} must use base value"
        );
    }
}

#[test]
fn generated_safe_expression_trees_match_their_oracle() {
    #[derive(Clone)]
    struct Expr {
        source: String,
        expected: f32,
    }

    fn build(depth: usize, seed: &mut u32) -> Expr {
        if depth == 0 {
            let leaf = match *seed % 3 {
                0 => Expr {
                    source: "value".into(),
                    expected: 6.0,
                },
                1 => Expr {
                    source: "2.0".into(),
                    expected: 2.0,
                },
                _ => Expr {
                    source: "3.0".into(),
                    expected: 3.0,
                },
            };
            *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            return leaf;
        }

        *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let left = build(depth - 1, seed);
        let right = build(depth - 1, seed);
        match *seed % 4 {
            0 => Expr {
                source: format!("({}) + ({})", left.source, right.source),
                expected: left.expected + right.expected,
            },
            1 => Expr {
                source: format!("({}) - ({})", left.source, right.source),
                expected: left.expected - right.expected,
            },
            2 => Expr {
                source: format!("({}) * ({})", left.source, right.source),
                expected: left.expected * right.expected,
            },
            _ => Expr {
                source: format!(
                    "clamp({}, -100.0, 100.0) + clamp({}, -100.0, 100.0)",
                    left.source, right.source
                ),
                expected: left.expected.clamp(-100.0, 100.0)
                    + right.expected.clamp(-100.0, 100.0),
            },
        }
    }

    let engine = build_engine();
    let mut seed = 0x51A7_5EED;
    for depth in 0..=6 {
        for case in 0..32 {
            let expression = build(depth, &mut seed);
            let actual = eval_f32(&engine, &expression.source, 6.0, 48, 24);
            assert!(
                actual.is_finite(),
                "generated expression {depth}/{case} returned {actual}: {}",
                expression.source
            );
            assert!(
                (actual - expression.expected).abs() < 1e-4,
                "generated expression {depth}/{case} changed meaning: {} -> {actual}, expected {}",
                expression.source,
                expression.expected
            );
        }
    }
}

#[test]
fn representative_effects_have_semantic_pixel_contracts() {
    let width = 8;
    let height = 8;
    let input = patterned_pixels(width, height);
    let effects = [
        (
            "invert",
            EffectType::Invert {
                invert_alpha: false,
            },
        ),
        (
            "color tint",
            EffectType::ColorTint {
                color: constant([1.0, 0.0, 0.0, 1.0]),
                intensity: constant(35.0),
            },
        ),
        (
            "gaussian blur",
            EffectType::GaussianBlur {
                blur_radius: constant(2.0),
            },
        ),
        (
            "vignette",
            EffectType::Vignette {
                intensity: constant(80.0),
                roundness: constant(1.0),
                feather: constant(40.0),
                color: constant([0.0, 0.0, 0.0, 1.0]),
            },
        ),
        (
            "linear wipe",
            EffectType::LinearWipe {
                completion: constant(35.0),
                angle: constant(0.0),
            },
        ),
        (
            "offset",
            EffectType::Offset {
                shift_x: constant(1.0),
                shift_y: constant(1.0),
            },
        ),
        (
            "threshold",
            EffectType::Threshold {
                threshold: constant(128.0),
            },
        ),
    ];

    for (name, effect_type) in effects {
        let mut rendered = input.clone();
        apply_layer_effects(
            None,
            None,
            &mut rendered,
            width,
            height,
            &[Effect {
                id: name.into(),
                name: name.into(),
                effect_type,
                enabled: true,
            }],
            0,
            24,
        );
        assert_eq!(rendered.len(), input.len(), "{name} changed buffer size");
        assert_ne!(
            rendered, input,
            "{name} accepted a non-identity parameter but changed nothing"
        );
        assert!(
            distinct_rgb_count(&rendered) > 1,
            "{name} collapsed the frame to one RGB color"
        );
        assert!(
            rendered.chunks_exact(4).any(|pixel| pixel[3] > 0),
            "{name} erased every pixel's alpha"
        );
    }
}

#[test]
fn invalid_effect_buffer_shape_is_a_safe_no_op() {
    let effect = Effect {
        id: "blur".into(),
        name: "Gaussian Blur".into(),
        effect_type: EffectType::GaussianBlur {
            blur_radius: constant(8.0),
        },
        enabled: true,
    };

    for (width, height, length) in [(4, 4, 0), (4, 4, 3), (4, 4, 4 * 4 * 4 - 1), (0, 8, 4)] {
        let mut pixels = vec![73u8; length];
        let before = pixels.clone();
        apply_layer_effects(
            None,
            None,
            &mut pixels,
            width,
            height,
            std::slice::from_ref(&effect),
            0,
            24,
        );
        assert_eq!(pixels, before, "invalid {width}x{height}/{length} buffer mutated");
    }
}

#[test]
fn threshold_has_binary_rgb_semantics_and_preserves_alpha() {
    let mut pixels = vec![
        20, 80, 20, 17, // below threshold
        240, 200, 240, 39, // above threshold
        128, 128, 128, 91, // exactly threshold
        0, 0, 255, 123, // below luminance threshold
    ];
    apply_layer_effects(
        None,
        None,
        &mut pixels,
        4,
        1,
        &[Effect {
            id: "threshold".into(),
            name: "Threshold".into(),
            effect_type: EffectType::Threshold {
                threshold: constant(128.0),
            },
            enabled: true,
        }],
        0,
        24,
    );

    assert_eq!(&pixels[0..4], &[0, 0, 0, 17]);
    assert_eq!(&pixels[4..8], &[255, 255, 255, 39]);
    assert_eq!(&pixels[8..12], &[255, 255, 255, 91]);
    assert_eq!(&pixels[12..16], &[0, 0, 0, 123]);
}

#[test]
fn linear_wipe_progresses_from_left_to_right_at_zero_degrees() {
    let mut pixels = vec![255u8; 8 * 4];
    apply_layer_effects(
        None,
        None,
        &mut pixels,
        8,
        1,
        &[Effect {
            id: "wipe".into(),
            name: "Linear Wipe".into(),
            effect_type: EffectType::LinearWipe {
                completion: constant(50.0),
                angle: constant(0.0),
            },
            enabled: true,
        }],
        0,
        24,
    );
    let alpha: Vec<_> = pixels.chunks_exact(4).map(|pixel| pixel[3]).collect();
    assert_eq!(&alpha[..4], &[255, 255, 255, 255]);
    assert_eq!(&alpha[4..], &[0, 0, 0, 0]);
}

#[test]
fn invert_and_tint_have_pixel_level_semantics() {
    let mut inverted = vec![12, 80, 200, 37, 255, 0, 17, 91];
    apply_layer_effects(
        None,
        None,
        &mut inverted,
        2,
        1,
        &[Effect {
            id: "invert".into(),
            name: "Invert".into(),
            effect_type: EffectType::Invert {
                invert_alpha: false,
            },
            enabled: true,
        }],
        0,
        24,
    );
    assert_eq!(inverted, vec![243, 175, 55, 37, 0, 255, 238, 91]);

    let mut tinted = vec![100, 50, 0, 23];
    apply_layer_effects(
        None,
        None,
        &mut tinted,
        1,
        1,
        &[Effect {
            id: "tint".into(),
            name: "Color Tint".into(),
            effect_type: EffectType::ColorTint {
                color: constant([1.0, 0.0, 0.0, 1.0]),
                intensity: constant(50.0),
            },
            enabled: true,
        }],
        0,
        24,
    );
    assert_eq!(tinted, vec![178, 25, 0, 23]);
}

#[test]
fn mfr_render_reports_each_expected_frame_once() {
    let mut queue = ParallelRenderQueue::new();
    let item_count = 4;
    let frames_per_item = 17;
    for item in 0..item_count {
        queue.add_item(RenderQueueItem {
            comp_name: format!("comp-{item}"),
            start_frame: 3,
            end_frame: 3 + frames_per_item - 1,
            output_path: format!("/tmp/semantic-{item}.rgba"),
            status: RenderStatus::Pending,
        });
    }

    let rendered = Arc::new(Mutex::new(Vec::<(String, u32)>::new()));
    let rendered_for_callback = Arc::clone(&rendered);
    let external_cancel = std::sync::atomic::AtomicBool::new(false);
    let result =
        queue.render_all_mfr_with_external_cancel_checked(&external_cancel, move |name, frame| {
            rendered_for_callback
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .push((name.to_owned(), frame));
            vec![0u8; 4]
        });
    assert_eq!(result, Ok(()));

    let rendered = rendered.lock().unwrap_or_else(|error| error.into_inner());
    assert_eq!(rendered.len(), item_count * frames_per_item as usize);
    let unique: HashSet<_> = rendered.iter().cloned().collect();
    assert_eq!(
        unique.len(),
        rendered.len(),
        "MFR rendered a duplicate frame"
    );
    for item in 0..item_count {
        let name = format!("comp-{item}");
        let frames: Vec<_> = rendered
            .iter()
            .filter_map(|(rendered_name, frame)| (rendered_name == &name).then_some(*frame))
            .collect();
        assert_eq!(
            frames.len(),
            frames_per_item as usize,
            "{name} dropped a frame"
        );
        assert!(frames.iter().all(|frame| (3..=19).contains(frame)));
    }
    assert_eq!(
        queue
            .total_frames_rendered
            .load(std::sync::atomic::Ordering::SeqCst),
        (item_count as u32) * frames_per_item
    );
}

#[test]
fn mfr_cancellation_keeps_frame_identity_and_accounting_consistent() {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    let mut queue = ParallelRenderQueue::new();
    for item in 0..4 {
        queue.add_item(RenderQueueItem {
            comp_name: format!("cancel-{item}"),
            start_frame: 0,
            end_frame: 255,
            output_path: format!("/tmp/cancel-{item}.rgba"),
            status: RenderStatus::Pending,
        });
    }

    let external_cancel = Arc::new(AtomicBool::new(false));
    let first_callback = AtomicUsize::new(0);
    let seen = Arc::new(Mutex::new(Vec::<(String, u32)>::new()));
    let seen_for_callback = Arc::clone(&seen);
    let cancel_for_callback = Arc::clone(&external_cancel);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .expect("test rayon pool must build");
    let result = pool.install(|| {
        queue.render_all_mfr_with_external_cancel_checked(&external_cancel, move |name, frame| {
            seen_for_callback
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .push((name.to_owned(), frame));
            if first_callback.fetch_add(1, Ordering::SeqCst) == 0 {
                cancel_for_callback.store(true, Ordering::SeqCst);
            }
            std::thread::yield_now();
            vec![0u8; 4]
        })
    });

    assert_eq!(result, Ok(()), "cooperative cancellation is not a render error");
    assert!(queue.is_cancelled(), "workers must publish cancellation");
    let seen = seen.lock().unwrap_or_else(|error| error.into_inner());
    let unique: HashSet<_> = seen.iter().cloned().collect();
    assert_eq!(unique.len(), seen.len(), "cancellation duplicated a frame");
    assert!(!seen.is_empty(), "the first callback must have started");
    assert!(
        seen.len() <= queue.total_frames as usize,
        "cancellation rendered more callbacks than the queue total"
    );
    assert_eq!(
        queue.total_frames_rendered.load(Ordering::SeqCst) as usize,
        seen.len(),
        "progress accounting diverged from callback execution"
    );
}

#[test]
fn render_callback_failure_is_reported_with_item_and_frame_identity() {
    let mut queue = ParallelRenderQueue::new();
    queue.add_item(RenderQueueItem {
        comp_name: "broken-comp".into(),
        start_frame: 0,
        end_frame: 4,
        output_path: "/tmp/broken-comp.rgba".into(),
        status: RenderStatus::Pending,
    });

    let external_cancel = std::sync::atomic::AtomicBool::new(false);
    let result = queue.render_all_with_external_cancel_checked(&external_cancel, |_name, frame| {
        assert_ne!(
            frame, 2,
            "frame 2 must be converted to a structured failure"
        );
        vec![0u8; 4]
    });

    assert_eq!(
        result,
        Err(RenderFailure {
            item_index: 0,
            frame: 2,
        })
    );
    assert!(
        queue.is_cancelled(),
        "a worker failure must cancel the queue"
    );
    assert_eq!(
        queue
            .total_frames_rendered
            .load(std::sync::atomic::Ordering::SeqCst),
        2,
        "frames after the failed callback must not be counted as rendered"
    );
}
