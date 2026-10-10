use serde::{Deserialize, Serialize};

// ─── Mask Blend Mode ───────────────────────────────────────────────────────

/// How this mask combines with masks below it on the same layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MaskMode {
    /// Add to the existing alpha (default AE mode)
    #[default]
    Add,
    /// Subtract from the existing alpha
    Subtract,
    /// Only keep the intersection of this and existing alpha
    Intersect,
    /// Exclude the union (XOR-style)
    Lighten,
    /// Darken the existing alpha
    Darken,
    /// Difference with existing alpha
    Difference,
    /// Disable this mask (show full layer)
    None,
}

// ─── Mask Vertex ───────────────────────────────────────────────────────────

/// A single vertex in a Bezier mask path.
/// Positions are in composition pixel coordinates (0,0 = top-left).
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaskVertex {
    /// Anchor point (the vertex position)
    pub position: [f32; 2],
    /// Outgoing tangent handle (relative to position)
    pub tangent_out: [f32; 2],
    /// Incoming tangent handle (relative to position)
    pub tangent_in: [f32; 2],
}

#[allow(dead_code)]
impl MaskVertex {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            position: [x, y],
            tangent_out: [0.0, 0.0],
            tangent_in: [0.0, 0.0],
        }
    }

    pub fn with_tangents(x: f32, y: f32, tx: f32, ty: f32) -> Self {
        Self {
            position: [x, y],
            tangent_out: [tx, ty],
            tangent_in: [-tx, -ty],
        }
    }
}

// ─── Cubic Bezier Helper ───────────────────────────────────────────────────

/// Compute a point on a 2D cubic Bezier curve at parameter t in [0, 1].
pub fn eval_cubic_bezier(
    p0: [f32; 2],
    c0: [f32; 2],
    c1: [f32; 2],
    p1: [f32; 2],
    t: f32,
) -> [f32; 2] {
    let t_safe = t.clamp(0.0, 1.0);
    let u = 1.0 - t_safe;
    let u2 = u * u;
    let u3 = u2 * u;
    let t2 = t_safe * t_safe;
    let t3 = t2 * t_safe;

    let x = u3 * p0[0] + 3.0 * u2 * t_safe * c0[0] + 3.0 * u * t2 * c1[0] + t3 * p1[0];
    let y = u3 * p0[1] + 3.0 * u2 * t_safe * c0[1] + 3.0 * u * t2 * c1[1] + t3 * p1[1];
    [x, y]
}
