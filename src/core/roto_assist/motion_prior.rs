use crate::core::roto_brush_engine::filter::squared_distance_transform_1d_into;
use crate::core::roto_brush_engine::{RotoStroke, RotoStrokeType};

const ROTO_MOTION_PRIOR_MARGIN_RATIO: f32 = 0.005;

pub(super) fn try_filled_vec<T: Clone>(len: usize, value: T) -> Option<Vec<T>> {
    let mut values = Vec::new();
    values.try_reserve_exact(len).ok()?;
    values.resize(len, value);
    Some(values)
}

pub(super) fn roto_motion_prior_margin(width: u32, height: u32) -> u32 {
    ((width.max(height) as f32 * ROTO_MOTION_PRIOR_MARGIN_RATIO).ceil() as u32).max(2)
}

#[cfg(test)]
pub(super) fn constrain_matte_to_dilated_boundary(
    matte: &mut [u8],
    width: u32,
    height: u32,
    boundary: &[[f32; 2]],
    margin: u32,
    correction_strokes: &[RotoStroke],
) -> bool {
    constrain_matte_to_dilated_boundary_cancellable(
        matte,
        width,
        height,
        boundary,
        margin,
        correction_strokes,
        &mut || false,
    )
}

pub(super) fn constrain_matte_to_dilated_boundary_cancellable(
    matte: &mut [u8],
    width: u32,
    height: u32,
    boundary: &[[f32; 2]],
    margin: u32,
    correction_strokes: &[RotoStroke],
    is_cancelled: &mut impl FnMut() -> bool,
) -> bool {
    if is_cancelled() {
        return false;
    }
    let Some(pixel_count) = (width as usize).checked_mul(height as usize) else {
        return false;
    };
    if width == 0
        || height == 0
        || matte.len() != pixel_count
        || boundary.len() < 3
        || boundary
            .iter()
            .flatten()
            .any(|coordinate| !coordinate.is_finite())
    {
        return false;
    }

    let w = width as usize;
    let h = height as usize;
    let Some(mut inside) = try_filled_vec(pixel_count, 0u8) else {
        return false;
    };
    let mut intersections = Vec::new();
    if intersections.try_reserve_exact(boundary.len()).is_err() {
        return false;
    }
    for y in 0..h {
        if is_cancelled() {
            return false;
        }
        intersections.clear();
        let scan_y = y as f32 + 0.5;
        for (index, point) in boundary.iter().enumerate() {
            let next = boundary[(index + 1) % boundary.len()];
            if (point[1] > scan_y) != (next[1] > scan_y) {
                let x =
                    point[0] + (scan_y - point[1]) * (next[0] - point[0]) / (next[1] - point[1]);
                if x.is_finite() {
                    intersections.push(x);
                }
            }
        }
        intersections.sort_by(f32::total_cmp);
        for pair in intersections.as_chunks::<2>().0 {
            let left = (pair[0].ceil() as i64).clamp(0, width as i64) as usize;
            let right = (pair[1].ceil() as i64).clamp(0, width as i64) as usize;
            if left < right {
                inside[y * w + left..y * w + right].fill(1);
            }
        }
    }

    let foreground_correction_count = correction_strokes
        .iter()
        .filter(|stroke| stroke.stroke_type == RotoStrokeType::Foreground)
        .count();
    let mut foreground_corrections = Vec::new();
    if foreground_corrections
        .try_reserve_exact(foreground_correction_count)
        .is_err()
    {
        return false;
    }
    foreground_corrections.extend(
        correction_strokes
            .iter()
            .filter(|stroke| stroke.stroke_type == RotoStrokeType::Foreground)
            .cloned(),
    );
    let correction_support = if !foreground_corrections.is_empty() {
        let Some(mut correction_support) = try_filled_vec(pixel_count, 0u8) else {
            return false;
        };
        if crate::core::roto_brush_engine::matte::apply_stroke_constraints(
            &mut correction_support,
            width,
            height,
            &foreground_corrections,
            is_cancelled,
        )
        .is_none()
        {
            return false;
        }
        Some(correction_support)
    } else {
        None
    };

    let radius = margin.min(width.max(height)) as f32;
    let radius_squared = radius * radius;
    let max_dimension = w.max(h);
    let Some(mut line_input) = try_filled_vec(max_dimension, f32::INFINITY) else {
        return false;
    };
    let Some(mut line_output) = try_filled_vec(max_dimension, f32::INFINITY) else {
        return false;
    };
    let Some(mut sites) = try_filled_vec(max_dimension, 0usize) else {
        return false;
    };
    let Some(boundary_count) = max_dimension.checked_add(1) else {
        return false;
    };
    let Some(mut boundaries) = try_filled_vec(boundary_count, 0.0f64) else {
        return false;
    };
    let Some(mut squared_distances) = try_filled_vec(pixel_count, f32::INFINITY) else {
        return false;
    };
    for y in 0..h {
        if is_cancelled() {
            return false;
        }
        for x in 0..w {
            line_input[x] = if inside[y * w + x] > 0 {
                0.0
            } else {
                f32::INFINITY
            };
        }
        squared_distance_transform_1d_into(
            &line_input[..w],
            &mut squared_distances[y * w..(y + 1) * w],
            &mut sites[..w],
            &mut boundaries[..=w],
        );
    }
    drop(inside);

    for x in 0..w {
        if is_cancelled() {
            return false;
        }
        for y in 0..h {
            line_input[y] = squared_distances[y * w + x];
        }
        squared_distance_transform_1d_into(
            &line_input[..h],
            &mut line_output[..h],
            &mut sites[..h],
            &mut boundaries[..=h],
        );
        for (y, &distance_squared) in line_output[..h].iter().enumerate() {
            let index = y * w + x;
            let prior_allows = distance_squared <= radius_squared;
            let correction_allows = correction_support
                .as_ref()
                .is_some_and(|support| support[index] > 0);
            if !prior_allows && !correction_allows {
                matte[index] = 0;
            }
        }
    }
    true
}
