use super::*;
use crate::core::property::Animatable;
use crate::core::timeline::{Layer, LayerType, TrackerPoint};

/// Builds a 64x64 gray frame with a bright square at `sq_x`.
fn frame_with_square(sq_x: usize) -> Vec<u8> {
    let w = 64usize;
    let h = 64usize;
    let mut px = vec![30u8; w * h * 4];
    for y in 20..40 {
        for x in sq_x..sq_x + 12 {
            let idx = (y * w + x) * 4;
            px[idx] = 240;
            px[idx + 1] = 240;
            px[idx + 2] = 240;
            px[idx + 3] = 255;
        }
    }
    px
}

#[test]
fn test_real_pixel_tracking_follows_moving_feature() {
    // Square at x=10 in frame A, x=16 in frame B → tracker must move ~+6
    let curr = frame_with_square(10);
    let next = frame_with_square(16);
    let (w, h) = (64usize, 64usize);

    let mut layer = Layer::new("l".into(), "L".into(), LayerType::Null, 30);
    layer.trackers.push(TrackerPoint {
        id: "t1".into(),
        name: "Track".into(),
        // Straddle the square's LEFT EDGE (square spans x 10..22): gives the
        // patch a unique texture so the SAD minimum is well-defined.
        position: Animatable::new_constant([10.0, 32.0]),
        search_size: 24.0,
        feature_size: 12.0,
        reference_pattern: None,
    });

    let result = TrackerEngine::track_next_frame_pixels(&layer, 0, 0, &curr, &next, w, h);
    assert!(result.is_some());
    let new_pos = result.unwrap();
    // The tracked edge pattern moved +6 px; tracker follows onto the new
    // edge position.
    assert!(
        (new_pos[0] - 16.0).abs() < 1.5,
        "expected x≈16 (edge followed +6), got {}",
        new_pos[0]
    );
    // Y should stay put
    assert!(
        (new_pos[1] - 32.0).abs() < 1.5,
        "expected y≈32, got {}",
        new_pos[1]
    );
}

#[test]
fn test_tracking_stationary_scene_stays_put() {
    let f = frame_with_square(20);
    let mut layer = Layer::new("l".into(), "L".into(), LayerType::Null, 30);
    layer.trackers.push(TrackerPoint {
        id: "t1".into(),
        name: "T".into(),
        position: Animatable::new_constant([20.0, 32.0]), // on the edge
        search_size: 20.0,
        feature_size: 12.0,
        reference_pattern: None,
    });
    let r = TrackerEngine::track_next_frame_pixels(&layer, 0, 0, &f, &f.clone(), 64, 64).unwrap();
    assert!(
        (r[0] - 20.0).abs() < 0.6 && (r[1] - 32.0).abs() < 0.6,
        "{:?}",
        r
    );
}
