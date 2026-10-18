use super::*;

pub(super) fn apply(
    pixels: &mut [u8],
    effect_type: &EffectType,
    context: LayerEffectRenderContext<'_>,
) -> bool {
    if !matches!(
        effect_type,
        EffectType::Emboss { .. }
            | EffectType::StarField { .. }
            | EffectType::LightningArc { .. }
            | EffectType::LaserBeam { .. }
            | EffectType::FireAutomaton { .. }
            | EffectType::LumaKeyRange { .. }
            | EffectType::Halftone { .. }
            | EffectType::Solarize { .. }
            | EffectType::PixelSort { .. }
            | EffectType::PinchPunch { .. }
            | EffectType::ScanlineGlitch { .. }
            | EffectType::GlassEdgeBevel { .. }
            | EffectType::DirectionalSharpen { .. }
            | EffectType::RefractionLens { .. }
            | EffectType::GradientMap { .. }
            | EffectType::LightLeak { .. }
            | EffectType::BevelAlpha { .. }
            | EffectType::CrossHatch { .. }
            | EffectType::CmykHalftone { .. }
            | EffectType::ReflectionMap { .. }
            | EffectType::PerlinFlow { .. }
            | EffectType::FbmTurbulence { .. }
            | EffectType::Letterbox { .. }
            | EffectType::SliderControl { .. }
            | EffectType::AngleControl { .. }
            | EffectType::PointControl { .. }
            | EffectType::ColorControl { .. }
            | EffectType::CheckboxControl { .. }
            | EffectType::DropdownControl { .. }
            | EffectType::Point3DControl { .. }
    ) {
        return false;
    }
    let LayerEffectRenderContext {
        comp,
        layer_idx,
        width,
        height,
        frame,
        fps,
        light_screen,
    } = context;
    let _ = (comp, layer_idx, width, height, frame, fps, light_screen);
    match effect_type {
        EffectType::Emboss { angle_deg, depth } => {
            let a = angle_deg.evaluate(frame);
            let d = depth.evaluate(frame);
            if !crate::core::compute_pipeline::try_gpu_emboss(pixels, width, height, d) {
                use crate::core::ae_effects_pack_v17::apply_emboss;
                apply_emboss(pixels, width, height, a, d);
            }
        }
        EffectType::StarField {
            num_stars,
            depth_speed,
        } => {
            use crate::core::ae_effects_pack_v18::apply_star_field;
            apply_star_field(
                pixels,
                width,
                height,
                num_stars.evaluate(frame).round().clamp(1.0, 2000.0) as u32,
                depth_speed.evaluate(frame),
                frame as f32 / fps.max(1) as f32,
            );
        }
        EffectType::LightningArc {
            start_x,
            start_y,
            end_x,
            end_y,
            seed,
            glow,
        } => {
            let start = [
                start_x.evaluate(frame).clamp(0.0, 1.0) * width as f32,
                start_y.evaluate(frame).clamp(0.0, 1.0) * height as f32,
            ];
            let end = [
                end_x.evaluate(frame).clamp(0.0, 1.0) * width as f32,
                end_y.evaluate(frame).clamp(0.0, 1.0) * height as f32,
            ];
            let bolt_seed =
                seed.evaluate(frame).round().clamp(0.0, 99999.0) as u64 ^ (frame as u64);
            let glow_val = glow.evaluate(frame).clamp(0.0, 5.0);
            let config = crate::core::lightning_beam_engine::AdvancedLightningConfig {
                origin: start,
                destination: end,
                seed: bolt_seed,
                segments: 5,
                displacement_amplitude: 35.0,
                branch_probability: 0.35,
                main_thickness: 2.5 + glow_val,
                glow_color: [0.3, 0.6, 1.0, 0.8],
                core_color: [1.0, 1.0, 1.0, 1.0],
            };
            crate::core::lightning_beam_engine::render_lightning_to_buffer(
                pixels, width, height, &config,
            );
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
            let start = [
                start_x.evaluate(frame).clamp(0.0, 1.0) * width as f32,
                start_y.evaluate(frame).clamp(0.0, 1.0) * height as f32,
            ];
            let end = [
                end_x.evaluate(frame).clamp(0.0, 1.0) * width as f32,
                end_y.evaluate(frame).clamp(0.0, 1.0) * height as f32,
            ];
            let config = crate::core::lightning_beam_engine::LaserBeamConfig {
                start_point: start,
                end_point: end,
                time_progress: progress.evaluate(frame).clamp(0.0, 1.0),
                beam_length_percent: length.evaluate(frame).clamp(0.0, 100.0),
                starting_thickness: starting_thickness.evaluate(frame).max(0.1),
                ending_thickness: ending_thickness.evaluate(frame).max(0.1),
                core_color: core_color.evaluate(frame),
                glow_color: glow_color.evaluate(frame),
            };
            crate::core::lightning_beam_engine::render_laser_beam_to_buffer(
                pixels, width, height, &config,
            );
        }
        EffectType::FireAutomaton { intensity } => {
            use crate::core::ae_effects_pack_v18::apply_fire_automaton;
            apply_fire_automaton(pixels, width, height, intensity.evaluate(frame));
        }
        EffectType::LumaKeyRange {
            low_threshold,
            high_threshold,
            invert,
        } => {
            let low = low_threshold.evaluate(frame).round().clamp(0.0, 255.0) / 255.0;
            let high = high_threshold.evaluate(frame).round().clamp(0.0, 255.0) / 255.0;
            if !crate::core::compute_pipeline::try_gpu_luma_key_range(
                pixels, width, height, low, high, *invert,
            ) {
                use crate::core::ae_effects_pack_v16::apply_luma_key_range;
                apply_luma_key_range(pixels, (low * 255.0) as u8, (high * 255.0) as u8, *invert);
            }
        }
        EffectType::Halftone { cell_size } => {
            let cs = cell_size.evaluate(frame).round().clamp(2.0, 64.0);
            if !crate::core::compute_pipeline::try_gpu_halftone(pixels, width, height, cs) {
                use crate::core::ae_effects_pack_v14::apply_halftone_screen;
                apply_halftone_screen(pixels, width, height, cs as u32);
            }
        }
        EffectType::Solarize { threshold } => {
            let t = threshold.evaluate(frame).round().clamp(0.0, 255.0);
            if !crate::core::compute_pipeline::try_gpu_solarize(pixels, width, height, t / 255.0) {
                use crate::core::ae_effects_pack_v14::apply_solarize_effect;
                apply_solarize_effect(pixels, t as u8);
            }
        }
        EffectType::PixelSort { threshold } => {
            use crate::core::ae_effects_pack_v14::apply_pixel_sort_glitch;
            apply_pixel_sort_glitch(
                pixels,
                width,
                height,
                threshold.evaluate(frame).round().clamp(0.0, 255.0) as u8,
            );
        }
        EffectType::PinchPunch { radius, amount } => {
            let amt = amount.evaluate(frame);
            if !crate::core::compute_pipeline::try_gpu_pinch_punch(pixels, width, height, amt) {
                use crate::core::ae_effects_pack_v15::apply_pinch_punch_distortion;
                apply_pinch_punch_distortion(
                    pixels,
                    width,
                    height,
                    [width as f32 * 0.5, height as f32 * 0.5],
                    radius.evaluate(frame).max(1.0),
                    amt,
                );
            }
        }
        EffectType::ScanlineGlitch {
            jitter_amount,
            seed,
        } => {
            let ja = jitter_amount.evaluate(frame);
            let s = (seed.evaluate(frame).round().clamp(0.0, 99999.0) as u32 ^ frame) as f32;
            if !crate::core::compute_pipeline::try_gpu_scanline_glitch(pixels, width, height, ja, s)
            {
                use crate::core::ae_effects_pack_v15::apply_scanline_glitch_jitter;
                apply_scanline_glitch_jitter(pixels, width, height, ja, s as u32);
            }
        }
        EffectType::GlassEdgeBevel {
            bevel_size,
            refraction,
        } => {
            use crate::core::ae_effects_pack_v12::apply_glass_edge_bevel;
            apply_glass_edge_bevel(
                pixels,
                width,
                height,
                bevel_size.evaluate(frame).round().clamp(1.0, 128.0) as u32,
                refraction.evaluate(frame),
            );
        }
        EffectType::DirectionalSharpen {
            angle_deg,
            strength,
        } => {
            let a = angle_deg.evaluate(frame).to_radians();
            let s = strength.evaluate(frame);
            if !crate::core::compute_pipeline::try_gpu_directional_sharpen(
                pixels, width, height, a, s,
            ) {
                use crate::core::ae_effects_pack_v15::apply_directional_sharpen;
                apply_directional_sharpen(pixels, width, height, a, s);
            }
        }
        EffectType::RefractionLens { radius, ior } => {
            let i = ior.evaluate(frame);
            if !crate::core::compute_pipeline::try_gpu_refraction(pixels, width, height, i) {
                use crate::core::ae_effects_pack_v16::apply_spherical_refraction_lens;
                apply_spherical_refraction_lens(
                    pixels,
                    width,
                    height,
                    [width as f32 * 0.5, height as f32 * 0.5],
                    radius.evaluate(frame).max(1.0),
                    i,
                );
            }
        }
        EffectType::GradientMap {
            low_color,
            mid_color,
            high_color,
        } => {
            let l = low_color.evaluate(frame);
            let m = mid_color.evaluate(frame);
            let h = high_color.evaluate(frame);
            if !crate::core::compute_pipeline::try_gpu_gradient_map(pixels, width, height, l, m, h)
            {
                use crate::core::ae_effects_pack_v15::apply_gradient_map_color;
                let to_c3 = |c: [f32; 3]| {
                    [
                        (c[0].clamp(0.0, 1.0) * 255.0).round() as u8,
                        (c[1].clamp(0.0, 1.0) * 255.0).round() as u8,
                        (c[2].clamp(0.0, 1.0) * 255.0).round() as u8,
                    ]
                };
                apply_gradient_map_color(pixels, to_c3(l), to_c3(m), to_c3(h));
            }
        }
        EffectType::LightLeak {
            pos_x,
            pos_y,
            intensity,
        } => {
            use crate::core::ae_effects_pack_v14::apply_light_leak_synth;
            let pos = [
                pos_x.evaluate(frame).clamp(0.0, 1.0) * width as f32,
                pos_y.evaluate(frame).clamp(0.0, 1.0) * height as f32,
            ];
            // Warm cinematic leak colour (fixed tint; position/intensity animate).
            apply_light_leak_synth(
                pixels,
                width,
                height,
                pos,
                intensity.evaluate(frame),
                [255, 180, 90],
            );
        }
        EffectType::BevelAlpha {
            depth,
            light_angle_deg,
        } => {
            let d = depth.evaluate(frame);
            let a = light_angle_deg.evaluate(frame).to_radians();
            if !crate::core::compute_pipeline::try_gpu_bevel_alpha(pixels, width, height, d, a) {
                use crate::core::ae_effects_pack_v14::apply_bevel_alpha_3d;
                apply_bevel_alpha_3d(pixels, width, height, d.round().clamp(1.0, 32.0) as u32, a);
            }
        }
        EffectType::CrossHatch {
            line_gap,
            threshold,
        } => {
            let gap = line_gap.evaluate(frame).round().clamp(2.0, 64.0);
            let thr = threshold.evaluate(frame).round().clamp(0.0, 255.0) / 255.0;
            if !crate::core::compute_pipeline::try_gpu_crosshatch(pixels, width, height, gap, thr) {
                use crate::core::ae_effects_pack_v17::apply_cross_hatch;
                apply_cross_hatch(pixels, width, height, gap as u32, (thr * 255.0) as u8);
            }
        }
        EffectType::CmykHalftone { dot_size } => {
            use crate::core::ae_effects_pack_v16::apply_color_halftone_cmyk;
            apply_color_halftone_cmyk(
                pixels,
                width,
                height,
                dot_size.evaluate(frame).round().clamp(2.0, 64.0) as u32,
            );
        }
        EffectType::ReflectionMap {
            reflect_y,
            fade_dist,
            opacity,
        } => {
            let ry = reflect_y.evaluate(frame) > 0.5;
            let fd = fade_dist.evaluate(frame).max(1.0);
            if !crate::core::compute_pipeline::try_gpu_reflection_map(pixels, width, height, ry, fd)
            {
                use crate::core::ae_effects_pack_v21::apply_reflection_map;
                apply_reflection_map(
                    pixels,
                    width,
                    height,
                    ry as u32,
                    fd,
                    opacity.evaluate(frame).clamp(0.0, 1.0),
                );
            }
        }
        EffectType::PerlinFlow { scale } => {
            use crate::core::ae_effects_pack_v16::apply_perlin_flow_noise;
            apply_perlin_flow_noise(
                pixels,
                width,
                height,
                frame as f32 / fps.max(1) as f32,
                scale.evaluate(frame).max(0.01),
            );
        }
        EffectType::FbmTurbulence { octaves, amplitude } => {
            use crate::core::ae_effects_pack_v18::apply_fbm_turbulence;
            apply_fbm_turbulence(
                pixels,
                width,
                height,
                octaves.evaluate(frame).round().clamp(1.0, 8.0) as u32,
                amplitude.evaluate(frame),
                frame as f32 / fps.max(1) as f32,
            );
        }
        EffectType::Letterbox { frac } => {
            let f = frac.evaluate(frame);
            if !crate::core::compute_pipeline::try_gpu_letterbox(pixels, width, height, f) {
                use crate::core::ae_effects_pack_v24::apply_letterbox;
                apply_letterbox(pixels, width, height, f);
            }
        }
        // Expression Controls: non-rendering utility effects (values are read
        // by the expression engine via effect_param), so CPU pass is a no-op.
        EffectType::SliderControl { .. } => {}
        EffectType::AngleControl { .. } => {}
        EffectType::PointControl { .. } => {}
        EffectType::ColorControl { .. } => {}
        EffectType::CheckboxControl { .. } => {}
        EffectType::DropdownControl { .. } => {}
        EffectType::Point3DControl { .. } => {}
        _ => unreachable!(),
    }
    true
}
