use super::super::{global, gpu_effects_enabled};

/// Try running a GPU gaussian blur with the global context.
/// Returns false when disabled, unavailable, or unsupported size —
/// callers must fall back to the CPU implementation in that case.
pub fn try_gpu_gaussian_blur(pixels: &mut [u8], width: u32, height: u32, radius: u32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    match global() {
        Some(ctx) => ctx.gaussian_blur(pixels, width, height, radius),
        None => false,
    }
}

/// Try running a GPU directional (motion) blur with the global context.
pub fn try_gpu_directional_blur(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    length_px: u32,
    angle_deg: f32,
) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    match global() {
        Some(ctx) => ctx.directional_blur(pixels, width, height, length_px, angle_deg),
        None => false,
    }
}

/// Try running a GPU radial (zoom) blur with the global context.
pub fn try_gpu_radial_blur(pixels: &mut [u8], width: u32, height: u32, taps: u32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.radial_blur(pixels, width, height, taps))
        .unwrap_or(false)
}

pub fn try_gpu_color_correct(
    pixels: &mut [u8],
    w: u32,
    h: u32,
    br: f32,
    ct: f32,
    sat: f32,
    hue: f32,
) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_color_correct(pixels, w, h, br, ct, sat, hue))
        .unwrap_or(false)
}
pub fn try_gpu_sharpen(pixels: &mut [u8], w: u32, h: u32, strength: f32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_sharpen(pixels, w, h, strength))
        .unwrap_or(false)
}
pub fn try_gpu_threshold(pixels: &mut [u8], w: u32, h: u32, cutoff: f32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_threshold(pixels, w, h, cutoff))
        .unwrap_or(false)
}
pub fn try_gpu_emboss(pixels: &mut [u8], w: u32, h: u32, strength: f32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_emboss(pixels, w, h, strength))
        .unwrap_or(false)
}
pub fn try_gpu_edge_detect(pixels: &mut [u8], w: u32, h: u32, blend: f32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_edge_detect(pixels, w, h, blend))
        .unwrap_or(false)
}
pub fn try_gpu_invert(pixels: &mut [u8], w: u32, h: u32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_invert(pixels, w, h))
        .unwrap_or(false)
}
pub fn try_gpu_solarize(pixels: &mut [u8], w: u32, h: u32, threshold: f32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_solarize(pixels, w, h, threshold))
        .unwrap_or(false)
}
pub fn try_gpu_posterize(pixels: &mut [u8], w: u32, h: u32, levels: f32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_posterize(pixels, w, h, levels))
        .unwrap_or(false)
}
pub fn try_gpu_color_tint(
    pixels: &mut [u8],
    w: u32,
    h: u32,
    tint_rgb: [f32; 3],
    intensity: f32,
) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_color_tint(pixels, w, h, tint_rgb, intensity))
        .unwrap_or(false)
}
pub fn try_gpu_drop_shadow(
    pixels: &mut [u8],
    w: u32,
    h: u32,
    color: [f32; 4],
    distance: f32,
    angle: f32,
    blur_r: u32,
) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_drop_shadow(pixels, w, h, color, distance, angle, blur_r))
        .unwrap_or(false)
}
pub fn try_gpu_glow(
    pixels: &mut [u8],
    w: u32,
    h: u32,
    threshold: f32,
    radius: u32,
    intensity: f32,
) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_glow(pixels, w, h, threshold, radius, intensity))
        .unwrap_or(false)
}
#[allow(clippy::too_many_arguments)]
pub fn try_gpu_levels(
    pixels: &mut [u8],
    w: u32,
    h: u32,
    in_black: f32,
    in_white: f32,
    gamma: f32,
    out_black: f32,
    out_white: f32,
) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| {
            c.gpu_levels(
                pixels, w, h, in_black, in_white, gamma, out_black, out_white,
            )
        })
        .unwrap_or(false)
}
pub fn try_gpu_hue_saturation(
    pixels: &mut [u8],
    w: u32,
    h: u32,
    hue_shift: f32,
    saturation: f32,
    lightness: f32,
) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_hue_saturation(pixels, w, h, hue_shift, saturation, lightness))
        .unwrap_or(false)
}
