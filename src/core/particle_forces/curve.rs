use serde::{Deserialize, Serialize};

/// Control-point curve sampled over normalized lifetime (t = 0 birth .. 1 death).
///
/// Stored as an ordered list of control values; value at t is the linear
/// interpolation between neighbouring control points. An empty curve
/// evaluates to 1.0 (neutral multiplier).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LifeCurve(pub Vec<f32>);

impl Default for LifeCurve {
    fn default() -> Self {
        Self(vec![1.0])
    }
}

impl LifeCurve {
    /// Constant curve.
    pub fn constant(v: f32) -> Self {
        Self(vec![v])
    }

    /// Sample the curve at normalized lifetime progress `t` (clamped 0..1).
    pub fn eval(&self, t: f32) -> f32 {
        let pts = &self.0;
        if pts.is_empty() {
            return 1.0;
        }
        if pts.len() == 1 {
            return pts[0];
        }
        let t = t.clamp(0.0, 1.0);
        let scaled = t * (pts.len() - 1) as f32;
        let i = scaled.floor() as usize;
        let f = scaled - i as f32;
        let a = pts[i.min(pts.len() - 1)];
        let b = pts[(i + 1).min(pts.len() - 1)];
        a + (b - a) * f
    }
}
