use super::segmentation::*;
use crate::core::keyframe::{InterpolationType, Keyframe};
use crate::core::mask::Mask;
use crate::core::property::Animatable;
use crate::core::timeline::TrackerPoint;

const ROTO_FLOW_MAX_DIMENSION: f32 = 160.0;
const ROTO_FLOW_SEARCH_RADIUS: i32 = 8;

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
    frame_provider: F,
) -> Result<Animatable<Vec<[f32; 2]>>, String>
where
    F: FnMut(u32) -> Option<Vec<u8>>,
{
    bake_optical_flow_roto_matte(
        base_mask,
        &[],
        &crate::core::roto_brush_engine::RotoBrushSettings::default(),
        anchor_frame,
        start_frame,
        end_frame,
        width,
        height,
        frame_provider,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn bake_optical_flow_roto_matte<F>(
    base_mask: &Mask,
    source_strokes: &[crate::core::roto_brush_engine::RotoStroke],
    settings: &crate::core::roto_brush_engine::RotoBrushSettings,
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
        return Err(format!(
            "Source frame {anchor_frame} has invalid dimensions"
        ));
    }

    let flow_scale = (ROTO_FLOW_MAX_DIMENSION / width.max(height) as f32).min(1.0);
    let flow_width = ((width as f32 * flow_scale).round() as u32).max(1);
    let flow_height = ((height as f32 * flow_scale).round() as u32).max(1);
    let anchor_small = downsample_rgba(&anchor_pixels, width, height, flow_width, flow_height);
    let mut keyframes = vec![Keyframe::new(
        anchor_frame,
        boundary.clone(),
        InterpolationType::Linear,
    )];
    let anchor_strokes =
        crate::core::roto_brush_engine::strokes_for_frame(source_strokes, anchor_frame);

    for direction in [-1i64, 1i64] {
        let mut previous_frame = anchor_frame;
        let mut previous_small = anchor_small.clone();
        boundary = base_mask.path.to_polygon(anchor_frame, 16);
        let mut propagated_strokes = anchor_strokes.clone();
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
                ROTO_FLOW_SEARCH_RADIUS,
            );
            let reverse_flow = crate::core::optical_flow_timewarp::compute_dense_optical_flow(
                &current_small,
                &previous_small,
                flow_width,
                flow_height,
                1,
                ROTO_FLOW_SEARCH_RADIUS,
            );
            boundary = warp_roto_boundary_with_flow(&boundary, &flow, &reverse_flow, width, height);
            propagated_strokes =
                warp_roto_strokes(&propagated_strokes, &flow, &reverse_flow, width, height);
            propagated_strokes.extend(
                source_strokes
                    .iter()
                    .filter(|stroke| stroke.frame == Some(next_frame))
                    .cloned()
                    .map(|mut stroke| {
                        stroke.frame = None;
                        stroke
                    }),
            );
            if let Some(refined_boundary) = resegment_propagated_boundary(
                &pixels,
                width,
                height,
                &boundary,
                &propagated_strokes,
                settings,
            ) {
                boundary = refined_boundary;
            }
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

pub(crate) fn warp_roto_strokes(
    strokes: &[crate::core::roto_brush_engine::RotoStroke],
    flow: &crate::core::optical_flow_timewarp::DenseFlowField,
    reverse_flow: &crate::core::optical_flow_timewarp::DenseFlowField,
    width: u32,
    height: u32,
) -> Vec<crate::core::roto_brush_engine::RotoStroke> {
    strokes
        .iter()
        .cloned()
        .map(|mut stroke| {
            stroke.points =
                warp_roto_boundary_with_flow(&stroke.points, flow, reverse_flow, width, height);
            stroke.frame = None;
            stroke
        })
        .collect()
}

pub(crate) fn resegment_propagated_boundary(
    pixels: &[u8],
    width: u32,
    height: u32,
    previous_boundary: &[[f32; 2]],
    strokes: &[crate::core::roto_brush_engine::RotoStroke],
    settings: &crate::core::roto_brush_engine::RotoBrushSettings,
) -> Option<Vec<[f32; 2]>> {
    let mut geometry_settings = settings.clone();
    geometry_settings.feather_radius = 0.0;
    let foreground_seed = strokes
        .iter()
        .find(|stroke| {
            stroke.stroke_type == crate::core::roto_brush_engine::RotoStrokeType::Foreground
        })?
        .points
        .first()
        .copied()?;
    let alpha = crate::core::roto_brush_engine::generate_rotobrush_matte(
        pixels,
        width,
        height,
        strokes,
        &geometry_settings,
    );
    let mut binary = alpha
        .into_iter()
        .map(|value| if value >= 128 { 255 } else { 0 })
        .collect::<Vec<_>>();
    retain_component_at_seed(&mut binary, width, height, foreground_seed)?;
    let polygon = trace_contour_to_polygon(&binary, width, height, 1.5);
    if polygon.len() < 3 {
        return None;
    }
    Some(resample_closed_polygon(
        &polygon,
        previous_boundary.len().max(3),
    ))
}

pub(crate) fn retain_component_at_seed(
    binary: &mut [u8],
    width: u32,
    height: u32,
    seed: [f32; 2],
) -> Option<()> {
    if width == 0 || height == 0 || binary.len() != width as usize * height as usize {
        return None;
    }
    let sx = seed[0].round().clamp(0.0, width.saturating_sub(1) as f32) as u32;
    let sy = seed[1].round().clamp(0.0, height.saturating_sub(1) as f32) as u32;
    let start = (sy * width + sx) as usize;
    if binary[start] != 255 {
        return None;
    }

    let mut visited = vec![false; binary.len()];
    let mut queue = std::collections::VecDeque::from([start]);
    visited[start] = true;
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
                if binary[next] == 255 && !visited[next] {
                    visited[next] = true;
                    queue.push_back(next);
                }
            }
        }
    }
    for (value, keep) in binary.iter_mut().zip(visited) {
        if !keep {
            *value = 0;
        }
    }
    Some(())
}

