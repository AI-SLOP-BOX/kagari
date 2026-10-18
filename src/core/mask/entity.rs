use super::path::MaskPath;
use super::types::MaskMode;
use crate::core::property::Animatable;
use serde::{Deserialize, Serialize};

// ─── Mask ──────────────────────────────────────────────────────────────────

/// A single named mask attached to a layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mask {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub mode: MaskMode,
    pub path: MaskPath,
    /// Feather/blur radius in pixels applied to the mask edge
    pub feather: Animatable<f32>,
    /// Mask opacity (0–100)
    pub opacity: Animatable<f32>,
    /// Positive = expand, negative = contract (pixels)
    pub expansion: Animatable<f32>,
    /// Whether the mask is inverted
    pub inverted: bool,
    /// Wiggle Paths organic deformation (AE parity)
    #[serde(default)]
    pub wiggle: Option<crate::core::wiggle_paths::WigglePathsOptions>,
}

impl Mask {
    pub fn new_rect(id: String, name: String, x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            id,
            name,
            enabled: true,
            mode: MaskMode::Add,
            path: MaskPath::new_rect(x, y, w, h),
            feather: Animatable::new_constant(0.0),
            opacity: Animatable::new_constant(100.0),
            expansion: Animatable::new_constant(0.0),
            inverted: false,
            wiggle: None,
        }
    }

    pub fn new_ellipse(id: String, name: String, cx: f32, cy: f32, rx: f32, ry: f32) -> Self {
        Self {
            id,
            name,
            enabled: true,
            mode: MaskMode::Add,
            path: MaskPath::new_ellipse(cx, cy, rx, ry),
            feather: Animatable::new_constant(0.0),
            opacity: Animatable::new_constant(100.0),
            expansion: Animatable::new_constant(0.0),
            inverted: false,
            wiggle: None,
        }
    }

    pub fn new_closed(id: String, name: String, verts: Vec<[f32; 2]>) -> Self {
        Self {
            id,
            name,
            enabled: true,
            mode: MaskMode::Add,
            path: MaskPath::new_closed(verts),
            feather: Animatable::new_constant(0.0),
            opacity: Animatable::new_constant(100.0),
            expansion: Animatable::new_constant(0.0),
            inverted: false,
            wiggle: None,
        }
    }
}

// ─── Point-in-Polygon Test (CPU Alpha Cutout) ─────────────────────────────

/// Ray-casting point-in-polygon test.
/// Returns true if (px, py) is inside the polygon defined by `verts`.
#[allow(dead_code)]
pub fn point_in_polygon(px: f32, py: f32, verts: &[[f32; 2]]) -> bool {
    let n = verts.len();
    if n < 3 {
        return false;
    }

    // Check if point is directly on any polygon boundary edge within tolerance
    for i in 0..n {
        let j = (i + 1) % n;
        let x1 = verts[i][0];
        let y1 = verts[i][1];
        let x2 = verts[j][0];
        let y2 = verts[j][1];
        let dx = x2 - x1;
        let dy = y2 - y1;
        let len_sq = dx * dx + dy * dy;
        if len_sq > 1e-6 {
            let t = (((px - x1) * dx + (py - y1) * dy) / len_sq).clamp(0.0, 1.0);
            let proj_x = x1 + t * dx;
            let proj_y = y1 + t * dy;
            let dist_sq = (px - proj_x).powi(2) + (py - proj_y).powi(2);
            if dist_sq < 1e-4 {
                return true;
            }
        }
    }

    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let xi = verts[i][0];
        let yi = verts[i][1];
        let xj = verts[j][0];
        let yj = verts[j][1];
        if ((yi > py) != (yj > py)) && (px < (xj - xi) * (py - yi) / (yj - yi) + xi) {
            inside = !inside;
        }
        j = i;
    }
    inside
}
