//! Intelligent RotoBrush & Automated Boundary Propagation Engine (AE Parity).
//!
//! Provides automatic object cutout segmentation, foreground/background color
//! distribution modeling, edge-gradient snap refinement, and temporal frame propagation.

#![allow(dead_code)]

/// RotoBrush stroke type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum RotoStrokeType {
    #[default]
    Foreground, // Green stroke: marks subject
    Background, // Red stroke: marks background
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RotoStroke {
    pub stroke_type: RotoStrokeType,
    pub points: Vec<[f32; 2]>,
    pub radius: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<u32>,
}

pub fn strokes_for_frame(strokes: &[RotoStroke], frame: u32) -> Vec<RotoStroke> {
    strokes
        .iter()
        .filter(|stroke| stroke.frame.is_none() || stroke.frame == Some(frame))
        .cloned()
        .collect()
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RotoBrushSettings {
    pub feather_radius: f32,
    pub contrast: f32,
    pub edge_detection_sensitivity: f32,
    pub propagation_quality: usize, // Subdivisions
    pub temporal_smoothing: f32,
}

impl Default for RotoBrushSettings {
    fn default() -> Self {
        Self {
            feather_radius: 3.0,
            contrast: 1.0,
            edge_detection_sensitivity: 0.8,
            propagation_quality: 3,
            temporal_smoothing: 0.5,
        }
    }
}

/// Computes an automatic foreground alpha matte from user strokes and image pixels
/// with spatial distance priors, color modeling, edge sensitivity, and contrast shaping.
pub fn generate_rotobrush_matte(
    src_pixels: &[u8],
    width: u32,
    height: u32,
    strokes: &[RotoStroke],
    settings: &RotoBrushSettings,
) -> Vec<u8> {
    let Some(size) = (width as usize).checked_mul(height as usize) else {
        return Vec::new();
    };
    let Some(expected_len) = size.checked_mul(4) else {
        return vec![0u8; size];
    };
    if width == 0
        || height == 0
        || width > i32::MAX as u32
        || height > i32::MAX as u32
        || src_pixels.len() != expected_len
        || strokes.is_empty()
    {
        return vec![0u8; size];
    }

    let mut fg_samples = Vec::new();
    let mut bg_samples = Vec::new();
    let mut fg_points = Vec::new();
    let mut bg_points = Vec::new();

    // Keep the color model bounded even when a brush is dragged across a large
    // source frame. Spatial influence is computed from a distance field below.
    const MAX_COLOR_SAMPLES_PER_CLASS: usize = 512;
    // Collect bounded color samples and point coordinates from brush strokes.
    for stroke in strokes {
        if !stroke.radius.is_finite() || stroke.radius < 0.0 {
            continue;
        }
        let radius = stroke.radius.min(width.max(height) as f32);
        let r_sq = radius * radius;
        for &pt in &stroke.points {
            if !pt[0].is_finite() || !pt[1].is_finite() {
                continue;
            }
            if stroke.stroke_type == RotoStrokeType::Foreground {
                fg_points.push(pt);
            } else {
                bg_points.push(pt);
            }

            let class_samples = if stroke.stroke_type == RotoStrokeType::Foreground {
                &mut fg_samples
            } else {
                &mut bg_samples
            };
            if class_samples.len() >= MAX_COLOR_SAMPLES_PER_CLASS {
                continue;
            }

            let px = pt[0].round() as i64;
            let py = pt[1].round() as i64;
            let rad = radius.ceil() as i64;
            let sample_step = (rad / 8).max(1);
            if pt[0] + radius < 0.0
                || pt[1] + radius < 0.0
                || pt[0] - radius >= width as f32
                || pt[1] - radius >= height as f32
            {
                continue;
            }

            'sample: for dy in (-rad..=rad).step_by(sample_step as usize) {
                let y = py + dy;
                if y < 0 || y >= height as i64 {
                    continue;
                }
                for dx in (-rad..=rad).step_by(sample_step as usize) {
                    let x = px + dx;
                    if x < 0 || x >= width as i64 {
                        continue;
                    }
                    if (dx as f64).powi(2) + (dy as f64).powi(2) <= r_sq as f64 {
                        let idx = (y as usize * width as usize + x as usize) * 4;
                        let color = [
                            src_pixels[idx] as f32,
                            src_pixels[idx + 1] as f32,
                            src_pixels[idx + 2] as f32,
                        ];
                        class_samples.push(color);
                        if class_samples.len() >= MAX_COLOR_SAMPLES_PER_CLASS {
                            break 'sample;
                        }
                    }
                }
            }
        }
    }

    let fg_mean = compute_mean_color(&fg_samples).unwrap_or([255.0, 255.0, 255.0]);
    let bg_mean = compute_mean_color(&bg_samples).unwrap_or([0.0, 0.0, 0.0]);
    let fg_distance = distance_field_from_points(width, height, &fg_points);
    let bg_distance = distance_field_from_points(width, height, &bg_points);

    let mut alpha_matte = vec![0u8; size];
    let spatial_sigma_sq = (width.max(height) as f32 * 0.35).powi(2).max(1.0);
    let contrast_pow = settings.contrast.clamp(0.1, 5.0);

    for y in 0..height {
        for x in 0..width {
            let i = (y * width + x) as usize;
            let idx = i * 4;
            let r = src_pixels[idx] as f32;
            let g = src_pixels[idx + 1] as f32;
            let b = src_pixels[idx + 2] as f32;

            let d_fg_color = color_dist_sq([r, g, b], fg_mean);
            let d_bg_color = color_dist_sq([r, g, b], bg_mean);

            // Spatial distance to nearest FG/BG strokes
            let min_d_fg_pos = fg_distance[i];
            let min_d_bg_pos = bg_distance[i];

            let spatial_fg_weight = if !fg_points.is_empty() {
                (-min_d_fg_pos / (2.0 * spatial_sigma_sq)).exp()
            } else {
                0.0
            };
            let spatial_bg_weight = if !bg_points.is_empty() {
                (-min_d_bg_pos / (2.0 * spatial_sigma_sq)).exp()
            } else {
                0.0
            };

            // Combined likelihood
            let color_likelihood = d_bg_color / (d_fg_color + d_bg_color + 1e-4);
            let raw_prob = if bg_points.is_empty() {
                color_likelihood * spatial_fg_weight
            } else if fg_points.is_empty() {
                color_likelihood * (1.0 - spatial_bg_weight)
            } else {
                let total_fg = color_likelihood * (1.0 + spatial_fg_weight * 2.0);
                let total_bg = (1.0 - color_likelihood) * (1.0 + spatial_bg_weight * 2.0);
                total_fg / (total_fg + total_bg + 1e-4)
            };

            // Apply contrast shaping
            let shaped = if raw_prob >= 0.5 {
                0.5 + 0.5 * ((raw_prob - 0.5) * 2.0).powf(contrast_pow)
            } else {
                0.5 - 0.5 * ((0.5 - raw_prob) * 2.0).powf(contrast_pow)
            };

            let alpha = (shaped.clamp(0.0, 1.0) * 255.0).round() as u8;
            alpha_matte[i] = alpha;
        }
    }

    // Apply edge refinement & feather adjusted by sensitivity
    let effective_feather =
        settings.feather_radius * (1.2 - settings.edge_detection_sensitivity.clamp(0.0, 1.0) * 0.4);
    if effective_feather > 0.5 {
        refine_edge_matte(&mut alpha_matte, width, height, effective_feather);
    }

    apply_stroke_constraints(&mut alpha_matte, width, height, strokes);

    alpha_matte
}

