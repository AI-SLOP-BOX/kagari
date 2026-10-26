#![allow(dead_code)]
use rayon::prelude::*;

mod gaussian_blur;
#[cfg(test)]
pub(crate) use gaussian_blur::boxes_for_gauss;
pub use gaussian_blur::*;
mod layer_styles;
pub use layer_styles::*;
mod distortion;
pub use distortion::*;
/// Pack of 20 Additional Essential VFX Effects & Filters (Part 2).
// 1. Fast Box Blur (Separable 2-pass with O(n) sliding window per pass)
pub fn apply_fast_box_blur(pixels: &mut [u8], width: u32, height: u32, radius: u32) {
    if radius == 0 || width == 0 || height == 0 {
        return;
    }
    let r = radius as i32;
    let w = width as usize;
    let h = height as usize;
    let stride = w * 4;

    // --- Horizontal pass: pixels -> temp_h (sliding window) ---
    let mut temp_h = vec![0u8; pixels.len()];
    for y in 0..h {
        let row = y * stride;
        // Initialize accumulator for x=0
        let mut acc = [0f32; 4];
        let mut count = 0f32;
        for dx in -r..=r {
            let px = dx.clamp(0, w as i32 - 1) as usize;
            let idx = row + px * 4;
            for c in 0..4 {
                acc[c] += pixels[idx + c] as f32;
            }
            count += 1.0;
        }
        for c in 0..4 {
            temp_h[row + c] = (acc[c] / count).round() as u8;
        }

        // Slide the window across the row
        for x in 1..w {
            // Add new right edge
            let add_px = (x as i32 + r).min(w as i32 - 1) as usize;
            let add_idx = row + add_px * 4;
            for c in 0..4 {
                acc[c] += pixels[add_idx + c] as f32;
            }

            // Remove old left edge
            let rem_px = (x as i32 - r - 1).max(0) as usize;
            let rem_idx = row + rem_px * 4;
            for c in 0..4 {
                acc[c] -= pixels[rem_idx + c] as f32;
            }

            let out_idx = row + x * 4;
            for c in 0..4 {
                temp_h[out_idx + c] = (acc[c] / count).round() as u8;
            }
        }
    }

    // --- Vertical pass: temp_h -> pixels (sliding window) ---
    for x in 0..w {
        let col = x * 4;
        // Initialize accumulator for y=0
        let mut acc = [0f32; 4];
        let mut count = 0f32;
        for dy in -r..=r {
            let py = dy.clamp(0, h as i32 - 1) as usize;
            let idx = py * stride + col;
            for c in 0..4 {
                acc[c] += temp_h[idx + c] as f32;
            }
            count += 1.0;
        }
        let out_idx = col;
        for c in 0..4 {
            pixels[out_idx + c] = (acc[c] / count).round() as u8;
        }

        // Slide the window down the column
        for y in 1..h {
            // Add new bottom edge
            let add_py = (y as i32 + r).min(h as i32 - 1) as usize;
            let add_idx = add_py * stride + col;
            for c in 0..4 {
                acc[c] += temp_h[add_idx + c] as f32;
            }

            // Remove old top edge
            let rem_py = (y as i32 - r - 1).max(0) as usize;
            let rem_idx = rem_py * stride + col;
            for c in 0..4 {
                acc[c] -= temp_h[rem_idx + c] as f32;
            }

            let out_idx = y * stride + col;
            for c in 0..4 {
                pixels[out_idx + c] = (acc[c] / count).round() as u8;
            }
        }
    }
}

