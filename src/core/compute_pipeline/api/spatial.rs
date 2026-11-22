use super::super::{global, gpu_effects_enabled};

pub fn try_gpu_offset(pixels: &mut [u8], w: u32, h: u32, shift_x: i32, shift_y: i32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_offset(pixels, w, h, shift_x, shift_y))
        .unwrap_or(false)
}
pub fn try_gpu_twirl(pixels: &mut [u8], w: u32, h: u32, angle: f32, cx: f32, cy: f32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_twirl(pixels, w, h, angle, cx, cy))
        .unwrap_or(false)
}
pub fn try_gpu_bulge(pixels: &mut [u8], w: u32, h: u32, strength: f32, cx: f32, cy: f32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_bulge(pixels, w, h, strength, cx, cy))
        .unwrap_or(false)
}
pub fn try_gpu_spherize(
    pixels: &mut [u8],
    w: u32,
    h: u32,
    strength: f32,
    cx: f32,
    cy: f32,
) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_spherize(pixels, w, h, strength, cx, cy))
        .unwrap_or(false)
}
pub fn try_gpu_wave_warp(
    pixels: &mut [u8],
    w: u32,
    h: u32,
    amplitude: f32,
    frequency: f32,
    phase: f32,
    direction: f32,
) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_wave_warp(pixels, w, h, amplitude, frequency, phase, direction))
        .unwrap_or(false)
}
pub fn try_gpu_turbulent_displace(
    pixels: &mut [u8],
    w: u32,
    h: u32,
    amplitude: f32,
    scale: f32,
) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_turbulent_displace(pixels, w, h, amplitude, scale))
        .unwrap_or(false)
}
pub fn try_gpu_chromatic_aberration(pixels: &mut [u8], w: u32, h: u32, amount: f32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_chromatic_aberration(pixels, w, h, amount))
        .unwrap_or(false)
}
pub fn try_gpu_vignette(pixels: &mut [u8], w: u32, h: u32, radius: f32, softness: f32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_vignette(pixels, w, h, radius, softness))
        .unwrap_or(false)
}
pub fn try_gpu_minimax(pixels: &mut [u8], w: u32, h: u32, radius: u32, maximize: bool) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_minimax(pixels, w, h, radius, maximize))
        .unwrap_or(false)
}
pub fn try_gpu_linear_wipe(
    p: &mut [u8],
    w: u32,
    h: u32,
    completion: f32,
    angle: f32,
    feather: f32,
) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_linear_wipe(p, w, h, completion, angle, feather))
        .unwrap_or(false)
}
pub fn try_gpu_shift_channels(
    p: &mut [u8],
    w: u32,
    h: u32,
    r: u32,
    g: u32,
    b: u32,
    a: u32,
) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_shift_channels(p, w, h, r, g, b, a))
        .unwrap_or(false)
}
