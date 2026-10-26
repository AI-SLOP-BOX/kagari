use super::*;

// 16. Twirl
pub fn apply_twirl(pixels: &mut [u8], width: u32, height: u32, angle_deg: f32, radius: f32) {
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;
    let max_r = radius.max(1.0);
    let max_angle = angle_deg.to_radians();

    let temp = pixels.to_vec();

    // Per-pixel independent — parallelize with rayon
    pixels
        .par_chunks_exact_mut(4)
        .enumerate()
        .for_each(|(i, out)| {
            let x = i % width as usize;
            let y = i / width as usize;
            let rx = x as f32 - cx;
            let ry = y as f32 - cy;
            let r = (rx * rx + ry * ry).sqrt();

            if r < max_r {
                let factor = (1.0 - r / max_r).powi(2);
                let twirl_a = max_angle * factor;
                let curr_a = ry.atan2(rx);
                let new_a = curr_a + twirl_a;

                let sx = (cx + new_a.cos() * r).clamp(0.0, width as f32 - 1.0) as u32;
                let sy = (cy + new_a.sin() * r).clamp(0.0, height as f32 - 1.0) as u32;

                let s_idx = ((sy * width + sx) * 4) as usize;
                out.copy_from_slice(&temp[s_idx..s_idx + 4]);
            } else {
                out.copy_from_slice(&temp[i * 4..i * 4 + 4]);
            }
        });
}

// 17. Bulge
pub fn apply_bulge(pixels: &mut [u8], width: u32, height: u32, amount: f32, radius: f32) {
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;
    let temp = pixels.to_vec();

    pixels
        .par_chunks_exact_mut(4)
        .enumerate()
        .for_each(|(i, out)| {
            let x = i % width as usize;
            let y = i / width as usize;
            let rx = x as f32 - cx;
            let ry = y as f32 - cy;
            let r = (rx * rx + ry * ry).sqrt();

            if r < radius && r > 0.001 {
                let norm_r = r / radius;
                let factor = 1.0 + amount * (1.0 - norm_r * norm_r);
                let sx = (cx + rx * factor).clamp(0.0, width as f32 - 1.0) as u32;
                let sy = (cy + ry * factor).clamp(0.0, height as f32 - 1.0) as u32;

                let s_idx = ((sy * width + sx) * 4) as usize;
                out.copy_from_slice(&temp[s_idx..s_idx + 4]);
            } else {
                out.copy_from_slice(&temp[i * 4..i * 4 + 4]);
            }
        });
}

// 18. Offset
pub fn apply_offset(pixels: &mut [u8], width: u32, height: u32, shift_x: i32, shift_y: i32) {
    let temp = pixels.to_vec();
    let w = width as i32;
    let h = height as i32;

    for y in 0..h {
        for x in 0..w {
            let sx = ((x - shift_x) % w + w) % w;
            let sy = ((y - shift_y) % h + h) % h;

            let idx = ((y * w + x) * 4) as usize;
            let s_idx = ((sy * w + sx) * 4) as usize;
            pixels[idx..idx + 4].copy_from_slice(&temp[s_idx..s_idx + 4]);
        }
    }
}

// 19. Venetian Blinds
pub fn apply_venetian_blinds(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    completion: f32,
    width_px: u32,
) {
    if completion <= 0.0 || width_px == 0 || width == 0 || height == 0 {
        return;
    }
    let blind_w = width_px as usize;
    let cut_w = (blind_w as f32 * (completion * 0.01)) as usize;

    for y in 0..height as usize {
        for x in 0..width as usize {
            if (x % blind_w) < cut_w {
                let idx = (y * width as usize + x) * 4;
                pixels[idx + 3] = 0;
            }
        }
    }
}

// 20. Linear Wipe
pub fn apply_linear_wipe(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    completion: f32,
    angle_deg: f32,
) {
    if completion <= 0.0 {
        return;
    }
    if completion >= 100.0 {
        for i in (3..pixels.len()).step_by(4) {
            pixels[i] = 0;
        }
        return;
    }

    let rad = angle_deg.to_radians();
    let dir = [rad.cos(), rad.sin()];
    let corners = [
        0.0,
        width.saturating_sub(1) as f32 * dir[0],
        height.saturating_sub(1) as f32 * dir[1],
        width.saturating_sub(1) as f32 * dir[0] + height.saturating_sub(1) as f32 * dir[1],
    ];
    let min_proj = corners.iter().copied().fold(f32::INFINITY, f32::min);
    let max_proj = corners.iter().copied().fold(f32::NEG_INFINITY, f32::max);

    for y in 0..height {
        for x in 0..width {
            let proj = x as f32 * dir[0] + y as f32 * dir[1];
            let normalized = if max_proj > min_proj {
                (proj - min_proj) / (max_proj - min_proj)
            } else {
                0.0
            };
            if normalized > completion * 0.01 {
                let idx = ((y * width + x) * 4 + 3) as usize;
                pixels[idx] = 0;
            }
        }
    }
}
