use super::super::types::ParamRefRef;
use crate::core::timeline::EffectType;

pub fn populate_primary_params_ref<'a>(
    effect: &'a EffectType,
    out: &mut Vec<(&'static str, ParamRefRef<'a>)>,
) -> bool {
    macro_rules! push {
        ($label:expr, $field:expr, Scalar) => {
            out.push(($label, ParamRefRef::Scalar($field)))
        };
        ($label:expr, $field:expr, Vec2) => {
            out.push(($label, ParamRefRef::Vec2($field)))
        };
        ($label:expr, $field:expr, Vec3) => {
            out.push(($label, ParamRefRef::Vec3($field)))
        };
        ($label:expr, $field:expr, Color) => {
            out.push(($label, ParamRefRef::Vec4Color($field)))
        };
    }
    match effect {
        EffectType::GaussianBlur { blur_radius } => push!("Blur Radius", blur_radius, Scalar),
        EffectType::ColorTint { color, intensity } => {
            push!("Tint Color", color, Color);
            push!("Intensity", intensity, Scalar);
        }
        EffectType::DropShadow {
            color,
            opacity,
            direction,
            distance,
            softness,
        } => {
            push!("Shadow Color", color, Color);
            push!("Opacity", opacity, Scalar);
            push!("Direction", direction, Scalar);
            push!("Distance", distance, Scalar);
            push!("Softness", softness, Scalar);
        }
        EffectType::ChromaticAberration {
            shift_r,
            shift_b,
            edge_falloff,
            iris_linked: _,
        } => {
            push!("Red Shift", shift_r, Scalar);
            push!("Blue Shift", shift_b, Scalar);
            push!("Edge Falloff", edge_falloff, Scalar);
        }
        EffectType::Vignette {
            intensity,
            roundness,
            feather,
            color,
        } => {
            push!("Vignette Color", color, Color);
            push!("Intensity", intensity, Scalar);
            push!("Roundness", roundness, Scalar);
            push!("Feather", feather, Scalar);
        }
        EffectType::Levels {
            input_black,
            input_white,
            gamma,
            output_black,
            output_white,
        } => {
            push!("Input Black", input_black, Scalar);
            push!("Input White", input_white, Scalar);
            push!("Gamma", gamma, Scalar);
            push!("Output Black", output_black, Scalar);
            push!("Output White", output_white, Scalar);
        }
        EffectType::HueSaturation {
            hue_shift,
            saturation,
            lightness,
        } => {
            push!("Hue Shift", hue_shift, Scalar);
            push!("Saturation", saturation, Scalar);
            push!("Lightness", lightness, Scalar);
        }
        EffectType::Glow {
            threshold,
            radius,
            intensity,
            color,
        } => {
            push!("Glow Color", color, Color);
            push!("Threshold", threshold, Scalar);
            push!("Radius", radius, Scalar);
            push!("Intensity", intensity, Scalar);
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
            push!("Flare Enabled", enabled, Scalar);
            push!("Position X", position_x, Scalar);
            push!("Position Y", position_y, Scalar);
            push!("Intensity", intensity, Scalar);
            push!("Threshold", threshold, Scalar);
            push!("Flare Color", color, Color);
        }
        EffectType::MotionBlur { shutter_angle, .. } => {
            push!("Shutter Angle", shutter_angle, Scalar)
        }
        EffectType::MeshWarp {
            top_left,
            top_right,
            bottom_left,
            bottom_right,
        } => {
            push!("Top Left", top_left, Vec2);
            push!("Top Right", top_right, Vec2);
            push!("Bottom Left", bottom_left, Vec2);
            push!("Bottom Right", bottom_right, Vec2);
        }
        EffectType::CornerPin {
            top_left,
            top_right,
            bottom_right,
            bottom_left,
        } => {
            push!("Top Left", top_left, Vec2);
            push!("Top Right", top_right, Vec2);
            push!("Bottom Right", bottom_right, Vec2);
            push!("Bottom Left", bottom_left, Vec2);
        }
        EffectType::FilmGrain { intensity, .. } => push!("Grain Intensity", intensity, Scalar),
        EffectType::Twirl { angle, radius } => {
            push!("Angle", angle, Scalar);
            push!("Radius", radius, Scalar);
        }
        EffectType::Bulge { amount, radius } => {
            push!("Amount", amount, Scalar);
            push!("Radius", radius, Scalar);
        }
        EffectType::Posterize { levels } => push!("Levels", levels, Scalar),
        EffectType::Offset { shift_x, shift_y } => {
            push!("Shift X", shift_x, Scalar);
            push!("Shift Y", shift_y, Scalar);
        }
        EffectType::DirectionalBlur { angle, length } => {
            push!("Angle", angle, Scalar);
            push!("Length", length, Scalar);
        }
        EffectType::RadialBlur { amount } => push!("Amount", amount, Scalar),
        EffectType::RadialFastBlur { amount, .. } => push!("Amount", amount, Scalar),
        EffectType::Sharpen { amount } => push!("Amount", amount, Scalar),
        EffectType::Threshold { threshold } => push!("Threshold", threshold, Scalar),
        EffectType::LinearWipe { completion, angle } => {
            push!("Completion", completion, Scalar);
            push!("Angle", angle, Scalar);
        }
        EffectType::SimpleChoker { choke_amount } => {
            push!("Choke Amount", choke_amount, Scalar)
        }
        EffectType::ChromaKey {
            screen_gain,
            clip_black,
            clip_white,
            ..
        } => {
            push!("Screen Gain", screen_gain, Scalar);
            push!("Clip Black", clip_black, Scalar);
            push!("Clip White", clip_white, Scalar);
        }
        EffectType::Spherize {
            radius,
            refractive_index,
        } => {
            push!("Radius", radius, Scalar);
            push!("Refractive Index", refractive_index, Scalar);
        }
        EffectType::TurbulentDisplace {
            amount,
            size,
            evolution,
            complexity,
        } => {
            push!("Amount", amount, Scalar);
            push!("Size", size, Scalar);
            push!("Evolution", evolution, Scalar);
            push!("Complexity", complexity, Scalar);
        }
        EffectType::DisplacementMap {
            source_layer,
            max_horizontal,
            max_vertical,
        } => {
            push!("Source Layer Idx", source_layer, Scalar);
            push!("Max Horizontal", max_horizontal, Scalar);
            push!("Max Vertical", max_vertical, Scalar);
        }
        EffectType::CompoundBlur {
            source_layer,
            max_blur,
        } => {
            push!("Source Layer Idx", source_layer, Scalar);
            push!("Max Blur", max_blur, Scalar);
        }
        EffectType::Minimax { operation, radius } => {
            push!("Operation", operation, Scalar);
            push!("Radius", radius, Scalar);
        }
        EffectType::ShiftChannels {
            take_red,
            take_green,
            take_blue,
            take_alpha,
        } => {
            push!("Take Red", take_red, Scalar);
            push!("Take Green", take_green, Scalar);
            push!("Take Blue", take_blue, Scalar);
            push!("Take Alpha", take_alpha, Scalar);
        }
        EffectType::WaveWarp {
            wave_height,
            wave_width,
            speed,
            direction_deg,
            ..
        } => {
            push!("Wave Height", wave_height, Scalar);
            push!("Wave Width", wave_width, Scalar);
            push!("Speed", speed, Scalar);
            push!("Direction °", direction_deg, Scalar);
        }
        EffectType::CcLens { convergence, zoom } => {
            push!("Convergence", convergence, Scalar);
            push!("Zoom", zoom, Scalar);
        }
        EffectType::PolarCoordinates { interpolation, .. } => {
            push!("Interpolation", interpolation, Scalar)
        }
        EffectType::OpticsCompensation {
            field_of_view_deg,
            zoom,
            ..
        } => {
            push!("Field of View", field_of_view_deg, Scalar);
            push!("Zoom", zoom, Scalar);
        }
        EffectType::ColorBalance { .. } => {}
        EffectType::ChannelMixer { .. } => {}
        EffectType::LightSweep {
            direction_deg,
            center,
            width,
            sweep_intensity,
            edge_intensity,
        } => {
            push!("Direction °", direction_deg, Scalar);
            push!("Center", center, Scalar);
            push!("Width", width, Scalar);
            push!("Sweep Intensity", sweep_intensity, Scalar);
            push!("Edge Intensity", edge_intensity, Scalar);
        }
        EffectType::BendIt {
            top_offset,
            bottom_offset,
        } => {
            push!("Top Offset", top_offset, Scalar);
            push!("Bottom Offset", bottom_offset, Scalar);
        }
        EffectType::Tiler { scale_percent, .. } => push!("Scale %", scale_percent, Scalar),
        EffectType::Tritone { .. } => {}
        EffectType::MatteChoker {
            choke_amount,
            gray_level,
        } => {
            push!("Choke Amount", choke_amount, Scalar);
            push!("Gray Level", gray_level, Scalar);
        }
        EffectType::VenetianBlinds { completion, width } => {
            push!("Completion", completion, Scalar);
            push!("Stripe Width", width, Scalar);
        }
        EffectType::Vibrance { amount } => push!("Amount", amount, Scalar),
        EffectType::WhiteBalance { temperature, tint } => {
            push!("Temperature", temperature, Scalar);
            push!("Tint", tint, Scalar);
        }
        EffectType::HslAdjust {
            hue_deg,
            saturation,
            lightness,
        } => {
            push!("Hue °", hue_deg, Scalar);
            push!("Saturation", saturation, Scalar);
            push!("Lightness", lightness, Scalar);
        }
        EffectType::GlowPro {
            threshold,
            radius,
            intensity,
            ..
        } => {
            push!("Threshold", threshold, Scalar);
            push!("Radius", radius, Scalar);
            push!("Intensity", intensity, Scalar);
        }
        EffectType::CrtScanlines {
            line_spacing,
            intensity,
            ..
        } => {
            push!("Line Spacing", line_spacing, Scalar);
            push!("Intensity", intensity, Scalar);
        }
        EffectType::Vortex {
            radius, angle_deg, ..
        } => {
            push!("Radius", radius, Scalar);
            push!("Angle", angle_deg, Scalar);
        }
        EffectType::HeatDistortion {
            strength, speed, ..
        } => {
            push!("Strength", strength, Scalar);
            push!("Speed", speed, Scalar);
        }
        EffectType::RainRipples {
            drop_count,
            wave_strength,
            ..
        } => {
            push!("Drop Count", drop_count, Scalar);
            push!("Wave Strength", wave_strength, Scalar);
        }
        EffectType::Fisheye { strength, .. } => push!("Strength", strength, Scalar),
        EffectType::LensCorrection { k1, k2, .. } => {
            push!("K1", k1, Scalar);
            push!("K2", k2, Scalar);
        }
        EffectType::GlitchDisplacement { seed, amount, .. } => {
            push!("Seed", seed, Scalar);
            push!("Amount", amount, Scalar);
        }
        EffectType::RGBSplit {
            red_offset,
            green_offset,
            blue_offset,
            ..
        } => {
            push!("Red Offset", red_offset, Vec2);
            push!("Green Offset", green_offset, Vec2);
            push!("Blue Offset", blue_offset, Vec2);
        }
        _ => return false,
    }
    true
}