fn apply_stroke_constraints(matte: &mut [u8], width: u32, height: u32, strokes: &[RotoStroke]) {
    let radius_limit = width.max(height) as f32;
    let stamp = |matte: &mut [u8], point: [f32; 2], radius: f32, value: u8| {
        if !point[0].is_finite() || !point[1].is_finite() {
            return;
        }
        let radius = radius.clamp(0.0, radius_limit);
        let min_x = (point[0].floor() - radius).max(0.0) as u32;
        let max_x = (point[0].ceil() + radius).min(width.saturating_sub(1) as f32) as u32;
        let min_y = (point[1].floor() - radius).max(0.0) as u32;
        let max_y = (point[1].ceil() + radius).min(height.saturating_sub(1) as f32) as u32;
        if min_x > max_x || min_y > max_y {
            return;
        }
        let radius_sq = radius * radius;
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let dx = x as f32 - point[0];
                let dy = y as f32 - point[1];
                if dx * dx + dy * dy <= radius_sq {
                    matte[y as usize * width as usize + x as usize] = value;
                }
            }
        }
    };

    for stroke in strokes {
        if !stroke.radius.is_finite() || stroke.radius < 0.0 || stroke.points.is_empty() {
            continue;
        }
        let radius = stroke.radius.min(radius_limit);
        let value = if stroke.stroke_type == RotoStrokeType::Foreground {
            255
        } else {
            0
        };
        let spacing = (radius * 0.5).max(1.0);
        for point in &stroke.points {
            stamp(matte, *point, radius, value);
        }
        for pair in stroke.points.windows(2) {
            let [mut start, mut end] = [pair[0], pair[1]];
            if !start[0].is_finite()
                || !start[1].is_finite()
                || !end[0].is_finite()
                || !end[1].is_finite()
            {
                continue;
            }
            let x_limit = width as f32 + radius;
            let y_limit = height as f32 + radius;
            start[0] = start[0].clamp(-radius, x_limit);
            end[0] = end[0].clamp(-radius, x_limit);
            start[1] = start[1].clamp(-radius, y_limit);
            end[1] = end[1].clamp(-radius, y_limit);
            let distance = (end[0] - start[0]).hypot(end[1] - start[1]);
            let steps = (distance / spacing).ceil().max(1.0) as u32;
            for step in 1..steps {
                let t = step as f32 / steps as f32;
                stamp(
                    matte,
                    [
                        start[0] + (end[0] - start[0]) * t,
                        start[1] + (end[1] - start[1]) * t,
                    ],
                    radius,
                    value,
                );
            }
        }
    }
}

