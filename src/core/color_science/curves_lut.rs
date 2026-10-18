/// RGB to HSL color space conversion.
/// Input: R, G, B in [0.0, 1.0].
/// Output: Hue in [0.0, 360.0], Saturation in [0.0, 1.0], Lightness in [0.0, 1.0].
pub fn rgb_to_hsl(r: f32, g: f32, b: f32) -> [f32; 3] {
    let r = if r.is_nan() { 0.0 } else { r.clamp(0.0, 1.0) };
    let g = if g.is_nan() { 0.0 } else { g.clamp(0.0, 1.0) };
    let b = if b.is_nan() { 0.0 } else { b.clamp(0.0, 1.0) };

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = ((max + min) * 0.5).clamp(0.0, 1.0);

    if (max - min).abs() < 1e-5 {
        return [0.0, 0.0, l];
    }

    let d = max - min;
    let denom = (1.0 - (2.0 * l - 1.0).abs()).max(1e-5);
    let s = (d / denom).clamp(0.0, 1.0);

    let mut h = if (max - r).abs() < 1e-5 {
        (g - b) / d + (if g < b { 6.0 } else { 0.0 })
    } else if (max - g).abs() < 1e-5 {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    h = (h * 60.0).rem_euclid(360.0);
    if h.is_nan() {
        h = 0.0;
    }

    [h, s, l]
}

/// HSL back to RGB color space conversion.
/// Input: Hue [0.0, 360.0], Saturation [0.0, 1.0], Lightness [0.0, 1.0].
/// Output: R, G, B in [0.0, 1.0].
pub fn hsl_to_rgb(h: f32, s: f32, l: f32) -> [f32; 3] {
    let h = if h.is_nan() { 0.0 } else { h.rem_euclid(360.0) };
    let s = if s.is_nan() { 0.0 } else { s.clamp(0.0, 1.0) };
    let l = if l.is_nan() { 0.0 } else { l.clamp(0.0, 1.0) };

    if s < 1e-5 {
        return [l, l, l];
    }

    let hue_to_rgb = |p: f32, q: f32, mut t: f32| -> f32 {
        if t < 0.0 {
            t += 1.0;
        }
        if t > 1.0 {
            t -= 1.0;
        }
        if t < 1.0 / 6.0 {
            return p + (q - p) * 6.0 * t;
        }
        if t < 1.0 / 2.0 {
            return q;
        }
        if t < 2.0 / 3.0 {
            return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
        }
        p
    };

    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;

    let h_normalized = h / 360.0;
    let r = hue_to_rgb(p, q, h_normalized + 1.0 / 3.0).clamp(0.0, 1.0);
    let g = hue_to_rgb(p, q, h_normalized).clamp(0.0, 1.0);
    let b = hue_to_rgb(p, q, h_normalized - 1.0 / 3.0).clamp(0.0, 1.0);

    [r, g, b]
}

/// Levels Adjustment (Standard Input/Output clamp and gamma mapping).
///
/// Input parameter ranges:
/// - `in_black`, `in_white` inside [0.0, 1.0]
/// - `gamma` inside [0.1, 9.9]
/// - `out_black`, `out_white` inside [0.0, 1.0]
#[allow(dead_code)]
pub fn apply_levels(
    val: f32,
    in_black: f32,
    in_white: f32,
    gamma: f32,
    out_black: f32,
    out_white: f32,
) -> f32 {
    let span = (in_white - in_black).max(1e-5);
    let normalized = ((val - in_black) / span).clamp(0.0, 1.0);

    let g_safe = gamma.clamp(0.1, 10.0);
    let gamma_corrected = normalized.powf(1.0 / g_safe);

    (out_black + gamma_corrected * (out_white - out_black)).clamp(0.0, 1.0)
}

/// Hue, Saturation, Lightness shifting utility.
///
/// - `hue_shift`: angle shift in degrees [-360.0, 360.0]
/// - `sat_mult`: saturation multiplier >= 0.0 (1.0 = identity)
/// - `light_mult`: lightness multiplier >= 0.0 (1.0 = identity)
pub fn shift_hsl(rgb: [f32; 3], hue_shift: f32, sat_mult: f32, light_mult: f32) -> [f32; 3] {
    let [mut h, mut s, mut l] = rgb_to_hsl(rgb[0], rgb[1], rgb[2]);
    h = (h + hue_shift).rem_euclid(360.0);
    s = (s * sat_mult).clamp(0.0, 1.0);
    l = (l * light_mult).clamp(0.0, 1.0);
    hsl_to_rgb(h, s, l)
}

/// 3D LUT Color Grade Table with Tetrahedral Interpolation from NextVFX Engine.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[allow(dead_code)]
pub struct Lut3D {
    pub size: usize,
    pub data: Vec<f32>, // Flat RGB array of length size * size * size * 3
}

#[allow(dead_code)]
impl Lut3D {
    /// Create a flat identity 3D LUT of dimension `size`.
    pub fn identity(size: usize) -> Self {
        let mut data = Vec::with_capacity(size * size * size * 3);
        let s = (size - 1) as f32;
        for r in 0..size {
            for g in 0..size {
                for b in 0..size {
                    data.push(r as f32 / s);
                    data.push(g as f32 / s);
                    data.push(b as f32 / s);
                }
            }
        }
        Self { size, data }
    }

    /// Tetrahedral 3D LUT interpolation algorithm from NextVFX.
    /// Performs exact tetrahedral simplex subdivision to avoid color banding.
    pub fn apply(&self, r: f32, g: f32, b: f32) -> (f32, f32, f32) {
        if self.size == 0 || self.data.is_empty() {
            return (r, g, b);
        }
        let r_safe = if r.is_nan() { 0.0 } else { r };
        let g_safe = if g.is_nan() { 0.0 } else { g };
        let b_safe = if b.is_nan() { 0.0 } else { b };

        let s = (self.size - 1) as f32;
        let fr = (r_safe * s).clamp(0.0, s - 0.001);
        let fg = (g_safe * s).clamp(0.0, s - 0.001);
        let fb = (b_safe * s).clamp(0.0, s - 0.001);

        let ir = fr as usize;
        let ig = fg as usize;
        let ib = fb as usize;

        let dr = fr - ir as f32;
        let dg = fg - ig as f32;
        let db = fb - ib as f32;

        let get = |x: usize, y: usize, z: usize| {
            let idx = (x * self.size * self.size + y * self.size + z) * 3;
            if idx + 2 < self.data.len() {
                (self.data[idx], self.data[idx + 1], self.data[idx + 2])
            } else {
                (r, g, b)
            }
        };

        let c000 = get(ir, ig, ib);
        let c111 = get(ir + 1, ig + 1, ib + 1);

        if dr > dg {
            if dg > db {
                // R > G > B
                let c100 = get(ir + 1, ig, ib);
                let c110 = get(ir + 1, ig + 1, ib);
                (
                    c000.0
                        + (c100.0 - c000.0) * dr
                        + (c110.0 - c100.0) * dg
                        + (c111.0 - c110.0) * db,
                    c000.1
                        + (c100.1 - c000.1) * dr
                        + (c110.1 - c100.1) * dg
                        + (c111.1 - c110.1) * db,
                    c000.2
                        + (c100.2 - c000.2) * dr
                        + (c110.2 - c100.2) * dg
                        + (c111.2 - c110.2) * db,
                )
            } else if dr > db {
                // R > B > G
                let c100 = get(ir + 1, ig, ib);
                let c101 = get(ir + 1, ig, ib + 1);
                (
                    c000.0
                        + (c100.0 - c000.0) * dr
                        + (c101.0 - c100.0) * db
                        + (c111.0 - c101.0) * dg,
                    c000.1
                        + (c100.1 - c000.1) * dr
                        + (c101.1 - c100.1) * db
                        + (c111.1 - c101.1) * dg,
                    c000.2
                        + (c100.2 - c000.2) * dr
                        + (c101.2 - c100.2) * db
                        + (c111.2 - c101.2) * dg,
                )
            } else {
                // B > R > G
                let c001 = get(ir, ig, ib + 1);
                let c101 = get(ir + 1, ig, ib + 1);
                (
                    c000.0
                        + (c001.0 - c000.0) * db
                        + (c101.0 - c001.0) * dr
                        + (c111.0 - c101.0) * dg,
                    c000.1
                        + (c001.1 - c000.1) * db
                        + (c101.1 - c001.1) * dr
                        + (c111.1 - c101.1) * dg,
                    c000.2
                        + (c001.2 - c000.2) * db
                        + (c101.2 - c001.2) * dr
                        + (c111.2 - c101.2) * dg,
                )
            }
        } else if db > dg {
            // B > G > R
            let c001 = get(ir, ig, ib + 1);
            let c011 = get(ir, ig + 1, ib + 1);
            (
                c000.0 + (c001.0 - c000.0) * db + (c011.0 - c001.0) * dg + (c111.0 - c011.0) * dr,
                c000.1 + (c001.1 - c000.1) * db + (c011.1 - c001.1) * dg + (c111.1 - c011.1) * dr,
                c000.2 + (c001.2 - c000.2) * db + (c011.2 - c001.2) * dg + (c111.2 - c011.2) * dr,
            )
        } else if db > dr {
            // G > B > R
            let c010 = get(ir, ig + 1, ib);
            let c011 = get(ir, ig + 1, ib + 1);
            (
                c000.0 + (c010.0 - c000.0) * dg + (c011.0 - c010.0) * db + (c111.0 - c011.0) * dr,
                c000.1 + (c010.1 - c000.1) * dg + (c011.1 - c010.1) * db + (c111.1 - c011.1) * dr,
                c000.2 + (c010.2 - c000.2) * dg + (c011.2 - c010.2) * db + (c111.2 - c011.2) * dr,
            )
        } else {
            // G > R > B
            let c010 = get(ir, ig + 1, ib);
            let c110 = get(ir + 1, ig + 1, ib);
            (
                c000.0 + (c010.0 - c000.0) * dg + (c110.0 - c010.0) * dr + (c111.0 - c110.0) * db,
                c000.1 + (c010.1 - c000.1) * dg + (c110.1 - c010.1) * dr + (c111.1 - c110.1) * db,
                c000.2 + (c010.2 - c000.2) * dg + (c110.2 - c010.2) * dr + (c111.2 - c110.2) * db,
            )
        }
    }
}
