use super::types::ParamRef;
use crate::core::timeline::EffectType;

impl EffectType {
    /// All keyframeable parameters of this effect, in display order.
    pub fn animatable_params(&mut self) -> Vec<(&'static str, ParamRef<'_>)> {
        let mut out: Vec<(&'static str, ParamRef<'_>)> = Vec::new();
        macro_rules! push {
            ($label:expr, $field:expr, Scalar) => {
                out.push(($label, ParamRef::Scalar($field)))
            };
            ($label:expr, $field:expr, Vec2) => {
                out.push(($label, ParamRef::Vec2($field)))
            };
            ($label:expr, $field:expr, Vec3) => {
                out.push(($label, ParamRef::Vec3($field)))
            };
            ($label:expr, $field:expr, Color) => {
                out.push(($label, ParamRef::Vec4Color($field)))
            };
        }
        match self {
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
            // ── Newly registered effects (session 3) ──
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
            EffectType::GradientMap { .. } => {} // [f32;3] not yet keyframeable
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
            EffectType::CustomShader { .. } => {}
            #[allow(unreachable_patterns)]
            _ => {}
        }
        out
    }
}
