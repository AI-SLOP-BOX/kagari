use super::*;

pub(super) fn apply(
    pixels: &mut [u8],
    effect_type: &EffectType,
    context: LayerEffectRenderContext<'_>,
) -> bool {
    if !matches!(
        effect_type,
        EffectType::CrtScanlines { .. }
            | EffectType::Vortex { .. }
            | EffectType::HeatDistortion { .. }
            | EffectType::RainRipples { .. }
            | EffectType::Fisheye { .. }
            | EffectType::LensCorrection { .. }
            | EffectType::GlitchDisplacement { .. }
            | EffectType::RGBSplit { .. }
            | EffectType::Flicker { .. }
            | EffectType::BlockGlitch { .. }
            | EffectType::SliceTear { .. }
            | EffectType::MatteChokeSpread { .. }
            | EffectType::AlphaFeather { .. }
            | EffectType::AlphaFromLuminance { .. }
            | EffectType::NightVision { .. }
            | EffectType::IrisWipe { .. }
            | EffectType::RadialWipe { .. }
            | EffectType::FilmEmulation { .. }
            | EffectType::GodRays { .. }
            | EffectType::AudioSpectrum { .. }
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
    match effect_type {
        EffectType::CrtScanlines {
            line_spacing,
            intensity,
        } => {
            let sp = line_spacing.evaluate(frame).round().clamp(1.0, 200.0);
            let inten = intensity.evaluate(frame);
            if !crate::core::compute_pipeline::try_gpu_crt_scanlines(
                pixels, width, height, sp, inten,
            ) {
                use crate::core::ae_effects_pack_v12::apply_crt_scanlines;
                apply_crt_scanlines(pixels, width, height, sp as u32, inten);
            }
        }
        EffectType::Vortex { radius, angle_deg } => {
            let r = radius.evaluate(frame).clamp(1.0, 10000.0);
            let a = angle_deg.evaluate(frame);
            if !crate::core::compute_pipeline::try_gpu_vortex(pixels, width, height, a, r) {
                use crate::core::ae_effects_pack_v13::apply_vortex_distortion;
                let cx = width as f32 * 0.5;
                let cy = height as f32 * 0.5;
                apply_vortex_distortion(pixels, width, height, [cx, cy], r, a);
            }
        }
        EffectType::HeatDistortion { strength, speed } => {
            use crate::core::ae_effects_pack_v13::apply_heat_distortion;
            let time = frame as f32 / fps.max(1) as f32 * speed.evaluate(frame);
            apply_heat_distortion(pixels, width, height, time, strength.evaluate(frame));
        }
        EffectType::RainRipples {
            drop_count,
            wave_strength,
        } => {
            use crate::core::ae_effects_pack_v12::apply_rain_ripples;
            apply_rain_ripples(
                pixels,
                width,
                height,
                frame,
                drop_count.evaluate(frame).round().clamp(0.0, 100.0) as u32,
                wave_strength.evaluate(frame),
            );
        }
        EffectType::Fisheye { strength } => {
            let s = strength.evaluate(frame);
            if !crate::core::compute_pipeline::try_gpu_fisheye(pixels, width, height, s) {
                use crate::core::ae_effects_pack_v21::apply_fisheye;
                apply_fisheye(pixels, width, height, s);
            }
        }
        EffectType::LensCorrection { k1, k2 } => {
            let k1v = k1.evaluate(frame);
            let k2v = k2.evaluate(frame);
            if !crate::core::compute_pipeline::try_gpu_lens_correction(
                pixels, width, height, k1v, k2v,
            ) {
                use crate::core::ae_effects_pack_v21::apply_barrel_correction;
                apply_barrel_correction(pixels, width, height, k1v, k2v);
            }
        }
        EffectType::GlitchDisplacement { seed, amount } => {
            let s = seed.evaluate(frame).round().clamp(0.0, 99999.0);
            let a = amount.evaluate(frame);
            if !crate::core::compute_pipeline::try_gpu_glitch_displacement(
                pixels, width, height, a, s,
            ) {
                use crate::core::ae_effects_pack_v13::apply_glitch_displacement;
                apply_glitch_displacement(pixels, width, height, s as u32, a);
            }
        }
        EffectType::RGBSplit {
            red_offset,
            green_offset,
            blue_offset,
        } => {
            use crate::core::glitch_effects::apply_rgb_split;
            apply_rgb_split(
                pixels,
                width,
                height,
                red_offset.evaluate(frame),
                green_offset.evaluate(frame),
                blue_offset.evaluate(frame),
            );
        }
        EffectType::Flicker {
            amount,
            speed,
            seed,
        } => {
            use crate::core::glitch_effects::apply_flicker;
            apply_flicker(
                pixels,
                amount.evaluate(frame),
                speed.evaluate(frame),
                seed.evaluate(frame).round().clamp(0.0, 99999.0) as u32,
                frame,
                fps,
            );
        }
        EffectType::BlockGlitch {
            block_size,
            amount,
            seed,
            corruption,
        } => {
            use crate::core::glitch_effects::apply_block_glitch;
            apply_block_glitch(
                pixels,
                width,
                height,
                block_size.evaluate(frame),
                amount.evaluate(frame),
                seed.evaluate(frame).round().clamp(0.0, 99999.0) as u32,
                frame,
                corruption.evaluate(frame),
            );
        }
        EffectType::SliceTear {
            slices,
            max_offset,
            seed,
            vertical,
        } => {
            use crate::core::glitch_effects::apply_slice_tear;
            apply_slice_tear(
                pixels,
                width,
                height,
                slices.evaluate(frame),
                max_offset.evaluate(frame),
                seed.evaluate(frame).round().clamp(0.0, 99999.0) as u32,
                frame,
                *vertical,
            );
        }
        EffectType::MatteChokeSpread { radius, expand } => {
            use crate::core::ae_effects_pack_v22::apply_matte_choke;
            apply_matte_choke(
                pixels,
                width,
                height,
                radius.evaluate(frame).round().clamp(1.0, 64.0) as u32,
                *expand,
            );
        }
        EffectType::AlphaFeather { radius } => {
            use crate::core::ae_effects_pack_v22::apply_alpha_feather;
            apply_alpha_feather(
                pixels,
                width,
                height,
                radius.evaluate(frame).round().clamp(1.0, 64.0) as u32,
            );
        }
        EffectType::AlphaFromLuminance { invert } => {
            if !crate::core::compute_pipeline::try_gpu_alpha_from_luminance(
                pixels, width, height, *invert,
            ) {
                use crate::core::ae_effects_pack_v22::apply_alpha_from_luminance;
                apply_alpha_from_luminance(pixels, *invert);
            }
        }
        EffectType::NightVision { amplification } => {
            use crate::core::ae_effects_pack_v20::apply_night_vision;
            // Seed advances with the frame: deterministic per frame, animated over time.
            apply_night_vision(
                pixels,
                amplification.evaluate(frame),
                frame.wrapping_mul(2654435761),
            );
        }
        EffectType::IrisWipe { completion } => {
            use crate::core::ae_effects_pack_v2::apply_iris_wipe;
            apply_iris_wipe(pixels, width, height, completion.evaluate(frame));
        }
        EffectType::RadialWipe { completion } => {
            use crate::core::ae_effects_pack_v2::apply_radial_wipe;
            apply_radial_wipe(pixels, width, height, completion.evaluate(frame));
        }
        EffectType::FilmEmulation {
            lift,
            gamma,
            gain,
            hue_shift_deg,
        } => {
            use crate::core::ae_effects_pack_v20::apply_film_emulation;
            apply_film_emulation(
                pixels,
                lift.evaluate(frame),
                gamma.evaluate(frame).max(0.01),
                gain.evaluate(frame).max(0.0),
                hue_shift_deg.evaluate(frame),
            );
        }
        EffectType::GodRays {
            sun_x,
            sun_y,
            samples,
            decay,
            weight,
        } => {
            use crate::core::ae_effects_pack_v25::apply_god_rays;
            let sun = [
                sun_x.evaluate(frame).clamp(0.0, 1.0) * width as f32,
                sun_y.evaluate(frame).clamp(0.0, 1.0) * height as f32,
            ];
            apply_god_rays(
                pixels,
                width,
                height,
                sun,
                samples.evaluate(frame).round().clamp(1.0, 64.0) as u32,
                decay.evaluate(frame).clamp(0.5, 1.0),
                weight.evaluate(frame).max(0.0),
            );
        }
        EffectType::AudioSpectrum {
            enabled,
            bands,
            opacity,
            color_start,
            color_end,
            position_x,
            position_y,
            width: spec_w,
            height: spec_h,
        } => {
            if enabled.evaluate(frame) <= 0.5 || opacity.evaluate(frame) <= 0.01 {
                return true;
            }
            let bands_n = bands.evaluate(frame).clamp(1.0, 5.0) as usize;
            let opacity_n = opacity.evaluate(frame).clamp(0.0, 1.0);
            let cs = *color_start;
            let ce = *color_end;
            let px = position_x.evaluate(frame).clamp(0.0, 1.0);
            let py = position_y.evaluate(frame).clamp(0.0, 1.0);
            let w_frac = spec_w.evaluate(frame).clamp(0.01, 1.0);
            let h_frac = spec_h.evaluate(frame).clamp(0.01, 1.0);
            let x0 = (px * w_frac * width as f32).floor().max(0.0) as u32;
            let y0 = (py * h_frac * height as f32).floor().max(0.0) as u32;
            let x1 = ((px + w_frac) * width as f32).ceil().min(width as f32) as u32;
            let y1 = ((py + h_frac) * height as f32).ceil().min(height as f32) as u32;
            if x1 <= x0 || y1 <= y0 {
                return true;
            }
            let strip_h = y1 - y0;
            for b in 0..bands_n {
                let amp = crate::core::expression_engine::get_audio_band(b).clamp(0.0, 1.0);
                let bar_h = (amp * strip_h as f32) as u32;
                let bx0 = x0 + (b as u32) * (x1 - x0) / bands_n as u32;
                let bx1 = x0 + (b as u32 + 1) * (x1 - x0) / bands_n as u32;
                let t = b as f32 / (bands_n.saturating_sub(1).max(1)) as f32;
                let r = ((cs[0] + (ce[0] - cs[0]) * t) * 255.0).round() as u8;
                let g = ((cs[1] + (ce[1] - cs[1]) * t) * 255.0).round() as u8;
                let bb = ((cs[2] + (ce[2] - cs[2]) * t) * 255.0).round() as u8;
                let a = ((cs[3] + (ce[3] - cs[3]) * t) * 255.0 * opacity_n).round() as u8;
                for y in (y1.saturating_sub(bar_h))..y1 {
                    for x in bx0..bx1 {
                        if x < width && y < height {
                            let idx = ((y * width + x) * 4) as usize;
                            if idx + 3 < pixels.len() {
                                let src_a = pixels[idx + 3] as f32 / 255.0;
                                let out_a = (a as f32 / 255.0 * (1.0 - src_a)).clamp(0.0, 1.0);
                                pixels[idx] = (r as f32 * out_a).round() as u8;
                                pixels[idx + 1] = (g as f32 * out_a).round() as u8;
                                pixels[idx + 2] = (bb as f32 * out_a).round() as u8;
                                pixels[idx + 3] = (out_a * 255.0).round() as u8;
                            }
                        }
                    }
                }
            }
        }
        _ => unreachable!(),
    }
    true
}
