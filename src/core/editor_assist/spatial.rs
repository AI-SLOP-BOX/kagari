use super::types::{LoopCandidate, PixelBounds};

/// Chooses a subtitle/overlay rectangle with the least overlap with occupied regions.
pub fn find_safe_overlay_position(
    frame_size: [u32; 2],
    overlay_size: [u32; 2],
    occupied: &[PixelBounds],
    margin: u32,
) -> Option<PixelBounds> {
    if frame_size.contains(&0)
        || overlay_size.contains(&0)
        || overlay_size[0].saturating_add(margin.saturating_mul(2)) > frame_size[0]
        || overlay_size[1].saturating_add(margin.saturating_mul(2)) > frame_size[1]
    {
        return None;
    }
    let center_x = (frame_size[0] - overlay_size[0]) / 2;
    let center_y = (frame_size[1] - overlay_size[1]) / 2;
    let candidates = [
        [center_x, frame_size[1] - margin - overlay_size[1]],
        [center_x, margin],
        [margin, center_y],
        [frame_size[0] - margin - overlay_size[0], center_y],
        [center_x, center_y],
    ];
    candidates
        .into_iter()
        .map(|position| PixelBounds {
            x: position[0],
            y: position[1],
            width: overlay_size[0],
            height: overlay_size[1],
        })
        .min_by_key(|candidate| {
            occupied
                .iter()
                .map(|region| intersection_area(*candidate, *region))
                .sum::<u64>()
        })
}

fn intersection_area(a: PixelBounds, b: PixelBounds) -> u64 {
    let left = a.x.max(b.x);
    let top = a.y.max(b.y);
    let right = a.x.saturating_add(a.width).min(b.x.saturating_add(b.width));
    let bottom =
        a.y.saturating_add(a.height)
            .min(b.y.saturating_add(b.height));
    right.saturating_sub(left) as u64 * bottom.saturating_sub(top) as u64
}

/// Estimates image sharpness using variance of the luminance Laplacian.
pub fn sharpness_score(rgba: &[u8], width: u32, height: u32) -> Option<f32> {
    let expected = (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)?;
    if width < 3 || height < 3 || rgba.len() != expected {
        return None;
    }
    let luminance = |x: u32, y: u32| {
        let index = (y as usize * width as usize + x as usize) * 4;
        rgba[index] as f32 * 0.2126
            + rgba[index + 1] as f32 * 0.7152
            + rgba[index + 2] as f32 * 0.0722
    };
    let mut values = Vec::with_capacity(((width - 2) * (height - 2)) as usize);
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            values.push(
                luminance(x - 1, y)
                    + luminance(x + 1, y)
                    + luminance(x, y - 1)
                    + luminance(x, y + 1)
                    - 4.0 * luminance(x, y),
            );
        }
    }
    let mean = values.iter().sum::<f32>() / values.len() as f32;
    Some(
        values
            .iter()
            .map(|value| (value - mean).powi(2))
            .sum::<f32>()
            / values.len() as f32,
    )
}

/// Measures high-frequency camera shake from tracked frame-to-frame positions.
pub fn camera_shake_score(tracked_centers: &[[f32; 2]]) -> f32 {
    if tracked_centers.len() < 3
        || tracked_centers
            .iter()
            .flatten()
            .any(|value| !value.is_finite())
    {
        return 0.0;
    }
    let acceleration_energy = tracked_centers
        .windows(3)
        .map(|points| {
            let acceleration = [
                points[2][0] - 2.0 * points[1][0] + points[0][0],
                points[2][1] - 2.0 * points[1][1] + points[0][1],
            ];
            acceleration[0] * acceleration[0] + acceleration[1] * acceleration[1]
        })
        .sum::<f32>();
    (acceleration_energy / (tracked_centers.len() - 2) as f32).sqrt()
}

