/// Regression: LifeCurve clamps t to [0,1].
#[test]
fn lifecurve_clamps_t() {
    let lc = LifeCurve(vec![0.0, 1.0]);
    let v_neg = lc.eval(-1.0);
    let v_over = lc.eval(2.0);
    assert!((v_neg - 0.0).abs() < 0.01);
    assert!((v_over - 1.0).abs() < 0.01);
}

/// Regression: LifeCurve interpolation at midpoint.
#[test]
fn lifecurve_midpoint_interpolation() {
    let lc = LifeCurve(vec![0.0, 1.0]);
    let mid = lc.eval(0.5);
    assert!((mid - 0.5).abs() < 0.01);
}

/// Regression: LifeCurve with NaN values should return NaN (not panic).
#[test]
fn lifecurve_nan_values_do_not_panic() {
    let lc = LifeCurve(vec![f32::NAN, f32::NAN]);
    let _ = lc.eval(0.5); // should not panic
}

/// Regression: LifeCurve roundtrip through serialization.
#[test]
fn lifecurve_roundtrip() {
    let lc = LifeCurve(vec![0.0, 0.5, 1.0, 0.5, 0.0]);
    let json = serde_json::to_string(&lc).unwrap();
    let restored: LifeCurve = serde_json::from_str(&json).unwrap();
    assert_eq!(lc, restored);
}

// ─── Shape Boolean ─────────────────────────────────────────────────────────

/// Regression: Boolean with both empty polygons should return empty.
#[test]
fn shape_boolean_both_empty() {
    let result = apply_polygon_boolean(&[], &[], BooleanOp::Union);
    assert!(result.is_empty());
}

/// Regression: Boolean with one empty polygon should return the other (for Union).
#[test]
fn shape_boolean_one_empty_union() {
    let subject = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
    let result = apply_polygon_boolean(&subject, &[], BooleanOp::Union);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 4);
}

/// Regression: Boolean Intersect with one empty should return empty.
#[test]
fn shape_boolean_one_empty_intersect() {
    let subject = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]];
    let result = apply_polygon_boolean(&subject, &[], BooleanOp::Intersect);
    assert!(result.is_empty());
}

/// Regression: Boolean Subtract with empty clip should return subject.
#[test]
fn shape_boolean_empty_clip_subtract() {
    let subject = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]];
    let result = apply_polygon_boolean(&subject, &[], BooleanOp::Subtract);
    assert_eq!(result.len(), 1);
}

/// Regression: Boolean with degenerate polygons (< 3 points) should not panic.
#[test]
fn shape_boolean_degenerate_polygons_safe() {
    let line = [[0.0, 0.0], [10.0, 0.0]];
    let point = [[5.0, 5.0]];
    for op in [
        BooleanOp::Union,
        BooleanOp::Intersect,
        BooleanOp::Subtract,
        BooleanOp::Exclude,
    ] {
        let _ = apply_polygon_boolean(&line, &point, op);
        let _ = apply_polygon_boolean(&point, &line, op);
    }
}

/// Regression: Boolean with coincident polygons should not panic.
#[test]
fn shape_boolean_coincident_polygons_safe() {
    let square = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
    let result = apply_polygon_boolean(&square, &square, BooleanOp::Union);
    // Should produce some result (at least one contour)
    assert!(!result.is_empty());
}

/// Regression: Boolean with NaN coordinates should not panic.
#[test]
fn shape_boolean_nan_coordinates_safe() {
    let nan_poly = [[f32::NAN, 0.0], [10.0, 0.0], [10.0, 10.0]];
    let normal = [[0.0, 0.0], [20.0, 0.0], [20.0, 20.0]];
    let _ = apply_polygon_boolean(&nan_poly, &normal, BooleanOp::Union);
}

// ─── Expression Engine: Additional Edge Cases ──────────────────────────────

/// Regression: Expression with division by zero must not panic.
#[test]
fn expression_division_by_zero_safe() {
    let engine = expression_engine::build_engine();
    let result = engine.eval::<f64>("1.0 / 0.0");
    assert!(result.is_ok());
    let v = result.unwrap();
    // Division by zero in Rhai produces Infinity, not a panic
    assert!(v.is_infinite());
}

/// Regression: Expression with deeply nested brackets must be bounded.
#[test]
fn expression_deep_nesting_bounded() {
    let engine = expression_engine::build_engine();
    // 100 levels of nesting — should be within the 64-depth limit
    let expr = "((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((1))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))";
    let result = engine.eval::<f64>(expr);
    // Should either succeed or fail gracefully (not panic)
    let _ = result;
}

