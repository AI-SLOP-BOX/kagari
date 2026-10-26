#![allow(unused_variables)]

pub const MAX_BLUR_RADIUS: u32 = 64;

pub fn set_gpu_effects_enabled(_enabled: bool) {}

pub fn gpu_effects_enabled() -> bool {
    false
}

pub fn try_gpu_gaussian_blur(pixels: &mut [u8], width: u32, height: u32, radius: u32) -> bool {
    false
}

pub fn try_gpu_directional_blur(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    length_px: u32,
    angle_deg: f32,
) -> bool {
    false
}

pub fn try_gpu_radial_blur(pixels: &mut [u8], width: u32, height: u32, taps: u32) -> bool {
    false
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
    false
}

pub fn try_gpu_sharpen(pixels: &mut [u8], w: u32, h: u32, strength: f32) -> bool {
    false
}

pub fn try_gpu_threshold(pixels: &mut [u8], w: u32, h: u32, cutoff: f32) -> bool {
    false
}

pub fn try_gpu_emboss(pixels: &mut [u8], w: u32, h: u32, strength: f32) -> bool {
    false
}

pub fn try_gpu_edge_detect(pixels: &mut [u8], w: u32, h: u32, blend: f32) -> bool {
    false
}

pub fn try_gpu_invert(pixels: &mut [u8], w: u32, h: u32) -> bool {
    false
}

pub fn try_gpu_solarize(pixels: &mut [u8], w: u32, h: u32, threshold: f32) -> bool {
    false
}

pub fn try_gpu_posterize(pixels: &mut [u8], w: u32, h: u32, levels: f32) -> bool {
    false
}

pub fn try_gpu_color_tint(
    pixels: &mut [u8],
    w: u32,
    h: u32,
    tint_rgb: [f32; 3],
    intensity: f32,
) -> bool {
    false
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
    false
}

pub fn try_gpu_glow(
    pixels: &mut [u8],
    w: u32,
    h: u32,
    threshold: f32,
    radius: u32,
    intensity: f32,
) -> bool {
    false
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
    false
}

pub fn try_gpu_hue_saturation(
    pixels: &mut [u8],
    w: u32,
    h: u32,
    hue_shift: f32,
    saturation: f32,
    lightness: f32,
) -> bool {
    false
}

pub fn try_gpu_offset(pixels: &mut [u8], w: u32, h: u32, shift_x: i32, shift_y: i32) -> bool {
    false
}

pub fn try_gpu_twirl(pixels: &mut [u8], w: u32, h: u32, angle: f32, cx: f32, cy: f32) -> bool {
    false
}

pub fn try_gpu_bulge(pixels: &mut [u8], w: u32, h: u32, strength: f32, cx: f32, cy: f32) -> bool {
    false
}

pub fn try_gpu_spherize(
    pixels: &mut [u8],
    w: u32,
    h: u32,
    strength: f32,
    cx: f32,
    cy: f32,
) -> bool {
    false
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
    false
}

pub fn try_gpu_turbulent_displace(
    pixels: &mut [u8],
    w: u32,
    h: u32,
    amplitude: f32,
    scale: f32,
) -> bool {
    false
}

pub fn try_gpu_chromatic_aberration(pixels: &mut [u8], w: u32, h: u32, amount: f32) -> bool {
    false
}

pub fn try_gpu_vignette(pixels: &mut [u8], w: u32, h: u32, radius: f32, softness: f32) -> bool {
    false
}

pub fn try_gpu_minimax(pixels: &mut [u8], w: u32, h: u32, radius: u32, maximize: bool) -> bool {
    false
}

pub fn try_gpu_linear_wipe(
    p: &mut [u8],
    w: u32,
    h: u32,
    completion: f32,
    angle: f32,
    feather: f32,
) -> bool {
    false
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
    false
}

pub fn try_gpu_color_balance(p: &mut [u8], w: u32, h: u32, shift: [f32; 3]) -> bool {
    false
}

pub fn try_gpu_vibrance(p: &mut [u8], w: u32, h: u32, amount: f32) -> bool {
    false
}

pub fn try_gpu_white_balance(p: &mut [u8], w: u32, h: u32, temp: f32, tint: f32) -> bool {
    false
}

pub fn try_gpu_hsl_adjust(p: &mut [u8], w: u32, h: u32, hue: f32, sat: f32, light: f32) -> bool {
    false
}

pub fn try_gpu_crt_scanlines(p: &mut [u8], w: u32, h: u32, spacing: f32, intensity: f32) -> bool {
    false
}

pub fn try_gpu_alpha_from_luminance(p: &mut [u8], w: u32, h: u32, invert: bool) -> bool {
    false
}

