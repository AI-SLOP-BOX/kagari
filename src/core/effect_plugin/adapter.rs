use super::params::EffectParams;
use crate::core::timeline::{ColorConversionMode, EffectType};

/// The core trait every render effect plugin must implement.
pub trait RenderEffectPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn type_id(&self) -> &str;
    fn apply_to_params(&self, frame: u32, params: &mut EffectParams);
}

/// Adapts a serializable `EffectType` enum variant into the plugin trait.
pub struct EnumEffectPlugin {
    pub effect_type: EffectType,
}

impl RenderEffectPlugin for EnumEffectPlugin {
    fn name(&self) -> &str {
        match &self.effect_type {
            EffectType::GaussianBlur { .. } => "Gaussian Blur",
            EffectType::ColorTint { .. } => "Color Tint",
            EffectType::DropShadow { .. } => "Drop Shadow",
            EffectType::ChromaticAberration { .. } => "Chromatic Aberration",
            EffectType::Vignette { .. } => "Vignette",
            EffectType::Levels { .. } => "Levels",
            EffectType::HueSaturation { .. } => "Hue / Saturation",
            EffectType::Glow { .. } => "Glow",
            EffectType::MotionBlur { .. } => "Motion Blur",
            EffectType::MeshWarp { .. } => "Mesh Warp",
            EffectType::CornerPin { .. } => "Corner Pin",
            EffectType::ColorGradeLUT { .. } => "3D LUT Color Grading",
            EffectType::ColorSpaceConvert { .. } => "Color Space Converter",
            EffectType::FilmGrain { .. } => "Physical Film Grain",
            EffectType::Twirl { .. } => "Twirl",
            EffectType::Bulge { .. } => "Bulge",
            EffectType::Posterize { .. } => "Posterize",
            EffectType::Invert { .. } => "Invert",
            EffectType::Offset { .. } => "Offset",
            EffectType::DirectionalBlur { .. } => "Directional Blur",
            EffectType::RadialBlur { .. } => "Radial Blur",
            EffectType::Sharpen { .. } => "Sharpen",
            EffectType::Threshold { .. } => "Threshold",
            EffectType::LinearWipe { .. } => "Linear Wipe",
            EffectType::SimpleChoker { .. } => "Simple Choker",
            EffectType::ChromaKey { .. } => "Chroma Key",
            EffectType::Spherize { .. } => "Spherize",
            EffectType::TurbulentDisplace { .. } => "Turbulent Displace",
            EffectType::Colorama { .. } => "Colorama",
            EffectType::FractalNoise { .. } => "Fractal Noise",
            EffectType::Curves { .. } => "Curves",
            EffectType::DisplacementMap { .. } => "Displacement Map",
            EffectType::CompoundBlur { .. } => "Compound Blur",
            EffectType::Minimax { .. } => "Minimax",
            EffectType::ShiftChannels { .. } => "Shift Channels",
            EffectType::WaveWarp { .. } => "Wave Warp",
            EffectType::CcLens { .. } => "CC Lens",
            EffectType::PolarCoordinates { .. } => "Polar Coordinates",
            EffectType::OpticsCompensation { .. } => "Optics Compensation",
            EffectType::ColorBalance { .. } => "Color Balance",
            EffectType::ChannelMixer { .. } => "Channel Mixer",
            EffectType::LightSweep { .. } => "CC Light Sweep",
            EffectType::RadialFastBlur { .. } => "CC Radial Fast Blur",
            EffectType::BendIt { .. } => "CC Bend It",
            EffectType::Tiler { .. } => "CC Tiler",
            EffectType::Tritone { .. } => "Tritone",
            EffectType::MatteChoker { .. } => "Matte Choker",
            EffectType::VenetianBlinds { .. } => "Venetian Blinds",
            EffectType::Vibrance { .. } => "Vibrance",
            EffectType::WhiteBalance { .. } => "White Balance",
            EffectType::HslAdjust { .. } => "HSL Adjust",
            EffectType::GlowPro { .. } => "Glow",
            EffectType::CrtScanlines { .. } => "CRT Scanlines",
            EffectType::Vortex { .. } => "Vortex Distortion",
            EffectType::HeatDistortion { .. } => "Heat Distortion",
            EffectType::RainRipples { .. } => "Rain Ripples",
            EffectType::Fisheye { .. } => "Fisheye",
            EffectType::LensCorrection { .. } => "Lens Correction",
            EffectType::GlitchDisplacement { .. } => "Glitch Displacement",
            EffectType::RGBSplit { .. } => "RGB Split",
            EffectType::Flicker { .. } => "Flicker",
            EffectType::BlockGlitch { .. } => "Block Glitch",
            EffectType::SliceTear { .. } => "Slice Tear",
            EffectType::MatteChokeSpread { .. } => "Matte Choke / Spread",
            EffectType::AlphaFeather { .. } => "Alpha Feather",
            EffectType::AlphaFromLuminance { .. } => "Alpha From Luminance",
            EffectType::NightVision { .. } => "Night Vision",
            EffectType::IrisWipe { .. } => "Iris Wipe",
            EffectType::RadialWipe { .. } => "Radial Wipe",
            EffectType::FilmEmulation { .. } => "Film Emulation",
            EffectType::GodRays { .. } => "God Rays",
            EffectType::RadialBlurZoom { .. } => "Zoom Blur",
            EffectType::MedianFilter { .. } => "Median Filter",
            EffectType::SobelEdges { .. } => "Sobel Edges",
            EffectType::Mosaic { .. } => "Mosaic",
            EffectType::TiltShift { .. } => "Tilt Shift",
            EffectType::Emboss { .. } => "Emboss",
            EffectType::StarField { .. } => "Star Field",
            EffectType::LightningArc { .. } => "Lightning",
            EffectType::LaserBeam { .. } => "Laser Beam",
            EffectType::FireAutomaton { .. } => "Fire",
            EffectType::LumaKeyRange { .. } => "Luma Key Range",
            EffectType::Halftone { .. } => "Halftone",
            EffectType::Solarize { .. } => "Solarize",
            EffectType::PixelSort { .. } => "Pixel Sort",
            EffectType::PinchPunch { .. } => "Pinch / Punch",
            EffectType::ScanlineGlitch { .. } => "Scanline Glitch",
            EffectType::GlassEdgeBevel { .. } => "Glass Edge Bevel",
            EffectType::DirectionalSharpen { .. } => "Directional Sharpen",
            EffectType::RefractionLens { .. } => "Refraction Lens",
            EffectType::GradientMap { .. } => "Gradient Map",
            EffectType::LightLeak { .. } => "Light Leak",
            EffectType::BevelAlpha { .. } => "Bevel Alpha 3D",
            EffectType::CrossHatch { .. } => "Cross Hatch",
            EffectType::CmykHalftone { .. } => "CMYK Halftone",
            EffectType::ReflectionMap { .. } => "Reflection Map",
            EffectType::PerlinFlow { .. } => "Perlin Flow Noise",
            EffectType::FbmTurbulence { .. } => "FBM Turbulence",
            EffectType::SliderControl { .. } => "Slider Control",
            EffectType::AngleControl { .. } => "Angle Control",
            EffectType::PointControl { .. } => "Point Control",
            EffectType::ColorControl { .. } => "Color Control",
            EffectType::CheckboxControl { .. } => "Checkbox Control",
            EffectType::DropdownControl { .. } => "Dropdown Control",
            EffectType::Point3DControl { .. } => "3D Point Control",
            EffectType::LensFlare { .. } => "Lens Flare",
            EffectType::AudioSpectrum { .. } => "Audio Spectrum",
            EffectType::Letterbox { .. } => "Letterbox (Cinema Bars)",
            EffectType::CustomShader { .. } => "Custom Shader (WGSL)",
            EffectType::MergePaths { .. } => "Merge Paths",
            EffectType::OffsetPath { .. } => "Offset Path",
            EffectType::BassTreble { .. } => "Bass & Treble",
            EffectType::Flanger { .. } => "Flanger",
            EffectType::Chorus { .. } => "Chorus",
            EffectType::ParametricEQ { .. } => "Parametric EQ",
            EffectType::OpticalFlares { .. } => "Optical Flares",
            EffectType::MotionTile { .. } => "Motion Tile",
            EffectType::PageTurn { .. } => "CC Page Turn",
            EffectType::SetMatte { .. } => "Set Matte",
            EffectType::Echo { .. } => "Echo",
            EffectType::FindEdges { .. } => "Find Edges",
            EffectType::Transform { .. } => "Transform",
            EffectType::CameraLensBlur { .. } => "Camera Lens Blur",
            EffectType::LinearColorKey { .. } => "Linear Color Key",
            EffectType::ChannelCombiner { .. } => "Channel Combiner",
        }
    }

