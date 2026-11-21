#[cfg(test)]
pub(super) use super::motion_prior::constrain_matte_to_dilated_boundary;
use super::motion_prior::try_filled_vec;
pub(super) use super::motion_prior::{
    constrain_matte_to_dilated_boundary_cancellable, roto_motion_prior_margin,
};
use super::segmentation::*;
use crate::core::keyframe::{InterpolationType, Keyframe};
use crate::core::mask::Mask;
use crate::core::property::Animatable;
use crate::core::timeline::TrackerPoint;

// 160px collapsed pedestrians and narrow boundaries into only a handful of
// flow cells on HD footage. 320px keeps more subject detail while retaining a
// bounded analysis size for 4K and larger compositions.
const ROTO_FLOW_MAX_DIMENSION: f32 = 320.0;
const ROTO_FLOW_SEARCH_RADIUS: i32 = 8;
// A single-frame color re-segmentation can absorb nearby people or background
// regions. Keep those jumps from replacing the motion-warped matte wholesale.
const ROTO_MAX_RESEGMENT_AREA_RATIO: f32 = 1.35;
// Corrections may intentionally reshape the matte, while still requiring a
// finite limit so one ambiguous set of strokes cannot select the whole scene.
const ROTO_MAX_CORRECTED_AREA_RATIO: f32 = 2.0;

