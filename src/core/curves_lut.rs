//! Arbitrary Control-Point Spline Curves LUT (After Effects / DaVinci Curves parity).
//!
//! Uses Fritsch-Carlson monotone cubic Hermite interpolation so curves never
//! overshoot or ring between control points, baking a 1024-entry LUT per channel
//! (Master + R + G + B) for fast CPU and GPU 1D texture lookup.

pub const CURVES_LUT_SIZE: usize = 1024;

#[derive(Debug, Clone)]
pub struct CurveChannel {
    /// Sorted `(x, y)` control points in `0.0..=1.0`.
    pub points: Vec<[f32; 2]>,
}

impl Default for CurveChannel {
    fn default() -> Self {
        Self {
            points: vec![[0.0, 0.0], [1.0, 1.0]],
        }
    }
}

impl CurveChannel {
    pub fn from_points(mut points: Vec<[f32; 2]>) -> Self {
        if points.is_empty() {
            return Self::default();
        }
        for p in &mut points {
            p[0] = p[0].clamp(0.0, 1.0);
            p[1] = p[1].clamp(0.0, 1.0);
        }
        points.sort_by(|a, b| a[0].total_cmp(&b[0]));
        points.dedup_by(|a, b| (a[0] - b[0]).abs() < 1e-5);
        Self { points }
    }

    /// Evaluates the monotone cubic Hermite spline at `x` in `0.0..=1.0`.
    pub fn evaluate(&self, x: f32) -> f32 {
        let x = x.clamp(0.0, 1.0);
        let pts = &self.points;
        let n = pts.len();
        if n == 0 {
            return x;
        }
        if n == 1 || x <= pts[0][0] {
            return pts[0][1];
        }
        if x >= pts[n - 1][0] {
            return pts[n - 1][1];
        }

        // Compute secant slopes
        let mut d = vec![0.0_f32; n - 1];
        for i in 0..(n - 1) {
            let dx = (pts[i + 1][0] - pts[i][0]).max(1e-6);
            d[i] = (pts[i + 1][1] - pts[i][1]) / dx;
        }
        // Compute tangents (Fritsch-Carlson monotonicity)
        let mut m = vec![0.0_f32; n];
        m[0] = d[0];
        m[n - 1] = d[n - 2];
        for i in 1..(n - 1) {
            if d[i - 1] * d[i] <= 0.0 {
                m[i] = 0.0;
            } else {
                m[i] = 0.5 * (d[i - 1] + d[i]);
            }
        }
        for i in 0..(n - 1) {
            if d[i].abs() < 1e-6 {
                m[i] = 0.0;
                m[i + 1] = 0.0;
            } else {
                let a = m[i] / d[i];
                let b = m[i + 1] / d[i];
                let sq = a * a + b * b;
                if sq > 9.0 {
                    let tau = 3.0 / sq.sqrt();
                    m[i] = tau * a * d[i];
                    m[i + 1] = tau * b * d[i];
                }
            }
        }

        // Find segment
        let mut seg = 0;
        for i in 0..(n - 1) {
            if x <= pts[i + 1][0] {
                seg = i;
                break;
            }
        }
        let h = (pts[seg + 1][0] - pts[seg][0]).max(1e-6);
        let t = ((x - pts[seg][0]) / h).clamp(0.0, 1.0);
        let t2 = t * t;
        let t3 = t2 * t;
        let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
        let h10 = t3 - 2.0 * t2 + t;
        let h01 = -2.0 * t3 + 3.0 * t2;
        let h11 = t3 - t2;
        (h00 * pts[seg][1] + h10 * h * m[seg] + h01 * pts[seg + 1][1] + h11 * h * m[seg + 1])
            .clamp(0.0, 1.0)
    }

    pub fn bake_lut(&self) -> Vec<f32> {
        (0..CURVES_LUT_SIZE)
            .map(|i| self.evaluate(i as f32 / (CURVES_LUT_SIZE - 1) as f32))
            .collect()
    }
}

#[derive(Debug, Clone, Default)]
pub struct CurvesParams {
    pub master: CurveChannel,
    pub red: CurveChannel,
    pub green: CurveChannel,
    pub blue: CurveChannel,
}

/// Baked `[R, G, B, 1.0]` 1024-entry LUT ready for CPU lookup or GPU `Rgba32Float` 1D/2D texture upload.
pub struct BakedCurvesLut {
    pub rgba_lut: Vec<f32>, // CURVES_LUT_SIZE * 4
}

impl BakedCurvesLut {
    pub fn bake(params: &CurvesParams) -> Self {
        let m = params.master.bake_lut();
        let r = params.red.bake_lut();
        let g = params.green.bake_lut();
        let b = params.blue.bake_lut();
        let sample_m = |v: f32| -> f32 {
            let fx = v.clamp(0.0, 1.0) * (CURVES_LUT_SIZE - 1) as f32;
            let i0 = (fx.floor() as usize).min(CURVES_LUT_SIZE - 1);
            let i1 = (i0 + 1).min(CURVES_LUT_SIZE - 1);
            let t = fx - i0 as f32;
            m[i0] * (1.0 - t) + m[i1] * t
        };

        let mut rgba_lut = Vec::with_capacity(CURVES_LUT_SIZE * 4);
        for i in 0..CURVES_LUT_SIZE {
            rgba_lut.push(sample_m(r[i]));
            rgba_lut.push(sample_m(g[i]));
            rgba_lut.push(sample_m(b[i]));
            rgba_lut.push(1.0);
        }
        Self { rgba_lut }
    }

    #[inline]
    pub fn sample_channel(&self, v: f32, ch: usize) -> f32 {
        let fx = v.clamp(0.0, 1.0) * (CURVES_LUT_SIZE - 1) as f32;
        let i0 = (fx.floor() as usize).min(CURVES_LUT_SIZE - 1);
        let i1 = (i0 + 1).min(CURVES_LUT_SIZE - 1);
        let t = fx - i0 as f32;
        self.rgba_lut[i0 * 4 + ch] * (1.0 - t) + self.rgba_lut[i1 * 4 + ch] * t
    }

    pub fn apply_rgba_f32(&self, rgba: &mut [f32]) {
        for px in rgba.chunks_exact_mut(4) {
            px[0] = self.sample_channel(px[0], 0);
            px[1] = self.sample_channel(px[1], 1);
            px[2] = self.sample_channel(px[2], 2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn s_curve_increases_contrast_monotonically_without_overshoot() {
        let s = CurveChannel::from_points(vec![
            [0.0, 0.0],
            [0.25, 0.15],
            [0.75, 0.85],
            [1.0, 1.0],
        ]);
        let baked = BakedCurvesLut::bake(&CurvesParams {
            master: s,
            ..Default::default()
        });
        let mut px = [0.25_f32, 0.5, 0.75, 1.0];
        baked.apply_rgba_f32(&mut px);
        assert!((px[0] - 0.15).abs() < 1e-3);
        assert!((px[1] - 0.50).abs() < 1e-2);
        assert!((px[2] - 0.85).abs() < 1e-3);
    }
}
