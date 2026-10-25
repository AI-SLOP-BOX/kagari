mod tests {
    use super::*;

    #[test]
    fn next_layer_id_skips_existing_ids() {
        let mut comp = Composition::new("c".into(), "C".into(), 16, 16, 30, 30);
        comp.layers.push(Layer::new(
            "image_0".into(),
            "First".into(),
            LayerType::Null,
            30,
        ));
        comp.layers.push(Layer::new(
            "image_2".into(),
            "Second".into(),
            LayerType::Null,
            30,
        ));
        assert_eq!(comp.next_layer_id("image"), "image_3");
        comp.layers.push(Layer::new(
            "image_3".into(),
            "Third".into(),
            LayerType::Null,
            30,
        ));
        assert_eq!(comp.next_layer_id("image"), "image_4");
    }

    #[test]
    fn project_composition_id_generation_checks_nested_compositions() {
        let mut project = Project::empty_for_new_project();
        project.compositions[0].id = "comp_1".into();
        project.compositions[0]
            .sub_compositions
            .push(Composition::new(
                "comp_2".into(),
                "Nested".into(),
                16,
                16,
                30,
                30,
            ));

        assert_eq!(project.next_composition_id("comp"), "comp_3");
    }

    #[test]
    fn project_item_id_generation_skips_ids_after_asset_removal() {
        let mut project = Project::empty_for_new_project();
        project.assets = vec![
            ProjectItem::new(
                "asset_0",
                "First",
                ProjectItemType::Folder {
                    name: "First".into(),
                },
            ),
            ProjectItem::new(
                "asset_2",
                "Second",
                ProjectItemType::Folder {
                    name: "Second".into(),
                },
            ),
        ];

        assert_eq!(project.next_project_item_id("asset"), "asset_3");
    }

    #[test]
    fn project_item_missing_media_detection() {
        let missing = ProjectItem::new(
            "a",
            "gone.png",
            ProjectItemType::Image {
                path: "/nonexistent/kagari_missing_001.png".into(),
                width: 64,
                height: 64,
            },
        );
        assert_eq!(
            missing.media_path(),
            Some("/nonexistent/kagari_missing_001.png")
        );
        assert!(missing.is_media_missing());

        let solid = ProjectItem::new("b", "bg", ProjectItemType::Solid { color: [0.0; 4] });
        assert_eq!(solid.media_path(), None);
        assert!(!solid.is_media_missing());
    }

    #[test]
    fn relink_recurses_into_precomps_and_invalidates_video_cache() {
        let root = std::env::temp_dir().join(format!(
            "kagari_relink_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after unix epoch")
                .as_nanos()
        ));
        let footage_dir = root.join("Footage");
        std::fs::create_dir_all(&footage_dir).expect("test footage directory should be created");
        let footage = footage_dir.join("nested.mp4");
        std::fs::write(&footage, b"test placeholder").expect("test footage should be created");

        let mut project = Project::default();
        let mut sub = Composition::new("sub".into(), "Nested".into(), 64, 64, 24, 24);
        sub.layers.push(Layer::new(
            "video".into(),
            "Nested Video".into(),
            LayerType::Video {
                source: "/missing/nested.mp4".into(),
                frames_dir: "/old/cache/frames".into(),
                frame_count: 12,
                audio_wav: Some("/old/cache/audio.wav".into()),
                audio_volume: crate::core::property::Animatable::new_constant(0.0),
                speed: 1.0,
            },
            24,
        ));
        project.compositions[0].sub_compositions.push(sub);

        assert_eq!(project.resolve_relative_footage_paths(&root), 1);
        let LayerType::Video {
            source,
            frames_dir,
            frame_count,
            audio_wav,
            ..
        } = &project.compositions[0].sub_compositions[0].layers[0].layer_type
        else {
            panic!("expected nested video layer");
        };
        assert_eq!(source, &footage.to_string_lossy());
        assert!(frames_dir.is_empty());
        assert_eq!(*frame_count, 0);
        assert!(audio_wav.is_none());

        std::fs::remove_dir_all(root).expect("test footage should be removable");
    }

    #[test]
    fn test_remap_frame_for_loop_cycle() {
        // Keyframe interval: 0 to 30 (span = 30)
        // Frame 15 -> 15
        assert_eq!(remap_frame_for_loop(15, 0, 30, false), 15);
        // Frame 30 -> 30
        assert_eq!(remap_frame_for_loop(30, 0, 30, false), 30);
        // Frame 45 -> (45-0)%30 = 15
        assert_eq!(remap_frame_for_loop(45, 0, 30, false), 15);
    }

    #[test]
    fn test_remap_frame_for_loop_pingpong() {
        // Keyframe interval: 0 to 30 (span = 30)
        // Cycle 0 (0..30): frame 15 -> 15
        assert_eq!(remap_frame_for_loop(15, 0, 30, true), 15);
        // Cycle 1 (30..60, odd cycle = reverse): frame 45 -> 30 - 15 = 15
        assert_eq!(remap_frame_for_loop(45, 0, 30, true), 15);
        // Cycle 2 (60..90, even cycle = forward): frame 75 -> 0 + 15 = 15
        assert_eq!(remap_frame_for_loop(75, 0, 30, true), 15);
    }

    #[test]
    fn test_resolve_all_world_transforms_cached() {
        let mut comp = Composition::new("c1".into(), "Comp".into(), 1920, 1080, 30, 300);
        let mut parent = Layer::new("p1".into(), "Parent".into(), LayerType::Null, 300);
        parent.transform.position = Animatable::new_constant([100.0, 100.0]);

        let mut child = Layer::new("c1".into(), "Child".into(), LayerType::Null, 300);
        child.parent_id = Some("p1".into());
        child.transform.position = Animatable::new_constant([50.0, 50.0]);

        comp.add_layer(parent);
        comp.add_layer(child);

        let cached = comp.resolve_all_world_transforms_cached(0);
        assert_eq!(cached.len(), 2);
        assert_eq!(cached.get("c1").unwrap().0, [150.0, 150.0]);
    }

    #[test]
    fn test_parent_cycle_prevention() {
        let mut comp = Composition::new("c1".into(), "Comp".into(), 1920, 1080, 30, 300);
        let l1 = Layer::new("l1".into(), "L1".into(), LayerType::Null, 300);
        let l2 = Layer::new("l2".into(), "L2".into(), LayerType::Null, 300);

        comp.add_layer(l1);
        comp.add_layer(l2);

        assert!(comp.set_layer_parent("l2", Some("l1".into())));
        assert!(
            !comp.set_layer_parent("l1", Some("l2".into())),
            "Cycle parent assignment must be rejected"
        );
    }

    #[test]
    fn test_rove_across_time() {
        let mut kfs = vec![
            crate::core::keyframe::Keyframe::new(
                0,
                [0.0, 0.0],
                crate::core::keyframe::InterpolationType::Linear,
            ),
            crate::core::keyframe::Keyframe::new(
                10,
                [100.0, 0.0],
                crate::core::keyframe::InterpolationType::Linear,
            ), // Intermediate point at 50% distance
            crate::core::keyframe::Keyframe::new(
                60,
                [200.0, 0.0],
                crate::core::keyframe::InterpolationType::Linear,
            ),
        ];
        rove_across_time(&mut kfs);
        // Distance 0->100 is 100, 100->200 is 100 => midpoint should be remapped from 10 to 30
        assert_eq!(kfs[0].frame, 0);
        assert_eq!(kfs[1].frame, 30);
        assert_eq!(kfs[2].frame, 60);
    }

    #[test]
    fn test_sequence_layers() {
        let mut l1 = Layer::new("1".into(), "L1".into(), LayerType::Null, 30);
        l1.in_frame = 0;
        l1.out_frame = 30;
        let mut l2 = Layer::new("2".into(), "L2".into(), LayerType::Null, 30);
        l2.in_frame = 0;
        l2.out_frame = 30;

        let mut layers = vec![l1, l2];
        sequence_layers(&mut layers, 5, true);

        assert_eq!(layers[0].in_frame, 0);
        assert_eq!(layers[0].out_frame, 30);
        assert_eq!(layers[1].in_frame, 25);
        assert_eq!(layers[1].out_frame, 55);
    }

    #[test]
    fn test_export_youtube_chapters() {
        let markers = vec![
            TimelineMarker {
                frame: 0,
                label: "Introduction".into(),
                color: [1.0, 1.0, 1.0],
            },
            TimelineMarker {
                frame: 90,
                label: "Main Feature".into(),
                color: [1.0, 1.0, 1.0],
            },
        ];
        let chapters = export_youtube_chapters(&markers, 30);
        assert!(chapters.contains("00:00 Introduction"));
        assert!(chapters.contains("00:03 Main Feature"));
    }

    #[test]
    fn test_precompose_layers_move_and_leave_attributes() {
        let mut comp = Composition::new("main".into(), "Main".into(), 1920, 1080, 30, 300);
        let mut l1 = Layer::new("l1".into(), "Text 1".into(), LayerType::Null, 300);
        l1.transform.position = Animatable::new_constant([123.0, 456.0]);
        comp.add_layer(l1);

        let sub_comp = comp
            .precompose_layers(
                &["l1".into()],
                "sub1".into(),
                "Sub Comp 1".into(),
                PrecompAttributesMode::MoveToNewComp,
            )
            .expect("precompose should succeed");

        assert_eq!(comp.layers.len(), 1);
        assert!(matches!(
            comp.layers[0].layer_type,
            LayerType::PreComp { .. }
        ));
        assert_eq!(sub_comp.layers.len(), 1);
        assert_eq!(
            sub_comp.layers[0].transform.position.evaluate(0),
            [123.0, 456.0]
        );
    }

    #[test]
    fn test_precompose_moves_referenced_scene_objects() {
        let mut comp = Composition::new("main".into(), "Main".into(), 1920, 1080, 30, 300);
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
            300,
        );
        camera_layer.scene_object = Some(SceneObjectRef::Camera {
            id: "camera_1".into(),
        });
        let light = Light3D {
            id: "light_2".into(),
            ..Default::default()
        };
        comp.lights.push(light);
        let mut light_layer =
            Layer::new("light_layer".into(), "Light 2".into(), LayerType::Null, 300);
        light_layer.scene_object = Some(SceneObjectRef::Light {
            id: "light_2".into(),
        });
        comp.add_layer(camera_layer);
        comp.add_layer(light_layer);

        let nested = comp
            .precompose_layers(
                &["camera_layer".into(), "light_layer".into()],
                "sub1".into(),
                "Sub Comp 1".into(),
                PrecompAttributesMode::MoveToNewComp,
            )
            .expect("precompose should succeed");

        assert!(!comp.cameras.iter().any(|camera| camera.id == "camera_1"));
        assert!(!comp.lights.iter().any(|light| light.id == "light_2"));
        assert!(nested.cameras.iter().any(|camera| camera.id == "camera_1"));
        assert!(nested.lights.iter().any(|light| light.id == "light_2"));
        assert_eq!(nested.resolve_camera().id, "camera_1");
    }
}