/// Builds an aspect-ratio crop centered around the combined important regions.
pub fn smart_aspect_crop(
    frame_size: [u32; 2],
    target_aspect: f32,
    important_regions: &[PixelBounds],
) -> Option<PixelBounds> {
    if frame_size.contains(&0) || !target_aspect.is_finite() || target_aspect <= 0.0 {
        return None;
    }
    let source_aspect = frame_size[0] as f32 / frame_size[1] as f32;
    let (crop_width, crop_height) = if target_aspect >= source_aspect {
        (
            frame_size[0],
            (frame_size[0] as f32 / target_aspect).round() as u32,
        )
    } else {
        (
            (frame_size[1] as f32 * target_aspect).round() as u32,
            frame_size[1],
        )
    };
    let crop_width = crop_width.clamp(1, frame_size[0]);
    let crop_height = crop_height.clamp(1, frame_size[1]);
    let frame_bounds = PixelBounds {
        x: 0,
        y: 0,
        width: frame_size[0],
        height: frame_size[1],
    };
    let valid: Vec<_> = important_regions
        .iter()
        .filter_map(|region| intersect_bounds(*region, frame_bounds))
        .collect();
    let center = if valid.is_empty() {
        [frame_size[0] as f64 * 0.5, frame_size[1] as f64 * 0.5]
    } else {
        let total_weight = valid
            .iter()
            .map(|region| region.width as f64 * region.height as f64)
            .sum::<f64>();
        let weighted = valid.iter().fold([0.0, 0.0], |mut sum, region| {
            let weight = region.width as f64 * region.height as f64;
            sum[0] += (region.x as f64 + region.width as f64 * 0.5) * weight;
            sum[1] += (region.y as f64 + region.height as f64 * 0.5) * weight;
            sum
        });
        [weighted[0] / total_weight, weighted[1] / total_weight]
    };
    let x = (center[0] - crop_width as f64 * 0.5)
        .round()
        .clamp(0.0, (frame_size[0] - crop_width) as f64) as u32;
    let y = (center[1] - crop_height as f64 * 0.5)
        .round()
        .clamp(0.0, (frame_size[1] - crop_height) as f64) as u32;
    Some(PixelBounds {
        x,
        y,
        width: crop_width,
        height: crop_height,
    })
}

fn intersect_bounds(a: PixelBounds, b: PixelBounds) -> Option<PixelBounds> {
    let x = a.x.max(b.x);
    let y = a.y.max(b.y);
    let right = a.x.saturating_add(a.width).min(b.x.saturating_add(b.width));
    let bottom =
        a.y.saturating_add(a.height)
            .min(b.y.saturating_add(b.height));
    if right <= x || bottom <= y {
        return None;
    }
    Some(PixelBounds {
        x,
        y,
        width: right - x,
        height: bottom - y,
    })
}

/// Suggests visually seamless frame pairs, ordered by lowest RGB error.
pub fn find_loop_candidates(
    frames: &[&[u8]],
    width: u32,
    height: u32,
    minimum_length: usize,
    maximum_results: usize,
) -> Vec<LoopCandidate> {
    let Some(frame_len) = (width as usize)
        .checked_mul(height as usize)
        .and_then(|value| value.checked_mul(4))
    else {
        return Vec::new();
    };
    if frame_len == 0
        || minimum_length == 0
        || maximum_results == 0
        || frames.iter().any(|frame| frame.len() != frame_len)
    {
        return Vec::new();
    }
    let pixels = width as f32 * height as f32;
    let mut candidates = Vec::new();
    for start in 0..frames.len() {
        for end in start.saturating_add(minimum_length)..frames.len() {
            let error = frames[start]
                .as_chunks::<4>()
                .0
                .iter()
                .zip(frames[end].as_chunks::<4>().0.iter())
                .map(|(a, b)| {
                    (a[0].abs_diff(b[0]) as f32
                        + a[1].abs_diff(b[1]) as f32
                        + a[2].abs_diff(b[2]) as f32)
                        / 3.0
                })
                .sum::<f32>()
                / pixels;
            let candidate = LoopCandidate {
                start_frame: start,
                end_frame: end,
                error,
            };
            if candidates.len() < maximum_results {
                candidates.push(candidate);
            } else {
                let worst = candidates
                    .iter()
                    .enumerate()
                    .max_by(|(_, left), (_, right)| compare_loop_candidates(left, right));
                if let Some((worst_index, worst)) = worst {
                    if compare_loop_candidates(&candidate, worst).is_lt() {
                        candidates[worst_index] = candidate;
                    }
                }
            }
        }
    }
    candidates.sort_by(compare_loop_candidates);
    candidates
}

fn compare_loop_candidates(a: &LoopCandidate, b: &LoopCandidate) -> std::cmp::Ordering {
    a.error
        .total_cmp(&b.error)
        .then_with(|| b.end_frame.cmp(&a.end_frame))
        .then_with(|| a.start_frame.cmp(&b.start_frame))
}
