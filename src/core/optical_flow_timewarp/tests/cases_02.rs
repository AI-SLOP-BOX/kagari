#[test]
fn pose_motion_metrics_are_finite_and_deterministic() {
    let pose = MarkerlessPoseTrack {
        frames: vec![
            MarkerlessPoseFrame {
                frame: 0,
                joints: vec![[0.0, 0.0]],
                root: [0.0, 0.0],
                confidence: 1.0,
            },
            MarkerlessPoseFrame {
                frame: 1,
                joints: vec![[2.0, 0.0]],
                root: [2.0, 0.0],
                confidence: 1.0,
            },
            MarkerlessPoseFrame {
                frame: 2,
                joints: vec![[5.0, 0.0]],
                root: [5.0, 0.0],
                confidence: 1.0,
            },
        ],
        bones: Vec::new(),
        bone_lengths: Vec::new(),
    };
    assert_eq!(pose_joint_velocities(&pose, 2), vec![[3.0, 0.0]]);
    assert_eq!(pose_root_acceleration(&pose, 2), [1.0, 0.0]);
    assert_eq!(pose_root_acceleration(&pose, 0), [0.0, 0.0]);
    let events = detect_markerless_motion_events(&pose, 2.5, 0.5);
    assert!(events
        .iter()
        .any(|event| event.kind == MarkerlessMotionEventKind::Peak));
    assert_eq!(pose_quality(&pose, 1), (1, 1.0));
    let mut filtered = pose.clone();
    assert_eq!(filter_pose_frames_by_quality(&mut filtered, 1, 0.9), 0);
}

struct TestPoseBackend;

impl PoseInferenceBackend for TestPoseBackend {
    fn infer_joints(
        &mut self,
        _frame_rgba: &[u8],
        _width: u32,
        _height: u32,
    ) -> Vec<([f32; 2], f32)> {
        vec![
            ([2.0, 3.0], 1.4),
            ([f32::NAN, 0.0], 0.8),
            ([4.0, 5.0], -0.5),
        ]
    }
}

struct TestNormalizedPoseBackend;

impl NormalizedPoseInferenceBackend for TestNormalizedPoseBackend {
    fn infer_normalized_rgb(
        &mut self,
        rgb: &[f32],
        _width: u32,
        _height: u32,
    ) -> Vec<([f32; 2], f32)> {
        if rgb.iter().all(|value| (0.0..=1.0).contains(value)) {
            vec![([1.0, 1.0], 1.0)]
        } else {
            Vec::new()
        }
    }
}

struct TestPose3DBackend;

