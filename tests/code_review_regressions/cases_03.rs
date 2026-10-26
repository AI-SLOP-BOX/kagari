/// Regression: FadeCurve::apply must return values in [0,1] for all inputs.
#[test]
/// Regression: FadeCurve::apply must return values in [0,1] for all inputs.
fn fade_curve_output_in_unit_range() {
    for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let linear = FadeCurve::Linear.apply(t);
        let ease_in = FadeCurve::EaseIn.apply(t);
        let ease_out = FadeCurve::EaseOut.apply(t);
        assert!((0.0..=1.0).contains(&linear), "Linear({}) = {}", t, linear);
        assert!(
            (0.0..=1.0).contains(&ease_in),
            "EaseIn({}) = {}",
            t,
            ease_in
        );
        assert!(
            (0.0..=1.0).contains(&ease_out),
            "EaseOut({}) = {}",
            t,
            ease_out
        );
    }
}

/// Regression: FadeCurve with clamped input.
#[test]
fn fade_curve_clamps_input() {
    // Values outside 0..1 should be clamped
    let r = FadeCurve::Linear.apply(-1.0);
    assert!((0.0..=1.0).contains(&r));
    let r = FadeCurve::Linear.apply(2.0);
    assert!((0.0..=1.0).contains(&r));
}

// ─── ShapeType Roundtrip ────────────────────────────────────────────────────

/// Regression: all ShapeType variants must serialize/deserialize.
#[test]
fn shape_type_roundtrip() {
    let shapes = [
        ShapeType::Rectangle {
            width: Animatable::new_constant(100.0),
            height: Animatable::new_constant(50.0),
            corner_radius: Animatable::new_constant(5.0),
        },
        ShapeType::Ellipse {
            width: Animatable::new_constant(100.0),
            height: Animatable::new_constant(50.0),
        },
        ShapeType::Star {
            points: Animatable::new_constant(5.0),
            inner_radius: Animatable::new_constant(20.0),
            outer_radius: Animatable::new_constant(50.0),
        },
        ShapeType::Polygon {
            sides: Animatable::new_constant(6.0),
            radius: Animatable::new_constant(50.0),
        },
    ];
    for shape in shapes {
        let json = serde_json::to_string(&shape).unwrap();
        let restored: ShapeType = serde_json::from_str(&json).unwrap();
        assert_eq!(format!("{:?}", shape), format!("{:?}", restored));
    }
}
