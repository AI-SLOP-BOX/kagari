#[test]
fn lightning_output_has_endpoints_for_extreme_segment_settings() {
    for segments in [0, 1, 2, 128, 10_000] {
        let config = AdvancedLightningConfig {
            segments,
            ..Default::default()
        };
        let branches = generate_lightning_arcs(&config);
        assert!(!branches.is_empty());
        assert_eq!(branches[0].points.first(), Some(&config.origin));
        assert_eq!(branches[0].points.last(), Some(&config.destination));
    }
}
#[test]
fn particle_zero_dt_does_not_change_existing_state() {
    let mut sim = ParticleSimulation3D::new();
    let config = EmitterConfig3D {
        birth_rate_per_sec: 10.0,
        ..Default::default()
    };
    sim.update(0.1, &config);
    let before = sim.particles.clone();
    sim.update(0.0, &config);
    assert_eq!(sim.particles.len(), before.len());
    assert!(sim
        .particles
        .iter()
        .zip(before.iter())
        .all(|(a, b)| a.position == b.position && a.age_sec == b.age_sec));
}
#[test]
fn particle_restitution_and_friction_are_clamped() {
    let mut sim = ParticleSimulation3D::new();
    let config = EmitterConfig3D {
        birth_rate_per_sec: 10.0,
        collision_planes: vec![CollisionPlane {
            origin: [0.0; 3],
            normal: [0.0, 1.0, 0.0],
            bounce_restitution: 99.0,
            friction: -99.0,
        }],
        ..Default::default()
    };
    sim.update(0.1, &config);
    assert!(sim
        .particles
        .iter()
        .all(|p| p.velocity.iter().all(|v| v.is_finite())));
}
#[test]
fn obj_normals_are_unit_length_when_generated() {
    let mesh = parse_obj_str("v 0 0 0\nv 3 0 0\nv 0 4 0\nf 1 2 3\n").unwrap();
    assert!(mesh.normals.iter().all(|n| {
        let length = n.iter().map(|v| v * v).sum::<f32>().sqrt();
        (length - 1.0).abs() < 1e-4
    }));
}
#[test]
fn roto_mismatched_input_returns_exact_expected_length() {
    let result = generate_rotobrush_matte(&[1, 2, 3], 10, 10, &[], &Default::default());
    assert_eq!(result.len(), 100);
    assert!(result.iter().all(|v| *v == 0));
}
#[test]
fn bend_warp_mismatched_output_does_not_partially_write() {
    let src = vec![1u8; 4 * 4 * 4];
    let mut dst = vec![77u8; 4 * 4 * 4 - 1];
    apply_bend_warp(&src, 4, 4, &mut dst, &BendWarpConfig::default());
    assert!(dst.iter().all(|v| *v == 77));
}

macro_rules! contract_test {
    ($name:ident, $body:block) => {
        #[test]
        fn $name() $body
    };
}

