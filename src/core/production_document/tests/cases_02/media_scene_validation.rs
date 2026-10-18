#[test]
fn rejects_invalid_media_scene_markers_and_masks() {
    let mut invalid_project = Project::default();
    invalid_project.compositions[0]
        .layers
        .push(crate::core::timeline::Layer::new(
            "invalid-video".into(),
            "Invalid Video".into(),
            LayerType::Video {
                source: "video.mp4".into(),
                frames_dir: "frames".into(),
                frame_count: 0,
                audio_wav: None,
                speed: 1.0,
            },
            300,
        ));
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    invalid_project.compositions[0]
        .layers
        .push(crate::core::timeline::Layer::new(
            "empty-video-audio".into(),
            "Empty Video Audio".into(),
            LayerType::Video {
                source: "video.mp4".into(),
                frames_dir: "frames".into(),
                frame_count: 1,
                audio_wav: Some("  ".into()),
                speed: 1.0,
            },
            300,
        ));
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    let duplicate_camera = invalid_project.compositions[0].active_camera.clone();
    invalid_project.compositions[0]
        .cameras
        .push(duplicate_camera);
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    invalid_project.compositions[0].active_camera.fov_degrees = f32::NAN;
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    for track in 0..3 {
        let mut invalid_project = Project::default();
        let camera = &mut invalid_project.compositions[0].active_camera;
        let bad =
            Animatable::new_animated(vec![Keyframe::new(12, f32::NAN, InterpolationType::Linear)]);
        match track {
            0 => camera.fov_animation = Some(bad.clone()),
            1 => camera.focus_distance_animation = Some(bad.clone()),
            _ => camera.aperture_animation = Some(bad),
        }
        assert!(
            ProductionDocument::new(invalid_project).validate().is_err(),
            "camera lens track {track} must reject non-finite values"
        );
    }

    let mut invalid_project = Project::default();
    invalid_project.compositions[0]
        .lights
        .push(crate::core::timeline::Light3D {
            id: "bad-light".into(),
            name: "Bad Light".into(),
            intensity: -1.0,
            ..Default::default()
        });
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    let duration_frames = invalid_project.compositions[0].duration_frames;
    invalid_project.compositions[0]
        .markers
        .push(crate::core::timeline::TimelineMarker {
            frame: duration_frames,
            label: "Marker".into(),
            color: [1.1, 0.0, 0.0],
        });
    invalid_project.compositions[0]
        .markers
        .push(crate::core::timeline::TimelineMarker {
            frame: 0,
            label: "x".repeat(4_097),
            color: [0.0, 0.0, 0.0],
        });
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    invalid_project.compositions[0]
        .markers
        .push(crate::core::timeline::TimelineMarker {
            frame: 0,
            label: "cut\n01".into(),
            color: [0.0, 0.0, 0.0],
        });
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    let mut invalid_mask = crate::core::mask::Mask::new_rect(
        "invalid-mask".into(),
        "Mask".into(),
        0.0,
        0.0,
        100.0,
        100.0,
    );
    if let crate::core::property::Animatable::Constant(vertices) = &mut invalid_mask.path.vertices {
        vertices[0][0] = f32::NAN;
    }
    invalid_project.compositions[0].layers[0]
        .masks
        .push(invalid_mask);
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    let mut invalid_mask = crate::core::mask::Mask::new_rect(
        "invalid-mask-animation".into(),
        "Mask".into(),
        0.0,
        0.0,
        100.0,
        100.0,
    );
    invalid_mask.opacity = crate::core::property::Animatable::new_constant(f32::NAN);
    invalid_project.compositions[0].layers[0]
        .masks
        .push(invalid_mask);
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    let mut invalid_mask = crate::core::mask::Mask::new_rect(
        "invalid-mask-opacity".into(),
        "Mask".into(),
        0.0,
        0.0,
        100.0,
        100.0,
    );
    invalid_mask.opacity = crate::core::property::Animatable::new_constant(-0.1);
    invalid_project.compositions[0].layers[0]
        .masks
        .push(invalid_mask);
    assert!(ProductionDocument::new(invalid_project).validate().is_err());
}
