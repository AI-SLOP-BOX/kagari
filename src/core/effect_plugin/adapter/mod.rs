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
            _ => secondary::apply_secondary_effect(&self.effect_type, frame, params),
        }
    }
}

mod gpu_slots;
mod secondary;

pub(crate) use gpu_slots::has_wgsl_shader_effect;
pub use gpu_slots::{
    GpuEffectSlot, gpu_preview_effect_slot, gpu_preview_effect_supported,
};

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