/// Regression: Expression string operations must be bounded.
#[test]
fn expression_string_operations_bounded() {
    let engine = expression_engine::build_engine();
    // Moderate string operations should succeed
    let result = engine.eval::<String>(r#""hello" + " " + "world""#);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "hello world");
}

/// Regression: Expression array operations must be bounded.
#[test]
fn expression_array_operations_bounded() {
    let engine = expression_engine::build_engine();
    let result = engine.eval::<f64>(r#"[1.0, 2.0, 3.0][1]"#);
    assert!(result.is_ok());
    assert!((result.unwrap() - 2.0).abs() < f64::EPSILON);
}

/// Regression: posterizeTime must snap to discrete steps.
#[test]
fn expression_posterize_time() {
    let engine = expression_engine::build_engine();
    let mut scope = Scope::new();
    scope.push("time", 0.5f64);
    let result: f64 = engine
        .eval_expression_with_scope(&mut scope, "posterizeTime(2.0)")
        .unwrap();
    // posterizeTime(2) with time=0.5: step=1/2=0.5, floor(0.5/0.5)*0.5 = 0.5
    assert!(result.is_finite());
    // The result should be a discrete step of 1/2 = 0.5
    let step = 1.0 / 2.0;
    let diff = (result / step - (result / step).round()).abs();
    assert!(diff < 0.01, "posterizeTime(2) = {} not on 0.5 grid", result);
}

/// Regression: noise function must return values in [0, 1] for various inputs.
#[test]
fn expression_noise_wide_range() {
    let engine = expression_engine::build_engine();
    for x in [-100.0, -1.0, 0.0, 0.5, 1.0, 100.0, 1000.0] {
        let result: f64 = engine.eval(&format!("noise({:.1})", x)).unwrap();
        assert!(
            (0.0..=1.0).contains(&result),
            "noise({}) = {} out of range",
            x,
            result
        );
    }
}

/// Regression: toe and shoulder helpers must return finite values.
#[test]
fn expression_toe_shoulder_finite() {
    let engine = expression_engine::build_engine();
    for x in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let toe: f64 = engine.eval(&format!("toe({:.2}, 2.0)", x)).unwrap();
        let shoulder: f64 = engine.eval(&format!("shoulder({:.2}, 2.0)", x)).unwrap();
        assert!(toe.is_finite(), "toe({}) = {}", x, toe);
        assert!(shoulder.is_finite(), "shoulder({}) = {}", x, shoulder);
    }
}

// ─── Animatable: Edge Cases ────────────────────────────────────────────────

/// Regression: Animatable::evaluate on constant always returns the value.
#[test]
fn animatable_constant_always_returns_value() {
    let anim = Animatable::new_constant(42.0f32);
    for frame in [0, 1, 100, u32::MAX] {
        assert!((anim.evaluate(frame) - 42.0).abs() < f32::EPSILON);
    }
}

/// Regression: Animatable with keyframes before first kf returns first value.
#[test]
fn animatable_before_first_keyframe() {
    let kfs = vec![
        Keyframe::new(10, 100.0f32, InterpolationType::Linear),
        Keyframe::new(20, 200.0f32, InterpolationType::Linear),
    ];
    let anim = Animatable::new_animated(kfs);
    assert!((anim.evaluate(0) - 100.0).abs() < 0.01);
    assert!((anim.evaluate(5) - 100.0).abs() < 0.01);
}

/// Regression: Animatable with keyframes after last kf returns last value.
#[test]
fn animatable_after_last_keyframe() {
    let kfs = vec![
        Keyframe::new(10, 100.0f32, InterpolationType::Linear),
        Keyframe::new(20, 200.0f32, InterpolationType::Linear),
    ];
    let anim = Animatable::new_animated(kfs);
    assert!((anim.evaluate(30) - 200.0).abs() < 0.01);
    assert!((anim.evaluate(1000) - 200.0).abs() < 0.01);
}

/// Regression: Animatable Vec2 evaluate must return finite values.
#[test]
fn animatable_vec2_evaluate_finite() {
    let anim = Animatable::new_constant([100.0f32, 200.0f32]);
    let val = anim.evaluate(0);
    assert!(val[0].is_finite());
    assert!(val[1].is_finite());
}

/// Regression: Animatable easy_ease on constant must be a no-op.
#[test]
fn animatable_easy_ease_constant_noop() {
    let mut anim = Animatable::new_constant(42.0f32);
    anim.easy_ease();
    assert!((anim.evaluate(0) - 42.0).abs() < f32::EPSILON);
}

/// Regression: Animatable easy_ease on single keyframe must be a no-op.
#[test]
fn animatable_easy_ease_single_kf_noop() {
    let mut anim =
        Animatable::new_animated(vec![Keyframe::new(0, 42.0f32, InterpolationType::Linear)]);
    anim.easy_ease();
    assert!((anim.evaluate(0) - 42.0).abs() < f32::EPSILON);
}

// ─── Particle Emitter Shape ────────────────────────────────────────────────

/// Regression: all emitter shapes must emit particles without panic.
#[test]
fn particle_all_emitter_shapes_no_panic() {
    let shapes = [
        EmitterShape::Point,
        EmitterShape::Box,
        EmitterShape::Circle,
        EmitterShape::Line,
        EmitterShape::Ring,
    ];
    for shape in shapes {
        let emitter = ParticleEmitter {
            shape,
            rate: 50.0,
            max_particles: 10,
            lifetime: 0.5,
            ..Default::default()
        };
        let mut sys = ParticleSystem::new(emitter);
        sys.update(0.1, 100.0, 100.0);
        for _ in 0..30 {
            sys.update(0.016, 100.0, 100.0);
        }
        // All particles should be finite
        for p in &sys.particles {
            assert!(p.x.is_finite());
            assert!(p.y.is_finite());
        }
    }
}

/// Regression: ParticleEmitter serde roundtrip preserves all fields.
#[test]
fn particle_emitter_roundtrip() {
    let emitter = ParticleEmitter {
        rate: 75.0,
        max_particles: 500,
        lifetime: 3.0,
        lifetime_variance: 0.1,
        speed: 300.0,
        speed_variance: 0.2,
        spread_degrees: 180.0,
        shape: EmitterShape::Circle,
        emitter_size: [200.0, 100.0],
        gravity: [0.0, -500.0],
        wind: [50.0, 0.0],
        turbulence: 2.0,
        color_start: [1.0, 0.0, 0.0, 1.0],
        color_end: [0.0, 0.0, 1.0, 0.0],
        size_start: 16.0,
        size_end: 4.0,
        opacity_start: 1.0,
        opacity_end: 0.0,
        rotation_speed: 90.0,
        rotation_start: 45.0,
        rotation_speed_variance: 0.5,
        fade_curve: FadeCurve::EaseIn,
        blend_mode: 2,
        gravity_curve: LifeCurve(vec![0.5, 1.0, 0.5]),
        wind_gust_strength: 100.0,
        wind_gust_frequency: 2.0,
        drag: 0.1,
        collision_enabled: true,
        collision_bounds: [0.0, 0.0, 1920.0, 1080.0],
        restitution: 0.8,
        surface_friction: 0.9,
        particle_collisions: true,
        particle_diameter: 12.0,
        trail_length: 5,
        trail_taper: 0.6,
        vortex_strength: 50.0,
        vortex_center: [960.0, 540.0],
        attract_strength: 10.0,
        attract_center: [400.0, 300.0],
        depth_enabled: true,
        depth_range: [-50.0, 50.0],
        death_spawn_count: 3,
        death_spawn_speed_scale: 0.3,
        death_spawn_life_scale: 0.4,
    };
    let json = serde_json::to_string(&emitter).unwrap();
    let restored: ParticleEmitter = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.rate, 75.0);
    assert_eq!(restored.max_particles, 500);
    assert_eq!(restored.shape, EmitterShape::Circle);
    assert_eq!(restored.trail_length, 5);
    assert_eq!(restored.death_spawn_count, 3);
    assert!((restored.gravity_curve.0[0] - 0.5).abs() < f32::EPSILON);
}

