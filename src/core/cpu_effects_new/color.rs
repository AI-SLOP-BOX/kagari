use crate::core::effect_utils;
use crate::core::software_renderer::rgba_buffer_size;
use rayon::prelude::*;

/// Apply levels adjustment (tone remapping).
#[allow(clippy::too_many_arguments)]
pub fn apply_levels(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    input_black: f32,
    input_white: f32,
    gamma: f32,
    output_black: f32,
    output_white: f32,
) {
    let Some(expected) = rgba_buffer_size(width, height) else {
        return;
    };
    if pixels.len() < expected {
        return;
    }
    let range = (input_white - input_black).max(0.001);
    let inv_gamma = if gamma > 0.0 { 1.0 / gamma } else { 1.0 };

    let mut table = [0u8; 256];
    for (i, entry) in table.iter_mut().enumerate() {
        let v = i as f32 / 255.0;
        let normalized = ((v - input_black) / range).clamp(0.0, 1.0);
        let corrected = normalized.powf(inv_gamma);
        let out = output_black + (output_white - output_black) * corrected;
        *entry = (out.clamp(0.0, 1.0) * 255.0) as u8;
    }

    pixels.par_chunks_mut(4).for_each(|chunk| {
        if chunk.len() < 4 {
            return;
        }
        chunk[0] = table[chunk[0] as usize];
        chunk[1] = table[chunk[1] as usize];
        chunk[2] = table[chunk[2] as usize];
    });
}

/// Apply HSL (Hue/Saturation/Lightness) adjustment.
pub fn apply_hue_saturation(
    pixels: &mut [u8],
    _width: u32,
    _height: u32,
    hue_shift: f32,
    saturation: f32,
    lightness: f32,
) {
    let shift_norm = hue_shift / 360.0;

    pixels.par_chunks_mut(4).for_each(|chunk| {
        if chunk.len() < 4 {
            return;
        }
        let r = chunk[0] as f32 / 255.0;
        let g = chunk[1] as f32 / 255.0;
        let b = chunk[2] as f32 / 255.0;

        let (h, s, l) = effect_utils::rgb_to_hsl(r, g, b);
        let new_h = (h + shift_norm).fract();
        let new_h = if new_h < 0.0 { new_h + 1.0 } else { new_h };
        let new_s = (s * (1.0 + saturation)).clamp(0.0, 1.0);
        let new_l = (l + lightness).clamp(0.0, 1.0);

        let (rr, gg, bb) = effect_utils::hsl_to_rgb(new_h, new_s, new_l);
        chunk[0] = (rr.clamp(0.0, 1.0) * 255.0) as u8;
        chunk[1] = (gg.clamp(0.0, 1.0) * 255.0) as u8;
        chunk[2] = (bb.clamp(0.0, 1.0) * 255.0) as u8;
    });
}

/// Apply 3D LUT color grading.
pub fn apply_color_grade_lut(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    lut_data: &[[f32; 3]],
    lut_size: usize,
    intensity: f32,
) {
    let Some(expected) = rgba_buffer_size(width, height) else {
        return;
    };
    if pixels.len() < expected || lut_size < 2 || lut_data.len() < lut_size * lut_size * lut_size {
        return;
    }
    let ls = lut_size as f32 - 1.0;

    pixels.par_chunks_mut(4).for_each(|chunk| {
        if chunk.len() < 4 {
            return;
        }
        let r = chunk[0] as f32 / 255.0;
        let g = chunk[1] as f32 / 255.0;
        let b = chunk[2] as f32 / 255.0;

        let fx = r * ls;
        let fy = g * ls;
        let fz = b * ls;

        let x0 = fx.floor() as usize;
        let y0 = fy.floor() as usize;
        let z0 = fz.floor() as usize;
        let x1 = (x0 + 1).min(lut_size - 1);
        let y1 = (y0 + 1).min(lut_size - 1);
        let z1 = (z0 + 1).min(lut_size - 1);
        let dx = fx - fx.floor();
        let dy = fy - fy.floor();
        let dz = fz - fz.floor();

        let idx3d = |xi: usize, yi: usize, zi: usize| -> usize {
            zi * lut_size * lut_size + yi * lut_size + xi
        };

        let fetch_lut =
            |xi: usize, yi: usize, zi: usize| -> [f32; 3] { lut_data[idx3d(xi, yi, zi)] };

        let c000 = fetch_lut(x0, y0, z0);
        let c100 = fetch_lut(x1, y0, z0);
        let c010 = fetch_lut(x0, y1, z0);
        let c110 = fetch_lut(x1, y1, z0);
        let c001 = fetch_lut(x0, y0, z1);
        let c101 = fetch_lut(x1, y0, z1);
        let c011 = fetch_lut(x0, y1, z1);
        let c111 = fetch_lut(x1, y1, z1);

        let mut result = [0.0f32; 3];
        for ch in 0..3 {
            let v = c000[ch] * (1.0 - dx) * (1.0 - dy) * (1.0 - dz)
                + c100[ch] * dx * (1.0 - dy) * (1.0 - dz)
                + c010[ch] * (1.0 - dx) * dy * (1.0 - dz)
                + c110[ch] * dx * dy * (1.0 - dz)
                + c001[ch] * (1.0 - dx) * (1.0 - dy) * dz
                + c101[ch] * dx * (1.0 - dy) * dz
                + c011[ch] * (1.0 - dx) * dy * dz
                + c111[ch] * dx * dy * dz;
            result[ch] = v;
        }

        for ch in 0..3 {
            let orig = chunk[ch] as f32 / 255.0;
            let graded = result[ch];
            let blended = orig + (graded - orig) * intensity;
            chunk[ch] = (blended.clamp(0.0, 1.0) * 255.0) as u8;
        }
    });
}

