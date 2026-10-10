#[inline]
fn sample_bilinear(src: &[u8], w: u32, h: u32, fx: f32, fy: f32, out: &mut [u8; 4]) {
    crate::core::effect_utils::sample_bilinear_into(src, w, h, fx, fy, out);
}

/// Parameters for [`apply_card_wipe`].
#[derive(Debug, Clone, Copy)]
pub struct CardWipeParams {
    /// Transition progress (0.0 = unaffected, 1.0 = completely flipped/transparent).
    pub progress: f32,
    /// Number of horizontal grid columns.
    pub cols: u32,
    /// Number of vertical grid rows.
    pub rows: u32,
}

impl Default for CardWipeParams {
    fn default() -> Self {
        Self {
            progress: 0.0,
            cols: 8,
            rows: 6,
        }
    }
}

/// Grid-based rotating 3D card wipe transition.
pub fn apply_card_wipe(pixels: &mut [u8], width: u32, height: u32, p: &CardWipeParams) {
    if width == 0
        || height == 0
        || p.progress <= 0.0
        || pixels.len() < (width as usize) * (height as usize) * 4
    {
        return;
    }
    let src = pixels.to_vec();
    pixels.fill(0); // clear to black / transparent

    let cols = p.cols.max(1);
    let rows = p.rows.max(1);
    let card_w = width as f32 / cols as f32;
    let card_h = height as f32 / rows as f32;

    let mut out_px = [0u8; 4];
    for r in 0..rows {
        for c in 0..cols {
            // Stagger animation across cards (diagonal wave)
            let card_norm = (c as f32 / cols as f32 + r as f32 / rows as f32) * 0.5;
            let card_prog = ((p.progress - card_norm * 0.5) / 0.5).clamp(0.0, 1.0);
            if card_prog >= 1.0 {
                // Card has fully flipped out
                continue;
            }

            // Flip angle 0 -> 90 degrees
            let angle = card_prog * std::f32::consts::FRAC_PI_2;
            let cos_a = angle.cos().max(0.01);

            let card_cx = (c as f32 + 0.5) * card_w;
            let _card_cy = (r as f32 + 0.5) * card_h;

            let y_start = (r as f32 * card_h).floor() as u32;
            let y_end = (((r + 1) as f32 * card_h).ceil() as u32).min(height);

            for y in y_start..y_end {
                let x_start = (card_cx - (card_w * 0.5 * cos_a)).floor().max(0.0) as u32;
                let x_end = (card_cx + (card_w * 0.5 * cos_a)).ceil().min(width as f32) as u32;

                for x in x_start..x_end {
                    let rel_x = (x as f32 - card_cx) / cos_a;
                    let sx = card_cx + rel_x;
                    let sy = y as f32;

                    sample_bilinear(&src, width, height, sx, sy, &mut out_px);
                    // Slight shading as card rotates
                    let shade = cos_a.powf(0.5);
                    let idx = ((y * width + x) * 4) as usize;
                    for ch in 0..3 {
                        pixels[idx + ch] = (out_px[ch] as f32 * shade).round() as u8;
                    }
                    pixels[idx + 3] = out_px[3];
                }
            }
        }
    }
}
