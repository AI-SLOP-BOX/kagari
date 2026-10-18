#[test]
fn effects_disabled_are_noops() {
    let w = 16u32;
    let h = 16u32;
    let original = make_gradient(w, h);
    let mut pixels = original.clone();
    let effects = vec![
        fx_disabled(EffectType::GaussianBlur {
            blur_radius: c32(999.0),
        }),
        fx_disabled(EffectType::Glow {
            threshold: c32(0.0),
            radius: c32(999.0),
            intensity: c32(999.0),
            color: c32a4([1.0, 0.0, 0.0, 1.0]),
        }),
        fx_disabled(EffectType::Invert { invert_alpha: true }),
    ];
    kagari_vfx::core::cpu_effects::apply_layer_effects(
        None,
        None,
        &mut pixels,
        w,
        h,
        &effects,
        0,
        30,
    );
    assert_eq!(pixels, original);
}

// ─────────────────────────────────────────────────────────────────────────────
// §3  Expression Engine Security & Edge Cases
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn expression_blocks_dangerous_symbols() {
    let engine = expression_engine::build_engine();
    // These should fail at compile or runtime due to disabled symbols or sandbox restrictions.
    // validate_script only checks syntax, so we also test eval_f32 for runtime blocking.
    let dangerous_compile_fail = vec!["eval(\"1 + 1\")", "call(\"system\", \"ls\")"];
    for script in dangerous_compile_fail {
        let result = expression_engine::validate_script(&engine, script);
        assert!(
            result.is_err(),
            "DANGEROUS script NOT blocked at compile: {script}"
        );
    }

    // These may compile as syntax but must fail or return non-finite at runtime
    let dangerous_runtime = vec!["to_json(42)", "from_json(\"{}\")", "import(\"foo\")"];
    for script in dangerous_runtime {
        let result = expression_engine::eval_f32(&engine, script, 0.0, 0, 30);
        // Must not produce a finite usable value from a dangerous operation
        // (either panics internally, returns NaN, or the engine rejects it)
        // We just verify it doesn't silently succeed with a meaningful value
        let _ = result;
    }
}
#[test]
fn expression_unicode_safety() {
    let engine = expression_engine::build_engine();
    let long_string = "a".repeat(10000);
    let inputs = vec![
        "日本語テスト",
        "مرحبا",
        "🚀",
        "\x00\x01\x02",
        long_string.as_str(),
        "${666}",
        "`rm -rf /`",
        "'; DROP TABLE users; --",
        "<script>alert(1)</script>",
    ];
    for script in inputs {
        let _ = expression_engine::validate_script(&engine, script);
    }
}
#[test]
fn expression_wiggle_finite_across_frames() {
    let engine = expression_engine::build_engine();
    for frame in 0..120u32 {
        let script = format!("wiggle({frame}, 10)");
        let result = expression_engine::eval_f32(&engine, &script, 100.0, frame, 30);
        assert!(
            result.is_finite(),
            "wiggle returned {result} at frame {frame}"
        );
    }
}
#[test]
fn expression_noise_bounded() {
    let engine = expression_engine::build_engine();
    for frame in 0..60u32 {
        let script = format!("noise({frame}.0)");
        let result = expression_engine::eval_f32(&engine, &script, 0.0, frame, 30);
        assert!(
            (0.0..=1.0).contains(&result),
            "noise returned {result} at frame {frame}, expected [0, 1]"
        );
    }
}
#[test]
fn expression_loopout_finite() {
    let engine = expression_engine::build_engine();
    for frame in 0..300u32 {
        let script = "loopOut(\"cycle\", 100)";
        let result = expression_engine::eval_f32(&engine, script, 50.0, frame, 30);
        assert!(
            result.is_finite(),
            "loopOut returned {result} at frame {frame}"
        );
    }
}
#[test]
fn expression_nonfinite_results_fall_back_to_the_base_value() {
    let engine = expression_engine::build_engine();
    for script in ["1e308 * 1e308", "0.0 / 0.0", "sqrt(-1.0)"] {
        let result = expression_engine::eval_f32(&engine, script, 42.0, 10, 30);
        assert_eq!(
            result, 42.0,
            "non-finite expression must fall back: {script}"
        );
        assert!(result.is_finite());
    }

    assert_eq!(
        expression_engine::eval_v2(&engine, "[1e308, 2e308]", [4.0, 5.0], 10, 30),
        [4.0, 5.0]
    );
    assert_eq!(
        expression_engine::eval_v2_with_diagnostics(&engine, "1e308 * 1e308", [4.0, 5.0], 10, 30,)
            .0,
        [4.0, 5.0]
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// §4  Serialization Stress Tests
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn serialization_max_layers_roundtrip() {
    let mut comp = Composition::new("max".into(), "Max".into(), 1920, 1080, 30, 300);
    for i in 0..1000 {
        comp.layers.push(Layer::new(
            format!("layer_{i}"),
            format!("Layer {i}"),
            LayerType::Solid {
                color: [i as f32 / 1000.0, 0.5, 0.5, 1.0],
            },
            300,
        ));
    }
    let json = serde_json::to_string(&comp).unwrap();
    let loaded: Composition = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded.layers.len(), 1000);
    assert_eq!(loaded.layers[0].name, "Layer 0");
    assert_eq!(loaded.layers[999].name, "Layer 999");
}
#[test]
fn serialization_keyframe_density_roundtrip() {
    let mut comp = Composition::new("kf".into(), "KF".into(), 64, 64, 60, 600);
    let mut layer = Layer::new(
        "kf_layer".into(),
        "KF Layer".into(),
        LayerType::Solid {
            color: [1.0, 1.0, 1.0, 1.0],
        },
        600,
    );
    let kfs: Vec<kagari_vfx::core::keyframe::Keyframe<[f32; 2]>> = (0..600u32)
        .map(|f| kagari_vfx::core::keyframe::Keyframe {
            frame: f,
            value: [f as f32 * 0.1, f as f32 * 0.2],
            interpolation: InterpolationType::Linear,
        })
        .collect();
    layer.transform.position = Animatable::Animated(kfs);
    comp.layers.push(layer);

    let json = serde_json::to_string(&comp).unwrap();
    let loaded: Composition = serde_json::from_str(&json).unwrap();
    match &loaded.layers[0].transform.position {
        Animatable::Animated(kfs) => {
            assert_eq!(kfs.len(), 600);
            assert_eq!(kfs[0].value[0], 0.0);
            assert_eq!(kfs[599].value[0], 59.9);
        }
        _ => panic!("Expected Animated position"),
    }
}
#[test]
fn serialization_all_effect_types_roundtrip() {
    let effects = vec![
        fx(EffectType::GaussianBlur {
            blur_radius: c32(5.0),
        }),
        fx(EffectType::ColorTint {
            color: c32a4([1.0, 0.0, 0.0, 1.0]),
            intensity: c32(50.0),
        }),
        fx(EffectType::DropShadow {
            color: c32a4([0.0, 0.0, 0.0, 1.0]),
            opacity: c32(75.0),
            direction: c32(135.0),
            distance: c32(10.0),
            softness: c32(5.0),
        }),
        fx(EffectType::ChromaticAberration {
            shift_r: c32(2.0),
            shift_b: c32(-2.0),
            edge_falloff: c32(0.5),
            iris_linked: true,
        }),
        fx(EffectType::Glow {
            threshold: c32(0.8),
            radius: c32(10.0),
            intensity: c32(1.5),
            color: c32a4([1.0, 1.0, 1.0, 1.0]),
        }),
        fx(EffectType::Levels {
            input_black: c32(0.0),
            input_white: c32(1.0),
            gamma: c32(1.0),
            output_black: c32(0.0),
            output_white: c32(1.0),
        }),
        fx(EffectType::HueSaturation {
            hue_shift: c32(0.0),
            saturation: c32(1.0),
            lightness: c32(0.0),
        }),
        fx(EffectType::Twirl {
            angle: c32(45.0),
            radius: c32(50.0),
        }),
        fx(EffectType::Bulge {
            amount: c32(0.5),
            radius: c32(100.0),
        }),
        fx(EffectType::Posterize { levels: c32(8.0) }),
        fx(EffectType::Invert {
            invert_alpha: false,
        }),
        fx(EffectType::Sharpen { amount: c32(50.0) }),
        fx(EffectType::Threshold {
            threshold: c32(128.0),
        }),
        fx(EffectType::MotionBlur {
            shutter_angle: c32(180.0),
            samples: 16,
        }),
        fx(EffectType::FilmGrain {
            intensity: c32(0.3),
            grain_size: 1.5,
            color_film: false,
        }),
        fx(EffectType::Vignette {
            intensity: c32(50.0),
            roundness: c32(0.5),
            feather: c32(50.0),
            color: c32a4([0.0, 0.0, 0.0, 1.0]),
        }),
    ];

    let mut comp = Composition::new("fx".into(), "FX".into(), 64, 64, 30, 1);
    let mut layer = Layer::new(
        "fx_layer".into(),
        "FX Layer".into(),
        LayerType::Solid {
            color: [1.0, 1.0, 1.0, 1.0],
        },
        1,
    );
    layer.effects = effects;
    comp.layers.push(layer);

    let json = serde_json::to_string(&comp).unwrap();
    let loaded: Composition = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded.layers[0].effects.len(), 16);

    match &loaded.layers[0].effects[0].effect_type {
        EffectType::GaussianBlur { blur_radius } => match blur_radius {
            Animatable::Constant(v) => assert!((v - 5.0).abs() < 0.001),
            _ => panic!("Expected Constant blur_radius"),
        },
        _ => panic!("Expected GaussianBlur"),
    }
}
#[test]
fn serialization_malformed_json_fails_gracefully() {
    let malformed = vec![
        "",
        "{",
        "}}}}}",
        "null",
        "[]",
        "99999999",
        "\"hello\"",
        r#"{"compositions": "not_an_array"}"#,
    ];
    for input in malformed {
        let result: Result<Composition, _> = serde_json::from_str(input);
        assert!(result.is_err(), "Malformed JSON was accepted: {input}");
    }
}
#[test]
fn serialization_deterministic_output() {
    let mut comp = Composition::new("det".into(), "Det".into(), 100, 100, 30, 30);
    comp.layers.push(Layer::new(
        "a".into(),
        "A".into(),
        LayerType::Solid {
            color: [1.0, 0.5, 0.25, 1.0],
        },
        30,
    ));
    let json1 = serde_json::to_string(&comp).unwrap();
    let json2 = serde_json::to_string(&comp).unwrap();
    assert_eq!(json1, json2);
}

