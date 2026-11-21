/// Regression: each EchoOperator must produce valid output without panic.
#[test]
fn echo_all_operators_no_panic() {
    let ops = [
        EchoOperator::Add,
        EchoOperator::Screen,
        EchoOperator::Maximum,
        EchoOperator::Minimum,
        EchoOperator::CompositeInBack,
        EchoOperator::CompositeInFront,
        EchoOperator::Blend,
    ];
    for op in ops {
        let mut acc = vec![128u8; 16]; // 2x2
        let echo = vec![200u8; 16];
        blend_echo_frame(&mut acc, &echo, 2, 2, 0.5, op);
        assert_eq!(acc.len(), 16);
    }
}

/// Regression: Echo with NaN weight should clamp to 0 (no panic, no NaN in output).
#[test]
fn echo_nan_weight_clamps_to_zero() {
    let mut acc = vec![128u8; 16];
    let echo = vec![200u8; 16];
    blend_echo_frame(&mut acc, &echo, 2, 2, f32::NAN, EchoOperator::Add);
    // NaN weight → w=0 → echo contribution is 0 → output unchanged
    assert_eq!(acc, vec![128u8; 16]);
}

/// Regression: Echo with Inf weight should clamp to 2.0 (not infinity).
#[test]
fn echo_inf_weight_clamped() {
    let mut acc = vec![128u8; 4];
    let echo = vec![128u8; 4];
    let original = acc.clone();
    blend_echo_frame(&mut acc, &echo, 1, 1, f32::INFINITY, EchoOperator::Add);
    // Should not panic and buffer should be modified (or at least not corrupted)
    assert_eq!(acc.len(), 4);
    // With Inf weight clamped to 2.0 and Add mode: output should be saturated
    assert!(acc[0] >= original[0]);
}

/// Regression: Echo with zero dimensions should be a no-op.
#[test]
fn echo_zero_dimensions_safe() {
    let mut acc = vec![128u8; 16];
    let echo = vec![200u8; 16];
    blend_echo_frame(&mut acc, &echo, 0, 0, 0.5, EchoOperator::Add);
    assert_eq!(acc, vec![128u8; 16]);
}

/// Regression: Echo with width*height overflow should be safe.
#[test]
fn echo_overflow_dimensions_safe() {
    let mut acc = vec![128u8; 4];
    let echo = vec![200u8; 4];
    // u32::MAX * u32::MAX would overflow; the checked_mul guard should handle it
    blend_echo_frame(&mut acc, &echo, u32::MAX, u32::MAX, 0.5, EchoOperator::Add);
}

// ─── SetMatte: Edge Cases ──────────────────────────────────────────────────

/// Regression: SetMatte with mismatched buffer sizes should not panic.
#[test]
fn set_matte_mismatched_sizes_safe() {
    let mut target = vec![255u8; 16]; // 2x2
    let source = vec![128u8; 4]; // 1x1 (smaller)
    let params = SetMatteParams::default();
    apply_set_matte(&mut target, 2, 2, &source, 1, 1, &params);
    // Should not panic — sizes don't match but function returns gracefully
}

/// Regression: SetMatte with empty buffers should not panic.
#[test]
fn set_matte_empty_buffers_safe() {
    let mut target: Vec<u8> = vec![];
    let source: Vec<u8> = vec![];
    let params = SetMatteParams::default();
    apply_set_matte(&mut target, 0, 0, &source, 0, 0, &params);
}

/// Regression: SetMatte with zero width/height should be a no-op.
#[test]
fn set_matte_zero_dimensions_safe() {
    let mut target = vec![255u8; 16];
    let source = vec![128u8; 16];
    let params = SetMatteParams::default();
    apply_set_matte(&mut target, 0, 4, &source, 0, 4, &params);
    assert_eq!(target, vec![255u8; 16]);
}

