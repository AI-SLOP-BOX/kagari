#[cfg(test)]
mod multi_camera_tests {
    use super::*;
    use crate::core::keyframe::{InterpolationType, Keyframe};

    #[test]
    fn scene_object_ids_are_unique_and_layer_refs_are_backward_compatible() {
        let mut comp = Composition::new("c".into(), "C".into(), 100, 100, 30, 30);
        assert_eq!(comp.next_camera_id(), "camera_1");
        assert_eq!(comp.next_light_id(), "light_2");

        let camera = Camera3D {
            id: "camera_1".into(),
            ..Default::default()
        };
        comp.cameras.push(camera);
        assert_eq!(comp.next_camera_id(), "camera_2");

        let mut layer = Layer::new(
            "camera_layer".into(),
            "Camera 1".into(),
            LayerType::Null,
            30,
        );
        layer.scene_object = Some(SceneObjectRef::Camera {
            id: "camera_1".into(),
        });
        let json = serde_json::to_string(&layer).expect("scene layer should serialize");
        let restored: Layer = serde_json::from_str(&json).expect("scene layer should deserialize");
        assert_eq!(restored.scene_object, layer.scene_object);

        let mut legacy_value: serde_json::Value =
            serde_json::from_str(&json).expect("serialized layer should be valid JSON");
        legacy_value
            .as_object_mut()
            .expect("layer JSON should be an object")
            .remove("scene_object");
        let legacy: Layer =
            serde_json::from_value(legacy_value).expect("legacy layer should deserialize");
        assert!(legacy.scene_object.is_none());
    }

    #[test]
    fn removing_scene_layer_removes_its_camera_or_light() {
        let mut comp = Composition::new("c".into(), "C".into(), 100, 100, 30, 30);

        let camera = Camera3D {
            id: "camera_1".into(),
            name: "Camera 1".into(),
            ..Default::default()
        };
        comp.cameras.push(camera);
        comp.set_active_camera(Some(0));

        let mut camera_layer = Layer::new(
            "camera_layer".into(),
            "Camera 1".into(),
            LayerType::Null,
            30,
        );
        camera_layer.is_3d = true;
        camera_layer.scene_object = Some(SceneObjectRef::Camera {
            id: "camera_1".into(),
        });
        comp.layers.push(camera_layer);

        let light = Light3D {
            id: "light_2".into(),
            ..Default::default()
        };
        let mut light_layer =
            Layer::new("light_layer".into(), "Light 2".into(), LayerType::Null, 30);
        light_layer.is_3d = true;
        light_layer.scene_object = Some(SceneObjectRef::Light {
            id: "light_2".into(),
        });
        comp.lights.push(light);
        comp.layers.push(light_layer);

        comp.remove_layer_at(0).expect("camera layer should exist");
        assert!(comp.cameras.is_empty());
        assert!(comp.resolve_camera().active);

        comp.remove_layer_at(0).expect("light layer should exist");
        assert!(!comp.lights.iter().any(|light| light.id == "light_2"));
    }

    #[test]
    fn scene_layer_transforms_drive_render_camera_and_light_values() {
        let mut comp = Composition::new("c".into(), "C".into(), 100, 100, 30, 30);

        let camera = Camera3D {
            id: "camera_1".into(),
            active: true,
            ..Default::default()
        };
        comp.cameras.push(camera);
        let mut camera_layer = Layer::new(
            "camera_layer".into(),
            "Camera 1".into(),
            LayerType::Null,
            30,
        );
        camera_layer.is_3d = true;
        camera_layer.scene_object = Some(SceneObjectRef::Camera {
            id: "camera_1".into(),
        });
        camera_layer.transform_3d.position = Animatable::new_constant([10.0, 20.0, -300.0]);
        camera_layer.transform_3d.rotation = Animatable::new_constant([5.0, 10.0, 15.0]);
        comp.layers.push(camera_layer);

        let light = Light3D {
            id: "light_2".into(),
            ..Default::default()
        };
        comp.lights.push(light.clone());
        let mut light_layer =
            Layer::new("light_layer".into(), "Light 2".into(), LayerType::Null, 30);
        light_layer.is_3d = true;
        light_layer.scene_object = Some(SceneObjectRef::Light {
            id: "light_2".into(),
        });
        light_layer.transform_3d.position = Animatable::new_constant([40.0, 50.0, -100.0]);
        comp.layers.push(light_layer);

        let resolved_camera = comp.resolve_camera_at();
        assert_eq!(
            resolved_camera.transform.position.evaluate(0),
            [10.0, 20.0, -300.0]
        );
        assert_eq!(
            resolved_camera.transform.rotation.evaluate(0),
            [5.0, 10.0, 15.0]
        );
        assert_eq!(comp.light_position_at(&light, 0), [40.0, 50.0, -100.0]);
    }