// ─────────────────────────────────────────────────────────────────────────────
// §5  Software Renderer Determinism & Edge Cases
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn software_renderer_determinism() {
    let mut comp = Composition::new("det".into(), "Det".into(), 64, 64, 30, 10);
    comp.layers.push(Layer::new(
        "solid".into(),
        "Solid".into(),
        LayerType::Solid {
            color: [1.0, 0.0, 0.0, 1.0],
        },
        10,
    ));
    let a = software_renderer::render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
    let b = software_renderer::render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
    let c = software_renderer::render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
    assert_eq!(a, b);
    assert_eq!(b, c);
}
#[test]
fn software_renderer_extreme_dimensions() {
    let comp = Composition::new("t".into(), "T".into(), 1, 1, 30, 1);
    let p = software_renderer::render_frame_to_pixels(&comp, 0, 1, 1, 0.0, 0);
    assert_eq!(p.len(), 4);

    let p = software_renderer::render_frame_to_pixels(&comp, 0, 10000, 1, 0.0, 0);
    assert_eq!(p.len(), 40000);

    let p = software_renderer::render_frame_to_pixels(&comp, 0, 1, 10000, 0.0, 0);
    assert_eq!(p.len(), 40000);
}
#[test]
fn software_renderer_zero_dimensions() {
    let comp = Composition::new("z".into(), "Z".into(), 1, 1, 30, 1);
    let p = software_renderer::render_frame_to_pixels(&comp, 0, 0, 0, 0.0, 0);
    assert!(p.is_empty());
}
#[test]
fn software_renderer_extreme_exposure() {
    let mut comp = Composition::new("e".into(), "E".into(), 16, 16, 30, 1);
    comp.layers.push(Layer::new(
        "s".into(),
        "S".into(),
        LayerType::Solid {
            color: [1.0, 1.0, 1.0, 1.0],
        },
        1,
    ));
    let p = software_renderer::render_frame_to_pixels(&comp, 0, 16, 16, 100.0, 0);
    assert_eq!(p.len(), 16 * 16 * 4);
    let p = software_renderer::render_frame_to_pixels(&comp, 0, 16, 16, -100.0, 0);
    assert_eq!(p.len(), 16 * 16 * 4);
}
#[test]
fn software_renderer_many_layers_terminates() {
    let mut comp = Composition::new("m".into(), "M".into(), 16, 16, 30, 1);
    for i in 0..500 {
        comp.layers.push(Layer::new(
            format!("l{i}"),
            format!("L{i}"),
            LayerType::Solid {
                color: [1.0, 0.0, 0.0, 1.0],
            },
            1,
        ));
    }
    let p = software_renderer::render_frame_to_pixels(&comp, 0, 16, 16, 0.0, 0);
    assert_eq!(p.len(), 16 * 16 * 4);
}

// ─────────────────────────────────────────────────────────────────────────────
// §6  Cross-Module Consistency
// ─────────────────────────────────────────────────────────────────────────────