/// Regression: SetMatte all source channels must not panic.
#[test]
fn set_matte_all_channels_no_panic() {
    let channels = [
        MatteSourceChannel::Alpha,
        MatteSourceChannel::Luminance,
        MatteSourceChannel::Red,
        MatteSourceChannel::Green,
        MatteSourceChannel::Blue,
        MatteSourceChannel::Lightness,
    ];
    for ch in channels {
        let mut target = vec![255u8; 16];
        let source = vec![128u8; 16];
        let params = SetMatteParams {
            source_channel: ch,
            ..Default::default()
        };
        apply_set_matte(&mut target, 2, 2, &source, 2, 2, &params);
    }
}

/// Regression: SetMatte all composite modes must not panic.
#[test]
fn set_matte_all_composite_modes_no_panic() {
    let modes = [
        MatteCompositeMode::Replace,
        MatteCompositeMode::Intersect,
        MatteCompositeMode::Add,
        MatteCompositeMode::Subtract,
    ];
    for mode in modes {
        let mut target = vec![255u8; 16];
        let source = vec![128u8; 16];
        let params = SetMatteParams {
            composite_mode: mode,
            ..Default::default()
        };
        apply_set_matte(&mut target, 2, 2, &source, 2, 2, &params);
    }
}

/// Regression: SetMatte with invert_matte must produce inverted alpha.
#[test]
fn set_matte_invert_works() {
    // Source is fully opaque (alpha=255), target is fully transparent (alpha=0)
    // With invert: extracted_alpha = 1.0 - 1.0 = 0.0, so target alpha stays 0
    let mut target = vec![0u8; 4]; // RGBA: [0,0,0,0]
    target[3] = 0;
    let mut source = vec![0u8; 4];
    source[3] = 255; // fully opaque alpha
    let params = SetMatteParams {
        invert_matte: true,
        composite_mode: MatteCompositeMode::Replace,
        ..Default::default()
    };
    apply_set_matte(&mut target, 1, 1, &source, 1, 1, &params);
    // Inverted alpha from opaque source = 0, so target alpha should be 0
    assert_eq!(target[3], 0);
}

/// Regression: SetMatte roundtrip through serialization.
#[test]
fn set_matte_params_roundtrip() {
    let params = SetMatteParams {
        source_layer_idx: 3,
        source_channel: MatteSourceChannel::Luminance,
        invert_matte: true,
        composite_mode: MatteCompositeMode::Intersect,
    };
    let json = serde_json::to_string(&params).unwrap();
    let restored: SetMatteParams = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.source_layer_idx, 3);
    assert_eq!(restored.source_channel, MatteSourceChannel::Luminance);
    assert!(restored.invert_matte);
    assert_eq!(restored.composite_mode, MatteCompositeMode::Intersect);
}

// ─── History: NaN Deduplication ─────────────────────────────────────────────

/// Regression: History dedup should handle projects with NaN values.
/// NaN != NaN in JSON, so the dedup check may not catch no-op commits.
/// The history should still function correctly (not corrupt state).
#[test]
fn history_nan_dedup_does_not_corrupt_state() {
    use kagari_vfx::core::history::ProjectHistory;

    let initial = Project::default();
    let mut history = ProjectHistory::new(initial);

    // Create a project with a NaN value in a property
    let mut comp = Composition::new("c1".into(), "Test".into(), 100, 100, 30, 30);
    let mut layer = Layer::new(
        "l1".into(),
        "Solid".into(),
        LayerType::Solid {
            color: [1.0, 1.0, 1.0, 1.0],
        },
        30,
    );
    // Position with NaN — this is a degraded state but should not crash history
    layer.transform.position = Animatable::new_constant([f32::NAN, f32::NAN]);
    comp.add_layer(layer);

    let project = Project {
        compositions: vec![comp],
        ..Default::default()
    };

    history.commit_action(project.clone(), "add nan layer");
    assert_eq!(history.current().compositions[0].layers.len(), 1);

    // Commit same project — dedup may or may not catch it (NaN != NaN)
    // but should not corrupt the history
    history.commit_action(project, "duplicate nan");
    // History should have at least 1 entry
    assert!(!history.is_empty());
}

// ─── History: Byte Budget ──────────────────────────────────────────────────