// 2. Directional Blur (rayon-parallelized)
pub fn apply_directional_blur(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    angle_deg: f32,
    length: f32,
) {
    if length <= 0.01 || width == 0 || height == 0 {
        return;
    }
    let rad = angle_deg.to_radians();
    let dx = rad.sin();
    let dy = -rad.cos();

    let temp = pixels.to_vec();
    let samples = ((length * 2.0) as usize).max(4).next_power_of_two();

    let sigma = (samples as f32 * 0.5 / 3.0).max(1.0);
    let mut weights = vec![0f32; samples];
    let mut w_sum = 0f32;
    for (s, w) in weights.iter_mut().enumerate() {
        let t = (s as f32 - (samples as f32 - 1.0) * 0.5) / sigma;
        *w = (-t * t * 0.5).exp();
        w_sum += *w;
    }
    for w in weights.iter_mut() {
        *w /= w_sum;
    }

    pixels
        .par_chunks_exact_mut(4)
        .enumerate()
        .for_each(|(i, px)| {
            let x = (i % width as usize) as u32;
            let y = (i / width as usize) as u32;
            let mut acc = [0f32; 4];
            for (s, &wt) in weights.iter().enumerate() {
                let offset = (s as f32) - (samples as f32 * 0.5);
                let sx = (x as f32 + dx * offset)
                    .round()
                    .clamp(0.0, width as f32 - 1.0) as u32;
                let sy = (y as f32 + dy * offset)
                    .round()
                    .clamp(0.0, height as f32 - 1.0) as u32;
                let idx = ((sy * width + sx) * 4) as usize;
                for c in 0..4 {
                    acc[c] += temp[idx + c] as f32 * wt;
                }
            }
            for c in 0..4 {
                px[c] = acc[c].round().clamp(0.0, 255.0) as u8;
            }
        });
}

// 3. Radial Blur (Spin) (rayon-parallelized)
pub fn apply_radial_blur(pixels: &mut [u8], width: u32, height: u32, amount: f32) {
    if amount.abs() < 0.01 || width == 0 || height == 0 {
        return;
    }
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;
    let temp = pixels.to_vec();

    let steps = 16;
    let mut weights = [0f32; 16];
    let sigma = (steps as f32 * 0.5 / 3.0).max(1.0);
    let mut w_sum = 0f32;
    for (s, w) in weights.iter_mut().enumerate() {
        let t = (s as f32 - (steps as f32 - 1.0) * 0.5) / sigma;
        *w = (-t * t * 0.5).exp();
        w_sum += *w;
    }
    for w in weights.iter_mut() {
        *w /= w_sum;
    }

    pixels
        .par_chunks_exact_mut(4)
        .enumerate()
        .for_each(|(i, px)| {
            let x = (i % width as usize) as u32;
            let y = (i / width as usize) as u32;
            let mut acc = [0f32; 4];
            let rx = x as f32 - cx;
            let ry = y as f32 - cy;
            let base_angle = ry.atan2(rx);
            let dist = (rx * rx + ry * ry).sqrt();

            for (s, &wt) in weights.iter().enumerate() {
                let t = (s as f32 / steps as f32 - 0.5) * amount * 0.05;
                let a = base_angle + t;
                let sx = (cx + a.cos() * dist).clamp(0.0, width as f32 - 1.0) as u32;
                let sy = (cy + a.sin() * dist).clamp(0.0, height as f32 - 1.0) as u32;
                let idx = ((sy * width + sx) * 4) as usize;
                for c in 0..4 {
                    acc[c] += temp[idx + c] as f32 * wt;
                }
            }
            for c in 0..4 {
                px[c] = acc[c].round().clamp(0.0, 255.0) as u8;
            }
        });
}

// 4. Unsharp Mask
pub fn apply_unsharp_mask(pixels: &mut [u8], width: u32, height: u32, amount: f32, radius: u32) {
    let mut blurred = pixels.to_vec();
    apply_fast_box_blur(&mut blurred, width, height, radius);

    let k = amount * 0.01;
    for i in (0..pixels.len()).step_by(4) {
        for c in 0..3 {
            let orig = pixels[i + c] as f32;
            let blur = blurred[i + c] as f32;
            pixels[i + c] = (orig + (orig - blur) * k).clamp(0.0, 255.0) as u8;
        }
    }
}

// 5. Sharpen
pub fn apply_sharpen(pixels: &mut [u8], width: u32, height: u32, amount: f32) {
    apply_unsharp_mask(pixels, width, height, amount, 1);
}

