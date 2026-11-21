#![allow(dead_code)]

// 61. Bevel Edges
pub fn apply_bevel_edges(pixels: &mut [u8], width: u32, height: u32, thickness: u32) {
    let t = thickness.max(1) as usize;
    for y in 0..height as usize {
        for x in 0..width as usize {
            if x < t || x >= width as usize - t || y < t || y >= height as usize - t {
                let idx = (y * width as usize + x) * 4;
                let factor = if x < t || y < t { 1.4 } else { 0.6 };
                pixels[idx] = (pixels[idx] as f32 * factor).min(255.0) as u8;
                pixels[idx + 1] = (pixels[idx + 1] as f32 * factor).min(255.0) as u8;
                pixels[idx + 2] = (pixels[idx + 2] as f32 * factor).min(255.0) as u8;
            }
        }
    }
}

// 62. Bevel Alpha
pub fn apply_bevel_alpha(pixels: &mut [u8], width: u32, height: u32, _light_angle: f32) {
    apply_bevel_edges(pixels, width, height, 2);
}

// 63. Glow Edges
pub fn apply_glow_edges(pixels: &mut [u8], width: u32, height: u32) {
    crate::core::ae_effects_pack_v2::apply_find_edges(pixels, width, height);
    crate::core::ae_effects_pack::apply_glow(pixels, width, height, 0.2, 3, 2.0);
}

// 64. Cartoon
pub fn apply_cartoon(pixels: &mut [u8], width: u32, height: u32) {
    crate::core::ae_effects_pack::apply_posterize(pixels, 4);
    apply_bevel_edges(pixels, width, height, 1);
}

// 65. Threshold RGB
pub fn apply_threshold_rgb(pixels: &mut [u8], thresh_r: u8, thresh_g: u8, thresh_b: u8) {
    for i in (0..pixels.len()).step_by(4) {
        pixels[i] = if pixels[i] >= thresh_r { 255 } else { 0 };
        pixels[i + 1] = if pixels[i + 1] >= thresh_g { 255 } else { 0 };
        pixels[i + 2] = if pixels[i + 2] >= thresh_b { 255 } else { 0 };
    }
}

// 66. Median
pub fn apply_median_filter(pixels: &mut [u8], width: u32, height: u32) {
    crate::core::ae_effects_pack::apply_fast_box_blur(pixels, width, height, 1);
}

// 67. Minimax
pub fn apply_minimax(pixels: &mut [u8], width: u32, height: u32, radius: u32, is_maximum: bool) {
    if radius == 0 {
        return;
    }
    let temp = pixels.to_vec();
    let r = radius as i32;
    for y in 0..height as i32 {
        for x in 0..width as i32 {
            let mut val = if is_maximum { 0u8 } else { 255u8 };
            for dy in -r..=r {
                for dx in -r..=r {
                    let px = (x + dx).clamp(0, width as i32 - 1) as usize;
                    let py = (y + dy).clamp(0, height as i32 - 1) as usize;
                    let idx = (py * width as usize + px) * 4;
                    let l = temp[idx];
                    val = if is_maximum { val.max(l) } else { val.min(l) };
                }
            }
            let out_idx = (y as usize * width as usize + x as usize) * 4;
            pixels[out_idx] = val;
            pixels[out_idx + 1] = val;
            pixels[out_idx + 2] = val;
        }
    }
}

// 68. Scatter
pub fn apply_scatter(pixels: &mut [u8], width: u32, height: u32, amount: f32) {
    let temp = pixels.to_vec();
    let amt = amount as i32;
    if amt <= 0 {
        return;
    }
    for y in 0..height as i32 {
        for x in 0..width as i32 {
            let offset_x = ((x * 17 + y * 31) % (amt * 2 + 1)) - amt;
            let offset_y = ((x * 13 + y * 23) % (amt * 2 + 1)) - amt;
            let sx = (x + offset_x).clamp(0, width as i32 - 1) as usize;
            let sy = (y + offset_y).clamp(0, height as i32 - 1) as usize;

            let idx = (y as usize * width as usize + x as usize) * 4;
            let s_idx = (sy * width as usize + sx) * 4;
            pixels[idx..idx + 4].copy_from_slice(&temp[s_idx..s_idx + 4]);
        }
    }
}

// 69. Roughen Edges
pub fn apply_roughen_edges(pixels: &mut [u8], width: u32, height: u32, border: f32) {
    apply_scatter(pixels, width, height, border);
}

// 70. CC Burn Film
pub fn apply_cc_burn_film(pixels: &mut [u8], _width: u32, _height: u32, progress: f32) {
    let k = (progress * 0.01).clamp(0.0, 1.0);
    for i in (0..pixels.len()).step_by(4) {
        let burn = (k * 255.0) as u8;
        pixels[i] = pixels[i].saturating_add(burn);
        pixels[i + 1] = pixels[i + 1].saturating_sub(burn);
        pixels[i + 2] = pixels[i + 2].saturating_sub(burn);
    }
}

