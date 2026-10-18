//! Phase-3 compositing for the software renderer.
//!
//! Blends one effect-processed layer buffer over the frame with full
//! blend-mode support, track mattes, composite-time opacity, and
//! optional linear-light blending.

mod blend_modes;
use blend_modes::{evaluate_blend_mode, overlay, soft_light};
use rayon::prelude::*;
use super::mask::CpuMaskEntry;
use crate::core::timeline::{BlendMode, Composition, Layer, TrackMatteMode};

pub(crate) struct AdjustmentLayerCtx<'a> {
    pub comp: &'a Composition,
    pub layer: &'a Layer,
    pub sorted_idx: usize,
    pub source_frame: u32,
    pub width: u32,
    pub height: u32,
    pub opacity: f32,
    pub masks: &'a [CpuMaskEntry],
}

#[allow(clippy::chunks_exact_to_as_chunks)]
pub(crate) fn apply_adjustment_layer(buffer: &mut [u8], ctx: AdjustmentLayerCtx<'_>) {
    let AdjustmentLayerCtx {
        comp,
        layer,
        sorted_idx,
        source_frame,
        width,
        height,
        opacity,
        masks,
    } = ctx;
    if !layer.effects_enabled || layer.effects.is_empty() || opacity <= 0.003 {
        return;
    }
    let mut adjusted = buffer.to_vec();
    crate::core::cpu_effects::apply_layer_effects(
        Some(comp),
        Some(sorted_idx),
        &mut adjusted,
        width,
        height,
        &layer.effects,
        source_frame,
        comp.fps,
    );
    let use_mask = !masks.is_empty();
    let adj_blend = layer.blend_mode;

    if !use_mask && (opacity - 1.0).abs() < 0.003 && adj_blend == BlendMode::Normal {
        for (dst_chunk, src_chunk) in buffer.chunks_exact_mut(4).zip(adjusted.chunks_exact(4)) {
            dst_chunk[0] = src_chunk[0];
            dst_chunk[1] = src_chunk[1];
            dst_chunk[2] = src_chunk[2];
            // Preserve dst_chunk[3] (the composition alpha) so effects on adjustment layers
            // do not zero out comp transparency or reveal background checkerboard!
        }
        return;
    }

    if !use_mask {
        use rayon::prelude::*;
        buffer
            .par_chunks_exact_mut(4)
            .zip(adjusted.par_chunks_exact(4))
            .for_each(|(dst_chunk, src_chunk)| {
                let src_r = src_chunk[0] as f32 / 255.0;
                let src_g = src_chunk[1] as f32 / 255.0;
                let src_b = src_chunk[2] as f32 / 255.0;
                let dst_r = dst_chunk[0] as f32 / 255.0;
                let dst_g = dst_chunk[1] as f32 / 255.0;
                let dst_b = dst_chunk[2] as f32 / 255.0;
                let (blended_r, blended_g, blended_b) = match adj_blend {
                    BlendMode::Multiply => (src_r * dst_r, src_g * dst_g, src_b * dst_b),
                    BlendMode::Screen => (
                        1.0 - (1.0 - src_r) * (1.0 - dst_r),
                        1.0 - (1.0 - src_g) * (1.0 - dst_g),
                        1.0 - (1.0 - src_b) * (1.0 - dst_b),
                    ),
                    BlendMode::Add => (
                        (src_r + dst_r).min(1.0),
                        (src_g + dst_g).min(1.0),
                        (src_b + dst_b).min(1.0),
                    ),
                    BlendMode::Overlay => (
                        overlay(src_r, dst_r),
                        overlay(src_g, dst_g),
                        overlay(src_b, dst_b),
                    ),
                    BlendMode::SoftLight => (
                        soft_light(src_r, dst_r),
                        soft_light(src_g, dst_g),
                        soft_light(src_b, dst_b),
                    ),
                    _ => (src_r, src_g, src_b),
                };
                dst_chunk[0] = ((blended_r * opacity + dst_r * (1.0 - opacity)) * 255.0)
                    .round()
                    .clamp(0.0, 255.0) as u8;
                dst_chunk[1] = ((blended_g * opacity + dst_g * (1.0 - opacity)) * 255.0)
                    .round()
                    .clamp(0.0, 255.0) as u8;
                dst_chunk[2] = ((blended_b * opacity + dst_b * (1.0 - opacity)) * 255.0)
                    .round()
                    .clamp(0.0, 255.0) as u8;
            });
        return;
    }

    use rayon::prelude::*;
    let row_stride = (width * 4) as usize;
    buffer
        .par_chunks_exact_mut(row_stride)
        .zip(adjusted.par_chunks_exact(row_stride))
        .enumerate()
        .for_each(|(py_idx, (dst_row, src_row))| {
            let py = py_idx as u32;
            for px in 0..width {
                let mask_alpha = super::mask::compute_combined_mask_coverage(
                    px as f32 + 0.5,
                    py as f32 + 0.5,
                    masks,
                );
                if mask_alpha <= 0.001 {
                    continue;
                }
                let effect_opacity = opacity * mask_alpha;
                let offset = (px * 4) as usize;
                let src_r = src_row[offset] as f32 / 255.0;
                let src_g = src_row[offset + 1] as f32 / 255.0;
                let src_b = src_row[offset + 2] as f32 / 255.0;
                let dst_r = dst_row[offset] as f32 / 255.0;
                let dst_g = dst_row[offset + 1] as f32 / 255.0;
                let dst_b = dst_row[offset + 2] as f32 / 255.0;
                let (blended_r, blended_g, blended_b) = match adj_blend {
                    BlendMode::Multiply => (src_r * dst_r, src_g * dst_g, src_b * dst_b),
                    BlendMode::Screen => (
                        1.0 - (1.0 - src_r) * (1.0 - dst_r),
                        1.0 - (1.0 - src_g) * (1.0 - dst_g),
                        1.0 - (1.0 - src_b) * (1.0 - dst_b),
                    ),
                    BlendMode::Add => (
                        (src_r + dst_r).min(1.0),
                        (src_g + dst_g).min(1.0),
                        (src_b + dst_b).min(1.0),
                    ),
                    BlendMode::Overlay => (
                        overlay(src_r, dst_r),
                        overlay(src_g, dst_g),
                        overlay(src_b, dst_b),
                    ),
                    BlendMode::SoftLight => (
                        soft_light(src_r, dst_r),
                        soft_light(src_g, dst_g),
                        soft_light(src_b, dst_b),
                    ),
                    _ => (src_r, src_g, src_b),
                };
                dst_row[offset] = ((blended_r * effect_opacity + dst_r * (1.0 - effect_opacity)) * 255.0)
                    .round()
                    .clamp(0.0, 255.0) as u8;
                dst_row[offset + 1] = ((blended_g * effect_opacity + dst_g * (1.0 - effect_opacity)) * 255.0)
                    .round()
                    .clamp(0.0, 255.0) as u8;
                dst_row[offset + 2] = ((blended_b * effect_opacity + dst_b * (1.0 - effect_opacity)) * 255.0)
                    .round()
                    .clamp(0.0, 255.0) as u8;
            }
        });
}


