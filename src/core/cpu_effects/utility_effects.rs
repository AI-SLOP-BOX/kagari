use super::*;

pub(super) fn apply(
    pixels: &mut [u8],
    effect_type: &EffectType,
    context: LayerEffectRenderContext<'_>,
) -> bool {
    if !matches!(
        effect_type,
        EffectType::LensFlare { .. }
            | EffectType::CustomShader { .. }
            | EffectType::MergePaths { .. }
            | EffectType::OffsetPath { .. }
            | EffectType::CustomPlugin { .. }
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
    use crate::core::ae_effects_pack as pack;
    match effect_type {
        EffectType::LensFlare {
            enabled,
            position_x,
            position_y,
            intensity,
            threshold,
            color,
            ..
        } => {
            if enabled.evaluate(frame) > 0.5 {
                let [fx, fy] = light_screen
                    .unwrap_or([position_x.evaluate(frame), position_y.evaluate(frame)]);
                let c = color.evaluate(frame);
                pack::apply_lens_flare(
                    pixels,
                    width,
                    height,
                    &pack::LensFlareParams {
                        pos_x: fx,
                        pos_y: fy,
                        intensity: intensity.evaluate(frame),
                        threshold: threshold.evaluate(frame),
                        color: [
                            (c[0] * 255.0).clamp(0.0, 255.0) as u8,
                            (c[1] * 255.0).clamp(0.0, 255.0) as u8,
                            (c[2] * 255.0).clamp(0.0, 255.0) as u8,
                        ],
                    },
                );
            }
        }
        EffectType::CustomShader {
            wgsl_source,
            uniform_values,
        } => {
            crate::core::custom_shader_cpu::apply(
                pixels,
                width,
                height,
                wgsl_source,
                uniform_values,
                frame,
                fps,
            );
        }
        EffectType::MergePaths { .. } => {
            // Merge Paths is a vector shape operator, applied during shape rasterization.
            // CPU effect pass is a no-op for this.
        }
        EffectType::OffsetPath { amount, .. } => {
            let offset = amount.evaluate(frame);
            if offset != 0.0 {
                crate::core::ae_effects_pack::apply_simple_choker(pixels, offset);
            }
        }
        EffectType::CustomPlugin { plugin_id } => {
            let time_sec = if fps > 0 {
                frame as f32 / fps as f32
            } else {
                0.0
            };
            crate::core::effect_plugin::registry::global_custom_plugins()
                .process_layer(plugin_id, pixels, width, height, frame, time_sec);
        }
        _ => unreachable!(),
    }
    true
}
