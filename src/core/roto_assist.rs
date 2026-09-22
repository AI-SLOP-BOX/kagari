//! RotoAssist: tracker-driven mask animation — the practical, non-ML stand-in
//! for Rotobrush-style assistance. Bakes a tracked point's motion onto every
//! vertex of a base polygon so the matte follows the footage automatically.

use crate::core::keyframe::{InterpolationType, Keyframe};
use crate::core::mask::Mask;
use crate::core::property::Animatable;
use crate::core::timeline::TrackerPoint;

/// Bake per-vertex keyframes: each polygon vertex follows the tracker's
/// sampled offset curve (position(t) − position(frame0)), preserving shape.
/// Returns a NEW animated mask path; caller assigns to `mask.path`.
pub fn bake_tracked_mask(
    base_mask: &Mask,
    tracker: &TrackerPoint,
    start_frame: u32,
    end_frame: u32,
) -> Result<Animatable<Vec<[f32; 2]>>, String> {
    if end_frame < start_frame {
        return Err("end frame must not precede start frame".into());
    }
    let base_poly = base_mask.path.to_polygon(start_frame.max(1), 16);
    if base_poly.is_empty() {
        return Err("base mask has no vertices".into());
    }
    let origin = tracker.position.evaluate(start_frame);
    let mut kfs: Vec<Keyframe<Vec<[f32; 2]>>> = Vec::new();
    let step = (end_frame.saturating_sub(start_frame) / 60).max(1); // ≤ ~61 samples
    let mut f = start_frame;
    loop {
        let cur = tracker.position.evaluate(f);
        let dx = cur[0] - origin[0];
        let dy = cur[1] - origin[1];
        let moved: Vec<[f32; 2]> = base_poly.iter().map(|p| [p[0] + dx, p[1] + dy]).collect();
        kfs.push(Keyframe::new(f, moved, InterpolationType::Linear));
        if f >= end_frame {
            break;
        }
        f = f.saturating_add(step).min(end_frame);
    }
    if kfs.is_empty() {
        return Err("no frames to bake".into());
    }
    Ok(Animatable::Animated(kfs))
}

pub fn bake_tracked_mask_from_trackers(
    base_mask: &Mask,
    trackers: &[TrackerPoint],
    tracker_index: usize,
    start_frame: u32,
    end_frame: u32,
) -> Result<Animatable<Vec<[f32; 2]>>, String> {
    let tracker = trackers
        .get(tracker_index)
        .ok_or_else(|| format!("tracker point {} does not exist", tracker_index + 1))?;
    bake_tracked_mask(base_mask, tracker, start_frame, end_frame)
}

/// Propagates a roto boundary through every frame using local dense motion
/// between adjacent source images. The frame provider must return the
/// selected layer rendered into composition-sized RGBA pixels.
pub fn bake_optical_flow_mask<F>(
    base_mask: &Mask,
    anchor_frame: u32,
    start_frame: u32,
    end_frame: u32,
    width: u32,
    height: u32,
    mut frame_provider: F,
) -> Result<Animatable<Vec<[f32; 2]>>, String>
where
    F: FnMut(u32) -> Option<Vec<u8>>,
{
    if end_frame < start_frame || anchor_frame < start_frame || anchor_frame > end_frame {
        return Err("roto propagation range must contain the anchor frame".into());
    }
    let expected_len = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .filter(|_| width > 0 && height > 0)
        .ok_or_else(|| "invalid composition dimensions for roto propagation".to_owned())?;
    let mut boundary = base_mask.path.to_polygon(anchor_frame, 16);
    if boundary.len() < 3 {
        return Err("Roto Brush Matte has no closed boundary to propagate".into());
    }

    let anchor_pixels = frame_provider(anchor_frame)
        .ok_or_else(|| format!("Could not render source frame {anchor_frame}"))?;
    if anchor_pixels.len() != expected_len {
        return Err(format!("Source frame {anchor_frame} has invalid dimensions"));
    }

    let flow_scale = (320.0 / width.max(height) as f32).min(1.0);
    let flow_width = ((width as f32 * flow_scale).round() as u32).max(1);
    let flow_height = ((height as f32 * flow_scale).round() as u32).max(1);
    let anchor_small = downsample_rgba(&anchor_pixels, width, height, flow_width, flow_height);
    let mut keyframes = vec![Keyframe::new(
        anchor_frame,
        boundary.clone(),
        InterpolationType::Linear,
    )];

    for direction in [-1i64, 1i64] {
        let mut previous_frame = anchor_frame;
        let mut previous_small = anchor_small.clone();
        boundary = base_mask.path.to_polygon(anchor_frame, 16);
        loop {
            let next_frame = previous_frame as i64 + direction;
            if next_frame < start_frame as i64 || next_frame > end_frame as i64 {
                break;
            }
            let next_frame = next_frame as u32;
            let pixels = frame_provider(next_frame)
                .ok_or_else(|| format!("Could not render source frame {next_frame}"))?;
            if pixels.len() != expected_len {
                return Err(format!("Source frame {next_frame} has invalid dimensions"));
            }
            let current_small = downsample_rgba(&pixels, width, height, flow_width, flow_height);
            let flow = crate::core::optical_flow_timewarp::compute_dense_optical_flow(
                &previous_small,
                &current_small,
                flow_width,
                flow_height,
                1,
                4,
            );
            let reverse_flow = crate::core::optical_flow_timewarp::compute_dense_optical_flow(
                &current_small,
                &previous_small,
                flow_width,
                flow_height,
                1,
                4,
            );
            boundary = warp_roto_boundary_with_flow(
                &boundary,
                &flow,
                &reverse_flow,
                width,
                height,
            );
            keyframes.push(Keyframe::new(
                next_frame,
                boundary.clone(),
                InterpolationType::Linear,
            ));
            previous_frame = next_frame;
            previous_small = current_small;
        }
    }

    keyframes.sort_by_key(|keyframe| keyframe.frame);
    Ok(Animatable::Animated(keyframes))
}

