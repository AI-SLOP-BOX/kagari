struct Globals {
    viewport_size: vec2<f32>,
    exposure_ev: f32,
    lut_mode: u32,
    shadow_enabled: u32,
    shadow_strength: f32,
    channel_mode: u32,
    _pad_globals: u32,
};

struct Layer {
    transform_matrix: mat4x4<f32>,

    color_r: f32,
    color_g: f32,
    color_b: f32,
    color_a: f32,

    opacity: f32,
    layer_type: u32,
    shape_type: u32,

    effect_tint_enabled: u32,
    effect_tint_color_r: f32,
    effect_tint_color_g: f32,
    effect_tint_color_b: f32,
    effect_tint_color_a: f32,
    effect_tint_intensity: f32,
    effect_blur_enabled: u32,
    effect_blur_radius: f32,

    effect_shadow_enabled: u32,
    effect_shadow_color_r: f32,
    effect_shadow_color_g: f32,
    effect_shadow_color_b: f32,
    effect_shadow_color_a: f32,
    effect_shadow_opacity: f32,
    effect_shadow_direction: f32,
    effect_shadow_distance: f32,
    effect_shadow_softness: f32,

    effect_ca_enabled: u32,
    effect_ca_shift_r: f32,
    effect_ca_shift_b: f32,
    effect_ca_edge_falloff: f32,

    effect_vignette_enabled: u32,
    effect_vignette_intensity: f32,
    effect_vignette_roundness: f32,
    effect_vignette_feather: f32,
    effect_vignette_color_r: f32,
    effect_vignette_color_g: f32,
    effect_vignette_color_b: f32,
    effect_vignette_color_a: f32,
    blend_mode: u32,

    levels_enabled: u32,
    levels_in_black: f32,
    levels_in_white: f32,
    levels_gamma: f32,
    levels_out_black: f32,
    levels_out_white: f32,

    huesat_enabled: u32,
    huesat_hue: f32,
    huesat_sat: f32,
    huesat_light: f32,

    glow_enabled: u32,
    glow_threshold: f32,
    glow_radius: f32,
    glow_intensity: f32,
    glow_color_r: f32,
    glow_color_g: f32,
    glow_color_b: f32,
    glow_color_a: f32,

    grain_enabled: u32,
    grain_intensity: f32,
    grain_size: f32,

    track_matte_mode: u32,

    shape_params_x: f32,
    shape_params_y: f32,
    shape_params_z: f32,
    shape_params_w: f32,

    meshwarp_enabled: u32,
    corner_top_left_x: f32,
    corner_top_left_y: f32,
    corner_top_right_x: f32,
    corner_top_right_y: f32,
    corner_bottom_left_x: f32,
    corner_bottom_left_y: f32,
    corner_bottom_right_x: f32,
    corner_bottom_right_y: f32,

    motionblur_enabled: u32,
    motionblur_shutter: f32,
    motionblur_velocity_x: f32,
    motionblur_velocity_y: f32,
    motionblur_samples: u32,

    flare_enabled: u32,
    flare_pos_x: f32,
    flare_pos_y: f32,
    flare_intensity: f32,
    flare_threshold: f32,
    flare_color_r: f32,
    flare_color_g: f32,
    flare_color_b: f32,
    flare_color_a: f32,

    mask_enabled: u32,
    mask_mode: u32,
    mask_inverted: u32,
    mask_feather: f32,

    trim_start: f32,
    trim_end: f32,
    trim_offset: f32,
    _pad_trim: f32,

    fill_type: u32,
    grad_start_x: f32,
    grad_start_y: f32,
    grad_end_x: f32,
    grad_end_y: f32,
    grad_color1_r: f32,
    grad_color1_g: f32,
    grad_color1_b: f32,
    grad_color1_a: f32,
    grad_color2_r: f32,
    grad_color2_g: f32,
    grad_color2_b: f32,
    grad_color2_a: f32,
    grad_center_x: f32,
    grad_center_y: f32,
    grad_radius: f32,
    _grad_pad: f32,