contract_test!(lightning_default_has_main_branch, {
    assert!(!generate_lightning_arcs(&AdvancedLightningConfig::default()).is_empty());
});
contract_test!(lightning_negative_amplitude_stays_finite, {
    let c = AdvancedLightningConfig {
        displacement_amplitude: -100.0,
        ..Default::default()
    };
    assert!(finite_points(&generate_lightning_arcs(&c)));
});
contract_test!(lightning_zero_branch_probability_has_one_branch, {
    let c = AdvancedLightningConfig {
        branch_probability: 0.0,
        ..Default::default()
    };
    assert_eq!(generate_lightning_arcs(&c).len(), 1);
});
contract_test!(laser_start_progress_has_nonnegative_length, {
    let c = LaserBeamConfig {
        time_progress: 0.0,
        ..Default::default()
    };
    let (tail, head, _, _) = evaluate_laser_beam_segment(&c).unwrap();
    assert!(head != tail);
});
contract_test!(laser_end_progress_has_nonnegative_length, {
    let c = LaserBeamConfig {
        time_progress: 1.0,
        ..Default::default()
    };
    let (tail, head, _, _) = evaluate_laser_beam_segment(&c).unwrap();
    assert!(head[0] >= tail[0]);
});
contract_test!(motion_blur_sample_count_is_clamped_low, {
    let c = SubframeMotionBlurSettings {
        samples_per_frame: 0,
        ..Default::default()
    };
    assert_eq!(evaluate_subframe_samples(0, 24, &c).len(), 1);
});
contract_test!(motion_blur_sample_count_is_clamped_high, {
    let c = SubframeMotionBlurSettings {
        samples_per_frame: usize::MAX,
        ..Default::default()
    };
    assert_eq!(evaluate_subframe_samples(0, 24, &c).len(), 64);
});
contract_test!(motion_blur_zero_shutter_has_equal_time_samples, {
    let c = SubframeMotionBlurSettings {
        shutter_angle_deg: 0.0,
        ..Default::default()
    };
    let s = evaluate_subframe_samples(2, 30, &c);
    assert!(s
        .windows(2)
        .all(|w| w[0].subframe_time_sec == w[1].subframe_time_sec));
});
contract_test!(bend_empty_buffers_are_safe, {
    let mut dst = Vec::new();
    apply_bend_warp(&[], 0, 0, &mut dst, &BendWarpConfig::default());
    assert!(dst.is_empty());
});
contract_test!(bend_negative_amount_is_supported, {
    let src = vec![10u8; 8 * 8 * 4];
    let mut dst = vec![0u8; src.len()];
    apply_bend_warp(
        &src,
        8,
        8,
        &mut dst,
        &BendWarpConfig {
            bend_amount: -100.0,
            ..Default::default()
        },
    );
    assert_eq!(dst.len(), src.len());
});
contract_test!(bend_large_amount_does_not_change_buffer_size, {
    let src = vec![10u8; 8 * 8 * 4];
    let mut dst = vec![0u8; src.len()];
    apply_bend_warp(
        &src,
        8,
        8,
        &mut dst,
        &BendWarpConfig {
            bend_amount: 1000.0,
            ..Default::default()
        },
    );
    assert_eq!(dst.len(), src.len());
});
contract_test!(particles_empty_step_stays_empty, {
    let mut s = ParticleSimulation3D::new();
    s.update(0.0, &EmitterConfig3D::default());
    assert!(s.particles.is_empty());
});
contract_test!(particles_box_emitter_is_deterministic_from_new_state, {
    let c = EmitterConfig3D {
        emitter_type: EmitterType3D::Box { size: [10.0; 3] },
        birth_rate_per_sec: 20.0,
        ..Default::default()
    };
    let mut a = ParticleSimulation3D::new();
    let mut b = ParticleSimulation3D::new();
    a.update(0.2, &c);
    b.update(0.2, &c);
    assert_eq!(a.particles[0].position, b.particles[0].position);
});
contract_test!(particles_negative_lifespan_does_not_survive, {
    let mut s = ParticleSimulation3D::new();
    s.update(
        0.1,
        &EmitterConfig3D {
            lifespan_sec: -1.0,
            birth_rate_per_sec: 5.0,
            ..Default::default()
        },
    );
    assert!(s.particles.is_empty());
});
contract_test!(obj_face_with_too_few_vertices_has_no_indices, {
    let m = parse_obj_str("v 0 0 0\nv 1 0 0\nf 1 2\n").unwrap();
    assert!(m.indices.is_empty());
});
contract_test!(obj_extra_face_components_are_accepted, {
    assert!(parse_obj_str("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1/1/1/extra 2/2/1 3/3/1\n").is_ok());
});
contract_test!(obj_generated_bounds_are_finite, {
    let m = parse_obj_str("v -1 -2 -3\nv 4 5 6\n").unwrap();
    assert!(m
        .bbox_min
        .iter()
        .chain(m.bbox_max.iter())
        .all(|v| v.is_finite()));
});
contract_test!(color_zero_maps_to_zero_linear, {
    let mut d = [1.0];
    convert_rgba8_to_rgba32f(&[0], &mut d);
    assert_eq!(d[0], 0.0);
});
contract_test!(color_white_maps_to_one_linear, {
    let mut d = [0.0];
    convert_rgba8_to_rgba32f(&[255], &mut d);
    assert_eq!(d[0], 1.0);
});
contract_test!(color_float_clamps_above_one, {
    let mut d = [0];
    convert_rgba32f_to_rgba8(&[2.0], &mut d);
    assert_eq!(d[0], 255);
});
contract_test!(color_float_clamps_below_zero, {
    let mut d = [0];
    convert_rgba32f_to_rgba8(&[-1.0], &mut d);
    assert_eq!(d[0], 0);
});
contract_test!(roto_no_strokes_returns_zero_matte, {
    let result = generate_rotobrush_matte(&[255; 16], 2, 2, &[], &Default::default());
    assert_eq!(result, vec![0; 4]);
});
contract_test!(roto_out_of_bounds_stroke_is_safe, {
    let stroke = RotoStroke {
        stroke_type: RotoStrokeType::Foreground,
        points: vec![[999.0, -999.0]],
        radius: 5.0,
        frame: None,
    };
    let result = generate_rotobrush_matte(&[0; 4 * 4 * 4], 4, 4, &[stroke], &Default::default());
    assert_eq!(result.len(), 16);
});
contract_test!(roto_negative_radius_is_safe, {
    let stroke = RotoStroke {
        stroke_type: RotoStrokeType::Foreground,
        points: vec![[1.0, 1.0]],
        radius: -5.0,
        frame: None,
    };
    assert_eq!(
        generate_rotobrush_matte(&[0; 4 * 4 * 4], 4, 4, &[stroke], &Default::default()).len(),
        16
    );
});
contract_test!(boolean_empty_subject_union_returns_clip, {
    let clip = vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]];
    assert_eq!(
        apply_polygon_boolean(&[], &clip, BooleanOp::Union),
        vec![clip]
    );
});
contract_test!(boolean_empty_intersection_is_empty, {
    let a = vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]];
    assert!(apply_polygon_boolean(&a, &[], BooleanOp::Intersect).is_empty());
});
contract_test!(boolean_offset_zero_is_identity, {
    let a = vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]];
    assert_eq!(offset_polygon_path(&a, 0.0), a);
});
contract_test!(audio_invalid_fps_returns_empty, {
    assert!(extract_multiband_audio_keyframes(
        &[1.0, 1.0],
        48_000,
        0,
        3,
        &AudioKeyframeOptions::default()
    )
    .master
    .is_empty());
});
contract_test!(audio_invalid_sample_rate_returns_empty, {
    assert!(extract_multiband_audio_keyframes(
        &[1.0, 1.0],
        0,
        24,
        3,
        &AudioKeyframeOptions::default()
    )
    .master
    .is_empty());
});
contract_test!(audio_keyframe_frames_are_monotonic, {
    let r = extract_multiband_audio_keyframes(
        &[0.2; 1000],
        48_000,
        24,
        8,
        &AudioKeyframeOptions::default(),
    );
    assert!(r.master.windows(2).all(|w| w[0].frame < w[1].frame));
});

