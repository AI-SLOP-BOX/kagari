use super::super::{global, gpu_effects_enabled};

pub fn try_gpu_color_balance(p: &mut [u8], w: u32, h: u32, shift: [f32; 3]) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_color_balance(p, w, h, shift))
        .unwrap_or(false)
}
pub fn try_gpu_vibrance(p: &mut [u8], w: u32, h: u32, amount: f32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_vibrance(p, w, h, amount))
        .unwrap_or(false)
}
pub fn try_gpu_white_balance(p: &mut [u8], w: u32, h: u32, temp: f32, tint: f32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_white_balance(p, w, h, temp, tint))
        .unwrap_or(false)
}
pub fn try_gpu_hsl_adjust(p: &mut [u8], w: u32, h: u32, hue: f32, sat: f32, light: f32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_hsl_adjust(p, w, h, hue, sat, light))
        .unwrap_or(false)
}
pub fn try_gpu_crt_scanlines(p: &mut [u8], w: u32, h: u32, spacing: f32, intensity: f32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_crt_scanlines(p, w, h, spacing, intensity))
        .unwrap_or(false)
}
pub fn try_gpu_alpha_from_luminance(p: &mut [u8], w: u32, h: u32, invert: bool) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_alpha_from_luminance(p, w, h, invert))
        .unwrap_or(false)
}
pub fn try_gpu_luma_key_range(
    p: &mut [u8],
    w: u32,
    h: u32,
    low: f32,
    high: f32,
    invert: bool,
) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_luma_key_range(p, w, h, low, high, invert))
        .unwrap_or(false)
}
pub fn try_gpu_venetian_blinds(p: &mut [u8], w: u32, h: u32, completion: f32, width: f32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_venetian_blinds(p, w, h, completion, width))
        .unwrap_or(false)
}
pub fn try_gpu_tritone(
    p: &mut [u8],
    w: u32,
    h: u32,
    shadow: [f32; 3],
    mid: [f32; 3],
    highlight: [f32; 3],
) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_tritone(p, w, h, shadow, mid, highlight))
        .unwrap_or(false)
}
pub fn try_gpu_gradient_map(
    p: &mut [u8],
    w: u32,
    h: u32,
    low: [f32; 3],
    mid: [f32; 3],
    high: [f32; 3],
) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_gradient_map(p, w, h, low, mid, high))
        .unwrap_or(false)
}
pub fn try_gpu_letterbox(p: &mut [u8], w: u32, h: u32, frac: f32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_letterbox(p, w, h, frac))
        .unwrap_or(false)
}
pub fn try_gpu_cc_lens(p: &mut [u8], w: u32, h: u32, convergence: f32, zoom: f32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_cc_lens(p, w, h, convergence, zoom))
        .unwrap_or(false)
}
pub fn try_gpu_polar_coords(p: &mut [u8], w: u32, h: u32, to_polar: bool) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_polar_coords(p, w, h, to_polar))
        .unwrap_or(false)
}
pub fn try_gpu_optics_comp(p: &mut [u8], w: u32, h: u32, fov: f32, reverse: bool) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_optics_comp(p, w, h, fov, reverse))
        .unwrap_or(false)
}
pub fn try_gpu_fisheye(p: &mut [u8], w: u32, h: u32, strength: f32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_fisheye(p, w, h, strength))
        .unwrap_or(false)
}
pub fn try_gpu_lens_correction(p: &mut [u8], w: u32, h: u32, k1: f32, k2: f32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_lens_correction(p, w, h, k1, k2))
        .unwrap_or(false)
}
pub fn try_gpu_vortex(p: &mut [u8], w: u32, h: u32, angle: f32, radius: f32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_vortex(p, w, h, angle, radius))
        .unwrap_or(false)
}
pub fn try_gpu_pinch_punch(p: &mut [u8], w: u32, h: u32, strength: f32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_pinch_punch(p, w, h, strength))
        .unwrap_or(false)
}
pub fn try_gpu_refraction(p: &mut [u8], w: u32, h: u32, ior: f32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_refraction(p, w, h, ior))
        .unwrap_or(false)
}
pub fn try_gpu_bend_it(p: &mut [u8], w: u32, h: u32, top: f32, bottom: f32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_bend_it(p, w, h, top, bottom))
        .unwrap_or(false)
}
pub fn try_gpu_tiler(p: &mut [u8], w: u32, h: u32, scale: f32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_tiler(p, w, h, scale))
        .unwrap_or(false)
}
pub fn try_gpu_directional_sharpen(
    p: &mut [u8],
    w: u32,
    h: u32,
    angle: f32,
    strength: f32,
) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_directional_sharpen(p, w, h, angle, strength))
        .unwrap_or(false)
}
