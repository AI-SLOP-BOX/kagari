use super::types::{
    checked_animated_glyph_pixels, checked_text_dimensions, parse_font, zeroed_glyph_bitmap,
    zeroed_glyph_coverage, zeroed_text_bitmap,
};
use ab_glyph::{Font, PxScale, ScaleFont};

fn visible_glyph_axis_range(
    center: f32,
    destination_extent: i32,
    output_extent: u32,
) -> std::ops::Range<i32> {
    if !center.is_finite() || destination_extent <= 0 || output_extent == 0 {
        return 0..0;
    }
    let half = destination_extent as f64 * 0.5;
    let center = center as f64;
    let start = (half - center - 0.5).floor() + 1.0;
    let end = (output_extent as f64 - 0.5 - center + half).ceil();
    let start = start.clamp(-2.0, destination_extent as f64) as i32;
    let end = end.clamp(-2.0, destination_extent as f64) as i32;
    start..end.max(start)
}

#[allow(clippy::too_many_arguments)]
pub fn rasterize_text_animated(
    font_data: &[u8],
    text: &str,
    font_size: f32,
    color: [f32; 4],
    tracking: f32,
    leading: f32,
    box_width: f32,
    alignment: crate::core::text_layout::TextAlign,
    animator: &crate::core::text_animator::TextAnimatorSettings,
    time: f32,
) -> Option<(u32, u32, Vec<u8>)> {
    if !font_size.is_finite()
        || !(0.1..=8192.0).contains(&font_size)
        || !tracking.is_finite()
        || !leading.is_finite()
        || leading <= 0.0
        || leading > 100.0
        || !box_width.is_finite()
        || box_width < 0.0
        || !time.is_finite()
        || !color.iter().all(|value| value.is_finite())
    {
        return None;
    }
    let font = parse_font(font_data)?;
    let scaled_font = font.as_scaled(PxScale::from(font_size));

    let flat_text: String = text.chars().filter(|&c| c != '\n' && c != '\r').collect();
    if flat_text.is_empty() {
        return None;
    }
    let xforms = crate::core::text_animator::TextAnimatorEngine::eval_character_transforms_extended(
        &flat_text,
        &animator.selector,
        animator.position_offset,
        animator.scale,
        animator.opacity,
        animator.tracking,
        animator.rotation,
        animator.blur_amount,
        false,
        time,
    );

    let layout = crate::core::text_layout::layout_text(
        text, font_size, tracking, leading, box_width, alignment,
    );
    if layout.lines.is_empty() {
        return None;
    }

    let pad = (font_size.ceil() as u32 + 96).max(128);
    let line_height = font_size * leading;
    let (buf_w, buf_h) = checked_text_dimensions(
        layout.total_width.max(box_width),
        layout.total_height.max(line_height),
        pad,
    )?;
    let mut pixels = zeroed_text_bitmap(buf_w, buf_h)?;
    let r = (color[0].clamp(0.0, 1.0) * 255.0) as u8;
    let g = (color[1].clamp(0.0, 1.0) * 255.0) as u8;
    let b = (color[2].clamp(0.0, 1.0) * 255.0) as u8;
    let color_alpha = color[3].clamp(0.0, 1.0);
    let max_top = font_size * 0.8;

    let mut char_idx: usize = 0;
    for line in &layout.lines {
        let mut actual_line_width = 0.0f32;
        let mut measure_characters = line.text.chars().peekable();
        while let Some(character) = measure_characters.next() {
            let glyph_id = font.glyph_id(character);
            actual_line_width += scaled_font.h_advance(glyph_id);
            if let Some(next) = measure_characters.peek() {
                actual_line_width += scaled_font.kern(glyph_id, font.glyph_id(*next)) + tracking;
            }
        }
        let align_x =
            crate::core::text_layout::get_alignment_offset(actual_line_width, box_width, alignment);
        let justify_space = crate::core::text_layout::justification_space_width(
            line,
            actual_line_width,
            box_width,
            alignment,
        );
        let mut cursor_x = align_x;
        let cursor_y = line.y_offset;

        let mut characters = line.text.chars().peekable();
        while let Some(ch) = characters.next() {
            let c = xforms
                .get(char_idx)
                .cloned()
                .unwrap_or_else(crate::core::text_animator::CharacterTransform::default);
            let glyph_id = font.glyph_id(ch);
            let h_advance = scaled_font.h_advance(glyph_id);
            let kerning = characters
                .peek()
                .map(|next| scaled_font.kern(glyph_id, font.glyph_id(*next)))
                .unwrap_or(0.0);

            if ch != ' ' && (c.opacity_multiplier > 0.01) {
                let glyph = glyph_id
                    .with_scale_and_position(PxScale::from(font_size), ab_glyph::point(0.0, 0.0));
                if let Some(outlined) = font.outline_glyph(glyph) {
                    let bounds = outlined.px_bounds();
                    let bw = bounds.width().ceil() as usize;
                    let bh = bounds.height().ceil() as usize;
                    if let Some(glyph_pixels) = checked_animated_glyph_pixels(bw, bh) {
                        let Some(mut cov) = zeroed_glyph_coverage(glyph_pixels) else {
                            continue;
                        };
                        {
                            let cov_ref = &mut cov;
                            outlined.draw(|x, y, coverage| {
                                let xi = x as usize;
                                let yi = y as usize;
                                if xi < bw && yi < bh {
                                    cov_ref[yi * bw + xi] = coverage;
                                }
                            });
                        }

                        let Some(mut src_rgba) = zeroed_glyph_bitmap(bw, bh) else {
                            continue;
                        };
                        for (i, cv) in cov.iter().enumerate() {
                            src_rgba[i * 4] = r;
                            src_rgba[i * 4 + 1] = g;
                            src_rgba[i * 4 + 2] = b;
                            src_rgba[i * 4 + 3] = (cv * 255.0) as u8;
                        }
                        let blur_px = c.blur.round().max(0.0) as u32;
                        if blur_px >= 1 {
                            crate::core::ae_effects_pack::apply_gaussian_blur(
                                &mut src_rgba,
                                bw as u32,
                                bh as u32,
                                blur_px.min(16),
                            );
                        }

                        let dest_x = pad as f32
                            + cursor_x
                            + c.position_offset[0]
                            + c.tracking_offset
                            + bounds.min.x;
                        let dest_y =
                            pad as f32 + cursor_y + max_top + bounds.min.y + c.position_offset[1];

                        let sx = c.scale_multiplier[0].max(0.01);
                        let sy = c.scale_multiplier[1].max(0.01);
                        let rad = c.rotation_deg.to_radians();
                        let (cos_r, sin_r) = (rad.cos(), rad.sin());
                        let dst_w = ((bounds.width() * sx).ceil() as i32).saturating_add(4);
                        let dst_h = ((bounds.height() * sy).ceil() as i32).saturating_add(4);
                        let cx_dst = dest_x + dst_w as f32 * 0.5;
                        let cy_dst = dest_y + dst_h as f32 * 0.5;
                        let visible_x = visible_glyph_axis_range(cx_dst, dst_w, buf_w);
                        let visible_y = visible_glyph_axis_range(cy_dst, dst_h, buf_h);
                        for dy in visible_y {
                            for dx in visible_x.clone() {
                                let rx = dx as f32 - dst_w as f32 * 0.5;
                                let ry = dy as f32 - dst_h as f32 * 0.5;
                                let ux = (rx * cos_r + ry * sin_r) / sx + bounds.width() * 0.5;
                                let uy = (-rx * sin_r + ry * cos_r) / sy + bounds.height() * 0.5;
                                let sxi = ux.floor() as isize;
                                let syi = uy.floor() as isize;
                                if sxi < 0 || syi < 0 || sxi >= bw as isize || syi >= bh as isize {
                                    continue;
                                }
                                let a = src_rgba[((syi as usize) * bw + (sxi as usize)) * 4 + 3];
                                if a == 0 {
                                    continue;
                                }
                                let pxi = (cx_dst + rx).round() as isize;
                                let pyi = (cy_dst + ry).round() as isize;
                                if pxi < 0
                                    || pyi < 0
                                    || pxi >= buf_w as isize
                                    || pyi >= buf_h as isize
                                {
                                    continue;
                                }
                                let out_a = (a as f32 * c.opacity_multiplier * color_alpha)
                                    .round()
                                    .clamp(0.0, 255.0)
                                    as u8;
                                if out_a == 0 {
                                    continue;
                                }
                                let idx = ((pyi as u32 * buf_w + pxi as u32) * 4) as usize;
                                pixels[idx] = r;
                                pixels[idx + 1] = g;
                                pixels[idx + 2] = b;
                                pixels[idx + 3] = out_a;
                            }
                        }
                    }
                }
            }

            cursor_x += h_advance
                + kerning
                + tracking
                + c.tracking_offset
                + if ch == ' ' { justify_space } else { 0.0 };
            char_idx += 1;
        }
    }

    Some((buf_w, buf_h, pixels))
}

