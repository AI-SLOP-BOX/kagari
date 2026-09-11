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
        cx,
        cy,
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
        // Text layers: rasterize glyphs via ab_glyph and composite
        use crate::core::font_rasterizer::with_font_rasterizer;
        use crate::core::text_layout::TextAlign;

        let text_color = *color;
        let stroke_c = *stroke_color;
        let stroke_w = *stroke_width;
        let text_str = text.clone();
        let fs = *font_size as f32;
        let tk = *tracking;
        let ld = *leading;
        let alignment = match align {
            1 => TextAlign::Center,
            2 => TextAlign::Right,
            _ => TextAlign::Left,
        };
        let family = font_family.clone();

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
                        let Some(rg) =
                            rasterizer.rasterize_glyph(&family_name, g.char_code, fs)
                        else {
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
                            (rg.width.max(rg.height) as f32 * 0.5 * std::f32::consts::SQRT_2)
                                .ceil();
                        let gx0 = (gcx - half_diag).floor().max(0.0) as u32;
                        let gy0 = (gcy - half_diag).floor().max(0.0) as u32;
                        let gx1 = (gcx + half_diag).ceil().min(width as f32) as u32;
                        let gy1 = (gcy + half_diag).ceil().min(height as f32) as u32;

                        for py in gy0..gy1 {
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
                                buffer[didx] = (text_color[0] * 255.0 * src_a
                                    + buffer[didx] as f32 * inv)
                                    as u8;
                                buffer[didx + 1] = (text_color[1] * 255.0 * src_a
                                    + buffer[didx + 1] as f32 * inv)
                                    as u8;
                                buffer[didx + 2] = (text_color[2] * 255.0 * src_a
                                    + buffer[didx + 2] as f32 * inv)
                                    as u8;
                                buffer[didx + 3] =
                                    ((src_a + buffer[didx + 3] as f32 / 255.0 * inv) * 255.0)
                                        as u8;
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
                rasterizer.rasterize_text_animated_stack(
                    &family_name,
                    &text_str,
                    fs,
                    text_color,
                    tk,
                    lead,
                    0.0,
                    alignment,
                    stack,
                    frame as f32 / comp.fps.max(1) as f32,
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
                rasterizer.rasterize_text_animated(
                    &family_name,
                    &text_str,
                    fs,
                    text_color,
                    tk,
                    lead,
                    0.0,
                    alignment,
                    &anim_owned,
                    frame as f32 / comp.fps.max(1) as f32,
                )
            } else {
                rasterizer.rasterize_text_formatted(
                    &family_name,
                    &text_str,
                    fs,
                    text_color,
                    tk,
                    ld,
                    0.0,
                    alignment,
                )
            };
            if let Some((tw, th, text_pixels)) = maybe_text {
                let text_w = tw as i32;
                let text_h = th as i32;
                let origin_x = (cx - tw as f32 * 0.5) as i32;
                let origin_y = (cy - th as f32 * 0.5) as i32;
                let stroke_radius = (stroke_w * 0.5).ceil() as i32;
                // Tighten the scan to bitmap bounds (+ stroke neighbor
                // margin): pixels outside cannot receive coverage, so
                // skipping them is exactly identical output at a fraction
                // of the cost (also shrinks time spent under the global
                // font lock during parallel renders).
                let qx0 = (origin_x - stroke_radius)
                    .max(min_x as i32)
                    .clamp(0, width as i32) as u32;
                let qy0 = (origin_y - stroke_radius)
                    .max(min_y as i32)
                    .clamp(0, height as i32) as u32;
                let qx1 = (origin_x + text_w + stroke_radius)
                    .clamp(min_x as i32, max_x as i32)
                    .max(qx0 as i32) as u32;
                let qy1 = (origin_y + text_h + stroke_radius)
                    .clamp(min_y as i32, max_y as i32)
                    .max(qy0 as i32) as u32;

                for py in qy0..qy1 {
                    for px in qx0..qx1 {
                        // Vector mask check
                        let mut mask_alpha = 1.0;
                        if !masks.is_empty() {
                            mask_alpha =
                                compute_combined_mask_coverage(px as f32, py as f32, masks);
                        }
                        if mask_alpha <= 0.001 {
                            continue;
                        }

                        let tx = px as i32 - origin_x;
                        let ty = py as i32 - origin_y;

                        // Sample fill alpha
                        let fill_alpha = if tx >= 0 && ty >= 0 && tx < text_w && ty < text_h {
                            let tidx = ((ty as u32 * tw + tx as u32) * 4) as usize;
                            if tidx + 3 < text_pixels.len() {
                                text_pixels[tidx + 3] as f32 / 255.0
                            } else {
                                0.0
                            }
                        } else {
                            0.0
                        };

                        // Sample stroke alpha: check if any neighbor within radius has fill
                        let mut stroke_alpha = 0.0f32;
                        if stroke_w > 0.1 && fill_alpha < 0.001 {
                            for dy in -stroke_radius..=stroke_radius {
                                for dx in -stroke_radius..=stroke_radius {
                                    let nx = tx + dx;
                                    let ny = ty + dy;
                                    if nx >= 0 && ny >= 0 && nx < text_w && ny < text_h {
                                        let dist = ((dx * dx + dy * dy) as f32).sqrt();
                                        if dist <= stroke_w * 0.5 {
                                            let nidx =
                                                ((ny as u32 * tw + nx as u32) * 4) as usize;
                                            if nidx + 3 < text_pixels.len() {
                                                let n_alpha =
                                                    text_pixels[nidx + 3] as f32 / 255.0;
                                                if n_alpha > 0.001 {
                                                    let edge_dist = stroke_w * 0.5 - dist;
                                                    stroke_alpha = stroke_alpha.max(
                                                        (edge_dist / (stroke_w * 0.25))
                                                            .clamp(0.0, 1.0),
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Composite: stroke behind fill
                        let lidx = (((py - min_y) * bw + (px - min_x)) * 4) as usize;
                        if lidx + 3 < layer_buf.len() {
                            if stroke_alpha > 0.001 {
                                let src_a = stroke_alpha * mask_alpha;
                                layer_buf[lidx] = (stroke_c[0] * 255.0) as u8;
                                layer_buf[lidx + 1] = (stroke_c[1] * 255.0) as u8;
                                layer_buf[lidx + 2] = (stroke_c[2] * 255.0) as u8;
                                layer_buf[lidx + 3] = (src_a * 255.0) as u8;
                            }
                            if fill_alpha > 0.001 {
                                let src_a = fill_alpha * mask_alpha;
                                layer_buf[lidx] = (text_color[0] * 255.0) as u8;
                                layer_buf[lidx + 1] = (text_color[1] * 255.0) as u8;
                                layer_buf[lidx + 2] = (text_color[2] * 255.0) as u8;
                                layer_buf[lidx + 3] = (src_a * 255.0) as u8;
                            }
                        }
                    }
                }
            }
        });
}
