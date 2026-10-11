use serde::{Deserialize, Serialize};

/// Bezier tangent handle for easing
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BezierHandle {
    /// Incoming tangent X (time offset, normalized 0..1)
    pub in_x: f32,
    /// Incoming tangent Y (value offset, normalized 0..1)
    pub in_y: f32,
    /// Outgoing tangent X (time offset, normalized 0..1)
    pub out_x: f32,
    /// Outgoing tangent Y (value offset, normalized 0..1)
    pub out_y: f32,
}

impl Default for BezierHandle {
    fn default() -> Self {
        Self {
            in_x: 0.0,
            in_y: 0.0,
            out_x: 1.0,
            out_y: 1.0,
        }
    }
}

impl BezierHandle {
    /// Linear interpolation (no easing)
    pub fn linear() -> Self {
        Self {
            in_x: 0.0,
            in_y: 0.0,
            out_x: 1.0,
            out_y: 1.0,
        }
    }
    /// Ease in (slow start)
    pub fn ease_in() -> Self {
        Self {
            in_x: 0.42,
            in_y: 0.0,
            out_x: 1.0,
            out_y: 1.0,
        }
    }
    /// Ease out (slow end)
    pub fn ease_out() -> Self {
        Self {
            in_x: 0.0,
            in_y: 0.0,
            out_x: 0.58,
            out_y: 1.0,
        }
    }
    /// Ease in-out
    pub fn ease_in_out() -> Self {
        Self {
            in_x: 0.42,
            in_y: 0.0,
            out_x: 0.58,
            out_y: 1.0,
        }
    }
    /// Ease out back (overshoot)
    pub fn ease_out_back() -> Self {
        Self {
            in_x: 0.0,
            in_y: 0.0,
            out_x: 0.34,
            out_y: 1.3,
        }
    }
    /// Bounce ease (approximation via cubic bezier)
    pub fn bounce() -> Self {
        Self {
            in_x: 0.0,
            in_y: 0.0,
            out_x: 0.34,
            out_y: 1.2,
        }
    }
}

/// Evaluate cubic bezier at parameter t ∈ \[0,1]
pub fn cubic_bezier(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    let mt3 = mt2 * mt;
    mt3 * p0 + 3.0 * mt2 * t * p1 + 3.0 * mt * t2 * p2 + t3 * p3
}

/// Interpolate between two values using bezier handles
pub fn interpolate_bezier(
    from_val: f32,
    to_val: f32,
    from_time: f32,
    to_time: f32,
    current_time: f32,
    out_handle: &BezierHandle,
    in_handle: &BezierHandle,
) -> f32 {
    // Normalize time to [0,1]
    let t = if (to_time - from_time).abs() < 1e-6 {
        0.0
    } else {
        ((current_time - from_time) / (to_time - from_time)).clamp(0.0, 1.0)
    };

    // Use the outgoing handle of the start key and incoming handle of the end key
    // Map bezier control points: P0=(0,0), P1=(out_handle.out_x, out_handle.out_y),
    //                            P2=(in_handle.in_x, in_handle.in_y), P3=(1,1)
    let t_eased = cubic_bezier(0.0, out_handle.out_y, in_handle.in_y, 1.0, t);

    from_val + (to_val - from_val) * t_eased
}
