use kagari_vfx::core::cpu_effects::apply_layer_effects;
use kagari_vfx::core::expression_engine::{build_engine, eval_f32};
use kagari_vfx::core::keyframe::{InterpolationType, Keyframe};
use kagari_vfx::core::parallel_render::{ParallelRenderQueue, RenderQueueItem, RenderStatus};
use kagari_vfx::core::production_document::ProductionDocument;
use kagari_vfx::core::property::Animatable;
use kagari_vfx::core::software_renderer::render_frame_to_pixels;
use kagari_vfx::core::timeline::{Composition, Effect, EffectType, Layer, LayerType, Project};
use serde_json::Value;
use std::collections::HashSet;
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
                intensity: constant(100.0),
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
            rendered.chunks_exact(4).any(|pixel| pixel[3] > 0),
            "{name} erased every pixel's alpha"
        );
    }
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
