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
    pub cos_r: f32,
    pub sin_r: f32,
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
        layer,
        effective_frame,
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
        layer_buf,
        ..
    } = ctx;
        // Image layers load directly; Video layers resolve their frame PNG first.
        use crate::core::image_cache::with_image_cache;

        let img_path = match &layer.layer_type {
            LayerType::Video {
                frames_dir,
                frame_count,
                speed,
                ..
            } => {
                let seq_frame = ((effective_frame as f32 * speed.max(0.0)) as u32)
                    .min(frame_count.saturating_sub(1));
                std::path::Path::new(frames_dir)
                    .join(format!("frame_{:05}.png", seq_frame))
                    .to_string_lossy()
                    .to_string()
            }
            LayerType::Image { path } => path.clone(),
            _ => unreachable!(),
        };
        with_image_cache(|cache| {
            if let Some(img) = cache.load_image(&img_path) {
                let img_w = img.width as f32;
                let img_h = img.height as f32;

                for py in min_y..max_y {
                    for px in min_x..max_x {
                        // Vector mask check
                        let mut mask_alpha = 1.0;
                        if !masks.is_empty() {
                            mask_alpha =
                                compute_combined_mask_coverage(px as f32, py as f32, masks);
                        }
                        if mask_alpha <= 0.001 {
                            continue;
                        }

                        // Map pixel to image texture coordinates [0, 1]
                        let dx = px as f32 - cx;
                        let dy = py as f32 - cy;
                        let lx = dx * cos_r + dy * sin_r;
                        let ly = -dx * sin_r + dy * cos_r;
                        let u = (lx / bounds_x + 1.0) * 0.5;
                        let v = (ly / bounds_y + 1.0) * 0.5;

                        if (0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v) {
                            let tex_x =
                                ((u * (img_w - 1.0)).round() as u32).min(img_w as u32 - 1);
                            let tex_y =
                                ((v * (img_h - 1.0)).round() as u32).min(img_h as u32 - 1);
                            let tidx = ((tex_y * img.width + tex_x) * 4) as usize;
                            if tidx + 3 < img.pixels.len() {
                                let lidx = (((py - min_y) * bw + (px - min_x)) * 4) as usize;
                                if lidx + 3 < layer_buf.len() {
                                    let src_a = (img.pixels[tidx + 3] as f32 / 255.0) * mask_alpha;
                                    layer_buf[lidx] = img.pixels[tidx];
                                    layer_buf[lidx + 1] = img.pixels[tidx + 1];
                                    layer_buf[lidx + 2] = img.pixels[tidx + 2];
                                    layer_buf[lidx + 3] = (src_a * 255.0) as u8;
                                }
                            }
                        }
                    }
                }
            }
        });
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
            for px in min_x..max_x {
                // Vector mask check with feathering support
                let mut mask_alpha = 1.0;
                if !masks.is_empty() {
                    mask_alpha = compute_combined_mask_coverage(px as f32, py as f32, masks);
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
