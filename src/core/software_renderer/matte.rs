//! Track-matte buffer rendering for the software renderer.
//!
//! Renders the layer below a matted layer into a full-frame
//! alpha/luma matte buffer (solid/text/shape/image/particle/precomp
//! sources supported).

use super::{render_frame_to_pixels, render_precomp_layers};
use crate::core::sdf::rasterize_shape_sdf;
use crate::core::timeline::{Composition, Layer, LayerType, ShapeType, TrackMatteMode};

/// Render the track-matte source for `layer` (or `None` when it uses
/// no matte). Returned buffer is full-frame (`width` x `height` RGBA).
pub(crate) fn render_matte_buffer(
    comp: &Composition,
    layer: &Layer,
    frame: u32,
    width: u32,
    height: u32,
) -> Option<Vec<u8>> {
    if layer.track_matte != TrackMatteMode::None {
        // Find the layer below this one (the matte source)
        let layer_idx = comp.layers.iter().position(|l| l.id == layer.id);
        if let Some(idx) = layer_idx {
            if idx > 0 {
                let matte_layer = &comp.layers[idx - 1];
                if matte_layer.is_active(frame) && matte_layer.visible {
                    let m_frame = matte_layer.remap_frame(frame);
                    let (m_pos, m_scale, m_rot, m_opa) =
                        comp.resolve_world_transform(matte_layer, m_frame);
                    let m_opacity = (m_opa / 100.0).clamp(0.0, 1.0);
                    let m_rad = m_rot.to_radians();
                    let _m_cos = m_rad.cos();
                    let _m_sin = m_rad.sin();
                    let m_cx = m_pos[0];
                    let m_cy = m_pos[1];
                    let m_bw = width;
                    let m_bh = height;
                    let mut m_buf = vec![0u8; (m_bw * m_bh * 4) as usize];

                    // Render matte layer to get its alpha/luma
                    match &matte_layer.layer_type {
                        LayerType::Solid { color } => {
                            for py in 0..m_bh {
                                for px in 0..m_bw {
                                    let idx = ((py * m_bw + px) * 4) as usize;
                                    if idx + 3 < m_buf.len() {
                                        m_buf[idx] = (color[0] * 255.0) as u8;
                                        m_buf[idx + 1] = (color[1] * 255.0) as u8;
                                        m_buf[idx + 2] = (color[2] * 255.0) as u8;
                                        m_buf[idx + 3] = (color[3] * m_opacity * 255.0) as u8;
                                    }
                                }
                            }
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
                                if let Some((tw, th, text_pixels)) = rasterizer.rasterize_text(
                                    &family_name,
                                    &text_str,
                                    fs,
                                    text_color,
                                    tk,
                                ) {
                                    let origin_x = (m_cx - tw as f32 * 0.5) as i32;
                                    let origin_y = (m_cy - th as f32 * 0.5) as i32;
                                    for py in 0..m_bh {
                                        for px in 0..m_bw {
                                            let tx = px as i32 - origin_x;
                                            let ty = py as i32 - origin_y;
                                            if tx >= 0
                                                && ty >= 0
                                                && (tx as u32) < tw
                                                && (ty as u32) < th
                                            {
                                                let tidx =
                                                    ((ty as u32 * tw + tx as u32) * 4) as usize;
                                                let lidx = ((py * m_bw + px) * 4) as usize;
                                                if tidx + 3 < text_pixels.len()
                                                    && lidx + 3 < m_buf.len()
                                                {
                                                    m_buf[lidx] = text_pixels[tidx];
                                                    m_buf[lidx + 1] = text_pixels[tidx + 1];
                                                    m_buf[lidx + 2] = text_pixels[tidx + 2];
                                                    m_buf[lidx + 3] = (text_pixels[tidx + 3]
                                                        as f32
                                                        * m_opacity)
                                                        as u8;
                                                }
                                            }
                                        }
                                    }
                                }
                            });
                        }
                        LayerType::Shape {
                            shape_type,
                            color,
                            stroke_color,
                            stroke_width,
                            fill_type,
                            ..
                        } => {
                            let (shape_w, shape_h) = match shape_type {
                                ShapeType::Rectangle { width, height, .. }
                                | ShapeType::Ellipse { width, height } => {
                                    (width.evaluate(m_frame), height.evaluate(m_frame))
                                }
                                ShapeType::Star { outer_radius, .. }
                                | ShapeType::Polygon {
                                    radius: outer_radius,
                                    ..
                                } => {
                                    let diameter = outer_radius.evaluate(m_frame).abs() * 2.0;
                                    (diameter, diameter)
                                }
                                ShapeType::FreeformBezier { points, .. } => {
                                    let (mut max_x, mut max_y) = (0.0f32, 0.0f32);
                                    for point in points {
                                        max_x = max_x.max(point[0].abs());
                                        max_y = max_y.max(point[1].abs());
                                    }
                                    (max_x * 2.0, max_y * 2.0)
                                }
                            };
                            let bounds_x =
                                (shape_w.abs() * (m_scale[0].abs() / 100.0) * 0.5).max(0.0);
                            let bounds_y =
                                (shape_h.abs() * (m_scale[1].abs() / 100.0) * 0.5).max(0.0);
                            rasterize_shape_sdf(
                                &mut m_buf,
                                m_bw,
                                m_bh,
                                0,
                                0,
                                m_cx,
                                m_cy,
                                bounds_x,
                                bounds_y,
                                *color,
                                fill_type,
                                *stroke_color,
                                *stroke_width,
                                m_opacity,
                                shape_type,
                                m_frame,
                                matte_layer.trim_paths.as_ref(),
                            );
                        }
                        LayerType::Image { .. } | LayerType::Video { .. } => {
                            use crate::core::image_cache::with_image_cache;
                            let image_path = match &matte_layer.layer_type {
                                LayerType::Image { path } => path.clone(),
                                LayerType::Video {
                                    frames_dir,
                                    frame_count,
                                    speed,
                                    ..
                                } => {
                                    let sequence_frame = ((m_frame as f32 * speed.max(0.0))
                                        as u32)
                                        .min(frame_count.saturating_sub(1));
                                    crate::core::video_import::frame_path_in_dir(
                                        frames_dir,
                                        sequence_frame,
                                    )
                                        .to_string_lossy()
                                        .into_owned()
                                }
                                _ => unreachable!(),
                            };
                            with_image_cache(|cache| {
                                if let Some(image) = cache.load_image(&image_path) {
                                    let iw = image.width as f32;
                                    let ih = image.height as f32;
                                    let cos_r = m_rot.to_radians().cos();
                                    let sin_r = m_rot.to_radians().sin();
                                    let half_w =
                                        (comp.width as f32 * (m_scale[0].abs() / 100.0) * 0.5)
                                            .max(0.001);
                                    let half_h =
                                        (comp.height as f32 * (m_scale[1].abs() / 100.0) * 0.5)
                                            .max(0.001);
                                    for py in 0..m_bh {
                                        for px in 0..m_bw {
                                            let dx = px as f32 + 0.5 - m_cx;
                                            let dy = py as f32 + 0.5 - m_cy;
                                            let lx = dx * cos_r + dy * sin_r;
                                            let ly = -dx * sin_r + dy * cos_r;
                                            let u = lx / (half_w * 2.0) + 0.5;
                                            let v = ly / (half_h * 2.0) + 0.5;
                                            if !(0.0..1.0).contains(&u)
                                                || !(0.0..1.0).contains(&v)
                                            {
                                                continue;
                                            }
                                            let sx = ((u * iw) as u32).min(image.width - 1);
                                            let sy = ((v * ih) as u32).min(image.height - 1);
                                            let sidx = ((sy * image.width + sx) * 4) as usize;
                                            let didx = ((py * m_bw + px) * 4) as usize;
                                            if sidx + 3 < image.pixels.len()
                                                && didx + 3 < m_buf.len()
                                            {
                                                m_buf[didx..didx + 4].copy_from_slice(
                                                    &image.pixels[sidx..sidx + 4],
                                                );
                                                m_buf[didx + 3] = (m_buf[didx + 3] as f32
                                                    * m_opacity)
                                                    .round()
                                                    .clamp(0.0, 255.0)
                                                    as u8;
                                            }
                                        }
                                    }
                                }
                            });
                        }
                        LayerType::PreComp { comp_id } => {
                            // Render the nested composition as the matte source
                            if let Some(sub_comp) =
                                comp.sub_compositions.iter().find(|c| c.id == *comp_id)
                            {
                                let sub_buf =
                                    render_precomp_layers(comp, sub_comp, m_frame, m_bw, m_bh);
                                // Copy sub-comp pixels into matte buffer, applying matte layer opacity
                                for i in (0..m_buf.len()).step_by(4) {
                                    if i + 3 < sub_buf.len() && i + 3 < m_buf.len() {
                                        m_buf[i] = sub_buf[i];
                                        m_buf[i + 1] = sub_buf[i + 1];
                                        m_buf[i + 2] = sub_buf[i + 2];
                                        m_buf[i + 3] =
                                            (sub_buf[i + 3] as f32 * m_opacity) as u8;
                                    }
                                }
                            } else {
                                // Sub-comp not found: fall back to white
                                for py in 0..m_bh {
                                    for px in 0..m_bw {
                                        let idx = ((py * m_bw + px) * 4) as usize;
                                        if idx + 3 < m_buf.len() {
                                            m_buf[idx] = 255;
                                            m_buf[idx + 1] = 255;
                                            m_buf[idx + 2] = 255;
                                            m_buf[idx + 3] = (m_opacity * 255.0) as u8;
                                        }
                                    }
                                }
                            }
                        }
                        LayerType::Particle { .. } => {
                            let mut matte_comp = comp.clone();
                            let mut matte_source = matte_layer.clone();
                            matte_source.track_matte = TrackMatteMode::None;
                            matte_comp.layers = vec![matte_source];
                            let particle_pixels = render_frame_to_pixels(
                                &matte_comp,
                                m_frame,
                                m_bw,
                                m_bh,
                                0.0,
                                0,
                            );
                            let copy_len = m_buf.len().min(particle_pixels.len());
                            m_buf[..copy_len].copy_from_slice(&particle_pixels[..copy_len]);
                            for alpha in m_buf[3..].iter_mut().step_by(4) {
                                *alpha =
                                    (*alpha as f32 * m_opacity).round().clamp(0.0, 255.0) as u8;
                            }
                        }
                        _ => {
                            // Unsupported matte sources remain conservative full mattes.
                            for py in 0..m_bh {
                                for px in 0..m_bw {
                                    let idx = ((py * m_bw + px) * 4) as usize;
                                    if idx + 3 < m_buf.len() {
                                        m_buf[idx] = 255;
                                        m_buf[idx + 1] = 255;
                                        m_buf[idx + 2] = 255;
                                        m_buf[idx + 3] = (m_opacity * 255.0) as u8;
                                    }
                                }
                            }
                        }
                    }
                    Some(m_buf)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
        }
}
