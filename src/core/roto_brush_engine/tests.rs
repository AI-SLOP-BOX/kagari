#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use crate::core::roto_brush_engine::filter::*;
    use crate::core::roto_brush_engine::matte;
    use crate::core::roto_brush_engine::*;

    #[test]
    fn oversized_or_invalid_matte_dimensions_return_without_allocating_a_frame() {
        let settings = RotoBrushSettings::default();
        let strokes = [RotoStroke {
            stroke_type: RotoStrokeType::Foreground,
            points: vec![[1.0, 1.0]],
            pressures: Vec::new(),
            radius: 1.0,
            frame: None,
        }];

        assert!(
            generate_rotobrush_matte_cancellable(&[], 4096, 2160, &strokes, &settings, || false)
                .unwrap()
                .is_empty()
        );
        assert!(generate_rotobrush_matte_cancellable(
            &[],
            i32::MAX as u32 + 1,
            1,
            &strokes,
            &settings,
            || false
        )
        .unwrap()
        .is_empty());
        assert_eq!(MAX_ROTO_FRAME_PIXELS, 8_388_608);
        assert!(distance_field_from_points(4096, 2160, &[[1.0, 1.0]]).is_empty());
    }

    #[test]
    fn mismatched_input_keeps_the_legacy_zero_matte_contract_within_the_limit() {
        let matte =
            generate_rotobrush_matte(&[1, 2, 3], 10, 10, &[], &RotoBrushSettings::default());
        assert_eq!(matte, vec![0; 100]);
    }

    #[test]
    fn color_samples_are_budgeted_across_the_full_stroke() {
        assert_eq!(matte::bounded_color_sample_plan(8, 512), (1, 64));
        assert_eq!(matte::bounded_color_sample_plan(1_000, 512), (2, 1));
        assert_eq!(matte::bounded_color_sample_plan(0, 512), (1, 0));
    }

    #[test]
    fn color_model_preserves_distinct_foreground_appearance_clusters() {
        let samples = [
            [250.0, 8.0, 4.0],
            [245.0, 12.0, 6.0],
            [5.0, 7.0, 245.0],
            [8.0, 6.0, 250.0],
        ];
        let prototypes = color_prototypes(&samples, 2);

        assert_eq!(prototypes.len(), 2);
        assert!(prototypes
            .iter()
            .any(|color| color_dist_sq(*color, [247.0, 10.0, 5.0]) < 20.0));
        assert!(prototypes
            .iter()
            .any(|color| color_dist_sq(*color, [6.0, 6.0, 247.0]) < 20.0));
    }

    #[test]
    fn test_rotobrush_foreground_extraction() {
        let width = 32u32;
        let height = 32u32;
        let mut pixels = vec![0u8; (width * height * 4) as usize];

        // Fill left side with white (FG), right side with black (BG)
        for y in 0..height {
            for x in 0..width {
                let idx = ((y * width + x) * 4) as usize;
                let val = if x < 16 { 255 } else { 0 };
                pixels[idx] = val;
                pixels[idx + 1] = val;
                pixels[idx + 2] = val;
                pixels[idx + 3] = 255;
            }
        }

        let strokes = vec![
            RotoStroke {
                stroke_type: RotoStrokeType::Foreground,
                points: vec![[8.0, 16.0]],
                pressures: Vec::new(),
                radius: 4.0,
                frame: None,
            },
            RotoStroke {
                stroke_type: RotoStrokeType::Background,
                points: vec![[24.0, 16.0]],
                pressures: Vec::new(),
                radius: 4.0,
                frame: None,
            },
        ];

        let settings = RotoBrushSettings {
            feather_radius: 0.0,
            ..Default::default()
        };
        let matte = generate_rotobrush_matte(&pixels, width, height, &strokes, &settings);

        // Foreground pixel should have high alpha
        assert!(matte[(16 * width + 8) as usize] > 180);
        // Background pixel should have low alpha
        assert!(matte[(16 * width + 24) as usize] < 80);
    }

    #[test]
    fn test_boundary_temporal_propagation() {
        let boundary = vec![[10.0, 10.0], [20.0, 10.0], [20.0, 20.0]];
        let settings = RotoBrushSettings {
            temporal_smoothing: 0.0,
            propagation_quality: 2,
            ..Default::default()
        };
        let propagated = propagate_roto_boundary(&boundary, [6.0, -4.0], &settings);
        assert_eq!(propagated[0], [16.0, 6.0]);
    }

    #[test]
    fn test_rotobrush_zero_radius_does_not_mark_entire_flat_image() {
        let pixels = vec![255u8; 3 * 3 * 4];
        let stroke = RotoStroke {
            stroke_type: RotoStrokeType::Foreground,
            points: vec![[1.0, 1.0]],
            pressures: Vec::new(),
            radius: 0.0,
            frame: None,
        };
        let settings = RotoBrushSettings {
            feather_radius: 0.0,
            ..Default::default()
        };
        let matte = generate_rotobrush_matte(&pixels, 3, 3, &[stroke], &settings);
        assert!(matte[4] > 200);
        assert!(matte
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != 4)
            .all(|(_, a)| *a < 200));
    }

    #[test]
    fn roto_brush_strokes_are_hard_constraints_even_when_color_model_disagrees() {
        let width = 12;
        let height = 8;
        let mut pixels = vec![127u8; width * height * 4];
        for rgba in pixels.as_chunks_mut::<4>().0 {
            rgba[3] = 255;
        }
        let strokes = vec![
            RotoStroke {
                stroke_type: RotoStrokeType::Foreground,
                points: vec![[2.0, 4.0]],
                pressures: Vec::new(),
                radius: 1.0,
                frame: None,
            },
            RotoStroke {
                stroke_type: RotoStrokeType::Background,
                points: vec![[9.0, 4.0]],
                pressures: Vec::new(),
                radius: 1.0,
                frame: None,
            },
        ];
        let settings = RotoBrushSettings {
            feather_radius: 0.0,
            ..Default::default()
        };
        let matte =
            generate_rotobrush_matte(&pixels, width as u32, height as u32, &strokes, &settings);
        assert_eq!(matte[4 * width + 2], 255);
        assert_eq!(matte[4 * width + 9], 0);
    }

    #[test]
    fn ambiguous_pixels_equidistant_from_fg_and_bg_default_to_background() {
        let width = 64usize;
        let height = 16usize;
        let mut pixels = vec![96u8; width * height * 4];
        for rgba in pixels.as_chunks_mut::<4>().0 {
            rgba[3] = 255;
        }
        let strokes = [
            RotoStroke {
                stroke_type: RotoStrokeType::Foreground,
                points: vec![[8.0, 8.0]],
                pressures: Vec::new(),
                radius: 0.0,
                frame: None,
            },
            RotoStroke {
                stroke_type: RotoStrokeType::Background,
                points: vec![[56.0, 8.0]],
                pressures: Vec::new(),
                radius: 0.0,
                frame: None,
            },
        ];
        let settings = RotoBrushSettings {
            feather_radius: 0.0,
            ..Default::default()
        };

        let matte =
            generate_rotobrush_matte(&pixels, width as u32, height as u32, &strokes, &settings);

        assert_eq!(matte[8 * width + 8], 255);
        assert_eq!(matte[8 * width + 56], 0);
        assert!(matte[8 * width + 32] < 128);
    }

    #[test]
    fn single_foreground_stroke_does_not_select_distant_matching_color() {
        let width = 128usize;
        let height = 32usize;
        let mut pixels = vec![0u8; width * height * 4];
        for rgba in pixels.as_chunks_mut::<4>().0 {
            rgba.copy_from_slice(&[200, 40, 40, 255]);
        }
        let stroke = RotoStroke {
            stroke_type: RotoStrokeType::Foreground,
            points: vec![[8.0, 16.0]],
            pressures: Vec::new(),
            radius: 0.0,
            frame: None,
        };
        let settings = RotoBrushSettings {
            feather_radius: 0.0,
            ..Default::default()
        };

        let matte =
            generate_rotobrush_matte(&pixels, width as u32, height as u32, &[stroke], &settings);

        assert_eq!(matte[16 * width + 8], 255, "the foreground stroke is exact");
        assert!(
            matte[16 * width + 13] < 128,
            "matching color five pixels from a single foreground stroke must not remain foreground"
        );
    }

    #[test]
    fn distant_foreground_like_color_stays_background_with_both_stroke_classes() {
        let width = 128usize;
        let height = 32usize;
        let mut pixels = vec![0u8; width * height * 4];
        for rgba in pixels.as_chunks_mut::<4>().0 {
            rgba.copy_from_slice(&[220, 30, 30, 255]);
        }
        // The local background sample is deliberately unlike the distant,
        // foreground-colored region, reproducing a common matte leak.
        let bg_idx = (16 * width + 5) * 4;
        pixels[bg_idx..bg_idx + 4].copy_from_slice(&[20, 200, 20, 255]);
        let strokes = [
            RotoStroke {
                stroke_type: RotoStrokeType::Foreground,
                points: vec![[10.0, 16.0]],
                pressures: Vec::new(),
                radius: 0.0,
                frame: None,
            },
            RotoStroke {
                stroke_type: RotoStrokeType::Background,
                points: vec![[5.0, 16.0]],
                pressures: Vec::new(),
                radius: 0.0,
                frame: None,
            },
        ];
        let settings = RotoBrushSettings {
            feather_radius: 0.0,
            ..Default::default()
        };

        let matte =
            generate_rotobrush_matte(&pixels, width as u32, height as u32, &strokes, &settings);

        assert_eq!(matte[16 * width + 10], 255, "foreground stroke stays exact");
        assert_eq!(matte[16 * width + 5], 0, "background stroke stays exact");
        assert!(
            matte[16 * width + 120] < 128,
            "distant foreground-colored scenery should not leak into the matte"
        );
    }

    #[test]
    fn fast_roto_drag_interpolates_brush_seeds_without_gaps() {
        let width = 16;
        let height = 8;
        let pixels = vec![96u8; width * height * 4];
        let stroke = RotoStroke {
            stroke_type: RotoStrokeType::Foreground,
            points: vec![[2.0, 4.0], [13.0, 4.0]],
            pressures: Vec::new(),
            radius: 1.0,
            frame: None,
        };
        let settings = RotoBrushSettings {
            feather_radius: 0.0,
            ..Default::default()
        };
        let matte =
            generate_rotobrush_matte(&pixels, width as u32, height as u32, &[stroke], &settings);
        assert_eq!(matte[4 * width + 8], 255);
    }

    #[test]
    fn frame_scoped_strokes_do_not_leak_between_frames_and_legacy_strokes_remain_global() {
        let stroke = |frame| RotoStroke {
            stroke_type: RotoStrokeType::Foreground,
            points: vec![[1.0, 1.0]],
            pressures: Vec::new(),
            radius: 2.0,
            frame,
        };
        let strokes = vec![stroke(Some(3)), stroke(Some(7)), stroke(None)];
        assert_eq!(strokes_for_frame(&strokes, 3).len(), 2);
        assert_eq!(strokes_for_frame(&strokes, 7).len(), 2);
        assert_eq!(strokes_for_frame(&strokes, 5).len(), 1);
    }

    #[test]
    fn distance_field_matches_exact_nearest_euclidean_distance() {
        let distances = distance_field_from_points(5, 5, &[[1.0, 1.0], [3.0, 3.0]]);
        assert_eq!(distances[5 + 1], 0.0);
        for y in 0..5 {
            for x in 0..5 {
                let expected = [[1.0f32, 1.0f32], [3.0, 3.0]]
                    .iter()
                    .map(|point| (x as f32 - point[0]).powi(2) + (y as f32 - point[1]).powi(2))
                    .fold(f32::INFINITY, f32::min);
                assert!((distances[y * 5 + x] - expected).abs() < 1e-5);
            }
        }
    }

    #[test]
    fn geodesic_distance_penalizes_crossing_a_strong_image_edge() {
        let (width, height) = (16u32, 8u32);
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        for y in 0..height {
            for x in 0..width {
                let index = ((y * width + x) * 4) as usize;
                let value = if x < 8 { 32 } else { 224 };
                pixels[index..index + 4].copy_from_slice(&[value, value, value, 255]);
            }
        }

        let distances = geodesic_distance_field(&pixels, width, height, &[[2.0, 4.0]], || false)
            .expect("distance field should complete");
        let before_edge = distances[(4 * width + 6) as usize];
        let after_edge = distances[(4 * width + 12) as usize];

        assert!(
            after_edge > before_edge * 3,
            "before={before_edge}, after={after_edge}"
        );
    }

    #[test]
    fn geodesic_distance_field_stops_when_cancelled() {
        let (width, height) = (128u32, 128u32);
        let pixels = vec![128u8; (width * height * 4) as usize];
        let mut checks = 0;
        let distances = geodesic_distance_field(&pixels, width, height, &[[0.0, 0.0]], || {
            checks += 1;
            checks >= 2
        });

        assert!(distances.is_none());
        assert_eq!(checks, 2);
    }

    #[test]
    fn roto_matte_follows_a_high_contrast_subject_boundary() {
        let (width, height) = (48usize, 32usize);
        let mut pixels = vec![0u8; width * height * 4];
        for y in 0..height {
            for x in 0..width {
                let index = (y * width + x) * 4;
                let color: [u8; 3] = if (16..32).contains(&x) && (4..28).contains(&y) {
                    [210, 35, 28]
                } else {
                    [42, 45, 48]
                };
                pixels[index..index + 4].copy_from_slice(&[color[0], color[1], color[2], 255]);
            }
        }
        let strokes = [
            RotoStroke {
                stroke_type: RotoStrokeType::Foreground,
                points: vec![[24.0, 8.0], [24.0, 16.0], [24.0, 24.0]],
                pressures: Vec::new(),
                radius: 2.0,
                frame: None,
            },
            RotoStroke {
                stroke_type: RotoStrokeType::Background,
                points: vec![[8.0, 16.0]],
                pressures: Vec::new(),
                radius: 2.0,
                frame: None,
            },
            RotoStroke {
                stroke_type: RotoStrokeType::Background,
                points: vec![[40.0, 16.0]],
                pressures: Vec::new(),
                radius: 2.0,
                frame: None,
            },
        ];
        let settings = RotoBrushSettings {
            feather_radius: 0.0,
            ..Default::default()
        };

        let matte =
            generate_rotobrush_matte(&pixels, width as u32, height as u32, &strokes, &settings);

        assert!(matte[12 * width + 24] > 200);
        assert!(matte[16 * width + 12] < 80);
        assert!(matte[16 * width + 15] < 128);
        assert!(
            matte[16 * width + 17] > 128,
            "interior edge pixel alpha={}",
            matte[16 * width + 17]
        );
    }

    #[test]
    fn separable_feather_matches_clamped_square_box_filter() {
        let input = [0u8, 40, 100, 160, 220, 255];
        let mut actual = input;
        refine_edge_matte(&mut actual, 3, 2, 1.0);
        let expected: Vec<u8> = (0..2)
            .flat_map(|y| {
                (0..3).map(move |x| {
                    let sum: u32 = (-1isize..=1)
                        .flat_map(|dy| {
                            (-1isize..=1).map(move |dx| {
                                let sx = (x as isize + dx).clamp(0, 2) as usize;
                                let sy = (y as isize + dy).clamp(0, 1) as usize;
                                input[sy * 3 + sx] as u32
                            })
                        })
                        .sum();
                    (sum as f32 / 9.0).round() as u8
                })
            })
            .collect();
        assert_eq!(actual.as_slice(), expected);
    }

    #[test]
    fn roto_matte_generation_stops_when_cancelled_during_distance_field_construction() {
        let width = 64;
        let height = 48;
        let pixels = vec![127u8; width * height * 4];
        let strokes = [
            RotoStroke {
                stroke_type: RotoStrokeType::Foreground,
                points: vec![[12.0, 20.0]],
                pressures: Vec::new(),
                radius: 4.0,
                frame: None,
            },
            RotoStroke {
                stroke_type: RotoStrokeType::Background,
                points: vec![[50.0, 20.0]],
                pressures: Vec::new(),
                radius: 4.0,
                frame: None,
            },
        ];
        let settings = RotoBrushSettings::default();
        let mut checkpoints = 0;
        let result = generate_rotobrush_matte_cancellable(
            &pixels,
            width as u32,
            height as u32,
            &strokes,
            &settings,
            || {
                checkpoints += 1;
                checkpoints >= 30
            },
        );

        assert!(result.is_none());
        assert_eq!(checkpoints, 30);
    }
}
