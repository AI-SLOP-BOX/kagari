use super::*;
use crate::core::property::Animatable;
use crate::core::timeline::{Composition as QComp, Layer as QLayer, LayerType as QLayerType};

const RECT: [[f32; 2]; 4] = [[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]];

#[test]
fn test_identity_homography_is_identity() {
    let h = compute_homography(RECT, RECT).expect("identity solvable");
    for (r, row) in h.iter().enumerate() {
        for (c, val) in row.iter().enumerate() {
            let expected = if r == c { 1.0 } else { 0.0 };
            assert!((val - expected).abs() < 1e-9, "H[{r}][{c}]={val}");
        }
    }
}

#[test]
fn test_translation_homography_maps_interior_point() {
    let to: [[f32; 2]; 4] = RECT.map(|p| [p[0] + 25.0, p[1] - 15.0]);
    let h = compute_homography(RECT, to).expect("translation solvable");
    let m = apply_homography(&h, [60.0, 60.0]);
    assert!(
        (m[0] - 85.0).abs() < 1e-3 && (m[1] - 45.0).abs() < 1e-3,
        "{m:?}"
    );
}

#[test]
fn test_homography_rejects_nonfinite_inputs_without_propagation() {
    let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    assert_eq!(apply_homography(&identity, [f32::NAN, 1.0]), [f32::MAX; 2]);
    assert_eq!(
        apply_homography(&identity, [1.0, f32::INFINITY]),
        [f32::MAX; 2]
    );

    let invalid = [[f64::NAN, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    assert_eq!(apply_homography(&invalid, [1.0, 1.0]), [f32::MAX; 2]);

    let overflow = [[f64::MAX, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let mapped = apply_homography(&overflow, [2.0, 1.0]);
    assert!(mapped.iter().all(|value| value.is_finite()));
}

#[test]
fn test_perspective_maps_corners_and_rejects_degenerate() {
    let dst = [[12.0f32, 5.0], [70.0, 0.0], [84.0, 62.0], [4.0, 58.0]];
    let h = compute_homography(RECT, dst).expect("perspective solvable");
    for (s, d) in RECT.iter().zip(dst.iter()) {
        let m = apply_homography(&h, *s);
        assert!(
            (m[0] - d[0]).abs() < 1e-3 && (m[1] - d[1]).abs() < 1e-3,
            "{m:?} vs {d:?}"
        );
    }
    let bad = [[0.0, 0.0], [0.0, 0.0], [84.0, 62.0], [4.0, 58.0]];
    assert!(
        compute_homography(RECT, bad).is_none(),
        "duplicates degenerate"
    );
}

#[test]
fn test_analyze_quad_tracks_all_four_corners() {
    let mut comp = QComp::new("c".into(), "Q".into(), 320, 240, 30, 30);
    let mut layer = QLayer::new("l".into(), "L".into(), QLayerType::Null, 30);
    let base: [[f32; 2]; 4] = [[40.0, 30.0], [280.0, 30.0], [280.0, 210.0], [40.0, 210.0]];
    for (i, b) in base.iter().enumerate() {
        let kfs = (0..=10u32)
            .map(|f| {
                Keyframe::new(
                    f,
                    [b[0] + f as f32, b[1] - f as f32],
                    InterpolationType::Linear,
                )
            })
            .collect();
        let mut tp = crate::core::timeline::TrackerPoint::new(format!("t{i}"), format!("T{i}"), *b);
        tp.position = Animatable::Animated(kfs);
        layer.trackers.push(tp);
    }
    comp.layers.push(layer);

    let track = TrackerEngine::analyze_quad_track(&comp, 0, [0, 1, 2, 3], 0, 10);
    assert_eq!(track.frames.len(), 11);
    // Synthetic single-step convention: frame 5 carries frame-4 state (+4, −4).
    let q5 = track.corners[5];
    for (b, t) in base.iter().zip(q5.iter()) {
        assert!((t[0] - (b[0] + 4.0)).abs() < 1e-4, "{t:?} vs {b:?}");
        assert!((t[1] - (b[1] - 4.0)).abs() < 1e-4);
    }
    let hs = quad_homographies(&track, base);
    assert_eq!(hs.len(), 11);
    let centre = apply_homography(hs[5].as_ref().expect("frame 5 H"), [160.0, 120.0]);
    assert!(
        centre[0] > 160.0 && centre[1] < 120.0,
        "centre follows motion: {centre:?}"
    );

    // Bad indices → empty.
    assert!(
        TrackerEngine::analyze_quad_track(&comp, 0, [0, 1, 2, 9], 0, 5)
            .frames
            .is_empty()
    );
    assert!(TrackerEngine::analyze_quad_track(&comp, 7, [0; 4], 0, 5)
        .frames
        .is_empty());
}

#[test]
fn test_smooth_quad_track_removes_spikes_only() {
    // Uniform diagonal motion with one 40px spike at frame 3.
    let mut track = QuadTrackData::default();
    for f in 0..8u32 {
        let off = f as f32 * 2.0 + if f == 3 { 40.0 } else { 0.0 };
        track.frames.push(f);
        track.corners.push([[10.0 + off, 20.0 - off]; 4]);
    }
    let smoothed = smooth_quad_track(&track, 10.0);
    // Spike frame replaced by neighbour midpoint.
    let c = smoothed.corners[3][0];
    assert!((c[0] - 16.0).abs() < 1e-4, "spike x {c:?}");
    // Clean frames untouched.
    assert_eq!(smoothed.corners[2][0], track.corners[2][0]);
    // Short/degenerate inputs pass through unchanged.
    assert_eq!(
        smooth_quad_track(&QuadTrackData::default(), 10.0),
        QuadTrackData::default()
    );
}

#[test]
fn test_confidence_scores_lock_quality() {
    // Rigid translated rectangle keeps near-perfect lock.
    let mut good = QuadTrackData::default();
    for f in 0..5u32 {
        good.frames.push(f);
        let o = f as f32;
        good.corners.push([
            [10.0 + o, 10.0],
            [110.0 + o, 10.0],
            [110.0 + o, 110.0],
            [10.0 + o, 110.0],
        ]);
    }
    let conf = quad_track_confidence(&good, RECT);
    assert!(conf.iter().all(|&c| c > 0.95), "rigid motion: {conf:?}");

    // Flipped quad (corner crossing) → zero confidence.
    let mut bad = QuadTrackData::default();
    bad.frames.push(0);
    bad.corners
        .push([[0.0, 0.0], [100.0, 0.0], [0.0, 100.0], [100.0, 100.0]]);
    assert!(
        quad_track_confidence(&bad, RECT).iter().all(|&c| c == 0.0),
        "flipped quad must score zero"
    );
}

#[test]
fn test_extract_patch_exact_pixels_and_oob() {
    let mut frame = vec![7u8; 16 * 16 * 4];
    // Stamp a red pixel at (5,5).
    let idx = ((5 * 16 + 5) * 4) as usize;
    frame[idx] = 250;
    frame[idx + 1] = 10;
    frame[idx + 2] = 10;
    let patch = extract_patch(&frame, 16, 16, 5, 5, 3).expect("in-bounds");
    assert_eq!(patch.len(), 3 * 3 * 4);
    // Centre of the 3×3 patch is the red pixel.
    assert_eq!(&patch[16..20], &[250, 10, 10, 7]);
    // Corners remain background.
    assert_eq!(patch[0], 7);
    // Fully out of bounds rejected.
    assert!(extract_patch(&frame, 16, 16, 15, 15, 3).is_none());
    assert!(extract_patch(&frame, 16, 16, -1, 5, 3).is_none());
}

#[test]
fn test_blend_template_semantics() {
    let mut tmpl = vec![100u8, 200];
    // Frozen.
    blend_template(&mut tmpl, &[0, 0], 0.0);
    assert_eq!(tmpl, vec![100, 200]);
    // Half-way toward current.
    blend_template(&mut tmpl, &[0, 0], 0.5);
    assert_eq!(tmpl, vec![50, 100]);
    // Full adopt.
    blend_template(&mut tmpl, &[90, 90], 1.0);
    assert_eq!(tmpl, vec![90, 90]);
    // Length mismatch ignored.
    let mut keep = vec![1u8];
    blend_template(&mut keep, &[9, 9, 9], 1.0);
    assert_eq!(keep, vec![1]);
}

#[test]
fn test_persistent_template_follows_appearance_not_local_pixels() {
    // Gray field; an 8×8 checker sits centred at (40,32). The tracker is
    // seeded ON FLAT GRAY but carries the checker as its persistent
    // template — it must slide to the checker, not stay put.
    let w = 64usize;
    let h = 64usize;
    let mut frame = vec![30u8; w * h * 4];
    for dy in 0..8usize {
        for dx in 0..8usize {
            let v = if (dx + dy) % 2 == 0 { 240 } else { 15 };
            let idx = ((28 + dy) * w + 36 + dx) * 4;
            frame[idx] = v;
            frame[idx + 1] = v;
            frame[idx + 2] = v;
            frame[idx + 3] = 255;
        }
    }

    let template: Vec<f32> = extract_patch(&frame, 64, 64, 40, 32, 8)
        .expect("template in bounds")
        .iter()
        .map(|&b| b as f32 / 255.0)
        .collect();
    let mut layer = QLayer::new("l".into(), "L".into(), QLayerType::Null, 30);
    layer.trackers.push(crate::core::timeline::TrackerPoint {
        id: "t".into(),
        name: "T".into(),
        position: Animatable::new_constant([20.0, 32.0]),
        search_size: 30.0,
        feature_size: 8.0,
        reference_pattern: Some(template),
    });

    let result =
        TrackerEngine::track_next_frame_pixels(&layer, 0, 0, &frame, &frame.clone(), 64, 64);
    let new_pos = result.expect("match found");
    assert!(
        (new_pos[0] - 40.0).abs() < 2.0 && (new_pos[1] - 32.0).abs() < 2.0,
        "tracker must lock onto the templated checker: {new_pos:?}"
    );
}

#[test]
fn test_refine_replaces_only_low_confidence_frames() {
    // Rigid translation with frame 3 collapsed to a degenerate point.
    let mut track = QuadTrackData::default();
    for f in 0..6u32 {
        let o = f as f32 * 5.0;
        let quad = if f == 3 {
            [[50.0, 50.0]; 4]
        } else {
            [
                [10.0 + o, 10.0],
                [60.0 + o, 10.0],
                [60.0 + o, 60.0],
                [10.0 + o, 60.0],
            ]
        };
        track.frames.push(f);
        track.corners.push(quad);
    }
    let refined = refine_quad_track(&track, RECT, 0.5, 25.0);

    // Healthy frames bit-identical.
    assert_eq!(refined.corners[0], track.corners[0]);
    assert_eq!(refined.corners[4], track.corners[4]);
    // Degenerate frame pulled toward the smoothed midpoint: neighbour
    // corner-0 positions are (20,10)/(30,10) → midpoint (25,10).
    assert_ne!(refined.corners[3], track.corners[3]);
    let c = refined.corners[3][0];
    assert!(
        (c[0] - 25.0).abs() < 1e-4 && (c[1] - 10.0).abs() < 1e-4,
        "midpoint {c:?}"
    );
}

#[test]
fn test_tracker_apply_stabilize() {
    let mut comp = Composition::new("c".into(), "Comp".into(), 100, 100, 30, 30);
    let mut layer = Layer::new(
        "l".into(),
        "Shaky Layer".into(),
        crate::core::timeline::LayerType::Null,
        30,
    );
    layer.transform.position = Animatable::new_constant([50.0, 50.0]);

    // Tracker detects camera drifted by +10px X at frame 15
    let track_kfs = vec![
        Keyframe::new(0, [100.0, 100.0], InterpolationType::Linear),
        Keyframe::new(15, [110.0, 100.0], InterpolationType::Linear),
    ];
    layer.trackers.push(crate::core::timeline::TrackerPoint {
        id: "t1".into(),
        name: "Feature".into(),
        position: Animatable::Animated(track_kfs),
        search_size: 20.0,
        feature_size: 10.0,
        reference_pattern: None,
    });
    comp.add_layer(layer);

    TrackerEngine::apply_stabilize_to_layer(&mut comp, 0, 0, true, false);

    // Frame 0: position should be 50.0
    let p0 = comp.layers[0].transform.position.evaluate(0);
    assert_eq!(p0, [50.0, 50.0]);

    // Frame 15: position must counteract the +10px drift by moving to 40.0
    let p15 = comp.layers[0].transform.position.evaluate(15);
    assert_eq!(p15, [40.0, 50.0]);
}

#[test]
fn test_smooth_tracker_keyframes_reduces_noise() {
    let kfs = vec![
        Keyframe::new(0, [10.0, 10.0], InterpolationType::Linear),
        Keyframe::new(1, [100.0, 10.0], InterpolationType::Linear), // Noise spike
        Keyframe::new(2, [10.0, 10.0], InterpolationType::Linear),
    ];
    let smoothed = smooth_tracker_keyframes(&kfs, 1);
    assert_eq!(smoothed.len(), 3);
    // Spike at frame 1 should be damped significantly by its neighbours
    assert!(smoothed[1].value[0] < 100.0);
}

#[test]
fn pose_application_is_idempotent_for_repeated_analysis() {
    let mut layer = Layer::new(
        "l".into(),
        "Pose Layer".into(),
        crate::core::timeline::LayerType::Null,
        10,
    );
    let pose = crate::core::optical_flow_timewarp::MarkerlessPoseTrack {
        frames: vec![
            crate::core::optical_flow_timewarp::MarkerlessPoseFrame {
                frame: 0,
                joints: vec![[1.0, 2.0]],
                root: [1.0, 2.0],
                confidence: 1.0,
            },
            crate::core::optical_flow_timewarp::MarkerlessPoseFrame {
                frame: 1,
                joints: vec![[2.0, 2.0]],
                root: [2.0, 2.0],
                confidence: 1.0,
            },
        ],
        bones: vec![],
        bone_lengths: vec![],
    };
    assert_eq!(
        TrackerEngine::apply_pose_as_tracker_points(&mut layer, &pose, 0.5),
        1
    );
    assert_eq!(
        TrackerEngine::apply_pose_as_tracker_points(&mut layer, &pose, 0.5),
        1
    );
    assert_eq!(layer.trackers.len(), 1);
    assert_eq!(layer.trackers[0].position.evaluate(1), [2.0, 2.0]);
}

#[test]
fn pose_application_removes_stale_pose_points_but_keeps_manual_points() {
    let mut layer = Layer::new(
        "l".into(),
        "Pose Layer".into(),
        crate::core::timeline::LayerType::Null,
        10,
    );
    layer
        .trackers
        .push(crate::core::timeline::TrackerPoint::new(
            "manual".into(),
            "Manual".into(),
            [9.0, 9.0],
        ));
    layer
        .trackers
        .push(crate::core::timeline::TrackerPoint::new(
            "pose_old_joint".into(),
            "Old".into(),
            [8.0, 8.0],
        ));
    let pose = crate::core::optical_flow_timewarp::MarkerlessPoseTrack {
        frames: vec![crate::core::optical_flow_timewarp::MarkerlessPoseFrame {
            frame: 0,
            joints: vec![[1.0, 2.0]],
            root: [1.0, 2.0],
            confidence: 1.0,
        }],
        bones: vec![],
        bone_lengths: vec![],
    };
    TrackerEngine::apply_pose_as_tracker_points(&mut layer, &pose, 0.0);
    assert!(layer.trackers.iter().any(|tracker| tracker.id == "manual"));
    assert!(!layer
        .trackers
        .iter()
        .any(|tracker| tracker.id == "pose_old_joint"));
}
