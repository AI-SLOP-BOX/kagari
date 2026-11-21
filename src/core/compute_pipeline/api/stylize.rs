use super::super::{global, gpu_effects_enabled};

pub fn try_gpu_halftone(p: &mut [u8], w: u32, h: u32, cell_size: f32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_halftone(p, w, h, cell_size))
        .unwrap_or(false)
}
pub fn try_gpu_mosaic(p: &mut [u8], w: u32, h: u32, bw: f32, bh: f32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_mosaic(p, w, h, bw, bh))
        .unwrap_or(false)
}
pub fn try_gpu_crosshatch(p: &mut [u8], w: u32, h: u32, gap: f32, threshold: f32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_crosshatch(p, w, h, gap, threshold))
        .unwrap_or(false)
}
pub fn try_gpu_colorama(
    p: &mut [u8],
    w: u32,
    h: u32,
    hue_offset: f32,
    sat: f32,
    light: f32,
) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_colorama(p, w, h, hue_offset, sat, light))
        .unwrap_or(false)
}
pub fn try_gpu_bevel_alpha(p: &mut [u8], w: u32, h: u32, depth: f32, angle: f32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_bevel_alpha(p, w, h, depth, angle))
        .unwrap_or(false)
}
pub fn try_gpu_chroma_key(
    p: &mut [u8],
    w: u32,
    h: u32,
    key_rgb: [f32; 3],
    gain: f32,
    softness: f32,
) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_chroma_key(p, w, h, key_rgb, gain, softness))
        .unwrap_or(false)
}
pub fn try_gpu_color_space_convert(p: &mut [u8], w: u32, h: u32, mode: u32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_color_space_convert(p, w, h, mode))
        .unwrap_or(false)
}
pub fn try_gpu_film_grain(p: &mut [u8], w: u32, h: u32, intensity: f32, seed: f32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_film_grain(p, w, h, intensity, seed))
        .unwrap_or(false)
}
pub fn try_gpu_fractal_noise(
    p: &mut [u8],
    w: u32,
    h: u32,
    scale: f32,
    evolution: f32,
    blend: f32,
) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_fractal_noise(p, w, h, scale, evolution, blend))
        .unwrap_or(false)
}
pub fn try_gpu_glitch_displacement(p: &mut [u8], w: u32, h: u32, amount: f32, seed: f32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_glitch_displacement(p, w, h, amount, seed))
        .unwrap_or(false)
}
pub fn try_gpu_scanline_glitch(p: &mut [u8], w: u32, h: u32, amount: f32, seed: f32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_scanline_glitch(p, w, h, amount, seed))
        .unwrap_or(false)
}
pub fn try_gpu_reflection_map(p: &mut [u8], w: u32, h: u32, reflect_y: bool, fade: f32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_reflection_map(p, w, h, reflect_y, fade))
        .unwrap_or(false)
}