    #[test]
    fn shared_scene_object_survives_until_last_layer_is_removed() {
        let mut comp = Composition::new("c".into(), "C".into(), 100, 100, 30, 30);
        let camera = Camera3D {
            id: "camera_1".into(),
            ..Default::default()
        };
        comp.cameras.push(camera);
        for index in 0..2 {
            let mut layer = Layer::new(
                format!("camera_layer_{index}"),
                format!("Camera 1 copy {index}"),
                LayerType::Null,
                30,
            );
            layer.scene_object = Some(SceneObjectRef::Camera {
                id: "camera_1".into(),
            });
            comp.layers.push(layer);
        }

        comp.remove_layer_at(0)
            .expect("first shared row should exist");
        assert!(comp.cameras.iter().any(|camera| camera.id == "camera_1"));
        comp.remove_layer_at(0)
            .expect("last shared row should exist");
        assert!(!comp.cameras.iter().any(|camera| camera.id == "camera_1"));
    }

    #[test]
    fn precompose_shared_scene_object_keeps_parent_and_child_bindings() {
        let mut comp = Composition::new("main".into(), "Main".into(), 100, 100, 30, 30);
        let camera = Camera3D {
            id: "camera_1".into(),
            active: true,
            ..Default::default()
        };
        comp.cameras.push(camera);
        for index in 0..2 {
            let mut layer = Layer::new(
                format!("camera_layer_{index}"),
                format!("Camera 1 copy {index}"),
                LayerType::Null,
                30,
            );
            layer.scene_object = Some(SceneObjectRef::Camera {
                id: "camera_1".into(),
            });
            comp.layers.push(layer);
        }

        let nested = comp
            .precompose_layers(
                &["camera_layer_0".into()],
                "sub1".into(),
                "Sub Comp 1".into(),
                PrecompAttributesMode::MoveToNewComp,
            )
            .expect("precompose should succeed");

        assert!(comp.cameras.iter().any(|camera| camera.id == "camera_1"));
        assert!(nested.cameras.iter().any(|camera| camera.id == "camera_1"));
        assert_eq!(nested.resolve_camera().id, "camera_1");
    }

    #[test]
    fn test_resolve_camera_prefers_active_flag() {
        let mut comp = Composition::new("c".into(), "C".into(), 100, 100, 30, 30);
        assert_eq!(
            comp.resolve_camera().name,
            "Active Camera",
            "legacy fallback"
        );

        let wide = Camera3D {
            name: "Wide".into(),
            fov_degrees: 90.0,
            active: false,
            ..Default::default()
        };
        comp.cameras.push(wide);
        // No active scene camera → legacy active_camera wins
        assert_eq!(comp.resolve_camera().fov_degrees, 50.0);

        comp.set_active_camera(Some(0));
        let cam = comp.resolve_camera();
        assert!((cam.fov_degrees - 90.0).abs() < 1e-4, "active camera wins");
    }

    #[test]
    fn test_set_active_camera_exclusive() {
        let mut comp = Composition::new("c".into(), "C".into(), 100, 100, 30, 30);
        for i in 0..2 {
            let c = Camera3D {
                name: format!("Cam{}", i),
                active: false,
                ..Default::default()
            };
            comp.cameras.push(c);
        }
        comp.set_active_camera(Some(1));
        assert!(comp.cameras[1].active);
        assert!(!comp.cameras[0].active);
        comp.set_active_camera(Some(0));
        assert!(comp.cameras[0].active && !comp.cameras[1].active);
        comp.set_active_camera(None);
        assert!(comp.cameras.iter().all(|c| !c.active), "None clears all");
        // Falls back to legacy when nothing active
        assert_eq!(comp.resolve_camera().name, "Active Camera");
    }

    #[test]
    fn camera_lens_tracks_evaluate_and_fallback_to_legacy_values() {
        let defaults = Camera3D::default();
        assert_eq!(defaults.fov_at(12), 50.0);
        assert_eq!(defaults.focus_distance_at(12), 1000.0);
        assert_eq!(defaults.aperture_at(12), 2.8);
        let camera = Camera3D {
            fov_animation: Some(Animatable::new_animated(vec![
                Keyframe::new(0, 40.0, InterpolationType::Linear),
                Keyframe::new(10, 80.0, InterpolationType::Linear),
            ])),
            focus_distance_animation: Some(Animatable::new_animated(vec![
                Keyframe::new(0, 500.0, InterpolationType::Linear),
                Keyframe::new(10, 1500.0, InterpolationType::Linear),
            ])),
            aperture_animation: Some(Animatable::new_animated(vec![
                Keyframe::new(0, 2.0, InterpolationType::Linear),
                Keyframe::new(10, 4.0, InterpolationType::Linear),
            ])),
            ..defaults
        };
        assert_eq!(camera.fov_at(0), 40.0);
        assert_eq!(camera.fov_at(10), 80.0);
        assert_eq!(camera.focus_distance_at(5), 1000.0);
        assert_eq!(camera.aperture_at(5), 3.0);
    }

