use crate::core::effect_utils;
use crate::core::software_renderer::rgba_buffer_size;
use rayon::prelude::*;

/// Bilinear interpolation sample — delegates to shared `effect_utils`.
#[inline]
pub fn sample_bilinear(pixels: &[u8], width: u32, height: u32, x: f32, y: f32) -> [u8; 4] {
    effect_utils::sample_bilinear(pixels, width, height, x, y)
}

/// Apply directional motion blur.
/// angle: blur direction in degrees (0=right, 90=down).
/// length: blur distance in pixels (0=no blur).
pub fn apply_motion_blur(pixels: &mut [u8], width: u32, height: u32, angle: f32, length: f32) {
    let Some(expected) = rgba_buffer_size(width, height) else {
        return;
    };
    if pixels.len() < expected || length < 1.0 {
        return;
    }
    let rad = angle.to_radians();
    let dir_x = rad.cos();
    let dir_y = rad.sin();
    let samples = (length * 2.0 + 1.0).ceil() as i32;
    let inv_samples = 1.0 / samples as f32;
    let mut tmp = vec![0u8; pixels.len()];

    // Directional blur is sample-heavy: parallelize per pixel with rayon.
    tmp.par_chunks_exact_mut(4)
        .enumerate()
        .for_each(|(i, out)| {
            let x = (i % width as usize) as f32;
            let y = (i / width as usize) as f32;
            let mut sum_r = 0f32;
            let mut sum_g = 0f32;
            let mut sum_b = 0f32;
            let mut sum_a = 0f32;

            for s in -samples / 2..=samples / 2 {
                let offset = s as f32;
                let sx = x + dir_x * offset;
                let sy = y + dir_y * offset;
                let ch = sample_bilinear(pixels, width, height, sx, sy);
                sum_r += ch[0] as u32 as f32;
                sum_g += ch[1] as u32 as f32;
                sum_b += ch[2] as u32 as f32;
                sum_a += ch[3] as u32 as f32;
            }

            out[0] = (sum_r * inv_samples) as u8;
            out[1] = (sum_g * inv_samples) as u8;
            out[2] = (sum_b * inv_samples) as u8;
            out[3] = (sum_a * inv_samples) as u8;
        });
    pixels[..tmp.len()].copy_from_slice(&tmp);
}

/// Apply 4-corner mesh warp displacement.
/// top_left/top_right/bottom_left/bottom_right: \[x,y] displacement in pixels.
pub fn apply_mesh_warp(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    top_left: [f32; 2],
    top_right: [f32; 2],
    bottom_left: [f32; 2],
    bottom_right: [f32; 2],
) {
    let Some(expected) = rgba_buffer_size(width, height) else {
        return;
    };
    if pixels.len() < expected {
        return;
    }
    let mut tmp = vec![0u8; pixels.len()];
    let w = width as f32;
    let h = height as f32;

    // Bilinear homography warp is per-pixel independent — parallelize.
    tmp.par_chunks_exact_mut(4)
        .enumerate()
        .for_each(|(i, out)| {
            let out_x = i % width as usize;
            let out_y = i / width as usize;
            let u = out_x as f32 / w;
            let v = out_y as f32 / h;

            let tl_x = top_left[0] * w;
            let tl_y = top_left[1] * h;
            let tr_x = top_right[0] * w;
            let tr_y = top_right[1] * h;
            let bl_x = bottom_left[0] * w;
            let bl_y = bottom_left[1] * h;
            let br_x = bottom_right[0] * w;
            let br_y = bottom_right[1] * h;

            let src_x = tl_x * (1.0 - u) * (1.0 - v)
                + tr_x * u * (1.0 - v)
                + bl_x * (1.0 - u) * v
                + br_x * u * v;
            let src_y = tl_y * (1.0 - u) * (1.0 - v)
                + tr_y * u * (1.0 - v)
                + bl_y * (1.0 - u) * v
                + br_y * u * v;

            let ch = sample_bilinear(pixels, width, height, src_x, src_y);
            out.copy_from_slice(&ch);
        });
    pixels[..tmp.len()].copy_from_slice(&tmp);
}

/// Displace pixels using a source layer as the displacement map.
/// This is a placeholder - in production, source_layer would be a reference to another layer's rendered pixels.
/// Here we apply a simple horizontal/vertical displacement based on luminance.
/// Apply displacement map warp.
/// Uses the actual professional displacement map kernel with luminance reference channel.
pub fn apply_displacement_map(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    _source_layer: f32,
    max_horizontal: f32,
    max_vertical: f32,
) {
    let options = crate::core::displacement_map::DisplacementMapOptions {
        max_horizontal_displacement: max_horizontal,
        max_vertical_displacement: max_vertical,
        horizontal_channel: crate::core::displacement_map::DisplacementChannel::Luminance,
        vertical_channel: crate::core::displacement_map::DisplacementChannel::Luminance,
        wrap_pixels: false,
    };
    let orig = pixels.to_vec();
    let displaced = crate::core::displacement_map::apply_displacement_map(
        &orig, &orig, width, height, &options,
    );
    pixels.copy_from_slice(&displaced);
}
