//! Matte generation and boundary temporal propagation.

use super::filter::{
    color_dist_sq, color_prototypes, compute_mean_color,
    distance_field_from_point_slices_cancellable, geodesic_distance_field,
    refine_edge_matte_cancellable,
};
use super::types::{RotoBrushSettings, RotoStroke, RotoStrokeType};

/// Bounds the distance-field and heap working set to roughly 200 MiB at peak.
pub const MAX_ROTO_FRAME_PIXELS: usize = 8_388_608;

fn try_filled_vec<T: Clone>(len: usize, value: T) -> Option<Vec<T>> {
    let mut values = Vec::new();
    values.try_reserve_exact(len).ok()?;
    values.resize(len, value);
    Some(values)
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
    let Some(matte) =
        generate_rotobrush_matte_cancellable(src_pixels, width, height, strokes, settings, || {
            false
        })
    else {
        return Vec::new();
    };
    matte
}

pub fn generate_rotobrush_matte_cancellable(
    src_pixels: &[u8],
    width: u32,
    height: u32,
    strokes: &[RotoStroke],
    settings: &RotoBrushSettings,
    mut is_cancelled: impl FnMut() -> bool,
) -> Option<Vec<u8>> {
    if width == 0 || height == 0 || width > i32::MAX as u32 || height > i32::MAX as u32 {
        return Some(Vec::new());
    }
    let Some(size) = (width as usize).checked_mul(height as usize) else {
        return Some(Vec::new());
    };
    let Some(expected_len) = size.checked_mul(4) else {
        return Some(Vec::new());
    };
    if size > MAX_ROTO_FRAME_PIXELS {
        return Some(Vec::new());
    }
    if src_pixels.len() != expected_len {
        return try_filled_vec(size, 0u8);
    }
    if strokes.is_empty() {
        return try_filled_vec(size, 0u8);
    }
    if is_cancelled() {
        return None;
    }

    let mut fg_samples = Vec::new();
    let mut bg_samples = Vec::new();
    let mut fg_points = Vec::new();
    let mut bg_points = Vec::new();

    // Keep the color model bounded while sampling throughout each stroke. A
    // simple first-N cap biases long strokes toward their starting point.
    const MAX_COLOR_SAMPLES_PER_CLASS: usize = 512;
    let mut point_counts = [0usize; 2];
    for stroke in strokes {
        if is_cancelled() {
            return None;
        }
        if !stroke.radius.is_finite() || stroke.radius < 0.0 {
            continue;
        }
        let class_index = usize::from(stroke.stroke_type == RotoStrokeType::Background);
        point_counts[class_index] = point_counts[class_index].saturating_add(
            stroke
                .points
                .iter()
                .filter(|point| point.iter().all(|coordinate| coordinate.is_finite()))
                .count(),
        );
    }
    let sample_plans = [
        bounded_color_sample_plan(point_counts[0], MAX_COLOR_SAMPLES_PER_CLASS),
        bounded_color_sample_plan(point_counts[1], MAX_COLOR_SAMPLES_PER_CLASS),
    ];
    fg_points.try_reserve_exact(point_counts[0]).ok()?;
    bg_points.try_reserve_exact(point_counts[1]).ok()?;
    fg_samples
        .try_reserve_exact(MAX_COLOR_SAMPLES_PER_CLASS)
        .ok()?;
    bg_samples
        .try_reserve_exact(MAX_COLOR_SAMPLES_PER_CLASS)
        .ok()?;
    let mut point_indices = [0usize; 2];
    // Collect bounded color samples and point coordinates from brush strokes.
    for stroke in strokes {
        if is_cancelled() {
            return None;
        }
        if !stroke.radius.is_finite() || stroke.radius < 0.0 {
            continue;
        }
        for (stroke_point_index, &pt) in stroke.points.iter().enumerate() {
            if is_cancelled() {
                return None;
            }
            if !pt[0].is_finite() || !pt[1].is_finite() {
                continue;
            }
            let radius = stroke_radius_at(stroke, stroke_point_index, width.max(height) as f32);
            let r_sq = radius * radius;
            if stroke.stroke_type == RotoStrokeType::Foreground {
                fg_points.push(pt);
            } else {
                bg_points.push(pt);
            }

            let class_index = usize::from(stroke.stroke_type == RotoStrokeType::Background);
            let point_index = point_indices[class_index];
            point_indices[class_index] = point_index.saturating_add(1);
            let (point_stride, samples_per_point) = sample_plans[class_index];
            if point_index % point_stride != 0 || samples_per_point == 0 {
                continue;
            }
            let class_samples = if class_index == 0 {
                &mut fg_samples
            } else {
                &mut bg_samples
            };
            let sample_limit = class_samples.len().saturating_add(samples_per_point);

            let px = pt[0].round() as i64;
            let py = pt[1].round() as i64;
            let rad = radius.ceil() as i64;
            let sample_step =
                ((radius * 2.0 / (samples_per_point as f32).sqrt()).ceil() as i64).max(1);
            if pt[0] + radius < 0.0
                || pt[1] + radius < 0.0
                || pt[0] - radius >= width as f32
                || pt[1] - radius >= height as f32
            {
                continue;
            }

            // Always include the painted center so very long strokes with a
            // one-color quota still contribute a representative sample.
            let center_x = pt[0].round() as i64;
            let center_y = pt[1].round() as i64;
            if center_x >= 0 && center_x < width as i64 && center_y >= 0 && center_y < height as i64
            {
                let idx = (center_y as usize * width as usize + center_x as usize) * 4;
                class_samples.push([
                    src_pixels[idx] as f32,
                    src_pixels[idx + 1] as f32,
                    src_pixels[idx + 2] as f32,
                ]);
            }
            if class_samples.len() >= sample_limit {
                continue;
            }
            'sample: for dy in (-rad..=rad).step_by(sample_step as usize) {
                if is_cancelled() {
                    return None;
                }
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
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let color = [
                            src_pixels[idx] as f32,
                            src_pixels[idx + 1] as f32,
                            src_pixels[idx + 2] as f32,
                        ];
                        if class_samples.len() >= sample_limit {
                            break 'sample;
                        }
                        class_samples.push(color);
                    }
                }
            }
        }
    }

    if fg_points.is_empty() {
        return try_filled_vec(size, 0u8);
    }

    const MAX_COLOR_PROTOTYPES: usize = 8;
    let fg_prototypes = color_prototypes(&fg_samples, MAX_COLOR_PROTOTYPES);
    let bg_prototypes = color_prototypes(&bg_samples, MAX_COLOR_PROTOTYPES);
    let fg_fallback = compute_mean_color(&fg_samples).unwrap_or([255.0, 255.0, 255.0]);
    let bg_fallback = compute_mean_color(&bg_samples).unwrap_or([0.0, 0.0, 0.0]);
    let stroke_distance = distance_field_from_point_slices_cancellable(
        width,
        height,
        &[&fg_points, &bg_points],
        &mut is_cancelled,
    )?;
    let fg_distance =
        geodesic_distance_field(src_pixels, width, height, &fg_points, &mut is_cancelled)?;
    let bg_distance =
        geodesic_distance_field(src_pixels, width, height, &bg_points, &mut is_cancelled)?;

    let mut alpha_matte = try_filled_vec(size, 0u8)?;
    // Keep stroke influence local to the marked subject. A composition-wide
    // prior made distant, unmarked regions effectively neutral and therefore
    // foreground whenever their colors resembled the foreground samples.
    let spatial_decay = (width.max(height) as f32 * 0.12).max(10.0);
    let contrast_pow = if settings.contrast.is_finite() {
        settings.contrast.clamp(0.1, 5.0)
    } else {
        RotoBrushSettings::default().contrast
    };

    for y in 0..height {
        if is_cancelled() {
            return None;
        }
        for x in 0..width {
            let i = (y * width + x) as usize;
            let idx = i * 4;
            let r = src_pixels[idx] as f32;
            let g = src_pixels[idx + 1] as f32;
            let b = src_pixels[idx + 2] as f32;

            let pixel_color = [r, g, b];
            let d_fg_color = if fg_prototypes.is_empty() {
                color_dist_sq(pixel_color, fg_fallback)
            } else {
                fg_prototypes
                    .iter()
                    .map(|prototype| color_dist_sq(pixel_color, *prototype))
                    .fold(f32::INFINITY, f32::min)
            };
            let d_bg_color = if bg_prototypes.is_empty() {
                color_dist_sq(pixel_color, bg_fallback)
            } else {
                bg_prototypes
                    .iter()
                    .map(|prototype| color_dist_sq(pixel_color, *prototype))
                    .fold(f32::INFINITY, f32::min)
            };

            // Spatial distance to nearest FG/BG strokes
            let min_d_fg_pos = fg_distance[i];
            let min_d_bg_pos = bg_distance[i];

            let spatial_fg_weight = if !fg_points.is_empty() && min_d_fg_pos != u32::MAX {
                (-(min_d_fg_pos as f32) / spatial_decay).exp()
            } else {
                0.0
            };
            // Combined likelihood
            let color_likelihood = d_bg_color / (d_fg_color + d_bg_color + 1e-4);
            let raw_prob = if bg_points.is_empty() {
                color_likelihood * spatial_fg_weight
            } else {
                // Combine appearance and brush proximity as log-odds. A
                // slight background prior breaks exact ties toward removal;
                // otherwise an unclassified 50% pixel survives the matte's
                // 50% contour threshold and can connect unrelated regions.
                let color_log_odds = (d_bg_color + 1e-3).ln() - (d_fg_color + 1e-3).ln();
                let fg_path_cost = (min_d_fg_pos as f32 + 10.0).ln();
                let bg_path_cost = (min_d_bg_pos as f32 + 10.0).ln();
                let path_log_odds = bg_path_cost - fg_path_cost;
                // Fade only positive foreground evidence when a pixel is far
                // from every user stroke. Nearby boundaries keep their full
                // geodesic evidence, while distant unmarked regions default BG.
                let nearest_stroke_distance = stroke_distance[i].sqrt();
                let unmarked_distance = (nearest_stroke_distance - spatial_decay * 2.0).max(0.0);
                let spatial_confidence = (-unmarked_distance / spatial_decay).exp();
                let appearance_evidence = color_log_odds.clamp(-4.0, 4.0) * 0.1;
                let foreground_evidence = path_log_odds + appearance_evidence;
                let foreground_confidence = if foreground_evidence > 0.0 {
                    spatial_confidence
                } else {
                    1.0
                };
                let log_odds =
                    (foreground_evidence * foreground_confidence - 0.1).clamp(-20.0, 20.0);
                1.0 / (1.0 + (-log_odds).exp())
            };

            // Apply contrast shaping
            let shaped: f32 = if raw_prob >= 0.5 {
                0.5 + 0.5 * ((raw_prob - 0.5) * 2.0).powf(contrast_pow)
            } else {
                0.5 - 0.5 * ((0.5 - raw_prob) * 2.0).powf(contrast_pow)
            };

            let alpha = (shaped.clamp(0.0, 1.0) * 255.0).round() as u8;
            alpha_matte[i] = alpha;
        }
    }

    // Apply edge refinement & feather adjusted by sensitivity
    let defaults = RotoBrushSettings::default();
    let feather_radius = if settings.feather_radius.is_finite() {
        settings.feather_radius.clamp(0.0, 256.0)
    } else {
        defaults.feather_radius
    };
    let edge_sensitivity = if settings.edge_detection_sensitivity.is_finite() {
        settings.edge_detection_sensitivity.clamp(0.0, 1.0)
    } else {
        defaults.edge_detection_sensitivity
    };
    let effective_feather = feather_radius * (1.2 - edge_sensitivity * 0.4);
    if effective_feather > 0.5 {
        refine_edge_matte_cancellable(
            &mut alpha_matte,
            width,
            height,
            effective_feather,
            &mut is_cancelled,
        )?;
    }

    if is_cancelled() {
        return None;
    }
    apply_stroke_constraints(&mut alpha_matte, width, height, strokes, &mut is_cancelled)?;

    Some(alpha_matte)
}

