#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::timeline::{Composition, Layer, LayerType, TrackerPoint};

    #[test]
    fn test_tracker_apply_to_target() {
        let mut comp = Composition::new(
            "comp_1".to_string(),
            "TestComp".to_string(),
            1920,
            1080,
            30,
            100,
        );
        let mut src_layer = Layer::new(
            "src".to_string(),
            "Source".to_string(),
            LayerType::Null,
            100,
        );
        let mut tp = TrackerPoint::new("tp_1".to_string(), "Point1".to_string(), [100.0, 100.0]);
        tp.position = Animatable::Animated(vec![
            Keyframe::new(0, [100.0, 100.0], InterpolationType::Linear),
            Keyframe::new(10, [200.0, 150.0], InterpolationType::Linear),
        ]);
        src_layer.trackers.push(tp);
        let mut second_tp =
            TrackerPoint::new("tp_2".to_string(), "Point2".to_string(), [10.0, 20.0]);
        second_tp.position = Animatable::Animated(vec![
            Keyframe::new(0, [10.0, 20.0], InterpolationType::Linear),
            Keyframe::new(10, [40.0, 80.0], InterpolationType::Linear),
        ]);
        src_layer.trackers.push(second_tp);
        comp.layers.push(src_layer);

        let target_layer = Layer::new(
            "target".to_string(),
            "Target".to_string(),
            LayerType::Null,
            100,
        );
        comp.layers.push(target_layer);

        TrackerEngine::apply_tracker_to_target(&mut comp, 0, 0, 1, true, true);
        assert_eq!(
            comp.layers[1].transform.position.evaluate(0),
            [100.0, 100.0]
        );
        assert_eq!(
            comp.layers[1].transform.position.evaluate(10),
            [200.0, 150.0]
        );

        TrackerEngine::apply_tracker_to_target(&mut comp, 0, 1, 1, true, false);
        assert_eq!(comp.layers[1].transform.position.evaluate(0), [10.0, 20.0]);
        assert_eq!(comp.layers[1].transform.position.evaluate(10), [40.0, 80.0]);
    }

    #[test]
    fn test_stabilize_tracker_to_separate_target_inverts_motion() {
        let mut comp = Composition::new(
            "comp_stab".to_string(),
            "StabilizeComp".to_string(),
            1920,
            1080,
            30,
            100,
        );
        let mut source = Layer::new(
            "source".to_string(),
            "Tracked Source".to_string(),
            LayerType::Null,
            100,
        );
        let mut tracker =
            TrackerPoint::new("track".to_string(), "Track".to_string(), [100.0, 100.0]);
        tracker.position = Animatable::Animated(vec![
            Keyframe::new(0, [100.0, 100.0], InterpolationType::Linear),
            Keyframe::new(10, [160.0, 140.0], InterpolationType::Linear),
        ]);
        source.trackers.push(tracker);
        comp.layers.push(source);

        let mut target = Layer::new(
            "target".to_string(),
            "Stabilized Target".to_string(),
            LayerType::Null,
            100,
        );
        target.transform.position = Animatable::new_constant([500.0, 300.0]);
        comp.layers.push(target);

        TrackerEngine::apply_stabilize_tracker_to_target(&mut comp, 0, 0, 1, true, false);
        assert_eq!(
            comp.layers[1].transform.position.evaluate(0),
            [500.0, 300.0]
        );
        assert_eq!(
            comp.layers[1].transform.position.evaluate(10),
            [440.0, 260.0]
        );
    }

    #[test]
    fn test_sad_template_matching() {
        // Create 10x10 target image with a white 2x2 dot at (5, 5)
        let mut target_img = vec![0u8; 10 * 10 * 4];
        for y in 5..=6 {
            for x in 5..=6 {
                let idx = (y * 10 + x) * 4;
                target_img[idx] = 255;
                target_img[idx + 1] = 255;
                target_img[idx + 2] = 255;
                target_img[idx + 3] = 255;
            }
        }

        // Reference patch 2x2 white dot
        let ref_patch = vec![255u8; 2 * 2 * 4];

        let matched = compute_sad_match(&ref_patch, 2, 2, &target_img, 10, 10, 3, 3, 4);
        assert_eq!(
            matched,
            [6.0, 6.0],
            "SAD template matching should find the feature at (6, 6)"
        );
    }
}
