use super::types::ReframePoint;

/// Produces a smoothed, crop-safe center path from tracked subject positions.
pub fn build_auto_reframe_path(
    tracked_centers: &[[f32; 2]],
    source_size: [u32; 2],
    output_size: [u32; 2],
    smoothing: f32,
) -> Vec<ReframePoint> {
    if source_size.contains(&0)
        || output_size.contains(&0)
        || output_size[0] > source_size[0]
        || output_size[1] > source_size[1]
        || tracked_centers
            .iter()
            .flatten()
            .any(|value| !value.is_finite())
    {
        return Vec::new();
    }
    let smoothing = if smoothing.is_finite() {
        smoothing.clamp(0.0, 0.99)
    } else {
        0.8
    };
    let half = [output_size[0] as f32 * 0.5, output_size[1] as f32 * 0.5];
    let max = [
        source_size[0] as f32 - half[0],
        source_size[1] as f32 - half[1],
    ];
    let mut previous = None;
    tracked_centers
        .iter()
        .enumerate()
        .map(|(frame, center)| {
            let target = [
                center[0].clamp(half[0], max[0]),
                center[1].clamp(half[1], max[1]),
            ];
            let smoothed = previous.map_or(target, |old: [f32; 2]| {
                [
                    old[0] * smoothing + target[0] * (1.0 - smoothing),
                    old[1] * smoothing + target[1] * (1.0 - smoothing),
                ]
            });
            previous = Some(smoothed);
            ReframePoint {
                frame,
                center: smoothed,
            }
        })
        .collect()
}

/// Reduces 2D motion samples while keeping endpoints and the requested error bound.
pub fn reduce_motion_path(points: &[[f32; 2]], tolerance: f32) -> Vec<usize> {
    if points.len() <= 2 {
        return (0..points.len()).collect();
    }
    if points.iter().flatten().any(|v| !v.is_finite()) {
        return (0..points.len()).collect();
    }
    let tolerance = if tolerance.is_finite() {
        tolerance.max(0.0)
    } else {
        0.0
    };
    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[points.len() - 1] = true;
    simplify_range(points, 0, points.len() - 1, tolerance, &mut keep);
    keep.into_iter()
        .enumerate()
        .filter_map(|(index, retain)| retain.then_some(index))
        .collect()
}

fn simplify_range(
    points: &[[f32; 2]],
    start: usize,
    end: usize,
    tolerance: f32,
    keep: &mut [bool],
) {
    let mut farthest = 0.0;
    let mut farthest_index = None;
    for index in start + 1..end {
        let distance = point_segment_distance(points[index], points[start], points[end]);
        if distance > farthest {
            farthest = distance;
            farthest_index = Some(index);
        }
    }
    if farthest > tolerance {
        let index = farthest_index.expect("positive distance has an index");
        keep[index] = true;
        simplify_range(points, start, index, tolerance, keep);
        simplify_range(points, index, end, tolerance, keep);
    }
}

fn point_segment_distance(point: [f32; 2], start: [f32; 2], end: [f32; 2]) -> f32 {
    let delta = [end[0] - start[0], end[1] - start[1]];
    let length_squared = delta[0] * delta[0] + delta[1] * delta[1];
    if length_squared <= f32::EPSILON {
        return ((point[0] - start[0]).powi(2) + (point[1] - start[1]).powi(2)).sqrt();
    }
    let t = (((point[0] - start[0]) * delta[0] + (point[1] - start[1]) * delta[1])
        / length_squared)
        .clamp(0.0, 1.0);
    let projected = [start[0] + t * delta[0], start[1] + t * delta[1]];
    ((point[0] - projected[0]).powi(2) + (point[1] - projected[1]).powi(2)).sqrt()
}