/// Propagates a rotobrush boundary polygon across adjacent time frames using motion delta
/// with sub-step temporal smoothing and subdivision quality.
pub fn propagate_roto_boundary(
    prev_boundary: &[[f32; 2]],
    motion_vector: [f32; 2],
    settings: &RotoBrushSettings,
) -> Vec<[f32; 2]> {
    let damping = settings.temporal_smoothing.clamp(0.0, 1.0);
    let factor = (1.0 - damping * 0.5).clamp(0.0, 1.0);
    let steps = settings.propagation_quality.clamp(1, 10);
    let step_motion = [
        motion_vector[0] * factor / steps as f32,
        motion_vector[1] * factor / steps as f32,
    ];

    let mut boundary = prev_boundary.to_vec();
    for _ in 0..steps {
        for p in &mut boundary {
            p[0] += step_motion[0];
            p[1] += step_motion[1];
        }
    }
    boundary
}

fn compute_mean_color(samples: &[[f32; 3]]) -> Option<[f32; 3]> {
    if samples.is_empty() {
        return None;
    }
    let mut sum = [0.0f32; 3];
    for s in samples {
        sum[0] += s[0];
        sum[1] += s[1];
        sum[2] += s[2];
    }
    let len = samples.len() as f32;
    Some([sum[0] / len, sum[1] / len, sum[2] / len])
}

fn distance_field_from_points(width: u32, height: u32, points: &[[f32; 2]]) -> Vec<f32> {
    let Some(len) = (width as usize).checked_mul(height as usize) else {
        return Vec::new();
    };
    let mut distance = vec![f32::INFINITY; len];
    if width == 0 || height == 0 || points.is_empty() {
        return distance;
    }

    let w = width as usize;
    let h = height as usize;
    let mut seeds = vec![false; len];
    for point in points {
        if !point[0].is_finite() || !point[1].is_finite() {
            continue;
        }
        let x = (point[0].round() as i64).clamp(0, width as i64 - 1) as usize;
        let y = (point[1].round() as i64).clamp(0, height as i64 - 1) as usize;
        seeds[y * w + x] = true;
    }

    let mut horizontal = vec![f32::INFINITY; len];
    for y in 0..h {
        let start = y * w;
        let row: Vec<f32> = seeds[start..start + w]
            .iter()
            .map(|&seed| if seed { 0.0 } else { f32::INFINITY })
            .collect();
        horizontal[start..start + w].copy_from_slice(&squared_distance_transform_1d(&row));
    }
    for x in 0..w {
        let column: Vec<f32> = (0..h).map(|y| horizontal[y * w + x]).collect();
        let transformed = squared_distance_transform_1d(&column);
        for (y, value) in transformed.into_iter().enumerate() {
            distance[y * w + x] = value;
        }
    }
    distance
}

