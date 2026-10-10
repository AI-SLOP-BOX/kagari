#[cfg(test)]
mod tests {
    use crate::core::roto_brush_engine::filter::*;
    use crate::core::roto_brush_engine::*;

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
                radius: 4.0,
                frame: None,
            },
            RotoStroke {
                stroke_type: RotoStrokeType::Background,
                points: vec![[24.0, 16.0]],
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
                radius: 1.0,
                frame: None,
            },
            RotoStroke {
                stroke_type: RotoStrokeType::Background,
                points: vec![[9.0, 4.0]],
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
    fn fast_roto_drag_interpolates_brush_seeds_without_gaps() {
        let width = 16;
        let height = 8;
        let pixels = vec![96u8; width * height * 4];
        let stroke = RotoStroke {
            stroke_type: RotoStrokeType::Foreground,
            points: vec![[2.0, 4.0], [13.0, 4.0]],
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
}
