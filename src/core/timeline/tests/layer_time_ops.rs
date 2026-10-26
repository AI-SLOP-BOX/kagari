#[cfg(test)]
mod layer_time_ops_tests {
    use super::*;

    fn test_layer() -> Layer {
        let mut l = Layer::new(
            "l".into(),
            "L".into(),
            LayerType::Solid {
                color: [0.5, 0.5, 0.5, 1.0],
            },
            60,
        );
        l.in_frame = 10;
        l.out_frame = 40;
        l
    }

    #[test]
    fn test_enable_time_remapping_is_identity() {
        let mut l = test_layer();
        l.enable_time_remapping();
        assert_eq!(l.remap_frame(10), 10);
        assert_eq!(l.remap_frame(25), 25);
        assert_eq!(l.remap_frame(40), 40);
        // Idempotent: calling again must not clobber existing keyframes.
        l.enable_time_remapping();
        assert!(l.time_remap.is_some());
    }

    #[test]
    fn test_clear_time_remap_restores_passthrough() {
        let mut l = test_layer();
        l.freeze_at(17);
        assert_eq!(l.remap_frame(10), 17);
        l.clear_time_remap();
        assert_eq!(l.remap_frame(10), 10);
        assert_eq!(l.remap_frame(33), 33);
    }

    #[test]
    fn test_time_reverse_flaps_endpoints() {
        let mut l = test_layer();
        l.time_reverse();
        assert_eq!(l.remap_frame(10), 40);
        assert_eq!(l.remap_frame(25), 25);
        assert_eq!(l.remap_frame(40), 10);
    }

    #[test]
    fn test_freeze_at_holds_constant() {
        let mut l = test_layer();
        l.freeze_at(17);
        assert_eq!(l.remap_frame(10), 17);
        assert_eq!(l.remap_frame(99), 17);
    }

    #[test]
    fn test_easy_ease_transform_sets_bezier_and_skips_expressions() {
        use crate::core::keyframe::{InterpolationType, Keyframe};
        let mut l = test_layer();
        l.transform.position = Animatable::Animated(vec![
            Keyframe::new(0, [0.0, 0.0], InterpolationType::Linear),
            Keyframe::new(30, [100.0, 0.0], InterpolationType::Linear),
        ]);
        l.transform.rotation = Animatable::Animated(vec![
            Keyframe::new(0, 0.0, InterpolationType::Linear),
            Keyframe::new(30, 90.0, InterpolationType::Linear),
        ]);
        // Expression-driven properties must be left alone.
        l.transform.opacity_expression = Some(crate::core::timeline::Expression::Wiggle {
            frequency: 2.0,
            amplitude: 10.0,
        });
        l.transform.opacity = Animatable::Animated(vec![
            Keyframe::new(0, 100.0, InterpolationType::Linear),
            Keyframe::new(30, 0.0, InterpolationType::Linear),
        ]);

        l.easy_ease_transform();

        if let Some(kfs) = l.transform.position.keyframes() {
            for kf in kfs {
                assert!(matches!(kf.interpolation, InterpolationType::Bezier { .. }));
            }
        } else {
            panic!("position should be animated");
        }
        if let Some(kfs) = l.transform.rotation.keyframes() {
            assert!(matches!(
                kfs[0].interpolation,
                InterpolationType::Bezier { .. }
            ));
        }
        // Opacity has an expression: keyframes keep linear interpolation.
        if let Some(kfs) = l.transform.opacity.keyframes() {
            assert!(matches!(kfs[0].interpolation, InterpolationType::Linear));
        }
    }

    #[test]
    fn test_time_stretch_scales_span_and_source_rate() {
        let mut l = test_layer(); // in=10 out=40 (30 frames)
        l.time_stretch(2.0);
        // Span doubles.
        assert_eq!(l.in_frame, 10);
        assert_eq!(l.out_frame, 70);
        // Source advances at half rate: mid-span maps to mid source frame.
        assert_eq!(l.remap_frame(10), 10);
        assert_eq!(l.remap_frame(50), 30);
        assert_eq!(l.remap_frame(70), 40);

        // Speed-up: factor 0.5 halves the span; source runs at double rate.
        let mut fast = test_layer();
        fast.time_stretch(0.5);
        assert_eq!(fast.out_frame, 25);
        assert_eq!(fast.remap_frame(25), 40);

        // Degenerate factor is ignored.
        let mut safe = test_layer();
        safe.time_stretch(0.0);
        assert_eq!(safe.out_frame, 40);
        assert!(safe.time_remap.is_none());
    }

    #[test]
    fn test_easy_ease_on_constants_is_noop() {
        let mut l = test_layer();
        l.easy_ease_transform(); // all constants — must not panic or animate
        assert!(l.transform.position.keyframes().is_none());
    }
}
