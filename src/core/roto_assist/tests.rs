use super::flow_bake::*;
use super::segmentation::*;
use crate::core::mask::Mask;
use crate::core::timeline::TrackerPoint;

#[cfg(test)]
mod tests {
    use super::*;

    fn square_mask() -> Mask {
        // 12x12 square at origin
        crate::core::mask::Mask::new_rect("m".into(), "M".into(), 0.0, 0.0, 12.0, 12.0)
    }

    fn moving_tracker() -> TrackerPoint {
        let mut t = TrackerPoint::new("t".into(), "T".into(), [100.0, 100.0]);
        t.position = Animatable::Animated(vec![
            Keyframe::new(0, [100.0, 100.0], InterpolationType::Linear),
            Keyframe::new(10, [140.0, 90.0], InterpolationType::Linear),
        ]);
        t
    }

    #[test]
    fn test_bake_translates_shape_with_tracker() {
        let m = square_mask();
        let t = moving_tracker();
        let baked = bake_tracked_mask(&m, &t, 0, 10).expect("bakes");
        match &baked {
            Animatable::Animated(kfs) => {
                assert!(kfs.len() >= 2);
                let first = kfs[0].value.clone();
                let last = kfs.last().unwrap().value.clone();
                // Shape translated by (40, -10)
                assert!((last[0][0] - first[0][0] - 40.0).abs() < 0.01);
                assert!((last[0][1] - first[0][1] + 10.0).abs() < 0.01);
            }
            _ => panic!("expected animated"),
        }
    }

    #[test]
    fn bake_tracked_mask_uses_the_selected_tracker_point() {
        let mask = square_mask();
        let stationary = TrackerPoint::new("stationary".into(), "Stationary".into(), [0.0, 0.0]);
        let selected = moving_tracker();
        let baked = bake_tracked_mask_from_trackers(&mask, &[stationary, selected], 1, 0, 10)
            .expect("selected tracker should bake");
        let Animatable::Animated(keyframes) = baked else {
            panic!("expected animated mask")
        };
        assert_eq!(keyframes.first().unwrap().value[0], [0.0, 0.0]);
        assert!((keyframes.last().unwrap().value[0][0] - 40.0).abs() < 0.01);
        assert!((keyframes.last().unwrap().value[0][1] + 10.0).abs() < 0.01);
    }

    #[test]
    fn bake_tracked_mask_rejects_missing_tracker_index() {
        let error = bake_tracked_mask_from_trackers(&square_mask(), &[], 0, 0, 10)
            .expect_err("missing tracker must be reported");
        assert!(error.contains("tracker point 1"));
    }

    #[test]
    fn optical_flow_propagation_tracks_local_frame_motion() {
        fn textured_frame(width: u32, height: u32, shift_x: i32) -> Vec<u8> {
            let mut rgba = vec![0u8; width as usize * height as usize * 4];
            for y in 0..height {
                for x in 0..width {
                    let source_x = x as i32 - shift_x;
                    let index = ((y * width + x) * 4) as usize;
                    if source_x >= 0 && source_x < width as i32 {
                        let mut value = (source_x as u32)
                            .wrapping_mul(0x045d_9f3b)
                            .wrapping_add(y.wrapping_mul(0x119d_e1f3));
                        value ^= value >> 16;
                        value = value.wrapping_mul(0x045d_9f3b);
                        value ^= value >> 16;
                        rgba[index] = value as u8;
                        rgba[index + 1] = (value >> 8) as u8;
                        rgba[index + 2] = (value >> 16) as u8;
                        rgba[index + 3] = 255;
                    }
                }
            }
            rgba
        }

        let mask = Mask::new_rect("roto".into(), "Roto".into(), 40.0, 30.0, 16.0, 16.0);
        let left = textured_frame(128, 96, -6);
        let source = textured_frame(128, 96, 0);
        let right = textured_frame(128, 96, 6);
        let baked = bake_optical_flow_mask(&mask, 1, 0, 2, 128, 96, |frame| {
            Some(match frame {
                0 => left.clone(),
                1 => source.clone(),
                _ => right.clone(),
            })
        })
        .expect("bidirectional flow propagation should bake");

        let Animatable::Animated(keyframes) = baked else {
            panic!("expected animated mask path");
        };
        assert_eq!(keyframes.len(), 3);
        assert_eq!(keyframes[0].frame, 0);
        assert_eq!(keyframes[1].frame, 1);
        assert_eq!(keyframes[2].frame, 2);
        let left_point = keyframes[0].value[0];
        let anchor_point = keyframes[1].value[0];
        let right_point = keyframes[2].value[0];
        assert!(
            (left_point[0] - anchor_point[0] + 6.0).abs() <= 1.0,
            "left flow was {:?} from anchor {:?}",
            left_point,
            anchor_point
        );
        assert!(
            (right_point[0] - anchor_point[0] - 6.0).abs() <= 1.0,
            "right flow was {:?} from anchor {:?}",
            right_point,
            anchor_point
        );
        assert!((left_point[1] - anchor_point[1]).abs() <= 1.0);
        assert!((right_point[1] - anchor_point[1]).abs() <= 1.0);
    }