pub(super) fn bounded_color_sample_plan(point_count: usize, sample_limit: usize) -> (usize, usize) {
    if point_count == 0 || sample_limit == 0 {
        return (1, 0);
    }
    let stride = point_count.div_ceil(sample_limit).max(1);
    let selected_points = point_count.div_ceil(stride);
    let samples_per_point = (sample_limit / selected_points).max(1);
    (stride, samples_per_point)
}

fn stamp_constraint(
    matte: &mut [u8],
    width: u32,
    height: u32,
    point: [f32; 2],
    radius: f32,
    value: u8,
    is_cancelled: &mut impl FnMut() -> bool,
) -> Option<()> {
    if !point[0].is_finite() || !point[1].is_finite() {
        return Some(());
    }
    let min_x = (point[0].floor() - radius).max(0.0) as u32;
    let max_x = (point[0].ceil() + radius).min(width.saturating_sub(1) as f32) as u32;
    let min_y = (point[1].floor() - radius).max(0.0) as u32;
    let max_y = (point[1].ceil() + radius).min(height.saturating_sub(1) as f32) as u32;
    if min_x > max_x || min_y > max_y {
        return Some(());
    }
    let radius_sq = radius * radius;
    for y in min_y..=max_y {
        if is_cancelled() {
            return None;
        }
        for x in min_x..=max_x {
            let dx = x as f32 - point[0];
            let dy = y as f32 - point[1];
            if dx * dx + dy * dy <= radius_sq {
                matte[y as usize * width as usize + x as usize] = value;
            }
        }
    }
    Some(())
}

