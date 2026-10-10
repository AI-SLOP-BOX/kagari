pub fn tessellate_bezier_path(
    points: &[[f32; 2]],
    tangents: &[([f32; 2], [f32; 2])],
    closed: bool,
    subdivisions: u32,
) -> Vec<[f32; 2]> {
    if points.len() < 2 {
        return points.to_vec();
    }

    let mut result = Vec::new();
    let n = points.len();

    for i in 0..n {
        let p0 = points[i];
        let p1 = points[(i + 1) % n];

        let out_tan = if i < tangents.len() {
            tangents[i].1
        } else {
            p0
        };
        let in_tan = if (i + 1) % n < tangents.len() {
            tangents[(i + 1) % n].0
        } else {
            p1
        };

        let has_curves = (out_tan[0] - p0[0]).abs() > 0.01
            || (out_tan[1] - p0[1]).abs() > 0.01
            || (in_tan[0] - p1[0]).abs() > 0.01
            || (in_tan[1] - p1[1]).abs() > 0.01;

        if has_curves {
            let steps = subdivisions.max(4);
            for s in 0..=steps {
                let t = s as f32 / steps as f32;
                let t2 = t * t;
                let t3 = t2 * t;
                let mt = 1.0 - t;
                let mt2 = mt * mt;
                let mt3 = mt2 * mt;

                let x = mt3 * p0[0]
                    + 3.0 * mt2 * t * out_tan[0]
                    + 3.0 * mt * t2 * in_tan[0]
                    + t3 * p1[0];
                let y = mt3 * p0[1]
                    + 3.0 * mt2 * t * out_tan[1]
                    + 3.0 * mt * t2 * in_tan[1]
                    + t3 * p1[1];
                result.push([x, y]);
            }
        } else {
            result.push(p0);
        }
    }

    if closed && !result.is_empty() {
        result.push(result[0]);
    }

    result
}

/// SDF for axis-aligned rectangle centered at origin with half-extents (hx, hy).
pub fn sdf_rectangle(x: f32, y: f32, hx: f32, hy: f32) -> f32 {
    let dx = x.abs() - hx;
    let dy = y.abs() - hy;
    let outside = (dx.max(0.0), dy.max(0.0));
    let inside = dx.min(0.0).max(dy.min(0.0));
    (outside.0 * outside.0 + outside.1 * outside.1).sqrt() + inside
}

/// SDF for axis-aligned ellipse centered at origin with radii (rx, ry).
/// Returns negative inside, positive outside.
pub fn sdf_ellipse(x: f32, y: f32, rx: f32, ry: f32) -> f32 {
    let nx = x / rx;
    let ny = y / ry;
    let dist = (nx * nx + ny * ny).sqrt() - 1.0;
    dist * rx.min(ry)
}

/// SDF for a star with n points, outer radius, and inner radius.
/// Returns negative inside, positive outside.
pub fn sdf_star(x: f32, y: f32, points: u32, outer_r: f32, inner_r: f32) -> f32 {
    let angle = y.atan2(x);
    let radius = (x * x + y * y).sqrt();
    let segment = 2.0 * std::f32::consts::PI / (points as f32 * 2.0);
    let half = segment * 0.5;
    let a = ((angle + half) % segment - half).abs();
    let r = inner_r + (outer_r - inner_r) * (a / half).min(1.0);
    radius - r
}

/// SDF for a regular polygon with n sides and radius r.
/// Returns negative inside, positive outside.
pub fn sdf_polygon(x: f32, y: f32, sides: u32, radius: f32) -> f32 {
    let angle = y.atan2(x);
    let radius_point = (x * x + y * y).sqrt();
    let segment = 2.0 * std::f32::consts::PI / (sides as f32);
    let a = (angle % segment + segment) % segment - segment * 0.5;
    let s = radius * (a.cos() / (std::f32::consts::PI / sides as f32).cos());
    radius_point - s
}

/// SDF for an arbitrary polygon defined by vertices.
pub fn sdf_polygon_points(x: f32, y: f32, points: &[(f32, f32)]) -> f32 {
    if points.len() < 3 {
        return 1.0;
    }
    let n = points.len();
    let mut dist = f32::MAX;
    for i in 0..n {
        let a = points[i];
        let b = points[(i + 1) % n];
        let ab = (b.0 - a.0, b.1 - a.1);
        let ap = (x - a.0, y - a.1);
        let t = (ap.0 * ab.0 + ap.1 * ab.1) / (ab.0 * ab.0 + ab.1 * ab.1).max(1e-10);
        let t = t.clamp(0.0, 1.0);
        let closest = (a.0 + t * ab.0, a.1 + t * ab.1);
        let dx = x - closest.0;
        let dy = y - closest.1;
        dist = dist.min((dx * dx + dy * dy).sqrt());
    }
    let mut wn = 0i32;
    for i in 0..n {
        let a = points[i];
        let b = points[(i + 1) % n];
        if a.1 <= y {
            if b.1 > y && (b.0 - a.0) * (y - a.1) - (b.1 - a.1) * (x - a.0) > 0.0 {
                wn += 1;
            }
        } else if b.1 <= y && (b.0 - a.0) * (y - a.1) - (b.1 - a.1) * (x - a.0) < 0.0 {
                wn -= 1;
        }
    }
    if wn != 0 {
        -dist
    } else {
        dist
    }
}