// 6. Glow
pub fn apply_glow(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    threshold: f32,
    radius: u32,
    intensity: f32,
) {
    let num_bytes = pixels.len();
    let mut glow_map = vec![0u8; num_bytes];

    // Soft threshold: pixels near the cutoff get partial weight, which removes
    // the harsh banding edge a hard binary threshold produces.
    let soft_width = (radius.max(2) as f32 * 8.0).min(64.0);
    for i in (0..num_bytes).step_by(4) {
        let luma =
            pixels[i] as f32 * 0.299 + pixels[i + 1] as f32 * 0.587 + pixels[i + 2] as f32 * 0.114;
        let t = ((luma - threshold * 255.0) / soft_width + 0.5).clamp(0.0, 1.0);
        // Smoothstep the transition for a filmic rolloff
        let w = t * t * (3.0 - 2.0 * t);
        glow_map[i] = (pixels[i] as f32 * w) as u8;
        glow_map[i + 1] = (pixels[i + 1] as f32 * w) as u8;
        glow_map[i + 2] = (pixels[i + 2] as f32 * w) as u8;
        glow_map[i + 3] = pixels[i + 3];
    }

    // True Gaussian on the glow map: bloom quality depends directly on this blur
    apply_gaussian_blur(&mut glow_map, width, height, radius);

    for i in (0..num_bytes).step_by(4) {
        for c in 0..3 {
            let base = pixels[i + c] as f32;
            let g = glow_map[i + c] as f32 * intensity;
            pixels[i + c] = (base + g).clamp(0.0, 255.0) as u8;
        }
    }
}

// 7. Drop Shadow
pub fn apply_drop_shadow(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    distance: f32,
    angle_deg: f32,
    softness: u32,
    shadow_color: [u8; 4],
) {
    let Some(pixel_count) = (width as usize).checked_mul(height as usize) else {
        return;
    };
    let Some(pixel_bytes) = pixel_count.checked_mul(4) else {
        return;
    };
    if width == 0 || height == 0 || pixels.len() != pixel_bytes {
        return;
    }
    let angle_deg = if angle_deg.is_finite() {
        angle_deg
    } else {
        0.0
    };
    let distance = if distance.is_finite() {
        distance.clamp(-4096.0, 4096.0)
    } else {
        0.0
    };
    let rad = angle_deg.to_radians();
    let dx = (rad.sin() * distance).round() as i32;
    let dy = (-rad.cos() * distance).round() as i32;

    let mut shadow_buf = vec![0u8; pixels.len()];

    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            let alpha = pixels[idx + 3];
            if alpha > 0 {
                let sx = (x as i32 + dx).clamp(0, width as i32 - 1) as u32;
                let sy = (y as i32 + dy).clamp(0, height as i32 - 1) as u32;
                let s_idx = ((sy * width + sx) * 4) as usize;
                shadow_buf[s_idx] = shadow_color[0];
                shadow_buf[s_idx + 1] = shadow_color[1];
                shadow_buf[s_idx + 2] = shadow_color[2];
                shadow_buf[s_idx + 3] =
                    ((alpha as f32 * (shadow_color[3] as f32 / 255.0)).round()) as u8;
            }
        }
    }

    // True Gaussian blur on the shadow alpha gives soft, smooth edges
    // (a single box blur produces visible hard banding rings).
    apply_gaussian_blur(&mut shadow_buf, width, height, softness);

    // Composite original over shadow
    for i in (0..pixels.len()).step_by(4) {
        let fg_a = pixels[i + 3] as f32 / 255.0;
        let bg_a = shadow_buf[i + 3] as f32 / 255.0;
        let out_a = fg_a + bg_a * (1.0 - fg_a);

        if out_a > 0.0 {
            for c in 0..3 {
                let fg_c = pixels[i + c] as f32 / 255.0;
                let bg_c = shadow_buf[i + c] as f32 / 255.0;
                let out_c = (fg_c * fg_a + bg_c * bg_a * (1.0 - fg_a)) / out_a;
                pixels[i + c] = (out_c * 255.0).round() as u8;
            }
            pixels[i + 3] = (out_a * 255.0).round() as u8;
        }
    }
}

// 8. CC Radial Fast Blur
pub fn apply_radial_fast_blur(pixels: &mut [u8], width: u32, height: u32, amount: f32) {
    apply_radial_blur(pixels, width, height, amount * 1.5);
}

