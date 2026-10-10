/// Parameters for [`apply_advanced_vignette`].
#[derive(Debug, Clone, Copy)]
pub struct VignetteParams {
    pub center: [f32; 2],
    pub amount: f32,  // 0..1 darkness intensity
    pub radius: f32,  // 0..1 normalized size of clear center
    pub feather: f32, // 0..1 smoothness of edge transition
    pub tint_color: [f32; 4],
}

impl Default for VignetteParams {
    fn default() -> Self {
        Self {
            center: [0.5, 0.5],
            amount: 0.5,
            radius: 0.6,
            feather: 0.4,
            tint_color: [0.0, 0.0, 0.0, 1.0],
        }
    }
}

/// Oval vignette with tint and smooth feather falloff.
pub fn apply_advanced_vignette(pixels: &mut [u8], width: u32, height: u32, p: &VignetteParams) {
    if width == 0
        || height == 0
        || p.amount <= 0.001
        || pixels.len() < (width as usize) * (height as usize) * 4
    {
        return;
    }
    let cx = p.center[0] * width as f32;
    let cy = p.center[1] * height as f32;
    let max_rx = (width as f32 * 0.5).max(1.0);
    let max_ry = (height as f32 * 0.5).max(1.0);

    let inner_r = p.radius.clamp(0.01, 1.0);
    let outer_r = (inner_r + p.feather.clamp(0.01, 1.0)).max(inner_r + 0.01);

    for y in 0..height {
        for x in 0..width {
            let dx = (x as f32 - cx) / max_rx;
            let dy = (y as f32 - cy) / max_ry;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist > inner_r {
                let t = ((dist - inner_r) / (outer_r - inner_r)).clamp(0.0, 1.0);
                let smooth_t = t * t * (3.0 - 2.0 * t); // smoothstep falloff
                let dark_fac = 1.0 - smooth_t * p.amount.clamp(0.0, 1.0);

                let idx = ((y * width + x) * 4) as usize;
                for c in 0..3 {
                    let tinted = pixels[idx + c] as f32 * dark_fac
                        + p.tint_color[c] * 255.0 * (1.0 - dark_fac);
                    pixels[idx + c] = tinted.clamp(0.0, 255.0) as u8;
                }
            }
        }
    }
}