    fn type_id(&self) -> &str {
        match &self.effect_type {
            EffectType::GaussianBlur { .. } => "gaussian_blur",
            EffectType::ColorTint { .. } => "color_tint",
            EffectType::DropShadow { .. } => "drop_shadow",
            EffectType::ChromaticAberration { .. } => "chromatic_aberration",
            EffectType::Vignette { .. } => "vignette",
            EffectType::Levels { .. } => "levels",
            EffectType::HueSaturation { .. } => "hue_saturation",
            EffectType::Glow { .. } => "glow",
            EffectType::MotionBlur { .. } => "motion_blur",
            EffectType::MeshWarp { .. } => "mesh_warp",
            EffectType::ColorGradeLUT { .. } => "color_grade_lut",
            EffectType::ColorSpaceConvert { .. } => "color_space_convert",
            EffectType::FilmGrain { .. } => "film_grain",
            EffectType::Twirl { .. } => "twirl",
            EffectType::Bulge { .. } => "bulge",
            EffectType::Posterize { .. } => "posterize",
            EffectType::Invert { .. } => "invert",
            EffectType::Offset { .. } => "offset",
            EffectType::DirectionalBlur { .. } => "directional_blur",
            EffectType::RadialBlur { .. } => "radial_blur",
            EffectType::Sharpen { .. } => "sharpen",
            EffectType::Threshold { .. } => "threshold",
            EffectType::LinearWipe { .. } => "linear_wipe",
            EffectType::SimpleChoker { .. } => "simple_choker",
            EffectType::ChromaKey { .. } => "chroma_key",
            EffectType::Spherize { .. } => "spherize",
            EffectType::CornerPin { .. } => "corner_pin",
            EffectType::TurbulentDisplace { .. } => "turbulent_displace",
            EffectType::Colorama { .. } => "colorama",
            EffectType::FractalNoise { .. } => "fractal_noise",
            EffectType::Curves { .. } => "curves",
            EffectType::DisplacementMap { .. } => "displacement_map",
            EffectType::CompoundBlur { .. } => "compound_blur",
            EffectType::Minimax { .. } => "minimax",
            EffectType::ShiftChannels { .. } => "shift_channels",
            EffectType::WaveWarp { .. } => "wave_warp",
            EffectType::CcLens { .. } => "cc_lens",
            EffectType::PolarCoordinates { .. } => "polar_coordinates",
            EffectType::OpticsCompensation { .. } => "optics_compensation",
            EffectType::ColorBalance { .. } => "color_balance",
            EffectType::ChannelMixer { .. } => "channel_mixer",
            EffectType::LightSweep { .. } => "light_sweep",
            EffectType::RadialFastBlur { .. } => "radial_fast_blur",
            EffectType::BendIt { .. } => "bend_it",
            EffectType::Tiler { .. } => "tiler",
            EffectType::Tritone { .. } => "tritone",
            EffectType::MatteChoker { .. } => "matte_choker",
            EffectType::VenetianBlinds { .. } => "venetian_blinds",
            EffectType::Vibrance { .. } => "vibrance",
            EffectType::WhiteBalance { .. } => "white_balance",
            EffectType::HslAdjust { .. } => "hsl_adjust",
            EffectType::GlowPro { .. } => "glow_pro",
            EffectType::CrtScanlines { .. } => "crt_scanlines",
            EffectType::Vortex { .. } => "vortex_distortion",
            EffectType::HeatDistortion { .. } => "heat_distortion",
            EffectType::RainRipples { .. } => "rain_ripples",
            EffectType::Fisheye { .. } => "fisheye",
            EffectType::LensCorrection { .. } => "lens_correction",
            EffectType::GlitchDisplacement { .. } => "glitch_displacement",
            EffectType::RGBSplit { .. } => "rgb_split",
            EffectType::Flicker { .. } => "flicker",
            EffectType::BlockGlitch { .. } => "block_glitch",
            EffectType::SliceTear { .. } => "slice_tear",
            EffectType::MatteChokeSpread { .. } => "matte_choke_spread",
            EffectType::AlphaFeather { .. } => "alpha_feather",
            EffectType::AlphaFromLuminance { .. } => "alpha_from_luminance",
            EffectType::NightVision { .. } => "night_vision",
            EffectType::IrisWipe { .. } => "iris_wipe",
            EffectType::RadialWipe { .. } => "radial_wipe",
            EffectType::FilmEmulation { .. } => "film_emulation",
            EffectType::GodRays { .. } => "god_rays",
            EffectType::RadialBlurZoom { .. } => "radial_blur_zoom",
            EffectType::MedianFilter { .. } => "median_filter",
            EffectType::SobelEdges { .. } => "sobel_edges",
            EffectType::Mosaic { .. } => "mosaic",
            EffectType::TiltShift { .. } => "tilt_shift",
            EffectType::Emboss { .. } => "emboss",
            EffectType::StarField { .. } => "star_field",
            EffectType::LightningArc { .. } => "lightning_arc",
            EffectType::LaserBeam { .. } => "laser_beam",
            EffectType::FireAutomaton { .. } => "fire_automaton",
            EffectType::LumaKeyRange { .. } => "luma_key_range",
            EffectType::Halftone { .. } => "halftone",
            EffectType::Solarize { .. } => "solarize",
            EffectType::PixelSort { .. } => "pixel_sort",
            EffectType::PinchPunch { .. } => "pinch_punch",
            EffectType::ScanlineGlitch { .. } => "scanline_glitch",
            EffectType::GlassEdgeBevel { .. } => "glass_edge_bevel",
            EffectType::DirectionalSharpen { .. } => "directional_sharpen",
            EffectType::RefractionLens { .. } => "refraction_lens",
            EffectType::GradientMap { .. } => "gradient_map",
            EffectType::LightLeak { .. } => "light_leak",
            EffectType::BevelAlpha { .. } => "bevel_alpha",
            EffectType::CrossHatch { .. } => "cross_hatch",
            EffectType::CmykHalftone { .. } => "cmyk_halftone",
            EffectType::ReflectionMap { .. } => "reflection_map",
            EffectType::PerlinFlow { .. } => "perlin_flow",
            EffectType::FbmTurbulence { .. } => "fbm_turbulence",
            EffectType::SliderControl { .. } => "slider_control",
            EffectType::AngleControl { .. } => "angle_control",
            EffectType::PointControl { .. } => "point_control",
            EffectType::ColorControl { .. } => "color_control",
            EffectType::CheckboxControl { .. } => "checkbox_control",
            EffectType::DropdownControl { .. } => "dropdown_control",
            EffectType::Point3DControl { .. } => "point3d_control",
            EffectType::LensFlare { .. } => "lens_flare",
            EffectType::AudioSpectrum { .. } => "audio_spectrum",
            EffectType::Letterbox { .. } => "letterbox",
            EffectType::CustomShader { .. } => "custom_shader",
            EffectType::MergePaths { .. } => "merge_paths",
            EffectType::OffsetPath { .. } => "offset_path",
            EffectType::BassTreble { .. } => "bass_treble",
            EffectType::Flanger { .. } => "flanger",
            EffectType::Chorus { .. } => "chorus",
            EffectType::ParametricEQ { .. } => "parametric_eq",
            EffectType::OpticalFlares { .. } => "optical_flares",
            EffectType::MotionTile { .. } => "motion_tile",
            EffectType::PageTurn { .. } => "page_turn",
            EffectType::SetMatte { .. } => "set_matte",
            EffectType::Echo { .. } => "echo",
            EffectType::FindEdges { .. } => "find_edges",
            EffectType::Transform { .. } => "transform",
            EffectType::CameraLensBlur { .. } => "camera_lens_blur",
            EffectType::LinearColorKey { .. } => "linear_color_key",
            EffectType::ChannelCombiner { .. } => "channel_combiner",
        }
    }

