// Sampling Helper
#[inline]
fn sample_bilinear(src: &[u8], w: u32, h: u32, fx: f32, fy: f32, out: &mut [u8; 4]) {
    crate::core::effect_utils::sample_bilinear_into(src, w, h, fx, fy, out);
}

/// Parameters for [`apply_cc_lens`].
#[derive(Debug, Clone, Copy)]
pub struct CcLensParams {
    /// Lens centre in normalized coordinates `[0..1, 0..1]`.
    pub center: [f32; 2],
    /// Lens radius in pixels.
    pub size: f32,
    /// Convergence factor (-2.0 = concave/pinch, +2.0 = convex/fisheye, 0.0 = identity).
    pub convergence: f32,
}

impl Default for CcLensParams {
    fn default() -> Self {
        Self {
            center: [0.5, 0.5],
            size: 200.0,
            convergence: 1.0,
        }
    }
}

/// Spherical fisheye/lens distortion simulating CC Lens.
pub fn apply_cc_lens(pixels: &mut [u8], width: u32, height: u32, p: &CcLensParams) {
    if width == 0 || height == 0 || pixels.len() < (width as usize) * (height as usize) * 4 {
        return;
    }
    let src = pixels.to_vec();
    let cx = p.center[0].clamp(0.0, 1.0) * width as f32;
    let cy = p.center[1].clamp(0.0, 1.0) * height as f32;
    let r = p.size.max(1.0);
    let conv = p.convergence;

    let mut out_px = [0u8; 4];
    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist < r && dist > 1e-4 {
                let norm_dist = dist / r;
                // Spherical angle theta
                let theta = norm_dist * std::f32::consts::FRAC_PI_2;
                let factor = if conv >= 0.0 {
                    // Convex / Fisheye
                    let mapped = theta.sin().powf(conv.max(0.01));
                    mapped / norm_dist
                } else {
                    // Concave / Pinch
                    let mapped = 1.0 - (1.0 - norm_dist).powf((-conv).max(0.01));
                    mapped / norm_dist
                };
                let sx = cx + dx * factor;
                let sy = cy + dy * factor;
                sample_bilinear(&src, width, height, sx, sy, &mut out_px);
                let idx = ((y * width + x) * 4) as usize;
                pixels[idx..idx + 4].copy_from_slice(&out_px);
            }
        }
    }
}

/// Parameters for [`apply_directional_smear`].
#[derive(Debug, Clone, Copy)]
pub struct DirectionalSmearParams {
    /// Smear angle in degrees (0 = right, 90 = down).
    pub angle_deg: f32,
    /// Distance of the smear trail in pixels (0..200).
    pub length: f32,
    /// Falloff decay exponent (1.0 = linear, 2.0 = exponential decay).
    pub decay: f32,
    /// Number of sample taps (4..32).
    pub samples: u32,
}

impl Default for DirectionalSmearParams {
    fn default() -> Self {
        Self {
            angle_deg: 0.0,
            length: 20.0,
            decay: 1.2,
            samples: 12,
        }
    }
}

/// Smear trailing blur along an arbitrary angle.
pub fn apply_directional_smear(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    p: &DirectionalSmearParams,
) {
    if width == 0
        || height == 0
        || p.length <= 0.001
        || pixels.len() < (width as usize) * (height as usize) * 4
    {
        return;
    }
    let src = pixels.to_vec();
    let rad = p.angle_deg.to_radians();
    let dx = rad.cos();
    let dy = rad.sin();
    let steps = p.samples.clamp(4, 32);
    let step_dist = p.length / (steps as f32);

    let mut out_px = [0u8; 4];
    for y in 0..height {
        for x in 0..width {
            let mut acc = [0.0f32; 4];
            let mut total_w = 0.0f32;

            for s in 0..steps {
                let dist = s as f32 * step_dist;
                let sx = x as f32 - dx * dist;
                let sy = y as f32 - dy * dist;
                sample_bilinear(&src, width, height, sx, sy, &mut out_px);

                let weight = (1.0 - (s as f32 / steps as f32)).powf(p.decay.max(0.1));
                for c in 0..4 {
                    acc[c] += out_px[c] as f32 * weight;
                }
                total_w += weight;
            }

            let idx = ((y * width + x) * 4) as usize;
            if total_w > 1e-4 {
                for c in 0..4 {
                    pixels[idx + c] = (acc[c] / total_w).round().clamp(0.0, 255.0) as u8;
                }
            }
        }
    }
}
