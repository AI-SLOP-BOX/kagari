use super::*;

pub(super) fn apply(
    pixels: &mut [u8],
    effect_type: &EffectType,
    context: LayerEffectRenderContext<'_>,
) -> bool {
    if !matches!(
        effect_type,
        EffectType::RadialBlurZoom { .. }
            | EffectType::MedianFilter { .. }
            | EffectType::SobelEdges { .. }
            | EffectType::Mosaic { .. }
            | EffectType::OpticalFlares { .. }
            | EffectType::MotionTile { .. }
            | EffectType::PageTurn { .. }
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
        EffectType::RadialBlurZoom { amount } => {
            use crate::core::ae_effects_pack_v11::apply_radial_blur_zoom;
            let center = [width as f32 * 0.5, height as f32 * 0.5];
            apply_radial_blur_zoom(pixels, width, height, center, amount.evaluate(frame));
        }
        EffectType::MedianFilter { radius } => {
            use crate::core::ae_effects_pack_v19::apply_median_filter;
            apply_median_filter(
                pixels,
                width,
                height,
                radius.evaluate(frame).round().clamp(1.0, 16.0) as u32,
            );
        }
        EffectType::SobelEdges { invert } => {
            use crate::core::ae_effects_pack_v19::apply_sobel_edges;
            apply_sobel_edges(pixels, width, height, *invert);
        }
        EffectType::Mosaic { block_w, block_h } => {
            let bw = block_w.evaluate(frame).round().clamp(1.0, 256.0);
            let bh = block_h.evaluate(frame).round().clamp(1.0, 256.0);
            if !crate::core::compute_pipeline::try_gpu_mosaic(pixels, width, height, bw, bh) {
                use crate::core::ae_effects_pack_v19::apply_mosaic;
                apply_mosaic(pixels, width, height, bw as u32, bh as u32);
            }
        }
        EffectType::OpticalFlares {
            position,
            brightness,
            scale,
        } => {
            let pos = position.evaluate(frame);
            let bright = brightness.evaluate(frame);
            let scl = scale.evaluate(frame);
            let cfg = crate::core::optical_flare::OpticalFlareConfig {
                position: pos,
                overall_scale: scl,
                overall_brightness: bright,
                ..Default::default()
            };
            crate::core::optical_flare::render_optical_flare(pixels, width, height, &cfg);
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
            let params = crate::core::motion_tile::MotionTileParams {
                tile_center: tile_center.evaluate(frame),
                tile_width: tile_width.evaluate(frame),
                tile_height: tile_height.evaluate(frame),
                output_width: output_width.evaluate(frame),
                output_height: output_height.evaluate(frame),
                mirror_edges: *mirror_edges,
                phase: phase.evaluate(frame),
            };
            let tiled = crate::core::motion_tile::apply_motion_tile(pixels, width, height, &params);
            pixels.copy_from_slice(&tiled);
        }
        EffectType::PageTurn {
            fold_position,
            fold_radius,
            fold_direction_deg,
            light_direction_deg,
            back_opacity,
            back_color,
        } => {
            let params = crate::core::page_turn::PageTurnParams {
                fold_position: fold_position.evaluate(frame),
                fold_radius: fold_radius.evaluate(frame),
                fold_direction_deg: fold_direction_deg.evaluate(frame),
                light_direction_deg: light_direction_deg.evaluate(frame),
                back_opacity: back_opacity.evaluate(frame),
                back_color: back_color.evaluate(frame),
            };
            let turned = crate::core::page_turn::apply_page_turn(pixels, width, height, &params);
            pixels.copy_from_slice(&turned);
        }
        _ => unreachable!(),
    }
    true
}