/// Apply color space conversion.
pub fn apply_color_space_convert(pixels: &mut [u8], _width: u32, _height: u32, mode: u32) {
    match mode {
        0 => {
            pixels.par_chunks_mut(4).for_each(|chunk| {
                if chunk.len() < 4 {
                    return;
                }
                for ch in chunk.iter_mut().take(3) {
                    let v = *ch as f32 / 255.0;
                    let linear = if v <= 0.04045 {
                        v / 12.92
                    } else {
                        ((v + 0.055) / 1.055).powf(2.2)
                    };
                    *ch = (linear.clamp(0.0, 1.0) * 255.0) as u8;
                }
            });
        }
        1 => {
            pixels.par_chunks_mut(4).for_each(|chunk| {
                if chunk.len() < 4 {
                    return;
                }
                for ch in chunk.iter_mut().take(3) {
                    let v = *ch as f32 / 255.0;
                    let srgb = if v <= 0.0031308 {
                        v * 12.92
                    } else {
                        1.055 * v.powf(1.0 / 2.2) - 0.055
                    };
                    *ch = (srgb.clamp(0.0, 1.0) * 255.0) as u8;
                }
            });
        }
        2 => {
            pixels.par_chunks_mut(4).for_each(|chunk| {
                if chunk.len() < 4 {
                    return;
                }
                let r = chunk[0] as f32 / 255.0;
                let g = chunk[1] as f32 / 255.0;
                let b = chunk[2] as f32 / 255.0;
                let (_h, _s, l) = effect_utils::rgb_to_hsl(r, g, b);
                let v = (l * 255.0) as u8;
                chunk[0] = v;
                chunk[1] = v;
                chunk[2] = v;
            });
        }
        _ => {}
    }
}

/// Apply per-channel bezier spline tone curves.
pub fn apply_curves(pixels: &mut [u8], _width: u32, _height: u32, channel: f32) {
    let ch = channel as u32;
    let ch_idx = ch as usize;

    let points = [0.0f32, 0.2, 0.5, 0.8, 1.0];

    let mut lut = [0u8; 256];
    for (i, entry) in lut.iter_mut().enumerate() {
        let t = i as f32 / 255.0;
        let val = catmull_rom_spline(&points, t);
        *entry = (val.clamp(0.0, 1.0) * 255.0) as u8;
    }

    for px in pixels.as_chunks_mut::<4>().0 {
        if ch == 0 {
            px[0] = lut[px[0] as usize];
            px[1] = lut[px[1] as usize];
            px[2] = lut[px[2] as usize];
        } else if ch_idx <= 3 {
            px[ch_idx - 1] = lut[px[ch_idx - 1] as usize];
        }
    }
}

fn catmull_rom_spline(points: &[f32], t: f32) -> f32 {
    let n = points.len() - 1;
    let x = t * n as f32;
    let i = (x.floor() as usize).min(n - 1);
    let frac = x - i as f32;
    let p0 = points[i.saturating_sub(1)];
    let p1 = points[i];
    let p2 = points[(i + 1).min(n)];
    let p3 = points[(i + 2).min(n)];
    0.5 * ((2.0 * p1)
        + (-p0 + p2) * frac
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * frac * frac
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * frac * frac * frac)
}