pub(crate) fn apply_stroke_constraints(
    matte: &mut [u8],
    width: u32,
    height: u32,
    strokes: &[RotoStroke],
    is_cancelled: &mut impl FnMut() -> bool,
) -> Option<()> {
    let radius_limit = width.max(height) as f32;

    for stroke in strokes {
        if is_cancelled() {
            return None;
        }
        if !stroke.radius.is_finite() || stroke.radius < 0.0 || stroke.points.is_empty() {
            continue;
        }
        let value = if stroke.stroke_type == RotoStrokeType::Foreground {
            255
        } else {
            0
        };
        for (point_index, point) in stroke.points.iter().enumerate() {
            if is_cancelled() {
                return None;
            }
            let radius = stroke_radius_at(stroke, point_index, radius_limit);
            stamp_constraint(matte, width, height, *point, radius, value, is_cancelled)?;
        }
        for (point_index, pair) in stroke.points.windows(2).enumerate() {
            if is_cancelled() {
                return None;
            }
            let start_radius = stroke_radius_at(stroke, point_index, radius_limit);
            let end_radius = stroke_radius_at(stroke, point_index + 1, radius_limit);
            let max_radius = start_radius.max(end_radius);
            let [mut start, mut end] = [pair[0], pair[1]];
            if !start[0].is_finite()
                || !start[1].is_finite()
                || !end[0].is_finite()
                || !end[1].is_finite()
            {
                continue;
            }
            // Reject segments that cannot reach the image even with their
            // largest brush radius. Clamping these endpoints first would
            // collapse distant off-canvas strokes onto the image border.
            if start[0].max(end[0]) < -max_radius
                || start[1].max(end[1]) < -max_radius
                || start[0].min(end[0]) > width as f32 + max_radius
                || start[1].min(end[1]) > height as f32 + max_radius
            {
                continue;
            }
            let x_limit = width as f32 + max_radius;
            let y_limit = height as f32 + max_radius;
            start[0] = start[0].clamp(-max_radius, x_limit);
            end[0] = end[0].clamp(-max_radius, x_limit);
            start[1] = start[1].clamp(-max_radius, y_limit);
            end[1] = end[1].clamp(-max_radius, y_limit);
            let distance = (end[0] - start[0]).hypot(end[1] - start[1]);
            let spacing = (max_radius * 0.5).max(1.0);
            let steps = (distance / spacing).ceil().max(1.0) as u32;
            for step in 1..steps {
                let t = step as f32 / steps as f32;
                stamp_constraint(
                    matte,
                    width,
                    height,
                    [
                        start[0] + (end[0] - start[0]) * t,
                        start[1] + (end[1] - start[1]) * t,
                    ],
                    start_radius + (end_radius - start_radius) * t,
                    value,
                    is_cancelled,
                )?;
            }
        }
    }
    Some(())
}