// 9. Simple Choker
pub fn apply_simple_choker(pixels: &mut [u8], choke_amount: f32) {
    let k = 1.0 - (choke_amount * 0.01).clamp(-1.0, 1.0);
    for i in (3..pixels.len()).step_by(4) {
        let a = pixels[i] as f32 / 255.0;
        pixels[i] = (a * k * 255.0).round().clamp(0.0, 255.0) as u8;
    }
}

// 10. Matte Choker
pub fn apply_matte_choker(pixels: &mut [u8], choke_amount: f32, gray_level: f32) {
    apply_simple_choker(pixels, choke_amount);
    let thresh = (gray_level * 255.0) as u8;
    for i in (3..pixels.len()).step_by(4) {
        if pixels[i] < thresh {
            pixels[i] = 0;
        }
    }
}

// 11. Tint
pub fn apply_tint(pixels: &mut [u8], black_to: [u8; 3], white_to: [u8; 3], amount: f32) {
    let k = amount.clamp(0.0, 1.0);
    for i in (0..pixels.len()).step_by(4) {
        let luma = (pixels[i] as f32 * 0.299
            + pixels[i + 1] as f32 * 0.587
            + pixels[i + 2] as f32 * 0.114)
            / 255.0;
        for c in 0..3 {
            let tinted = black_to[c] as f32 + (white_to[c] as f32 - black_to[c] as f32) * luma;
            let orig = pixels[i + c] as f32;
            pixels[i + c] = (orig + (tinted - orig) * k).round() as u8;
        }
    }
}

// 12. Tritone
pub fn apply_tritone(pixels: &mut [u8], shadow_c: [u8; 3], mid_c: [u8; 3], high_c: [u8; 3]) {
    for i in (0..pixels.len()).step_by(4) {
        let luma = (pixels[i] as f32 * 0.299
            + pixels[i + 1] as f32 * 0.587
            + pixels[i + 2] as f32 * 0.114)
            / 255.0;
        for c in 0..3 {
            let val = if luma < 0.5 {
                shadow_c[c] as f32 + (mid_c[c] as f32 - shadow_c[c] as f32) * (luma * 2.0)
            } else {
                mid_c[c] as f32 + (high_c[c] as f32 - mid_c[c] as f32) * ((luma - 0.5) * 2.0)
            };
            pixels[i + c] = val.round() as u8;
        }
    }
}

// 13. Posterize
pub fn apply_posterize(pixels: &mut [u8], levels: u32) {
    let l = levels.max(2) as f32;
    for i in (0..pixels.len()).step_by(4) {
        for c in 0..3 {
            let norm = pixels[i + c] as f32 / 255.0;
            pixels[i + c] = ((norm * l).floor() / l * 255.0).round() as u8;
        }
    }
}

// 14. Invert
pub fn apply_invert(pixels: &mut [u8], invert_alpha: bool) {
    for i in (0..pixels.len()).step_by(4) {
        pixels[i] = 255 - pixels[i];
        pixels[i + 1] = 255 - pixels[i + 1];
        pixels[i + 2] = 255 - pixels[i + 2];
        if invert_alpha {
            pixels[i + 3] = 255 - pixels[i + 3];
        }
    }
}

// 15. Threshold
pub fn apply_threshold(pixels: &mut [u8], threshold: u8) {
    for i in (0..pixels.len()).step_by(4) {
        let luma =
            (pixels[i] as u32 * 299 + pixels[i + 1] as u32 * 587 + pixels[i + 2] as u32 * 114)
                / 1000;
        let val = if luma as u8 >= threshold { 255 } else { 0 };
        pixels[i] = val;
        pixels[i + 1] = val;
        pixels[i + 2] = val;
    }
}

#[cfg(test)]
#[path = "ae_effects_pack/camera_shake_tests.rs"]
mod camera_shake_tests;
#[cfg(test)]
#[path = "ae_effects_pack/directional_radial_tests.rs"]
mod directional_radial_tests;
#[cfg(test)]
#[path = "ae_effects_pack/gaussian_tests.rs"]
mod gaussian_tests;
#[cfg(test)]
#[path = "ae_effects_pack/glow_tests.rs"]
mod glow_tests;
#[cfg(test)]
#[path = "ae_effects_pack/shadow_tests.rs"]
mod shadow_tests;
#[cfg(test)]
#[path = "ae_effects_pack/tests.rs"]
mod tests;
