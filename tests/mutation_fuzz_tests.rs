//! Mutation-based fuzzing of project JSON parsing and expression evaluation.
//!
//! Takes valid seeds and applies deterministic random mutations, then verifies
//! that parsing never panics, rendering never panics, and the expression engine
//! always returns (fallback or value — never a crash or hang).

use kagari_vfx::core::expression_engine::{build_engine, eval_f32};
use kagari_vfx::core::production_document::ProductionDocument;
use kagari_vfx::core::property::Animatable;
use kagari_vfx::core::software_renderer::render_frame_to_pixels;
use kagari_vfx::core::timeline::{Composition, Layer, LayerType, Project};
use serde_json::{json, Value};
use std::panic::{catch_unwind, AssertUnwindSafe};

fn seed_project_json() -> String {
    let mut comp = Composition::new("c1".into(), "Seed".into(), 32, 32, 30, 30);
    let mut l = Layer::new(
        "l1".into(),
        "Solid".into(),
        LayerType::Solid { color: [0.5; 4] },
        30,
    );
    l.transform.position = Animatable::new_animated(vec![
        kagari_vfx::core::keyframe::Keyframe::new(
            0,
            [16.0, 16.0],
            kagari_vfx::core::keyframe::InterpolationType::Linear,
        ),
        kagari_vfx::core::keyframe::Keyframe::new(
            29,
            [20.0, 20.0],
            kagari_vfx::core::keyframe::InterpolationType::Bezier {
                outgoing: kagari_vfx::core::keyframe::BezierControlPoint::default(),
                incoming: kagari_vfx::core::keyframe::BezierControlPoint::default(),
                custom_bezier: Some([0.33; 4]),
            },
        ),
    ]);
    comp.layers.push(l);
    serde_json::to_string(&Project {
        use_gpu_compute: false,
        compositions: vec![comp],
        active_composition_idx: 0,
        assets: Vec::new(),
    })
    .unwrap()
}

/// Deterministic PRNG.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let x = ((self.0 >> 18) ^ self.0) >> 27;
        x.rotate_right((self.0 >> 59) as u32)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

#[test]
fn fuzz_mutated_project_json_never_panics() {
    let seed = seed_project_json();
    let mut rng = Rng(0xDEADBEEF);

    for _round in 0..500 {
        let mut bytes = seed.clone().into_bytes();
        let mutations = 1 + rng.below(8);
        for _ in 0..mutations {
            match rng.below(3) {
                // Byte flip
                0 => {
                    let i = rng.below(bytes.len());
                    bytes[i] = (rng.next() & 0xFF) as u8;
                }
                // Byte deletion
                1 if bytes.len() > 2 => {
                    let i = rng.below(bytes.len());
                    bytes.remove(i);
                }
                // Byte insertion
                _ => {
                    let i = rng.below(bytes.len());
                    bytes.insert(i, (rng.next() & 0xFF) as u8);
                }
            }
        }

        let payload = String::from_utf8_lossy(&bytes).to_string();
        // Parse must never panic (Ok or Err are both fine)
        if let Ok(project) = serde_json::from_str::<Project>(&payload) {
            // A syntactically valid Project is not necessarily a valid document.
            // Validate before rendering instead of indexing into an unchecked vector.
            if ProductionDocument::new(project.clone()).validate().is_ok() {
                for frame in [0u32, 15] {
                    let pixels =
                        render_frame_to_pixels(&project.compositions[0], frame, 32, 32, 0.0, 0);
                    assert_eq!(pixels.len(), 32 * 32 * 4);
                }
            }
        }
    }
}

