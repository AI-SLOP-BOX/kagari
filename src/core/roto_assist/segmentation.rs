use crate::core::keyframe::{InterpolationType, Keyframe};
use crate::core::mask::Mask;
use crate::core::property::Animatable;

fn try_filled_vec<T: Clone>(len: usize, value: T) -> Option<Vec<T>> {
    let mut values = Vec::new();
    values.try_reserve_exact(len).ok()?;
    values.resize(len, value);
    Some(values)
}

pub fn apply_roto_refinement(
    mask: &mut Mask,
    frame: u32,
    smoothness: f32,
    feather_px: f32,
    edge_shift_px: f32,
) -> bool {
    let smoothness = if smoothness.is_finite() {
        smoothness.clamp(0.0, 10.0)
    } else {
        0.0
    };
    if smoothness > 0.0 && mask.path.is_closed {
        let changed = match &mut mask.path.vertices {
            Animatable::Constant(vertices) => {
                if vertices.len() >= 3 {
                    *vertices = smooth_closed(vertices, smoothness / 10.0);
                    true
                } else {
                    false
                }
            }
            Animatable::Animated(keyframes) => {
                for keyframe in keyframes.iter_mut() {
                    if keyframe.value.len() >= 3 {
                        keyframe.value = smooth_closed(&keyframe.value, smoothness / 10.0);
                    }
                }
                keyframes.iter().any(|keyframe| keyframe.value.len() >= 3)
            }
        };
        if changed {
            mask.path.tangents = None;
        }
    }

    let feather = if feather_px.is_finite() {
        feather_px.clamp(0.0, 500.0)
    } else {
        0.0
    };
    let edge_shift = if edge_shift_px.is_finite() {
        edge_shift_px.clamp(-500.0, 500.0)
    } else {
        0.0
    };
    mask.feather.set_value_at_frame(frame, feather);
    mask.expansion.set_value_at_frame(frame, edge_shift);
    true
}

pub fn set_roto_matte_keyframe(mask: &mut Mask, frame: u32, polygon: Vec<[f32; 2]>) -> bool {
    if polygon.len() < 3
        || polygon
            .iter()
            .flatten()
            .any(|coordinate| !coordinate.is_finite())
    {
        return false;
    }
    let new_keyframe = Keyframe::new(frame, polygon, InterpolationType::Hold);
    if let Animatable::Constant(existing) = &mask.path.vertices {
        let initial = existing.clone();
        mask.path.vertices = if frame == 0 {
            Animatable::Animated(vec![new_keyframe])
        } else {
            Animatable::Animated(vec![
                Keyframe::new(0, initial, InterpolationType::Hold),
                new_keyframe,
            ])
        };
    } else if let Animatable::Animated(keyframes) = &mut mask.path.vertices {
        if let Some(existing) = keyframes
            .iter_mut()
            .find(|keyframe| keyframe.frame == frame)
        {
            *existing = new_keyframe;
        } else {
            keyframes.push(new_keyframe);
        }
        keyframes.sort_by_key(|keyframe| keyframe.frame);
    }
    mask.path.tangents = None;
    true
}

pub(crate) fn smooth_closed(vertices: &[[f32; 2]], amount: f32) -> Vec<[f32; 2]> {
    if vertices.len() < 3 {
        return vertices.to_vec();
    }
    let amount = amount.clamp(0.0, 1.0);
    (0..vertices.len())
        .map(|index| {
            let previous = vertices[(index + vertices.len() - 1) % vertices.len()];
            let current = vertices[index];
            let next = vertices[(index + 1) % vertices.len()];
            if current.iter().all(|value| value.is_finite())
                && previous.iter().all(|value| value.is_finite())
                && next.iter().all(|value| value.is_finite())
            {
                let target = [
                    (previous[0] + current[0] * 2.0 + next[0]) * 0.25,
                    (previous[1] + current[1] * 2.0 + next[1]) * 0.25,
                ];
                [
                    current[0] + (target[0] - current[0]) * amount,
                    current[1] + (target[1] - current[1]) * amount,
                ]
            } else {
                current
            }
        })
        .collect()
}

// ──────────────── Roto Brush & Refine Edge Engine ────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RotoStroke {
    pub is_foreground: bool,
    pub points: Vec<[f32; 2]>,
    pub radius: f32,
}