pub(crate) fn resample_closed_polygon(points: &[[f32; 2]], count: usize) -> Vec<[f32; 2]> {
    if points.len() < 3 || count < 3 {
        return points.to_vec();
    }
    let lengths: Vec<f32> = (0..points.len())
        .map(|index| {
            let next = points[(index + 1) % points.len()];
            (next[0] - points[index][0]).hypot(next[1] - points[index][1])
        })
        .collect();
    let perimeter: f32 = lengths.iter().sum();
    if !perimeter.is_finite() || perimeter <= f32::EPSILON {
        return points[..count.min(points.len())].to_vec();
    }
    let mut output = Vec::with_capacity(count);
    let mut edge = 0;
    let mut edge_start = 0.0;
    for sample in 0..count {
        let distance = perimeter * sample as f32 / count as f32;
        while edge + 1 < lengths.len() && edge_start + lengths[edge] < distance {
            edge_start += lengths[edge];
            edge += 1;
        }
        let length = lengths[edge].max(f32::EPSILON);
        let amount = ((distance - edge_start) / length).clamp(0.0, 1.0);
        let a = points[edge];
        let b = points[(edge + 1) % points.len()];
        output.push([a[0] + (b[0] - a[0]) * amount, a[1] + (b[1] - a[1]) * amount]);
    }
    output
}

pub(crate) fn downsample_rgba(
    source: &[u8],
    width: u32,
    height: u32,
    out_w: u32,
    out_h: u32,
) -> Vec<u8> {
    let mut output = vec![0u8; out_w as usize * out_h as usize * 4];
    for y in 0..out_h {
        let sy = (((y as f32 + 0.5) * height as f32 / out_h as f32).floor() as u32).min(height - 1);
        for x in 0..out_w {
            let sx =
                (((x as f32 + 0.5) * width as f32 / out_w as f32).floor() as u32).min(width - 1);
            let src = ((sy * width + sx) * 4) as usize;
            let dst = ((y * out_w + x) * 4) as usize;
            output[dst..dst + 4].copy_from_slice(&source[src..src + 4]);
        }
    }
    output
}

pub(crate) fn warp_roto_boundary_with_flow(
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
            let round_trip_error = (forward[0] + backward[0]).hypot(forward[1] + backward[1]);
            let tolerance = 1.5 + forward[0].hypot(forward[1]) * 0.5;
            if !round_trip_error.is_finite() || round_trip_error > tolerance {
                *point
            } else {
                mapped
            }
        })
        .collect()
}

pub(crate) fn sample_dense_flow(
    flow: &crate::core::optical_flow_timewarp::DenseFlowField,
    x: f32,
    y: f32,
    width: u32,
    height: u32,
) -> [f32; 2] {
    let fx = (x * flow.width as f32 / width as f32).clamp(0.0, flow.width.saturating_sub(1) as f32);
    let fy =
        (y * flow.height as f32 / height as f32).clamp(0.0, flow.height.saturating_sub(1) as f32);
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
        (a[0] * (1.0 - tx) + b[0] * tx) * (1.0 - ty) + (c[0] * (1.0 - tx) + d[0] * tx) * ty,
        (a[1] * (1.0 - tx) + b[1] * tx) * (1.0 - ty) + (c[1] * (1.0 - tx) + d[1] * tx) * ty,
    ]
}