pub fn try_gpu_luma_key_range(
    p: &mut [u8],
    w: u32,
    h: u32,
    low: f32,
    high: f32,
    invert: bool,
) -> bool {
    false
}

pub fn try_gpu_venetian_blinds(p: &mut [u8], w: u32, h: u32, completion: f32, width: f32) -> bool {
    false
}

pub fn try_gpu_tritone(
    p: &mut [u8],
    w: u32,
    h: u32,
    shadow: [f32; 3],
    mid: [f32; 3],
    highlight: [f32; 3],
) -> bool {
    false
}

pub fn try_gpu_gradient_map(
    p: &mut [u8],
    w: u32,
    h: u32,
    low: [f32; 3],
    mid: [f32; 3],
    high: [f32; 3],
) -> bool {
    false
}

pub fn try_gpu_letterbox(p: &mut [u8], w: u32, h: u32, frac: f32) -> bool {
    false
}

pub fn try_gpu_cc_lens(p: &mut [u8], w: u32, h: u32, convergence: f32, zoom: f32) -> bool {
    false
}

pub fn try_gpu_polar_coords(p: &mut [u8], w: u32, h: u32, to_polar: bool) -> bool {
    false
}

pub fn try_gpu_optics_comp(p: &mut [u8], w: u32, h: u32, fov: f32, reverse: bool) -> bool {
    false
}

pub fn try_gpu_fisheye(p: &mut [u8], w: u32, h: u32, strength: f32) -> bool {
    false
}

pub fn try_gpu_lens_correction(p: &mut [u8], w: u32, h: u32, k1: f32, k2: f32) -> bool {
    false
}

pub fn try_gpu_vortex(p: &mut [u8], w: u32, h: u32, angle: f32, radius: f32) -> bool {
    false
}

pub fn try_gpu_pinch_punch(p: &mut [u8], w: u32, h: u32, strength: f32) -> bool {
    false
}

pub fn try_gpu_refraction(p: &mut [u8], w: u32, h: u32, ior: f32) -> bool {
    false
}

pub fn try_gpu_bend_it(p: &mut [u8], w: u32, h: u32, top: f32, bottom: f32) -> bool {
    false
}

pub fn try_gpu_tiler(p: &mut [u8], w: u32, h: u32, scale: f32) -> bool {
    false
}

pub fn try_gpu_directional_sharpen(
    p: &mut [u8],
    w: u32,
    h: u32,
    angle: f32,
    strength: f32,
) -> bool {
    false
}

pub fn try_gpu_halftone(p: &mut [u8], w: u32, h: u32, cell_size: f32) -> bool {
    false
}

pub fn try_gpu_mosaic(p: &mut [u8], w: u32, h: u32, bw: f32, bh: f32) -> bool {
    false
}

pub fn try_gpu_crosshatch(p: &mut [u8], w: u32, h: u32, gap: f32, threshold: f32) -> bool {
    false
}

pub fn try_gpu_colorama(
    p: &mut [u8],
    w: u32,
    h: u32,
    hue_offset: f32,
    sat: f32,
    light: f32,
) -> bool {
    false
}

pub fn try_gpu_bevel_alpha(p: &mut [u8], w: u32, h: u32, depth: f32, angle: f32) -> bool {
    false
}

pub fn try_gpu_chroma_key(
    p: &mut [u8],
    w: u32,
    h: u32,
    key_rgb: [f32; 3],
    gain: f32,
    softness: f32,
) -> bool {
    false
}

pub fn try_gpu_color_space_convert(p: &mut [u8], w: u32, h: u32, mode: u32) -> bool {
    false
}

pub fn try_gpu_film_grain(p: &mut [u8], w: u32, h: u32, intensity: f32, seed: f32) -> bool {
    false
}

pub fn try_gpu_fractal_noise(
    p: &mut [u8],
    w: u32,
    h: u32,
    scale: f32,
    evolution: f32,
    blend: f32,
) -> bool {
    false
}

pub fn try_gpu_glitch_displacement(p: &mut [u8], w: u32, h: u32, amount: f32, seed: f32) -> bool {
    false
}

pub fn try_gpu_scanline_glitch(p: &mut [u8], w: u32, h: u32, amount: f32, seed: f32) -> bool {
    false
}

pub fn try_gpu_reflection_map(p: &mut [u8], w: u32, h: u32, reflect_y: bool, fade: f32) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_only_build_keeps_gpu_effects_disabled_and_uses_cpu_fallbacks() {
        set_gpu_effects_enabled(true);
        assert!(!gpu_effects_enabled());

        let mut pixels = vec![12, 34, 56, 255];
        assert!(!try_gpu_invert(&mut pixels, 1, 1));
        assert_eq!(pixels, [12, 34, 56, 255]);
        assert_eq!(MAX_BLUR_RADIUS, 64);
    }
}