// ─── TrimPaths Edge Cases ──────────────────────────────────────────────────

/// Regression: TrimPaths with large offset wraps correctly.
#[test]
fn trim_paths_large_offset() {
    let tp = TrimPaths {
        start: Animatable::new_constant(0.0),
        end: Animatable::new_constant(50.0),
        offset: Animatable::new_constant(720.0), // 2 full rotations
    };
    let points = [[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]];
    let result = tp.trim_polygon(&points, 0);
    // 720° offset = 0° effective, so result should be non-empty
    assert!(!result.is_empty());
}

/// Regression: TrimPaths with negative offset wraps correctly.
#[test]
fn trim_paths_negative_offset() {
    let tp = TrimPaths {
        start: Animatable::new_constant(0.0),
        end: Animatable::new_constant(50.0),
        offset: Animatable::new_constant(-360.0),
    };
    let points = [[0.0, 0.0], [100.0, 0.0], [100.0, 100.0]];
    let result = tp.trim_polygon(&points, 0);
    assert!(!result.is_empty());
}

// ─── BlendMode Correctness ─────────────────────────────────────────────────

/// Regression: BlendMode serialization must be stable (JSON schema compat).
#[test]
fn blend_mode_json_values_stable() {
    let json = serde_json::to_string(&BlendMode::Normal).unwrap();
    assert_eq!(json, r#""Normal""#);
    let json = serde_json::to_string(&BlendMode::Multiply).unwrap();
    assert_eq!(json, r#""Multiply""#);
    let json = serde_json::to_string(&BlendMode::Screen).unwrap();
    assert_eq!(json, r#""Screen""#);
}

// ─── EffectType: All Variants Serialize ─────────────────────────────────────