/// Everything [`composite_layer_buffer`] needs that is not the two
/// pixel buffers.
pub(crate) struct CompositeCtx<'a> {
    pub layer: &'a Layer,
    pub matte: Option<&'a [u8]>,
    pub min_x: u32,
    pub min_y: u32,
    pub bw: u32,
    pub bh: u32,
    pub width: u32,
    pub height: u32,
    pub blend_linear: bool,
    pub l_opacity: f32,
}

/// Composite `layer_buf` over `buffer` (both RGBA8) with the layer's
/// blend mode, track matte, and composite-time opacity.
pub(crate) fn composite_layer_buffer(buffer: &mut [u8], layer_buf: &[u8], ctx: &CompositeCtx<'_>) {
    let CompositeCtx {
        layer,
        matte,
        min_x,
        min_y,
        bw,
        bh,
        width,
        height,
        blend_linear,
        l_opacity,
    } = ctx;
    let (min_x, min_y, bw, bh, width, height) = (*min_x, *min_y, *bw, *bh, *width, *height);
    let (blend_linear, l_opacity) = (*blend_linear, *l_opacity);

    let row_stride = (width * 4) as usize;
    let sub_start = (min_y * width * 4) as usize;
    let sub_len = (bh * width * 4) as usize;
    if sub_start + sub_len > buffer.len() {
        return;
    }
    let target_slice = &mut buffer[sub_start..sub_start + sub_len];

    // Fast-path for BlendMode::Add without track matte and in non-linear mode
    if matte.is_none() && !blend_linear && !layer.preserve_transparency && layer.blend_mode == BlendMode::Add {
        target_slice
            .par_chunks_exact_mut(row_stride)
            .enumerate()
            .for_each(|(ly, dst_row)| {
                let l_row_idx = ly * (bw * 4) as usize;
                let min_x_offset = (min_x * 4) as usize;
                for lx in 0..bw as usize {
                    let lidx = l_row_idx + lx * 4;
                    let src_a_byte = layer_buf[lidx + 3];
                    if src_a_byte == 0 {
                        continue;
                    }
                    let src_a = (src_a_byte as f32 / 255.0) * l_opacity;
                    if src_a <= 0.001 {
                        continue;
                    }
                    let idx = min_x_offset + lx * 4;
                    if idx + 3 >= dst_row.len() {
                        continue;
                    }
                    let sa_u16 = (src_a * 255.0).round() as u16;
                    let sr_u16 = (layer_buf[lidx] as u16 * sa_u16) / 255;
                    let sg_u16 = (layer_buf[lidx + 1] as u16 * sa_u16) / 255;
                    let sb_u16 = (layer_buf[lidx + 2] as u16 * sa_u16) / 255;

                    let dr = dst_row[idx] as u16 + sr_u16;
                    let dg = dst_row[idx + 1] as u16 + sg_u16;
                    let db = dst_row[idx + 2] as u16 + sb_u16;
                    let da = dst_row[idx + 3] as u16 + sa_u16;

                    dst_row[idx] = dr.min(255) as u8;
                    dst_row[idx + 1] = dg.min(255) as u8;
                    dst_row[idx + 2] = db.min(255) as u8;
                    dst_row[idx + 3] = da.min(255) as u8;
                }
            });
        return;
    }

    // Fast-path for BlendMode::Normal without track matte and in non-linear mode
    if matte.is_none() && !blend_linear && !layer.preserve_transparency && layer.blend_mode == BlendMode::Normal {
        target_slice
            .par_chunks_exact_mut(row_stride)
            .enumerate()
            .for_each(|(ly, dst_row)| {
                let l_row_idx = ly * (bw * 4) as usize;
                let min_x_offset = (min_x * 4) as usize;
                for lx in 0..bw as usize {
                    let lidx = l_row_idx + lx * 4;
                    let src_a_byte = layer_buf[lidx + 3];
                    if src_a_byte == 0 {
                        continue;
                    }
                    let src_a = (src_a_byte as f32 / 255.0) * l_opacity;
                    if src_a <= 0.001 {
                        continue;
                    }
                    let idx = min_x_offset + lx * 4;
                    if idx + 3 >= dst_row.len() {
                        continue;
                    }
                    let dst_a = dst_row[idx + 3] as f32 / 255.0;
                    let out_a = src_a + dst_a * (1.0 - src_a);
                    if out_a > 0.0 {
                        let sr = layer_buf[lidx] as f32;
                        let sg = layer_buf[lidx + 1] as f32;
                        let sb = layer_buf[lidx + 2] as f32;
                        let dr = dst_row[idx] as f32;
                        let dg = dst_row[idx + 1] as f32;
                        let db = dst_row[idx + 2] as f32;
                        let inv_src = 1.0 - src_a;
                        let dst_factor = dst_a * inv_src;

                        dst_row[idx] = ((sr * src_a + dr * dst_factor) / out_a).clamp(0.0, 255.0) as u8;
                        dst_row[idx + 1] = ((sg * src_a + dg * dst_factor) / out_a).clamp(0.0, 255.0) as u8;
                        dst_row[idx + 2] = ((sb * src_a + db * dst_factor) / out_a).clamp(0.0, 255.0) as u8;
                        dst_row[idx + 3] = (out_a * 255.0).clamp(0.0, 255.0) as u8;
                    }
                }
            });
        return;
    }

    target_slice
        .par_chunks_exact_mut(row_stride)
        .enumerate()
        .for_each(|(ly, dst_row)| {
            let py = min_y + ly as u32;
            let l_row_idx = ly * (bw * 4) as usize;
            let min_x_offset = (min_x * 4) as usize;
            for lx in 0..bw as usize {
                let px = min_x + lx as u32;
                let lidx = l_row_idx + lx * 4;
                // Composite-time opacity: applied once here so effect stacks cannot destroy it,
                // uniformly across all blend modes via the src_a lerp below.
                let mut src_a = layer_buf[lidx + 3] as f32 / 255.0 * l_opacity;
                if src_a <= 0.001 {
                    continue;
                }
                let idx = min_x_offset + lx * 4;
                if idx + 3 >= dst_row.len() {
                    continue;
                }

                // Apply track matte masking
                if let Some(matte_buf) = matte {
                    let mx = px.min(width - 1);
                    let my = py.min(height - 1);
                    let midx = ((my * width + mx) * 4) as usize;
                    if midx + 3 < matte_buf.len() {
                        let matte_a = matte_buf[midx + 3] as f32 / 255.0;
                        let matte_luma = (matte_buf[midx] as f32 * 0.299
                            + matte_buf[midx + 1] as f32 * 0.587
                            + matte_buf[midx + 2] as f32 * 0.114)
                            / 255.0;
                        src_a = match layer.track_matte {
                            TrackMatteMode::AlphaMatte => src_a * matte_a,
                            TrackMatteMode::AlphaMatteInverted => src_a * (1.0 - matte_a),
                            TrackMatteMode::LumaMatte => src_a * matte_luma,
                            TrackMatteMode::LumaMatteInverted => src_a * (1.0 - matte_luma),
                            _ => src_a,
                        };
                        if src_a <= 0.001 {
                            continue;
                        }
                    }
                }

                let mut src_r = layer_buf[lidx] as f32 / 255.0;
                let mut src_g = layer_buf[lidx + 1] as f32 / 255.0;
                let mut src_b = layer_buf[lidx + 2] as f32 / 255.0;

                let mut dst_r = dst_row[idx] as f32 / 255.0;
                let mut dst_g = dst_row[idx + 1] as f32 / 255.0;
                let mut dst_b = dst_row[idx + 2] as f32 / 255.0;
                let dst_a = dst_row[idx + 3] as f32 / 255.0;

                // Linear-light mode: decode both sides with the exact IEC
                // sRGB piecewise EOTF so Add/Screen/Glow blends behave
                // physically and match GPU sRGB hardware encoders.
                if blend_linear {
                    use crate::core::color::srgb_to_linear_piecewise as to_lin;
                    src_r = to_lin(src_r);
                    src_g = to_lin(src_g);
                    src_b = to_lin(src_b);
                    dst_r = to_lin(dst_r);
                    dst_g = to_lin(dst_g);
                    dst_b = to_lin(dst_b);
                }

                // Compute BlendMode calculations
                let (blended_r, blended_g, blended_b) = evaluate_blend_mode(
                    layer.blend_mode,
                    src_r,
                    src_g,
                    src_b,
                    src_a,
                    dst_r,
                    dst_g,
                    dst_b,
                    dst_a,
                );

                // Preserve Underlying Transparency (AE 'T' switch)
                if layer.preserve_transparency {
                    src_a *= dst_a;
                    if src_a <= 0.001 {
                        continue;
                    }
                }

                // Alpha blending formula: Standard Source-Over or Transparency Preservation
                let out_a = if layer.preserve_transparency {
                    dst_a
                } else {
                    src_a + dst_a * (1.0 - src_a)
                };
                let out_r = if out_a > 0.0 {
                    (blended_r * src_a + dst_r * dst_a * (1.0 - src_a)) / out_a
                } else {
                    0.0
                };
                let out_g = if out_a > 0.0 {
                    (blended_g * src_a + dst_g * dst_a * (1.0 - src_a)) / out_a
                } else {
                    0.0
                };
                let out_b = if out_a > 0.0 {
                    (blended_b * src_a + dst_b * dst_a * (1.0 - src_a)) / out_a
                } else {
                    0.0
                };

                // Encode back to display space when in linear-light mode.
                let (or_, og_, ob_) = if blend_linear {
                    use crate::core::color::linear_to_srgb_piecewise as to_srgb;
                    (
                        to_srgb(out_r.max(0.0)),
                        to_srgb(out_g.max(0.0)),
                        to_srgb(out_b.max(0.0)),
                    )
                } else {
                    (out_r, out_g, out_b)
                };
                dst_row[idx] = (or_ * 255.0) as u8;
                dst_row[idx + 1] = (og_ * 255.0) as u8;
                dst_row[idx + 2] = (ob_ * 255.0) as u8;
                dst_row[idx + 3] = (out_a * 255.0) as u8;
            }
        });
}
