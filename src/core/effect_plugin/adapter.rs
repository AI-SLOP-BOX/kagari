use super::naming::{effect_name, effect_type_id};
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
        effect_name(&self.effect_type)
    }

    fn type_id(&self) -> &str {
        effect_type_id(&self.effect_type)
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
                let amt = (intensity.evaluate(frame) / 100.0).clamp(0.0, 1.0);
                params.lut_enabled = 1;
                params.lut_intensity = amt;
                params.sim_enabled = 3;
                params.sim_type = 76;
                params.sim_p1 = amt;
            }
            EffectType::ColorSpaceConvert { mode } => {
                let mode_val = match mode {
                    ColorConversionMode::LogCToLinear => 1,
                    ColorConversionMode::LinearToLogC => 2,
                    ColorConversionMode::SLog3ToLinear => 3,
                    ColorConversionMode::LinearToSLog3 => 4,
                };
                params.color_convert_mode = mode_val;
                params.sim_enabled = 3;
                params.sim_type = 85;
                params.sim_p1 = mode_val as f32;
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
            EffectType::Invert { invert_alpha } => {
                params.invert_enabled = if *invert_alpha { 2 } else { 1 };
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
            EffectType::DirectionalBlur { angle, length } => {
                let rad = angle.evaluate(frame).to_radians();
                let len = length.evaluate(frame);
                params.motionblur_enabled = 1;
                params.motionblur_shutter = 0.0;
                params.motionblur_samples = 24;
                params.motionblur_velocity_x = rad.sin() * len;
                params.motionblur_velocity_y = -rad.cos() * len;
            }
            EffectType::CornerPin {
                top_left,
                top_right,
                bottom_right,
                bottom_left,
            } => {
                params.meshwarp_enabled = 1;
                params.corner_top_left = top_left.evaluate(frame);
                params.corner_top_right = top_right.evaluate(frame);
                params.corner_bottom_right = bottom_right.evaluate(frame);
                params.corner_bottom_left = bottom_left.evaluate(frame);
            }
            EffectType::Mosaic { block_w, block_h } => {
                params.sim_enabled = 3;
                params.sim_type = 0;
                params.sim_p1 = block_w.evaluate(frame).max(1.0);
                params.sim_p2 = block_h.evaluate(frame).max(1.0);
            }
            EffectType::Sharpen { amount } => {
                params.sim_enabled = 3;
                params.sim_type = 1;
                params.sim_p1 = amount.evaluate(frame) / 100.0;
            }
            EffectType::Solarize { threshold } => {
                params.sim_enabled = 3;
                params.sim_type = 2;
                params.sim_p1 = (threshold.evaluate(frame) / 255.0).clamp(0.0, 1.0);
            }
            EffectType::LinearWipe { completion, angle } => {
                params.sim_enabled = 3;
                params.sim_type = 3;
                params.sim_p1 = (completion.evaluate(frame) / 100.0).clamp(0.0, 1.0);
                params.sim_p2 = angle.evaluate(frame).to_radians();
            }
            EffectType::ChromaKey {
                screen_color,
                screen_gain,
                clip_black,
                clip_white,
            } => {
                let sc = screen_color.evaluate(frame);
                params.sim_enabled = 3;
                params.sim_type = 4;
                params.shadow_color = [sc[0], sc[1], sc[2], 1.0];
                params.sim_p1 = screen_gain.evaluate(frame).max(0.01);
                params.sim_p2 = (clip_black.evaluate(frame) / 100.0).clamp(0.0, 1.0);
                params.sim_p3 = (clip_white.evaluate(frame) / 100.0).clamp(0.0, 1.0);
            }
            EffectType::Vibrance { amount } => {
                params.sim_enabled = 3;
                params.sim_type = 5;
                params.sim_p1 = amount.evaluate(frame) / 100.0;
            }
            EffectType::WhiteBalance { temperature, tint } => {
                params.sim_enabled = 3;
                params.sim_type = 6;
                params.sim_p1 = temperature.evaluate(frame) / 100.0;
                params.sim_p2 = tint.evaluate(frame) / 100.0;
            }
            EffectType::SobelEdges { invert } | EffectType::FindEdges { invert } => {
                params.sim_enabled = 3;
                params.sim_type = 7;
                params.sim_p1 = if *invert { 1.0 } else { 0.0 };
            }
            EffectType::Letterbox { frac } => {
                params.sim_enabled = 3;
                params.sim_type = 8;
                params.sim_p1 = frac.evaluate(frame).clamp(0.0, 0.5);
            }
            EffectType::Checkerboard {
                box_size,
                color_1,
                color_2,
            } => {
                params.sim_enabled = 3;
                params.sim_type = 9;
                params.sim_p1 = box_size.evaluate(frame).max(1.0);
                params.tint_black = color_1.evaluate(frame);
                params.tint_white = color_2.evaluate(frame);
            }
            EffectType::GradientRamp {
                start_color,
                end_color,
                is_radial,
            } => {
                params.sim_enabled = 3;
                params.sim_type = 10;
                params.sim_p1 = if *is_radial { 1.0 } else { 0.0 };
                params.tint_black = start_color.evaluate(frame);
                params.tint_white = end_color.evaluate(frame);
            }
            EffectType::RadialBlur { amount } | EffectType::RadialBlurZoom { amount } => {
                params.sim_enabled = 3;
                params.sim_type = 11;
                params.sim_p1 = (amount.evaluate(frame) / 100.0).clamp(0.0, 2.0);
            }
            EffectType::VenetianBlinds { completion, width } => {
                params.sim_enabled = 3;
                params.sim_type = 12;
                params.sim_p1 = (completion.evaluate(frame) / 100.0).clamp(0.0, 1.0);
                params.sim_p2 = width.evaluate(frame).max(1.0);
            }
            EffectType::IrisWipe { completion } => {
                params.sim_enabled = 3;
                params.sim_type = 13;
                params.sim_p1 = (completion.evaluate(frame) / 100.0).clamp(0.0, 1.0);
            }
            EffectType::RadialWipe { completion } => {
                params.sim_enabled = 3;
                params.sim_type = 14;
                params.sim_p1 = (completion.evaluate(frame) / 100.0).clamp(0.0, 1.0);
            }
            EffectType::RGBSplit {
                red_offset,
                green_offset,
                blue_offset,
            } => {
                params.sim_enabled = 3;
                params.sim_type = 15;
                params.corner_top_left = red_offset.evaluate(frame);
                params.corner_top_right = green_offset.evaluate(frame);
                params.corner_bottom_left = blue_offset.evaluate(frame);
            }
            EffectType::CCKaleida { sides } => {
                params.sim_enabled = 3;
                params.sim_type = 16;
                params.sim_p1 = sides.evaluate(frame).max(2.0);
            }
            EffectType::Halftone { cell_size } => {
                params.sim_enabled = 3;
                params.sim_type = 17;
                params.sim_p1 = cell_size.evaluate(frame).max(2.0);
            }
            EffectType::Flicker {
                amount,
                speed,
                seed,
            } => {
                params.sim_enabled = 3;
                params.sim_type = 18;
                let phase = (((frame as f32 * speed.evaluate(frame) + seed.evaluate(frame))
                    * 12.9898)
                    .sin()
                    * 43758.5453)
                    .fract();
                params.sim_p1 = 1.0 + (phase - 0.5) * 2.0 * (amount.evaluate(frame) / 100.0);
            }
            EffectType::LumaKeyRange {
                low_threshold,
                high_threshold,
                invert,
            } => {
                params.sim_enabled = 3;
                params.sim_type = 19;
                params.sim_p1 = (low_threshold.evaluate(frame) / 100.0).clamp(0.0, 1.0);
                params.sim_p2 = (high_threshold.evaluate(frame) / 100.0).clamp(0.0, 1.0);
                params.sim_p3 = if *invert { 1.0 } else { 0.0 };
            }
            EffectType::AlphaFromLuminance { invert } => {
                params.sim_enabled = 3;
                params.sim_type = 20;
                params.sim_p1 = if *invert { 1.0 } else { 0.0 };
            }
            EffectType::NightVision { amplification } => {
                params.sim_enabled = 3;
                params.sim_type = 21;
                params.sim_p1 = amplification.evaluate(frame).max(0.5);
                params.sim_p2 = frame as f32;
            }
            EffectType::ColorBalance {
                shadows,
                midtones,
                highlights,
                ..
            } => {
                params.sim_enabled = 3;
                params.sim_type = 22;
                params.tint_black = [shadows[0], shadows[1], shadows[2], 1.0];
                params.shadow_color = [midtones[0], midtones[1], midtones[2], 1.0];
                params.tint_white = [highlights[0], highlights[1], highlights[2], 1.0];
            }
            EffectType::SimpleChoker { choke_amount } => {
                params.sim_enabled = 3;
                params.sim_type = 23;
                params.sim_p1 = choke_amount.evaluate(frame);
            }
            EffectType::Offset { shift_x, shift_y } => {
                params.sim_enabled = 3;
                params.sim_type = 24;
                params.sim_p1 = shift_x.evaluate(frame);
                params.sim_p2 = shift_y.evaluate(frame);
            }
            EffectType::Tiler {
                scale_percent,
                mirror,
            } => {
                params.sim_enabled = 3;
                params.sim_type = 25;
                params.sim_p1 = (100.0 / scale_percent.evaluate(frame).max(1.0)).clamp(0.1, 64.0);
                params.sim_p2 = if *mirror { 1.0 } else { 0.0 };
            }
            EffectType::Fisheye { strength } => {
                params.sim_enabled = 3;
                params.sim_type = 26;
                params.sim_p1 = strength.evaluate(frame);
            }
            EffectType::CcLens { convergence, zoom } => {
                params.sim_enabled = 3;
                params.sim_type = 26;
                params.sim_p1 = convergence.evaluate(frame) / 100.0;
                params.sim_p2 = (zoom.evaluate(frame) / 100.0).max(0.1);
            }
            EffectType::PolarCoordinates {
                to_polar,
                interpolation,
            } => {
                params.sim_enabled = 3;
                params.sim_type = 27;
                params.sim_p1 = (interpolation.evaluate(frame) / 100.0).clamp(0.0, 1.0);
                params.sim_p2 = if *to_polar { 1.0 } else { 0.0 };
            }
            EffectType::Vortex { radius, angle_deg } => {
                params.sim_enabled = 3;
                params.sim_type = 28;
                params.sim_p1 = (radius.evaluate(frame) / 1000.0).max(0.01);
                params.sim_p2 = angle_deg.evaluate(frame).to_radians();
            }
            EffectType::FilmEmulation {
                lift,
                gamma,
                gain,
                ..
            } => {
                params.sim_enabled = 3;
                params.sim_type = 29;
                params.sim_p1 = lift.evaluate(frame);
                params.sim_p2 = gamma.evaluate(frame).max(0.05);
                params.sim_p3 = gain.evaluate(frame);
            }
            EffectType::PinchPunch { radius, amount } => {
                params.sim_enabled = 3;
                params.sim_type = 30;
                params.sim_p1 = (radius.evaluate(frame) / 1000.0).max(0.01);
                params.sim_p2 = amount.evaluate(frame);
            }
            EffectType::ScanlineGlitch {
                jitter_amount,
                seed,
            } => {
                params.sim_enabled = 3;
                params.sim_type = 31;
                params.sim_p1 = jitter_amount.evaluate(frame);
                params.sim_p2 = seed.evaluate(frame) + frame as f32 * 0.37;
            }
            EffectType::LightSweep {
                direction_deg,
                center,
                width,
                sweep_intensity,
                ..
            } => {
                params.sim_enabled = 3;
                params.sim_type = 32;
                params.sim_p1 = direction_deg.evaluate(frame).to_radians();
                params.sim_p2 = center.evaluate(frame) / 100.0;
                params.sim_p3 = (width.evaluate(frame) / 1000.0).max(0.01);
                params.tint_amount = (sweep_intensity.evaluate(frame) / 100.0).clamp(0.0, 2.0);
            }
            EffectType::ShiftChannels {
                take_red,
                take_green,
                take_blue,
                take_alpha,
            } => {
                params.sim_enabled = 3;
                params.sim_type = 33;
                params.sim_p1 = take_red.evaluate(frame);
                params.sim_p2 = take_green.evaluate(frame);
                params.sim_p3 = take_blue.evaluate(frame);
                params.sim_p4 = take_alpha.evaluate(frame);
            }
            EffectType::OpticsCompensation {
                field_of_view_deg,
                reverse,
                zoom,
            } => {
                params.sim_enabled = 3;
                params.sim_type = 34;
                params.sim_p1 = (field_of_view_deg.evaluate(frame) / 90.0)
                    * if *reverse { -1.0 } else { 1.0 };
                params.sim_p2 = (zoom.evaluate(frame) / 100.0).max(0.1);
            }
            EffectType::LensCorrection { k1, k2 } => {
                params.sim_enabled = 3;
                params.sim_type = 35;
                params.sim_p1 = k1.evaluate(frame);
                params.sim_p2 = k2.evaluate(frame);
            }
            EffectType::RadialFastBlur { amount, .. } => {
                params.sim_enabled = 3;
                params.sim_type = 11;
                params.sim_p1 = (amount.evaluate(frame) / 100.0).clamp(0.0, 2.0);
            }
            EffectType::TiltShift {
                focus_y,
                focus_height,
                max_blur,
            } => {
                params.sim_enabled = 3;
                params.sim_type = 36;
                params.sim_p1 = focus_y.evaluate(frame).clamp(0.0, 1.0);
                params.sim_p2 = focus_height.evaluate(frame).max(0.01);
                params.sim_p3 = max_blur.evaluate(frame).max(0.0);
            }
            EffectType::Emboss { angle_deg, depth } => {
                params.sim_enabled = 3;
                params.sim_type = 37;
                params.sim_p1 = angle_deg.evaluate(frame).to_radians();
                params.sim_p2 = depth.evaluate(frame);
            }
            EffectType::LinearColorKey {
                key_color,
                tolerance,
                softness,
                ..
            } => {
                let kc = key_color.evaluate(frame);
                params.sim_enabled = 3;
                params.sim_type = 4;
                params.shadow_color = [kc[0], kc[1], kc[2], 1.0];
                params.sim_p1 = 1.0;
                params.sim_p2 = (tolerance.evaluate(frame) / 100.0).clamp(0.0, 1.0);
                params.sim_p3 = ((tolerance.evaluate(frame) + softness.evaluate(frame)) / 100.0)
                    .clamp(0.01, 1.0);
            }
            EffectType::BlockGlitch {
                block_size,
                amount,
                seed,
                ..
            } => {
                params.sim_enabled = 3;
                params.sim_type = 38;
                params.sim_p1 = block_size.evaluate(frame).max(4.0);
                params.sim_p2 = amount.evaluate(frame);
                params.sim_p3 = seed.evaluate(frame) + frame as f32 * 0.13;
            }
            EffectType::SliceTear {
                slices,
                max_offset,
                seed,
                vertical,
            } => {
                params.sim_enabled = 3;
                params.sim_type = 39;
                params.sim_p1 = slices.evaluate(frame).max(1.0);
                params.sim_p2 = max_offset.evaluate(frame);
                params.sim_p3 = seed.evaluate(frame) + frame as f32 * 0.29;
                params.sim_p4 = if *vertical { 1.0 } else { 0.0 };
            }
            EffectType::LightLeak {
                pos_x,
                pos_y,
                intensity,
            } => {
                params.sim_enabled = 3;
                params.sim_type = 40;
                params.sim_p1 = pos_x.evaluate(frame);
                params.sim_p2 = pos_y.evaluate(frame);
                params.sim_p3 = (intensity.evaluate(frame) / 100.0).clamp(0.0, 2.0);
            }
            EffectType::CmykHalftone { dot_size } => {
                params.sim_enabled = 3;
                params.sim_type = 17;
                params.sim_p1 = dot_size.evaluate(frame).max(2.0);
            }
            EffectType::StrobeLight {
                interval_frames,
                strobe_color,
            } => {
                params.sim_enabled = 3;
                params.sim_type = 41;
                let interval = (interval_frames.evaluate(frame).round() as u32).max(1);
                params.sim_p1 = if frame % interval == 0 { 1.0 } else { 0.0 };
                params.shadow_color = strobe_color.evaluate(frame);
            }
            EffectType::ChannelMixer { matrix, monochrome } => {
                params.sim_enabled = 3;
                params.sim_type = 42;
                params.sim_p1 = if *monochrome { 1.0 } else { 0.0 };
                params.tint_black = [matrix[0][0], matrix[0][1], matrix[0][2], 1.0];
                params.shadow_color = [matrix[1][0], matrix[1][1], matrix[1][2], 1.0];
                params.tint_white = [matrix[2][0], matrix[2][1], matrix[2][2], 1.0];
            }
            EffectType::HslAdjust {
                hue_deg,
                saturation,
                lightness,
            } => {
                params.sim_enabled = 3;
                params.sim_type = 43;
                params.sim_p1 = hue_deg.evaluate(frame) / 360.0;
                params.sim_p2 = saturation.evaluate(frame);
                params.sim_p3 = lightness.evaluate(frame);
            }
            EffectType::DirectionalSharpen { angle_deg, strength } => {
                params.sim_enabled = 3;
                params.sim_type = 44;
                params.sim_p1 = angle_deg.evaluate(frame).to_radians();
                params.sim_p2 = strength.evaluate(frame);
            }
            EffectType::BevelAlpha {
                depth,
                light_angle_deg,
            } => {
                params.sim_enabled = 3;
                params.sim_type = 45;
                params.sim_p1 = depth.evaluate(frame).max(0.0);
                params.sim_p2 = light_angle_deg.evaluate(frame).to_radians();
            }
            EffectType::GlassEdgeBevel {
                bevel_size,
                refraction,
            } => {
                params.sim_enabled = 3;
                params.sim_type = 46;
                params.sim_p1 = bevel_size.evaluate(frame).max(0.5);
                params.sim_p2 = refraction.evaluate(frame);
            }
            EffectType::MatteChoker {
                choke_amount,
                gray_level,
            } => {
                params.sim_enabled = 3;
                params.sim_type = 47;
                params.sim_p1 = choke_amount.evaluate(frame);
                params.sim_p2 = (gray_level.evaluate(frame) / 100.0).clamp(0.0, 1.0);
            }
            EffectType::MatteChokeSpread { radius, expand } => {
                params.sim_enabled = 3;
                params.sim_type = 47;
                let r = radius.evaluate(frame).max(0.0);
                params.sim_p1 = if *expand { -r } else { r };
                params.sim_p2 = 0.5;
            }
            EffectType::AlphaFeather { radius } => {
                params.sim_enabled = 3;
                params.sim_type = 48;
                params.sim_p1 = radius.evaluate(frame).max(0.0);
            }
            EffectType::Minimax { operation, radius } => {
                params.sim_enabled = 3;
                params.sim_type = 49;
                params.sim_p1 = operation.evaluate(frame);
                params.sim_p2 = radius.evaluate(frame).max(0.0);
            }
            EffectType::MedianFilter { radius } => {
                params.sim_enabled = 3;
                params.sim_type = 50;
                params.sim_p1 = radius.evaluate(frame).max(0.5);
                params.sim_p2 = 1.0;
                params.sim_p3 = 0.0;
            }
            EffectType::BilateralFilter {
                radius,
                color_sigma,
                ..
            } => {
                params.sim_enabled = 3;
                params.sim_type = 50;
                params.sim_p1 = radius.evaluate(frame).max(0.5);
                params.sim_p2 = color_sigma.evaluate(frame).max(0.01);
                params.sim_p3 = 1.0;
            }
            EffectType::MotionTile {
                tile_center,
                tile_width,
                tile_height,
                output_width,
                output_height,
                mirror_edges,
                phase,
            } => {
                let tc = tile_center.evaluate(frame);
                params.sim_enabled = 3;
                params.sim_type = 51;
                params.sim_p1 = tc[0];
                params.sim_p2 = tc[1];
                params.sim_p3 = (output_width.evaluate(frame) / 100.0).max(0.05);
                params.sim_p4 = (output_height.evaluate(frame) / 100.0).max(0.05);
                params.tint_black = [
                    (tile_width.evaluate(frame) / 100.0).max(0.05),
                    (tile_height.evaluate(frame) / 100.0).max(0.05),
                    if *mirror_edges { 1.0 } else { 0.0 },
                    phase.evaluate(frame) / 360.0,
                ];
            }
            EffectType::Transform {
                anchor_point,
                position,
                scale_width,
                scale_height,
                uniform_scale,
                skew_deg,
                rotation_deg,
                opacity,
                ..
            } => {
                let anc = anchor_point.evaluate(frame);
                let pos = position.evaluate(frame);
                let sw = (scale_width.evaluate(frame) / 100.0).max(0.001);
                let sh = if *uniform_scale {
                    sw
                } else {
                    (scale_height.evaluate(frame) / 100.0).max(0.001)
                };
                params.sim_enabled = 3;
                params.sim_type = 52;
                params.sim_p1 = pos[0];
                params.sim_p2 = pos[1];
                params.sim_p3 = sw;
                params.sim_p4 = sh;
                params.tint_black = [
                    anc[0],
                    anc[1],
                    rotation_deg.evaluate(frame).to_radians(),
                    (opacity.evaluate(frame) / 100.0).clamp(0.0, 1.0),
                ];
                params.tint_white = [skew_deg.evaluate(frame).to_radians(), 0.0, 0.0, 1.0];
            }
            EffectType::PageTurn {
                fold_position,
                fold_radius,
                fold_direction_deg,
                back_color,
                ..
            } => {
                let fp = fold_position.evaluate(frame);
                params.sim_enabled = 3;
                params.sim_type = 53;
                params.sim_p1 = fp[0];
                params.sim_p2 = fp[1];
                params.sim_p3 = fold_radius.evaluate(frame).max(0.01);
                params.sim_p4 = fold_direction_deg.evaluate(frame).to_radians();
                params.shadow_color = back_color.evaluate(frame);
            }
            EffectType::BendIt {
                top_offset,
                bottom_offset,
            } => {
                params.sim_enabled = 3;
                params.sim_type = 54;
                params.sim_p1 = top_offset.evaluate(frame);
                params.sim_p2 = bottom_offset.evaluate(frame);
            }
            EffectType::GlitchDisplacement { seed, amount } => {
                params.sim_enabled = 3;
                params.sim_type = 55;
                params.sim_p1 = amount.evaluate(frame);
                params.sim_p2 = seed.evaluate(frame) + frame as f32 * 0.17;
            }
            EffectType::PixelSort { threshold } => {
                params.sim_enabled = 3;
                params.sim_type = 56;
                params.sim_p1 = threshold.evaluate(frame).clamp(0.0, 1.0);
            }
            EffectType::CrossHatch {
                line_gap,
                threshold,
            } => {
                params.sim_enabled = 3;
                params.sim_type = 57;
                params.sim_p1 = line_gap.evaluate(frame).max(2.0);
                params.sim_p2 = threshold.evaluate(frame).clamp(0.0, 1.0);
            }
            EffectType::GodRays {
                sun_x,
                sun_y,
                decay,
                weight,
                ..
            } => {
                params.sim_enabled = 3;
                params.sim_type = 58;
                params.sim_p1 = sun_x.evaluate(frame);
                params.sim_p2 = sun_y.evaluate(frame);
                params.sim_p3 = decay.evaluate(frame).clamp(0.5, 0.99);
                params.sim_p4 = weight.evaluate(frame).clamp(0.0, 2.0);
            }
            EffectType::OpticalFlares {
                position,
                brightness,
                scale,
            } => {
                let p = position.evaluate(frame);
                params.sim_enabled = 3;
                params.sim_type = 59;
                params.sim_p1 = p[0];
                params.sim_p2 = p[1];
                params.sim_p3 = brightness.evaluate(frame).max(0.0);
                params.sim_p4 = scale.evaluate(frame).max(0.1);
            }
            EffectType::StarField {
                num_stars,
                depth_speed,
            } => {
                params.sim_enabled = 3;
                params.sim_type = 60;
                params.sim_p1 = num_stars.evaluate(frame).max(1.0);
                params.sim_p2 = depth_speed.evaluate(frame) * frame as f32 * 0.02;
            }
            EffectType::LightningArc {
                start_x,
                start_y,
                end_x,
                end_y,
                seed,
                glow,
            } => {
                params.sim_enabled = 3;
                params.sim_type = 61;
                params.sim_p1 = start_x.evaluate(frame);
                params.sim_p2 = start_y.evaluate(frame);
                params.sim_p3 = end_x.evaluate(frame);
                params.sim_p4 = end_y.evaluate(frame);
                params.tint_black = [
                    seed.evaluate(frame) + frame as f32 * 0.31,
                    glow.evaluate(frame).max(0.0),
                    0.0,
                    1.0,
                ];
            }
            EffectType::LaserBeam {
                start_x,
                start_y,
                end_x,
                end_y,
                progress,
                starting_thickness,
                ending_thickness,
                core_color,
                glow_color,
                ..
            } => {
                params.sim_enabled = 3;
                params.sim_type = 62;
                params.sim_p1 = start_x.evaluate(frame);
                params.sim_p2 = start_y.evaluate(frame);
                params.sim_p3 = end_x.evaluate(frame);
                params.sim_p4 = end_y.evaluate(frame);
                params.tint_black = core_color.evaluate(frame);
                params.shadow_color = glow_color.evaluate(frame);
                params.tint_white = [
                    (progress.evaluate(frame) / 100.0).clamp(0.0, 1.0),
                    starting_thickness.evaluate(frame).max(1.0),
                    ending_thickness.evaluate(frame).max(1.0),
                    1.0,
                ];
            }
            EffectType::PerlinFlow { scale } => {
                params.sim_enabled = 3;
                params.sim_type = 63;
                params.sim_p1 = scale.evaluate(frame).max(0.1);
                params.sim_p2 = frame as f32 * 0.03;
            }
            EffectType::FbmTurbulence { octaves, amplitude } => {
                params.sim_enabled = 3;
                params.sim_type = 64;
                params.sim_p1 = octaves.evaluate(frame).clamp(1.0, 6.0);
                params.sim_p2 = amplitude.evaluate(frame);
                params.sim_p3 = frame as f32 * 0.02;
            }
            EffectType::VRHorizon {
                pitch_deg,
                yaw_deg,
                roll_deg,
            } => {
                params.sim_enabled = 3;
                params.sim_type = 65;
                params.sim_p1 = pitch_deg.evaluate(frame).to_radians();
                params.sim_p2 = yaw_deg.evaluate(frame).to_radians();
                params.sim_p3 = roll_deg.evaluate(frame).to_radians();
            }
            EffectType::CCPixelPolly { shatter_progress } => {
                params.sim_enabled = 3;
                params.sim_type = 66;
                params.sim_p1 = (shatter_progress.evaluate(frame) / 100.0).clamp(0.0, 2.0);
            }
            EffectType::Colorama {
                preset_index,
                cycle_phase,
            } => {
                params.sim_enabled = 3;
                params.sim_type = 67;
                params.sim_p1 = preset_index.evaluate(frame);
                params.sim_p2 = cycle_phase.evaluate(frame) / 360.0;
            }
            EffectType::GradientMap {
                low_color,
                mid_color,
                high_color,
            } => {
                let lc = low_color.evaluate(frame);
                let mc = mid_color.evaluate(frame);
                let hc = high_color.evaluate(frame);
                params.sim_enabled = 3;
                params.sim_type = 68;
                params.tint_black = [lc[0], lc[1], lc[2], 1.0];
                params.shadow_color = [mc[0], mc[1], mc[2], 1.0];
                params.tint_white = [hc[0], hc[1], hc[2], 1.0];
            }
            EffectType::RefractionLens { radius, ior } => {
                params.sim_enabled = 3;
                params.sim_type = 69;
                params.sim_p1 = radius.evaluate(frame).max(0.01);
                params.sim_p2 = ior.evaluate(frame);
            }
            EffectType::ChannelCombiner {
                from_channel,
                to_target,
                invert,
            } => {
                use crate::core::channel_combiner::{ChannelCombinerFrom, ChannelCombinerTo};
                params.sim_enabled = 3;
                params.sim_type = 70;
                params.sim_p1 = match from_channel {
                    ChannelCombinerFrom::Red => 0.0,
                    ChannelCombinerFrom::Green => 1.0,
                    ChannelCombinerFrom::Blue => 2.0,
                    ChannelCombinerFrom::Alpha => 3.0,
                    ChannelCombinerFrom::Luminance => 4.0,
                    ChannelCombinerFrom::Hue => 5.0,
                    ChannelCombinerFrom::Lightness => 6.0,
                    ChannelCombinerFrom::Saturation => 7.0,
                    ChannelCombinerFrom::MinRGB => 8.0,
                    ChannelCombinerFrom::MaxRGB => 9.0,
                };
                params.sim_p2 = match to_target {
                    ChannelCombinerTo::Red => 0.0,
                    ChannelCombinerTo::Green => 1.0,
                    ChannelCombinerTo::Blue => 2.0,
                    ChannelCombinerTo::Alpha => 3.0,
                    ChannelCombinerTo::RGBOnly => 4.0,
                    ChannelCombinerTo::RGBA => 5.0,
                    ChannelCombinerTo::Lightness => 6.0,
                };
                params.sim_p3 = if *invert { 1.0 } else { 0.0 };
            }
            EffectType::CameraLensBlur {
                blur_radius,
                highlight_gain,
                highlight_threshold,
                ..
            } => {
                params.sim_enabled = 3;
                params.sim_type = 71;
                params.sim_p1 = blur_radius.evaluate(frame).max(0.0);
                params.sim_p2 = highlight_gain.evaluate(frame).max(0.0);
                params.sim_p3 = highlight_threshold.evaluate(frame).clamp(0.0, 1.0);
            }
            EffectType::VRSphericalBlur { radius } => {
                params.sim_enabled = 3;
                params.sim_type = 72;
                params.sim_p1 = radius.evaluate(frame).max(0.0);
            }
            EffectType::AudioSpectrum {
                enabled,
                bands,
                opacity,
                color_start,
                color_end,
                position_x,
                position_y,
                width,
                height,
                ..
            } => {
                if enabled.evaluate(frame) > 0.5 {
                    params.sim_enabled = 3;
                    params.sim_type = 73;
                    params.sim_p1 = bands.evaluate(frame).clamp(4.0, 64.0);
                    params.sim_p2 = opacity.evaluate(frame).clamp(0.0, 1.0);
                    params.sim_p3 = position_x.evaluate(frame);
                    params.sim_p4 = position_y.evaluate(frame);
                    params.tint_black = *color_start;
                    params.shadow_color = *color_end;
                    params.tint_white = [
                        width.evaluate(frame).max(0.01),
                        height.evaluate(frame).max(0.01),
                        frame as f32 * 0.15,
                        1.0,
                    ];
                }
            }
            EffectType::FireAutomaton { intensity } => {
                params.sim_enabled = 3;
                params.sim_type = 74;
                params.sim_p1 = intensity.evaluate(frame).clamp(0.0, 2.0);
                params.sim_p2 = frame as f32 * 0.08;
            }
            EffectType::Curves { channel } => {
                params.sim_enabled = 3;
                params.sim_type = 75;
                params.sim_p1 = channel.evaluate(frame);
            }
            EffectType::ZDepthFog {
                fog_color,
                fog_near,
                fog_far,
            } => {
                params.sim_enabled = 3;
                params.sim_type = 77;
                params.shadow_color = fog_color.evaluate(frame);
                params.sim_p1 = fog_near.evaluate(frame);
                params.sim_p2 = fog_far.evaluate(frame).max(params.sim_p1 + 0.001);
            }
            EffectType::SSAO { radius, strength } => {
                params.sim_enabled = 3;
                params.sim_type = 78;
                params.sim_p1 = radius.evaluate(frame).max(0.5);
                params.sim_p2 = strength.evaluate(frame).clamp(0.0, 2.0);
            }
            EffectType::ReflectionMap {
                reflect_y,
                fade_dist,
                opacity,
            } => {
                params.sim_enabled = 3;
                params.sim_type = 79;
                params.sim_p1 = reflect_y.evaluate(frame).clamp(0.0, 1.0);
                params.sim_p2 = fade_dist.evaluate(frame).max(0.01);
                params.sim_p3 = opacity.evaluate(frame).clamp(0.0, 1.0);
            }
            EffectType::CompoundBlur { max_blur, .. } => {
                params.sim_enabled = 3;
                params.sim_type = 80;
                params.sim_p1 = max_blur.evaluate(frame).max(0.0);
            }
            EffectType::DisplacementMap {
                max_horizontal,
                max_vertical,
                ..
            } => {
                params.sim_enabled = 3;
                params.sim_type = 81;
                params.sim_p1 = max_horizontal.evaluate(frame);
                params.sim_p2 = max_vertical.evaluate(frame);
            }
            EffectType::TemporalDenoise {
                strength,
                feature_threshold,
                ..
            } => {
                params.sim_enabled = 3;
                params.sim_type = 82;
                params.sim_p1 = strength.evaluate(frame).clamp(0.0, 1.0);
                params.sim_p2 = feature_threshold.evaluate(frame).max(0.01);
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GpuEffectSlot {
    // Reserved until spatial effects pass full-frame GPU/software parity for
    // every animated parameter. They currently use the CPU preview fallback.
    Distortion = 0,
    // 1. Lens flare inside sample_layer_color (line 655)
    LensFlare = 1,
    // 2. ColorTint after sample_layer_color (line 777)
    ColorTint = 2,
    // 3. Levels (line 868)
    Levels = 3,
    // 4. Invert (line 930)
    Invert = 4,
    // 5. Posterize (line 940)
    Posterize = 5,
}

impl GpuEffectSlot {
    /// Returns the execution order in the GPU fragment shader.
    pub fn execution_order(self) -> u32 {
        self as u32
    }
}

/// Returns whether the realtime GPU preview can represent this effect without
/// silently dropping parameters.
pub fn gpu_preview_effect_supported(effect: &EffectType) -> bool {
    gpu_preview_effect_slot(effect).is_some()
}

/// Returns whether the realtime GPU preview has a WGSL shader implementation for
/// this effect (either single-pass or multi-pass scratch).
pub(crate) fn has_wgsl_shader_effect(effect: &EffectType) -> bool {
    matches!(
        effect,
        EffectType::GaussianBlur { .. }
            | EffectType::ColorTint { .. }
            | EffectType::DropShadow { .. }
            | EffectType::ChromaticAberration { .. }
            | EffectType::Vignette { .. }
            | EffectType::ColorSpaceConvert { .. }
            | EffectType::FilmGrain { .. }
            | EffectType::Levels { .. }
            | EffectType::HueSaturation { .. }
            | EffectType::Glow { .. }
            | EffectType::GlowPro { .. }
            | EffectType::MeshWarp { .. }
            | EffectType::MotionBlur { .. }
            | EffectType::LensFlare {
                link_to_light: None,
                ..
            }
            | EffectType::Invert { .. }
            | EffectType::Posterize { .. }
            | EffectType::Threshold { .. }
            | EffectType::Tritone { .. }
            | EffectType::CrtScanlines { .. }
            | EffectType::FractalNoise { .. }
            | EffectType::TurbulentDisplace { .. }
            | EffectType::WaveWarp { .. }
            | EffectType::Twirl { .. }
            | EffectType::Bulge { .. }
            | EffectType::Spherize { .. }
            | EffectType::HeatDistortion { .. }
            | EffectType::RainRipples { .. }
            | EffectType::DirectionalBlur { .. }
            | EffectType::CornerPin { .. }
            | EffectType::Mosaic { .. }
            | EffectType::Sharpen { .. }
            | EffectType::Solarize { .. }
            | EffectType::LinearWipe { .. }
            | EffectType::ChromaKey { .. }
            | EffectType::Vibrance { .. }
            | EffectType::WhiteBalance { .. }
            | EffectType::SobelEdges { .. }
            | EffectType::FindEdges { .. }
            | EffectType::Letterbox { .. }
            | EffectType::Checkerboard { .. }
            | EffectType::GradientRamp { .. }
            | EffectType::RadialBlur { .. }
            | EffectType::RadialBlurZoom { .. }
            | EffectType::VenetianBlinds { .. }
            | EffectType::IrisWipe { .. }
            | EffectType::RadialWipe { .. }
            | EffectType::RGBSplit { .. }
            | EffectType::CCKaleida { .. }
            | EffectType::Halftone { .. }
            | EffectType::Flicker { .. }
            | EffectType::LumaKeyRange { .. }
            | EffectType::AlphaFromLuminance { .. }
            | EffectType::NightVision { .. }
            | EffectType::ColorBalance { .. }
            | EffectType::SimpleChoker { .. }
            | EffectType::Offset { .. }
            | EffectType::Tiler { .. }
            | EffectType::Fisheye { .. }
            | EffectType::CcLens { .. }
            | EffectType::PolarCoordinates { .. }
            | EffectType::Vortex { .. }
            | EffectType::FilmEmulation { .. }
            | EffectType::PinchPunch { .. }
            | EffectType::ScanlineGlitch { .. }
            | EffectType::LightSweep { .. }
            | EffectType::ShiftChannels { .. }
            | EffectType::OpticsCompensation { .. }
            | EffectType::LensCorrection { .. }
            | EffectType::RadialFastBlur { .. }
            | EffectType::TiltShift { .. }
            | EffectType::Emboss { .. }
            | EffectType::LinearColorKey { .. }
            | EffectType::BlockGlitch { .. }
            | EffectType::SliceTear { .. }
            | EffectType::LightLeak { .. }
            | EffectType::CmykHalftone { .. }
            | EffectType::StrobeLight { .. }
            | EffectType::ChannelMixer { .. }
            | EffectType::HslAdjust { .. }
            | EffectType::DirectionalSharpen { .. }
            | EffectType::BevelAlpha { .. }
            | EffectType::GlassEdgeBevel { .. }
            | EffectType::MatteChoker { .. }
            | EffectType::MatteChokeSpread { .. }
            | EffectType::AlphaFeather { .. }
            | EffectType::Minimax { .. }
            | EffectType::MedianFilter { .. }
            | EffectType::BilateralFilter { .. }
            | EffectType::MotionTile { .. }
            | EffectType::Transform { .. }
            | EffectType::PageTurn { .. }
            | EffectType::BendIt { .. }
            | EffectType::GlitchDisplacement { .. }
            | EffectType::PixelSort { .. }
            | EffectType::CrossHatch { .. }
            | EffectType::GodRays { .. }
            | EffectType::OpticalFlares { .. }
            | EffectType::StarField { .. }
            | EffectType::LightningArc { .. }
            | EffectType::LaserBeam { .. }
            | EffectType::PerlinFlow { .. }
            | EffectType::FbmTurbulence { .. }
            | EffectType::VRHorizon { .. }
            | EffectType::CCPixelPolly { .. }
            | EffectType::Colorama { .. }
            | EffectType::GradientMap { .. }
            | EffectType::RefractionLens { .. }
            | EffectType::ChannelCombiner { .. }
            | EffectType::CameraLensBlur { .. }
            | EffectType::VRSphericalBlur { .. }
            | EffectType::AudioSpectrum { .. }
            | EffectType::FireAutomaton { .. }
            | EffectType::Curves { .. }
            | EffectType::ColorGradeLUT { .. }
            | EffectType::ZDepthFog { .. }
            | EffectType::SSAO { .. }
            | EffectType::ReflectionMap { .. }
            | EffectType::CompoundBlur { .. }
            | EffectType::DisplacementMap { .. }
            | EffectType::TemporalDenoise { .. }
            | EffectType::SetMatte { .. }
            | EffectType::Echo { .. }
            | EffectType::CustomShader { .. }
            | EffectType::BassTreble { .. }
            | EffectType::Flanger { .. }
            | EffectType::Chorus { .. }
            | EffectType::ParametricEQ { .. }
            | EffectType::MergePaths { .. }
            | EffectType::OffsetPath { .. }
            | EffectType::SliderControl { .. }
            | EffectType::AngleControl { .. }
            | EffectType::PointControl { .. }
            | EffectType::ColorControl { .. }
            | EffectType::CheckboxControl { .. }
            | EffectType::DropdownControl { .. }
            | EffectType::Point3DControl { .. }
    )
}

/// Returns the specific GPU shader pipeline slot for an effect, or None if unsupported on GPU preview.
pub fn gpu_preview_effect_slot(effect: &EffectType) -> Option<GpuEffectSlot> {
    match effect {
        EffectType::LensFlare {
            link_to_light: None,
            ..
        } => Some(GpuEffectSlot::LensFlare),
        EffectType::ColorTint { .. } => Some(GpuEffectSlot::ColorTint),
        EffectType::Levels { .. } => Some(GpuEffectSlot::Levels),
        EffectType::Invert {
            invert_alpha: false,
        } => Some(GpuEffectSlot::Invert),
        EffectType::Posterize { .. } => Some(GpuEffectSlot::Posterize),
        _ => None,
    }
}