/// Estimates binary foreground mask from user paint strokes (Green = Foreground, Red = Background).
pub fn segment_roto_brush(
    image: &[u8],
    width: u32,
    height: u32,
    strokes: &[RotoStroke],
) -> Vec<u8> {
    let Some(len) = (width as usize).checked_mul(height as usize) else {
        return Vec::new();
    };
    if width == 0
        || height == 0
        || width > i32::MAX as u32
        || height > i32::MAX as u32
        || len > crate::core::roto_brush_engine::MAX_ROTO_FRAME_PIXELS
    {
        return Vec::new();
    }
    let Some(mut mask) = try_filled_vec(len, 0u8) else {
        return Vec::new();
    };
    let Some(image_len) = len.checked_mul(4) else {
        return mask;
    };
    if width == 0 || height == 0 || strokes.is_empty() || image.len() != image_len {
        return mask;
    }

    const MAX_COLOR_SAMPLES_PER_CLASS: usize = 256;
    const MAX_COLOR_PROTOTYPES: usize = 8;
    let mut point_counts = [0usize; 2];
    for stroke in strokes {
        let radius = if stroke.radius.is_finite() {
            stroke.radius.clamp(1.0, 512.0)
        } else {
            1.0
        };
        let class = usize::from(!stroke.is_foreground);
        point_counts[class] = point_counts[class].saturating_add(
            stroke
                .points
                .iter()
                .filter(|point| {
                    point.iter().all(|coordinate| coordinate.is_finite())
                        && point[0] >= -radius
                        && point[1] >= -radius
                        && point[0] <= width as f32 + radius
                        && point[1] <= height as f32 + radius
                })
                .count(),
        );
    }
    let strides = point_counts.map(|count| count.div_ceil(MAX_COLOR_SAMPLES_PER_CLASS).max(1));
    let mut fg_colors: Vec<[f32; 3]> = Vec::new();
    let mut bg_colors: Vec<[f32; 3]> = Vec::new();
    if fg_colors
        .try_reserve_exact(point_counts[0].min(MAX_COLOR_SAMPLES_PER_CLASS))
        .is_err()
        || bg_colors
            .try_reserve_exact(point_counts[1].min(MAX_COLOR_SAMPLES_PER_CLASS))
            .is_err()
    {
        return mask;
    }
    let mut sampled_points = [0usize; 2];

    let w_i = width as i32;
    let h_i = height as i32;

    for stroke in strokes {
        let radius = if stroke.radius.is_finite() {
            stroke.radius.clamp(1.0, 512.0)
        } else {
            1.0
        };
        let class = usize::from(!stroke.is_foreground);
        for &pt in &stroke.points {
            if !pt.iter().all(|coordinate| coordinate.is_finite())
                || pt[0] < -radius
                || pt[1] < -radius
                || pt[0] > width as f32 + radius
                || pt[1] > height as f32 + radius
            {
                continue;
            }
            let point_index = sampled_points[class];
            sampled_points[class] = point_index.saturating_add(1);
            if !point_index.is_multiple_of(strides[class])
                || point_index / strides[class] >= MAX_COLOR_SAMPLES_PER_CLASS
            {
                continue;
            }
            let x = (pt[0].round() as i32).clamp(0, w_i - 1);
            let y = (pt[1].round() as i32).clamp(0, h_i - 1);
            let idx = ((y * w_i + x) * 4) as usize;
            let color = [
                image[idx] as f32,
                image[idx + 1] as f32,
                image[idx + 2] as f32,
            ];
            if stroke.is_foreground {
                fg_colors.push(color);
            } else {
                bg_colors.push(color);
            }
        }
    }

    if fg_colors.is_empty() {
        return mask;
    }

    let fg_colors =
        crate::core::roto_brush_engine::filter::color_prototypes(&fg_colors, MAX_COLOR_PROTOTYPES);
    let bg_colors =
        crate::core::roto_brush_engine::filter::color_prototypes(&bg_colors, MAX_COLOR_PROTOTYPES);

    // Classify each pixel based on minimum Euclidean color distance.
    for y in 0..height {
        for x in 0..width {
            let px_idx = ((y * width + x) * 4) as usize;
            let p_col = [
                image[px_idx] as f32,
                image[px_idx + 1] as f32,
                image[px_idx + 2] as f32,
            ];

            let min_fg_dist = fg_colors
                .iter()
                .map(|&c| {
                    (c[0] - p_col[0]).powi(2)
                        + (c[1] - p_col[1]).powi(2)
                        + (c[2] - p_col[2]).powi(2)
                })
                .fold(f32::INFINITY, f32::min);

            let min_bg_dist = if bg_colors.is_empty() {
                2500.0 // Default threshold
            } else {
                bg_colors
                    .iter()
                    .map(|&c| {
                        (c[0] - p_col[0]).powi(2)
                            + (c[1] - p_col[1]).powi(2)
                            + (c[2] - p_col[2]).powi(2)
                    })
                    .fold(f32::INFINITY, f32::min)
            };

            let out_idx = (y * width + x) as usize;
            if min_fg_dist < min_bg_dist {
                mask[out_idx] = 255;
            } else {
                mask[out_idx] = 0;
            }
        }
    }

    mask
}