/// Regression: History byte budget must actually trim oldest entries and the
/// saturating accounting must never underflow, even with mixed-size projects.
#[test]
fn history_byte_budget_no_underflow() {
    use kagari_vfx::core::history::ProjectHistory;
    use kagari_vfx::core::keyframe::{InterpolationType, Keyframe};
    use kagari_vfx::core::property::Animatable;

    fn huge_project(tag: usize) -> Project {
        let mut comp = Composition::new(format!("c{}", tag), "Test".into(), 100, 100, 30, 30);
        let payload = "x".repeat(700_000);
        // Serialized snapshots include these distinct layer names, keeping the
        // fixture above the 128MiB history budget with a realistic byte estimate.
        for j in 0..10 {
            let mut layer = Layer::new(
                format!("l{}_{}", tag, j),
                format!("Layer {}{}", j, payload),
                LayerType::Solid {
                    color: [1.0, 0.0, 0.0, 1.0],
                },
                30,
            );
            let kfs2: Vec<Keyframe<[f32; 2]>> = (0..1000)
                .map(|k| Keyframe::new(k, [k as f32, 0.0], InterpolationType::Linear))
                .collect();
            let kfs1: Vec<Keyframe<f32>> = (0..1000)
                .map(|k| Keyframe::new(k, k as f32, InterpolationType::Linear))
                .collect();
            layer.transform.position = Animatable::new_animated(kfs2.clone());
            layer.transform.scale = Animatable::new_animated(kfs2);
            layer.transform.rotation = Animatable::new_animated(kfs1.clone());
            layer.transform.opacity = Animatable::new_animated(kfs1);
            comp.add_layer(layer);
        }
        Project {
            compositions: vec![comp],
            ..Default::default()
        }
    }

    let mut history = ProjectHistory::new(Project::default());

    for i in 0..20 {
        history.commit_action(huge_project(i), &format!("edit {}", i));
    }

    assert!(
        history.approx_bytes() <= ProjectHistory::MAX_HISTORY_BYTES,
        "budget must be enforced, got {} bytes",
        history.approx_bytes()
    );
    assert!(
        history.len() < 21,
        "oldest entries must be evicted, kept {}",
        history.len()
    );
    assert_eq!(history.current_action_name(), "edit 19");
    // Eviction must not corrupt undo: oldest surviving entry is still reachable.
    assert!(history.can_undo());
    let _ = history.undo();
    assert!(!history.current().compositions.is_empty());
}

// ─── Keyframe Interpolation ────────────────────────────────────────────────

/// Regression: Bezier solver with extreme control points must not produce NaN.
#[test]
fn bezier_solver_extreme_values_no_nan() {
    use kagari_vfx::core::keyframe::solve_bezier_eased_time;

    let extreme_cases = [
        (0.0, 0.0, 0.0, 1.0, 1.0),   // linear
        (0.5, 0.25, 0.1, 0.25, 1.0), // standard ease
        (0.5, 0.5, 0.2, 0.6, 0.8),   // elastic
        (0.5, 0.5, 0.3, 0.7, 0.1),   // bounce
        (0.0, 0.0, 0.0, 0.0, 0.0),   // degenerate
        (1.0, 1.0, 1.0, 1.0, 1.0),   // degenerate
    ];
    for (x, x1, y1, x2, y2) in extreme_cases {
        let result = solve_bezier_eased_time(x, x1, y1, x2, y2);
        assert!(
            result.is_finite(),
            "solve_bezier({}, {}, {}, {}, {}) = {}",
            x,
            x1,
            y1,
            x2,
            y2,
            result
        );
        assert!(
            (0.0..=1.0).contains(&result),
            "solve_bezier out of range: {}",
            result
        );
    }
    // Overshoot easing legitimately produces values > 1.0 (that's the point)
    let overshoot = solve_bezier_eased_time(0.5, 0.68, -0.4, 0.265, 1.4);
    assert!(overshoot.is_finite(), "overshoot must be finite");
}

