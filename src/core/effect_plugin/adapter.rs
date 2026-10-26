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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GpuEffectSlot {
    // Execution order in fs_main:
    // 0. Spatial UV distortion (sample_layer_color line 359)
    Distortion = 0,
    // 1. ColorTint (line 777)
    ColorTint = 1,
    // 2. Levels (line 868)
    Levels = 2,
    // 3. Invert (line 930)
    Invert = 3,
    // 4. Posterize (line 940)
    Posterize = 4,
    // LensFlare is handled as additive lighting pass
    LensFlare = 5,
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

/// Returns the specific GPU shader pipeline slot for an effect, or None if unsupported on GPU preview.
pub fn gpu_preview_effect_slot(effect: &EffectType) -> Option<GpuEffectSlot> {
    match effect {
        EffectType::TurbulentDisplace { .. }
        | EffectType::WaveWarp { .. }
        | EffectType::Twirl { .. }
        | EffectType::Bulge { .. }
        | EffectType::Spherize { .. }
        | EffectType::HeatDistortion { .. } => Some(GpuEffectSlot::Distortion),
        EffectType::ColorTint { .. } => Some(GpuEffectSlot::ColorTint),
        EffectType::Levels { .. } => Some(GpuEffectSlot::Levels),
        EffectType::Invert { invert_alpha: false } => Some(GpuEffectSlot::Invert),
        EffectType::Posterize { .. } => Some(GpuEffectSlot::Posterize),
        EffectType::LensFlare { link_to_light: None, .. } => Some(GpuEffectSlot::LensFlare),
        _ => None,
    }
}