    #[test]
    fn optical_flow_propagation_resegments_user_corrections_on_occluded_frames() {
        fn frame(with_occluder: bool) -> Vec<u8> {
            let (width, height) = (128usize, 96usize);
            let mut pixels = vec![0u8; width * height * 4];
            for y in 0..height {
                for x in 0..width {
                    let index = (y * width + x) * 4;
                    let subject = (40..=84).contains(&x) && (28..=68).contains(&y);
                    let occluder =
                        with_occluder && (60..=68).contains(&x) && (20..=76).contains(&y);
                    let color = if occluder {
                        [20, 220, 20, 255]
                    } else if subject {
                        [220, 20, 20, 255]
                    } else {
                        [20, 40, 220, 255]
                    };
                    pixels[index..index + 4].copy_from_slice(&color);
                }
            }
            pixels
        }

        fn contains(polygon: &[[f32; 2]], point: [f32; 2]) -> bool {
            let mut inside = false;
            let mut previous = polygon.len() - 1;
            for current in 0..polygon.len() {
                let a = polygon[current];
                let b = polygon[previous];
                if (a[1] > point[1]) != (b[1] > point[1])
                    && point[0] < (b[0] - a[0]) * (point[1] - a[1]) / (b[1] - a[1]) + a[0]
                {
                    inside = !inside;
                }
                previous = current;
            }
            inside
        }

        let anchor = frame(false);
        let occluded = frame(true);
        let mask = Mask::new_rect("roto".into(), "Roto".into(), 36.0, 24.0, 52.0, 48.0);
        let strokes = vec![
            crate::core::roto_brush_engine::RotoStroke {
                stroke_type: crate::core::roto_brush_engine::RotoStrokeType::Foreground,
                points: vec![[48.0, 48.0], [52.0, 48.0]],
                radius: 4.0,
                frame: Some(1),
            },
            crate::core::roto_brush_engine::RotoStroke {
                stroke_type: crate::core::roto_brush_engine::RotoStrokeType::Background,
                points: vec![[30.0, 48.0]],
                radius: 4.0,
                frame: Some(1),
            },
            crate::core::roto_brush_engine::RotoStroke {
                stroke_type: crate::core::roto_brush_engine::RotoStrokeType::Background,
                points: vec![[92.0, 48.0]],
                radius: 4.0,
                frame: Some(1),
            },
            crate::core::roto_brush_engine::RotoStroke {
                stroke_type: crate::core::roto_brush_engine::RotoStrokeType::Background,
                points: vec![[64.0, 48.0]],
                radius: 6.0,
                frame: Some(2),
            },
        ];
        let baked = bake_optical_flow_roto_matte(
            &mask,
            &strokes,
            &crate::core::roto_brush_engine::RotoBrushSettings {
                feather_radius: 0.0,
                contrast: 1.0,
                ..Default::default()
            },
            1,
            1,
            2,
            128,
            96,
            |frame| {
                Some(if frame == 1 {
                    anchor.clone()
                } else {
                    occluded.clone()
                })
            },
        )
        .expect("corrected roto should propagate through the occlusion");
        let Animatable::Animated(keyframes) = baked else {
            panic!("expected per-frame roto keys");
        };
        let vertex_count = keyframes[0].value.len();
        assert!(keyframes
            .iter()
            .all(|keyframe| keyframe.value.len() == vertex_count));
        let corrected = &keyframes
            .iter()
            .find(|keyframe| keyframe.frame == 2)
            .expect("corrected frame must have a key")
            .value;
        assert!(
            contains(corrected, [50.0, 48.0]),
            "visible subject was lost"
        );
        assert!(
            !contains(corrected, [64.0, 48.0]),
            "occluder correction was ignored"
        );
    }