/// Regression: Hold interpolation must return the start value for all t < 1.0.
#[test]
fn hold_interpolation_returns_start_value() {
    let kfs = vec![
        Keyframe::new(0, 10.0f32, InterpolationType::Hold),
        Keyframe::new(30, 50.0f32, InterpolationType::Hold),
    ];
    let anim = Animatable::new_animated(kfs);
    // Frame 0 → 10.0, frame 15 → 10.0 (Hold), frame 30 → 50.0
    assert!((anim.evaluate(0) - 10.0).abs() < f32::EPSILON);
    assert!((anim.evaluate(15) - 10.0).abs() < f32::EPSILON);
    assert!((anim.evaluate(30) - 50.0).abs() < f32::EPSILON);
}

/// Regression: Linear interpolation must produce correct intermediate values.
#[test]
fn linear_interpolation_correct() {
    let kfs = vec![
        Keyframe::new(0, 0.0f32, InterpolationType::Linear),
        Keyframe::new(100, 100.0f32, InterpolationType::Linear),
    ];
    let anim = Animatable::new_animated(kfs);
    assert!((anim.evaluate(0) - 0.0).abs() < 0.01);
    assert!((anim.evaluate(50) - 50.0).abs() < 0.01);
    assert!((anim.evaluate(100) - 100.0).abs() < 0.01);
}

/// Regression: Animated property with empty keyframes must not panic.
#[test]
fn animated_empty_keyframes_safe() {
    let anim: Animatable<f32> = Animatable::new_animated(vec![]);
    let val = anim.evaluate(0);
    assert!(val.is_finite());
}

/// Regression: Animated property with single keyframe must return that value.
#[test]
fn animated_single_keyframe() {
    let kfs = vec![Keyframe::new(10, 42.0f32, InterpolationType::Linear)];
    let anim = Animatable::new_animated(kfs);
    assert!((anim.evaluate(0) - 42.0).abs() < 0.01);
    assert!((anim.evaluate(10) - 42.0).abs() < 0.01);
    assert!((anim.evaluate(100) - 42.0).abs() < 0.01);
}

/// Regression: EasePreset control points must all be finite.
#[test]
fn ease_preset_control_points_finite() {
    use kagari_vfx::core::keyframe::EasePreset;
    let presets = [
        EasePreset::Standard,
        EasePreset::FastIn,
        EasePreset::SmoothOut,
        EasePreset::Overshoot,
        EasePreset::Sine,
        EasePreset::EaseIn,
        EasePreset::EaseOut,
        EasePreset::FastOut,
        EasePreset::SlowIn,
        EasePreset::CustomEase,
        EasePreset::MirrorEase,
        EasePreset::Elastic,
        EasePreset::Bounce,
        EasePreset::Cycle,
        EasePreset::MirrorEase2,
        EasePreset::Custom0,
        EasePreset::Custom1,
        EasePreset::Custom2,
        EasePreset::Custom3,
    ];
    for preset in presets {
        let cp = preset.control_points();
        for (i, &v) in cp.iter().enumerate() {
            assert!(
                v.is_finite(),
                "EasePreset::{:?} control_point[{}] = {}",
                preset,
                i,
                v
            );
        }
    }
}

// ─── LifeCurve ─────────────────────────────────────────────────────────────

/// Regression: LifeCurve with empty points must return 1.0.
#[test]
fn lifecurve_empty_returns_one() {
    let lc = LifeCurve(vec![]);
    assert!((lc.eval(0.0) - 1.0).abs() < f32::EPSILON);
    assert!((lc.eval(0.5) - 1.0).abs() < f32::EPSILON);
    assert!((lc.eval(1.0) - 1.0).abs() < f32::EPSILON);
}

/// Regression: LifeCurve with single point must return that constant.
#[test]
fn lifecurve_single_point_constant() {
    let lc = LifeCurve(vec![0.5]);
    assert!((lc.eval(0.0) - 0.5).abs() < f32::EPSILON);
    assert!((lc.eval(0.5) - 0.5).abs() < f32::EPSILON);
    assert!((lc.eval(1.0) - 0.5).abs() < f32::EPSILON);
}