    #[test]
    fn legacy_camera_json_deserializes_without_lens_tracks() {
        let json = r#"{"name":"Legacy","active":true,"fov_degrees":65.0,"focus_distance":900.0,"aperture":4.0,"transform":{"position":{"type":"Constant","value":[0.0,0.0,0.0]},"rotation":{"type":"Constant","value":[0.0,0.0,0.0]},"scale":{"type":"Constant","value":[100.0,100.0,100.0]}},"dof_enabled":false,"dof_max_blur":16.0,"dof_iris_sides":0}"#;
        let camera: Camera3D = serde_json::from_str(json).expect("legacy camera JSON");
        assert!(camera.fov_animation.is_none());
        assert!(camera.focus_distance_animation.is_none());
        assert!(camera.aperture_animation.is_none());
        assert_eq!(camera.fov_at(100), 65.0);
        assert_eq!(camera.focus_distance_at(100), 900.0);
        assert_eq!(camera.aperture_at(100), 4.0);
    }

    #[test]
    fn animated_camera_lens_tracks_survive_json_roundtrip() {
        let camera = Camera3D {
            fov_animation: Some(Animatable::new_animated(vec![
                Keyframe::new(0, 50.0, InterpolationType::Linear),
                Keyframe::new(24, 90.0, InterpolationType::Linear),
            ])),
            focus_distance_animation: Some(Animatable::new_animated(vec![
                Keyframe::new(0, 1000.0, InterpolationType::Linear),
                Keyframe::new(24, 2400.0, InterpolationType::Linear),
            ])),
            aperture_animation: Some(Animatable::new_animated(vec![
                Keyframe::new(0, 2.8, InterpolationType::Linear),
                Keyframe::new(24, 8.0, InterpolationType::Linear),
            ])),
            ..Default::default()
        };
        let restored: Camera3D =
            serde_json::from_str(&serde_json::to_string(&camera).unwrap()).unwrap();
        assert_eq!(restored.fov_at(12), 70.0);
        assert_eq!(restored.focus_distance_at(12), 1700.0);
        assert!((restored.aperture_at(12) - 5.4).abs() < 1e-4);
    }

    #[test]
    fn animated_fov_changes_the_3d_projection_matrix_per_frame() {
        let mut comp =
            Composition::new("camera-test".into(), "Camera Test".into(), 640, 360, 30, 30);
        comp.active_camera.fov_animation = Some(Animatable::new_animated(vec![
            Keyframe::new(0, 30.0, InterpolationType::Linear),
            Keyframe::new(10, 90.0, InterpolationType::Linear),
        ]));
        let layer = Layer::new(
            "3d-layer".into(),
            "3D Layer".into(),
            LayerType::Solid {
                color: [1.0, 1.0, 1.0, 1.0],
            },
            30,
        );
        let near = comp.resolve_world_transform_3d(&layer, 0);
        let far = comp.resolve_world_transform_3d(&layer, 10);
        assert_ne!(near[0][0], far[0][0]);
        assert!((comp.resolve_camera().fov_at(5) - 60.0).abs() < 1e-4);
    }

    #[test]
    fn animated_dof_controls_follow_the_current_frame() {
        let camera = Camera3D {
            dof_enabled_animation: Some(Animatable::new_animated(vec![
                Keyframe::new(0, 0.0, InterpolationType::Linear),
                Keyframe::new(10, 1.0, InterpolationType::Linear),
            ])),
            dof_max_blur_animation: Some(Animatable::new_animated(vec![
                Keyframe::new(0, 4.0, InterpolationType::Linear),
                Keyframe::new(10, 24.0, InterpolationType::Linear),
            ])),
            ..Default::default()
        };
        assert!(!camera.dof_enabled_at(0));
        assert!(camera.dof_enabled_at(10));
        assert_eq!(camera.dof_max_blur_at(5), 14.0);
    }

    #[test]
    fn setting_first_lens_key_preserves_legacy_value_before_keyframe() {
        let mut camera = Camera3D::default();
        camera.set_fov_at(10, 80.0);
        camera.set_focus_distance_at(10, 2000.0);
        camera.set_aperture_at(10, 8.0);
        camera.set_dof_max_blur_at(10, 32.0);
        camera.set_dof_enabled_at(10, true);
        assert_eq!(camera.fov_at(0), 50.0);
        assert_eq!(camera.focus_distance_at(0), 1000.0);
        assert_eq!(camera.aperture_at(0), 2.8);
        assert_eq!(camera.dof_max_blur_at(0), 16.0);
        assert!(!camera.dof_enabled_at(0));
        assert!(camera.dof_enabled_at(10));
    }

    #[test]
    fn test_resolve_camera_mut_targets_active() {
        let mut comp = Composition::new("c".into(), "C".into(), 100, 100, 30, 30);
        let c = Camera3D {
            name: "B".into(),
            ..Default::default()
        };
        comp.cameras.push(c);
        comp.set_active_camera(Some(0));
        comp.resolve_camera_mut().fov_degrees = 120.0;
        assert!((comp.cameras[0].fov_degrees - 120.0).abs() < 1e-4);
    }
}