    fn apply_to_params(&self, frame: u32, params: &mut EffectParams) {
        match &self.effect_type {
            EffectType::GaussianBlur { blur_radius } => {
                params.blur_enabled = 1;
                params.blur_radius = blur_radius.evaluate(frame);
            }
            EffectType::ColorTint { color, intensity } => {
                params.tint_enabled = 1;
                params.tint_color = color.evaluate(frame);
                params.tint_intensity = intensity.evaluate(frame) / 100.0;
            }
            EffectType::DropShadow {
                color,
                opacity,
                direction,
                distance,
                softness,
            } => {
                params.shadow_enabled = 1;
                params.shadow_color = color.evaluate(frame);
                params.shadow_opacity = opacity.evaluate(frame) / 100.0;
                params.shadow_direction = direction.evaluate(frame);
                params.shadow_distance = distance.evaluate(frame);
                params.shadow_softness = softness.evaluate(frame);
            }
            EffectType::ChromaticAberration {
                shift_r,
                shift_b,
                edge_falloff,
                iris_linked: _,
            } => {
                params.chromatic_enabled = 1;
                params.chromatic_shift_r = shift_r.evaluate(frame);
                params.chromatic_shift_b = shift_b.evaluate(frame);
                params.chromatic_edge_falloff = edge_falloff.evaluate(frame);
            }
            EffectType::Vignette {
                intensity,
                roundness,
                feather,
                color,
            } => {
                params.vignette_enabled = 1;
                params.vignette_intensity = intensity.evaluate(frame) / 100.0;
                params.vignette_roundness = roundness.evaluate(frame);
                params.vignette_feather = feather.evaluate(frame) / 100.0;
                params.vignette_color = color.evaluate(frame);
            }
            EffectType::ColorGradeLUT { intensity, .. } => {
                params.lut_enabled = 1;
                params.lut_intensity = intensity.evaluate(frame) / 100.0;
            }
            EffectType::ColorSpaceConvert { mode } => {
                let mode_val = match mode {
                    ColorConversionMode::LogCToLinear => 1,
                    ColorConversionMode::LinearToLogC => 2,
                    ColorConversionMode::SLog3ToLinear => 3,
                    ColorConversionMode::LinearToSLog3 => 4,
                };
                params.color_convert_mode = mode_val;
            }
            EffectType::FilmGrain {
                intensity,
                grain_size,
                ..
            } => {
                params.grain_enabled = 1;
                params.grain_intensity = intensity.evaluate(frame) / 100.0;
                params.grain_size = *grain_size;
            }
            EffectType::Levels {
                input_black,
                input_white,
                gamma,
                output_black,
                output_white,
            } => {
                params.levels_enabled = 1;
                params.levels_in_black = input_black.evaluate(frame);
                params.levels_in_white = input_white.evaluate(frame);
                params.levels_gamma = gamma.evaluate(frame);
                params.levels_out_black = output_black.evaluate(frame);
                params.levels_out_white = output_white.evaluate(frame);
            }
            EffectType::HueSaturation {
                hue_shift,
                saturation,
                lightness,
            } => {
                params.huesat_enabled = 1;
                params.huesat_hue = hue_shift.evaluate(frame);
                params.huesat_sat = 1.0 + (saturation.evaluate(frame) / 100.0);
                params.huesat_light = 1.0 + (lightness.evaluate(frame) / 100.0);
            }
            EffectType::Glow {
                threshold,
                radius,
                intensity,
                color,
            } => {
                params.glow_enabled = 1;
                params.glow_threshold = threshold.evaluate(frame) / 100.0;
                params.glow_radius = radius.evaluate(frame);
                params.glow_intensity = intensity.evaluate(frame) / 100.0;
                params.glow_color = color.evaluate(frame);
            }
            EffectType::GlowPro {
                threshold,
                radius,
                intensity,
            } => {
                params.glow_enabled = 1;
                params.glow_threshold = threshold.evaluate(frame).clamp(0.0, 1.0);
                params.glow_radius = radius.evaluate(frame);
                params.glow_intensity = (intensity.evaluate(frame) / 4.0).clamp(0.0, 1.0);
            }
            EffectType::MeshWarp {
                top_left,
                top_right,
                bottom_left,
                bottom_right,
            } => {
                params.meshwarp_enabled = 1;
                params.corner_top_left = top_left.evaluate(frame);
                params.corner_top_right = top_right.evaluate(frame);
                params.corner_bottom_left = bottom_left.evaluate(frame);
                params.corner_bottom_right = bottom_right.evaluate(frame);
            }
            EffectType::MotionBlur {
                shutter_angle,
                samples,
            } => {
                params.motionblur_enabled = 1;
                params.motionblur_shutter = shutter_angle.evaluate(frame) / 360.0;
                params.motionblur_samples = *samples;
            }
            EffectType::LensFlare {
                enabled,
                position_x,
                position_y,
                intensity,
                threshold,
                color,
                ..
            } => {
                params.flare_enabled = if enabled.evaluate(frame) > 0.5 { 1 } else { 0 };
                params.flare_pos_x = position_x.evaluate(frame);
                params.flare_pos_y = position_y.evaluate(frame);
                params.flare_intensity = intensity.evaluate(frame);
                params.flare_threshold = threshold.evaluate(frame);
                let c = color.evaluate(frame);
                params.flare_color = [c[0], c[1], c[2], c[3]];
            }
            EffectType::Invert { .. } => {
                params.invert_enabled = 1;
            }
            EffectType::Posterize { levels } => {
                params.posterize_enabled = 1;
                params.posterize_levels = levels.evaluate(frame);
            }
            EffectType::Threshold { threshold } => {
                params.posterize_enabled = 1;
                params.posterize_levels = 2.0;
                params.threshold_level = threshold.evaluate(frame) / 255.0;
            }
            EffectType::Tritone {
                highlight_color,
                shadow_color,
                ..
            } => {
                params.tint_enabled = 1;
                params.tint_amount = 1.0;
                let sh = shadow_color.evaluate(frame);
                let hi = highlight_color.evaluate(frame);
                params.tint_black = [sh[0], sh[1], sh[2], 1.0];
                params.tint_white = [hi[0], hi[1], hi[2], 1.0];
            }
            EffectType::CrtScanlines {
                line_spacing,
                intensity,
            } => {
                params.crt_enabled = 1;
                params.crt_scanline_count =
                    (1080.0 / line_spacing.evaluate(frame).max(1.0)).clamp(50.0, 1000.0);
                params.crt_scanline_intensity = intensity.evaluate(frame).clamp(0.0, 1.0);
            }
            EffectType::FractalNoise {
                contrast,
                brightness,
                evolution,
                complexity,
                ..
            } => {
                params.sim_enabled = 1;
                params.sim_type = 1;
                params.sim_p1 = contrast.evaluate(frame) / 100.0;
                params.sim_p2 = brightness.evaluate(frame) / 100.0;
                params.sim_p3 = evolution.evaluate(frame) / 360.0;
                params.sim_p4 = complexity.evaluate(frame);
            }
            EffectType::TurbulentDisplace {
                amount,
                size,
                evolution,
                ..
            } => {
                params.sim_enabled = 1;
                params.sim_type = 2;
                params.sim_p1 = amount.evaluate(frame);
                params.sim_p2 = size.evaluate(frame) / 10.0;
                params.sim_p3 = evolution.evaluate(frame) / 360.0;
            }
            EffectType::WaveWarp {
                wave_height,
                wave_width,
                speed,
                ..
            } => {
                params.sim_enabled = 1;
                params.sim_type = 3;
                let w = wave_width.evaluate(frame).max(1.0);
                params.sim_p1 = (1000.0 / w).clamp(1.0, 100.0);
                params.sim_p2 = wave_height.evaluate(frame);
                params.sim_p3 = (frame as f32) * speed.evaluate(frame) * 0.1;
            }
            EffectType::Twirl { angle, radius } => {
                params.sim_enabled = 1;
                params.sim_type = 4;
                params.sim_p1 = (radius.evaluate(frame) / 1000.0).max(0.01);
                params.sim_p2 = angle.evaluate(frame);
            }
            EffectType::Bulge { amount, radius } => {
                params.sim_enabled = 1;
                params.sim_type = 5;
                params.sim_p1 = (radius.evaluate(frame) / 1000.0).max(0.01);
                params.sim_p2 = amount.evaluate(frame);
            }
            EffectType::Spherize { radius, .. } => {
                params.sim_enabled = 1;
                params.sim_type = 6;
                params.sim_p1 = (radius.evaluate(frame) / 1000.0).max(0.01);
            }
            EffectType::HeatDistortion { strength, speed } => {
                params.sim_enabled = 1;
                params.sim_type = 7;
                params.sim_p1 = strength.evaluate(frame);
                params.sim_p2 = (frame as f32) * speed.evaluate(frame) * 0.05;
            }
            EffectType::RainRipples {
                drop_count,
                wave_strength,
            } => {
                params.sim_enabled = 1;
                params.sim_type = 7;
                params.sim_p1 = wave_strength.evaluate(frame);
                params.sim_p2 = (frame as f32) * drop_count.evaluate(frame) * 0.02;
            }
            _ => {}
        }
    }
}

