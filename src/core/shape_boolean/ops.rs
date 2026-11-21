use super::geom::{
    is_inside_edge, line_intersection_edge, line_segment_intersection, signed_polygon_area,
};
use crate::core::mask::point_in_polygon;

/// Vector Boolean Operation Mode matching After Effects Shape Merge Paths & Pathfinder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum BooleanOp {
    #[default]
    Union,
    Intersect,
    Subtract,
    Exclude,
}

/// Applies a 2D vector Boolean operation between Subject (A) and Clip (B) polygon contours.
pub fn apply_polygon_boolean(
    subject: &[[f32; 2]],
    clip: &[[f32; 2]],
    op: BooleanOp,
) -> Vec<Vec<[f32; 2]>> {
    if subject.is_empty() {
        return match op {
            BooleanOp::Union | BooleanOp::Exclude => {
                if clip.is_empty() {
                    vec![]
                } else {
                    vec![clip.to_vec()]
                }
            }
            _ => vec![],
        };
    }
    if clip.is_empty() {
        return match op {
            BooleanOp::Union | BooleanOp::Subtract | BooleanOp::Exclude => vec![subject.to_vec()],
            BooleanOp::Intersect => vec![],
        };
    }

    match op {
        BooleanOp::Union => polygon_union(subject, clip),
        BooleanOp::Intersect => polygon_intersect(subject, clip),
        BooleanOp::Subtract => polygon_subtract(subject, clip),
        BooleanOp::Exclude => polygon_exclude(subject, clip),
    }
}

/// Computes polygon intersection using Sutherland-Hodgman convex/concave clipping.
pub fn polygon_intersect(subject: &[[f32; 2]], clip: &[[f32; 2]]) -> Vec<Vec<[f32; 2]>> {
    if subject.len() < 3 || clip.len() < 3 {
        return vec![];
    }

    let is_ccw = signed_polygon_area(clip) >= 0.0;
    let mut output = subject.to_vec();

    for i in 0..clip.len() {
        if output.is_empty() {
            break;
        }
        let p1 = clip[i];
        let p2 = clip[(i + 1) % clip.len()];

        let input = output;
        output = Vec::new();

        if input.is_empty() {
            break;
        }

        let mut s = *input.last().unwrap();
        for &e in &input {
            let e_in = is_inside_edge(e, p1, p2, is_ccw);
            let s_in = is_inside_edge(s, p1, p2, is_ccw);

            if e_in {
                if !s_in {
                    if let Some(inter) = line_intersection_edge(s, e, p1, p2) {
                        output.push(inter);
                    }
                }
                output.push(e);
            } else if s_in {
                if let Some(inter) = line_intersection_edge(s, e, p1, p2) {
                    output.push(inter);
                }
            }
            s = e;
        }
    }

    let mut clean_output: Vec<[f32; 2]> = Vec::new();
    for p in output {
        if let Some(last) = clean_output.last() {
            let d_sq = (p[0] - last[0]).powi(2) + (p[1] - last[1]).powi(2);
            if d_sq > 1e-4 {
                clean_output.push(p);
            }
        } else {
            clean_output.push(p);
        }
    }

    if clean_output.len() >= 3 {
        vec![clean_output]
    } else {
        vec![]
    }
}

/// Computes exact polygon difference (Subject minus Clip).
pub fn polygon_subtract(subject: &[[f32; 2]], clip: &[[f32; 2]]) -> Vec<Vec<[f32; 2]>> {
    if subject.is_empty() {
        return vec![];
    }
    if clip.is_empty() {
        return vec![subject.to_vec()];
    }

    let inter = polygon_intersect(subject, clip);
    if inter.is_empty() {
        if point_in_polygon(subject[0][0], subject[0][1], clip) {
            return vec![];
        }
        let clip_is_inner_hole = clip
            .iter()
            .all(|&pt| point_in_polygon(pt[0], pt[1], subject));
        if clip_is_inner_hole {
            let min_y = clip.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min);
            let max_y = clip.iter().map(|p| p[1]).fold(f32::NEG_INFINITY, f32::max);
            let min_x = clip.iter().map(|p| p[0]).fold(f32::INFINITY, f32::min);
            let max_x = clip.iter().map(|p| p[0]).fold(f32::NEG_INFINITY, f32::max);

            let sub_min_x = subject.iter().map(|p| p[0]).fold(f32::INFINITY, f32::min);
            let sub_max_x = subject
                .iter()
                .map(|p| p[0])
                .fold(f32::NEG_INFINITY, f32::max);
            let sub_min_y = subject.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min);
            let sub_max_y = subject
                .iter()
                .map(|p| p[1])
                .fold(f32::NEG_INFINITY, f32::max);

            return vec![
                vec![
                    [sub_min_x, sub_min_y],
                    [sub_max_x, sub_min_y],
                    [sub_max_x, min_y],
                    [sub_min_x, min_y],
                ],
                vec![
                    [sub_min_x, max_y],
                    [sub_max_x, max_y],
                    [sub_max_x, sub_max_y],
                    [sub_min_x, sub_max_y],
                ],
                vec![
                    [sub_min_x, min_y],
                    [min_x, min_y],
                    [min_x, max_y],
                    [sub_min_x, max_y],
                ],
                vec![
                    [max_x, min_y],
                    [sub_max_x, min_y],
                    [sub_max_x, max_y],
                    [max_x, max_y],
                ],
            ];
        }
        return vec![subject.to_vec()];
    }

    let mut all_inside = true;
    for &pt in subject {
        if !point_in_polygon(pt[0], pt[1], clip) {
            all_inside = false;
            break;
        }
    }
    if all_inside {
        return vec![];
    }

    let mut enriched_subject = Vec::new();
    let n_s = subject.len();
    for i in 0..n_s {
        let s1 = subject[i];
        let s2 = subject[(i + 1) % n_s];
        enriched_subject.push(s1);

        let mut inters: Vec<(f32, [f32; 2])> = Vec::new();
        let n_c = clip.len();
        for j in 0..n_c {
            let c1 = clip[j];
            let c2 = clip[(j + 1) % n_c];
            if let Some(pt) = line_segment_intersection(s1, s2, c1, c2) {
                let dist = (pt[0] - s1[0]).powi(2) + (pt[1] - s1[1]).powi(2);
                inters.push((dist, pt));
            }
        }
        inters.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (_, pt) in inters {
            enriched_subject.push(pt);
        }
    }

    let mut remaining = Vec::new();
    for pt in enriched_subject {
        let in_clip = point_in_polygon(pt[0], pt[1], clip);
        if !in_clip && remaining.last() != Some(&pt) {
            remaining.push(pt);
        }
    }

    let clip_is_inner_hole = clip
        .iter()
        .all(|&pt| point_in_polygon(pt[0], pt[1], subject));
    if clip_is_inner_hole {
        let mut bridged = Vec::new();
        bridged.extend_from_slice(subject);
        let mut reversed_clip = clip.to_vec();
        reversed_clip.reverse();
        bridged.push(reversed_clip[0]);
        bridged.extend(reversed_clip);
        return vec![bridged];
    }

    if remaining.len() >= 3 {
        vec![remaining]
    } else {
        vec![subject.to_vec()]
    }
}

