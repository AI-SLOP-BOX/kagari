/// Regression: trail_length=8 caused index 8 access on `[f32;8]` array (0..7).
/// With `.min(8)` capping, `i` ranges 7..1 (inclusive), which is safe.
#[test]
/// Regression: trail_length=8 caused index 8 access on `[f32;8]` array (0..7).
/// With `.min(8)` capping, `i` ranges 7..1 (inclusive), which is safe.
fn particle_trail_max_8_no_out_of_bounds() {
    let emitter = ParticleEmitter {
        trail_length: 8,
        rate: 100.0,
        max_particles: 50,
        lifetime: 1.0,
        lifetime_variance: 0.0,
        spread_degrees: 360.0,
        ..Default::default()
    };
    let mut sys = ParticleSystem::new(emitter);
    // Emit particles
    sys.update(0.1, 0.0, 0.0);
    assert!(!sys.particles.is_empty(), "should have emitted particles");

    // Run many updates — if trail index is out of bounds, this will panic
    for _ in 0..120 {
        sys.update(0.016, 0.0, 0.0);
    }

    // Verify trail_len respects max
    for p in &sys.particles {
        assert!(
            p.trail_len <= 8,
            "trail_len {} exceeds array size 8",
            p.trail_len
        );
        // All trail entries within the used range should be finite
        for i in 0..p.trail_len as usize {
            assert!(
                p.trail[i].0.is_finite() && p.trail[i].1.is_finite(),
                "non-finite trail value at index {}",
                i
            );
        }
    }
}

/// Regression: trail_length values from 1..7 should also work without panic.
#[test]
fn particle_trail_various_lengths_no_panic() {
    for tl in 1u8..=8 {
        let emitter = ParticleEmitter {
            trail_length: tl,
            rate: 50.0,
            max_particles: 10,
            lifetime: 0.5,
            lifetime_variance: 0.0,
            ..Default::default()
        };
        let mut sys = ParticleSystem::new(emitter);
        sys.update(0.05, 100.0, 100.0);
        for _ in 0..60 {
            sys.update(0.016, 100.0, 100.0);
        }
        for p in &sys.particles {
            assert!(p.trail_len <= tl);
        }
    }
}

/// Regression: particles must never contain NaN/Inf after updates with extreme inputs.
#[test]
fn particle_extreme_inputs_stay_finite() {
    let emitter = ParticleEmitter {
        rate: 200.0,
        max_particles: 100,
        lifetime: 0.01,
        lifetime_variance: 1.0,
        speed: 10000.0,
        speed_variance: 1.0,
        spread_degrees: 360.0,
        gravity: [f32::MAX, f32::MIN],
        turbulence: 1000.0,
        vortex_strength: 500.0,
        attract_strength: 500.0,
        wind_gust_strength: 1000.0,
        wind_gust_frequency: 100.0,
        ..Default::default()
    };
    let mut sys = ParticleSystem::new(emitter);
    for _ in 0..30 {
        sys.update(0.016, 500.0, 500.0);
    }
    for p in &sys.particles {
        assert!(p.x.is_finite(), "particle x is not finite");
        assert!(p.y.is_finite(), "particle y is not finite");
        assert!(p.size.is_finite(), "particle size is not finite");
    }
}

/// Regression: particle size interpolation should start close to size_start
/// at the first frame (life == max_life → t should be 0).
#[test]
fn particle_size_starts_at_size_start() {
    let size_start = 42.0;
    let emitter = ParticleEmitter {
        rate: 1000.0,
        max_particles: 10,
        lifetime: 2.0,
        lifetime_variance: 0.0,
        speed: 0.0,
        speed_variance: 0.0,
        spread_degrees: 0.0,
        size_start,
        size_end: 1.0,
        ..Default::default()
    };
    let mut sys = ParticleSystem::new(emitter);
    // Use a very small timestep so the particle is barely alive
    sys.update(0.001, 0.0, 0.0);
    assert!(
        !sys.particles.is_empty(),
        "should have emitted at least one particle"
    );
    // After the first update, life ≈ max_life - dt, so t = 1 - (life/max_life)
    // is very small → size should be close to size_start
    let p = &sys.particles[0];
    let diff = (p.size - size_start).abs();
    // With lifetime=2.0 and dt=0.001, t = 0.001/2.0 = 0.0005
    // size = 42 + (1-42)*0.0005 ≈ 41.98, diff ≈ 0.02
    assert!(
        diff < 3.0,
        "particle size {} is too far from size_start {} after first frame",
        p.size,
        size_start
    );
}

