use super::raster::RasterizationRegion;
use crate::core::timeline::{Composition, Layer, LayerType};

pub struct LayerTransformBounds {
    pub bw: u32,
    pub bh: u32,
    pub min_x: u32,
    pub min_y: u32,
    pub max_x: u32,
    pub max_y: u32,
    pub cx: f32,
    pub cy: f32,
    pub bounds_x: f32,
    pub bounds_y: f32,
    pub cos_r: f32,
    pub sin_r: f32,
}

pub struct RasterizeParams<'a> {
    pub comp: &'a Composition,
    pub layer: &'a Layer,
    pub frame: u32,
    pub source_frame: u32,
    pub base_color: [f32; 4],
    pub raster_region: RasterizationRegion,
    pub comp_size: (u32, u32),
}

pub fn rasterize_layer_content(
    p: &RasterizeParams,
    tb: &LayerTransformBounds,
    layer_buf: &mut [u8],
) {
    let (width, height) = p.comp_size;
    match &p.layer.layer_type {
        LayerType::Shape {
            shape_type,
            color,
            stroke_color,
            stroke_width,
            fill_type,
            ..
        } => {
            crate::core::sdf::rasterize_shape_sdf(
                layer_buf,
                tb.bw,
                tb.bh,
                tb.min_x,
                tb.min_y,
                tb.cx,
                tb.cy,
                tb.bounds_x,
                tb.bounds_y,
                *color,
                fill_type,
                *stroke_color,
                *stroke_width,
                1.0,
                shape_type,
                p.source_frame,
                p.layer.trim_paths.as_ref(),
            );
        }
        LayerType::Image { path } => {
            let proxy_path = crate::core::software_renderer::preview_proxy_path(p.layer, 0);
            let image_path = proxy_path.as_deref().unwrap_or(path);
            super::raster::rasterize_texture_layer(layer_buf, image_path, None, 0.0, p.raster_region);
        }
        LayerType::Video {
            frames_dir,
            frame_count,
            speed,
            ..
        } => {
            let source_position = (match &p.layer.posterize_time {
                Some(settings) if settings.enabled => p.source_frame as f32,
                _ => p.layer.remap_frame_f32(p.frame),
            } * speed.max(0.0))
            .max(0.0);
            let first = (source_position.floor() as u32).min(frame_count.saturating_sub(1));
            let second = (first + 1).min(frame_count.saturating_sub(1));
            let blend_t = if p.layer.frame_blending {
                (source_position - first as f32).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let proxy_path = crate::core::software_renderer::preview_proxy_path(p.layer, first);
            let first_path = proxy_path
                .as_deref()
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| {
                    crate::core::video_import::frame_path_in_dir(frames_dir, first)
                });
            let second_path = crate::core::video_import::frame_path_in_dir(frames_dir, second);
            let first_path_text = first_path.to_string_lossy();
            let second_path_text = second_path.to_string_lossy();
            super::raster::rasterize_texture_layer(
                layer_buf,
                &first_path_text,
                if proxy_path.is_some() {
                    None
                } else {
                    Some(&second_path_text)
                },
                if proxy_path.is_some() { 0.0 } else { blend_t },
                p.raster_region,
            );
        }
        LayerType::PreComp { comp_id } => {
            if let Some(nested_comp) = p.comp.find_sub_comp(comp_id) {
                let mut resolved_nested_comp = nested_comp.clone();
                crate::core::essential_properties::apply_essential_overrides(
                    &mut resolved_nested_comp,
                    &p.layer.essential_properties,
                );
                let nested_pixels = super::super::render_precomp_layers(
                    p.comp,
                    &resolved_nested_comp,
                    p.source_frame,
                    width,
                    height,
                );
                super::raster::rasterize_pixel_layer(
                    layer_buf,
                    &nested_pixels,
                    width,
                    height,
                    p.raster_region,
                );
            }
        }
        LayerType::Model3D { .. } => {
            let mut isolated = p.comp.clone();
            isolated.layers = vec![p.layer.clone()];
            isolated.background_color[3] = 0.0;
            let model_pixels = crate::core::software_renderer::render_frame_to_pixels(
                &isolated,
                p.source_frame,
                width,
                height,
                0.0,
                0,
            );
            super::raster::rasterize_pixel_layer(layer_buf, &model_pixels, width, height, p.raster_region);
        }
        LayerType::Text {
            text,
            font_size,
            color,
            font_family,
            tracking,
            ..
        } => {
            use crate::core::font_rasterizer::with_font_rasterizer;
            let text_color = *color;
            let text_str = text.clone();
            let fs = *font_size as f32;
            let tk = *tracking;
            let family = font_family.clone();
            with_font_rasterizer(|rasterizer| {
                let family_name = rasterizer.resolve_family(&family);
                if let Some((tw, th, text_pixels)) =
                    rasterizer.rasterize_text(&family_name, &text_str, fs, text_color, tk)
                {
                    let origin_x = (tb.cx - tw as f32 * 0.5) as i32;
                    let origin_y = (tb.cy - th as f32 * 0.5) as i32;
                    let text_w = tw as i32;
                    let text_h = th as i32;
                    let qx0 = origin_x.max(tb.min_x as i32).clamp(0, width as i32) as u32;
                    let qy0 = origin_y.max(tb.min_y as i32).clamp(0, height as i32) as u32;
                    let qx1 = (origin_x + text_w)
                        .clamp(tb.min_x as i32, tb.max_x as i32)
                        .max(qx0 as i32) as u32;
                    let qy1 = (origin_y + text_h)
                        .clamp(tb.min_y as i32, tb.max_y as i32)
                        .max(qy0 as i32) as u32;
                    for py in qy0..qy1 {
                        for px in qx0..qx1 {
                            let tx = px as i32 - origin_x;
                            let ty = py as i32 - origin_y;
                            if tx < 0 || ty < 0 || (tx as u32) >= tw || (ty as u32) >= th {
                                continue;
                            }
                            let tidx = ((ty as u32 * tw + tx as u32) * 4) as usize;
                            if tidx + 3 >= text_pixels.len() {
                                continue;
                            }
                            let glyph_a = text_pixels[tidx + 3] as f32 / 255.0;
                            if glyph_a <= 0.001 {
                                continue;
                            }
                            let lidx = (((py - tb.min_y) * tb.bw + (px - tb.min_x)) * 4) as usize;
                            if lidx + 3 < layer_buf.len() {
                                let src_a = glyph_a;
                                layer_buf[lidx] = (text_color[0] * 255.0) as u8;
                                layer_buf[lidx + 1] = (text_color[1] * 255.0) as u8;
                                layer_buf[lidx + 2] = (text_color[2] * 255.0) as u8;
                                layer_buf[lidx + 3] = (src_a * 255.0) as u8;
                            }
                        }
                    }
                }
            });
        }
        _ => {
            for py in tb.min_y..tb.max_y {
                for px in tb.min_x..tb.max_x {
                    let dx = px as f32 - tb.cx;
                    let dy = py as f32 - tb.cy;
                    let lx = dx * tb.cos_r + dy * tb.sin_r;
                    let ly = -dx * tb.sin_r + dy * tb.cos_r;
                    if lx >= -tb.bounds_x && lx <= tb.bounds_x && ly >= -tb.bounds_y && ly <= tb.bounds_y {
                        let lidx = (((py - tb.min_y) * tb.bw + (px - tb.min_x)) * 4) as usize;
                        if lidx + 3 < layer_buf.len() {
                            let src_a = p.base_color[3];
                            layer_buf[lidx] = (p.base_color[0] * 255.0) as u8;
                            layer_buf[lidx + 1] = (p.base_color[1] * 255.0) as u8;
                            layer_buf[lidx + 2] = (p.base_color[2] * 255.0) as u8;
                            layer_buf[lidx + 3] = (src_a * 255.0) as u8;
                        }
                    }
                }
            }
        }
    }
}