/// Evaluate all active plugins on a layer's effects into a single EffectParams.
pub fn evaluate_effects(effects: &[crate::core::timeline::Effect], frame: u32) -> EffectParams {
    let mut params = EffectParams::default();
    for effect in effects {
        if effect.enabled {
            let plugin = EnumEffectPlugin {
                effect_type: effect.effect_type.clone(),
            };
            plugin.apply_to_params(frame, &mut params);
        }
    }
    params
}

/// Returns whether the realtime GPU preview can represent this effect without
/// silently dropping parameters.
pub fn gpu_preview_effect_supported(effect: &EffectType) -> bool {
    match effect {
        EffectType::MotionBlur { .. }
        | EffectType::FilmGrain { .. }
        | EffectType::FractalNoise { .. }
        | EffectType::Tritone { .. }
        | EffectType::Glow { .. }
        | EffectType::GlowPro { .. }
        | EffectType::HueSaturation { .. }
        | EffectType::TurbulentDisplace { .. }
        | EffectType::WaveWarp { .. }
        | EffectType::Spherize { .. }
        | EffectType::HeatDistortion { .. }
        | EffectType::GaussianBlur { .. }
        | EffectType::DropShadow { .. }
        | EffectType::ChromaticAberration { .. }
        | EffectType::Vignette { .. }
        | EffectType::MeshWarp { .. }
        | EffectType::Posterize { .. }
        | EffectType::CrtScanlines { .. }
        | EffectType::Twirl { .. }
        | EffectType::Bulge { .. }
        | EffectType::LensFlare {
            link_to_light: Some(_),
            ..
        }
        | EffectType::Invert { invert_alpha: true } => false,
        _ => matches!(
            effect,
            EffectType::ColorTint { .. }
                | EffectType::Levels { .. }
                | EffectType::LensFlare {
                    link_to_light: None,
                    ..
                }
                | EffectType::Invert {
                    invert_alpha: false
                }
        ),
    }
}
