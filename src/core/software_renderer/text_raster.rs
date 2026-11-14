//! Text layer rasterization (glyph bitmaps, animators, path text).

use super::mask::compute_combined_mask_coverage;
use super::raster::RasterCtx;
use crate::core::timeline::LayerType;

/// Rasterize a text layer into its local buffer.
pub(crate) fn rasterize_text_layer(ctx: RasterCtx<'_>) {
    let RasterCtx {
        comp,
        layer,
        frame,
        effective_frame,
        masks,
        min_x,
        min_y,
        max_x,
        max_y,
        bw,
        width,
        height,
        scale,
        cx,
        cy,
        cos_r,
        sin_r,
        l_opacity,
        buffer,
        layer_buf,
        ..
    } = ctx;
    let LayerType::Text {
        text,
        font_size,
        color,
        font_family,
        tracking,
        stroke_color,
        stroke_width,
        leading,
        align,
        text_on_path,
        ..
    } = &layer.layer_type
    else {
        return;
    };
    let formatting = layer.text_formatting.as_ref();
    let effective_family = formatting
        .map(|tf| tf.font_family.clone())
        .unwrap_or_else(|| font_family.clone());
    let effective_tracking = formatting.map(|tf| tf.tracking).unwrap_or(*tracking);
    let effective_leading = formatting.map(|tf| tf.leading).unwrap_or(*leading);
    let effective_align = formatting.map(|tf| tf.alignment as usize).unwrap_or(*align);
    let effective_box_width = formatting.map(|tf| tf.box_width).unwrap_or(0.0);
    let effective_box_height = formatting.map(|tf| tf.box_height).unwrap_or(0.0);
    let effective_stroke_color = formatting
        .map(|tf| tf.stroke_color.unwrap_or([0.0, 0.0, 0.0, 0.0]))
        .unwrap_or(*stroke_color);
    let effective_stroke_width = formatting
        .map(|tf| {
            if tf.stroke_color.is_some() {
                tf.stroke_width
            } else {
                0.0
            }
        })
        .unwrap_or(*stroke_width);
    let faux_style = formatting
        .map(|tf| crate::core::font_rasterizer::FauxTextStyle {
            bold: tf.faux_bold,
            italic: tf.faux_italic,
            all_caps: tf.all_caps,
            small_caps: tf.small_caps,
        })
        .unwrap_or_default();
    // Text layers: rasterize glyphs via ab_glyph and composite
    use crate::core::font_rasterizer::with_font_rasterizer;
    use crate::core::text_layout::TextAlign;

    let text_color = *color;
    let stroke_c = effective_stroke_color;
    let stroke_w = effective_stroke_width;
    let text_str = text.clone();
    let fs = *font_size as f32;
    let tk = effective_tracking;
    let ld = effective_leading;
    let alignment = match effective_align {
        1 => TextAlign::Center,
        2 => TextAlign::Right,
        3 => TextAlign::Justify,
        _ => TextAlign::Left,
    };
    let family = effective_family.clone();

    with_font_rasterizer(|rasterizer| {
        let family_name = rasterizer.resolve_family(&family);

        // ── Text on Path: lay glyphs out along the first mask path ──
        if *text_on_path && !layer.masks.is_empty() {
            use crate::core::mask::MaskVertex;
            use crate::core::path_text::{layout_text_along_path, PathTextOptions};

            let mask = &layer.masks[0];
            let path_points = mask.path.to_polygon(effective_frame, 12);
            if path_points.len() >= 2 {
                let verts: Vec<MaskVertex> = path_points
                    .windows(2)
                    .map(|w| MaskVertex {
                        position: w[0],
                        tangent_in: [0.0; 2],
                        tangent_out: [w[1][0] - w[0][0], w[1][1] - w[0][1]],
                    })
                    .collect();

                let glyphs = layout_text_along_path(
                    &text_str,
                    fs,
                    &verts,
                    mask.path.is_closed,
                    &PathTextOptions::default(),
                );
                for g in &glyphs {
                    if super::render_cancelled() {
                        return;
                    }
                    let Some(rg) = rasterizer.rasterize_glyph(&family_name, g.char_code, fs) else {
                        continue;
                    };
                    if rg.width == 0 || rg.height == 0 {
                        continue;
                    }

                    // Glyph center in comp coordinates, rotated by path tangent
                    let angle = g.rotation_deg.to_radians();
                    let cos_a = angle.cos();
                    let sin_a = angle.sin();
                    let gcx = g.position[0];
                    let gcy = g.position[1] + fs * 0.35; // approximate baseline centering

                    // Rotated blit over the bounding box of the rotated glyph
                    let half_diag =
                        (rg.width.max(rg.height) as f32 * 0.5 * std::f32::consts::SQRT_2).ceil();
                    let gx0 = (gcx - half_diag).floor().max(0.0) as u32;
                    let gy0 = (gcy - half_diag).floor().max(0.0) as u32;
                    let gx1 = (gcx + half_diag).ceil().min(width as f32) as u32;
                    let gy1 = (gcy + half_diag).ceil().min(height as f32) as u32;

                    for py in gy0..gy1 {
                        if super::render_cancelled() {
                            return;
                        }
                        for px in gx0..gx1 {
                            // Inverse-rotate the destination point into glyph space
                            let dx = px as f32 + 0.5 - gcx;
                            let dy = py as f32 + 0.5 - gcy;
                            let lx = dx * cos_a + dy * sin_a;
                            let ly = -dx * sin_a + dy * cos_a;
                            // Glyph local coords: center the bitmap around the path point
                            let tx = lx + rg.width as f32 * 0.5 + rg.left as f32;
                            let ty = ly + rg.height as f32 * 0.5 + rg.top as f32;
                            if tx < 0.0
                                || ty < 0.0
                                || tx >= rg.width as f32
                                || ty >= rg.height as f32
                            {
                                continue;
                            }
                            let tidx = ((ty as u32 * rg.width + tx as u32) * 4) as usize;
                            if tidx + 3 >= rg.pixels.len() {
                                continue;
                            }
                            let cov = rg.pixels[tidx + 3] as f32 / 255.0;
                            if cov <= 0.001 {
                                continue;
                            }

                            let src_a = cov * l_opacity;
                            if src_a <= 0.001 {
                                continue;
                            }
                            let didx = (((py * width) + px) * 4) as usize;
                            if didx + 3 >= buffer.len() {
                                continue;
                            }
                            // Straight-alpha over using glyph coverage tinted with text color
                            let inv = 1.0 - src_a;
                            buffer[didx] =
                                (text_color[0] * 255.0 * src_a + buffer[didx] as f32 * inv) as u8;
                            buffer[didx + 1] = (text_color[1] * 255.0 * src_a
                                + buffer[didx + 1] as f32 * inv)
                                as u8;
                            buffer[didx + 2] = (text_color[2] * 255.0 * src_a
                                + buffer[didx + 2] as f32 * inv)
                                as u8;
                            buffer[didx + 3] =
                                ((src_a + buffer[didx + 3] as f32 / 255.0 * inv) * 255.0) as u8;
                        }
                    }
                }
            }
            return;
        }

        // Text Animator: prefer stack (multi-animator) if present, else single animator
        let maybe_text = if let Some(stack) = layer
            .text_animator_stack
            .as_ref()
            .filter(|s| !s.animators.is_empty())
        {
            let lead = layer
                .text_formatting
                .as_ref()
                .map(|tf| tf.leading)
                .unwrap_or(1.2);
            rasterizer.rasterize_text_animated_stack_styled(
                &family_name,
                &text_str,
                fs,
                text_color,
                tk,
                lead,
                effective_box_width,
                alignment,
                stack,
                frame as f32 / comp.fps.max(1) as f32,
                faux_style,
            )
        } else if let Some(anim) = layer.text_animator.as_ref().filter(|a| a.enabled) {
            let lead = layer
                .text_formatting
                .as_ref()
                .map(|tf| tf.leading)
                .unwrap_or(1.2);
            let anim_owned = if let Some(ref oa) = anim.selector.offset_anim {
                let mut a2 = anim.clone();
                a2.selector.offset = oa.evaluate(frame);
                a2
            } else {
                anim.clone()
            };
            rasterizer.rasterize_text_animated_styled(
                &family_name,
                &text_str,
                fs,
                text_color,
                tk,
                lead,
                effective_box_width,
                alignment,
                &anim_owned,
                frame as f32 / comp.fps.max(1) as f32,
                faux_style,
            )
        } else {
            rasterizer.rasterize_text_formatted_styled(
                &family_name,
                &text_str,
                fs,
                text_color,
                tk,
                ld,
                effective_box_width,
                alignment,
                faux_style,
            )
        };
        if let Some(rasterized) = maybe_text {
            let (base_w, base_h, base_pixels) =
                crate::core::font_rasterizer::clip_text_bitmap_height(
                    rasterized,
                    effective_box_height,
                );
            let (tw, th, text_pixels) = if stroke_w > 0.1 {
                crate::core::text_stroke::bake_text_stroke(
                    &base_pixels,
                    base_w,
                    base_h,
                    stroke_c,
                    stroke_w,
                )
            } else {
                (base_w, base_h, base_pixels)
            };
            let scale_x = scale[0] / 100.0;
            let scale_y = scale[1] / 100.0;
            if tw == 0 || th == 0 || scale_x.abs() <= f32::EPSILON || scale_y.abs() <= f32::EPSILON
            {
                return;
            }

            // GPU text quads use the rasterized bitmap's dimensions for their
            // anchor transform; software layer bounds use composition size.
            let offset_x = (tw as f32 - comp.width as f32) * 0.5 * scale_x;
            let offset_y = (th as f32 - comp.height as f32) * 0.5 * scale_y;
            let text_cx = cx + cos_r * offset_x - sin_r * offset_y;
            let text_cy = cy + sin_r * offset_x + cos_r * offset_y;
            let half_x = cos_r.abs() * tw as f32 * scale_x.abs() * 0.5
                + sin_r.abs() * th as f32 * scale_y.abs() * 0.5;
            let half_y = sin_r.abs() * tw as f32 * scale_x.abs() * 0.5
                + cos_r.abs() * th as f32 * scale_y.abs() * 0.5;
            let qx0 = (text_cx - half_x).floor().max(min_x as f32).max(0.0) as u32;
            let qy0 = (text_cy - half_y).floor().max(min_y as f32).max(0.0) as u32;
            let qx1 = (text_cx + half_x)
                .ceil()
                .min(max_x as f32)
                .min(width as f32) as u32;
            let qy1 = (text_cy + half_y)
                .ceil()
                .min(max_y as f32)
                .min(height as f32) as u32;
            for py in qy0..qy1 {
                if super::render_cancelled() {
                    return;
                }
                for px in qx0..qx1 {
                    let dx = px as f32 + 0.5 - text_cx;
                    let dy = py as f32 + 0.5 - text_cy;
                    let local_x = dx * cos_r + dy * sin_r;
                    let local_y = -dx * sin_r + dy * cos_r;
                    let u = local_x / (tw as f32 * scale_x) + 0.5;
                    let v = local_y / (th as f32 * scale_y) + 0.5;
                    let Some(sample) =
                        super::sampling::sample_rgba8_linear(&text_pixels, tw, th, u, v)
                    else {
                        continue;
                    };
                    let mut alpha = (sample[3] / 255.0).clamp(0.0, 1.0);
                    if alpha <= 0.001 {
                        continue;
                    }
                    if !masks.is_empty() {
                        alpha *=
                            compute_combined_mask_coverage(px as f32 + 0.5, py as f32 + 0.5, masks);
                    }
                    let lidx = (((py - min_y) * bw + (px - min_x)) * 4) as usize;
                    if lidx + 3 < layer_buf.len() && alpha > 0.001 {
                        layer_buf[lidx] = sample[0].round().clamp(0.0, 255.0) as u8;
                        layer_buf[lidx + 1] = sample[1].round().clamp(0.0, 255.0) as u8;
                        layer_buf[lidx + 2] = sample[2].round().clamp(0.0, 255.0) as u8;
                        layer_buf[lidx + 3] = (alpha * 255.0).round() as u8;
                    }
                }
            }
        }
    });
}
