//! Shared projective mapping helpers for flat 3D footage layers.

/// Project the four corners of the renderer's centered unit quad into output
/// pixels. Returned entries are `[screen_x, screen_y, u, v]` in GPU vertex order.
pub(super) fn project_quad(
    matrix: [[f32; 4]; 4],
    width: f32,
    height: f32,
) -> Option<Vec<[f32; 4]>> {
    let local_corners = [
        [-0.5, -0.5, 0.0, 0.0],
        [0.5, -0.5, 1.0, 0.0],
        [0.5, 0.5, 1.0, 1.0],
        [-0.5, 0.5, 0.0, 1.0],
    ];
    // Clip in homogeneous coordinates just like the GPU. Rejecting the entire
    // plane when one corner crosses the near plane makes the CPU preview lose
    // geometry that the GPU still renders.
    let mut polygon: Vec<[f32; 6]> = local_corners
        .into_iter()
        .map(|[x, y, u, v]| {
            let clip = transform_point(matrix, x, y);
            [clip[0], clip[1], clip[2], clip[3], u, v]
        })
        .collect();
    for plane in 0..6 {
        if polygon.is_empty() {
            return None;
        }
        let input = std::mem::take(&mut polygon);
        let mut previous = *input.last()?;
        let mut previous_distance = clip_plane_distance(previous, plane);
        for current in input {
            let current_distance = clip_plane_distance(current, plane);
            let previous_inside = previous_distance >= 0.0;
            let current_inside = current_distance >= 0.0;
            if previous_inside != current_inside {
                let denominator = previous_distance - current_distance;
                if denominator.abs() > f32::EPSILON {
                    let t = (previous_distance / denominator).clamp(0.0, 1.0);
                    polygon.push(std::array::from_fn(|i| {
                        previous[i] + (current[i] - previous[i]) * t
                    }));
                }
            }
            if current_inside {
                polygon.push(current);
            }
            previous = current;
            previous_distance = current_distance;
        }
    }

    let mut projected = Vec::with_capacity(polygon.len());
    for [x, y, _z, w, u, v] in polygon {
        if !w.is_finite() || w <= 1e-5 {
            continue;
        }
        let ndc_x = x / w;
        let ndc_y = y / w;
        let screen = [
            (ndc_x + 1.0) * width * 0.5,
            (1.0 - ndc_y) * height * 0.5,
            u,
            v,
        ];
        if screen.iter().all(|component| component.is_finite()) {
            projected.push(screen);
        }
    }
    (projected.len() >= 3).then_some(projected)
}

fn clip_plane_distance(vertex: [f32; 6], plane: usize) -> f32 {
    let [x, y, z, w, _, _] = vertex;
    match plane {
        0 => x + w,
        1 => w - x,
        2 => y + w,
        3 => w - y,
        4 => z,
        _ => w - z,
    }
}

/// Invert the planar perspective transform at an output pixel center to get
/// the same projectively interpolated UVs used by the GPU rasterizer.
pub(super) fn screen_to_uv(
    matrix: [[f32; 4]; 4],
    width: f32,
    height: f32,
    screen_x: f32,
    screen_y: f32,
) -> Option<[f32; 2]> {
    let ndc_x = screen_x * 2.0 / width - 1.0;
    let ndc_y = 1.0 - screen_y * 2.0 / height;

    // clip.x / clip.w = ndc.x and clip.y / clip.w = ndc.y form a 2x2
    // system in local layer coordinates (z is always zero for footage planes).
    let a = matrix[0][0] - ndc_x * matrix[0][3];
    let b = matrix[1][0] - ndc_x * matrix[1][3];
    let c = ndc_x * matrix[3][3] - matrix[3][0];
    let d = matrix[0][1] - ndc_y * matrix[0][3];
    let e = matrix[1][1] - ndc_y * matrix[1][3];
    let f = ndc_y * matrix[3][3] - matrix[3][1];
    let determinant = a * e - b * d;
    if !determinant.is_finite() || determinant.abs() < 1e-8 {
        return None;
    }

    let local_x = (c * e - b * f) / determinant;
    let local_y = (a * f - c * d) / determinant;
    if !local_x.is_finite() || !local_y.is_finite() {
        return None;
    }
    Some([local_x + 0.5, local_y + 0.5])
}

fn transform_point(matrix: [[f32; 4]; 4], x: f32, y: f32) -> [f32; 4] {
    let point = [x, y, 0.0, 1.0];
    std::array::from_fn(|row| {
        matrix[0][row] * point[0]
            + matrix[1][row] * point[1]
            + matrix[2][row] * point[2]
            + matrix[3][row] * point[3]
    })
}

#[cfg(test)]
mod tests {
    use super::{project_quad, screen_to_uv};

    #[test]
    fn projective_mapping_round_trips_a_tilted_quad() {
        // A nonsymmetric projective homography exercises both axes and w.
        let matrix = [
            [1.8, 0.2, 0.0, 0.12],
            [-0.3, -1.6, 0.0, -0.08],
            [0.0, 0.0, 1.0, 1.0],
            [0.1, -0.05, 0.0, 1.4],
        ];
        let corners = project_quad(matrix, 128.0, 96.0).expect("quad should be in front");
        for [screen_x, screen_y, expected_u, expected_v] in corners {
            let uv = screen_to_uv(matrix, 128.0, 96.0, screen_x, screen_y)
                .expect("projected point should be invertible");
            assert!((uv[0] - expected_u).abs() < 1e-5);
            assert!((uv[1] - expected_v).abs() < 1e-5);
        }
    }

    #[test]
    fn project_quad_clips_against_the_near_plane() {
        let matrix = [
            [1.0, 0.0, 1.0, 1.0],
            [0.0, -1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.4, 0.5],
        ];
        let clipped = project_quad(matrix, 128.0, 96.0)
            .expect("visible portion should survive near-plane clipping");
        assert!(clipped.len() > 4, "clipping should add boundary vertices");
        assert!(clipped
            .iter()
            .all(|vertex| vertex.iter().all(|v| v.is_finite())));
        assert!(clipped
            .iter()
            .any(|vertex| vertex[2] > 0.0 && vertex[2] < 1.0));
    }

    #[test]
    fn project_quad_rejects_geometry_behind_the_camera() {
        let matrix = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, -1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, -1.0],
        ];
        assert!(project_quad(matrix, 128.0, 96.0).is_none());
    }
}
