use crate::core::cpu_effects_new::warp::sample_bilinear;
use crate::core::software_renderer::rgba_buffer_size;
use rayon::prelude::*;

/// Apply chromatic aberration by shifting R and B channels.
/// shift_r/shift_b: pixel offset for red/blue channels (negative = left/up).
/// edge_falloff: 0.0=hard edge, 1.0=smooth falloff at borders.
pub fn apply_chromatic_aberration(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    shift_r: f32,
    shift_b: f32,
    edge_falloff: f32,
) {
    let Some(expected) = rgba_buffer_size(width, height) else {
        return;
    };
    if pixels.len() < expected {
        return;
    }
    let mut tmp = vec![0u8; pixels.len()];
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;

    // Per-pixel independent — parallelized across rows with rayon.
    tmp.par_chunks_exact_mut(4)
        .enumerate()
        .for_each(|(i, out)| {
            let x = (i % width as usize) as f32;
            let y = (i / width as usize) as f32;
            let dx = (x - cx) / cx;
            let dy = (y - cy) / cy;
            let dist = ((dx * dx + dy * dy).sqrt()).min(1.0);
            let falloff = if edge_falloff < 1.0 {
                dist.powf(1.0 - edge_falloff.clamp(0.0, 1.0))
            } else {
                dist
            };

            // Radial dispersion model (from NextVFX): channels shift ALONG the
            // vector from image center through this pixel — like real lens
            // chromatic aberration, which fringes radially, not horizontally.
            let dir_x = if dist > 1e-6 { dx / dist } else { 0.0 };
            let dir_y = if dist > 1e-6 { dy / dist } else { 0.0 };
            let r_sample = sample_bilinear(
                pixels,
                width,
                height,
                x + shift_r * falloff * dir_x,
                y + shift_r * falloff * dir_y,
            );
            let g_sample = sample_bilinear(pixels, width, height, x, y);
            let b_sample = sample_bilinear(
                pixels,
                width,
                height,
                x - shift_b * falloff * dir_x,
                y - shift_b * falloff * dir_y,
            );

            out[0] = r_sample[0];
            out[1] = g_sample[1];
            out[2] = b_sample[2];
            out[3] = g_sample[3];
        });
    pixels[..tmp.len()].copy_from_slice(&tmp);
}

/// Apply vignette darkening effect.
/// intensity: 0.0=no effect, 1.0=full black edges.
/// roundness: shape of vignette (0.0=circular, 1.0=elliptical).
/// feather: edge softness (0.0=hard, 1.0=very soft).
/// color: vignette color \[r,g,b].
pub fn apply_vignette(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    intensity: f32,
    roundness: f32,
    feather: f32,
    color: [f32; 4],
) {
    let Some(expected) = rgba_buffer_size(width, height) else {
        return;
    };
    if pixels.len() < expected {
        return;
    }
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;
    let aspect = width as f32 / height as f32;

    pixels.par_chunks_mut(4).enumerate().for_each(|(i, chunk)| {
        if chunk.len() < 4 {
            return;
        }
        let px = (i as u32) % width;
        let py = (i as u32) / width;
        let dx = (px as f32 - cx) / cx;
        let dy = (py as f32 - cy) / cy;
        let rx = dx * aspect;
        let ry = dy;
        let norm_dist = ((rx * rx + ry * ry).powf(roundness.clamp(0.1, 10.0) * 0.5)).min(1.0);
        // feather: 0.0=hard edge (vignette only at border), 1.0=very soft (vignette reaches center)
        let soft_start = (1.0 - feather.clamp(0.0, 1.0)).max(0.001);
        let vignette = ((norm_dist - soft_start) / (1.0 - soft_start)).clamp(0.0, 1.0);
        let factor = vignette * intensity;

        for ch in 0..3 {
            let orig = chunk[ch] as f32 / 255.0;
            let blended = orig * (1.0 - factor) + color[ch] * factor;
            chunk[ch] = (blended.clamp(0.0, 1.0) * 255.0) as u8;
        }
    });
}

fn pcg_hash(input: u32) -> u32 {
    let state = input.wrapping_mul(747796405).wrapping_add(2891336453);
    let word = ((state >> ((state >> 28) + 4)) ^ state).wrapping_mul(277803737);
    (word >> 22) ^ word
}

/// Apply film grain noise.
/// intensity: grain strength (0.0=none, 1.0=heavy).
/// grain_size: block size in pixels (1=fine, 5=coarse).
/// seed: random seed (use frame number for temporal variation).
pub fn apply_film_grain(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    intensity: f32,
    grain_size: u32,
    seed: u32,
) {
    let Some(expected) = rgba_buffer_size(width, height) else {
        return;
    };
    if pixels.len() < expected || grain_size == 0 {
        return;
    }
    let gs = grain_size.max(1);
    let blocks_w = width.div_ceil(gs);
    let blocks_h = height.div_ceil(gs);
    let mut noise_map = vec![0i16; (blocks_w * blocks_h) as usize];

    for by in 0..blocks_h {
        for bx in 0..blocks_w {
            let hash = pcg_hash(seed.wrapping_add(by * blocks_w + bx));
            let val = ((hash as i32).rem_euclid(512)) - 256;
            let idx = (by * blocks_w + bx) as usize;
            if idx < noise_map.len() {
                noise_map[idx] = val as i16;
            }
        }
    }

    pixels.par_chunks_mut(4).enumerate().for_each(|(i, chunk)| {
        if chunk.len() < 4 {
            return;
        }
        let px = (i as u32) % width;
        let py = (i as u32) / width;
        let bx = (px / gs) as usize;
        let by = (py / gs) as usize;
        let block_w = width.div_ceil(gs) as usize;
        let idx = by * block_w + bx;
        if idx >= noise_map.len() {
            return;
        }
        let noise = noise_map[idx] as f32 * intensity;
        for ch in chunk.iter_mut().take(3) {
            let v = *ch as f32 + noise;
            *ch = v.clamp(0.0, 255.0) as u8;
        }
    });
}