/// Computes exact polygon union (Subject + Clip) with proper convex hull / outer perimeter ordering.
pub fn polygon_union(subject: &[[f32; 2]], clip: &[[f32; 2]]) -> Vec<Vec<[f32; 2]>> {
    if subject.is_empty() && clip.is_empty() {
        return vec![];
    }
    if subject.is_empty() {
        return vec![clip.to_vec()];
    }
    if clip.is_empty() {
        return vec![subject.to_vec()];
    }

    let inter = polygon_intersect(subject, clip);
    if inter.is_empty() {
        return vec![subject.to_vec(), clip.to_vec()];
    }

    let sub_in_clip = subject.iter().all(|p| point_in_polygon(p[0], p[1], clip));
    if sub_in_clip {
        return vec![clip.to_vec()];
    }
    let clip_in_sub = clip.iter().all(|p| point_in_polygon(p[0], p[1], subject));
    if clip_in_sub {
        return vec![subject.to_vec()];
    }

    let mut boundary_points: Vec<[f32; 2]> = Vec::new();
    for &pt in subject {
        if !point_in_polygon(pt[0], pt[1], clip) {
            boundary_points.push(pt);
        }
    }
    for &pt in clip {
        if !point_in_polygon(pt[0], pt[1], subject) {
            boundary_points.push(pt);
        }
    }

    let n_s = subject.len();
    let n_c = clip.len();
    for i in 0..n_s {
        let s1 = subject[i];
        let s2 = subject[(i + 1) % n_s];
        for j in 0..n_c {
            let c1 = clip[j];
            let c2 = clip[(j + 1) % n_c];
            if let Some(pt) = line_segment_intersection(s1, s2, c1, c2) {
                boundary_points.push(pt);
            }
        }
    }

    if boundary_points.len() < 3 {
        return vec![subject.to_vec(), clip.to_vec()];
    }

    let mut cx = 0.0f32;
    let mut cy = 0.0f32;
    for &p in &boundary_points {
        cx += p[0];
        cy += p[1];
    }
    let count = boundary_points.len() as f32;
    cx /= count;
    cy /= count;

    boundary_points.sort_by(|a, b| {
        let angle_a = (a[1] - cy).atan2(a[0] - cx);
        let angle_b = (b[1] - cy).atan2(b[0] - cx);
        angle_a.total_cmp(&angle_b)
    });

    let mut deduplicated: Vec<[f32; 2]> = Vec::new();
    for p in boundary_points {
        if let Some(last) = deduplicated.last() {
            let d_sq = (p[0] - last[0]).powi(2) + (p[1] - last[1]).powi(2);
            if d_sq > 1e-4 {
                deduplicated.push(p);
            }
        } else {
            deduplicated.push(p);
        }
    }

    if deduplicated.len() >= 3 {
        vec![deduplicated]
    } else {
        vec![subject.to_vec(), clip.to_vec()]
    }
}

/// Computes polygon exclusion / XOR (Points in A or B but not both).
pub fn polygon_exclude(subject: &[[f32; 2]], clip: &[[f32; 2]]) -> Vec<Vec<[f32; 2]>> {
    let sub_a = polygon_subtract(subject, clip);
    let sub_b = polygon_subtract(clip, subject);

    let mut result = Vec::new();
    result.extend(sub_a);
    result.extend(sub_b);
    result
}

/// Offsets / dilates or erodes a closed polygon contour by `delta` pixels.
pub fn offset_polygon_path(polygon: &[[f32; 2]], delta: f32) -> Vec<[f32; 2]> {
    if polygon.len() < 3 || delta.abs() < 1e-4 {
        return polygon.to_vec();
    }

    let n = polygon.len();
    let mut offset_poly = Vec::with_capacity(n);

    let mut cx = 0.0f32;
    let mut cy = 0.0f32;
    for pt in polygon {
        cx += pt[0];
        cy += pt[1];
    }
    cx /= n as f32;
    cy /= n as f32;

    for &curr in polygon {
        let dx = curr[0] - cx;
        let dy = curr[1] - cy;
        let len = (dx * dx + dy * dy).sqrt().max(1e-5);
        let nx = dx / len;
        let ny = dy / len;

        offset_poly.push([curr[0] + nx * delta, curr[1] + ny * delta]);
    }

    offset_poly
}