/// Trace the boundary of a matte (binary or soft alpha) at a 50% coverage threshold.
/// Uses Moore-neighborhood contour tracing with Douglas-Peucker simplification.
pub fn trace_contour_to_polygon(
    mask: &[u8],
    width: u32,
    height: u32,
    simplify_tolerance: f32,
) -> Vec<[f32; 2]> {
    let Some(pixel_count) = (width as usize).checked_mul(height as usize) else {
        return Vec::new();
    };
    if width == 0
        || height == 0
        || width > i32::MAX as u32
        || height > i32::MAX as u32
        || pixel_count > crate::core::roto_brush_engine::MAX_ROTO_FRAME_PIXELS
        || mask.len() != pixel_count
    {
        return Vec::new();
    }
    let w = width as i32;
    let h = height as i32;
    if mask.is_empty() {
        return Vec::new();
    }
    let Some(state_count) = (w as usize)
        .checked_mul(h as usize)
        .and_then(|pixels| pixels.checked_mul(8))
    else {
        return Vec::new();
    };
    let at = |x: i32, y: i32| -> u8 {
        if x >= 0 && x < w && y >= 0 && y < h {
            mask[(y * w + x) as usize]
        } else {
            0
        }
    };
    // Find a starting border pixel (leftmost foreground pixel on the topmost row)
    let mut start: Option<(i32, i32)> = None;
    'outer: for y in 0..h {
        for x in 0..w {
            if at(x, y) >= 128 {
                start = Some((x, y));
                break 'outer;
            }
        }
    }
    let (sx, sy) = match start {
        Some(s) => s,
        None => return Vec::new(),
    };
    // Moore neighborhood directions: E, NE, N, NW, W, SW, S, SE
    let dirs: [(i32, i32); 8] = [
        (1, 0),
        (1, -1),
        (0, -1),
        (-1, -1),
        (-1, 0),
        (-1, 1),
        (0, 1),
        (1, 1),
    ];
    let mut contour: Vec<[f32; 2]> = Vec::new();
    let mut visited_states = Vec::new();
    if visited_states.try_reserve_exact(state_count).is_err() {
        return Vec::new();
    }
    visited_states.resize(state_count, false);
    let mut bx = sx;
    let mut by = sy;
    let mut dir = 6usize; // start searching from S (we came from the left)
    let max_iters = state_count;
    for _ in 0..max_iters {
        let state_index = ((by as usize * w as usize + bx as usize) * 8) + dir;
        if visited_states[state_index] {
            break;
        }
        visited_states[state_index] = true;
        if contour.len() == contour.capacity() && contour.try_reserve(1).is_err() {
            return Vec::new();
        }
        contour.push([bx as f32 + 0.5, by as f32 + 0.5]);
        // Search neighbors starting from (dir + 5) % 8 (turn left from entry direction)
        let mut found = false;
        for step in 0..8 {
            let nd = (dir + 5 + step) % 8;
            let nx = bx + dirs[nd].0;
            let ny = by + dirs[nd].1;
            if at(nx, ny) >= 128 {
                bx = nx;
                by = ny;
                dir = nd;
                found = true;
                break;
            }
        }
        if !found {
            break;
        }
        if bx == sx && by == sy {
            break;
        }
    }
    if contour.len() < 3 {
        return contour;
    }
    // Douglas-Peucker simplification using an explicit range stack so noisy
    // contours cannot recurse deeply or allocate intermediate polygon copies.
    fn dp_simplify(pts: &[[f32; 2]], eps: f32) -> Option<Vec<[f32; 2]>> {
        let mut keep = try_filled_vec(pts.len(), 0u8)?;
        if pts.len() <= 2 {
            let mut output = Vec::new();
            output.try_reserve_exact(pts.len()).ok()?;
            output.extend_from_slice(pts);
            return Some(output);
        }

        let mut ranges = Vec::new();
        ranges.try_reserve_exact(1).ok()?;
        ranges.push((0usize, pts.len() - 1));
        keep[0] = 1;
        keep[pts.len() - 1] = 1;
        while let Some((start, end)) = ranges.pop() {
            let (p0, p1) = (pts[start], pts[end]);
            let dx = p1[0] - p0[0];
            let dy = p1[1] - p0[1];
            let len_sq = dx * dx + dy * dy;
            let mut max_d = 0.0f32;
            let mut max_i = start;
            for (index, pt) in pts.iter().enumerate().take(end).skip(start + 1) {
                let d = if len_sq < 1e-12 {
                    ((pt[0] - p0[0]).powi(2) + (pt[1] - p0[1]).powi(2)).sqrt()
                } else {
                    let t = ((pt[0] - p0[0]) * dx + (pt[1] - p0[1]) * dy) / len_sq;
                    let t = t.clamp(0.0, 1.0);
                    let proj_x = p0[0] + t * dx;
                    let proj_y = p0[1] + t * dy;
                    ((pt[0] - proj_x).powi(2) + (pt[1] - proj_y).powi(2)).sqrt()
                };
                if d > max_d {
                    max_d = d;
                    max_i = index;
                }
            }
            if max_d > eps {
                keep[max_i] = 1;
                ranges.try_reserve(2).ok()?;
                ranges.push((start, max_i));
                ranges.push((max_i, end));
            }
        }

        let kept_count = keep.iter().filter(|&&value| value != 0).count();
        let mut output = Vec::new();
        output.try_reserve_exact(kept_count).ok()?;
        output.extend(
            pts.iter()
                .zip(keep)
                .filter_map(|(&point, keep)| (keep != 0).then_some(point)),
        );
        Some(output)
    }
    let simplify_tolerance = simplify_tolerance.max(0.0);
    let Some(polygon) = dp_simplify(&contour, simplify_tolerance) else {
        return Vec::new();
    };
    const MAX_CONTOUR_VERTICES: usize = 512;
    if polygon.len() > MAX_CONTOUR_VERTICES {
        super::flow_bake::resample_closed_polygon(&polygon, MAX_CONTOUR_VERTICES)
    } else {
        polygon
    }
}

