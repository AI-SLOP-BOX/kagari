#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use super::super::*;

    #[test]
    fn test_solve_bezier_eased_time_precision() {
        let t0 = solve_bezier_eased_time(0.0, 0.25, 0.1, 0.25, 1.0);
        let t1 = solve_bezier_eased_time(1.0, 0.25, 0.1, 0.25, 1.0);
        let t_mid = solve_bezier_eased_time(0.5, 0.25, 0.1, 0.25, 1.0);

        assert_eq!(t0, 0.0);
        assert_eq!(t1, 1.0);
        assert!(t_mid > 0.0 && t_mid < 1.0);
    }

    #[test]
    fn test_compute_ae_bezier_control_points() {
        let outgoing = BezierControlPoint {
            influence: 0.33,
            speed: 100.0,
        };
        let incoming = BezierControlPoint {
            influence: 0.33,
            speed: 0.0,
        };
        let pts = compute_ae_bezier_control_points(&outgoing, &incoming, 30.0, 100.0, 30.0);
        assert!(pts[0] >= 0.0 && pts[0] <= 1.0);
        assert!(pts[2] >= 0.0 && pts[2] <= 1.0);

        // Inverse mapping roundtrip
        let (rec_out, rec_in) = ae_bezier_control_points_to_speed_influence(pts, 30.0, 100.0, 30.0);
        assert!((rec_out.influence - outgoing.influence).abs() < 0.01);
        assert!((rec_out.speed - outgoing.speed).abs() < 1.0);
        assert!((rec_in.influence - incoming.influence).abs() < 0.01);
    }

    #[test]
    fn test_evaluate_scalar_speed_at_frame() {
        let kfs = vec![
            Keyframe::new(0, 0.0, InterpolationType::Linear),
            Keyframe::new(30, 300.0, InterpolationType::Linear), // 300 units in 1 sec (30 frames at 30fps) => velocity = 300 units/sec
        ];
        let speed_at_15 = evaluate_scalar_speed_at_frame(&kfs, 15.0, 30);
        assert!(
            (speed_at_15 - 300.0).abs() < 1.0,
            "linear speed should be 300 units/sec, got {}",
            speed_at_15
        );
    }

    #[test]
    fn test_ease_preset_control_points() {
        let std_pts = EasePreset::Standard.control_points();
        assert_eq!(std_pts, [0.25, 0.1, 0.25, 1.0]);

        let fast_pts = EasePreset::FastIn.control_points();
        assert_eq!(fast_pts, [0.42, 0.0, 1.0, 1.0]);

        // Evaluate solve_bezier_eased_time for Overshoot preset
        let over_pts = EasePreset::Overshoot.control_points();
        let eased =
            solve_bezier_eased_time(0.5, over_pts[0], over_pts[1], over_pts[2], over_pts[3]);
        assert!(!eased.is_nan());
    }

    #[test]
    fn test_bezier_mirror_math() {
        let original: [f32; 4] = [0.25, 0.1, 0.75, 0.9];
        let mirrored = [
            1.0 - original[2],
            1.0 - original[3],
            1.0 - original[0],
            1.0 - original[1],
        ];
        assert!((mirrored[0] - 0.25).abs() < 1e-5);
        assert!((mirrored[1] - 0.1).abs() < 1e-5);
        assert!((mirrored[2] - 0.75).abs() < 1e-5);
        assert!((mirrored[3] - 0.9).abs() < 1e-5);

        let fast_in: [f32; 4] = [0.42, 0.0, 1.0, 1.0];
        let mirrored_fast_in = [
            1.0 - fast_in[2],
            1.0 - fast_in[3],
            1.0 - fast_in[0],
            1.0 - fast_in[1],
        ];
        assert!((mirrored_fast_in[0] - 0.0).abs() < 1e-5);
        assert!((mirrored_fast_in[1] - 0.0).abs() < 1e-5);
        assert!((mirrored_fast_in[2] - 0.58).abs() < 1e-5);
        assert!((mirrored_fast_in[3] - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_bezier_handle_defaults() {
        let h = BezierHandle::default();
        assert!((h.in_x - 0.0).abs() < 1e-6);
        assert!((h.in_y - 0.0).abs() < 1e-6);
        assert!((h.out_x - 1.0).abs() < 1e-6);
        assert!((h.out_y - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cubic_bezier_endpoints() {
        // At t=0 should return p0, at t=1 should return p3
        let v0 = cubic_bezier(0.0, 0.5, 0.5, 1.0, 0.0);
        let v1 = cubic_bezier(0.0, 0.5, 0.5, 1.0, 1.0);
        assert!((v0 - 0.0).abs() < 1e-6);
        assert!((v1 - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_interpolate_bezier_linear() {
        let lin = BezierHandle::linear();
        let v = interpolate_bezier(0.0, 100.0, 0.0, 10.0, 5.0, &lin, &lin);
        assert!((v - 50.0).abs() < 1e-3);
    }

    #[test]
    fn test_interpolate_bezier_ease_in() {
        let ease_in = BezierHandle {
            in_x: 0.42,
            in_y: 0.0,
            out_x: 1.0,
            out_y: 0.8,
        };
        let lin = BezierHandle::linear();
        let v_mid = interpolate_bezier(0.0, 100.0, 0.0, 10.0, 5.0, &ease_in, &lin);
        // With out_y=0.8 the curve is below linear at midpoint → value < 50
        assert!(v_mid < 50.0);
        // At endpoints the value should match exactly
        let v_start = interpolate_bezier(0.0, 100.0, 0.0, 10.0, 0.0, &ease_in, &lin);
        let v_end = interpolate_bezier(0.0, 100.0, 0.0, 10.0, 10.0, &ease_in, &lin);
        assert!((v_start - 0.0).abs() < 1e-3);
        assert!((v_end - 100.0).abs() < 1e-3);
    }

    fn handle(x: f32, y: f32) -> BezierHandle {
        BezierHandle {
            in_x: x,
            in_y: y,
            out_x: x,
            out_y: y,
        }
    }

    #[test]
    fn test_new_presets_have_valid_control_points() {
        let presets = [
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
            EasePreset::EaseIn,
            EasePreset::EaseOut,
        ];
        for p in presets {
            let pts = p.control_points();
            assert!((0.0..=1.0).contains(&pts[0]), "{:?} x1 out of range", p);
            assert!((0.0..=1.0).contains(&pts[2]), "{:?} x2 out of range", p);
            assert!(pts[0] <= pts[2], "{:?} x1 must be <= x2", p);
        }
    }

    #[test]
    fn test_new_presets_interpolate_monotonically() {
        for preset in [
            EasePreset::FastOut,
            EasePreset::SlowIn,
            EasePreset::Sine,
            EasePreset::Cycle,
        ] {
            let [x1, y1, x2, y2] = preset.control_points();
            let out_h = handle(x1, y1);
            let in_h = handle(x2, y2);
            let mut prev = interpolate_bezier(0.0, 1.0, 0.0, 10.0, 0.0, &out_h, &in_h);
            for i in 1..=10 {
                let t = i as f32 / 10.0;
                let v = interpolate_bezier(0.0, 1.0, 0.0, 10.0, 10.0 * t, &out_h, &in_h);
                assert!(v >= prev - 1e-3, "{:?} not monotonic at t={}", preset, t);
                prev = v;
            }
        }
    }

    #[test]
    fn test_ease_in_out_endpoints() {
        for preset in [
            EasePreset::EaseIn,
            EasePreset::EaseOut,
            EasePreset::FastOut,
            EasePreset::SlowIn,
        ] {
            let [x1, y1, x2, y2] = preset.control_points();
            let out_h = handle(x1, y1);
            let in_h = handle(x2, y2);
            let v_start = interpolate_bezier(0.0, 100.0, 0.0, 10.0, 0.0, &out_h, &in_h);
            let v_end = interpolate_bezier(0.0, 100.0, 0.0, 10.0, 10.0, &out_h, &in_h);
            assert!(
                v_start.abs() < 1e-3,
                "{:?} start != 0, got {}",
                preset,
                v_start
            );
            assert!(
                (v_end - 100.0).abs() < 1e-3,
                "{:?} end != 100, got {}",
                preset,
                v_end
            );
        }
    }
}
