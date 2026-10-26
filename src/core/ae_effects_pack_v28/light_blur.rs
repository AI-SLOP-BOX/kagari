#[inline]
fn sample_bilinear(src: &[u8], w: u32, h: u32, fx: f32, fy: f32, out: &mut [u8; 4]) {
    crate::core::effect_utils::sample_bilinear_into(src, w, h, fx, fy, out);
}

/// Parameters for [`apply_light_sweep`].
#[derive(Debug, Clone, Copy)]
pub struct LightSweepParams {
    /// Sweep direction in degrees (0 = left→right, 90 = top→bottom).
    pub direction_deg: f32,
    /// Band centre position across the projection axis (0..1).
    pub center: f32,
    /// Band width relative to the projection extent (0.01..1).
    pub width: f32,
    /// Peak highlight strength (0..1).
    pub sweep_intensity: f32,
    /// Extra brightness applied at band edges (0..1).
    pub edge_intensity: f32,
}

impl Default for LightSweepParams {
    fn default() -> Self {
        Self {
            direction_deg: 0.0,
            center: 0.5,
            width: 0.25,
            sweep_intensity: 0.6,
            edge_intensity: 0.3,
        }
    }
}

/// Additive specular highlight band travelling along `direction_deg`.
/// The band profile combines a smooth centre peak with thin bright edges,
/// matching the look of AE's CC Light Sweep.
pub fn apply_light_sweep(pixels: &mut [u8], width: u32, height: u32, p: &LightSweepParams) {
    if width == 0 || height == 0 || pixels.len() < (width as usize) * (height as usize) * 4 {
        return;
    }
    let dir = p.direction_deg.to_radians();
    let (dx, dy) = (dir.cos(), dir.sin());
    // Projection extent along the sweep axis (diagonal coverage).
    let extent = (width as f32 * dx.abs() + height as f32 * dy.abs()).max(1.0);
    let center_px = p.center.clamp(0.0, 1.0) * extent;
    let half_w = (p.width.clamp(0.01, 1.0) * extent * 0.5).max(1.0);

    for y in 0..height {
        for x in 0..width {
            let proj = x as f32 * dx + y as f32 * dy;
            let d = ((proj - center_px) / half_w).abs();
            if d >= 1.0 {
                continue;
            }
            // Centre peak (cosine falloff) + narrow edge spikes near |d|≈1.
            let peak = (1.0 - d * d).max(0.0);
            let edge = ((d - 0.85) / 0.15).clamp(0.0, 1.0);
            let gain = peak * p.sweep_intensity + edge * p.edge_intensity;
            if gain <= 0.001 {
                continue;
            }
            let idx = ((y * width + x) * 4) as usize;
            for c in 0..3 {
                let v = pixels[idx + c] as f32 + 255.0 * gain;
                pixels[idx + c] = v.min(255.0) as u8;
            }
        }
    }
}

/// Zoom blur: averages `samples` radially scaled copies towards `center`.
/// `amount` is the fraction of the distance collapsed at the edges (0..1).
pub fn apply_radial_fast_blur(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    center: [f32; 2],
    amount: f32,
    samples: u32,
) {
    if width == 0 || height == 0 || pixels.len() < (width as usize) * (height as usize) * 4 {
        return;
    }
    let amt = amount.clamp(0.0, 1.0);
    if amt <= 0.001 {
        return;
    }
    let n = samples.clamp(2, 64);
    let temp = pixels.to_vec();
    let (cx, cy) = (center[0], center[1]);

    for y in 0..height {
        for x in 0..width {
            let mut acc = [0.0f32; 4];
            for s in 0..n {
                let t = s as f32 / (n - 1) as f32;
                let scale = 1.0 - amt * t;
                let mut rgba = [0u8; 4];
                sample_bilinear(
                    &temp,
                    width,
                    height,
                    cx + (x as f32 - cx) * scale,
                    cy + (y as f32 - cy) * scale,
                    &mut rgba,
                );
                for c in 0..4 {
                    acc[c] += rgba[c] as f32;
                }
            }
            let idx = ((y * width + x) * 4) as usize;
            for c in 0..4 {
                pixels[idx + c] = (acc[c] / n as f32).round().clamp(0.0, 255.0) as u8;
            }
        }
    }
}
