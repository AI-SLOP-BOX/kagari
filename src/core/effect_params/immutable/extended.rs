use super::super::types::ParamRefRef;
use crate::core::timeline::EffectType;

pub fn populate_extended_params_ref<'a>(
    effect: &'a EffectType,
    out: &mut Vec<(&'static str, ParamRefRef<'a>)>,
) {
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
        EffectType::Flicker {
            amount,
            speed,
            seed,
            ..
        } => {
            push!("Amount", amount, Scalar);
            push!("Speed", speed, Scalar);
            push!("Seed", seed, Scalar);
        }
        EffectType::BlockGlitch {
            block_size,
            amount,
            seed,
            corruption,
            ..
        } => {
            push!("Block Size", block_size, Scalar);
            push!("Amount", amount, Scalar);
            push!("Seed", seed, Scalar);
            push!("Corruption", corruption, Scalar);
        }
        EffectType::SliceTear {
            slices,
            max_offset,
            seed,
            ..
        } => {
            push!("Slices", slices, Scalar);
            push!("Max Offset", max_offset, Scalar);
            push!("Seed", seed, Scalar);
        }
        EffectType::MatteChokeSpread { radius, .. } => push!("Radius", radius, Scalar),
        EffectType::AlphaFeather { radius, .. } => push!("Radius", radius, Scalar),
        EffectType::AlphaFromLuminance { .. } => {}
        EffectType::NightVision { amplification, .. } => {
            push!("Amplification", amplification, Scalar)
        }
        EffectType::IrisWipe { completion, .. } => push!("Completion", completion, Scalar),
        EffectType::RadialWipe { completion, .. } => push!("Completion", completion, Scalar),
        EffectType::FilmEmulation {
            lift,
            gamma,
            gain,
            hue_shift_deg,
            ..
        } => {
            push!("Lift", lift, Scalar);
            push!("Gamma", gamma, Scalar);
            push!("Gain", gain, Scalar);
            push!("Hue Shift", hue_shift_deg, Scalar);
        }
        EffectType::GodRays {
            sun_x,
            sun_y,
            samples,
            decay,
            weight,
            ..
        } => {
            push!("Sun X", sun_x, Scalar);
            push!("Sun Y", sun_y, Scalar);
            push!("Samples", samples, Scalar);
            push!("Decay", decay, Scalar);
            push!("Weight", weight, Scalar);
        }
        EffectType::RadialBlurZoom { amount } => push!("Zoom Amount", amount, Scalar),
        EffectType::OpticalFlares {
            position,
            brightness,
            scale,
        } => {
            push!("Position", position, Vec2);
            push!("Brightness", brightness, Scalar);
            push!("Scale", scale, Scalar);
        }
        EffectType::MotionTile {
            tile_center,
            tile_width,
            tile_height,
            output_width,
            output_height,
            phase,
            ..
        } => {
            push!("Tile Center", tile_center, Vec2);
            push!("Tile Width", tile_width, Scalar);
            push!("Tile Height", tile_height, Scalar);
            push!("Output Width", output_width, Scalar);
            push!("Output Height", output_height, Scalar);
            push!("Phase", phase, Scalar);
        }
        EffectType::PageTurn {
            fold_position,
            fold_radius,
            fold_direction_deg,
            light_direction_deg,
            back_opacity,
            ..
        } => {
            push!("Fold Position", fold_position, Vec2);
            push!("Fold Radius", fold_radius, Scalar);
            push!("Fold Direction", fold_direction_deg, Scalar);
            push!("Light Direction", light_direction_deg, Scalar);
            push!("Back Opacity", back_opacity, Scalar);
        }
        EffectType::SetMatte { .. } => {}
        EffectType::Echo {
            echo_time_seconds,
            starting_intensity,
            decay,
            ..
        } => {
            push!("Echo Time (s)", echo_time_seconds, Scalar);
            push!("Starting Intensity", starting_intensity, Scalar);
            push!("Decay", decay, Scalar);
        }
        EffectType::FindEdges { .. } => {}
        EffectType::Transform {
            anchor_point,
            position,
            scale_width,
            scale_height,
            skew_deg,
            skew_axis_deg,
            rotation_deg,
            opacity,
            ..
        } => {
            push!("Anchor Point", anchor_point, Vec2);
            push!("Position", position, Vec2);
            push!("Scale Width", scale_width, Scalar);
            push!("Scale Height", scale_height, Scalar);
            push!("Skew", skew_deg, Scalar);
            push!("Skew Axis", skew_axis_deg, Scalar);
            push!("Rotation", rotation_deg, Scalar);
            push!("Opacity", opacity, Scalar);
        }
        EffectType::CameraLensBlur {
            blur_radius,
            iris_rotation_deg,
            iris_roundness,
            highlight_gain,
            highlight_threshold,
            ..
        } => {
            push!("Blur Radius", blur_radius, Scalar);
            push!("Iris Rotation", iris_rotation_deg, Scalar);
            push!("Iris Roundness", iris_roundness, Scalar);
            push!("Highlight Gain", highlight_gain, Scalar);
            push!("Highlight Threshold", highlight_threshold, Scalar);
        }
        EffectType::LinearColorKey {
            key_color,
            tolerance,
            softness,
            ..
        } => {
            push!("Key Color", key_color, Vec3);
            push!("Tolerance", tolerance, Scalar);
            push!("Softness", softness, Scalar);
        }
        EffectType::ChannelCombiner { .. } => {}
        EffectType::BevelAlpha {
            depth,
            light_angle_deg,
        } => {
            push!("Depth", depth, Scalar);
            push!("Light Angle", light_angle_deg, Scalar);
        }
        EffectType::CrossHatch {
            line_gap,
            threshold,
        } => {
            push!("Line Gap", line_gap, Scalar);
            push!("Threshold", threshold, Scalar);
        }
        EffectType::CmykHalftone { dot_size } => push!("Dot Size", dot_size, Scalar),
        EffectType::ColorGradeLUT { intensity, .. } => {
            push!("LUT Intensity", intensity, Scalar)
        }
        EffectType::Colorama {
            preset_index,
            cycle_phase,
        } => {
            push!("Preset", preset_index, Scalar);
            push!("Cycle Phase", cycle_phase, Scalar);
        }
        EffectType::ColorSpaceConvert { .. } => {}
        EffectType::Curves { channel } => push!("Channel", channel, Scalar),
        EffectType::DirectionalSharpen {
            angle_deg,
            strength,
        } => {
            push!("Angle", angle_deg, Scalar);
            push!("Strength", strength, Scalar);
        }
        EffectType::Emboss { angle_deg, depth } => {
            push!("Angle", angle_deg, Scalar);
            push!("Depth", depth, Scalar);
        }
        EffectType::FbmTurbulence { octaves, amplitude } => {
            push!("Octaves", octaves, Scalar);
            push!("Amplitude", amplitude, Scalar);
        }
        EffectType::FireAutomaton { intensity } => push!("Intensity", intensity, Scalar),
        EffectType::FractalNoise {
            fractal_type,
            contrast,
            brightness,
            complexity,
            evolution,
        } => {
            push!("Fractal Type", fractal_type, Scalar);
            push!("Contrast", contrast, Scalar);
            push!("Brightness", brightness, Scalar);
            push!("Complexity", complexity, Scalar);
            push!("Evolution", evolution, Scalar);
        }
        EffectType::GradientMap { .. } => {}
        EffectType::SliderControl { value } => push!("Value", value, Scalar),
        EffectType::AngleControl { angle_degrees } => push!("Angle", angle_degrees, Scalar),
        EffectType::PointControl { point } => push!("Point", point, Vec2),
        EffectType::ColorControl { color } => push!("Color", color, Color),
        EffectType::CheckboxControl { .. } => {}
        EffectType::DropdownControl { .. } => {}
        EffectType::Point3DControl { point } => push!("Point 3D", point, Vec3),
        EffectType::Letterbox { frac } => push!("Bars", frac, Scalar),
        EffectType::Halftone { cell_size } => push!("Cell Size", cell_size, Scalar),
        EffectType::Invert { .. } => {}
        EffectType::LightLeak {
            pos_x,
            pos_y,
            intensity,
        } => {
            push!("Position X", pos_x, Scalar);
            push!("Position Y", pos_y, Scalar);
            push!("Intensity", intensity, Scalar);
        }
        EffectType::LightningArc {
            start_x,
            start_y,
            end_x,
            end_y,
            seed,
            glow,
        } => {
            push!("Start X", start_x, Scalar);
            push!("Start Y", start_y, Scalar);
            push!("End X", end_x, Scalar);
            push!("End Y", end_y, Scalar);
            push!("Seed", seed, Scalar);
            push!("Glow", glow, Scalar);
        }
        EffectType::LaserBeam {
            start_x,
            start_y,
            end_x,
            end_y,
            progress,
            length,
            starting_thickness,
            ending_thickness,
            core_color,
            glow_color,
        } => {
            push!("Start X", start_x, Scalar);
            push!("Start Y", start_y, Scalar);
            push!("End X", end_x, Scalar);
            push!("End Y", end_y, Scalar);
            push!("Progress", progress, Scalar);
            push!("Length %", length, Scalar);
            push!("Start Thickness", starting_thickness, Scalar);
            push!("End Thickness", ending_thickness, Scalar);
            push!("Core Color", core_color, Color);
            push!("Glow Color", glow_color, Color);
        }
        EffectType::LumaKeyRange {
            low_threshold,
            high_threshold,
            ..
        } => {
            push!("Low Threshold", low_threshold, Scalar);
            push!("High Threshold", high_threshold, Scalar);
        }
        EffectType::MedianFilter { radius } => push!("Radius", radius, Scalar),
        EffectType::Mosaic { block_w, block_h } => {
            push!("Block Width", block_w, Scalar);
            push!("Block Height", block_h, Scalar);
        }
        EffectType::PerlinFlow { scale } => push!("Scale", scale, Scalar),
        EffectType::PinchPunch { radius, amount } => {
            push!("Radius", radius, Scalar);
            push!("Amount", amount, Scalar);
        }
        EffectType::PixelSort { threshold } => push!("Threshold", threshold, Scalar),
        EffectType::ReflectionMap {
            reflect_y,
            fade_dist,
            opacity,
        } => {
            push!("Reflect Y", reflect_y, Scalar);
            push!("Fade Distance", fade_dist, Scalar);
            push!("Opacity", opacity, Scalar);
        }
        EffectType::RefractionLens { radius, ior } => {
            push!("Radius", radius, Scalar);
            push!("IOR", ior, Scalar);
        }
        EffectType::ScanlineGlitch {
            jitter_amount,
            seed,
        } => {
            push!("Jitter", jitter_amount, Scalar);
            push!("Seed", seed, Scalar);
        }
        EffectType::SobelEdges { .. } => {}
        EffectType::Solarize { threshold } => push!("Threshold", threshold, Scalar),
        EffectType::TiltShift {
            focus_y,
            focus_height,
            max_blur,
        } => {
            push!("Focus Y", focus_y, Scalar);
            push!("Focus Height", focus_height, Scalar);
            push!("Max Blur", max_blur, Scalar);
        }
        EffectType::GlassEdgeBevel {
            bevel_size,
            refraction,
        } => {
            push!("Bevel Size", bevel_size, Scalar);
            push!("Refraction", refraction, Scalar);
        }
        EffectType::StarField {
            num_stars,
            depth_speed,
        } => {
            push!("Star Count", num_stars, Scalar);
            push!("Depth Speed", depth_speed, Scalar);
        }
        EffectType::MergePaths { operation } => {
            push!("Operation", operation, Scalar);
        }
        EffectType::OffsetPath {
            amount,
            line_join,
            miter_limit,
        } => {
            push!("Amount", amount, Scalar);
            push!("Line Join", line_join, Scalar);
            push!("Miter Limit", miter_limit, Scalar);
        }
        EffectType::VRHorizon {
            pitch_deg,
            yaw_deg,
            roll_deg,
        } => {
            push!("Pitch", pitch_deg, Scalar);
            push!("Yaw", yaw_deg, Scalar);
            push!("Roll", roll_deg, Scalar);
        }
        EffectType::VRSphericalBlur { radius } => {
            push!("Blur Radius", radius, Scalar);
        }
        EffectType::CustomShader { .. } => {}
        #[allow(unreachable_patterns)]
        _ => {}
    }
}