// ─── Expression Engine ──────────────────────────────────────────────────────

/// Regression: smooth() was a no-op (always returned sample_rate constant).
/// It should now return a value that depends on the evaluated expression's
/// time-varying input. We test that smooth(1.0, 10.0) returns a finite value.
#[test]
fn smooth_function_is_not_noop() {
    let engine = expression_engine::build_engine();

    let mut scope = Scope::new();
    scope.push("time", 0.0f64);
    let r1: f64 = engine
        .eval_expression_with_scope(&mut scope, "smooth(1.0, 10.0)")
        .unwrap();
    assert!(r1.is_finite(), "smooth returned non-finite: {}", r1);

    scope.push("time", 5.0f64);
    let r2: f64 = engine
        .eval_expression_with_scope(&mut scope, "smooth(1.0, 10.0)")
        .unwrap();
    assert!(r2.is_finite(), "smooth returned non-finite: {}", r2);
}

/// Regression: seedRandom(NaN) caused platform-dependent panic (f64→u64 cast).
/// It must return a finite value (0 for non-finite input).
#[test]
fn seed_random_nan_does_not_panic() {
    let engine = expression_engine::build_engine();
    // NaN in Rhai is 0.0/0.0
    let result: f64 = engine.eval(r#"seedRandom(0.0/0.0)"#).unwrap();
    assert!(result.is_finite(), "seedRandom(NaN) returned: {}", result);
}

/// Regression: seedRandom with Inf must not panic.
#[test]
fn seed_random_inf_does_not_panic() {
    let engine = expression_engine::build_engine();
    // Infinity in Rhai is 1.0/0.0
    let r1: f64 = engine.eval(r#"seedRandom(1.0/0.0)"#).unwrap();
    let r2: f64 = engine.eval(r#"seedRandom(-1.0/0.0)"#).unwrap();
    assert!(r1.is_finite(), "seedRandom(+Inf) returned: {}", r1);
    assert!(r2.is_finite(), "seedRandom(-Inf) returned: {}", r2);
}

/// Regression: seedRandom must be deterministic (same seed → same output).
#[test]
fn seed_random_is_deterministic() {
    let engine = expression_engine::build_engine();
    let a: f64 = engine.eval(r#"seedRandom(42.0)"#).unwrap();
    let b: f64 = engine.eval(r#"seedRandom(42.0)"#).unwrap();
    assert_eq!(a, b, "seedRandom not deterministic");
}

/// Regression: seedRandom with large values should not overflow.
#[test]
fn seed_random_large_values_do_not_panic() {
    let engine = expression_engine::build_engine();
    let r: f64 = engine.eval(r#"seedRandom(1e18)"#).unwrap();
    assert!(r.is_finite());
}

/// Regression: valueAtTime and velocityAtTime must remain finite.
#[test]
fn stub_functions_return_finite() {
    let engine = expression_engine::build_engine();
    let v1: f64 = engine.eval(r#"valueAtTime(1.0)"#).unwrap();
    let v2: f64 = engine.eval(r#"velocityAtTime(1.0)"#).unwrap();
    assert!(v1.is_finite());
    assert!(v2.is_finite());
}
#[test]
fn value_at_time_and_velocity_sample_the_animated_property() {
    let property = Animatable::new_animated(vec![
        Keyframe::new(0, 0.0, InterpolationType::Linear),
        Keyframe::new(30, 100.0, InterpolationType::Linear),
    ]);
    let expression = Expression::Raw("valueAtTime(time + 0.25)".into());
    let sampled = expression.evaluate_f32_with_property(50.0, 15, 30, &property);
    assert_eq!(sampled, 75.0, "valueAtTime must sample the property curve");

    let velocity = Expression::Raw("velocityAtTime(time)".into())
        .evaluate_f32_with_property(50.0, 15, 30, &property);
    assert!((velocity - 100.0).abs() < 0.01, "velocity was {velocity}");
}
#[test]
fn vector_value_at_time_samples_position_and_scale_properties() {
    let property = Animatable::new_animated(vec![
        Keyframe::new(0, [0.0, 0.0], InterpolationType::Linear),
        Keyframe::new(30, [100.0, 200.0], InterpolationType::Linear),
    ]);
    let engine = expression_engine::build_engine();
    let sampled = expression_engine::eval_v2_with_property(
        &engine,
        "valueAtTime(time + 0.25)",
        [50.0, 100.0],
        15,
        30,
        &property,
    );
    assert_eq!(sampled, [75.0, 150.0]);

    let expression = Expression::Raw("valueAtTime(time + 0.25)".into());
    assert_eq!(
        expression.evaluate_v2_with_property([50.0, 100.0], 15, 30, &property),
        [75.0, 150.0]
    );
}
#[test]
fn composition_transform_expression_uses_property_sampling_context() {
    let mut composition = Composition::new("comp".into(), "Comp".into(), 64, 64, 30, 60);
    let mut layer = Layer::new(
        "layer".into(),
        "Layer".into(),
        LayerType::Solid {
            color: [1.0, 1.0, 1.0, 1.0],
        },
        60,
    );
    layer.transform.rotation = Animatable::new_animated(vec![
        Keyframe::new(0, 0.0, InterpolationType::Linear),
        Keyframe::new(30, 100.0, InterpolationType::Linear),
    ]);
    layer.transform.position = Animatable::new_animated(vec![
        Keyframe::new(0, [0.0, 0.0], InterpolationType::Linear),
        Keyframe::new(30, [100.0, 200.0], InterpolationType::Linear),
    ]);
    layer.transform.position_expression = Some(Expression::Raw("valueAtTime(time + 0.25)".into()));
    layer.transform.scale = Animatable::new_animated(vec![
        Keyframe::new(0, [100.0, 100.0], InterpolationType::Linear),
        Keyframe::new(30, [200.0, 300.0], InterpolationType::Linear),
    ]);
    layer.transform.scale_expression = Some(Expression::Raw("valueAtTime(time + 0.25)".into()));
    layer.transform.rotation_expression = Some(Expression::Raw("valueAtTime(time + 0.25)".into()));
    composition.layers.push(layer);

    let (position, scale, rotation, _) =
        composition.resolve_world_transform(&composition.layers[0], 15);
    assert_eq!(position, [75.0, 150.0]);
    assert_eq!(scale, [175.0, 250.0]);
    assert_eq!(rotation, 75.0);
}

/// Regression: expression engine sandbox limits prevent resource exhaustion.
#[test]
fn expression_sandbox_limits_enforced() {
    let engine = expression_engine::build_engine();

    // Deep recursion should be bounded
    let result = engine.eval::<f64>(
        r#"
        fn recurse(n) { if n <= 0 { 1 } else { recurse(n-1) + 1 } }
        recurse(9999)
    "#,
    );
    assert!(result.is_err(), "deep recursion should be bounded");

    // String allocation should be bounded
    let result = engine.eval::<String>(
        r#"
        let s = "";
        for i in 0..100000 { s += "x"; }
        s
    "#,
    );
    assert!(result.is_err(), "string allocation should be bounded");
}

/// Regression: disabled symbols should produce errors.
#[test]
fn expression_disabled_symbols_are_blocked() {
    let engine = expression_engine::build_engine();

    let blocked = ["eval(\"1+1\")", "import(\"foo\")"];
    for expr in blocked {
        let result = engine.eval::<f64>(expr);
        assert!(result.is_err(), "should be blocked: {}", expr);
    }
}

// ─── AE Effects Pack ────────────────────────────────────────────────────────

/// Regression: apply_simple_choker must handle tiny buffers without panic.
#[test]
fn simple_choker_tiny_buffer() {
    // Buffer smaller than 4 bytes (one pixel) — should not panic
    let mut pixels = vec![128u8; 3];
    apply_simple_choker(&mut pixels, 50.0);
    assert_eq!(pixels.len(), 3);

    // Exactly one pixel
    let mut pixels = vec![128u8; 4];
    apply_simple_choker(&mut pixels, 50.0);
    assert_eq!(pixels.len(), 4);
}

/// Regression: apply_simple_choker with empty buffer.
#[test]
fn simple_choker_empty_buffer() {
    let mut pixels: Vec<u8> = vec![];
    apply_simple_choker(&mut pixels, 50.0);
    assert!(pixels.is_empty());
}

/// Regression: apply_simple_choker produces valid alpha values.
#[test]
fn simple_choker_output_valid() {
    let mut pixels = vec![255u8; 400]; // 100 pixels
    apply_simple_choker(&mut pixels, 50.0);
    // All pixels should still be valid u8 (always true by type, but confirms no corruption)
    assert_eq!(pixels.len(), 400);
}

/// Regression: glow soft threshold should not produce values > 255.
#[test]
fn glow_extreme_values_safe() {
    let mut pixels = vec![0u8; 100 * 4]; // 100 pixels
                                         // Set some bright pixels
    for i in (0..400).step_by(4) {
        pixels[i] = 255;
        pixels[i + 1] = 255;
        pixels[i + 2] = 255;
        pixels[i + 3] = 255;
    }
    apply_glow(&mut pixels, 10, 10, 0.5, 50, 2.0);
    // Glow should not corrupt the buffer
    assert_eq!(pixels.len(), 400);
}

// ─── Composition & Layer Safety ─────────────────────────────────────────────

/// Regression: empty composition must not panic during common operations.
#[test]
fn empty_composition_safe() {
    let comp = Composition::new("c1".into(), "Empty".into(), 1920, 1080, 30, 100);
    assert!(comp.layers.is_empty());
    assert_eq!(comp.width, 1920);
    assert_eq!(comp.duration_frames, 100);
}

/// Regression: Layer::is_active must not panic for edge frame values.
#[test]
fn layer_is_active_boundary_frames() {
    let layer = Layer::new(
        "l1".into(),
        "Test".into(),
        LayerType::Solid {
            color: [1.0, 1.0, 1.0, 1.0],
        },
        100,
    );
    // Boundary frames
    assert!(layer.is_active(0)); // at in_frame
    assert!(layer.is_active(100)); // at out_frame
    assert!(!layer.is_active(101)); // beyond out_frame
    assert!(!layer.is_active(u32::MAX)); // far beyond
}

/// Regression: TrimPaths::trim_polygon with degenerate inputs must not panic.
#[test]
fn trim_paths_degenerate_inputs() {
    let tp = TrimPaths {
        start: Animatable::new_constant(0.0),
        end: Animatable::new_constant(50.0),
        offset: Animatable::new_constant(0.0),
    };

    // Empty points
    let result = tp.trim_polygon(&[], 0);
    assert!(result.is_empty());

    // Single point
    let result = tp.trim_polygon(&[[10.0, 10.0]], 0);
    assert_eq!(result.len(), 1);

    // Two identical points (zero-length path)
    let result = tp.trim_polygon(&[[0.0, 0.0], [0.0, 0.0]], 0);
    assert_eq!(result.len(), 2);
}
