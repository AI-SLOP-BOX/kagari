pub(crate) fn signed_polygon_area(poly: &[[f32; 2]]) -> f32 {
    let mut area = 0.0f32;
    let n = poly.len();
    for i in 0..n {
        let j = (i + 1) % n;
        area += poly[i][0] * poly[j][1] - poly[j][0] * poly[i][1];
    }
    area * 0.5
}

pub(crate) fn is_inside_edge(p: [f32; 2], p1: [f32; 2], p2: [f32; 2], is_ccw: bool) -> bool {
    let cross = (p2[0] - p1[0]) * (p[1] - p1[1]) - (p2[1] - p1[1]) * (p[0] - p1[0]);
    if is_ccw {
        cross >= -1e-4
    } else {
        cross <= 1e-4
    }
}

pub(crate) fn line_intersection_edge(
    a1: [f32; 2],
    a2: [f32; 2],
    b1: [f32; 2],
    b2: [f32; 2],
) -> Option<[f32; 2]> {
    let d = (b2[1] - b1[1]) * (a2[0] - a1[0]) - (b2[0] - b1[0]) * (a2[1] - a1[1]);
    if d.abs() < 1e-6 {
        return None;
    }
    let ua = ((b2[0] - b1[0]) * (a1[1] - b1[1]) - (b2[1] - b1[1]) * (a1[0] - b1[0])) / d;
    Some([a1[0] + ua * (a2[0] - a1[0]), a1[1] + ua * (a2[1] - a1[1])])
}

pub(crate) fn line_segment_intersection(
    a1: [f32; 2],
    a2: [f32; 2],
    b1: [f32; 2],
    b2: [f32; 2],
) -> Option<[f32; 2]> {
    let d = (b2[1] - b1[1]) * (a2[0] - a1[0]) - (b2[0] - b1[0]) * (a2[1] - a1[1]);
    if d.abs() < 1e-6 {
        return None;
    }

    let ua = ((b2[0] - b1[0]) * (a1[1] - b1[1]) - (b2[1] - b1[1]) * (a1[0] - b1[0])) / d;
    let ub = ((a2[0] - a1[0]) * (a1[1] - b1[1]) - (a2[1] - a1[1]) * (a1[0] - b1[0])) / d;

    if (0.001..=0.999).contains(&ua) && (0.001..=0.999).contains(&ub) {
        Some([a1[0] + ua * (a2[0] - a1[0]), a1[1] + ua * (a2[1] - a1[1])])
    } else {
        None
    }
}
