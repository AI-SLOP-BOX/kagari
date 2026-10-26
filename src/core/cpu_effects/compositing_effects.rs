use super::*;

pub(super) fn apply(
    pixels: &mut [u8],
    effect_type: &EffectType,
    context: LayerEffectRenderContext<'_>,
) -> bool {
    if !matches!(
        effect_type,
        EffectType::SetMatte { .. } | EffectType::Echo { .. } | EffectType::FindEdges { .. }
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
        ..
    } = context;
    match effect_type {
        EffectType::SetMatte {
            source_layer_idx,
            source_channel,
            invert_matte,
            composite_mode,
        } => {
            let params = crate::core::set_matte::SetMatteParams {
                source_layer_idx: *source_layer_idx,
                source_channel: *source_channel,
                invert_matte: *invert_matte,
                composite_mode: *composite_mode,
            };
            let src_pixels = comp.map(|c| {
                crate::core::software_renderer::render_single_layer_pixels(
                    c,
                    *source_layer_idx,
                    frame,
                    width,
                    height,
                )
            });
            match src_pixels {
                Some(src) => {
                    crate::core::set_matte::apply_set_matte(
                        pixels, width, height, &src, width, height, &params,
                    );
                }
                None => {
                    let fallback = pixels.to_vec();
                    crate::core::set_matte::apply_set_matte(
                        pixels, width, height, &fallback, width, height, &params,
                    );
                }
            }
        }
        EffectType::Echo {
            echo_time_seconds,
            num_echoes,
            starting_intensity,
            decay,
            operator,
        } => {
            let start_w = starting_intensity.evaluate(frame);
            let dec = decay.evaluate(frame);
            let echo_dt = echo_time_seconds.evaluate(frame);
            let n = (*num_echoes as i32).max(1);
            let fps_val = fps as f32;
            let current_layer = layer_idx.unwrap_or(0);
            for i in 1..=n {
                let offset_secs = echo_dt * i as f32;
                let offset_frames = (offset_secs * fps_val).round() as i32;
                let echo_frame = (frame as i32 - offset_frames).max(0) as u32;
                let weight = start_w * dec.powi(i);
                if weight < 0.001 {
                    continue;
                }
                if let Some(c) = comp {
                    let echo_pixels = crate::core::software_renderer::render_single_layer_pixels(
                        c,
                        current_layer,
                        echo_frame,
                        width,
                        height,
                    );
                    crate::core::echo_effect::blend_echo_frame(
                        pixels,
                        &echo_pixels,
                        width,
                        height,
                        weight,
                        *operator,
                    );
                } else {
                    let echo_copy = pixels.to_vec();
                    crate::core::echo_effect::blend_echo_frame(
                        pixels, &echo_copy, width, height, weight, *operator,
                    );
                }
            }
        }
        EffectType::FindEdges { invert } => {
            let params = crate::core::find_edges::FindEdgesParams { invert: *invert };
            let edges = crate::core::find_edges::apply_find_edges(pixels, width, height, &params);
            pixels.copy_from_slice(&edges);
        }
        _ => unreachable!(),
    }
    true
}
