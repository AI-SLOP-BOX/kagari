const MAX_CONCAVE_FACE_VERTICES: usize = 4096;

pub(super) fn triangulate_obj_face(
    face: &[u32],
    vertices: &[[f32; 3]],
) -> Result<Vec<[u32; 3]>, String> {
    if face.len() < 3 {
        return Err("OBJ face must contain at least three vertices".into());
    }
    let mut projected = Vec::new();
    projected
        .try_reserve_exact(face.len())
        .map_err(|error| format!("Could not reserve OBJ face projection: {error}"))?;

    let mut normal = [0.0f64; 3];
    for index in 0..face.len() {
        let current = vertices
            .get(face[index] as usize)
            .ok_or_else(|| "OBJ face index exceeds parsed vertices".to_string())?;
        let next = vertices
            .get(face[(index + 1) % face.len()] as usize)
            .ok_or_else(|| "OBJ face index exceeds parsed vertices".to_string())?;
        let a = current.map(f64::from);
        let b = next.map(f64::from);
        normal[0] += (a[1] - b[1]) * (a[2] + b[2]);
        normal[1] += (a[2] - b[2]) * (a[0] + b[0]);
        normal[2] += (a[0] - b[0]) * (a[1] + b[1]);
    }
    let dominant_axis = (0..3)
        .max_by(|&left, &right| normal[left].abs().total_cmp(&normal[right].abs()))
        .unwrap_or(2);
    if !normal[dominant_axis].is_finite() || normal[dominant_axis] == 0.0 {
        return Err("OBJ face is degenerate and cannot be triangulated".into());
    }

    for &index in face {
        let point = vertices
            .get(index as usize)
            .ok_or_else(|| "OBJ face index exceeds parsed vertices".to_string())?;
        let point = point.map(f64::from);
        projected.push(match dominant_axis {
            0 => [point[1], point[2]],
            1 => [point[0], point[2]],
            _ => [point[0], point[1]],
        });
    }

    let (mut min_x, mut min_y) = (f64::INFINITY, f64::INFINITY);
    let (mut max_x, mut max_y) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    for point in &projected {
        min_x = min_x.min(point[0]);
        min_y = min_y.min(point[1]);
        max_x = max_x.max(point[0]);
        max_y = max_y.max(point[1]);
    }
    let scale = (max_x - min_x).max(max_y - min_y);
    let epsilon = scale * scale * 1.0e-12;
    let signed_area = projected
        .iter()
        .zip(projected.iter().cycle().skip(1))
        .take(projected.len())
        .map(|(a, b)| a[0] * b[1] - b[0] * a[1])
        .sum::<f64>()
        * 0.5;
    if !scale.is_finite()
        || scale == 0.0
        || !signed_area.is_finite()
        || signed_area.abs() <= epsilon
    {
        return Err("OBJ face is degenerate and cannot be triangulated".into());
    }
    let orientation = signed_area.signum();
    let mut convex = true;
    for index in 0..projected.len() {
        let turn = cross(
            projected[(index + projected.len() - 1) % projected.len()],
            projected[index],
            projected[(index + 1) % projected.len()],
        );
        if turn * orientation < -epsilon {
            convex = false;
            break;
        }
    }

    if !convex && face.len() > MAX_CONCAVE_FACE_VERTICES {
        return Err(format!(
            "Concave OBJ faces are limited to {MAX_CONCAVE_FACE_VERTICES} vertices"
        ));
    }
    let mut triangles = Vec::new();
    triangles
        .try_reserve_exact(face.len() - 2)
        .map_err(|error| format!("Could not reserve OBJ triangulation: {error}"))?;
    if convex {
        for index in 1..face.len() - 1 {
            triangles.push([face[0], face[index], face[index + 1]]);
        }
        return Ok(triangles);
    }

    let mut remaining = Vec::new();
    remaining
        .try_reserve_exact(face.len())
        .map_err(|error| format!("Could not reserve OBJ face work list: {error}"))?;
    remaining.extend(0..face.len());
    while remaining.len() > 3 {
        let mut ear = None;
        for current_index in 0..remaining.len() {
            let previous = remaining[(current_index + remaining.len() - 1) % remaining.len()];
            let current = remaining[current_index];
            let next = remaining[(current_index + 1) % remaining.len()];
            if cross(projected[previous], projected[current], projected[next]) * orientation
                <= epsilon
            {
                continue;
            }
            if remaining.iter().copied().any(|candidate| {
                candidate != previous
                    && candidate != current
                    && candidate != next
                    && point_in_triangle(
                        projected[candidate],
                        projected[previous],
                        projected[current],
                        projected[next],
                        orientation,
                        epsilon,
                    )
            }) {
                continue;
            }
            ear = Some((current_index, [face[previous], face[current], face[next]]));
            break;
        }
        let Some((ear_index, triangle)) = ear else {
            return Err("OBJ face is self-intersecting or cannot be triangulated".into());
        };
        triangles.push(triangle);
        remaining.remove(ear_index);
    }
    triangles.push([face[remaining[0]], face[remaining[1]], face[remaining[2]]]);
    Ok(triangles)
}

fn cross(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

fn point_in_triangle(
    point: [f64; 2],
    a: [f64; 2],
    b: [f64; 2],
    c: [f64; 2],
    orientation: f64,
    epsilon: f64,
) -> bool {
    cross(a, b, point) * orientation >= -epsilon
        && cross(b, c, point) * orientation >= -epsilon
        && cross(c, a, point) * orientation >= -epsilon
}
