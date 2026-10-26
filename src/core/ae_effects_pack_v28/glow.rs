/// Threshold bloom: pixels above `threshold` luminance bleed outwards with a
/// separable box blur (two O(n) passes) scaled by `intensity`.
pub fn apply_glow_pro(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    threshold: f32,
    radius: u32,
    intensity: f32,
) {
    if width < 2 || height < 2 || pixels.len() < (width as usize) * (height as usize) * 4 {
        return;
    }
    let thr = threshold.clamp(0.0, 1.0);
    let inten = intensity.clamp(0.0, 4.0);
    if inten <= 0.001 {
        return;
    }
    let r = radius.min(128) as usize;

    // Bright pass into f32 planes.
    let n = (width as usize) * (height as usize);
    let mut plane = vec![0.0f32; n * 3];
    for i in 0..n {
        let idx = i * 4;
        let luma = (pixels[idx] as f32 * 0.2126
            + pixels[idx + 1] as f32 * 0.7152
            + pixels[idx + 2] as f32 * 0.0722)
            / 255.0;
        if luma > thr {
            let k = ((luma - thr) / (1.0 - thr).max(1e-3)).min(1.0);
            for c in 0..3 {
                plane[i * 3 + c] = pixels[idx + c] as f32 * k;
            }
        }
    }

    // Separable box blur (horizontal then vertical), sliding window.
    if r > 0 {
        box_blur_h(&mut plane, width as usize, height as usize, r);
        box_blur_v(&mut plane, width as usize, height as usize, r);
    }

    // Screen-composite the bloom back over the source.
    for i in 0..n {
        let idx = i * 4;
        for c in 0..3 {
            let base = pixels[idx + c] as f32 / 255.0;
            let bloom = (plane[i * 3 + c] / 255.0) * inten;
            let screened = 1.0 - (1.0 - base) * (1.0 - bloom);
            pixels[idx + c] = (screened.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }
}

fn box_blur_h(plane: &mut [f32], w: usize, h: usize, r: usize) {
    let stride = w * 3;
    let mut row = vec![0.0f32; stride];
    for y in 0..h {
        let base = y * stride;
        row.copy_from_slice(&plane[base..base + stride]);
        for x in 0..w {
            let lo = x.saturating_sub(r);
            let hi = (x + r).min(w - 1);
            let count = ((hi - lo + 1) * 3) as f32;
            for c in 0..3 {
                let mut sum = 0.0f32;
                for xx in lo..=hi {
                    sum += row[xx * 3 + c];
                }
                plane[base + x * 3 + c] = sum / count;
            }
        }
    }
}

fn box_blur_v(plane: &mut [f32], w: usize, h: usize, r: usize) {
    let stride = w * 3;
    let mut col = vec![0.0f32; h * 3];
    for x in 0..w {
        for y in 0..h {
            let b = y * stride + x * 3;
            col[y * 3..y * 3 + 3].copy_from_slice(&plane[b..b + 3]);
        }
        for y in 0..h {
            let lo = y.saturating_sub(r);
            let hi = (y + r).min(h - 1);
            let count = (hi - lo + 1) as f32;
            for c in 0..3 {
                let mut sum = 0.0f32;
                for yy in lo..=hi {
                    sum += col[yy * 3 + c];
                }
                plane[y * stride + x * 3 + c] = sum / count;
            }
        }
    }
}