pub fn bake_tracked_mask(
    base_mask: &Mask,
    tracker: &TrackerPoint,
    start_frame: u32,
    end_frame: u32,
) -> Result<Animatable<Vec<[f32; 2]>>, String> {
    if end_frame < start_frame {
        return Err("end frame must not precede start frame".into());
    }
    let base_poly = base_mask.path.to_polygon(start_frame, 16);
    if base_poly.is_empty() {
        return Err("base mask has no vertices".into());
    }
    if base_poly
        .iter()
        .flatten()
        .any(|coordinate| !coordinate.is_finite())
    {
        return Err("base mask contains non-finite vertices".into());
    }
    let origin = tracker.position.evaluate(start_frame);
    if origin.iter().any(|coordinate| !coordinate.is_finite()) {
        return Err("tracker position is not finite".into());
    }
    let mut kfs: Vec<Keyframe<Vec<[f32; 2]>>> = Vec::new();
    let step = (end_frame.saturating_sub(start_frame) / 60).max(1); // ≤ ~61 samples
    let mut f = start_frame;
    loop {
        let cur = tracker.position.evaluate(f);
        if cur.iter().any(|coordinate| !coordinate.is_finite()) {
            return Err(format!("tracker position is not finite at frame {f}"));
        }
        let dx = cur[0] - origin[0];
        let dy = cur[1] - origin[1];
        if !dx.is_finite() || !dy.is_finite() {
            return Err(format!("tracker displacement is not finite at frame {f}"));
        }
        let moved: Vec<[f32; 2]> = base_poly.iter().map(|p| [p[0] + dx, p[1] + dy]).collect();
        if moved
            .iter()
            .flatten()
            .any(|coordinate| !coordinate.is_finite())
        {
            return Err(format!("baked mask position is not finite at frame {f}"));
        }
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
    frame_provider: F,
) -> Result<Animatable<Vec<[f32; 2]>>, String>
where
    F: FnMut(u32) -> Option<Vec<u8>>,
{
    bake_optical_flow_roto_matte_cancellable(
        base_mask,
        source_strokes,
        settings,
        anchor_frame,
        start_frame,
        end_frame,
        width,
        height,
        frame_provider,
        || false,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn bake_optical_flow_roto_matte_cancellable<F, C>(
    base_mask: &Mask,
    source_strokes: &[crate::core::roto_brush_engine::RotoStroke],
    settings: &crate::core::roto_brush_engine::RotoBrushSettings,
    anchor_frame: u32,
    start_frame: u32,
    end_frame: u32,
    width: u32,
    height: u32,
    mut frame_provider: F,
    mut is_cancelled: C,
) -> Result<Animatable<Vec<[f32; 2]>>, String>
where
    F: FnMut(u32) -> Option<Vec<u8>>,
    C: FnMut() -> bool,
{
    if end_frame < start_frame || anchor_frame < start_frame || anchor_frame > end_frame {
        return Err("roto propagation range must contain the anchor frame".into());
    }
    if is_cancelled() {
        return Err("Roto optical-flow propagation cancelled".into());
    }
    let pixel_count = (width as usize)
        .checked_mul(height as usize)
        .filter(|pixels| *pixels <= crate::core::roto_brush_engine::MAX_ROTO_FRAME_PIXELS)
        .ok_or_else(|| "Roto propagation frame exceeds the supported pixel limit".to_owned())?;
    let expected_len = pixel_count
        .checked_mul(4)
        .filter(|_| width > 0 && height > 0)
        .ok_or_else(|| "invalid composition dimensions for roto propagation".to_owned())?;
    let anchor_boundary = base_mask.path.to_polygon(anchor_frame, 16);
    if anchor_boundary.len() < 3 {
        return Err("Roto Brush Matte has no closed boundary to propagate".into());
    }
    if anchor_boundary
        .iter()
        .flatten()
        .any(|coordinate| !coordinate.is_finite())
    {
        return Err("Roto Brush Matte boundary contains non-finite vertices".into());
    }
    let mut boundary = anchor_boundary.clone();

    let anchor_pixels = match frame_provider(anchor_frame) {
        Some(pixels) => pixels,
        None if is_cancelled() => {
            return Err("Roto optical-flow propagation cancelled".into());
        }
        None => return Err(format!("Could not render source frame {anchor_frame}")),
    };
    if is_cancelled() {
        return Err("Roto optical-flow propagation cancelled".into());
    }
    if anchor_pixels.len() != expected_len {
        return Err(format!(
            "Source frame {anchor_frame} has invalid dimensions"
        ));
    }

    let flow_scale = (ROTO_FLOW_MAX_DIMENSION / width.max(height) as f32).min(1.0);
    let flow_width = ((width as f32 * flow_scale).round() as u32).max(1);
    let flow_height = ((height as f32 * flow_scale).round() as u32).max(1);
    let flow_buffer_len = (flow_width as usize)
        .checked_mul(flow_height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| "invalid downsampled Roto dimensions".to_owned())?;
    let Some(anchor_small) = downsample_rgba(
        &anchor_pixels,
        width,
        height,
        flow_width,
        flow_height,
        &mut is_cancelled,
    ) else {
        if is_cancelled() {
            return Err("Roto optical-flow propagation cancelled".into());
        }
        return Err("Could not allocate downsampled Roto source frame".into());
    };
    if anchor_small.len() != flow_buffer_len {
        return Err("Could not allocate downsampled Roto source frame".into());
    }
    drop(anchor_pixels);
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
        boundary = anchor_boundary.clone();
        let mut propagated_strokes = anchor_strokes.clone();
        loop {
            if is_cancelled() {
                return Err("Roto optical-flow propagation cancelled".into());
            }
            let next_frame = previous_frame as i64 + direction;
            if next_frame < start_frame as i64 || next_frame > end_frame as i64 {
                break;
            }
            let next_frame = next_frame as u32;
            let pixels = match frame_provider(next_frame) {
                Some(pixels) => pixels,
                None if is_cancelled() => {
                    return Err("Roto optical-flow propagation cancelled".into());
                }
                None => return Err(format!("Could not render source frame {next_frame}")),
            };
            if is_cancelled() {
                return Err("Roto optical-flow propagation cancelled".into());
            }
            if pixels.len() != expected_len {
                return Err(format!("Source frame {next_frame} has invalid dimensions"));
            }
            let Some(current_small) = downsample_rgba(
                &pixels,
                width,
                height,
                flow_width,
                flow_height,
                &mut is_cancelled,
            ) else {
                if is_cancelled() {
                    return Err("Roto optical-flow propagation cancelled".into());
                }
                return Err(format!(
                    "Could not allocate downsampled Roto source frame {next_frame}"
                ));
            };
            if current_small.len() != flow_buffer_len {
                return Err(format!(
                    "Could not allocate downsampled Roto source frame {next_frame}"
                ));
            }
            let Some(flow) =
                crate::core::optical_flow_timewarp::compute_dense_optical_flow_cancellable(
                    &previous_small,
                    &current_small,
                    flow_width,
                    flow_height,
                    1,
                    ROTO_FLOW_SEARCH_RADIUS,
                    &mut is_cancelled,
                )
            else {
                return Err("Roto optical-flow propagation cancelled".into());
            };
            if !flow.is_complete() {
                return Err(format!(
                    "Could not allocate Roto optical-flow field at frame {next_frame}"
                ));
            }
            if is_cancelled() {
                return Err("Roto optical-flow propagation cancelled".into());
            }
            let Some(reverse_flow) =
                crate::core::optical_flow_timewarp::compute_dense_optical_flow_cancellable(
                    &current_small,
                    &previous_small,
                    flow_width,
                    flow_height,
                    1,
                    ROTO_FLOW_SEARCH_RADIUS,
                    &mut is_cancelled,
                )
            else {
                return Err("Roto optical-flow propagation cancelled".into());
            };
            if !reverse_flow.is_complete() {
                return Err(format!(
                    "Could not allocate reverse Roto optical-flow field at frame {next_frame}"
                ));
            }
            if is_cancelled() {
                return Err("Roto optical-flow propagation cancelled".into());
            }
            boundary = warp_roto_boundary_with_flow(&boundary, &flow, &reverse_flow, width, height);
            propagated_strokes =
                warp_roto_strokes(&propagated_strokes, &flow, &reverse_flow, width, height);
            let frame_corrections = source_strokes
                .iter()
                .filter(|stroke| stroke.frame == Some(next_frame))
                .cloned()
                .collect::<Vec<_>>();
            propagated_strokes.extend(frame_corrections.iter().cloned().map(|mut stroke| {
                stroke.frame = None;
                stroke
            }));
            let has_user_correction = !frame_corrections.is_empty();
            let refined_boundary = resegment_propagated_boundary(
                &pixels,
                width,
                height,
                &boundary,
                &propagated_strokes,
                &frame_corrections,
                settings,
                &mut is_cancelled,
            );
            if is_cancelled() {
                return Err("Roto optical-flow propagation cancelled".into());
            }
            if let Some(refined_boundary) = refined_boundary {
                // Compare against the original anchor as well as the previous
                // frame: small segmentation errors can compound into a matte
                // that slowly absorbs the full scene without a single large jump.
                let max_area_ratio = if has_user_correction {
                    ROTO_MAX_CORRECTED_AREA_RATIO
                } else {
                    ROTO_MAX_RESEGMENT_AREA_RATIO
                };
                if is_plausible_resegment_area_with_limit(
                    &anchor_boundary,
                    &refined_boundary,
                    max_area_ratio,
                    has_user_correction,
                ) {
                    boundary = refined_boundary;
                } else if has_user_correction {
                    return Err(format!(
                        "Roto Brush correction at frame {next_frame} could not be applied safely"
                    ));
                }
            } else if has_user_correction {
                return Err(format!(
                    "Roto Brush correction at frame {next_frame} could not be segmented"
                ));
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

#[allow(clippy::too_many_arguments)]
pub(crate) fn resegment_propagated_boundary(
    pixels: &[u8],
    width: u32,
    height: u32,
    previous_boundary: &[[f32; 2]],
    strokes: &[crate::core::roto_brush_engine::RotoStroke],
    correction_strokes: &[crate::core::roto_brush_engine::RotoStroke],
    settings: &crate::core::roto_brush_engine::RotoBrushSettings,
    is_cancelled: &mut impl FnMut() -> bool,
) -> Option<Vec<[f32; 2]>> {
    if is_cancelled() {
        return None;
    }
    let has_user_correction = !correction_strokes.is_empty();
    let mut geometry_settings = settings.clone();
    geometry_settings.feather_radius = 0.0;
    let foreground_seed = latest_foreground_seed(strokes)?;
    let alpha = crate::core::roto_brush_engine::generate_rotobrush_matte_cancellable(
        pixels,
        width,
        height,
        strokes,
        &geometry_settings,
        &mut *is_cancelled,
    )?;
    if is_cancelled() {
        return None;
    }
    if width == 0 || height == 0 || alpha.len() != (width as usize).checked_mul(height as usize)? {
        return None;
    }
    let mut binary = alpha;
    for row in binary.chunks_exact_mut(width as usize) {
        if is_cancelled() {
            return None;
        }
        for value in row {
            *value = if *value >= 128 { 255 } else { 0 };
        }
    }
    if !constrain_matte_to_dilated_boundary_cancellable(
        &mut binary,
        width,
        height,
        previous_boundary,
        roto_motion_prior_margin(width, height),
        correction_strokes,
        &mut *is_cancelled,
    ) {
        return None;
    }
    if is_cancelled() {
        return None;
    }
    retain_component_at_seed(&mut binary, width, height, foreground_seed)?;
    let polygon = trace_contour_to_polygon(&binary, width, height, 1.5);
    if polygon.len() < 3 {
        return None;
    }
    let max_area_ratio = if has_user_correction {
        ROTO_MAX_CORRECTED_AREA_RATIO
    } else {
        ROTO_MAX_RESEGMENT_AREA_RATIO
    };
    if !is_plausible_resegment_area_with_limit(
        previous_boundary,
        &polygon,
        max_area_ratio,
        has_user_correction,
    ) {
        return None;
    }
    Some(resample_closed_polygon(
        &polygon,
        previous_boundary.len().max(3),
    ))
}

pub(super) fn latest_foreground_seed(
    strokes: &[crate::core::roto_brush_engine::RotoStroke],
) -> Option<[f32; 2]> {
    strokes
        .iter()
        .rev()
        .filter(|stroke| {
            stroke.stroke_type == crate::core::roto_brush_engine::RotoStrokeType::Foreground
        })
        .find_map(|stroke| {
            stroke
                .points
                .iter()
                .copied()
                .find(|point| point.iter().all(|coordinate| coordinate.is_finite()))
        })
}

#[cfg(test)]
pub(super) fn is_plausible_resegment_area(previous: &[[f32; 2]], candidate: &[[f32; 2]]) -> bool {
    is_plausible_resegment_area_with_limit(
        previous,
        candidate,
        ROTO_MAX_RESEGMENT_AREA_RATIO,
        false,
    )
}

fn is_plausible_resegment_area_with_limit(
    previous: &[[f32; 2]],
    candidate: &[[f32; 2]],
    max_ratio: f32,
    allow_shrink: bool,
) -> bool {
    let previous_area = polygon_area(previous);
    let candidate_area = polygon_area(candidate);
    if !previous_area.is_finite()
        || !candidate_area.is_finite()
        || previous_area <= f32::EPSILON
        || candidate_area <= f32::EPSILON
    {
        return false;
    }
    let ratio = candidate_area / previous_area;
    ratio <= max_ratio && (allow_shrink || ratio >= 1.0 / max_ratio)
}

fn polygon_area(points: &[[f32; 2]]) -> f32 {
    if points.len() < 3 {
        return 0.0;
    }
    let twice_area: f32 = points
        .iter()
        .enumerate()
        .map(|(index, point)| {
            let next = points[(index + 1) % points.len()];
            point[0] * next[1] - next[0] * point[1]
        })
        .sum();
    twice_area.abs() * 0.5
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

fn downsample_rgba(
    source: &[u8],
    width: u32,
    height: u32,
    out_w: u32,
    out_h: u32,
    is_cancelled: &mut impl FnMut() -> bool,
) -> Option<Vec<u8>> {
    let source_len = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))?;
    let output_len = (out_w as usize)
        .checked_mul(out_h as usize)
        .and_then(|pixels| pixels.checked_mul(4))?;
    if width == 0 || height == 0 || out_w == 0 || out_h == 0 || source.len() != source_len {
        return None;
    }

    let mut output = try_filled_vec(output_len, 0u8)?;
    let scale_x = width as f64 / out_w as f64;
    let scale_y = height as f64 / out_h as f64;
    for y in 0..out_h {
        if is_cancelled() {
            return None;
        }
        let top = y as f64 * scale_y;
        let bottom = (y + 1) as f64 * scale_y;
        let first_y = top.floor() as u32;
        let last_y = bottom.ceil().min(height as f64) as u32;
        for x in 0..out_w {
            let left = x as f64 * scale_x;
            let right = (x + 1) as f64 * scale_x;
            let first_x = left.floor() as u32;
            let last_x = right.ceil().min(width as f64) as u32;
            let area = (right - left) * (bottom - top);
            let mut sums = [0.0f64; 4];
            for sy in first_y..last_y {
                let weight_y = (bottom.min(sy as f64 + 1.0) - top.max(sy as f64)).max(0.0);
                for sx in first_x..last_x {
                    let weight_x = (right.min(sx as f64 + 1.0) - left.max(sx as f64)).max(0.0);
                    let weight = weight_x * weight_y;
                    let src = (sy as usize * width as usize + sx as usize) * 4;
                    for channel in 0..4 {
                        sums[channel] += source[src + channel] as f64 * weight;
                    }
                }
            }
            let dst = ((y * out_w + x) * 4) as usize;
            for (channel, sum) in sums.into_iter().enumerate() {
                output[dst + channel] = (sum / area).round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    Some(output)
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
