//! Matte generation and boundary temporal propagation.

use super::filter::{
    color_dist_sq, compute_mean_color, distance_field_from_points, refine_edge_matte,
};
use super::types::{RotoBrushSettings, RotoStroke, RotoStrokeType};

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