#[allow(clippy::too_many_arguments)]
pub fn rasterize_text_animated_stack(
    font_data: &[u8],
    text: &str,
    font_size: f32,
    color: [f32; 4],
    tracking: f32,
    leading: f32,
    box_width: f32,
    alignment: crate::core::text_layout::TextAlign,
    stack: &crate::core::text_animator_advanced::AnimatorStack,
    _time: f32,
) -> Option<(u32, u32, Vec<u8>)> {
    if !font_size.is_finite()
        || !(0.1..=8192.0).contains(&font_size)
        || !tracking.is_finite()
        || !leading.is_finite()
        || leading <= 0.0
        || leading > 100.0
        || !box_width.is_finite()
        || box_width < 0.0
        || !color.iter().all(|value| value.is_finite())
    {
        return None;
    }
    let font = parse_font(font_data)?;
    let scaled_font = font.as_scaled(PxScale::from(font_size));

    let flat_text: String = text.chars().filter(|&c| c != '\n' && c != '\r').collect();
    if flat_text.is_empty() {
        return None;
    }

    let advanced = stack.compose(&flat_text);
    let xforms: Vec<crate::core::text_animator::CharacterTransform> =
        advanced.iter().map(|a| a.base).collect();

    let layout = crate::core::text_layout::layout_text(
        text, font_size, tracking, leading, box_width, alignment,
    );
    if layout.lines.is_empty() {
        return None;
    }

    let pad = (font_size.ceil() as u32 + 96).max(128);
    let line_height = font_size * leading;
    let (buf_w, buf_h) = checked_text_dimensions(
        layout.total_width.max(box_width),
        layout.total_height.max(line_height),
        pad,
    )?;
    let mut pixels = zeroed_text_bitmap(buf_w, buf_h)?;
    let r = (color[0].clamp(0.0, 1.0) * 255.0) as u8;
    let g = (color[1].clamp(0.0, 1.0) * 255.0) as u8;
    let b = (color[2].clamp(0.0, 1.0) * 255.0) as u8;
    let color_alpha = color[3].clamp(0.0, 1.0);
    let max_top = font_size * 0.8;

    let mut char_idx: usize = 0;
    for line in &layout.lines {
        let mut actual_line_width = 0.0f32;
        let mut measure_characters = line.text.chars().peekable();
        while let Some(character) = measure_characters.next() {
            let glyph_id = font.glyph_id(character);
            actual_line_width += scaled_font.h_advance(glyph_id);
            if let Some(next) = measure_characters.peek() {
                actual_line_width += scaled_font.kern(glyph_id, font.glyph_id(*next)) + tracking;
            }
        }
        let align_x =
            crate::core::text_layout::get_alignment_offset(actual_line_width, box_width, alignment);
        let justify_space = crate::core::text_layout::justification_space_width(
            line,
            actual_line_width,
            box_width,
            alignment,
        );
        let mut cursor_x = align_x;
        let cursor_y = line.y_offset;

        let mut characters = line.text.chars().peekable();
        while let Some(ch) = characters.next() {
            let c = xforms
                .get(char_idx)
                .cloned()
                .unwrap_or_else(crate::core::text_animator::CharacterTransform::default);
            let glyph_id = font.glyph_id(ch);
            let h_advance = scaled_font.h_advance(glyph_id);
            let kerning = characters
                .peek()
                .map(|next| scaled_font.kern(glyph_id, font.glyph_id(*next)))
                .unwrap_or(0.0);

            if ch != ' ' && (c.opacity_multiplier > 0.01) {
                let glyph = glyph_id
                    .with_scale_and_position(PxScale::from(font_size), ab_glyph::point(0.0, 0.0));
                if let Some(outlined) = font.outline_glyph(glyph) {
                    let bounds = outlined.px_bounds();
                    let bw = bounds.width().ceil() as usize;
                    let bh = bounds.height().ceil() as usize;
                    if let Some(glyph_pixels) = checked_animated_glyph_pixels(bw, bh) {
                        let Some(mut cov) = zeroed_glyph_coverage(glyph_pixels) else {
                            continue;
                        };
                        outlined.draw(|x, y, coverage| {
                            let xi = x as usize;
                            let yi = y as usize;
                            if xi < bw && yi < bh {
                                cov[yi * bw + xi] = coverage;
                            }
                        });
                        let (char_r, char_g, char_b) =
                            if let Some(fc) = advanced.get(char_idx).and_then(|a| a.fill_color) {
                                (
                                    (fc[0].clamp(0.0, 1.0) * 255.0) as u8,
                                    (fc[1].clamp(0.0, 1.0) * 255.0) as u8,
                                    (fc[2].clamp(0.0, 1.0) * 255.0) as u8,
                                )
                            } else {
                                (r, g, b)
                            };
                        let Some(mut src_rgba) = zeroed_glyph_bitmap(bw, bh) else {
                            continue;
                        };
                        for (i, cv) in cov.iter().enumerate() {
                            src_rgba[i * 4] = char_r;
                            src_rgba[i * 4 + 1] = char_g;
                            src_rgba[i * 4 + 2] = char_b;
                            src_rgba[i * 4 + 3] = (cv * 255.0) as u8;
                        }
                        let blur_px = c.blur.round().max(0.0) as u32;
                        if blur_px >= 1 {
                            crate::core::ae_effects_pack::apply_gaussian_blur(
                                &mut src_rgba,
                                bw as u32,
                                bh as u32,
                                blur_px.min(16),
                            );
                        }

                        let dest_x = pad as f32
                            + cursor_x
                            + c.position_offset[0]
                            + c.tracking_offset
                            + bounds.min.x;
                        let dest_y =
                            pad as f32 + cursor_y + max_top + bounds.min.y + c.position_offset[1];

                        let sx = c.scale_multiplier[0].max(0.01);
                        let sy = c.scale_multiplier[1].max(0.01);
                        let rad = c.rotation_deg.to_radians();
                        let (cos_r, sin_r) = (rad.cos(), rad.sin());
                        let dst_w = ((bounds.width() * sx).ceil() as i32).saturating_add(4);
                        let dst_h = ((bounds.height() * sy).ceil() as i32).saturating_add(4);
                        let cx_dst = dest_x + dst_w as f32 * 0.5;
                        let cy_dst = dest_y + dst_h as f32 * 0.5;
                        let visible_x = visible_glyph_axis_range(cx_dst, dst_w, buf_w);
                        let visible_y = visible_glyph_axis_range(cy_dst, dst_h, buf_h);
                        for dy in visible_y {
                            for dx in visible_x.clone() {
                                let rx = dx as f32 - dst_w as f32 * 0.5;
                                let ry = dy as f32 - dst_h as f32 * 0.5;
                                let ux = (rx * cos_r + ry * sin_r) / sx + bounds.width() * 0.5;
                                let uy = (-rx * sin_r + ry * cos_r) / sy + bounds.height() * 0.5;
                                let sxi = ux.floor() as isize;
                                let syi = uy.floor() as isize;
                                if sxi < 0 || syi < 0 || sxi >= bw as isize || syi >= bh as isize {
                                    continue;
                                }
                                let sa = cov[(syi as usize) * bw + (sxi as usize)];
                                if sa < 0.005 {
                                    continue;
                                }
                                let pxi = (cx_dst + rx).round() as isize;
                                let pyi = (cy_dst + ry).round() as isize;
                                if pxi < 0
                                    || pyi < 0
                                    || pxi >= buf_w as isize
                                    || pyi >= buf_h as isize
                                {
                                    continue;
                                }
                                let out_a = (sa * 255.0 * c.opacity_multiplier * color_alpha)
                                    .round()
                                    .clamp(0.0, 255.0)
                                    as u8;
                                if out_a == 0 {
                                    continue;
                                }
                                let idx = ((pyi as u32 * buf_w + pxi as u32) * 4) as usize;
                                pixels[idx] = r;
                                pixels[idx + 1] = g;
                                pixels[idx + 2] = b;
                                pixels[idx + 3] = out_a;
                            }
                        }
                    }
                }
                cursor_x += h_advance
                    + kerning
                    + tracking
                    + c.tracking_offset
                    + if ch == ' ' { justify_space } else { 0.0 };
            } else {
                cursor_x +=
                    h_advance + kerning + tracking + if ch == ' ' { justify_space } else { 0.0 };
            }
            char_idx += 1;
        }
    }

    Some((buf_w, buf_h, pixels))
}