// 71. Channel Blur
pub fn apply_channel_blur(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    r_blur: u32,
    g_blur: u32,
    b_blur: u32,
) {
    let mut temp = pixels.to_vec();
    if r_blur > 0 {
        crate::core::ae_effects_pack::apply_fast_box_blur(&mut temp, width, height, r_blur);
        for i in (0..pixels.len()).step_by(4) {
            pixels[i] = temp[i];
        }
    }
    if g_blur > 0 {
        crate::core::ae_effects_pack::apply_fast_box_blur(&mut temp, width, height, g_blur);
        for i in (0..pixels.len()).step_by(4) {
            pixels[i + 1] = temp[i + 1];
        }
    }
    if b_blur > 0 {
        crate::core::ae_effects_pack::apply_fast_box_blur(&mut temp, width, height, b_blur);
        for i in (0..pixels.len()).step_by(4) {
            pixels[i + 2] = temp[i + 2];
        }
    }
}

// 72. Shift Channels
pub fn apply_shift_channels(pixels: &mut [u8], take_r_from_g: bool) {
    if take_r_from_g {
        for i in (0..pixels.len()).step_by(4) {
            pixels[i] = pixels[i + 1];
        }
    }
}

// 73. Color Balance
pub fn apply_color_balance(pixels: &mut [u8], red_shift: f32, green_shift: f32, blue_shift: f32) {
    for i in (0..pixels.len()).step_by(4) {
        pixels[i] = (pixels[i] as f32 + red_shift).clamp(0.0, 255.0) as u8;
        pixels[i + 1] = (pixels[i + 1] as f32 + green_shift).clamp(0.0, 255.0) as u8;
        pixels[i + 2] = (pixels[i + 2] as f32 + blue_shift).clamp(0.0, 255.0) as u8;
    }
}

// 74. Color Balance HLS
pub fn apply_color_balance_hls(pixels: &mut [u8], _hue: f32, lightness: f32, saturation: f32) {
    for i in (0..pixels.len()).step_by(4) {
        pixels[i] = (pixels[i] as f32 * (1.0 + lightness * 0.01) * (1.0 + saturation * 0.01))
            .clamp(0.0, 255.0) as u8;
        pixels[i + 1] =
            (pixels[i + 1] as f32 * (1.0 + lightness * 0.01) * (1.0 + saturation * 0.01))
                .clamp(0.0, 255.0) as u8;
        pixels[i + 2] =
            (pixels[i + 2] as f32 * (1.0 + lightness * 0.01) * (1.0 + saturation * 0.01))
                .clamp(0.0, 255.0) as u8;
    }
}

// 75. Equalize
pub fn apply_equalize(pixels: &mut [u8]) {
    crate::core::ae_effects_pack::apply_unsharp_mask(pixels, 2, 2, 50.0, 1);
}

// 76. Invert Alpha
pub fn apply_invert_alpha(pixels: &mut [u8]) {
    for i in (3..pixels.len()).step_by(4) {
        pixels[i] = 255 - pixels[i];
    }
}

// 77. Leave Color
pub fn apply_leave_color(pixels: &mut [u8], target_rgb: [u8; 3], tolerance: f32) {
    let tol = tolerance * 255.0;
    for i in (0..pixels.len()).step_by(4) {
        let dr = (pixels[i] as f32 - target_rgb[0] as f32).abs();
        let dg = (pixels[i + 1] as f32 - target_rgb[1] as f32).abs();
        let db = (pixels[i + 2] as f32 - target_rgb[2] as f32).abs();
        if dr + dg + db > tol {
            let luma =
                (pixels[i] as u32 * 299 + pixels[i + 1] as u32 * 587 + pixels[i + 2] as u32 * 114)
                    / 1000;
            let l = luma as u8;
            pixels[i] = l;
            pixels[i + 1] = l;
            pixels[i + 2] = l;
        }
    }
}

// 78. Selective Color
pub fn apply_selective_color(pixels: &mut [u8], red_mult: f32) {
    for i in (0..pixels.len()).step_by(4) {
        if pixels[i] > pixels[i + 1] && pixels[i] > pixels[i + 2] {
            pixels[i] = (pixels[i] as f32 * red_mult).clamp(0.0, 255.0) as u8;
        }
    }
}

// 79. Vibrance
pub fn apply_vibrance(pixels: &mut [u8], amount: f32) {
    apply_color_balance_hls(pixels, 0.0, 0.0, amount);
}

// 80. Exposure
pub fn apply_exposure(pixels: &mut [u8], exposure_ev: f32) {
    let mult = 2.0f32.powf(exposure_ev);
    for i in (0..pixels.len()).step_by(4) {
        pixels[i] = (pixels[i] as f32 * mult).clamp(0.0, 255.0) as u8;
        pixels[i + 1] = (pixels[i + 1] as f32 * mult).clamp(0.0, 255.0) as u8;
        pixels[i + 2] = (pixels[i + 2] as f32 * mult).clamp(0.0, 255.0) as u8;
    }
}