fn stroke_radius_at(stroke: &RotoStroke, point_index: usize, radius_limit: f32) -> f32 {
    let radius = stroke.radius.min(radius_limit);
    let pressure = if stroke.pressures.len() == stroke.points.len() {
        stroke
            .pressures
            .get(point_index)
            .copied()
            .filter(|value| value.is_finite())
            .map(|value| value.clamp(0.0, 1.0))
            .unwrap_or(1.0)
    } else {
        1.0
    };
    (radius * pressure).max(0.5).min(radius_limit)
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

#[cfg(test)]
mod pressure_tests {
    use super::apply_stroke_constraints;
    use crate::core::roto_brush_engine::{RotoStroke, RotoStrokeType};

    #[test]
    fn pressure_scales_hard_stroke_radius_and_legacy_strokes_keep_full_radius() {
        let mut matte = vec![0; 24 * 24];
        let stroke = RotoStroke {
            stroke_type: RotoStrokeType::Foreground,
            points: vec![[12.0, 12.0]],
            pressures: vec![0.25],
            radius: 8.0,
            frame: None,
        };
        apply_stroke_constraints(&mut matte, 24, 24, &[stroke], &mut || false)
            .expect("pressure-aware constraint should complete");
        assert_eq!(matte[12 * 24 + 14], 255);
        assert_eq!(matte[12 * 24 + 16], 0);

        let legacy = RotoStroke {
            stroke_type: RotoStrokeType::Foreground,
            points: vec![[12.0, 12.0]],
            pressures: Vec::new(),
            radius: 4.0,
            frame: None,
        };
        apply_stroke_constraints(&mut matte, 24, 24, &[legacy], &mut || false)
            .expect("legacy stroke should complete");
        assert_eq!(matte[12 * 24 + 16], 255);
    }
}
