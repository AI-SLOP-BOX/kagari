use super::mask::compute_combined_mask_coverage;
use super::{
    composite_layer_buffer, render_matte_buffer, render_precomp_layers, CompositeCtx, CpuMaskEntry,
};
use crate::core::timeline::{Composition, Layer, LayerType};

pub(crate) struct PreCompLayerCtx<'a> {
    pub comp: &'a Composition,
    pub layer: &'a Layer,
    pub frame: u32,
    pub source_frame: u32,
    pub width: u32,
    pub height: u32,
    pub position: [f32; 2],
    pub scale: [f32; 2],
    pub rotation: f32,
    pub opacity: f32,
    pub masks: &'a [CpuMaskEntry],
    pub blend_linear: bool,
    pub sorted_idx: usize,
}

pub(crate) fn render_precomp_layer(
    buffer: &mut [u8],
    ctx: PreCompLayerCtx<'_>,
) -> Result<bool, ()> {
    let PreCompLayerCtx {
        comp,
        layer,
        frame,
        source_frame,
        width,
        height,
        position: pos,
        scale,
        rotation,
        opacity: l_opacity,
        masks,
        blend_linear,
        sorted_idx,
    } = ctx;
    // through the layer's transform (position / scale / rotation / opacity).
    if let LayerType::PreComp { comp_id } = &layer.layer_type {
        if let Some(sub_comp) = comp.find_sub_comp(comp_id) {
            let mut resolved_sub_comp = sub_comp.clone();
            crate::core::essential_properties::apply_essential_overrides(
                &mut resolved_sub_comp,
                &layer.essential_properties,
            );
            let source_pixels =
                render_precomp_layers(comp, &resolved_sub_comp, source_frame, width, height);
            let Some(mut sub_pixels) = super::try_clone_buffer(&source_pixels) else {
                log::error!(
                    "[Renderer] Could not allocate {} bytes for pre-comp layer {}",
                    source_pixels.len(),
                    layer.id
                );
                return Err(());
            };
            if !sub_pixels.is_empty() {
                if !layer.paint_strokes.is_empty() {
                    let paint_transform =
                        crate::core::software_renderer::precomp::paint::PaintTransform {
                            scale: [100.0, 100.0],
                            cos_r: 1.0,
                            sin_r: 0.0,
                            cx: width as f32 * 0.5,
                            cy: height as f32 * 0.5,
                            min_x: 0,
                            min_y: 0,
                        };
                    crate::core::software_renderer::precomp::paint::apply_paint_strokes(
                        layer,
                        source_frame,
                        &paint_transform,
                        width,
                        height,
                        &mut sub_pixels,
                    );
                }
                if !layer.effects.is_empty() && layer.effects_enabled {
                    crate::core::cpu_effects::apply_layer_effects(
                        Some(comp),
                        Some(sorted_idx),
                        &mut sub_pixels,
                        width,
                        height,
                        &layer.effects,
                        source_frame,
                        comp.fps,
                    );
                }
                // Treat the rendered sub-comp as a full-frame texture and
                // sample it through the inverse layer transform.
                let pc_rad = rotation.to_radians();
                let pc_cos = pc_rad.cos();
                let pc_sin = pc_rad.sin();
                let pc_cx = pos[0];
                let pc_cy = pos[1];

                let pc_base_w = comp.width as f32;
                let pc_base_h = comp.height as f32;
                let pc_w = (scale[0].abs() / 100.0) * pc_base_w;
                let pc_h = (scale[1].abs() / 100.0) * pc_base_h;
                let pc_bx = pc_w * 0.5;
                let pc_by = pc_h * 0.5;

                let lo_x = (pc_cx - pc_bx - 2.0).floor().max(0.0) as u32;
                let hi_x = (pc_cx + pc_bx + 2.0).ceil().min(width as f32 - 1.0) as u32;
                let lo_y = (pc_cy - pc_by - 2.0).floor().max(0.0) as u32;
                let hi_y = (pc_cy + pc_by + 2.0).ceil().min(height as f32 - 1.0) as u32;
                if pc_bx > 0.0 && pc_by > 0.0 && lo_x <= hi_x && lo_y <= hi_y {
                    let precomp_width = hi_x - lo_x + 1;
                    let precomp_height = hi_y - lo_y + 1;
                    let Some(precomp_size) = (precomp_width as usize)
                        .checked_mul(precomp_height as usize)
                        .and_then(|pixels| pixels.checked_mul(4))
                    else {
                        log::error!(
                            "[Renderer] Pre-comp raster dimensions overflow for {}",
                            layer.id
                        );
                        return Err(());
                    };
                    let Some(mut precomp_buf) = super::try_zeroed_buffer(precomp_size) else {
                        log::error!(
                            "[Renderer] Could not allocate {precomp_size} bytes for pre-comp layer {}",
                            layer.id
                        );
                        return Err(());
                    };

                    for py in lo_y..=hi_y {
                        if super::render_cancelled() {
                            return Ok(true);
                        }
                        for px in lo_x..=hi_x {
                            let mut mask_alpha = 1.0;
                            if !masks.is_empty() {
                                mask_alpha = compute_combined_mask_coverage(
                                    px as f32 + 0.5,
                                    py as f32 + 0.5,
                                    masks,
                                );
                            }
                            if mask_alpha <= 0.001 {
                                continue;
                            }

                            // Inverse rotation into local space, then UV over sub-comp frame.
                            let dx = px as f32 + 0.5 - pc_cx;
                            let dy = py as f32 + 0.5 - pc_cy;
                            let lx = dx * pc_cos + dy * pc_sin;
                            let ly = -dx * pc_sin + dy * pc_cos;
                            let u = (lx / pc_bx + 1.0) * 0.5;
                            let v = (ly / pc_by + 1.0) * 0.5;
                            if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) {
                                continue;
                            }

                            let sw = width.max(1);
                            let sh = height.max(1);
                            let sx = ((u * (sw - 1) as f32).round() as u32).min(sw - 1);
                            let sy = ((v * (sh - 1) as f32).round() as u32).min(sh - 1);
                            let src_idx = ((sy * sw + sx) * 4) as usize;
                            if src_idx + 3 >= sub_pixels.len() {
                                continue;
                            }
                            let dst_idx =
                                (((py - lo_y) * precomp_width + (px - lo_x)) * 4) as usize;
                            precomp_buf[dst_idx] = sub_pixels[src_idx];
                            precomp_buf[dst_idx + 1] = sub_pixels[src_idx + 1];
                            precomp_buf[dst_idx + 2] = sub_pixels[src_idx + 2];
                            precomp_buf[dst_idx + 3] = (sub_pixels[src_idx + 3] as f32 * mask_alpha)
                                .round()
                                .clamp(0.0, 255.0)
                                as u8;
                        }
                    }

                    let matte_pixels = match render_matte_buffer(comp, layer, frame, width, height)
                    {
                        Ok(matte) => matte,
                        Err(()) => return Err(()),
                    };
                    composite_layer_buffer(
                        &mut buffer[..],
                        &precomp_buf,
                        &CompositeCtx {
                            layer,
                            matte: matte_pixels.as_deref(),
                            min_x: lo_x,
                            min_y: lo_y,
                            bw: precomp_width,
                            bh: precomp_height,
                            width,
                            height,
                            blend_linear,
                            l_opacity,
                        },
                    );
                }
            }
        }
        return Ok(true);
    }
    Ok(false)
}
