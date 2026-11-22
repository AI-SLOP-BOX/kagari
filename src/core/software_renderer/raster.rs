//! Layer content rasterization dispatcher for the software renderer.
//!
//! Owns [`RasterCtx`] (everything a raster arm needs) plus the image
//! and flat-fill arms. Shape and text arms live in `shape_raster` and
//! `text_raster` respectively.

use super::mask::{compute_combined_mask_coverage, CpuMaskEntry};
use crate::core::timeline::{Composition, Layer, LayerType};

pub(crate) struct RasterCtx<'a> {
    pub comp: &'a Composition,
    pub layer: &'a Layer,
    pub frame: u32,
    pub effective_frame: u32,
    pub source_frame: f32,
    pub masks: &'a [CpuMaskEntry],
    pub min_x: u32,
    pub min_y: u32,
    pub max_x: u32,
    pub max_y: u32,
    pub bw: u32,
    pub bh: u32,
    pub width: u32,
    pub height: u32,
    pub cx: f32,
    pub cy: f32,
    pub three_d_matrix: Option<[[f32; 4]; 4]>,
    pub scale: [f32; 2],
    pub cos_r: f32,
    pub sin_r: f32,
    pub flip_x: bool,
    pub flip_y: bool,
    pub bounds_x: f32,
    pub bounds_y: f32,
    pub base_color: [f32; 4],
    pub l_opacity: f32,
    pub buffer: &'a mut [u8],
    pub layer_buf: &'a mut [u8],
}

/// Dispatch the raster switch (shape/text/image/flat) for one layer.
pub(crate) fn rasterize_layer_content(ctx: RasterCtx<'_>) {
    use crate::core::timeline::LayerType;
    if let LayerType::Shape { .. } = &ctx.layer.layer_type {
        return super::shape_raster::rasterize_shape_layer(ctx);
    }
    if let LayerType::Text { .. } = &ctx.layer.layer_type {
        return super::text_raster::rasterize_text_layer(ctx);
    }
    if let LayerType::Model3D { .. } = &ctx.layer.layer_type {
        return super::model3d_raster::rasterize_model3d_layer(ctx);
    }
    if matches!(
        ctx.layer.layer_type,
        LayerType::Image { .. } | LayerType::Video { .. }
    ) {
        return rasterize_image_layer(ctx);
    }
    rasterize_flat_layer(ctx);
}