impl Pose3DInferenceBackend for TestPose3DBackend {
    fn infer_joints_3d(&mut self, rgb: &[f32], _width: u32, _height: u32) -> Vec<([f32; 3], f32)> {
        if rgb.iter().all(|value| (0.0..=1.0).contains(value)) {
            vec![([1.0, 2.0, 3.0], 1.2)]
        } else {
            Vec::new()
        }
    }
}
#[test]
fn pose_backend_adapter_filters_invalid_joints_and_clamps_confidence() {
    let frame = [0u8; 2 * 2 * 4];
    let refs = [&frame[..], &frame[..]];
    let mut backend = TestPoseBackend;
    let pose = estimate_pose_with_backend(&mut backend, &refs, 2, 2);
    assert_eq!(pose.frames.len(), 2);
    assert_eq!(pose.frames[0].joints[0], [2.0, 3.0]);
    assert!(pose.frames[0].joints[1].iter().all(|value| value.is_nan()));
    assert_eq!(pose.frames[0].joints[2], [4.0, 5.0]);
    assert!((pose.frames[0].confidence - (1.0 / 3.0)).abs() < 0.0001);
    assert_eq!(pose.bones, vec![[0, 1], [1, 2]]);
    assert_eq!(pose.bone_lengths, vec![0.0, 0.0]);
}
#[test]
fn pose_backend_adapter_rejects_malformed_frames_without_calling_backend() {
    let frame = vec![0u8; 3];
    let mut backend = TestPoseBackend;
    let pose = estimate_pose_with_backend(&mut backend, &[&frame], 2, 2);
    assert!(pose.frames.is_empty());
    assert!(pose.bones.is_empty());
}
#[test]
fn pose_input_conversion_drops_alpha_and_normalizes_channels() {
    let input = [0, 128, 255, 7, 255, 64, 32, 200];
    let rgb = rgba_to_normalized_rgb(&input, 2, 1).unwrap();
    assert_eq!(rgb.len(), 6);
    assert_eq!(rgb[0], 0.0);
    assert!((rgb[1] - 128.0 / 255.0).abs() < f32::EPSILON);
    assert_eq!(rgb[2], 1.0);
    assert!((rgb[3] - 1.0).abs() < f32::EPSILON);
    assert!(rgba_to_normalized_rgb(&input, 0, 1).is_none());
    assert!(rgba_to_normalized_rgb(&input[..7], 2, 1).is_none());
}
#[test]
fn normalized_pose_backend_receives_rgb_tensor_for_each_frame() {
    let frame = vec![255u8, 0, 128, 9];
    let mut backend = TestNormalizedPoseBackend;
    let pose = estimate_pose_with_normalized_backend(&mut backend, &[&frame, &frame], 1, 1);
    assert_eq!(pose.frames.len(), 2);
    assert_eq!(pose.frames[0].joints, vec![[1.0, 1.0]]);
    assert_eq!(pose.frames[1].frame, 1);
}
#[test]
fn pose_3d_backend_preserves_depth_and_builds_bones() {
    let frame = vec![255u8, 0, 128, 9];
    let mut backend = TestPose3DBackend;
    let pose = estimate_pose_3d_with_backend(&mut backend, &[&frame, &frame], 1, 1);
    assert_eq!(pose.frames.len(), 2);
    assert_eq!(pose.frames[0].joints, vec![[1.0, 2.0, 3.0]]);
    assert_eq!(pose.frames[0].confidence, 1.0);
    assert!(pose.bones.is_empty());
}
#[test]
fn pose_3d_projection_matches_pinhole_camera_and_rejects_behind_points() {
    let camera = PoseCameraModel {
        focal_length: 100.0,
        principal_point: [50.0, 40.0],
        position: [0.0, 0.0, 0.0],
    };
    assert_eq!(
        project_pose3d_point([1.0, 2.0, 10.0], camera),
        Some([60.0, 60.0])
    );
    assert_eq!(project_pose3d_point([1.0, 2.0, 0.0], camera), None);
    assert_eq!(project_pose3d_point([f32::NAN, 2.0, 10.0], camera), None);
    let rotated = project_pose3d_point_with_rotation([1.0, 0.0, 10.0], camera, [0.0, 90.0, 0.0]);
    assert!(rotated.is_none());
}
#[test]
fn pose_3d_track_projection_returns_editable_motion_tracks() {
    let pose = MarkerlessPose3DTrack {
        frames: vec![
            MarkerlessPose3DFrame {
                frame: 10,
                joints: vec![[0.0, 0.0, 10.0]],
                confidence: 0.8,
            },
            MarkerlessPose3DFrame {
                frame: 12,
                joints: vec![[1.0, 0.0, 10.0]],
                confidence: 0.9,
            },
        ],
        bones: vec![],
    };
    let tracks = project_pose3d_track_to_motion_tracks(
        &pose,
        PoseCameraModel {
            focal_length: 100.0,
            principal_point: [50.0, 40.0],
            position: [0.0, 0.0, 0.0],
        },
        [0.0; 3],
    );
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].samples[0].frame, 10);
    assert_eq!(tracks[0].samples[1].position, [60.0, 40.0]);
}
#[test]
fn pose_3d_csv_preserves_joint_names_and_depth() {
    let pose = MarkerlessPose3DTrack {
        frames: vec![MarkerlessPose3DFrame {
            frame: 4,
            joints: vec![[1.0, 2.0, 3.0]],
            confidence: 0.75,
        }],
        bones: vec![],
    };
    let csv = markerless_pose3d_to_csv(&pose);
    assert!(csv.lines().next().unwrap().contains("hip_z"));
    assert!(csv.lines().nth(1).unwrap().contains("4,0.75,1,2,3"));
    let json = markerless_pose3d_to_json(&pose).unwrap();
    let restored: MarkerlessPose3DTrack = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, pose);
}
#[test]
fn pose_3d_quality_counts_only_finite_joints() {
    let pose = MarkerlessPose3DTrack {
        frames: vec![MarkerlessPose3DFrame {
            frame: 0,
            joints: vec![[1.0, 2.0, 3.0], [f32::NAN; 3]],
            confidence: 1.5,
        }],
        bones: vec![],
    };
    assert_eq!(pose3d_quality(&pose, 0), (1, 1.0));
    assert_eq!(pose3d_quality(&pose, 9), (0, 0.0));
}
#[test]
fn pose_3d_quality_filter_removes_bad_frames_only() {
    let mut pose = MarkerlessPose3DTrack {
        frames: vec![
            MarkerlessPose3DFrame {
                frame: 0,
                joints: vec![[1.0, 2.0, 3.0]],
                confidence: 0.9,
            },
            MarkerlessPose3DFrame {
                frame: 1,
                joints: vec![[f32::NAN; 3]],
                confidence: 0.9,
            },
            MarkerlessPose3DFrame {
                frame: 2,
                joints: vec![[1.0, 2.0, 3.0]],
                confidence: 0.2,
            },
        ],
        bones: vec![],
    };
    assert_eq!(filter_pose3d_frames_by_quality(&mut pose, 1, 0.5), 2);
    assert_eq!(
        pose.frames
            .iter()
            .map(|frame| frame.frame)
            .collect::<Vec<_>>(),
        vec![0]
    );
}
#[test]
fn pose_3d_bone_normalization_removes_invalid_and_duplicate_edges() {
    let mut pose = MarkerlessPose3DTrack {
        frames: vec![MarkerlessPose3DFrame {
            frame: 0,
            joints: vec![[0.0, 0.0, 1.0]; 2],
            confidence: 1.0,
        }],
        bones: vec![[0, 1], [1, 0], [0, 1], [1, 1], [1, 4]],
    };
    assert_eq!(normalize_pose3d_bones(&mut pose), 3);
    assert_eq!(pose.bones, vec![[0, 1], [1, 0]]);
}
#[test]
fn pose_stabilization_repairs_short_gaps_and_smooths_jitter() {
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
                joints: vec![[4.0, 0.0]],
                root: [4.0, 0.0],
                confidence: 1.0,
            },
            MarkerlessPoseFrame {
                frame: 3,
                joints: vec![[3.0, 0.0]],
                root: [3.0, 0.0],
                confidence: 1.0,
            },
        ],
        bones: vec![],
        bone_lengths: vec![],
    };
    let changed = stabilize_markerless_pose_track(&mut pose, 1, 1);
    assert!(changed >= 2);
    assert_eq!(pose.frames[1].joints[0], [2.0, 0.0]);
    assert!(pose.frames[1].confidence < 1.0);
    assert!(pose.frames[2].joints[0][0] < 4.0);
}
