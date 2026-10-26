use crate::core::software_renderer::rgba_buffer_size;
use rayon::prelude::*;

// ──────────────────────────── FractalNoise ─────────────────────────────

/// Simple 2D value noise hash
fn value_noise(x: f32, y: f32, seed: u32) -> f32 {
    let mut n = (x as u32)
        .wrapping_mul(374761393)
        .wrapping_add((y as u32).wrapping_mul(668265263))
        .wrapping_add(seed);
    n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    (n as f32) / (u32::MAX as f32)
}

/// Smooth interpolation
fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// 2D interpolated value noise in [0,1]
fn noise_2d(x: f32, y: f32, seed: u32) -> f32 {
    let xi = x.floor();
    let yi = y.floor();
    let xf = smooth(x - xi);
    let yf = smooth(y - yi);
    let n00 = value_noise(xi, yi, seed);
    let n10 = value_noise(xi + 1.0, yi, seed);
    let n01 = value_noise(xi, yi + 1.0, seed);
    let n11 = value_noise(xi + 1.0, yi + 1.0, seed);
    let nx0 = n00 + (n10 - n00) * xf;
    let nx1 = n01 + (n11 - n01) * xf;
    nx0 + (nx1 - nx0) * yf
}

/// Fractional Brownian Motion (fBm) - multiple octaves of noise
fn fbm(x: f32, y: f32, octaves: u32, seed: u32) -> f32 {
    let mut value = 0.0f32;
    let mut amplitude = 0.5f32;
    let mut frequency = 1.0f32;
    for o in 0..octaves {
        value += noise_2d(x * frequency, y * frequency, seed.wrapping_add(o * 1000)) * amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    value
}

/// Apply FractalNoise - generates procedural noise texture
/// fractal_type: 0=Basic fBm, 1=Turbulence (abs of each octave), 2=Dynamic (turbulence), 3=Crane (ridged)
#[allow(clippy::too_many_arguments)]
pub fn apply_fractal_noise(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    fractal_type: f32,
    contrast: f32,
    brightness: f32,
    complexity: f32,
    evolution: f32,
) {
    let Some(expected) = rgba_buffer_size(width, height) else {
        return;
    };
    if pixels.len() < expected {
        return;
    }
    let w = width as f32;
    let h = height as f32;
    let octaves = (complexity.clamp(1.0, 10.0)) as u32;
    let ft = fractal_type as u32;
    let scale = 3.0; // base frequency scale
    let evo = evolution * 0.1; // slow down evolution
    let seed = (evo * 1000.0) as u32;

    for py in 0..height {
        for px in 0..width {
            let nx = px as f32 / w * scale;
            let ny = py as f32 / h * scale;
            let mut n = 0.0f32;

            match ft {
                0 => {
                    // Basic fBm
                    n = fbm(nx + evo, ny + evo, octaves, seed);
                }
                1 => {
                    // Turbulence: abs of each octave
                    let mut amp = 0.5f32;
                    let mut freq = 1.0f32;
                    for o in 0..octaves {
                        n += noise_2d(
                            (nx + evo) * freq,
                            (ny + evo) * freq,
                            seed.wrapping_add(o * 1000),
                        )
                        .abs()
                            * amp;
                        amp *= 0.5;
                        freq *= 2.0;
                    }
                    n = n.clamp(0.0, 1.0);
                }
                2 => {
                    // Dynamic (evolving turbulence)
                    let mut amp = 0.5f32;
                    let mut freq = 1.0f32;
                    for o in 0..octaves {
                        let ev = evo * (o as f32 + 1.0);
                        n += noise_2d(
                            (nx + ev) * freq,
                            (ny + ev * 0.7) * freq,
                            seed.wrapping_add(o * 1000),
                        )
                        .abs()
                            * amp;
                        amp *= 0.5;
                        freq *= 2.0;
                    }
                    n = n.clamp(0.0, 1.0);
                }
                3 => {
                    // Crane (ridged noise)
                    let mut amp = 0.5f32;
                    let mut freq = 1.0f32;
                    for o in 0..octaves {
                        let val = noise_2d(
                            (nx + evo) * freq,
                            (ny + evo) * freq,
                            seed.wrapping_add(o * 1000),
                        );
                        n += (1.0 - (val * 2.0 - 1.0).abs()).powf(2.0) * amp;
                        amp *= 0.5;
                        freq *= 2.0;
                    }
                    n = n.clamp(0.0, 1.0);
                }
                _ => {
                    n = fbm(nx + evo, ny + evo, octaves, seed);
                }
            }

            // Apply contrast and brightness
            n = ((n - 0.5) * contrast + 0.5 + brightness - 0.5).clamp(0.0, 1.0);
            let gray = (n * 255.0) as u8;
            let idx = ((py * width + px) * 4) as usize;
            if idx + 3 < pixels.len() {
                pixels[idx] = gray;
                pixels[idx + 1] = gray;
                pixels[idx + 2] = gray;
                // Alpha untouched: layer opacity / mattes live in alpha and
                // must survive generator effects (see BlockGlitch brief).
            }
        }
    }
}

// ──────────────────────────── CompoundBlur ─────────────────────────────

/// Apply variable blur using a luminance-based intensity map.
/// Bright areas in the source layer = more blur.
/// Apply variable compound blur using luminance as intensity map.
/// source_layer: layer ID for blur map (placeholder).
/// max_blur: maximum blur radius in pixels.
pub fn apply_compound_blur(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    _source_layer: f32,
    max_blur: f32,
) {
    apply_compound_blur_with_map(pixels, width, height, None, max_blur);
}

/// Apply compound blur using an optional external layer as the blur map.
/// Bright pixels in `blur_map` receive a larger blur radius. When no map is
/// supplied, the target image remains the map for backwards compatibility.
pub fn apply_compound_blur_with_map(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    blur_map: Option<&[u8]>,
    max_blur: f32,
) {
    if max_blur < 0.5 {
        return;
    }
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        .unwrap_or(0);
    let map = blur_map.filter(|candidate| candidate.len() == expected);
    let radius = max_blur.round() as u32;
    let orig = pixels.to_vec();

    // Parallel per-row processing
    pixels
        .par_chunks_mut((width * 4) as usize)
        .enumerate()
        .for_each(|(row, row_chunk)| {
            let py = row as u32;
            for px in 0..width {
                let idx = (px * 4) as usize;
                if idx + 3 >= row_chunk.len() {
                    continue;
                }

                // AE's Compound Blur uses the selected layer as an intensity
                // map. If no source layer is available, use the target as a
                // deterministic fallback.
                let map_px = map.unwrap_or(&orig);
                let map_idx = ((py * width + px) * 4) as usize;
                let lum = (map_px[map_idx] as f32
                    + map_px[map_idx + 1] as f32
                    + map_px[map_idx + 2] as f32)
                    / 765.0;
                let r = (lum * radius as f32).round() as u32;
                if r < 1 {
                    continue;
                }

                let mut sum_r = 0.0f32;
                let mut sum_g = 0.0f32;
                let mut sum_b = 0.0f32;
                let mut count = 0.0f32;

                for dy in -(r as i32)..=r as i32 {
                    for dx in -(r as i32)..=r as i32 {
                        if dx * dx + dy * dy > (r as i32 * r as i32) {
                            continue;
                        }
                        let sx = (px as i32 + dx).max(0).min(width as i32 - 1) as u32;
                        let sy = (py as i32 + dy).max(0).min(height as i32 - 1) as u32;
                        let sidx = ((sy * width + sx) * 4) as usize;
                        if sidx + 3 < orig.len() {
                            sum_r += orig[sidx] as f32;
                            sum_g += orig[sidx + 1] as f32;
                            sum_b += orig[sidx + 2] as f32;
                            count += 1.0;
                        }
                    }
                }
                if count > 0.0 {
                    row_chunk[idx] = (sum_r / count) as u8;
                    row_chunk[idx + 1] = (sum_g / count) as u8;
                    row_chunk[idx + 2] = (sum_b / count) as u8;
                }
            }
        });
}

// ──────────────────────────── Minimax ─────────────────────────────

/// Minimax - dilate (max) or erode (min) based on luminance.
/// operation: 0=Min (erode), 1=Max (dilate)
/// Apply minimax (dilate/erode) matte operation.
/// operation: 0=Min (erode dark areas), 1=Max (dilate bright areas).
/// radius: operation radius in pixels.
pub fn apply_minimax(pixels: &mut [u8], width: u32, height: u32, operation: f32, radius: f32) {
    let r = radius.round() as u32;
    if r < 1 {
        return;
    }
    let orig = pixels.to_vec();
    let use_max = operation > 0.5;

    pixels
        .par_chunks_mut((width * 4) as usize)
        .enumerate()
        .for_each(|(row, row_chunk)| {
            let py = row as u32;
            for px in 0..width {
                let idx = (px * 4) as usize;
                if idx + 3 >= row_chunk.len() {
                    continue;
                }

                let mut best = orig[((py * width + px) * 4) as usize] as f32;
                for dy in -(r as i32)..=r as i32 {
                    for dx in -(r as i32)..=r as i32 {
                        if dx * dx + dy * dy > (r as i32 * r as i32) {
                            continue;
                        }
                        let sx = (px as i32 + dx).max(0).min(width as i32 - 1) as u32;
                        let sy = (py as i32 + dy).max(0).min(height as i32 - 1) as u32;
                        let sidx = ((sy * width + sx) * 4) as usize;
                        if sidx + 3 < orig.len() {
                            let lum =
                                (orig[sidx] as f32 + orig[sidx + 1] as f32 + orig[sidx + 2] as f32)
                                    / 3.0;
                            if use_max {
                                best = best.max(lum);
                            } else {
                                best = best.min(lum);
                            }
                        }
                    }
                }
                let v = best as u8;
                row_chunk[idx] = v;
                row_chunk[idx + 1] = v;
                row_chunk[idx + 2] = v;
                // Keep alpha
            }
        });
}

// ──────────────────────────── ShiftChannels ─────────────────────────────

/// Shift RGBA channels - remap each channel to a different source.
/// take_red/green/blue/alpha: 0=Red, 1=Green, 2=Blue, 3=Alpha, 4=Off(0), 5=On(1)
/// Shift RGBA channels - remap each channel to a different source.
/// take_red/green/blue/alpha: 0=Red, 1=Green, 2=Blue, 3=Alpha, 4=Off(0), 5=On(1).
pub fn apply_shift_channels(
    pixels: &mut [u8],
    _width: u32,
    _height: u32,
    take_red: f32,
    take_green: f32,
    take_blue: f32,
    take_alpha: f32,
) {
    let src_r = take_red as usize;
    let src_g = take_green as usize;
    let src_b = take_blue as usize;
    let src_a = take_alpha as usize;

    for px in pixels.as_chunks_mut::<4>().0 {
        let orig = [px[0], px[1], px[2], px[3]];
        px[0] = match src_r {
            0..=3 => orig[src_r],
            4 => 0,
            5 => 255,
            _ => orig[0],
        };
        px[1] = match src_g {
            0..=3 => orig[src_g],
            4 => 0,
            5 => 255,
            _ => orig[1],
        };
        px[2] = match src_b {
            0..=3 => orig[src_b],
            4 => 0,
            5 => 255,
            _ => orig[2],
        };
        px[3] = match src_a {
            0..=3 => orig[src_a],
            4 => 0,
            5 => 255,
            _ => orig[3],
        };
    }
}
