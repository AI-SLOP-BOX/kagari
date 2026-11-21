/// Rasterize one soft-circle dot with the emitter's blend mode.
pub fn draw_dot(
    buffer: &mut [u8],
    dims: (u32, u32),
    center: (f32, f32),
    half: f32,
    color: [f32; 3],
    a: f32,
    blend_mode: u32,
) {
    draw_dot_with_cancel(buffer, dims, center, half, color, a, blend_mode, None);
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_dot_with_cancel(
    buffer: &mut [u8],
    dims: (u32, u32),
    center: (f32, f32),
    half: f32,
    color: [f32; 3],
    a: f32,
    blend_mode: u32,
    cancel_flag: Option<&std::sync::Arc<std::sync::atomic::AtomicBool>>,
) {
    let [r, g, b] = color;
    let buf_width = dims.0;
    let buf_height = dims.1;
    let cx = center.0;
    let cy = center.1;
    let Some(expected_len) = (buf_width as usize)
        .checked_mul(buf_height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
    else {
        return;
    };
    if buf_width == 0
        || buf_height == 0
        || buffer.len() < expected_len
        || half < 0.35
        || !half.is_finite()
        || !cx.is_finite()
        || !cy.is_finite()
        || !a.is_finite()
        || !color.iter().all(|value| value.is_finite())
        || a <= 0.001
    {
        return;
    }
    let x0 = ((cx - half).max(0.0)) as u32;
    let y0 = ((cy - half).max(0.0)) as u32;
    let x1 = ((cx + half).min(buf_width as f32 - 1.0)) as u32;
    let y1 = ((cy + half).min(buf_height as f32 - 1.0)) as u32;

    for py in y0..=y1 {
        if cancel_flag.is_some_and(|flag| flag.load(std::sync::atomic::Ordering::Relaxed)) {
            return;
        }
        for px in x0..=x1 {
            let dx = px as f32 - cx;
            let dy = py as f32 - cy;
            let dist = ((dx * dx + dy * dy).sqrt() / half).min(1.0);
            let falloff = (1.0 - dist * dist).max(0.0);
            let pixel_a = a * falloff;
            if pixel_a <= 0.001 {
                continue;
            }

            let idx = (py as usize * buf_width as usize + px as usize) * 4;

            let src_r = r * pixel_a;
            let src_g = g * pixel_a;
            let src_b = b * pixel_a;

            match blend_mode {
                1 => {
                    let dst_a = buffer[idx + 3] as f32 / 255.0;
                    let dr = buffer[idx] as f32 / 255.0 + src_r;
                    let dg = buffer[idx + 1] as f32 / 255.0 + src_g;
                    let db = buffer[idx + 2] as f32 / 255.0 + src_b;
                    let da = (dst_a + pixel_a).min(1.0);
                    buffer[idx] = (dr.min(1.0) * 255.0) as u8;
                    buffer[idx + 1] = (dg.min(1.0) * 255.0) as u8;
                    buffer[idx + 2] = (db.min(1.0) * 255.0) as u8;
                    buffer[idx + 3] = (da * 255.0) as u8;
                }
                2 => {
                    let dst_r = buffer[idx] as f32 / 255.0;
                    let dst_g = buffer[idx + 1] as f32 / 255.0;
                    let dst_b = buffer[idx + 2] as f32 / 255.0;
                    let dst_a = buffer[idx + 3] as f32 / 255.0;
                    let sa = pixel_a;
                    let sr = if sa > 0.001 { r } else { 0.0 };
                    let sg = if sa > 0.001 { g } else { 0.0 };
                    let sb = if sa > 0.001 { b } else { 0.0 };
                    let out_a = sa + dst_a * (1.0 - sa);
                    if out_a > 0.001 {
                        let out_r =
                            (sr * sa + (1.0 - sa) * dst_r * dst_a + sa * (1.0 - dst_a) * sr)
                                / out_a;
                        let out_g =
                            (sg * sa + (1.0 - sa) * dst_g * dst_a + sa * (1.0 - dst_a) * sg)
                                / out_a;
                        let out_b =
                            (sb * sa + (1.0 - sa) * dst_b * dst_a + sa * (1.0 - dst_a) * sb)
                                / out_a;
                        buffer[idx] = (out_r.clamp(0.0, 1.0) * 255.0) as u8;
                        buffer[idx + 1] = (out_g.clamp(0.0, 1.0) * 255.0) as u8;
                        buffer[idx + 2] = (out_b.clamp(0.0, 1.0) * 255.0) as u8;
                        buffer[idx + 3] = (out_a.clamp(0.0, 1.0) * 255.0) as u8;
                    }
                }
                _ => {
                    let dst_a = buffer[idx + 3] as f32 / 255.0;
                    let out_a = pixel_a + dst_a * (1.0 - pixel_a);
                    if out_a > 0.001 {
                        let out_r =
                            (src_r + buffer[idx] as f32 / 255.0 * dst_a * (1.0 - pixel_a)) / out_a;
                        let out_g = (src_g
                            + buffer[idx + 1] as f32 / 255.0 * dst_a * (1.0 - pixel_a))
                            / out_a;
                        let out_b = (src_b
                            + buffer[idx + 2] as f32 / 255.0 * dst_a * (1.0 - pixel_a))
                            / out_a;
                        buffer[idx] = (out_r.clamp(0.0, 1.0) * 255.0) as u8;
                        buffer[idx + 1] = (out_g.clamp(0.0, 1.0) * 255.0) as u8;
                        buffer[idx + 2] = (out_b.clamp(0.0, 1.0) * 255.0) as u8;
                        buffer[idx + 3] = (out_a.clamp(0.0, 1.0) * 255.0) as u8;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::draw_dot_with_cancel;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    #[test]
    fn cancelled_dot_draw_leaves_pixels_untouched() {
        let mut pixels = vec![23u8; 16 * 16 * 4];
        let original = pixels.clone();
        let flag = Arc::new(AtomicBool::new(true));

        draw_dot_with_cancel(
            &mut pixels,
            (16, 16),
            (8.0, 8.0),
            6.0,
            [1.0; 3],
            1.0,
            0,
            Some(&flag),
        );

        assert_eq!(pixels, original);
    }
}
