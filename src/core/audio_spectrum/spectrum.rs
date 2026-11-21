//! Spectrum visualizer geometry and RGBA bar rasterization.

use super::bands::generate_audio_spectrum_bands;
use super::types::{AudioSpectrumOptions, SpectrumBar};

/// Direct RGBA buffer rasterizer for spectrum bars
pub fn render_spectrum(
    buffer: &mut [u8],
    width: u32,
    height: u32,
    bands: &[f32],
    _options: &AudioSpectrumOptions,
    color_a: [u8; 3],
    color_b: [u8; 3],
) {
    if bands.is_empty() || width == 0 || height == 0 {
        return;
    }
    let band_w = (width as f32 / bands.len() as f32).max(1.0);
    let cy = height as f32 * 0.5;

    for (b_idx, &mag) in bands.iter().enumerate() {
        let bx = b_idx as f32 * band_w;
        let safe_mag = if mag.is_finite() { mag.max(0.0) } else { 0.0 };
        let bh = (safe_mag * (height as f32 * 0.45))
            .min(height as f32)
            .max(1.0);
        let top = (cy - bh).clamp(0.0, height as f32 - 1.0) as u32;
        let bot = (cy + bh).clamp(0.0, height as f32 - 1.0) as u32;

        let t = b_idx as f32 / bands.len() as f32;
        let r = ((1.0 - t) * color_a[0] as f32 + t * color_b[0] as f32) as u8;
        let g = ((1.0 - t) * color_a[1] as f32 + t * color_b[1] as f32) as u8;
        let b = ((1.0 - t) * color_a[2] as f32 + t * color_b[2] as f32) as u8;

        for y in top..=bot {
            for x in (bx as u32)..((bx + band_w * 0.8) as u32).min(width) {
                let Some(idx) = (y as usize)
                    .checked_mul(width as usize)
                    .and_then(|offset| offset.checked_add(x as usize))
                    .and_then(|offset| offset.checked_mul(4))
                else {
                    continue;
                };
                if idx + 3 < buffer.len() {
                    buffer[idx] = r;
                    buffer[idx + 1] = g;
                    buffer[idx + 2] = b;
                    buffer[idx + 3] = 255;
                }
            }
        }
    }
}

/// Generates visualizer geometry points/bars for an audio frame chunk.
pub fn generate_audio_spectrum(
    samples: &[f32],
    options: &AudioSpectrumOptions,
) -> Vec<SpectrumBar> {
    let bands = options.frequency_bands.clamp(4, 512);
    let magnitudes = generate_audio_spectrum_bands(samples, 44100, options);

    let mut bars = Vec::with_capacity(bands);

    if options.is_polar {
        let center = options.start_point;
        let base_r = options.polar_radius;

        for (i, &mag) in magnitudes.iter().enumerate() {
            let theta = (i as f32 / bands as f32) * std::f32::consts::TAU;
            let (sin_t, cos_t) = theta.sin_cos();

            let r_inner = base_r;
            let r_outer = base_r + mag * options.max_height;

            let p_start = [center[0] + cos_t * r_inner, center[1] + sin_t * r_inner];
            let p_end = [center[0] + cos_t * r_outer, center[1] + sin_t * r_outer];

            bars.push(SpectrumBar {
                start: p_start,
                end: p_end,
                magnitude: mag,
            });
        }
    } else {
        let p0 = options.start_point;
        let p1 = options.end_point;
        let dx = p1[0] - p0[0];
        let dy = p1[1] - p0[1];
        let len = (dx * dx + dy * dy).sqrt().max(1.0);
        let normal = [-dy / len, dx / len]; // Perpendicular unit normal

        for (i, &mag) in magnitudes.iter().enumerate() {
            let t = i as f32 / (bands.saturating_sub(1)).max(1) as f32;
            let base_x = p0[0] + dx * t;
            let base_y = p0[1] + dy * t;
            let h = mag * options.max_height;

            let p_start = [base_x - normal[0] * h * 0.5, base_y - normal[1] * h * 0.5];
            let p_end = [base_x + normal[0] * h * 0.5, base_y + normal[1] * h * 0.5];

            bars.push(SpectrumBar {
                start: p_start,
                end: p_end,
                magnitude: mag,
            });
        }
    }

    bars
}