    ls_stroke_width: f32,
    ls_stroke_r: f32,
    ls_stroke_g: f32,
    ls_stroke_b: f32,
    ls_color_overlay_r: f32,
    ls_color_overlay_g: f32,
    ls_color_overlay_b: f32,
    ls_color_overlay_a: f32,
    ls_gradient_start_x: f32,
    ls_gradient_start_y: f32,
    ls_gradient_end_x: f32,
    ls_gradient_end_y: f32,
    ls_gradient_color1_r: f32,
    ls_gradient_color1_g: f32,
    ls_gradient_color1_b: f32,
    ls_gradient_color1_a: f32,
    ls_gradient_color2_r: f32,
    ls_gradient_color2_g: f32,
    ls_gradient_color2_b: f32,
    ls_gradient_color2_a: f32,
    ls_inner_shadow_offset_x: f32,
    ls_inner_shadow_offset_y: f32,
    ls_inner_shadow_size: f32,
    ls_inner_shadow_opacity: f32,
    ls_inner_shadow_r: f32,
    ls_inner_shadow_g: f32,
    ls_inner_shadow_b: f32,
    ls_bevel_size: f32,
    ls_bevel_angle: f32,
    ls_bevel_strength: f32,
    ls_bevel_light_r: f32,
    ls_bevel_light_g: f32,
    ls_bevel_light_b: f32,
    ls_style_flags: u32,
    _ls_pad1: f32,
    _ls_pad2: f32,
    _ls_pad3: f32,

    extrusion_depth: f32,
    bevel_depth: f32,

    chromatic_enabled: u32,
    chromatic_amount: f32,
    chromatic_angle: f32,
    _pad_chromatic: f32,

    vignette_enabled: u32,
    vignette_amount: f32,
    vignette_midpoint: f32,
    vignette_feather: f32,

    invert_enabled: u32,
    posterize_enabled: u32,
    posterize_levels: f32,
    threshold_level: f32,

    tint_enabled: u32,
    tint_amount: f32,
    _pad_tint1: f32,
    _pad_tint2: f32,
    tint_black_r: f32,
    tint_black_g: f32,
    tint_black_b: f32,
    tint_black_a: f32,
    tint_white_r: f32,
    tint_white_g: f32,
    tint_white_b: f32,
    tint_white_a: f32,

    crt_enabled: u32,
    crt_scanline_count: f32,
    crt_scanline_intensity: f32,
    crt_curvature: f32,

    sim_enabled: u32,
    sim_type: u32,
    sim_p1: f32,
    sim_p2: f32,
    sim_p3: f32,
    sim_p4: f32,
    sim_p5: f32,
    sim_p6: f32,

    three_d_shade_r: f32,
    three_d_shade_g: f32,
    three_d_shade_b: f32,
    three_d_lighting_enabled: u32,
    accepts_shadows: u32,
    _ls_pad5: f32,
    video_frame_blend: f32,
    _video_frame_blend_pad0: f32,
    _video_frame_blend_pad1: f32,
    _video_frame_blend_pad2: f32,
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var<uniform> layer: Layer;
@group(2) @binding(0) var t_diffuse: texture_2d<f32>;
@group(2) @binding(1) var s_diffuse: sampler;
@group(2) @binding(2) var t_diffuse_next: texture_2d<f32>;
@group(2) @binding(3) var s_diffuse_next: sampler;
@group(3) @binding(0) var t_mask: texture_2d<f32>;
@group(3) @binding(1) var s_mask: sampler;
@group(3) @binding(2) var t_shadow: texture_2d<f32>;
@group(3) @binding(3) var s_shadow: sampler;
@group(3) @binding(4) var t_matte: texture_2d<f32>;
@group(3) @binding(5) var s_matte: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) local_pos: vec2<f32>,
};

@vertex
