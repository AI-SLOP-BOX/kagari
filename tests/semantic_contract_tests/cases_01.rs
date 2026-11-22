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
                expected: left.expected.clamp(-100.0, 100.0) + right.expected.clamp(-100.0, 100.0),
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
            rendered.as_chunks::<4>().0.iter().any(|pixel| pixel[3] > 0),
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
        assert_eq!(
            pixels, before,
            "invalid {width}x{height}/{length} buffer mutated"
        );
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
    let alpha: Vec<_> = pixels
        .as_chunks::<4>()
        .0
        .iter()
        .map(|pixel| pixel[3])
        .collect();
    assert_eq!(&alpha[..4], &[255, 255, 255, 255]);
    assert_eq!(&alpha[4..], &[0, 0, 0, 0]);
}