/// Rasterize image/video layers (texture-mapped, masked).
fn rasterize_image_layer(ctx: RasterCtx<'_>) {
    let RasterCtx {
        comp,
        layer,
        effective_frame: _,
        source_frame,
        masks,
        min_x,
        min_y,
        max_x,
        max_y,
        bw,
        bh: _,
        width,
        height,
        cx,
        cy,
        three_d_matrix,
        cos_r,
        sin_r,
        flip_x,
        flip_y,
        bounds_x,
        bounds_y,
        layer_buf,
        ..
    } = ctx;
    // Image layers load directly; Video layers resolve their frame PNG first.

    let (img_path, next_img_path, blend_t) = match &layer.layer_type {
        LayerType::Video {
            frames_dir,
            frame_count,
            speed,
            ..
        } => {
            let source_position = crate::core::timeline::video_sequence_frame(
                source_frame,
                *speed,
                frames_dir,
                comp.fps,
            );
            let first = (source_position.floor() as u32).min(frame_count.saturating_sub(1));
            let second = (first + 1).min(frame_count.saturating_sub(1));
            let t = if layer.frame_blending {
                (source_position - first as f32).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let first_path = super::preview_proxy_path(layer, first).unwrap_or_else(|| {
                crate::core::video_import::frame_path_in_dir(frames_dir, first)
                    .to_string_lossy()
                    .to_string()
            });
            let second_path = crate::core::video_import::frame_path_in_dir(frames_dir, second)
                .to_string_lossy()
                .to_string();
            (
                first_path,
                second_path,
                if super::preview_proxy_path(layer, first).is_some() {
                    0.0
                } else {
                    t
                },
            )
        }
        LayerType::Image { path } => (
            super::preview_proxy_path(layer, 0).unwrap_or_else(|| path.clone()),
            path.clone(),
            0.0,
        ),
        _ => unreachable!(),
    };
    let Some(img) = crate::core::image_cache::load_image_shared(&img_path) else {
        return;
    };
    let next_img = if blend_t > 0.0 && next_img_path != img_path {
        crate::core::image_cache::load_image_shared(&next_img_path)
            .filter(|next| next.width == img.width && next.height == img.height)
    } else {
        None
    };
    {
        for py in min_y..max_y {
            if super::render_cancelled() {
                return;
            }
            for px in min_x..max_x {
                // Vector mask check
                let mut mask_alpha = 1.0;
                if !masks.is_empty() {
                    mask_alpha =
                        compute_combined_mask_coverage(px as f32 + 0.5, py as f32 + 0.5, masks);
                }
                if mask_alpha <= 0.001 {
                    continue;
                }

                // Map pixel to image texture coordinates [0, 1]
                let dx = px as f32 + 0.5 - cx;
                let dy = py as f32 + 0.5 - cy;
                let lx = dx * cos_r + dy * sin_r;
                let ly = -dx * sin_r + dy * cos_r;
                let (u, v) = if let Some(matrix) = three_d_matrix {
                    let Some([u, v]) = super::projective::screen_to_uv(
                        matrix,
                        width as f32,
                        height as f32,
                        px as f32 + 0.5,
                        py as f32 + 0.5,
                    ) else {
                        continue;
                    };
                    (u, v)
                } else {
                    ((lx / bounds_x + 1.0) * 0.5, (ly / bounds_y + 1.0) * 0.5)
                };
                let (u, v) = if three_d_matrix.is_some() {
                    (u, v)
                } else {
                    (
                        if flip_x { 1.0 - u } else { u },
                        if flip_y { 1.0 - v } else { v },
                    )
                };

                if (0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v) {
                    let Some(sample_a) = super::sampling::sample_rgba8_linear(
                        &img.pixels,
                        img.width,
                        img.height,
                        u,
                        v,
                    ) else {
                        continue;
                    };
                    let sample_b = next_img
                        .as_ref()
                        .filter(|next| next.width == img.width && next.height == img.height)
                        .and_then(|next| {
                            super::sampling::sample_rgba8_linear(
                                &next.pixels,
                                next.width,
                                next.height,
                                u,
                                v,
                            )
                        })
                        .unwrap_or(sample_a);
                    let sample = crate::core::frame_blending::blend_rgba_samples(
                        sample_a, sample_b, blend_t,
                    );
                    let lidx = (((py - min_y) * bw + (px - min_x)) * 4) as usize;
                    if lidx + 3 < layer_buf.len() {
                        layer_buf[lidx] = sample[0].round().clamp(0.0, 255.0) as u8;
                        layer_buf[lidx + 1] = sample[1].round().clamp(0.0, 255.0) as u8;
                        layer_buf[lidx + 2] = sample[2].round().clamp(0.0, 255.0) as u8;
                        let src_a = (sample[3] / 255.0) * mask_alpha;
                        layer_buf[lidx + 3] = (src_a * 255.0).round().clamp(0.0, 255.0) as u8;
                    }
                }
            }
        }
    }
}

/// Flat rasterization for solid (and other non-shape) layers.
fn rasterize_flat_layer(ctx: RasterCtx<'_>) {
    let RasterCtx {
        masks,
        min_x,
        min_y,
        max_x,
        max_y,
        bw,
        bh: _,
        cx,
        cy,
        cos_r,
        sin_r,
        bounds_x,
        bounds_y,
        base_color,
        layer_buf,
        ..
    } = ctx;
    // Other non-shape layers: flat rasterization with mask support
    for py in min_y..max_y {
        if super::render_cancelled() {
            return;
        }
        for px in min_x..max_x {
            // Vector mask check with feathering support
            let mut mask_alpha = 1.0;
            if !masks.is_empty() {
                mask_alpha =
                    compute_combined_mask_coverage(px as f32 + 0.5, py as f32 + 0.5, masks);
            }

            if mask_alpha <= 0.001 {
                continue; // fully masked out pixel
            }

            // Inverse rotation & scale transform to local space
            let dx = px as f32 - cx;
            let dy = py as f32 - cy;
            let lx = dx * cos_r + dy * sin_r;
            let ly = -dx * sin_r + dy * cos_r;

            if lx >= -bounds_x && lx <= bounds_x && ly >= -bounds_y && ly <= bounds_y {
                let lidx = (((py - min_y) * bw + (px - min_x)) * 4) as usize;
                if lidx + 3 >= layer_buf.len() {
                    continue;
                }
                let src_a = base_color[3] * mask_alpha;
                layer_buf[lidx] = (base_color[0] * 255.0) as u8;
                layer_buf[lidx + 1] = (base_color[1] * 255.0) as u8;
                layer_buf[lidx + 2] = (base_color[2] * 255.0) as u8;
                layer_buf[lidx + 3] = (src_a * 255.0) as u8;
            }
        }
    }
}
