//! Auto Film Grain Analysis & Re-synthesis (Nuke F_ReGrain / Sapphire Grain parity).

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FilmGrainProfile {
    pub sigma_r: f32,
    pub sigma_g: f32,
    pub sigma_b: f32,
    pub shadow_weight: f32,
    pub highlight_weight: f32,
}

impl Default for FilmGrainProfile {
    fn default() -> Self {
        Self {
            sigma_r: 0.015,
            sigma_g: 0.012,
            sigma_b: 0.018,
            shadow_weight: 0.8,
            highlight_weight: 0.45,
        }
    }
}

/// Automatically measures per-channel high-frequency grain noise (`sigma_r`, `sigma_g`, `sigma_b`)
/// using a 3x3 Laplacian estimator over low-edge regions of `rgba` (`RGBA32F`).
pub fn analyze_film_grain(rgba: &[f32], width: u32, height: u32) -> Option<FilmGrainProfile> {
    if width < 3 || height < 3 || rgba.len() != (width as usize) * (height as usize) * 4 {
        return None;
    }
    let w = width as usize;
    let h = height as usize;
    let mut sum_sq = [0.0_f64; 3];
    let mut count = 0u64;

    // 3x3 discrete Laplacian normalized by sqrt(20) for i.i.d. Gaussian noise std-dev estimation
    for y in 1..(h - 1) {
        for x in 1..(w - 1) {
            let c = (y * w + x) * 4;
            let l = ((y) * w + (x - 1)) * 4;
            let r = ((y) * w + (x + 1)) * 4;
            let u = ((y - 1) * w + x) * 4;
            let d = ((y + 1) * w + x) * 4;
            for ch in 0..3 {
                let lap = 4.0 * rgba[c + ch] - rgba[l + ch] - rgba[r + ch] - rgba[u + ch] - rgba[d + ch];
                // Reject strong structural edges
                if lap.abs() < 0.25 {
                    sum_sq[ch] += (lap as f64) * (lap as f64);
                }
            }
            count += 1;
        }
    }
    if count == 0 {
        return Some(FilmGrainProfile::default());
    }
    let norm = 20.0_f64.sqrt();
    Some(FilmGrainProfile {
        sigma_r: ((sum_sq[0] / count as f64).sqrt() / norm) as f32,
        sigma_g: ((sum_sq[1] / count as f64).sqrt() / norm) as f32,
        sigma_b: ((sum_sq[2] / count as f64).sqrt() / norm) as f32,
        shadow_weight: 0.8,
        highlight_weight: 0.45,
    })
}

#[inline]
fn hash_gauss(x: u32, y: u32, frame: u32, ch: u32, seed: u64) -> f32 {
    let mut h = seed
        ^ (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9)
        ^ (frame as u64).wrapping_mul(0x94D0_49BB_1331_11EB)
        ^ (ch as u64).wrapping_mul(0x517C_C1B7_2722_0A95);
    h = (h ^ (h >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    let u1 = ((h & 0xFFFFFF) as f32 / 16777216.0).clamp(1e-5, 0.9999);
    let u2 = (((h >> 24) & 0xFFFFFF) as f32 / 16777216.0).clamp(0.0, 1.0);
    (-2.0 * u1.ln()).sqrt() * (2.0 * std::f32::consts::PI * u2).cos()
}

/// Applies film grain matching `profile` to `rgba` (`RGBA32F`).
pub fn synthesize_film_grain(
    rgba: &mut [f32],
    width: u32,
    height: u32,
    profile: &FilmGrainProfile,
    frame: u32,
    seed: u64,
) {
    let count = (width as usize).saturating_mul(height as usize);
    if rgba.len() != count * 4 {
        return;
    }
    for y in 0..height {
        for x in 0..width {
            let idx = (y as usize * width as usize + x as usize) * 4;
            let a = rgba[idx + 3].clamp(0.0, 1.0);
            if a <= 1e-4 {
                continue;
            }
            let luma = (0.2126 * rgba[idx] + 0.7152 * rgba[idx + 1] + 0.0722 * rgba[idx + 2])
                .clamp(0.0, 1.0);
            // Midtone peak response
            let resp = profile.shadow_weight * (1.0 - luma) + profile.highlight_weight * luma;
            let bell = (4.0 * luma * (1.0 - luma)).max(0.25);
            let w = resp * bell * a;
            rgba[idx] += hash_gauss(x, y, frame, 0, seed) * profile.sigma_r * w;
            rgba[idx + 1] += hash_gauss(x, y, frame, 1, seed) * profile.sigma_g * w;
            rgba[idx + 2] += hash_gauss(x, y, frame, 2, seed) * profile.sigma_b * w;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyze_and_resynthesize_film_grain_estimates_nonzero_sigma() {
        let w = 32u32;
        let h = 32u32;
        let mut plate = [0.5_f32, 0.5, 0.5, 1.0].repeat((w * h) as usize);
        let true_prof = FilmGrainProfile {
            sigma_r: 0.03,
            sigma_g: 0.02,
            sigma_b: 0.04,
            shadow_weight: 1.0,
            highlight_weight: 1.0,
        };
        synthesize_film_grain(&mut plate, w, h, &true_prof, 1, 77);
        let est = analyze_film_grain(&plate, w, h).expect("analyze");
        assert!(est.sigma_b > est.sigma_g && est.sigma_g > 0.005);
    }
}