    #[test]
    fn test_empty_base_mask_errors() {
        let mut m = Mask::new_rect("e".into(), "E".into(), 0.0, 0.0, 8.0, 8.0);
        m.path.vertices = Animatable::Animated(vec![]); // no vertices at all
        let t = moving_tracker();
        assert!(bake_tracked_mask(&m, &t, 0, 10).is_err());
    }

    #[test]
    fn test_bake_rejects_reversed_frame_range() {
        let m = square_mask();
        let t = moving_tracker();
        let error = bake_tracked_mask(&m, &t, 10, 9).expect_err("reversed range is invalid");
        assert!(error.contains("end frame"));
    }

    #[test]
    fn test_bake_includes_end_frame_without_u32_overflow() {
        let m = square_mask();
        let t = TrackerPoint::new("t".into(), "T".into(), [5.0, 5.0]);
        let baked = bake_tracked_mask(&m, &t, u32::MAX - 1, u32::MAX).expect("bakes safely");
        let Animatable::Animated(kfs) = baked else {
            panic!("expected animated mask")
        };
        assert_eq!(
            kfs.iter().map(|kf| kf.frame).collect::<Vec<_>>(),
            [u32::MAX - 1, u32::MAX]
        );
    }

    #[test]
    fn test_static_tracker_yields_constant_motion() {
        let m = square_mask();
        let t = TrackerPoint::new("t".into(), "T".into(), [5.0, 5.0]);
        let baked = bake_tracked_mask(&m, &t, 0, 20).expect("bakes");
        if let Animatable::Animated(kfs) = &baked {
            let first = kfs[0].value.clone();
            let last = kfs.last().unwrap().value.clone();
            assert_eq!(first[0], last[0], "no motion → identical vertices");
        } else {
            panic!("expected animated");
        }
    }

    #[test]
    fn test_roto_brush_segmentation_and_refine_edge() {
        let width = 16u32;
        let height = 16u32;
        let mut img = vec![0u8; (width * height * 4) as usize];

        // Left half Red (Foreground subject), Right half Blue (Background)
        for y in 0..height {
            for x in 0..width {
                let idx = ((y * width + x) * 4) as usize;
                if x < 8 {
                    img[idx] = 250;
                    img[idx + 1] = 20;
                    img[idx + 2] = 20;
                    img[idx + 3] = 255;
                } else {
                    img[idx] = 20;
                    img[idx + 1] = 20;
                    img[idx + 2] = 250;
                    img[idx + 3] = 255;
                }
            }
        }

        let strokes = vec![
            RotoStroke {
                is_foreground: true,
                points: vec![[4.0, 8.0]],
                radius: 2.0,
            },
            RotoStroke {
                is_foreground: false,
                points: vec![[12.0, 8.0]],
                radius: 2.0,
            },
        ];

        let rough_mask = segment_roto_brush(&img, width, height, &strokes);
        assert_eq!(
            rough_mask[8 * 16 + 2],
            255,
            "Left half must be segmented as foreground"
        );
        assert_eq!(
            rough_mask[8 * 16 + 14],
            0,
            "Right half must be segmented as background"
        );

        let refined = refine_edge_guided_filter(&img, &rough_mask, width, height, 2, 0.01);
        assert_eq!(refined.len(), (width * height) as usize);
        assert!(refined[8 * 16 + 2] > 200);
    }

