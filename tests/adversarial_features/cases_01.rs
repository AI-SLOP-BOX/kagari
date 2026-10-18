#[test]
fn lightning_is_deterministic_for_same_seed() {
    let config = AdvancedLightningConfig::default();
    let a = generate_lightning_arcs(&config);
    let b = generate_lightning_arcs(&config);
    assert_eq!(a.len(), b.len());
    assert_eq!(a[0].points, b[0].points);
}
#[test]
fn lightning_rejects_non_finite_configuration_output() {
    let config = AdvancedLightningConfig {
        displacement_amplitude: f32::NAN,
        ..Default::default()
    };
    assert!(finite_points(&generate_lightning_arcs(&config)));
}
#[test]
fn laser_zero_length_is_empty() {
    let config = LaserBeamConfig {
        start_point: [4.0, 4.0],
        end_point: [4.0, 4.0],
        ..Default::default()
    };
    assert!(evaluate_laser_beam_segment(&config).is_none());
}
#[test]
fn particles_are_finite_after_extreme_but_valid_step() {
    let mut sim = ParticleSimulation3D::new();
    let config = EmitterConfig3D {
        birth_rate_per_sec: 20.0,
        initial_speed: 100.0,
        gravity: [0.0, -1000.0, 0.0],
        ..Default::default()
    };
    sim.update(0.25, &config);
    assert!(sim.particles.iter().all(|p| {
        p.position
            .iter()
            .chain(p.velocity.iter())
            .all(|v| v.is_finite())
    }));
}
#[test]
fn particles_with_zero_lifespan_do_not_survive() {
    let mut sim = ParticleSimulation3D::new();
    let config = EmitterConfig3D {
        lifespan_sec: 0.0,
        birth_rate_per_sec: 100.0,
        ..Default::default()
    };
    sim.update(0.1, &config);
    assert!(sim.particles.is_empty());
}
#[test]
fn particles_with_zero_length_collision_normal_are_safe() {
    let mut sim = ParticleSimulation3D::new();
    let config = EmitterConfig3D {
        birth_rate_per_sec: 1.0,
        initial_speed: 0.0,
        collision_planes: vec![CollisionPlane {
            origin: [0.0; 3],
            normal: [0.0; 3],
            bounce_restitution: 1.0,
            friction: 0.0,
        }],
        ..Default::default()
    };
    sim.update(1.0, &config);
    assert!(sim
        .particles
        .iter()
        .all(|p| p.position.iter().all(|v| v.is_finite())));
}
#[test]
fn obj_quad_triangulation_has_valid_indices() {
    let mesh = parse_obj_str("v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3 4\n").unwrap();
    assert_eq!(mesh.indices.len(), 6);
    assert!(mesh
        .indices
        .iter()
        .all(|i| (*i as usize) < mesh.vertices.len()));
}
#[test]
fn obj_malformed_numeric_vertex_is_rejected() {
    assert!(parse_obj_str("v nope 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n").is_err());
}
#[test]
fn color_conversion_is_monotonic() {
    let mut previous = 0.0;
    for value in 0..=255u8 {
        let src = [value, value, value, 255];
        let mut linear = [0.0; 4];
        convert_rgba8_to_rgba32f(&src, &mut linear);
        assert!(linear[0] >= previous);
        assert_eq!(linear[0], linear[1]);
        assert_eq!(linear[1], linear[2]);
        assert_eq!(linear[3], 1.0);
        previous = linear[0];
    }
}
#[test]
fn color_conversion_rejects_nan_on_output() {
    let mut out = [0u8; 3];
    convert_rgba32f_to_rgba8(&[f32::NAN, f32::INFINITY, f32::NEG_INFINITY], &mut out);
    assert_eq!(out, [0, 255, 0]);
}
#[test]
fn roto_empty_frame_has_expected_size() {
    let result = generate_rotobrush_matte(&[], 0, 0, &[], &Default::default());
    assert!(result.is_empty());
}
#[test]
fn roto_output_is_binary_for_binary_input() {
    let pixels = vec![0u8; 4 * 4 * 4];
    let stroke = RotoStroke {
        stroke_type: RotoStrokeType::Background,
        points: vec![[1.0, 1.0]],
        radius: 1.0,
        frame: None,
    };
    let result = generate_rotobrush_matte(&pixels, 4, 4, &[stroke], &Default::default());
    assert_eq!(result.len(), 16);
    assert!(result.iter().all(|v| *v == 0 || *v == 255));
}
#[test]
fn polygon_intersection_is_inside_both_inputs() {
    let a = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
    let b = vec![[5.0, 5.0], [15.0, 5.0], [15.0, 15.0], [5.0, 15.0]];
    let result = polygon_intersect(&a, &b);
    assert!(result.iter().all(|p| p
        .iter()
        .all(|q| point_in_polygon(q[0], q[1], &a) && point_in_polygon(q[0], q[1], &b))));
}
#[test]
fn polygon_subtract_does_not_fill_cutout() {
    let a = vec![[0.0, 0.0], [20.0, 0.0], [20.0, 20.0], [0.0, 20.0]];
    let b = vec![[5.0, 5.0], [15.0, 5.0], [15.0, 15.0], [5.0, 15.0]];
    let result = polygon_subtract(&a, &b);
    assert!(!result.iter().any(|p| point_in_polygon(10.0, 10.0, p)));
}
#[test]
fn audio_keyframes_have_one_entry_per_requested_frame() {
    let options = AudioKeyframeOptions::default();
    let result = extract_multiband_audio_keyframes(&[0.2; 100], 48_000, 24, 17, &options);
    assert_eq!(result.master.len(), 17);
    assert_eq!(result.bass.len(), 17);
    assert_eq!(result.mid.len(), 17);
    assert_eq!(result.treble.len(), 17);
}
#[test]
fn motion_blur_weights_are_normalized_and_times_are_finite() {
    let settings = SubframeMotionBlurSettings::default();
    let samples = evaluate_subframe_samples(100, 30, &settings);
    let sum: f32 = samples.iter().map(|s| s.weight).sum();
    assert!((sum - 1.0).abs() < 1e-5);
    assert!(samples
        .iter()
        .all(|s| s.subframe_time_sec.is_finite() && s.weight.is_finite()));
}
#[test]
fn motion_blur_negative_weights_cannot_darken_or_invert_pixels() {
    let mut output = vec![0u8; 4];
    accumulate_motion_blur_buffers(&[(&[100, 100, 100, 255], -1.0)], 1, 1, &mut output);
    assert_eq!(output, vec![0, 0, 0, 0]);
}
#[test]
fn motion_blur_zero_fps_does_not_create_infinite_time() {
    let samples = evaluate_subframe_samples(1, 0, &SubframeMotionBlurSettings::default());
    assert!(samples.iter().all(|s| s.subframe_time_sec.is_finite()));
}
#[test]
fn bend_zero_amount_preserves_every_pixel_including_edges() {
    let w = 5u32;
    let h = 5u32;
    let src: Vec<u8> = (0..w * h * 4).map(|v| (v % 251) as u8).collect();
    let mut dst = vec![0u8; src.len()];
    apply_bend_warp(
        &src,
        w,
        h,
        &mut dst,
        &BendWarpConfig {
            bend_amount: 0.0,
            ..Default::default()
        },
    );
    assert_eq!(dst, src);
}
#[test]
fn bend_warp_rejects_mismatched_buffers_without_writing() {
    let src = vec![7u8; 16];
    let mut dst = vec![9u8; 15];
    apply_bend_warp(&src, 2, 2, &mut dst, &BendWarpConfig::default());
    assert!(dst.iter().all(|v| *v == 9));
}
#[test]
fn bend_warp_output_is_finite_and_same_size_for_all_modes() {
    for warp_type in [BendWarpType::Bend, BendWarpType::Pinch, BendWarpType::Twist] {
        let src = vec![128u8; 16 * 16 * 4];
        let mut dst = vec![0u8; src.len()];
        apply_bend_warp(
            &src,
            16,
            16,
            &mut dst,
            &BendWarpConfig {
                warp_type,
                ..Default::default()
            },
        );
        assert_eq!(dst.len(), src.len());
        assert_eq!(dst.len(), 16 * 16 * 4);
    }
}
#[test]
fn lightning_branch_probability_is_bounded() {
    let config = AdvancedLightningConfig {
        branch_probability: 1000.0,
        ..Default::default()
    };
    let branches = generate_lightning_arcs(&config);
    assert!(branches.len() <= config.segments.max(1));
}
#[test]
fn particle_update_with_negative_dt_does_not_age_particles_backwards() {
    let mut sim = ParticleSimulation3D::new();
    let config = EmitterConfig3D {
        birth_rate_per_sec: 10.0,
        ..Default::default()
    };
    sim.update(0.1, &config);
    let before = sim.particles.iter().map(|p| p.age_sec).collect::<Vec<_>>();
    sim.update(-1.0, &config);
    assert!(sim
        .particles
        .iter()
        .enumerate()
        .all(|(i, p)| p.age_sec >= before.get(i).copied().unwrap_or(0.0)));
}
#[test]
fn obj_parser_preserves_uv_and_normal_cardinality() {
    let obj = "v 0 0 0\nv 1 0 0\nv 0 1 0\nvt 0 0\nvt 1 0\nvt 0 1\nvn 0 0 1\nf 1/1/1 2/2/1 3/3/1\n";
    let mesh = parse_obj_str(obj).unwrap();
    assert_eq!(mesh.uvs.len(), mesh.vertices.len());
    assert_eq!(mesh.normals.len(), mesh.vertices.len());
}
#[test]
fn lightning_seeds_produce_repeatable_but_distinct_paths() {
    let a = generate_lightning_arcs(&AdvancedLightningConfig {
        seed: 1,
        ..Default::default()
    });
    let b = generate_lightning_arcs(&AdvancedLightningConfig {
        seed: 2,
        ..Default::default()
    });
    assert_ne!(a[0].points, b[0].points);
    assert_eq!(
        a[0].points,
        generate_lightning_arcs(&AdvancedLightningConfig {
            seed: 1,
            ..Default::default()
        })[0]
            .points
    );
}
#[test]
fn laser_segment_stays_between_endpoints() {
    let config = LaserBeamConfig {
        start_point: [-10.0, 5.0],
        end_point: [90.0, 25.0],
        time_progress: 0.7,
        beam_length_percent: 30.0,
        ..Default::default()
    };
    let (tail, head, _, _) = evaluate_laser_beam_segment(&config).unwrap();
    for p in [tail, head] {
        let t = (p[0] - config.start_point[0]) / (config.end_point[0] - config.start_point[0]);
        assert!((0.0..=1.0).contains(&t));
        assert!((p[1] - (config.start_point[1] + t * 20.0)).abs() < 1e-4);
    }
}
#[test]
fn particle_count_is_bounded_for_multiple_small_steps() {
    let mut sim = ParticleSimulation3D::new();
    let config = EmitterConfig3D {
        birth_rate_per_sec: 100.0,
        lifespan_sec: 0.2,
        ..Default::default()
    };
    for _ in 0..200 {
        sim.update(0.01, &config);
    }
    assert!(sim.particles.len() <= 100);
}
#[test]
fn particle_sphere_emitter_positions_are_within_radius() {
    let mut sim = ParticleSimulation3D::new();
    let config = EmitterConfig3D {
        emitter_type: EmitterType3D::Sphere { radius: 10.0 },
        birth_rate_per_sec: 100.0,
        initial_speed: 0.0,
        gravity: [0.0; 3],
        ..Default::default()
    };
    sim.update(0.2, &config);
    assert!(sim.particles.iter().all(|p| {
        p.position
            .iter()
            .zip(config.position.iter())
            .map(|(a, b)| (a - b) * (a - b))
            .sum::<f32>()
            <= 100.001
    }));
}
#[test]
fn obj_empty_and_comments_are_safe() {
    let mesh = parse_obj_str("# only comments\n\n").unwrap();
    assert!(mesh.vertices.is_empty());
    assert!(mesh.indices.is_empty());
    assert!(mesh.ray_intersect([0.0; 3], [1.0, 0.0, 0.0]).is_none());
}
#[test]
fn color_16_bit_expansion_is_exact() {
    let src = [0u8, 1, 2, 127, 128, 254, 255];
    let mut wide = [0u16; 7];
    let mut back = [0u8; 7];
    convert_rgba8_to_rgba16(&src, &mut wide);
    convert_rgba16_to_rgba8(&wide, &mut back);
    assert_eq!(back, src);
    assert_eq!(wide, [0, 257, 514, 32639, 32896, 65278, 65535]);
}
#[test]
fn roto_matte_is_deterministic() {
    let pixels = (0..(16 * 16 * 4))
        .map(|i| (i % 251) as u8)
        .collect::<Vec<_>>();
    let strokes = vec![RotoStroke {
        stroke_type: RotoStrokeType::Foreground,
        points: vec![[4.0, 4.0], [8.0, 8.0]],
        radius: 2.0,
        frame: None,
    }];
    let settings = RotoBrushSettings::default();
    assert_eq!(
        generate_rotobrush_matte(&pixels, 16, 16, &strokes, &settings),
        generate_rotobrush_matte(&pixels, 16, 16, &strokes, &settings)
    );
}
#[test]
fn motion_blur_single_buffer_is_identity() {
    let src = vec![37u8; 4 * 4 * 4];
    let mut out = vec![0u8; src.len()];
    accumulate_motion_blur_buffers(&[(&src, 1.0)], 4, 4, &mut out);
    assert_eq!(out, src);
}
#[test]
fn bend_warp_handles_all_small_dimensions() {
    for (w, h) in [(1, 1), (1, 8), (8, 1), (2, 2), (3, 5)] {
        let src = vec![128u8; w * h * 4];
        let mut dst = vec![0u8; src.len()];
        apply_bend_warp(
            &src,
            w as u32,
            h as u32,
            &mut dst,
            &BendWarpConfig::default(),
        );
        assert_eq!(dst.len(), src.len());
    }
}
#[test]
fn empty_audio_returns_empty_keyframe_sets() {
    let result =
        extract_multiband_audio_keyframes(&[], 48_000, 24, 10, &AudioKeyframeOptions::default());
    assert!(result.master.is_empty());
    assert!(result.bass.is_empty());
    assert!(result.mid.is_empty());
    assert!(result.treble.is_empty());
}
#[test]
fn audio_keyframe_values_stay_inside_multiplier_range() {
    let options = AudioKeyframeOptions {
        multiplier: 73.0,
        ..Default::default()
    };
    let result = extract_multiband_audio_keyframes(&[1.0; 4096], 48_000, 24, 10, &options);
    for values in [&result.master, &result.bass, &result.mid, &result.treble] {
        assert!(values.iter().all(|kf| (0.0..=73.0).contains(&kf.value)));
    }
}
#[test]
fn motion_blur_buffer_size_mismatch_is_noop() {
    let mut out = vec![11u8; 16];
    accumulate_motion_blur_buffers(&[(&[1u8; 3], 1.0)], 2, 2, &mut out);
    assert_eq!(out, vec![0u8; 16]);
}
#[test]
fn motion_blur_ignores_invalid_samples_and_keeps_valid_result() {
    let valid = vec![80u8; 4];
    let invalid = vec![200u8; 3];
    let mut out = vec![0u8; 4];
    accumulate_motion_blur_buffers(&[(&invalid, 1.0), (&valid, 1.0)], 1, 1, &mut out);
    assert_eq!(out, valid);
}
