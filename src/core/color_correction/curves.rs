/// A user-editable tone curve defined by control points in normalized
/// [0,1]² space. Evaluated with monotone cubic Hermite interpolation
/// (Fritsch–Carlson) so it never overshoots the control data.
#[derive(Debug, Clone, PartialEq)]
pub struct ToneCurve {
    /// Control points sorted by x. Each entry is [x, y] in 0..1.
    pub points: Vec<[f32; 2]>,
}

impl Default for ToneCurve {
    fn default() -> Self {
        Self::linear()
    }
}

impl ToneCurve {
    /// Identity curve.
    pub fn linear() -> Self {
        Self {
            points: vec![[0.0, 0.0], [1.0, 1.0]],
        }
    }

    /// Build from unsorted points; sorts by x and clamps to [0,1].
    pub fn new(mut points: Vec<[f32; 2]>) -> Self {
        points.retain(|p| p[0].is_finite() && p[1].is_finite());
        for p in &mut points {
            p[0] = p[0].clamp(0.0, 1.0);
            p[1] = p[1].clamp(0.0, 1.0);
        }
        points.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap_or(std::cmp::Ordering::Equal));
        // Deduplicate x values keeping the LAST point of each run (most recent edit wins).
        points.reverse();
        points.dedup_by(|a, b| (b[0] - a[0]).abs() < 1e-6);
        points.reverse();
        if points.len() < 2 {
            return Self::linear();
        }
        Self { points }
    }

    /// Evaluate the curve at x (clamped to 0..1).
    pub fn eval(&self, x: f32) -> f32 {
        let pts = &self.points;
        if pts.len() < 2 {
            return x.clamp(0.0, 1.0);
        }
        let x = x.clamp(0.0, 1.0);
        if x <= pts[0][0] {
            return pts[0][1];
        }
        let last = pts.len() - 1;
        if x >= pts[last][0] {
            return pts[last][1];
        }

        // Locate segment via binary search on x.
        let mut lo = 0usize;
        let mut hi = last;
        while hi - lo > 1 {
            let mid = (lo + hi) / 2;
            if pts[mid][0] <= x {
                lo = mid;
            } else {
                hi = mid;
            }
        }

        // Fritsch–Carlson tangents for this neighbourhood.
        let secant = |i: usize| -> f32 {
            let dx = pts[i + 1][0] - pts[i][0];
            if dx.abs() < 1e-9 {
                0.0
            } else {
                (pts[i + 1][1] - pts[i][1]) / dx
            }
        };
        let d_lo = secant(lo.saturating_sub(1));
        let d_mid = secant(lo);
        let d_hi = secant((lo + 1).min(last - 1));

        let m_left = if d_lo * d_mid <= 0.0 {
            0.0
        } else {
            (d_lo + d_mid) * 0.5
        };
        let m_right = if d_mid * d_hi <= 0.0 {
            0.0
        } else {
            (d_mid + d_hi) * 0.5
        };

        // Monotonicity clamp (Fritsch–Carlson 1980): ensures the Hermite
        // interpolant cannot produce non-monotonic sub-intervals.
        let (m_left, m_right) = if d_mid.abs() < 1e-9 {
            (0.0, 0.0)
        } else {
            let alpha = m_left / d_mid;
            let beta = m_right / d_mid;
            let hyp = alpha * alpha + beta * beta;
            if hyp > 9.0 {
                let tau = 3.0 / hyp.sqrt();
                (tau * alpha * d_mid, tau * beta * d_mid)
            } else {
                (m_left, m_right)
            }
        };

        let h = pts[lo + 1][0] - pts[lo][0];
        if h.abs() < 1e-9 {
            return pts[lo][1];
        }
        let t = (x - pts[lo][0]) / h;
        let t2 = t * t;
        let t3 = t2 * t;

        // Hermite basis.
        let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
        let h10 = t3 - 2.0 * t2 + t;
        let h01 = -2.0 * t3 + 3.0 * t2;
        let h11 = t3 - t2;

        let y = h00 * pts[lo][1] + h10 * h * m_left + h01 * pts[lo + 1][1] + h11 * h * m_right;
        y.clamp(0.0, 1.0)
    }

    /// Precompute a 256-entry lookup table mapping u8 in 0..255 to normalized float.
    pub fn build_lut(&self) -> [f32; 256] {
        let mut lut = [0.0f32; 256];
        for (i, slot) in lut.iter_mut().enumerate() {
            *slot = self.eval(i as f32 / 255.0);
        }
        lut
    }
}

/// Per-channel tone curves applied as master∘channel composition.
#[derive(Debug, Clone, Default)]
pub struct ChannelCurves {
    pub master: Option<ToneCurve>,
    pub red: Option<ToneCurve>,
    pub green: Option<ToneCurve>,
    pub blue: Option<ToneCurve>,
}

impl ChannelCurves {
    pub fn is_identity(&self) -> bool {
        self.master.is_none() && self.red.is_none() && self.green.is_none() && self.blue.is_none()
    }
}

/// Apply spline Curves to an RGBA8 buffer.
pub fn apply_curves(pixels: &mut [u8], curves: &ChannelCurves) {
    if curves.is_identity() || pixels.is_empty() {
        return;
    }
    let master_lut = curves.master.as_ref().map(ToneCurve::build_lut);
    let red_lut = curves.red.as_ref().map(ToneCurve::build_lut);
    let green_lut = curves.green.as_ref().map(ToneCurve::build_lut);
    let blue_lut = curves.blue.as_ref().map(ToneCurve::build_lut);

    let through = |lut: &Option<[f32; 256]>, v: u8| -> u8 {
        match lut {
            Some(table) => (table[v as usize].clamp(0.0, 1.0) * 255.0).round() as u8,
            None => v,
        }
    };

    for px in pixels.as_chunks_mut::<4>().0 {
        let r = through(&master_lut, px[0]);
        let g = through(&master_lut, px[1]);
        let b = through(&master_lut, px[2]);
        px[0] = through(&red_lut, r);
        px[1] = through(&green_lut, g);
        px[2] = through(&blue_lut, b);
    }
}