    #[test]
    fn roto_refinement_changes_geometry_and_mask_render_settings() {
        let mut mask = square_mask();
        let original = mask.path.vertices.evaluate(0);

        assert!(apply_roto_refinement(&mut mask, 0, 4.0, 6.0, -2.5));

        let refined = mask.path.vertices.evaluate(0);
        assert_eq!(refined.len(), original.len());
        assert_ne!(refined, original);
        assert_eq!(mask.feather.evaluate(0), 6.0);
        assert_eq!(mask.expansion.evaluate(0), -2.5);
    }

    #[test]
    fn roto_matte_corrections_are_frame_keyed_and_hold_variable_vertex_counts() {
        let mut mask = square_mask();
        let original = mask.path.vertices.evaluate(0);
        let first = vec![[1.0, 1.0], [5.0, 1.0], [3.0, 5.0]];
        let corrected = vec![[2.0, 1.0], [6.0, 1.0], [7.0, 3.0], [3.0, 6.0]];
        assert!(set_roto_matte_keyframe(&mut mask, 4, first.clone()));
        assert!(set_roto_matte_keyframe(&mut mask, 8, corrected.clone()));
        assert_eq!(mask.path.vertices.evaluate(3), original);
        assert_eq!(mask.path.vertices.evaluate(4), first);
        assert_eq!(mask.path.vertices.evaluate(7), first);
        assert_eq!(mask.path.vertices.evaluate(8), corrected);
        assert!(!set_roto_matte_keyframe(
            &mut mask,
            9,
            vec![[f32::NAN, 0.0], [1.0, 0.0], [0.0, 1.0]],
        ));
    }

    #[test]
    fn roto_refinement_preserves_animated_vertex_tracks_and_bounds_growth() {
        let mut mask = square_mask();
        let vertices = mask.path.vertices.evaluate(0);
        mask.path.vertices = Animatable::new_animated(vec![
            Keyframe::new(0, vertices.clone(), InterpolationType::Linear),
            Keyframe::new(10, vertices, InterpolationType::Linear),
        ]);
        mask.feather = Animatable::new_animated(vec![
            Keyframe::new(0, 2.0, InterpolationType::Linear),
            Keyframe::new(20, 4.0, InterpolationType::Linear),
        ]);
        mask.expansion = Animatable::new_animated(vec![
            Keyframe::new(0, -2.0, InterpolationType::Linear),
            Keyframe::new(20, 2.0, InterpolationType::Linear),
        ]);

        apply_roto_refinement(&mut mask, 10, 10.0, 3.0, 1.0);

        let Animatable::Animated(keyframes) = &mask.path.vertices else {
            panic!("refinement must preserve animation")
        };
        assert_eq!(keyframes.len(), 2);
        assert!(keyframes.iter().all(|keyframe| keyframe.value.len() == 4));
        assert_eq!(mask.feather.keyframes().unwrap().len(), 3);
        assert_eq!(mask.expansion.keyframes().unwrap().len(), 3);
        assert_eq!(mask.feather.evaluate(0), 2.0);
        assert_eq!(mask.feather.evaluate(10), 3.0);
        assert_eq!(mask.feather.evaluate(20), 4.0);
        assert_eq!(mask.expansion.evaluate(0), -2.0);
        assert_eq!(mask.expansion.evaluate(10), 1.0);
        assert_eq!(mask.expansion.evaluate(20), 2.0);
    }
}