/// Thresholds a soft alpha matte, keeps the component under the latest
/// foreground stroke, and traces that subject contour.
pub fn trace_roto_matte_contour(
    matte: &[u8],
    width: u32,
    height: u32,
    strokes: &[crate::core::roto_brush_engine::RotoStroke],
    simplify_tolerance: f32,
) -> Vec<[f32; 2]> {
    let Some(pixel_count) = (width as usize).checked_mul(height as usize) else {
        return Vec::new();
    };
    if width == 0
        || height == 0
        || width > i32::MAX as u32
        || height > i32::MAX as u32
        || pixel_count > crate::core::roto_brush_engine::MAX_ROTO_FRAME_PIXELS
        || matte.len() != pixel_count
    {
        return Vec::new();
    }
    let Some(seed) = strokes
        .iter()
        .rev()
        .find(|stroke| {
            stroke.stroke_type == crate::core::roto_brush_engine::RotoStrokeType::Foreground
        })
        .and_then(|stroke| stroke.points.first())
        .copied()
    else {
        return Vec::new();
    };
    let Some(mut binary) = try_filled_vec(pixel_count, 0u8) else {
        return Vec::new();
    };
    for (binary, &alpha) in binary.iter_mut().zip(matte) {
        *binary = if alpha >= 128 { 255 } else { 0 };
    }
    if retain_component_at_seed(&mut binary, width, height, seed).is_none() {
        return Vec::new();
    }
    trace_contour_to_polygon(&binary, width, height, simplify_tolerance)
}

