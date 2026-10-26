#[test]
fn test_dense_flow_bounds_allocation_and_rejects_oversized_compute() {
    let field = DenseFlowField::new(u32::MAX, u32::MAX);
    assert!(field.vectors.is_empty());
    assert_eq!(field.get(0, 0), [0.0, 0.0]);
    let flow = compute_dense_optical_flow(&[0; 4], &[0; 4], u32::MAX, u32::MAX, 1, 1);
    assert!(flow.vectors.is_empty());
}
#[test]
fn test_dense_optical_flow_translation() {
    let width = 16u32;
    let height = 16u32;
    let mut frame_a = vec![0u8; (width * height * 4) as usize];
    let mut frame_b = vec![0u8; (width * height * 4) as usize];

    // Draw a 4x4 white patch moving from (4, 4) to (6, 4)
    for y in 4..8 {
        for x in 4..8 {
            let idx = (y * width + x) as usize * 4;
            frame_a[idx] = 255;
            frame_a[idx + 1] = 255;
            frame_a[idx + 2] = 255;
            frame_a[idx + 3] = 255;
        }
    }

    for y in 4..8 {
        for x in 6..10 {
            let idx = (y * width + x) as usize * 4;
            frame_b[idx] = 255;
            frame_b[idx + 1] = 255;
            frame_b[idx + 2] = 255;
            frame_b[idx + 3] = 255;
        }
    }

    let flow = compute_dense_optical_flow(&frame_a, &frame_b, width, height, 1, 3);
    let center_vec = flow.get(5, 5);
    assert_eq!(center_vec[0], 2.0);
    assert_eq!(center_vec[1], 0.0);
}
#[test]
fn markerless_tracking_follows_translation_and_bakes_keyframes() {
    let (w, h) = (16u32, 16u32);
    let make_frame = |offset: u32| {
        let mut frame = vec![0u8; (w * h * 4) as usize];
        for y in 4..8 {
            for x in (4 + offset)..(8 + offset) {
                let i = (y * w + x) as usize * 4;
                frame[i..i + 4].copy_from_slice(&[255, 255, 255, 255]);
            }
        }
        frame
    };
    let frames = [make_frame(0), make_frame(2), make_frame(4)];
    let refs = frames.iter().map(Vec::as_slice).collect::<Vec<_>>();
    let tracks = track_markerless_motion(&refs, w, h, &[[5.0, 5.0]], 1, 3);
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].samples.len(), 3);
    assert!(tracks[0].samples[1].position[0] >= 6.0);
    assert!(tracks[0]
        .samples
        .iter()
        .all(|s| (0.0..=1.0).contains(&s.confidence)));
    let keyframes = markerless_track_keyframes(&tracks[0]);
    assert_eq!(keyframes.keyframes().map(|k| k.len()), Some(3));
}
#[test]
fn markerless_streaming_matches_in_memory_tracking_and_pose() {
    let (w, h) = (16u32, 16u32);
    let make_frame = |offset: u32| {
        let mut frame = vec![0u8; (w * h * 4) as usize];
        for y in 4..8 {
            for x in (4 + offset)..(8 + offset) {
                let i = (y * w + x) as usize * 4;
                frame[i..i + 4].copy_from_slice(&[255, 255, 255, 255]);
            }
        }
        frame
    };
    let frames = [make_frame(0), make_frame(2), make_frame(4)];
    let refs = frames.iter().map(Vec::as_slice).collect::<Vec<_>>();
    let seeds = [[5.0, 5.0], [f32::NAN, 1.0]];
    let expected = track_markerless_motion(&refs, w, h, &seeds, 1, 3);
    let streamed =
        track_markerless_motion_stream(frames.iter().cloned().map(Ok), w, h, &seeds, 1, 3).unwrap();
    assert_eq!(streamed, expected);

    let expected_pose = estimate_markerless_pose(&refs, w, h, 1, 2, 1, 3);
    let pose_seeds = detect_markerless_features(&frames[0], w, h, 1, 2);
    let streamed_pose = estimate_markerless_pose_from_tracks(&pose_seeds, &streamed, w, h);
    assert_eq!(
        markerless_pose_to_json(&streamed_pose).unwrap(),
        markerless_pose_to_json(&expected_pose).unwrap()
    );
}
#[test]
fn markerless_streaming_reports_decode_and_dimension_errors() {
    let valid = vec![0u8; 4 * 4 * 4];
    assert!(track_markerless_motion_stream(
        [Ok(valid.clone()), Err("decode failed".to_string())],
        4,
        4,
        &[[1.0, 1.0]],
        1,
        1
    )
    .unwrap_err()
    .contains("decode failed"));
    assert!(track_markerless_motion_stream(
        [Ok(valid.clone()), Ok(vec![0; 3])],
        4,
        4,
        &[[f32::NAN, 1.0]],
        1,
        1
    )
    .unwrap_err()
    .contains("does not match"));
}
#[test]
fn markerless_tracking_rejects_bad_frames_and_nonfinite_seeds() {
    let valid = vec![0u8; 4 * 4 * 4];
    let invalid = vec![0u8; 3];
    assert!(track_markerless_motion(
        &[valid.as_slice(), invalid.as_slice()],
        4,
        4,
        &[[1.0, 1.0]],
        1,
        1
    )
    .is_empty());
    assert!(track_markerless_motion(
        &[valid.as_slice()],
        4,
        4,
        &[[f32::NAN, 1.0], [f32::INFINITY, 2.0]],
        1,
        1
    )
    .is_empty());
}
#[test]
fn markerless_smoothing_reduces_low_confidence_spike() {
    let track = MarkerlessMotionTrack {
        samples: vec![
            MarkerlessMotionSample {
                frame: 0,
                position: [0.0, 0.0],
                confidence: 1.0,
            },
            MarkerlessMotionSample {
                frame: 1,
                position: [10.0, 0.0],
                confidence: 0.1,
            },
            MarkerlessMotionSample {
                frame: 2,
                position: [2.0, 0.0],
                confidence: 1.0,
            },
        ],
    };
    let smoothed = smooth_markerless_track(&track, 1);
    assert!(smoothed.samples[1].position[0] < 6.0);
    assert_eq!(smoothed.samples[0].frame, 0);
    assert_eq!(smoothed.samples[2].frame, 2);
}
#[test]
fn pose_track_uses_common_frames_and_computes_bones() {
    let make = |offset: f32| MarkerlessMotionTrack {
        samples: vec![
            MarkerlessMotionSample {
                frame: 0,
                position: [offset, 0.0],
                confidence: 1.0,
            },
            MarkerlessMotionSample {
                frame: 1,
                position: [offset + 1.0, 0.0],
                confidence: 0.8,
            },
        ],
    };
    let pose = build_markerless_pose_track(&[make(0.0), make(3.0)], &[(0, 1)]);
    assert_eq!(pose.frames.len(), 2);
    assert_eq!(pose.frames[0].root, [1.5, 0.0]);
    assert!((pose.bone_lengths[0] - 3.0).abs() < 0.001);
    assert!((pose.frames[1].confidence - 0.8).abs() < 0.001);
    let json = markerless_pose_to_json(&pose).unwrap();
    assert!(json.contains("bone_lengths"));
    let restored: MarkerlessPoseTrack = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, pose);
    let named = name_markerless_pose_track(&pose, &["hip".into(), "hand".into()]);
    let csv = markerless_pose_to_csv(&named);
    assert!(csv.lines().next().unwrap().contains("hip_x"));
    assert_eq!(csv.lines().count(), 3);
    assert_eq!(
        normalize_joint_name(" Left Shoulder-01 "),
        "left_shoulder_01"
    );
    assert_eq!(standard_humanoid_joint_names(2), vec!["hip", "spine"]);
    assert_eq!(standard_humanoid_bones(3), vec![[0, 1], [1, 2]]);
    let auto_named = name_and_connect_markerless_pose_track(&pose);
    assert_eq!(auto_named.joint_names, vec!["hip", "spine"]);
    let bvh = markerless_pose_to_bvh(&named, 0.0);
    assert!(bvh.contains("HIERARCHY"));
    assert!(bvh.contains("Frames: 2"));
    assert!(bvh.contains("Frame Time:"));
}
#[test]
fn pose_repair_interpolates_short_occlusion_and_lowers_confidence() {
    let mut pose = MarkerlessPoseTrack {
        frames: vec![
            MarkerlessPoseFrame {
                frame: 0,
                joints: vec![[0.0, 0.0]],
                root: [0.0, 0.0],
                confidence: 1.0,
            },
            MarkerlessPoseFrame {
                frame: 1,
                joints: vec![[f32::NAN, f32::NAN]],
                root: [0.0, 0.0],
                confidence: 1.0,
            },
            MarkerlessPoseFrame {
                frame: 2,
                joints: vec![[2.0, 0.0]],
                root: [2.0, 0.0],
                confidence: 1.0,
            },
        ],
        bones: Vec::new(),
        bone_lengths: Vec::new(),
    };
    assert_eq!(repair_markerless_pose_track(&mut pose, 1), 1);
    assert_eq!(pose.frames[1].joints[0], [1.0, 0.0]);
    assert!(pose.frames[1].confidence < 1.0);
}
#[test]
fn pose_repair_uses_frame_numbers_for_sparse_sequences() {
    let mut pose = MarkerlessPoseTrack {
        frames: vec![
            MarkerlessPoseFrame {
                frame: 10,
                joints: vec![[0.0, 0.0]],
                root: [0.0, 0.0],
                confidence: 1.0,
            },
            MarkerlessPoseFrame {
                frame: 12,
                joints: vec![[f32::NAN, f32::NAN]],
                root: [0.0, 0.0],
                confidence: 1.0,
            },
            MarkerlessPoseFrame {
                frame: 14,
                joints: vec![[8.0, 0.0]],
                root: [8.0, 0.0],
                confidence: 1.0,
            },
        ],
        bones: vec![],
        bone_lengths: vec![],
    };
    assert_eq!(repair_markerless_pose_track(&mut pose, 3), 1);
    assert_eq!(pose.frames[1].joints[0], [4.0, 0.0]);
}
#[test]
fn pose_repair_rejects_non_monotonic_frame_numbers() {
    let mut pose = MarkerlessPoseTrack {
        frames: vec![
            MarkerlessPoseFrame {
                frame: 10,
                joints: vec![[0.0, 0.0]],
                root: [0.0, 0.0],
                confidence: 1.0,
            },
            MarkerlessPoseFrame {
                frame: 9,
                joints: vec![[f32::NAN, f32::NAN]],
                root: [0.0, 0.0],
                confidence: 1.0,
            },
            MarkerlessPoseFrame {
                frame: 8,
                joints: vec![[8.0, 0.0]],
                root: [8.0, 0.0],
                confidence: 1.0,
            },
        ],
        bones: vec![],
        bone_lengths: vec![],
    };
    assert_eq!(repair_markerless_pose_track(&mut pose, 10), 0);
    assert!(pose.frames[1].joints[0][0].is_nan());
}
#[test]
fn feature_detection_is_bounded_spaced_and_deterministic() {
    let (w, h) = (12u32, 12u32);
    let mut frame = vec![0u8; (w * h * 4) as usize];
    for &(x, y) in &[(2, 2), (8, 3), (4, 9)] {
        let i = (y * w + x) as usize * 4;
        frame[i..i + 4].copy_from_slice(&[255, 255, 255, 255]);
    }
    let first = detect_markerless_features(&frame, w, h, 4, 2);
    let second = detect_markerless_features(&frame, w, h, 4, 2);
    assert_eq!(first, second);
    assert!(first.len() <= 4);
    assert!(first
        .iter()
        .all(|p| p[0] >= 1.0 && p[0] < 11.0 && p[1] >= 1.0 && p[1] < 11.0));
    let refs = [&frame[..]];
    assert_eq!(
        track_markerless_auto(&refs, w, h, 2, 2, 1, 1).len(),
        first.len().min(2)
    );
    let pose = estimate_markerless_pose(&refs, w, h, 4, 2, 1, 1);
    assert_eq!(pose.frames.len(), 1);
    assert_eq!(pose.bones.len(), 16);
    let assignments = assign_features_to_humanoid(&first, w, h);
    assert_eq!(assignments.len(), 17);
    let assigned = assignments
        .iter()
        .flatten()
        .copied()
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(
        assigned.len(),
        assignments.iter().filter(|value| value.is_some()).count()
    );
}
#[test]
fn motion_outlier_is_clamped_and_marked_low_confidence() {
    let mut track = MarkerlessMotionTrack {
        samples: vec![
            MarkerlessMotionSample {
                frame: 0,
                position: [0.0, 0.0],
                confidence: 1.0,
            },
            MarkerlessMotionSample {
                frame: 1,
                position: [100.0, 0.0],
                confidence: 1.0,
            },
        ],
    };
    assert_eq!(reject_markerless_motion_outliers(&mut track, 5.0), 1);
    assert_eq!(track.samples[1].position, [5.0, 0.0]);
    assert!(track.samples[1].confidence < 0.5);
    assert!(redetect_markerless_feature(&[0u8; 4 * 4 * 4], 4, 4, [1.0, 1.0], 2).is_none());
}