fn structured_project_mutations(seed: &Value) -> Vec<(&'static str, Value)> {
    let mut cases = Vec::new();

    let mut value = seed.clone();
    value["compositions"] = json!([]);
    cases.push(("empty compositions", value));

    let mut value = seed.clone();
    value["active_composition_idx"] = json!(u64::MAX);
    cases.push(("out of range active composition", value));

    for (name, field, replacement) in [
        ("zero width", "width", json!(0)),
        ("zero height", "height", json!(0)),
        ("zero fps", "fps", json!(0)),
        ("zero duration", "duration_frames", json!(0)),
        ("oversized width", "width", json!(u32::MAX)),
    ] {
        let mut value = seed.clone();
        value["compositions"][0][field] = replacement;
        cases.push((name, value));
    }

    let mut value = seed.clone();
    value["compositions"][0]["layers"] = json!([null]);
    cases.push(("null layer entry", value));

    let mut value = seed.clone();
    value["compositions"][0]["layers"] = json!([{"id": "broken"}]);
    cases.push(("incomplete layer object", value));

    let mut value = seed.clone();
    value["assets"] = json!([{
        "id": "asset",
        "name": "asset",
        "item_type": {"Image": {"path": "", "width": 0, "height": 0}}
    }]);
    cases.push(("invalid asset metadata", value));

    let mut value = seed.clone();
    value["compositions"][0]["layers"][0]["transform"]["position"] =
        json!({"type": "Animated", "value": [{"frame": 0, "value": ["x", 0]}]});
    cases.push(("wrong keyframe component type", value));

    let mut value = seed.clone();
    value["unknown_future_field"] = json!({"nested": [null, {"x": true}]});
    cases.push(("unknown forward-compatible field", value));

    let wrapped = json!({
        "schema_version": u32::MAX,
        "project_data": seed.clone()
    });
    cases.push(("future schema wrapper", wrapped));

    cases
}

#[test]
fn structured_project_mutations_return_validation_errors_or_valid_documents() {
    let seed: Value = serde_json::from_str(&seed_project_json()).unwrap();

    for (name, value) in structured_project_mutations(&seed) {
        let json = serde_json::to_string(&value).unwrap();
        let result = catch_unwind(AssertUnwindSafe(|| {
            kagari_vfx::core::project_migration::load_project_migrated(&json)
        }));
        let result = result.unwrap_or_else(|_| panic!("{name}: loading must not panic"));

        match result {
            Ok(project) => {
                ProductionDocument::new(project.clone())
                    .validate()
                    .unwrap_or_else(|error| panic!("{name}: accepted invalid document: {error}"));
                let pixels = render_frame_to_pixels(&project.compositions[0], 0, 32, 32, 0.0, 0);
                assert_eq!(pixels.len(), 32 * 32 * 4, "{name}: invalid render size");
            }
            Err(error) => assert!(!error.trim().is_empty(), "{name}: empty error message"),
        }
    }
}

#[test]
fn fuzz_expression_scripts_never_crash_or_hang() {
    let engine = build_engine();
    // Token soup: operators, functions, nesting, huge numbers, deep parens
    let tokens: Vec<String> = vec![
        "1".into(),
        "0".into(),
        "-1".into(),
        "1e308".into(),
        "value".into(),
        "time".into(),
        "frame".into(),
        "fps".into(),
        "+".into(),
        "-".into(),
        "*".into(),
        "/".into(),
        "%".into(),
        "(".into(),
        ")".into(),
        "[".into(),
        "]".into(),
        ",".into(),
        "sin(".into(),
        "cos(".into(),
        "abs(".into(),
        "clamp(".into(),
        "wiggle(".into(),
        "linear(".into(),
        "ease(".into(),
        "loopOut()".into(),
        "__loop_out_cycle".into(),
        "thisComp.layer(\"x\").transform.position[0]".into(),
        ";".into(),
        "let x =".into(),
        "if true { 1 } else { 0 }".into(),
        "true".into(),
        "false".into(),
        "\"str\"".into(),
        "9".repeat(40),
    ];

    let mut rng = Rng(0xCAFEBABE);
    for _round in 0..800 {
        let parts = 2 + rng.below(12);
        let mut script = String::new();
        for _ in 0..parts {
            script.push_str(&tokens[rng.below(tokens.len())]);
            script.push(' ');
        }

        // Must return some f32 (fallback base on error), never panic/hang.
        // max_operations caps runaway loops so this terminates quickly.
        let result = eval_f32(&engine, &script, 42.0, 10, 30);
        assert!(
            result.is_finite(),
            "unstructured expression produced {result}"
        );
    }
}

fn structured_expression(rng: &mut Rng, depth: usize) -> String {
    if depth == 0 {
        return match rng.below(5) {
            0 => "value".into(),
            1 => "time".into(),
            2 => "frame".into(),
            3 => "1e308".into(),
            _ => "-3.5".into(),
        };
    }

    let left = structured_expression(rng, depth - 1);
    let right = structured_expression(rng, depth - 1);
    match rng.below(7) {
        0 => format!("({left}) + ({right})"),
        1 => format!("({left}) * ({right})"),
        2 => format!("sin({left})"),
        3 => format!("clamp({left}, -1000000, 1000000)"),
        4 => format!("if ({left}) > 0 {{ {right} }} else {{ value }}"),
        5 => format!("[{left}, {right}]"),
        _ => format!("linear({left}, -1, 1, -10, 10)"),
    }
}