pub(crate) fn retain_component_at_seed(
    binary: &mut [u8],
    width: u32,
    height: u32,
    seed: [f32; 2],
) -> Option<()> {
    let pixel_count = (width as usize).checked_mul(height as usize)?;
    if width == 0
        || height == 0
        || width > i32::MAX as u32
        || height > i32::MAX as u32
        || pixel_count > crate::core::roto_brush_engine::MAX_ROTO_FRAME_PIXELS
        || binary.len() != pixel_count
        || !seed.iter().all(|v| v.is_finite())
    {
        return None;
    }
    let sx = seed[0].round().clamp(0.0, width.saturating_sub(1) as f32) as u32;
    let sy = seed[1].round().clamp(0.0, height.saturating_sub(1) as f32) as u32;
    let start = (sy * width + sx) as usize;
    if binary[start] != 255 {
        return None;
    }

    let mut queue = std::collections::VecDeque::new();
    queue.try_reserve(1).ok()?;
    queue.push_back(start);
    binary[start] = 127;
    while let Some(index) = queue.pop_front() {
        let x = (index % width as usize) as i32;
        let y = (index / width as usize) as i32;
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let nx = x + dx;
                let ny = y + dy;
                if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                    continue;
                }
                let next = ny as usize * width as usize + nx as usize;
                if binary[next] == 255 {
                    queue.try_reserve(1).ok()?;
                    binary[next] = 127;
                    queue.push_back(next);
                }
            }
        }
    }
    for value in binary {
        *value = if *value == 127 { 255 } else { 0 };
    }
    Some(())
}

/// Refines hair and soft translucent edges using a Guided Filter against the RGB guide image.
pub fn refine_edge_guided_filter(
    guide_image: &[u8],
    rough_mask: &[u8],
    width: u32,
    height: u32,
    radius: i32,
    eps: f32,
) -> Vec<u8> {
    let Some(len) = (width as usize).checked_mul(height as usize) else {
        return Vec::new();
    };
    if width == 0
        || height == 0
        || width > i32::MAX as u32
        || height > i32::MAX as u32
        || len > crate::core::roto_brush_engine::MAX_ROTO_FRAME_PIXELS
    {
        return Vec::new();
    }
    let Some(mut refined) = try_filled_vec(len, 0u8) else {
        return Vec::new();
    };
    let Some(image_len) = len.checked_mul(4) else {
        return refined;
    };
    if width == 0 || height == 0 || guide_image.len() != image_len || rough_mask.len() != len {
        return refined;
    }

    let r = radius.clamp(1, 64);
    let w_i = width as i32;
    let h_i = height as i32;

    // Convert guide to grayscale I and mask to p (0..1)
    let Some(mut i_gray) = try_filled_vec(len, 0.0f32) else {
        return Vec::new();
    };
    let Some(mut p_val) = try_filled_vec(len, 0.0f32) else {
        return Vec::new();
    };

    for i in 0..len {
        let px = i * 4;
        i_gray[i] = (guide_image[px] as f32 * 0.299
            + guide_image[px + 1] as f32 * 0.587
            + guide_image[px + 2] as f32 * 0.114)
            / 255.0;
        p_val[i] = rough_mask[i] as f32 / 255.0;
    }

    // Guided filter local linear model: q = a * I + b
    let Some(mut a_vals) = try_filled_vec(len, 0.0f32) else {
        return Vec::new();
    };
    let Some(mut b_vals) = try_filled_vec(len, 0.0f32) else {
        return Vec::new();
    };

    for y in 0..h_i {
        for x in 0..w_i {
            let mut mean_i = 0.0f32;
            let mut mean_p = 0.0f32;
            let mut mean_ii = 0.0f32;
            let mut mean_ip = 0.0f32;
            let mut count = 0.0f32;

            for dy in -r..=r {
                for dx in -r..=r {
                    let nx = x + dx;
                    let ny = y + dy;
                    if nx >= 0 && nx < w_i && ny >= 0 && ny < h_i {
                        let idx = (ny * w_i + nx) as usize;
                        let i_v = i_gray[idx];
                        let p_v = p_val[idx];
                        mean_i += i_v;
                        mean_p += p_v;
                        mean_ii += i_v * i_v;
                        mean_ip += i_v * p_v;
                        count += 1.0;
                    }
                }
            }

            mean_i /= count;
            mean_p /= count;
            mean_ii /= count;
            mean_ip /= count;

            let var_i = mean_ii - mean_i * mean_i;
            let cov_ip = mean_ip - mean_i * mean_p;

            let a = cov_ip / (var_i + eps);
            let b = mean_p - a * mean_i;

            let idx = (y * w_i + x) as usize;
            a_vals[idx] = a;
            b_vals[idx] = b;
        }
    }

    // Output q = mean(a) * I + mean(b)
    for y in 0..h_i {
        for x in 0..w_i {
            let idx = (y * w_i + x) as usize;
            let q = (a_vals[idx] * i_gray[idx] + b_vals[idx]).clamp(0.0, 1.0);
            refined[idx] = (q * 255.0).round() as u8;
        }
    }

    refined
}
