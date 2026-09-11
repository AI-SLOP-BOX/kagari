//! Layer mask evaluation for the software renderer.
//!
//! Covers [`CpuMaskEntry`] (pre-expanded mask polygons), polygon
//! offsetting, and per-pixel combined coverage with feathering.

use crate::core::mask::{point_in_polygon, MaskMode};

#[derive(Default)]
pub(crate) struct CpuMaskEntry {
    /// Vertices with mask expansion already applied (expanded once per
    /// layer render, never per pixel).
    pub(crate) vertices: Vec<[f32; 2]>,
    pub(crate) feather: f32,
    pub(crate) inverted: bool,
    pub(crate) mode: MaskMode,
}

/// Offset a polygon's vertices along their outward normals by `expansion` pixels.
/// Positive expansion always grows the polygon regardless of vertex winding
/// (a previous version assumed one winding, silently inverting expansion
/// for the common TL,TR,BR,BL rect order).
pub(crate) fn offset_polygon_vertices(vertices: &[[f32; 2]], expansion: f32) -> Vec<[f32; 2]> {
    if vertices.len() < 3 || !expansion.is_finite() || expansion.abs() < 0.01 {
        return vertices.to_vec();
    }
    if vertices
        .iter()
        .any(|point| !point.iter().all(|value| value.is_finite()))
    {
        return vertices.to_vec();
    }
    // Signed area: > 0 means visually clockwise in y-down screen coords,
    // whose outward normal is RIGHT of the edge direction (flip of the
    // default left normal).
    let n = vertices.len();
    let mut area = 0.0f32;
    for i in 0..n {
        let p = vertices[i];
        let q = vertices[(i + 1) % n];
        area += p[0] * q[1] - q[0] * p[1];
    }
    let flip = area > 0.0;
    let mut result = Vec::with_capacity(n);
    for i in 0..n {
        let prev = vertices[(i + n - 1) % n];
        let curr = vertices[i];
        let next = vertices[(i + 1) % n];

        let e1 = [curr[0] - prev[0], curr[1] - prev[1]];
        let e2 = [next[0] - curr[0], next[1] - curr[1]];

        let len1 = (e1[0] * e1[0] + e1[1] * e1[1]).sqrt().max(1e-6);
        let len2 = (e2[0] * e2[0] + e2[1] * e2[1]).sqrt().max(1e-6);
        let (n1, n2) = if flip {
            ([e1[1] / len1, -e1[0] / len1], [e2[1] / len2, -e2[0] / len2])
        } else {
            ([-e1[1] / len1, e1[0] / len1], [-e2[1] / len2, e2[0] / len2])
        };

        let avg_n = [(n1[0] + n2[0]) * 0.5, (n1[1] + n2[1]) * 0.5];
        let avg_len = (avg_n[0] * avg_n[0] + avg_n[1] * avg_n[1]).sqrt().max(1e-6);
        let normal = [avg_n[0] / avg_len, avg_n[1] / avg_len];

        result.push([
            curr[0] + normal[0] * expansion,
            curr[1] + normal[1] * expansion,
        ]);
    }
    result
}

pub(crate) fn distance_to_polygon(px: f32, py: f32, verts: &[[f32; 2]]) -> f32 {
    let mut min_dist = f32::INFINITY;
    let n = verts.len();
    if n < 2 {
        return 0.0;
    }
    for i in 0..n {
        let p1 = verts[i];
        let p2 = verts[(i + 1) % n];

        let dx = p2[0] - p1[0];
        let dy = p2[1] - p1[1];
        let l2 = dx * dx + dy * dy;
        if l2 < 1e-5 {
            let d = ((px - p1[0]).powi(2) + (py - p1[1]).powi(2)).sqrt();
            min_dist = min_dist.min(d);
            continue;
        }
        let t = ((px - p1[0]) * dx + (py - p1[1]) * dy) / l2;
        let t = t.clamp(0.0, 1.0);
        let proj_x = p1[0] + t * dx;
        let proj_y = p1[1] + t * dy;
        let d = ((px - proj_x).powi(2) + (py - proj_y).powi(2)).sqrt();
        min_dist = min_dist.min(d);
    }
    min_dist
}

/// Compute combined mask coverage for a pixel using all enabled masks,
/// matching the GPU path's `combine_mask_shapes` semantics.
pub(crate) fn compute_combined_mask_coverage(px: f32, py: f32, masks: &[CpuMaskEntry]) -> f32 {
    let mut acc = 0.0f32;
    let mut first = true;
    for mask in masks {
        if mask.vertices.len() < 3 {
            continue;
        }
        let inside = point_in_polygon(px, py, &mask.vertices);
        let mut cov = if mask.feather > 0.1 {
            let dist = distance_to_polygon(px, py, &mask.vertices);
            if inside {
                (dist / mask.feather).clamp(0.0, 1.0)
            } else {
                (1.0 - dist / mask.feather).clamp(0.0, 1.0)
            }
        } else if inside {
            1.0
        } else {
            0.0
        };
        if mask.inverted {
            cov = 1.0 - cov;
        }
        if first && mask.mode == MaskMode::Subtract {
            acc = 1.0;
        }
        acc = match mask.mode {
            MaskMode::Add | MaskMode::Lighten => acc + (cov * (1.0 - acc)),
            MaskMode::Subtract => acc * (1.0 - cov),
            MaskMode::Intersect | MaskMode::Darken => acc.min(cov),
            MaskMode::Difference => (acc - cov).abs(),
            MaskMode::None => acc,
        };
        first = false;
    }
    acc
}
