use super::*;

pub(super) fn apply(
    pixels: &mut [u8],
    effect_type: &EffectType,
    context: LayerEffectRenderContext<'_>,
) -> bool {
    if !matches!(
        effect_type,
        EffectType::ChromaKey { .. }
            | EffectType::Spherize { .. }
            | EffectType::TurbulentDisplace { .. }
            | EffectType::Colorama { .. }
    ) {
        return false;
    }
    let LayerEffectRenderContext {
        width,
        height,
        frame,
        ..
    } = context;

    match effect_type {
        // Effects from standalone core modules.
        EffectType::ChromaKey {
            screen_color,
            screen_gain,
            clip_black,
            clip_white,
        } => {
            let sc = screen_color.evaluate(frame);
            let gain = screen_gain.evaluate(frame);
            let softness = (clip_white.evaluate(frame) - clip_black.evaluate(frame)).max(0.01);
            if !crate::core::compute_pipeline::try_gpu_chroma_key(
                pixels, width, height, sc, gain, softness,
            ) {
                let opts = crate::core::chroma_key::ChromaKeyOptions {
                    screen_color: sc,
                    screen_gain: gain,
                    screen_balance: 0.5,
                    despill_strength: 0.8,
                    clip_black: clip_black.evaluate(frame),
                    clip_white: clip_white.evaluate(frame),
                };
                crate::core::chroma_key::apply_chroma_key(pixels, width, height, &opts);
            }
        }
        EffectType::Spherize {
            radius,
            refractive_index,
        } => {
            let r = radius.evaluate(frame);
            let ri = refractive_index.evaluate(frame);
            let cx = width as f32 * 0.5;
            let cy = height as f32 * 0.5;
            if !crate::core::compute_pipeline::try_gpu_spherize(pixels, width, height, ri, cx, cy) {
                let opts = crate::core::spherize::SpherizeOptions {
                    radius: r,
                    center: [cx, cy],
                    refractive_index: ri,
                };
                let out = crate::core::spherize::apply_spherize(pixels, width, height, &opts);
                pixels.copy_from_slice(&out);
            }
        }
        EffectType::TurbulentDisplace {
            amount,
            size,
            evolution,
            complexity,
        } => {
            let amt = amount.evaluate(frame);
            let sz = size.evaluate(frame);
            if !crate::core::compute_pipeline::try_gpu_turbulent_displace(
                pixels,
                width,
                height,
                amt,
                sz.max(1.0),
            ) {
                let opts = crate::core::turbulent_displace::TurbulentDisplaceOptions {
                    displace_type:
                        crate::core::turbulent_displace::TurbulentDisplaceType::Turbulent,
                    amount: amt,
                    size: sz,
                    evolution_deg: evolution.evaluate(frame),
                    complexity: complexity.evaluate(frame).clamp(1.0, 100.0) as u32,
                };
                let out = crate::core::turbulent_displace::apply_turbulent_displace(
                    pixels, width, height, &opts,
                );
                pixels.copy_from_slice(&out);
            }
        }
        EffectType::Colorama {
            preset_index,
            cycle_phase,
        } => {
            let cp = cycle_phase.evaluate(frame);
            if !crate::core::compute_pipeline::try_gpu_colorama(pixels, width, height, cp, 1.0, 1.0)
            {
                let idx = preset_index.evaluate(frame).round() as u32 % 4;
                let preset = match idx {
                    0 => crate::core::colorama::ColoramaPreset::Rainbow,
                    1 => crate::core::colorama::ColoramaPreset::Heatmap,
                    2 => crate::core::colorama::ColoramaPreset::Sepia,
                    _ => crate::core::colorama::ColoramaPreset::Solarize,
                };
                crate::core::colorama::apply_colorama(pixels, width, height, preset, cp);
            }
        }

        _ => unreachable!(),
    }
    true
}
