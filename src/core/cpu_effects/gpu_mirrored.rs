use super::*;

pub(super) fn apply(
    pixels: &mut [u8],
    effect_type: &EffectType,
    context: LayerEffectRenderContext<'_>,
) -> bool {
    if !matches!(
        effect_type,
        EffectType::GaussianBlur { .. }
            | EffectType::ColorTint { .. }
            | EffectType::DropShadow { .. }
            | EffectType::Glow { .. }
    ) {
        return false;
    }
    let LayerEffectRenderContext {
        width,
        height,
        frame,
        ..
    } = context;
    use crate::core::ae_effects_pack as pack;

    match effect_type {
        // Effects already present in the GPU pipeline, mirrored on CPU.
        EffectType::GaussianBlur { blur_radius } => {
            let r = blur_radius.evaluate(frame).max(0.0) as u32;
            // GPU compute path (opt-in via settings); CPU fallback keeps
            // byte-deterministic output when disabled or unavailable.
            let clamped = r.min(crate::core::compute_pipeline::MAX_BLUR_RADIUS);
            if !crate::core::compute_pipeline::try_gpu_gaussian_blur(pixels, width, height, clamped)
            {
                pack::apply_gaussian_blur(pixels, width, height, clamped);
            }
        }
        EffectType::ColorTint { color, intensity } => {
            let rgb = color3_to_u8(color.evaluate(frame));
            let amount = (intensity.evaluate(frame) / 100.0).clamp(0.0, 1.0);
            let tint_f3 = [
                rgb[0] as f32 / 255.0,
                rgb[1] as f32 / 255.0,
                rgb[2] as f32 / 255.0,
            ];
            if !crate::core::compute_pipeline::try_gpu_color_tint(
                pixels, width, height, tint_f3, amount,
            ) {
                pack::apply_tint(pixels, rgb, rgb, amount);
            }
        }
        EffectType::DropShadow {
            color,
            opacity,
            direction,
            distance,
            softness,
        } => {
            let mut sc = color_to_u8(color.evaluate(frame));
            sc[3] = (sc[3] as f32 * (opacity.evaluate(frame) / 100.0)).clamp(0.0, 255.0) as u8;
            let dist = distance.evaluate(frame).clamp(-10000.0, 10000.0);
            let dir = direction.evaluate(frame).clamp(-3600.0, 3600.0);
            let blur = softness.evaluate(frame).clamp(0.0, 1000.0) as u32;
            let sc_f32 = [
                sc[0] as f32 / 255.0,
                sc[1] as f32 / 255.0,
                sc[2] as f32 / 255.0,
                sc[3] as f32 / 255.0,
            ];
            if !crate::core::compute_pipeline::try_gpu_drop_shadow(
                pixels,
                width,
                height,
                sc_f32,
                dist,
                dir,
                blur.min(32),
            ) {
                pack::apply_drop_shadow(pixels, width, height, dist, dir, blur, sc);
            }
        }
        EffectType::Glow {
            threshold,
            radius,
            intensity,
            ..
        } => {
            let thr = threshold.evaluate(frame) / 100.0;
            let rad = radius.evaluate(frame).clamp(0.0, 1000.0) as u32;
            let inten = intensity.evaluate(frame) / 100.0;
            // Zero intensity adds no energy: skip the (expensive) blur.
            if inten > 0.001
                && !crate::core::compute_pipeline::try_gpu_glow(
                    pixels,
                    width,
                    height,
                    thr,
                    rad.min(32),
                    inten,
                )
            {
                pack::apply_glow(pixels, width, height, thr, rad, inten);
            }
        }

        _ => unreachable!(),
    }
    true
}
