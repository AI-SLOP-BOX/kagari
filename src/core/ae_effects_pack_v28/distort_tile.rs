#[inline]
fn sample_bilinear(src: &[u8], w: u32, h: u32, fx: f32, fy: f32, out: &mut [u8; 4]) {
    crate::core::effect_utils::sample_bilinear_into(src, w, h, fx, fy, out);
}

/// Bends the image horizontally: the top edge shifts by `top_offset` px and
/// the bottom edge by `bottom_offset` px, interpolated with a smooth ease so
/// the middle bows naturally (AE CC Bend It look).
pub fn apply_cc_bend_it_pro(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    top_offset: f32,
    bottom_offset: f32,
) {
    if width == 0 || height == 0 || pixels.len() < (width as usize) * (height as usize) * 4 {
        return;
    }
    let temp = pixels.to_vec();
    let fh = height as f32;

    for y in 0..height {
        let t = y as f32 / (fh - 1.0).max(1.0);
        // Smoothstep-eased interpolation between the two edge offsets.
        let eased = t * t * (3.0 - 2.0 * t);
        let shift = top_offset + (bottom_offset - top_offset) * eased;
        for x in 0..width {
            let mut rgba = [0u8; 4];
            sample_bilinear(&temp, width, height, x as f32 + shift, y as f32, &mut rgba);
            let idx = ((y * width + x) * 4) as usize;
            pixels[idx..idx + 4].copy_from_slice(&rgba);
        }
    }
}

/// Edge handling for [`apply_cc_tiler_pro`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileEdgeMode {
    Repeat,
    Mirror,
}

/// Tiles a scaled-down copy of the source across the canvas.
/// `scale_percent` > 100 shrinks the tile (more repeats); Mirror alternates
/// flipped copies so tile seams disappear.
pub fn apply_cc_tiler_pro(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    scale_percent: f32,
    edge_mode: TileEdgeMode,
) {
    if width == 0 || height == 0 || pixels.len() < (width as usize) * (height as usize) * 4 {
        return;
    }
    let factor = (100.0 / scale_percent.max(1.0)).clamp(0.02, 50.0); // source → tile scale
    let temp = pixels.to_vec();
    let fw = width as f32;
    let fh = height as f32;

    for y in 0..height {
        for x in 0..width {
            let mut u = x as f32 * factor / fw;
            let mut v = y as f32 * factor / fh;
            match edge_mode {
                TileEdgeMode::Repeat => {
                    u = u.rem_euclid(1.0);
                    v = v.rem_euclid(1.0);
                }
                TileEdgeMode::Mirror => {
                    u = mirror_fold(u);
                    v = mirror_fold(v);
                }
            }
            let mut rgba = [0u8; 4];
            sample_bilinear(&temp, width, height, u * fw, v * fh, &mut rgba);
            let idx = ((y * width + x) * 4) as usize;
            pixels[idx..idx + 4].copy_from_slice(&rgba);
        }
    }
}

/// Fold [0, ∞) into [0, 1] with alternating mirroring.
fn mirror_fold(t: f32) -> f32 {
    let m = t.rem_euclid(2.0);
    if m <= 1.0 {
        m
    } else {
        2.0 - m
    }
}