fn downsample_rgba(source: &[u8], width: u32, height: u32, out_w: u32, out_h: u32) -> Vec<u8> {
    let mut output = vec![0u8; out_w as usize * out_h as usize * 4];
    for y in 0..out_h {
        let sy = (((y as f32 + 0.5) * height as f32 / out_h as f32).floor() as u32)
            .min(height - 1);
        for x in 0..out_w {
            let sx = (((x as f32 + 0.5) * width as f32 / out_w as f32).floor() as u32)
                .min(width - 1);
            let src = ((sy * width + sx) * 4) as usize;
            let dst = ((y * out_w + x) * 4) as usize;
            output[dst..dst + 4].copy_from_slice(&source[src..src + 4]);
        }
    }
    output
}

fn warp_roto_boundary_with_flow(
    boundary: &[[f32; 2]],
    flow: &crate::core::optical_flow_timewarp::DenseFlowField,
    reverse_flow: &crate::core::optical_flow_timewarp::DenseFlowField,
    width: u32,
    height: u32,
) -> Vec<[f32; 2]> {
    if width == 0
        || height == 0
        || flow.width == 0
        || flow.height == 0
        || reverse_flow.width != flow.width
        || reverse_flow.height != flow.height
    {
        return boundary.to_vec();
    }
    boundary
        .iter()
        .map(|point| {
            let forward = sample_dense_flow(flow, point[0], point[1], width, height);
            let scale_x = width as f32 / flow.width as f32;
            let scale_y = height as f32 / flow.height as f32;
            let mapped = [
                point[0] + forward[0] * scale_x,
                point[1] + forward[1] * scale_y,
            ];
            let backward = sample_dense_flow(reverse_flow, mapped[0], mapped[1], width, height);
            let round_trip_error = (forward[0] + backward[0])
                .hypot(forward[1] + backward[1]);
            let tolerance = 1.5 + forward[0].hypot(forward[1]) * 0.5;
            if !round_trip_error.is_finite() || round_trip_error > tolerance {
                *point
            } else {
                mapped
            }
        })
        .collect()
}

fn sample_dense_flow(
    flow: &crate::core::optical_flow_timewarp::DenseFlowField,
    x: f32,
    y: f32,
    width: u32,
    height: u32,
) -> [f32; 2] {
    let fx = (x * flow.width as f32 / width as f32)
        .clamp(0.0, flow.width.saturating_sub(1) as f32);
    let fy = (y * flow.height as f32 / height as f32)
        .clamp(0.0, flow.height.saturating_sub(1) as f32);
    let x0 = fx.floor() as u32;
    let y0 = fy.floor() as u32;
    let x1 = (x0 + 1).min(flow.width - 1);
    let y1 = (y0 + 1).min(flow.height - 1);
    let tx = fx - x0 as f32;
    let ty = fy - y0 as f32;
    let a = flow.get(x0, y0);
    let b = flow.get(x1, y0);
    let c = flow.get(x0, y1);
    let d = flow.get(x1, y1);
    [
        (a[0] * (1.0 - tx) + b[0] * tx) * (1.0 - ty)
            + (c[0] * (1.0 - tx) + d[0] * tx) * ty,
        (a[1] * (1.0 - tx) + b[1] * tx) * (1.0 - ty)
            + (c[1] * (1.0 - tx) + d[1] * tx) * ty,
    ]
}

/// Applies roto cleanup settings to the stored mask geometry. Smoothing keeps
/// the keyframe vertex count stable; feather and edge shift use native mask
/// properties so preview and export share the same result.
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