#[test]
fn structured_expression_fuzz_preserves_finite_fallback_contract() {
    let engine = build_engine();
    let mut rng = Rng(0x51A7_5EED);

    for depth in 0..=5 {
        for _ in 0..120 {
            let script = structured_expression(&mut rng, depth);
            let result = eval_f32(&engine, &script, 42.0, 10, 30);
            assert!(
                result.is_finite(),
                "structured expression produced {result}: {script}"
            );
        }
    }
}

#[test]
fn fuzz_deeply_nested_json_arrays() {
    // Deep nesting must not blow the recursion stack in serde
    for depth in [64usize, 512, 2000] {
        let mut payload = String::from("{\"compositions\": ");
        for _ in 0..depth {
            payload.push('[');
        }
        payload.push(']');
        for _ in 0..depth {
            payload.push(']');
        }
        payload.push('}');
        // Ok or Err — but no stack overflow / abort
        let _: Result<Project, _> = serde_json::from_str(&payload);
    }
}

#[test]
fn rendering_is_deterministic_same_input_same_bytes() {
    // Determinism is a hard requirement: caching, undo, and export all assume
    // that the same project + frame always produces byte-identical output.
    let mut comp = Composition::new("c".into(), "Determinism".into(), 64, 64, 30, 30);
    let mut l = Layer::new(
        "s".into(),
        "Solid".into(),
        LayerType::Solid {
            color: [0.7, 0.2, 0.4, 1.0],
        },
        30,
    );
    l.transform.position = Animatable::new_animated(vec![
        kagari_vfx::core::keyframe::Keyframe::new(
            0,
            [20.0, 32.0],
            kagari_vfx::core::keyframe::InterpolationType::Bezier {
                outgoing: kagari_vfx::core::keyframe::BezierControlPoint::default(),
                incoming: kagari_vfx::core::keyframe::BezierControlPoint::default(),
                custom_bezier: Some([0.25, 0.1, 0.25, 1.0]),
            },
        ),
        kagari_vfx::core::keyframe::Keyframe::new(
            29,
            [44.0, 32.0],
            kagari_vfx::core::keyframe::InterpolationType::Bezier {
                outgoing: kagari_vfx::core::keyframe::BezierControlPoint::default(),
                incoming: kagari_vfx::core::keyframe::BezierControlPoint::default(),
                custom_bezier: Some([0.25, 0.1, 0.25, 1.0]),
            },
        ),
    ]);
    comp.layers.push(l);

    for frame in [0u32, 7, 15, 28] {
        let a = render_frame_to_pixels(&comp, frame, 64, 64, 0.0, 0);
        let b = render_frame_to_pixels(&comp, frame, 64, 64, 0.0, 0);
        let c = render_frame_to_pixels(&comp, frame, 64, 64, 0.0, 0);
        assert_eq!(a, b, "frame {} must be deterministic", frame);
        assert_eq!(b, c, "frame {} must be deterministic (3rd run)", frame);
    }
}

#[test]
fn roundtrip_project_json_preserves_render() {
    // Save → load → render must produce identical pixels (serialization fidelity)
    use kagari_vfx::core::project_migration::{load_project_migrated, save_project_versioned};
    let mut comp = Composition::new("c".into(), "Roundtrip".into(), 48, 48, 30, 30);
    let mut l = Layer::new(
        "s".into(),
        "Shape".into(),
        LayerType::Solid {
            color: [0.3, 0.9, 0.5, 1.0],
        },
        30,
    );
    l.transform.rotation = Animatable::new_constant(33.0);
    l.transform.position = Animatable::new_constant([24.0, 24.0]);
    comp.layers.push(l);
    let project = Project {
        compositions: vec![comp],
        active_composition_idx: 0,
        assets: Vec::new(),
        use_gpu_compute: false,
    };

    let before = render_frame_to_pixels(&project.compositions[0], 0, 48, 48, 0.0, 0);

    let json = save_project_versioned(&project).expect("serialize");
    let loaded = load_project_migrated(&json).expect("deserialize");
    let after = render_frame_to_pixels(&loaded.compositions[0], 0, 48, 48, 0.0, 0);

    assert_eq!(
        before, after,
        "render must survive a save/load roundtrip byte-for-byte"
    );
}