contract_test!(calculations_nan_opacity_is_safe, {
    let mut pixel = [128u8, 64, 32, 255];
    let before = pixel;
    apply_calculations_composite(
        &mut pixel,
        None,
        1,
        1,
        &CalculationsConfig {
            second_layer_opacity: f32::NAN,
            ..Default::default()
        },
    );
    assert_eq!(pixel[0], before[0]);
    assert_eq!(pixel[3], before[3]);
});
contract_test!(camera_tracker_rejects_insufficient_features, {
    let features = (0..5)
        .map(|id| CameraTrackFeaturePoint {
            id: format!("feat_{id}"),
            frames: vec![0, 1],
            coords_2d: vec![[1.0, 1.0], [2.0, 2.0]],
            world_3d: [0.0, 0.0, 0.0],
            confidence: 1.0,
        })
        .collect::<Vec<_>>();
    assert!(solve_3d_camera_from_tracks(&features, 100, 100, 45.0).is_err());
});
contract_test!(camera_tracker_rejects_nonfinite_observation, {
    let features = (0..8)
        .map(|id| CameraTrackFeaturePoint {
            id: format!("feat_{id}"),
            frames: vec![0, 1],
            coords_2d: vec![[f32::NAN, 1.0], [2.0, 2.0]],
            world_3d: [0.0, 0.0, 0.0],
            confidence: 1.0,
        })
        .collect::<Vec<_>>();
    assert!(solve_3d_camera_from_tracks(&features, 100, 100, 45.0).is_err());
});
contract_test!(camera_tracker_rejects_unaligned_frame_span, {
    let features = (0..8)
        .map(|id| CameraTrackFeaturePoint {
            id: format!("feat_{id}"),
            frames: vec![0],
            coords_2d: vec![[1.0, 1.0], [2.0, 2.0]],
            world_3d: [0.0, 0.0, 0.0],
            confidence: 1.0,
        })
        .collect::<Vec<_>>();
    assert!(solve_3d_camera_from_tracks(&features, 100, 100, 45.0).is_err());
});
contract_test!(cylinder_zero_dimensions_are_safe, {
    let mut out = Vec::new();
    apply_cylinder_projection(&[], 0, 0, &mut out, &CylinderProjectionConfig::default());
    assert!(out.is_empty());
});
contract_test!(sphere_nonfinite_radius_clears_output, {
    let src = vec![255u8; 16];
    let mut out = vec![99u8; 16];
    apply_sphere_projection(
        &src,
        2,
        2,
        &mut out,
        &SphereProjectionConfig {
            radius: f32::NAN,
            ..Default::default()
        },
    );
    assert!(out.iter().all(|v| *v == 0));
});
contract_test!(optical_flow_mismatched_input_returns_zero_flow, {
    let flow = compute_dense_optical_flow(&[], &[], 4, 4, 1, 2);
    assert_eq!(flow.vectors, vec![[0.0, 0.0]; 16]);
});
contract_test!(timewarp_one_pixel_frame_is_safe, {
    let a = [0u8, 0, 0, 255];
    let b = [255u8, 255, 255, 255];
    let flow = DenseFlowField::new(1, 1);
    let mut out = [0u8; 4];
    interpolate_timewarp_frame(&a, &b, 1, 1, &flow, &flow, 0.5, &mut out);
    assert!((out[0] as i16 - 128).abs() <= 1);
    assert_eq!(out[3], 255);
});
