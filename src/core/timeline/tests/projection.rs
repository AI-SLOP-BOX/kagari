#[cfg(test)]
mod projection_tests {
    use super::*;
    use crate::core::keyframe::{InterpolationType, Keyframe};

    /// Camera pulled back on -Z looking toward +Z (matches renderer convention:
    /// visible points have larger z than the camera).
    fn default_cam() -> Camera3D {
        let mut cam = Camera3D::default();
        cam.transform.position = Animatable::new_constant([0.0, 0.0, -600.0]);
        cam
    }

    #[test]
    fn test_project_point_centered_in_front() {
        let cam = default_cam();
        let sp = project_point_to_screen(&cam, [0.0, 0.0, -500.0], 1920.0, 1080.0);
        if let Some([x, y]) = sp {
            assert!((x - 0.5).abs() < 0.01, "centered point → 0.5, got {x}");
            assert!((y - 0.5).abs() < 0.01, "centered point → 0.5, got {y}");
        } else {
            panic!("point in front of camera must project");
        }
    }

    #[test]
    fn test_project_point_behind_camera_none() {
        let cam = default_cam();
        let sp = project_point_to_screen(&cam, [0.0, 0.0, -900.0], 1920.0, 1080.0);
        assert!(sp.is_none(), "point behind camera returns None");
    }

    #[test]
    fn test_project_point_right_of_center() {
        let cam = default_cam();
        let sp = project_point_to_screen(&cam, [250.0, 0.0, -500.0], 1000.0, 1000.0);
        let [x, _] = sp.expect("must project");
        assert!(x > 0.5, "point right of optical axis → x > 0.5, got {x}");
    }

    #[test]
    fn test_projection_matches_documented_formulas() {
        let cam = default_cam(); // pos [0,0,-600], fov 50
        let pos = [300.0, 200.0, -500.0];
        let sp = project_point_to_screen(&cam, pos, 1920.0, 1080.0).expect("projects");
        // Reference: d = point - campos; focal from vertical fov; sy flipped
        let focal = (1080.0 * 0.5) / (50f32.to_radians() * 0.5).tan();
        let expect_x = (960.0 + 300.0 * focal / 100.0) / 1920.0;
        let expect_y = (540.0 - 200.0 * focal / 100.0) / 1080.0;
        assert!((sp[0] - expect_x).abs() < 1e-4);
        assert!((sp[1] - expect_y).abs() < 1e-4);
    }

    #[test]
    fn frame_aware_projection_uses_animated_camera_transform_and_fov() {
        let mut cam = default_cam();
        cam.transform.position = Animatable::new_animated(vec![
            Keyframe::new(0, [0.0, 0.0, -600.0], InterpolationType::Linear),
            Keyframe::new(10, [100.0, 0.0, -600.0], InterpolationType::Linear),
        ]);
        cam.fov_animation = Some(Animatable::new_animated(vec![
            Keyframe::new(0, 40.0, InterpolationType::Linear),
            Keyframe::new(10, 80.0, InterpolationType::Linear),
        ]));
        let p0 =
            project_point_to_screen_at_frame(&cam, [0.0, 0.0, -500.0], 1920.0, 1080.0, 0).unwrap();
        let p10 =
            project_point_to_screen_at_frame(&cam, [0.0, 0.0, -500.0], 1920.0, 1080.0, 10).unwrap();
        assert!((p0[0] - 0.5).abs() < 1e-4);
        assert!(p10[0] < p0[0]);
    }
}
