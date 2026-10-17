use super::*;

pub(super) fn apply(
    pixels: &mut [u8],
    effect_type: &EffectType,
    context: LayerEffectRenderContext<'_>,
) -> bool {
    if !matches!(
        effect_type,
        EffectType::WaveWarp { .. }
            | EffectType::CcLens { .. }
            | EffectType::PolarCoordinates { .. }
            | EffectType::OpticsCompensation { .. }
            | EffectType::ColorBalance { .. }
            | EffectType::ChannelMixer { .. }
            | EffectType::LightSweep { .. }
            | EffectType::RadialFastBlur { .. }
            | EffectType::BendIt { .. }
            | EffectType::Tiler { .. }
            | EffectType::Tritone { .. }
            | EffectType::MatteChoker { .. }
            | EffectType::VenetianBlinds { .. }
            | EffectType::Vibrance { .. }
            | EffectType::WhiteBalance { .. }
            | EffectType::HslAdjust { .. }
            | EffectType::GlowPro { .. }
            | EffectType::VRHorizon { .. }
            | EffectType::VRSphericalBlur { .. }
            | EffectType::TemporalDenoise { .. }
    ) {
        return false;
    }
    let LayerEffectRenderContext {
        width,
        height,
        frame,
        fps,
        ..
    } = context;
    use crate::core::ae_effects_pack as pack;
    match effect_type {
        EffectType::WaveWarp {
            wave_height,
            wave_width,
            speed,
            direction_deg,
            wave_type,
            pinning,
        } => {
            let wh = wave_height.evaluate(frame);
            let ww = wave_width.evaluate(frame);
            let sp = speed.evaluate(frame);
            let time = frame as f32 / fps.max(1) as f32;
            let dir = direction_deg.evaluate(frame).to_radians();
            if !crate::core::compute_pipeline::try_gpu_wave_warp(
                pixels,
                width,
                height,
                wh,
                std::f32::consts::TAU / ww.max(1.0),
                sp * time,
                dir,
            ) {
                use crate::core::ae_effects_pack_v27::{
                    apply_wave_warp_pro, PinKind, WaveType, WaveWarpParams,
                };
                let params = WaveWarpParams {
                    wave_height: wh,
                    wave_width: ww,
                    speed: sp,
                    time,
                    direction_deg: direction_deg.evaluate(frame),
                    wave_type: match wave_type {
                        1 => WaveType::Triangle,
                        2 => WaveType::Square,
                        3 => WaveType::Sawtooth,
                        _ => WaveType::Sine,
                    },
                    pinning: match pinning {
                        1 => PinKind::LeftRight,
                        2 => PinKind::TopBottom,
                        3 => PinKind::None,
                        _ => PinKind::All,
                    },
                    ..Default::default()
                };
                apply_wave_warp_pro(pixels, width, height, &params);
            }
        }
        EffectType::CcLens { convergence, zoom } => {
            let cv = convergence.evaluate(frame);
            let z = zoom.evaluate(frame);
            if !crate::core::compute_pipeline::try_gpu_cc_lens(pixels, width, height, cv, z) {
                use crate::core::ae_effects_pack_v27::{apply_cc_lens_pro, CcLensParams};
                apply_cc_lens_pro(
                    pixels,
                    width,
                    height,
                    &CcLensParams {
                        convergence: cv,
                        zoom: z,
                    },
                );
            }
        }
        EffectType::PolarCoordinates {
            to_polar,
            interpolation,
        } => {
            if !crate::core::compute_pipeline::try_gpu_polar_coords(
                pixels, width, height, *to_polar,
            ) {
                use crate::core::ae_effects_pack_v27::{apply_polar_coordinates_pro, PolarMode};
                let mode = if *to_polar {
                    PolarMode::RectToPolar
                } else {
                    PolarMode::PolarToRect
                };
                apply_polar_coordinates_pro(
                    pixels,
                    width,
                    height,
                    mode,
                    interpolation.evaluate(frame),
                );
            }
        }
        EffectType::OpticsCompensation {
            field_of_view_deg,
            reverse,
            zoom,
        } => {
            let fov = field_of_view_deg.evaluate(frame);
            if !crate::core::compute_pipeline::try_gpu_optics_comp(
                pixels, width, height, fov, *reverse,
            ) {
                use crate::core::ae_effects_pack_v27::{
                    apply_optics_compensation, OpticsCompensationParams,
                };
                apply_optics_compensation(
                    pixels,
                    width,
                    height,
                    &OpticsCompensationParams {
                        field_of_view_deg: fov,
                        reverse: *reverse,
                        zoom: zoom.evaluate(frame),
                    },
                );
            }
        }
        EffectType::ColorBalance {
            shadows,
            midtones,
            highlights,
            preserve_luminosity,
        } => {
            let s = *shadows;
            let m = *midtones;
            let h = *highlights;
            let shift = [s[0] + m[0] + h[0], s[1] + m[1] + h[1], s[2] + m[2] + h[2]];
            if !crate::core::compute_pipeline::try_gpu_color_balance(pixels, width, height, shift) {
                use crate::core::color_correction::{apply_color_balance, ColorBalance};
                apply_color_balance(
                    pixels,
                    &ColorBalance {
                        shadows: s,
                        midtones: m,
                        highlights: h,
                        preserve_luminosity: *preserve_luminosity,
                    },
                );
            }
        }
        EffectType::ChannelMixer { matrix, monochrome } => {
            use crate::core::color_correction::{apply_channel_mixer, ChannelMixer};
            apply_channel_mixer(
                pixels,
                &ChannelMixer {
                    matrix: *matrix,
                    monochrome: *monochrome,
                },
            );
        }
        EffectType::LightSweep {
            direction_deg,
            center,
            width: sweep_width,
            sweep_intensity,
            edge_intensity,
        } => {
            use crate::core::ae_effects_pack_v28::{apply_light_sweep, LightSweepParams};
            apply_light_sweep(
                pixels,
                width,
                height,
                &LightSweepParams {
                    direction_deg: direction_deg.evaluate(frame),
                    center: center.evaluate(frame),
                    width: sweep_width.evaluate(frame),
                    sweep_intensity: sweep_intensity.evaluate(frame),
                    edge_intensity: edge_intensity.evaluate(frame),
                },
            );
        }
        EffectType::RadialFastBlur { amount, samples } => {
            use crate::core::ae_effects_pack_v28::apply_radial_fast_blur;
            let cx = width as f32 * 0.5;
            let cy = height as f32 * 0.5;
            apply_radial_fast_blur(
                pixels,
                width,
                height,
                [cx, cy],
                amount.evaluate(frame),
                *samples,
            );
        }
        EffectType::BendIt {
            top_offset,
            bottom_offset,
        } => {
            let t = top_offset.evaluate(frame);
            let b = bottom_offset.evaluate(frame);
            if !crate::core::compute_pipeline::try_gpu_bend_it(pixels, width, height, t, b) {
                use crate::core::ae_effects_pack_v28::apply_cc_bend_it_pro;
                apply_cc_bend_it_pro(pixels, width, height, t, b);
            }
        }
        EffectType::Tiler {
            scale_percent,
            mirror,
        } => {
            let sc = scale_percent.evaluate(frame) / 100.0;
            if !crate::core::compute_pipeline::try_gpu_tiler(pixels, width, height, sc) {
                use crate::core::ae_effects_pack_v28::{apply_cc_tiler_pro, TileEdgeMode};
                let mode = if *mirror {
                    TileEdgeMode::Mirror
                } else {
                    TileEdgeMode::Repeat
                };
                apply_cc_tiler_pro(pixels, width, height, scale_percent.evaluate(frame), mode);
            }
        }
        EffectType::Tritone {
            shadow_color,
            mid_color,
            highlight_color,
        } => {
            let s = shadow_color.evaluate(frame);
            let m = mid_color.evaluate(frame);
            let h = highlight_color.evaluate(frame);
            if !crate::core::compute_pipeline::try_gpu_tritone(pixels, width, height, s, m, h) {
                let to_c3 = |c: [f32; 3]| {
                    [
                        (c[0].clamp(0.0, 1.0) * 255.0).round() as u8,
                        (c[1].clamp(0.0, 1.0) * 255.0).round() as u8,
                        (c[2].clamp(0.0, 1.0) * 255.0).round() as u8,
                    ]
                };
                pack::apply_tritone(pixels, to_c3(s), to_c3(m), to_c3(h));
            }
        }
        EffectType::MatteChoker {
            choke_amount,
            gray_level,
        } => {
            pack::apply_matte_choker(
                pixels,
                choke_amount.evaluate(frame),
                gray_level.evaluate(frame),
            );
        }
        EffectType::VenetianBlinds {
            completion,
            width: blind_width,
        } => {
            let c = completion.evaluate(frame);
            let bw = blind_width.evaluate(frame).max(1.0);
            if !crate::core::compute_pipeline::try_gpu_venetian_blinds(pixels, width, height, c, bw)
            {
                pack::apply_venetian_blinds(pixels, width, height, c, bw as u32);
            }
        }
        EffectType::Vibrance { amount } => {
            let a = amount.evaluate(frame);
            if !crate::core::compute_pipeline::try_gpu_vibrance(pixels, width, height, a) {
                crate::core::color_correction::apply_vibrance(pixels, a);
            }
        }
        EffectType::WhiteBalance { temperature, tint } => {
            let t = temperature.evaluate(frame);
            let ti = tint.evaluate(frame);
            if !crate::core::compute_pipeline::try_gpu_white_balance(pixels, width, height, t, ti) {
                use crate::core::color_correction::{apply_white_balance, WhiteBalance};
                apply_white_balance(
                    pixels,
                    &WhiteBalance {
                        temperature: t,
                        tint: ti,
                    },
                );
            }
        }
        EffectType::HslAdjust {
            hue_deg,
            saturation,
            lightness,
        } => {
            let h = hue_deg.evaluate(frame);
            let s = saturation.evaluate(frame);
            let l = lightness.evaluate(frame);
            if !crate::core::compute_pipeline::try_gpu_hsl_adjust(pixels, width, height, h, s, l) {
                use crate::core::color_correction::{apply_hsl_adjust, HslAdjust};
                apply_hsl_adjust(
                    pixels,
                    &HslAdjust {
                        hue_deg: h,
                        saturation: s,
                        lightness: l,
                    },
                );
            }
        }
        EffectType::GlowPro {
            threshold,
            radius,
            intensity,
        } => {
            use crate::core::ae_effects_pack_v28::apply_glow_pro;
            apply_glow_pro(
                pixels,
                width,
                height,
                threshold.evaluate(frame),
                radius.evaluate(frame).round().clamp(0.0, 128.0) as u32,
                intensity.evaluate(frame),
            );
        }
        EffectType::VRHorizon {
            pitch_deg,
            yaw_deg,
            roll_deg,
        } => {
            crate::core::vr_effects::apply_vr_horizon(
                pixels,
                width,
                height,
                pitch_deg.evaluate(frame),
                yaw_deg.evaluate(frame),
                roll_deg.evaluate(frame),
            );
        }
        EffectType::VRSphericalBlur { radius } => {
            crate::core::vr_effects::apply_vr_blur(
                pixels,
                width,
                height,
                radius.evaluate(frame),
            );
        }
        EffectType::TemporalDenoise {
            strength,
            temporal_mix,
            feature_threshold,
        } => {
            let mut denoiser = crate::core::temporal_denoise::TemporalDenoiser::new();
            denoiser.set_strength(strength.evaluate(frame));
            denoiser.set_temporal_mix(temporal_mix.evaluate(frame));
            denoiser.set_feature_threshold(feature_threshold.evaluate(frame));
            denoiser.process(pixels, width, height);
        }
        _ => unreachable!(),
    }
    true
}
