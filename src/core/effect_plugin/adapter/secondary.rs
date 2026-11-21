use super::super::params::EffectParams;
use crate::core::timeline::EffectType;

pub(super) fn apply_secondary_effect(
    effect_type: &EffectType,
    frame: u32,
    params: &mut EffectParams,
) {
    match effect_type {
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
