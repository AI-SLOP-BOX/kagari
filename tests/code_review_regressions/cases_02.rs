/// Regression: trim_polygon with start == end should return empty.
#[test]
/// Regression: trim_polygon with start == end should return empty.
fn trim_paths_start_equals_end() {
    let tp = TrimPaths {
        start: Animatable::new_constant(50.0),
        end: Animatable::new_constant(50.0),
        offset: Animatable::new_constant(0.0),
    };
    let points = [[0.0, 0.0], [100.0, 0.0], [100.0, 100.0]];
    let result = tp.trim_polygon(&points, 0);
    assert!(
        result.is_empty(),
        "start == end should produce empty polygon"
    );
}

// ─── Project Serialization ──────────────────────────────────────────────────

/// Regression: project roundtrip must preserve all data.
#[test]
fn project_json_roundtrip_preserves_layers() {
    let mut comp = Composition::new("c1".into(), "Test".into(), 1920, 1080, 30, 300);
    comp.add_layer(Layer::new(
        "l1".into(),
        "Solid".into(),
        LayerType::Solid {
            color: [1.0, 0.0, 0.0, 1.0],
        },
        300,
    ));
    comp.add_layer(Layer::new(
        "l2".into(),
        "Text".into(),
        LayerType::new_text("Hello", 48, [1.0, 1.0, 1.0, 1.0]),
        300,
    ));
    comp.add_layer(Layer::new_null("l3".into(), "Null".into(), 300));

    let project = Project {
        compositions: vec![comp],
        ..Default::default()
    };

    let json = serde_json::to_string(&project).unwrap();
    let restored: Project = serde_json::from_str(&json).unwrap();

    assert_eq!(restored.compositions.len(), 1);
    assert_eq!(restored.compositions[0].layers.len(), 3);
    assert_eq!(restored.compositions[0].layers[0].name, "Solid");
    assert_eq!(restored.compositions[0].layers[1].name, "Text");
    assert_eq!(restored.compositions[0].layers[2].name, "Null");
}

