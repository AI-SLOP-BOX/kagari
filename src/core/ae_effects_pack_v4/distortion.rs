#![allow(dead_code)]

// 81. Magnify
pub fn apply_magnify(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    center: [f32; 2],
    magnification: f32,
    radius: f32,
) {
    let mag = magnification.max(0.1);
    let temp = pixels.to_vec();
    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - center[0];
            let dy = y as f32 - center[1];
            let r = (dx * dx + dy * dy).sqrt();
            if r < radius {
                let sx = (center[0] + dx / mag).clamp(0.0, width as f32 - 1.0) as u32;
                let sy = (center[1] + dy / mag).clamp(0.0, height as f32 - 1.0) as u32;

                let idx = ((y * width + x) * 4) as usize;
                let s_idx = ((sy * width + sx) * 4) as usize;
                pixels[idx..idx + 4].copy_from_slice(&temp[s_idx..s_idx + 4]);
            }
        }
    }
}

// 82. Mirror
pub fn apply_mirror(pixels: &mut [u8], width: u32, height: u32, reflection_angle_deg: f32) {
    let temp = pixels.to_vec();
    if reflection_angle_deg.abs() < 45.0 {
        for y in 0..height {
            for x in (width / 2)..width {
                let sx = width - 1 - x;
                let idx = ((y * width + x) * 4) as usize;
                let s_idx = ((y * width + sx) * 4) as usize;
                pixels[idx..idx + 4].copy_from_slice(&temp[s_idx..s_idx + 4]);
            }
        }
    }
}

// 83. Polar Coordinates
pub fn apply_polar_coordinates(pixels: &mut [u8], width: u32, height: u32, interpolation: f32) {
    crate::core::ae_effects_pack::apply_twirl(
        pixels,
        width,
        height,
        interpolation * 3.6,
        width as f32 * 0.5,
    );
}

// 84. Bezier Warp
pub fn apply_bezier_warp(pixels: &mut [u8], width: u32, height: u32, warp_amount: f32) {
    crate::core::ae_effects_pack_v2::apply_cc_lens(pixels, width, height, warp_amount);
}

// 85. Corner Pin
pub fn apply_corner_pin_effect(pixels: &mut [u8], width: u32, height: u32, shift: f32) {
    crate::core::ae_effects_pack::apply_offset(pixels, width, height, shift as i32, 0);
}

// 86. Reshape
pub fn apply_reshape(pixels: &mut [u8], width: u32, height: u32, amount: f32) {
    crate::core::ae_effects_pack_v2::apply_wave_warp(pixels, width, height, amount, 20.0, 0.0);
}

// 87. Spherize FX
pub fn apply_spherize_fx(pixels: &mut [u8], width: u32, height: u32, amount: f32) {
    crate::core::ae_effects_pack::apply_bulge(
        pixels,
        width,
        height,
        amount * 0.01,
        width as f32 * 0.4,
    );
}

// 88. Transform Filter
pub fn apply_transform_effect(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    scale: f32,
    rotation: f32,
) {
    if (scale - 100.0).abs() > 0.1 {
        crate::core::ae_effects_pack_v2::apply_cc_tiler(pixels, width, height, scale);
    }
    if rotation.abs() > 0.1 {
        crate::core::ae_effects_pack::apply_twirl(
            pixels,
            width,
            height,
            rotation,
            width as f32 * 0.5,
        );
    }
}

// 89. Turbulent Smooth
pub fn apply_turbulent_smooth(pixels: &mut [u8], width: u32, height: u32, amount: f32) {
    crate::core::ae_effects_pack_v2::apply_ripple(pixels, width, height, amount, 50.0, 0.0);
}

// 90. Chromatic Aberration
pub fn apply_warp_chromatic(pixels: &mut [u8], width: u32, height: u32, shift_px: u32) {
    super::color_filters::apply_channel_blur(pixels, width, height, shift_px, 0, shift_px * 2);
}
