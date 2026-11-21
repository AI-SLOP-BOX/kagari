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
) -> Result<Option<Vec<u8>>, ()> {
    if super::render_cancelled() {
        return Err(());
    }
    if layer.track_matte != TrackMatteMode::None {
        // Find the matte source layer: explicit track_matte_layer_id or fallback to adjacent layer above (idx - 1)
        let matte_layer_opt = if let Some(target_id) = &layer.track_matte_layer_id {
            comp.layers.iter().find(|l| l.id == *target_id)
        } else {
            let layer_idx = comp.layers.iter().position(|l| l.id == layer.id);
            layer_idx.and_then(|idx| {
                if idx > 0 {
                    comp.layers.get(idx - 1)
                } else {
                    None
                }
            })
        };

        if let Some(matte_layer) = matte_layer_opt {
            if matte_layer.is_active(frame) && matte_layer.visible {
                let source_frame = matte_layer.effective_render_frame(frame, comp.fps);
                let (m_pos, m_scale, m_rot, m_opa) =
                    comp.resolve_world_transform(matte_layer, frame);
                let m_opacity = (m_opa / 100.0).clamp(0.0, 1.0);
                let m_rad = m_rot.to_radians();
                let _m_cos = m_rad.cos();
                let _m_sin = m_rad.sin();
                let m_cx = m_pos[0];
                let m_cy = m_pos[1];
                let m_bw = width;
                let m_bh = height;
                let Some(matte_size) = super::rgba_buffer_size(m_bw, m_bh) else {
                    log::error!("[Renderer] Invalid track-matte dimensions {m_bw}x{m_bh}");
                    return Err(());
                };
                let Some(mut m_buf) = super::try_zeroed_buffer(matte_size) else {
                    log::error!(
                        "[Renderer] Could not allocate {matte_size} bytes for track matte {}",
                        matte_layer.id
                    );
                    return Err(());
                };

                // Render matte layer to get its alpha/luma
                match &matte_layer.layer_type {
                    LayerType::Solid { color } => {
                        for py in 0..m_bh {
                            if super::render_cancelled() {
                                break;
                            }
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
                                    if super::render_cancelled() {
                                        break;
                                    }
                                    for px in 0..m_bw {
                                        let tx = px as i32 - origin_x;
                                        let ty = py as i32 - origin_y;
                                        if tx >= 0
                                            && ty >= 0
                                            && (tx as u32) < tw
                                            && (ty as u32) < th
                                        {
                                            let tidx = ((ty as u32 * tw + tx as u32) * 4) as usize;
                                            let lidx = ((py * m_bw + px) * 4) as usize;
                                            if tidx + 3 < text_pixels.len()
                                                && lidx + 3 < m_buf.len()
                                            {
                                                m_buf[lidx] = text_pixels[tidx];
                                                m_buf[lidx + 1] = text_pixels[tidx + 1];
                                                m_buf[lidx + 2] = text_pixels[tidx + 2];
                                                m_buf[lidx + 3] = (text_pixels[tidx + 3] as f32
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
                                (width.evaluate(source_frame), height.evaluate(source_frame))
                            }
                            ShapeType::Star { outer_radius, .. }
                            | ShapeType::Polygon {
                                radius: outer_radius,
                                ..
                            } => {
                                let diameter = outer_radius.evaluate(source_frame).abs() * 2.0;
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
                        let bounds_x = (shape_w.abs() * (m_scale[0].abs() / 100.0) * 0.5).max(0.0);
                        let bounds_y = (shape_h.abs() * (m_scale[1].abs() / 100.0) * 0.5).max(0.0);
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
                            source_frame,
                            matte_layer.trim_paths.as_ref(),
                        );
                    }
                    LayerType::Image { .. } | LayerType::Video { .. } => {
                        let (image_path, next_image_path, blend_t) = match &matte_layer.layer_type {
                            LayerType::Image { path } => (path.clone(), None, 0.0),
                            LayerType::Video {
                                frames_dir,
                                frame_count,
                                speed,
                                ..
                            } => {
                                let remapped_source_frame = match &matte_layer.posterize_time {
                                    Some(settings) if settings.enabled => source_frame as f32,
                                    _ => matte_layer.remap_frame_f32(frame),
                                };
                                let source_position = crate::core::timeline::video_sequence_frame(
                                    remapped_source_frame,
                                    *speed,
                                    frames_dir,
                                    comp.fps,
                                );
                                let first = (source_position.floor() as u32)
                                    .min(frame_count.saturating_sub(1));
                                let second = (first + 1).min(frame_count.saturating_sub(1));
                                let blend_t = if matte_layer.frame_blending {
                                    (source_position - first as f32).clamp(0.0, 1.0)
                                } else {
                                    0.0
                                };
                                (
                                    crate::core::video_import::frame_path_in_dir(frames_dir, first)
                                        .to_string_lossy()
                                        .into_owned(),
                                    Some(
                                        crate::core::video_import::frame_path_in_dir(
                                            frames_dir, second,
                                        )
                                        .to_string_lossy()
                                        .into_owned(),
                                    ),
                                    blend_t,
                                )
                            }
                            _ => unreachable!(),
                        };
                        let images =
                            crate::core::image_cache::load_image_shared(&image_path).map(|image| {
                                let next_image = next_image_path
                                    .as_deref()
                                    .filter(|path| *path != image_path && blend_t > 0.0)
                                    .and_then(crate::core::image_cache::load_image_shared)
                                    .filter(|next| {
                                        next.width == image.width && next.height == image.height
                                    });
                                (image, next_image)
                            });
                        if let Some((image, next_image)) = images {
                            let iw = image.width as f32;
                            let ih = image.height as f32;
                            let cos_r = m_rot.to_radians().cos();
                            let sin_r = m_rot.to_radians().sin();
                            let half_w =
                                (comp.width as f32 * (m_scale[0].abs() / 100.0) * 0.5).max(0.001);
                            let half_h =
                                (comp.height as f32 * (m_scale[1].abs() / 100.0) * 0.5).max(0.001);
                            for py in 0..m_bh {
                                if super::render_cancelled() {
                                    break;
                                }
                                for px in 0..m_bw {
                                    let dx = px as f32 + 0.5 - m_cx;
                                    let dy = py as f32 + 0.5 - m_cy;
                                    let lx = dx * cos_r + dy * sin_r;
                                    let ly = -dx * sin_r + dy * cos_r;
                                    let u = lx / (half_w * 2.0) + 0.5;
                                    let v = ly / (half_h * 2.0) + 0.5;
                                    if !(0.0..1.0).contains(&u) || !(0.0..1.0).contains(&v) {
                                        continue;
                                    }
                                    let sx = ((u * iw) as u32).min(image.width - 1);
                                    let sy = ((v * ih) as u32).min(image.height - 1);
                                    let sidx = ((sy * image.width + sx) * 4) as usize;
                                    let didx = ((py * m_bw + px) * 4) as usize;
                                    if sidx + 3 < image.pixels.len() && didx + 3 < m_buf.len() {
                                        let sample_a = std::array::from_fn(|channel| {
                                            image.pixels[sidx + channel] as f32
                                        });
                                        let sample_b = next_image
                                            .as_ref()
                                            .filter(|next| {
                                                next.width == image.width
                                                    && next.height == image.height
                                            })
                                            .map(|next| {
                                                let next_idx =
                                                    ((sy * next.width + sx) * 4) as usize;
                                                std::array::from_fn(|channel| {
                                                    next.pixels[next_idx + channel] as f32
                                                })
                                            })
                                            .unwrap_or(sample_a);
                                        let sample =
                                            crate::core::frame_blending::blend_rgba_samples(
                                                sample_a, sample_b, blend_t,
                                            );
                                        m_buf[didx] = sample[0].round().clamp(0.0, 255.0) as u8;
                                        m_buf[didx + 1] = sample[1].round().clamp(0.0, 255.0) as u8;
                                        m_buf[didx + 2] = sample[2].round().clamp(0.0, 255.0) as u8;
                                        m_buf[didx + 3] =
                                            (sample[3] * m_opacity).round().clamp(0.0, 255.0) as u8;
                                    }
                                }
                            }
                        }
                    }
                    LayerType::PreComp { comp_id } => {
                        // Render the nested composition as the matte source
                        if let Some(sub_comp) =
                            comp.sub_compositions.iter().find(|c| c.id == *comp_id)
                        {
                            let sub_buf =
                                render_precomp_layers(comp, sub_comp, source_frame, m_bw, m_bh);
                            // Copy sub-comp pixels into matte buffer, applying matte layer opacity
                            for (pixel_index, pixel) in
                                m_buf.as_chunks_mut::<4>().0.iter_mut().enumerate()
                            {
                                if pixel_index % 4096 == 0 && super::render_cancelled() {
                                    break;
                                }
                                let i = pixel_index * 4;
                                if i + 3 < sub_buf.len() {
                                    pixel[0] = sub_buf[i];
                                    pixel[1] = sub_buf[i + 1];
                                    pixel[2] = sub_buf[i + 2];
                                    pixel[3] = (sub_buf[i + 3] as f32 * m_opacity) as u8;
                                }
                            }
                        } else {
                            // Sub-comp not found: fall back to white
                            for py in 0..m_bh {
                                if super::render_cancelled() {
                                    break;
                                }
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
                        let particle_pixels =
                            render_frame_to_pixels(&matte_comp, frame, m_bw, m_bh, 0.0, 0);
                        let copy_len = m_buf.len().min(particle_pixels.len());
                        m_buf[..copy_len].copy_from_slice(&particle_pixels[..copy_len]);
                        for (pixel_index, pixel) in
                            m_buf.as_chunks_mut::<4>().0.iter_mut().enumerate()
                        {
                            if pixel_index % 4096 == 0 && super::render_cancelled() {
                                break;
                            }
                            pixel[3] =
                                (pixel[3] as f32 * m_opacity).round().clamp(0.0, 255.0) as u8;
                        }
                    }
                    _ => {
                        // Unsupported matte sources remain conservative full mattes.
                        for py in 0..m_bh {
                            if super::render_cancelled() {
                                break;
                            }
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
                if super::render_cancelled() {
                    return Err(());
                }
                Ok(Some(m_buf))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::timeline::Composition;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    #[test]
    fn cancelled_matte_render_returns_without_a_buffer() {
        let comp = Composition::new("matte-cancel".into(), "Matte Cancel".into(), 8, 8, 24, 1);
        let layer = Layer::new("content".into(), "Content".into(), LayerType::Null, 24);
        let flag = Arc::new(AtomicBool::new(true));

        let matte = super::super::with_render_cancel_flag(flag, || {
            render_matte_buffer(&comp, &layer, 0, 8, 8)
        });

        assert!(matte.is_err());
    }
}
