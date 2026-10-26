/// The GPU-facing data that an effect plugin can read and modify.
/// Mirrors the relevant fields of LayerUniform without GPU-specific types.
#[derive(Debug, Clone, Copy)]
pub struct EffectParams {
    // Blur
    pub blur_enabled: u32,
    pub blur_radius: f32,

    // Tint
    pub tint_enabled: u32,
    pub tint_color: [f32; 4],
    pub tint_intensity: f32,

    // Drop Shadow
    pub shadow_enabled: u32,
    pub shadow_color: [f32; 4],
    pub shadow_opacity: f32,
    pub shadow_direction: f32,
    pub shadow_distance: f32,
    pub shadow_softness: f32,

    // Chromatic Aberration
    pub chromatic_enabled: u32,
    pub chromatic_shift_r: f32,
    pub chromatic_shift_b: f32,
    pub chromatic_edge_falloff: f32,

    // Vignette
    pub vignette_enabled: u32,
    pub vignette_intensity: f32,
    pub vignette_roundness: f32,
    pub vignette_feather: f32,
    pub vignette_color: [f32; 4],

    // Cinematic Color Grading (NextVFX integration)
    pub lut_enabled: u32,
    pub lut_intensity: f32,
    pub color_convert_mode: u32,

    // Physical Film Grain (NextVFX integration)
    pub grain_enabled: u32,
    pub grain_intensity: f32,
    pub grain_size: f32,

    // Levels Adjustment
    pub levels_enabled: u32,
    pub levels_in_black: f32,
    pub levels_in_white: f32,
    pub levels_gamma: f32,
    pub levels_out_black: f32,
    pub levels_out_white: f32,

    // Hue / Saturation / Lightness
    pub huesat_enabled: u32,
    pub huesat_hue: f32,
    pub huesat_sat: f32,
    pub huesat_light: f32,

    // Glow / Bloom
    pub glow_enabled: u32,
    pub glow_threshold: f32,
    pub glow_radius: f32,
    pub glow_intensity: f32,
    pub glow_color: [f32; 4],

    // Mesh Warp / Corner Pin
    pub meshwarp_enabled: u32,
    pub corner_top_left: [f32; 2],
    pub corner_top_right: [f32; 2],
    pub corner_bottom_left: [f32; 2],
    pub corner_bottom_right: [f32; 2],

    // Motion Blur
    pub motionblur_enabled: u32,
    pub motionblur_shutter: f32,
    pub motionblur_velocity_x: f32,
    pub motionblur_velocity_y: f32,
    pub motionblur_samples: u32,

    // Lens Flare
    pub flare_enabled: u32,
    pub flare_pos_x: f32,
    pub flare_pos_y: f32,
    pub flare_intensity: f32,
    pub flare_threshold: f32,
    pub flare_color: [f32; 4],

    // ── GPU Real-time Shader Effects ──
    pub chromatic_amount: f32,
    pub chromatic_angle: f32,

    pub vignette_amount: f32,
    pub vignette_midpoint: f32,

    pub invert_enabled: u32,
    pub posterize_enabled: u32,
    pub posterize_levels: f32,
    pub threshold_level: f32,

    pub tint_amount: f32,
    pub tint_black: [f32; 4],
    pub tint_white: [f32; 4],

    pub crt_enabled: u32,
    pub crt_scanline_count: f32,
    pub crt_scanline_intensity: f32,
    pub crt_curvature: f32,

    // ── GPU Simulation Effects ──
    pub sim_enabled: u32,
    pub sim_type: u32,
    pub sim_p1: f32,
    pub sim_p2: f32,
    pub sim_p3: f32,
    pub sim_p4: f32,
    pub sim_p5: f32,
    pub sim_p6: f32,
}

impl Default for EffectParams {
    fn default() -> Self {
        Self {
            blur_enabled: 0,
            blur_radius: 0.0,
            tint_enabled: 0,
            tint_color: [1.0; 4],
            tint_intensity: 0.0,
            shadow_enabled: 0,
            shadow_color: [0.0, 0.0, 0.0, 1.0],
            shadow_opacity: 0.0,
            shadow_direction: 135.0,
            shadow_distance: 5.0,
            shadow_softness: 5.0,
            chromatic_enabled: 0,
            chromatic_shift_r: 0.0,
            chromatic_shift_b: 0.0,
            chromatic_edge_falloff: 1.0,
            chromatic_amount: 0.0,
            chromatic_angle: 0.0,
            vignette_enabled: 0,
            vignette_intensity: 0.0,
            vignette_roundness: 1.0,
            vignette_feather: 50.0,
            vignette_amount: 0.0,
            vignette_midpoint: 0.5,
            vignette_color: [0.0, 0.0, 0.0, 1.0],
            lut_enabled: 0,
            lut_intensity: 0.0,
            color_convert_mode: 0,
            grain_enabled: 0,
            grain_intensity: 0.0,
            grain_size: 1.5,
            levels_enabled: 0,
            levels_in_black: 0.0,
            levels_in_white: 1.0,
            levels_gamma: 1.0,
            levels_out_black: 0.0,
            levels_out_white: 1.0,
            huesat_enabled: 0,
            huesat_hue: 0.0,
            huesat_sat: 1.0,
            huesat_light: 1.0,
            glow_enabled: 0,
            glow_threshold: 0.7,
            glow_radius: 10.0,
            glow_intensity: 0.0,
            glow_color: [1.0, 1.0, 1.0, 1.0],
            meshwarp_enabled: 0,
            corner_top_left: [0.0, 0.0],
            corner_top_right: [100.0, 0.0],
            corner_bottom_left: [0.0, 100.0],
            corner_bottom_right: [100.0, 100.0],
            motionblur_enabled: 0,
            motionblur_shutter: 0.5,
            motionblur_velocity_x: 0.0,
            motionblur_velocity_y: 0.0,
            motionblur_samples: 4,
            flare_enabled: 0,
            flare_pos_x: 0.5,
            flare_pos_y: 0.5,
            flare_intensity: 1.0,
            flare_threshold: 1.0,
            flare_color: [1.0, 0.9, 0.7, 1.0],
            invert_enabled: 0,
            posterize_enabled: 0,
            posterize_levels: 4.0,
            threshold_level: 0.5,
            tint_amount: 0.0,
            tint_black: [0.0, 0.0, 0.0, 1.0],
            tint_white: [1.0, 1.0, 1.0, 1.0],
            crt_enabled: 0,
            crt_scanline_count: 240.0,
            crt_scanline_intensity: 0.5,
            crt_curvature: 0.1,
            sim_enabled: 0,
            sim_type: 0,
            sim_p1: 0.0,
            sim_p2: 0.0,
            sim_p3: 0.0,
            sim_p4: 0.0,
            sim_p5: 0.0,
            sim_p6: 0.0,
        }
    }
}