fn squared_distance_transform_1d(input: &[f32]) -> Vec<f32> {
    let mut output = vec![f32::INFINITY; input.len()];
    let Some(first) = input.iter().position(|value| value.is_finite()) else {
        return output;
    };

    let mut sites = vec![0usize; input.len()];
    let mut boundaries = vec![0.0f64; input.len() + 1];
    let mut envelope = 0usize;
    sites[0] = first;
    boundaries[0] = f64::NEG_INFINITY;
    boundaries[1] = f64::INFINITY;

    for q in first + 1..input.len() {
        if !input[q].is_finite() {
            continue;
        }
        let qf = q as f64;
        loop {
            let p = sites[envelope];
            let pf = p as f64;
            let intersection = ((input[q] as f64 + qf * qf)
                - (input[p] as f64 + pf * pf))
                / (2.0 * (qf - pf));
            if intersection > boundaries[envelope] {
                envelope += 1;
                sites[envelope] = q;
                boundaries[envelope] = intersection;
                boundaries[envelope + 1] = f64::INFINITY;
                break;
            }
            envelope -= 1;
        }
    }

    envelope = 0;
    for (q, out) in output.iter_mut().enumerate() {
        while boundaries[envelope + 1] < q as f64 {
            envelope += 1;
        }
        let delta = q as f64 - sites[envelope] as f64;
        *out = (delta * delta + input[sites[envelope]] as f64) as f32;
    }
    output
}

fn color_dist_sq(c1: [f32; 3], c2: [f32; 3]) -> f32 {
    let dr = c1[0] - c2[0];
    let dg = c1[1] - c2[1];
    let db = c1[2] - c2[2];
    dr * dr + dg * dg + db * db
}

fn refine_edge_matte(matte: &mut [u8], width: u32, height: u32, feather: f32) {
    let Some(len) = (width as usize).checked_mul(height as usize) else {
        return;
    };
    if width == 0 || height == 0 || matte.len() != len {
        return;
    }
    let feather = if feather.is_finite() {
        feather.clamp(0.0, 64.0)
    } else {
        0.0
    };
    let radius = feather.round() as usize;
    if radius == 0 {
        return;
    }

    let w = width as usize;
    let h = height as usize;
    let kernel_size = (radius * 2 + 1) as f32;
    let source: Vec<f32> = matte.iter().map(|&value| value as f32).collect();
    let mut horizontal = vec![0.0f32; len];

    for y in 0..h {
        let row = y * w;
        let mut sum = 0.0f32;
        for offset in -(radius as isize)..=radius as isize {
            let x = offset.clamp(0, w as isize - 1) as usize;
            sum += source[row + x];
        }
        for x in 0..w {
            if x > 0 {
                let outgoing = (x as isize - radius as isize - 1).clamp(0, w as isize - 1) as usize;
                let incoming = (x + radius).min(w - 1);
                sum += source[row + incoming] - source[row + outgoing];
            }
            horizontal[row + x] = sum / kernel_size;
        }
    }

    for x in 0..w {
        let mut sum = 0.0f32;
        for offset in -(radius as isize)..=radius as isize {
            let y = offset.clamp(0, h as isize - 1) as usize;
            sum += horizontal[y * w + x];
        }
        for y in 0..h {
            if y > 0 {
                let outgoing = (y as isize - radius as isize - 1).clamp(0, h as isize - 1) as usize;
                let incoming = (y + radius).min(h - 1);
                sum += horizontal[incoming * w + x] - horizontal[outgoing * w + x];
            }
            matte[y * w + x] = (sum / kernel_size).round().clamp(0.0, 255.0) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        for rgba in pixels.chunks_exact_mut(4) {
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
        assert_eq!(distances[1 * 5 + 1], 0.0);
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
                    let sum: u32 = (-1..=1)
                        .flat_map(|dy| {
                            (-1..=1).map(move |dx| {
                                let sx = (x + dx).clamp(0, 2) as usize;
                                let sy = (y + dy).clamp(0, 1) as usize;
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