fn smooth_closed(vertices: &[[f32; 2]], amount: f32) -> Vec<[f32; 2]> {
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
    let mut mask = vec![0u8; len];
    let Some(image_len) = len.checked_mul(4) else {
        return mask;
    };
    if width == 0 || height == 0 || strokes.is_empty() || image.len() != image_len {
        return mask;
    }

    // Collect sampled foreground and background colors
    let mut fg_colors: Vec<[f32; 3]> = Vec::new();
    let mut bg_colors: Vec<[f32; 3]> = Vec::new();

    let w_i = width as i32;
    let h_i = height as i32;

    for stroke in strokes {
        let r = if stroke.radius.is_finite() {
            stroke.radius.clamp(1.0, 512.0) as i32
        } else {
            1
        };
        for &pt in &stroke.points {
            let cx = pt[0].round() as i32;
            let cy = pt[1].round() as i32;

            for dy in -r..=r {
                for dx in -r..=r {
                    let x = cx + dx;
                    let y = cy + dy;
                    if x >= 0 && x < w_i && y >= 0 && y < h_i && (dx * dx + dy * dy) <= r * r {
                        let idx = ((y * w_i + x) * 4) as usize;
                        let col = [
                            image[idx] as f32,
                            image[idx + 1] as f32,
                            image[idx + 2] as f32,
                        ];
                        if stroke.is_foreground {
                            fg_colors.push(col);
                        } else {
                            bg_colors.push(col);
                        }
                    }
                }
            }
        }
    }

    if fg_colors.is_empty() {
        return mask;
    }

    // Classify each pixel based on minimum Euclidean color distance
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

/// Trace the boundary of a binary mask (0/255) and return simplified polygon vertices.
/// Uses Moore-neighborhood contour tracing with Douglas-Peucker simplification.
pub fn trace_contour_to_polygon(
    mask: &[u8],
    width: u32,
    height: u32,
    simplify_tolerance: f32,
) -> Vec<[f32; 2]> {
    let w = width as i32;
    let h = height as i32;
    if mask.is_empty() || w == 0 || h == 0 {
        return Vec::new();
    }
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
            if at(x, y) == 255 {
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
    let mut bx = sx;
    let mut by = sy;
    let mut dir = 6usize; // start searching from S (we came from the left)
    let max_iters = (w * h * 4) as usize;
    for _ in 0..max_iters {
        contour.push([bx as f32 + 0.5, by as f32 + 0.5]);
        // Search neighbors starting from (dir + 5) % 8 (turn left from entry direction)
        let mut found = false;
        for step in 0..8 {
            let nd = (dir + 5 + step) % 8;
            let nx = bx + dirs[nd].0;
            let ny = by + dirs[nd].1;
            if at(nx, ny) == 255 {
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
    // Douglas-Peucker simplification
    fn dp_simplify(pts: &[[f32; 2]], eps: f32) -> Vec<[f32; 2]> {
        if pts.len() <= 2 {
            return pts.to_vec();
        }
        let mut max_d = 0.0f32;
        let mut max_i = 0usize;
        let (p0, p1) = (pts[0], pts[pts.len() - 1]);
        let dx = p1[0] - p0[0];
        let dy = p1[1] - p0[1];
        let len_sq = dx * dx + dy * dy;
        for (i, pt) in pts.iter().enumerate().skip(1).take(pts.len() - 2) {
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
                max_i = i;
            }
        }
        if max_d > eps {
            let left = dp_simplify(&pts[..=max_i], eps);
            let right = dp_simplify(&pts[max_i..], eps);
            let mut result = left;
            result.extend_from_slice(&right[1..]);
            result
        } else {
            vec![pts[0], pts[pts.len() - 1]]
        }
    }
    dp_simplify(&contour, simplify_tolerance)
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
    let mut refined = vec![0u8; len];
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
    let mut i_gray = vec![0.0f32; len];
    let mut p_val = vec![0.0f32; len];

    for i in 0..len {
        let px = i * 4;
        i_gray[i] = (guide_image[px] as f32 * 0.299
            + guide_image[px + 1] as f32 * 0.587
            + guide_image[px + 2] as f32 * 0.114)
            / 255.0;
        p_val[i] = rough_mask[i] as f32 / 255.0;
    }

    // Guided filter local linear model: q = a * I + b
    let mut a_vals = vec![0.0f32; len];
    let mut b_vals = vec![0.0f32; len];

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
                            .wrapping_mul(0x45d9_f3b)
                            .wrapping_add(y.wrapping_mul(0x119d_e1f3));
                        value ^= value >> 16;
                        value = value.wrapping_mul(0x45d9_f3b);
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

        let mask = Mask::new_rect("roto".into(), "Roto".into(), 8.0, 8.0, 16.0, 16.0);
        let left = textured_frame(64, 48, -2);
        let source = textured_frame(64, 48, 0);
        let right = textured_frame(64, 48, 2);
        let baked = bake_optical_flow_mask(&mask, 1, 0, 2, 64, 48, |frame| {
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
        assert!((left_point[0] - anchor_point[0] + 2.0).abs() <= 1.0);
        assert!((right_point[0] - anchor_point[0] - 2.0).abs() <= 1.0);
        assert!((left_point[1] - anchor_point[1]).abs() <= 1.0);
        assert!((right_point[1] - anchor_point[1]).abs() <= 1.0);
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
        assert_eq!(kfs.iter().map(|kf| kf.frame).collect::<Vec<_>>(), [u32::MAX - 1, u32::MAX]);
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
