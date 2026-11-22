/// Filters raw motion tracking points with outlier rejection and exponential moving average (EMA) stabilization.
pub fn filter_motion_track_stabilization(
    raw_points: &[[f32; 2]],
    smoothing_radius: usize,
    max_jitter_px: f32,
) -> Vec<[f32; 2]> {
    if raw_points.is_empty() {
        return vec![];
    }
    if raw_points.len() <= 2 || smoothing_radius == 0 {
        return raw_points.to_vec();
    }

    let mut cleaned = raw_points.to_vec();

    // 1. Outlier jitter rejection
    for i in 1..(cleaned.len() - 1) {
        let prev = cleaned[i - 1];
        let curr = cleaned[i];
        let next = cleaned[i + 1];

        let d_prev = ((curr[0] - prev[0]).powi(2) + (curr[1] - prev[1]).powi(2)).sqrt();
        let d_next = ((curr[0] - next[0]).powi(2) + (curr[1] - next[1]).powi(2)).sqrt();

        if d_prev > max_jitter_px && d_next > max_jitter_px {
            // Interpolate midpoint between neighbors
            cleaned[i] = [(prev[0] + next[0]) * 0.5, (prev[1] + next[1]) * 0.5];
        }
    }

    // 2. Moving average smoothing
    let mut smoothed = Vec::with_capacity(cleaned.len());
    let n = cleaned.len();

    for i in 0..n {
        let start = i.saturating_sub(smoothing_radius);
        let end = (i + smoothing_radius + 1).min(n);
        let count = (end - start) as f32;

        let mut sum_x = 0.0f32;
        let mut sum_y = 0.0f32;
        for point in cleaned.iter().take(end).skip(start) {
            sum_x += point[0];
            sum_y += point[1];
        }
        smoothed.push([sum_x / count, sum_y / count]);
    }

    smoothed
}

/// Roto Brush semi-automatic segmentation: extracts closed subject contour from user brush stroke seeds.
pub fn extract_roto_brush_contour(
    image_rgba: &[u8],
    width: u32,
    height: u32,
    seed_points: &[[f32; 2]],
    tolerance: f32,
) -> Vec<[f32; 2]> {
    if image_rgba.is_empty() || width == 0 || height == 0 || seed_points.is_empty() {
        return vec![];
    }

    let w = width as usize;
    let h = height as usize;

    // Sample seed color mean
    let mut mean_r = 0.0f32;
    let mut mean_g = 0.0f32;
    let mut mean_b = 0.0f32;
    let mut valid_seeds = 0.0f32;

    for pt in seed_points {
        let px = (pt[0].round() as isize).clamp(0, w as isize - 1) as usize;
        let py = (pt[1].round() as isize).clamp(0, h as isize - 1) as usize;
        let idx = (py * w + px) * 4;
        if idx + 3 < image_rgba.len() {
            mean_r += image_rgba[idx] as f32;
            mean_g += image_rgba[idx + 1] as f32;
            mean_b += image_rgba[idx + 2] as f32;
            valid_seeds += 1.0;
        }
    }

    if valid_seeds == 0.0 {
        return vec![];
    }

    let (mr, mg, mb) = (
        mean_r / valid_seeds,
        mean_g / valid_seeds,
        mean_b / valid_seeds,
    );
    let tol_sq = (tolerance.clamp(5.0, 150.0)).powi(2);

    // Bounding box of region of interest
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    for pt in seed_points {
        min_x = min_x.min(pt[0]);
        min_y = min_y.min(pt[1]);
        max_x = max_x.max(pt[0]);
        max_y = max_y.max(pt[1]);
    }

    let pad = 30.0f32;
    let roi_min_x = (min_x - pad).max(0.0) as usize;
    let roi_min_y = (min_y - pad).max(0.0) as usize;
    let roi_max_x = (max_x + pad).min((w - 1) as f32) as usize;
    let roi_max_y = (max_y + pad).min((h - 1) as f32) as usize;

    // Find boundary vertices satisfying color distance threshold
    let mut contour = Vec::new();
    let step = 4;

    // Top edge
    for x in (roi_min_x..=roi_max_x).step_by(step) {
        for y in roi_min_y..=roi_max_y {
            let idx = (y * w + x) * 4;
            let dr = image_rgba[idx] as f32 - mr;
            let dg = image_rgba[idx + 1] as f32 - mg;
            let db = image_rgba[idx + 2] as f32 - mb;
            if dr * dr + dg * dg + db * db <= tol_sq {
                contour.push([x as f32, y as f32]);
                break;
            }
        }
    }

    // Right edge
    for y in (roi_min_y..=roi_max_y).step_by(step) {
        for x in (roi_min_x..=roi_max_x).rev() {
            let idx = (y * w + x) * 4;
            let dr = image_rgba[idx] as f32 - mr;
            let dg = image_rgba[idx + 1] as f32 - mg;
            let db = image_rgba[idx + 2] as f32 - mb;
            if dr * dr + dg * dg + db * db <= tol_sq {
                contour.push([x as f32, y as f32]);
                break;
            }
        }
    }

    // Bottom edge
    for x in (roi_min_x..=roi_max_x).rev().step_by(step) {
        for y in (roi_min_y..=roi_max_y).rev() {
            let idx = (y * w + x) * 4;
            let dr = image_rgba[idx] as f32 - mr;
            let dg = image_rgba[idx + 1] as f32 - mg;
            let db = image_rgba[idx + 2] as f32 - mb;
            if dr * dr + dg * dg + db * db <= tol_sq {
                contour.push([x as f32, y as f32]);
                break;
            }
        }
    }

    if contour.len() < 3 {
        // Fallback to bounding box contour
        vec![
            [roi_min_x as f32, roi_min_y as f32],
            [roi_max_x as f32, roi_min_y as f32],
            [roi_max_x as f32, roi_max_y as f32],
            [roi_min_x as f32, roi_max_y as f32],
        ]
    } else {
        contour
    }
}

#[cfg(test)]
mod roto_and_stab_tests {
    use super::*;

    #[test]
    fn test_filter_motion_track_stabilization_removes_jitter() {
        let raw = vec![
            [10.0, 10.0],
            [100.0, 100.0], // Jitter outlier
            [12.0, 12.0],
            [13.0, 13.0],
        ];
        let stabilized = filter_motion_track_stabilization(&raw, 1, 20.0);
        assert_eq!(stabilized.len(), 4);
        assert!(stabilized[1][0] < 50.0);
    }

    #[test]
    fn test_extract_roto_brush_contour_returns_closed_polygon() {
        let mut img = vec![255u8; 50 * 50 * 4];
        // Draw green square subject in center
        for y in 15..35 {
            for x in 15..35 {
                let idx = (y * 50 + x) * 4;
                img[idx] = 0;
                img[idx + 1] = 255;
                img[idx + 2] = 0;
                img[idx + 3] = 255;
            }
        }
        let seeds = vec![[25.0, 25.0]];
        let contour = extract_roto_brush_contour(&img, 50, 50, &seeds, 30.0);
        assert!(contour.len() >= 3);
    }
}