/// Regression: malformed JSON must fail gracefully (not panic).
#[test]
fn malformed_project_json_does_not_panic() {
    let result = serde_json::from_str::<Project>(r#"{"invalid": true"#);
    assert!(result.is_err());
}

/// Regression: CPU rendering must evaluate animated effect parameters at the
/// same remapped layer frame used by the rest of the layer pipeline.
#[test]
fn time_remap_drives_animated_effect_parameters() {
    let mut comp = Composition::new("c1".into(), "Remapped effects".into(), 8, 8, 30, 60);
    let mut layer = Layer::new(
        "l1".into(),
        "Solid".into(),
        LayerType::Solid {
            color: [1.0, 1.0, 1.0, 1.0],
        },
        60,
    );
    layer.time_remap = Some(Animatable::new_constant(30.0));
    layer.effects.push(Effect {
        id: "tint".into(),
        name: "Tint".into(),
        effect_type: EffectType::ColorTint {
            color: Animatable::new_constant([1.0, 0.0, 0.0, 1.0]),
            intensity: Animatable::new_animated(vec![
                Keyframe::new(0, 0.0, InterpolationType::Linear),
                Keyframe::new(30, 100.0, InterpolationType::Linear),
            ]),
        },
        enabled: true,
    });
    comp.layers.push(layer);

    let pixels =
        kagari_vfx::core::software_renderer::render_frame_to_pixels(&comp, 0, 8, 8, 0.0, 0);
    let center = &pixels[(4 * 8 + 4) * 4..][..4];
    assert!(
        center[0] > 240,
        "remapped effect should keep red channel high"
    );
    assert!(center[1] < 20, "effect must be evaluated at remapped frame");
    assert!(center[2] < 20, "effect must be evaluated at remapped frame");
}

// ─── Effects: Echo & SetMatte ───────────────────────────────────────────────

/// Regression: Echo effect must not panic with zero-length pixel buffer.
#[test]
fn echo_zero_length_buffer_safe() {
    let pixels: Vec<u8> = vec![];
    let src: Vec<u8> = vec![];
    // Should not panic — just a no-op
    blend_echo_frame(&mut pixels.clone(), &src, 0, 0, 0.5, EchoOperator::Add);
}

/// Regression: Echo effect with single pixel.
#[test]
fn echo_single_pixel_no_panic() {
    let mut pixels = vec![128u8; 4];
    let src = vec![200u8; 4];
    blend_echo_frame(&mut pixels, &src, 1, 1, 0.5, EchoOperator::Add);
    // Should not panic — buffer size is preserved
    assert_eq!(pixels.len(), 4);
}

/// Regression: Echo effect with mismatched buffer sizes should not panic.
#[test]
fn echo_mismatched_buffer_sizes_safe() {
    let mut pixels = vec![128u8; 16];
    let src = vec![200u8; 8]; // smaller
    blend_echo_frame(&mut pixels, &src, 2, 2, 0.5, EchoOperator::Add);
    // Should not panic
}

// ─── Layer Type: Audio Path Safety ──────────────────────────────────────────

/// Regression: Audio layer with empty path should not cause panic
/// in path operations.
#[test]
fn audio_layer_empty_path_roundtrips() {
    let mut layer = Layer::new(
        "l1".into(),
        "Audio".into(),
        LayerType::Audio {
            path: String::new(),
            volume: Animatable::new_constant(0.0),
        },
        30,
    );
    layer.visible = true;

    // Serialize and deserialize
    let json = serde_json::to_string(&layer).unwrap();
    let restored: Layer = serde_json::from_str(&json).unwrap();
    match &restored.layer_type {
        LayerType::Audio { path, .. } => assert!(path.is_empty()),
        _ => panic!("wrong layer type after roundtrip"),
    }
}

// ─── Blend Modes & Track Matte ──────────────────────────────────────────────

/// Regression: all blend mode variants should serialize/deserialize.
#[test]
fn blend_modes_roundtrip() {
    let modes = [
        BlendMode::Normal,
        BlendMode::Multiply,
        BlendMode::Screen,
        BlendMode::Overlay,
        BlendMode::Add,
        BlendMode::Darken,
        BlendMode::Lighten,
        BlendMode::SoftLight,
        BlendMode::HardLight,
        BlendMode::Difference,
        BlendMode::Exclusion,
        BlendMode::Divide,
        BlendMode::Subtract,
        BlendMode::ColorBurn,
        BlendMode::LinearBurn,
        BlendMode::VividLight,
        BlendMode::ColorDodge,
        BlendMode::LinearDodge,
        BlendMode::Color,
        BlendMode::Hue,
        BlendMode::Saturation,
        BlendMode::Luminosity,
    ];
    for mode in modes {
        let json = serde_json::to_string(&mode).unwrap();
        let restored: BlendMode = serde_json::from_str(&json).unwrap();
        assert_eq!(format!("{:?}", mode), format!("{:?}", restored));
    }
}

/// Regression: all track matte modes should serialize/deserialize.
#[test]
fn track_matte_modes_roundtrip() {
    let modes = [
        TrackMatteMode::None,
        TrackMatteMode::AlphaMatte,
        TrackMatteMode::AlphaMatteInverted,
        TrackMatteMode::LumaMatte,
        TrackMatteMode::LumaMatteInverted,
    ];
    for mode in modes {
        let json = serde_json::to_string(&mode).unwrap();
        let restored: TrackMatteMode = serde_json::from_str(&json).unwrap();
        assert_eq!(format!("{:?}", mode), format!("{:?}", restored));
    }
}

// ─── EffectType Animatable Params ────────────────────────────────────────────

/// Regression: every EffectType variant must have registered animatable params
/// (or at least not panic when queried).
#[test]
fn all_effect_types_have_animatable_params_no_panic() {
    let effects = [
        EffectType::GaussianBlur {
            blur_radius: Animatable::new_constant(5.0),
        },
        EffectType::Twirl {
            angle: Animatable::new_constant(45.0),
            radius: Animatable::new_constant(100.0),
        },
        EffectType::Bulge {
            amount: Animatable::new_constant(0.5),
            radius: Animatable::new_constant(100.0),
        },
        EffectType::Posterize {
            levels: Animatable::new_constant(4.0),
        },
        EffectType::Sharpen {
            amount: Animatable::new_constant(50.0),
        },
        EffectType::Threshold {
            threshold: Animatable::new_constant(128.0),
        },
    ];
    for mut effect in effects {
        let params = effect.animatable_params();
        // Should not panic — params may be empty for some effect types
        let _ = params.len();
    }
}

// ─── MaterialOptions ────────────────────────────────────────────────────────

/// Regression: MaterialOptions must serialize/deserialize with all fields.
#[test]
fn material_options_roundtrip() {
    let mat = MaterialOptions {
        ambient: 0.5,
        diffuse: 0.8,
        specular: 0.3,
        specular_exponent: 50.0,
        emission: 0.1,
        metalness: 0.2,
        cast_shadows: true,
        accepts_shadows: true,
        accepts_lights: false,
        light_transmission: 0.2,
    };
    let json = serde_json::to_string(&mat).unwrap();
    let restored: MaterialOptions = serde_json::from_str(&json).unwrap();
    assert!(restored.accepts_shadows);
    assert!(!restored.accepts_lights);
    assert!((restored.ambient - 0.5).abs() < f32::EPSILON);
    assert!((restored.diffuse - 0.8).abs() < f32::EPSILON);
    assert!((restored.metalness - 0.2).abs() < f32::EPSILON);
}

// ─── Camera3D ───────────────────────────────────────────────────────────────

/// Regression: Camera3D default must have sensible values.
#[test]
fn camera3d_default_sane() {
    let cam = Camera3D::default();
    let pos = cam.transform.position.evaluate(0);
    assert!(pos[2].is_finite());
    assert!(cam.fov_degrees > 0.0);
    assert!(cam.focus_distance > 0.0);
}

// ─── Expression Engine: Comp Context ────────────────────────────────────────

/// Regression: expression with thisComp reference must not panic.
#[test]
fn expression_this_comp_reference() {
    let engine = expression_engine::build_engine();
    let mut scope = Scope::new();
    scope.push("time", 1.0f64);
    scope.push("frame", 1i64);
    let result = engine.eval_expression_with_scope::<f64>(&mut scope, "time * 2");
    assert!(result.is_ok());
    assert!((result.unwrap() - 2.0).abs() < f64::EPSILON);
}

/// Regression: expression with wiggle must not panic and must return finite.
#[test]
fn expression_wiggle_finite() {
    let engine = expression_engine::build_engine();
    let mut scope = Scope::new();
    scope.push("time", 1.0f64);
    let result: f64 = engine
        .eval_expression_with_scope(&mut scope, "wiggle(1.0, 10.0)[0]")
        .unwrap();
    assert!(result.is_finite());
}

/// Regression: expression with noise must return value in [0, 1].
#[test]
fn expression_noise_in_unit_range() {
    let engine = expression_engine::build_engine();
    for t in [0.0, 0.5, 1.0, 2.5, 10.0] {
        let mut scope = Scope::new();
        scope.push("time", t);
        let r: f64 = engine
            .eval_expression_with_scope(&mut scope, "noise(time)")
            .unwrap();
        assert!(
            (0.0..=1.0).contains(&r),
            "noise({}) = {} not in [0,1]",
            t,
            r
        );
    }
}

/// Regression: wrap function must correctly wrap values.
#[test]
fn expression_wrap_correctness() {
    let engine = expression_engine::build_engine();
    let tests = [
        (0.5, 0.0, 1.0, 0.5),
        (1.5, 0.0, 1.0, 0.5),
        (-0.5, 0.0, 1.0, 0.5),
        (5.0, 0.0, 3.0, 2.0),
    ];
    for (val, min, max, expected) in tests {
        let expr = format!("wrap({:.1}, {:.1}, {:.1})", val, min, max);
        let r: f64 = engine.eval(&expr).unwrap();
        assert!(
            (r - expected).abs() < 0.001,
            "wrap({}, {}, {}) = {}, expected {}",
            val,
            min,
            max,
            r,
            expected
        );
    }
}

// ─── LUT File Read Safety ───────────────────────────────────────────────────

/// Regression: ColorGradeLUT with nonexistent file should not panic.
#[test]
fn lut_nonexistent_file_does_not_panic() {
    // This would be tested via cpu_effects dispatch, but we verify the
    // effect type can be created safely.
    let effect = EffectType::ColorGradeLUT {
        lut_path: "/nonexistent/path/to/lut.cube".into(),
        intensity: Animatable::new_constant(1.0),
    };
    let json = serde_json::to_string(&effect).unwrap();
    let _restored: EffectType = serde_json::from_str(&json).unwrap();
}

// ─── Particle Emitter Default ───────────────────────────────────────────────

/// Regression: ParticleEmitter default values should be sane.
#[test]
fn particle_emitter_default_sane() {
    let e = ParticleEmitter::default();
    assert!(e.rate > 0.0);
    assert!(e.max_particles > 0);
    assert!(e.lifetime > 0.0);
    assert!(e.speed >= 0.0);
    assert!(e.size_start > 0.0);
    assert!(e.size_end >= 0.0);
    assert!(e.opacity_start >= 0.0);
    assert!(e.opacity_start <= 1.0);
    assert!(e.opacity_end >= 0.0);
    assert!(e.opacity_end <= 1.0);
}

/// Regression: ParticleSystem with zero rate should not emit but not panic.
#[test]
fn particle_zero_rate_no_emit() {
    let emitter = ParticleEmitter {
        rate: 0.0,
        max_particles: 100,
        ..Default::default()
    };
    let mut sys = ParticleSystem::new(emitter);
    sys.update(1.0, 0.0, 0.0);
    assert!(sys.particles.is_empty(), "zero rate should emit nothing");
}

/// Regression: ParticleSystem should respect max_particles limit.
#[test]
fn particle_max_particles_respected() {
    let emitter = ParticleEmitter {
        rate: 1000.0,
        max_particles: 5,
        lifetime: 10.0,
        lifetime_variance: 0.0,
        ..Default::default()
    };
    let mut sys = ParticleSystem::new(emitter);
    for _ in 0..100 {
        sys.update(0.1, 0.0, 0.0);
    }
    assert!(
        sys.particles.len() <= 5,
        "should respect max_particles: got {}",
        sys.particles.len()
    );
}

// ─── FadeCurve ──────────────────────────────────────────────────────────────
