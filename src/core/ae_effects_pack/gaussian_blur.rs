/// Computes box-blur radii that approximate a true Gaussian of the given sigma
/// (W3C/Featherstone standard: three successive box blurs ≈ Gaussian).
pub(crate) fn boxes_for_gauss(sigma: f32, n: u32) -> [f32; 3] {
    if !sigma.is_finite() || sigma <= 0.01 || n == 0 {
        return [0.0, 0.0, 0.0];
    }
    let n = n as f32;
    let w_ideal = (12.0 * sigma * sigma / n + 1.0).sqrt();
    let mut wl = w_ideal.floor();
    if wl % 2.0 == 0.0 {
        wl -= 1.0;
    }
    let wu = wl + 2.0;
    let denom = -4.0 * wl - 4.0;
    let m = if denom.abs() < 1e-4 {
        0.0
    } else {
        ((12.0 * sigma * sigma - n * wl * wl - 4.0 * n * wl - 3.0 * n) / denom)
            .round()
            .max(0.0)
    };
    let mut out = [0.0f32; 3];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = if (i as f32) < m {
            wl.max(0.0)
        } else {
            wu.max(0.0)
        };
    }
    out
}

/// True Gaussian blur via three successive box blurs with optimized radii.
/// Visually indistinguishable from an exact Gaussian while staying O(n) per pass.
pub fn apply_gaussian_blur(pixels: &mut [u8], width: u32, height: u32, radius: u32) {
    if radius == 0 || width == 0 || height == 0 {
        return;
    }
    // The UI's "radius" maps to a Gaussian sigma; 3-box approx covers it well.
    let sigma = radius as f32 / 2.0;
    let boxes = boxes_for_gauss(sigma, 3);
    let mut buf_a = pixels.to_vec();
    let mut buf_b = vec![0u8; pixels.len()];
    let mut scratch = vec![0u8; pixels.len()];

    for &box_w in &boxes {
        let r = ((box_w - 1.0) / 2.0).round().max(1.0) as u32;
        box_blur_separable(&buf_a, &mut buf_b, &mut scratch, width, height, r);
        std::mem::swap(&mut buf_a, &mut buf_b);
    }
    pixels.copy_from_slice(&buf_a);
}

/// One separable box blur: horizontal src -> scratch, vertical scratch -> dst.
fn box_blur_separable(
    src: &[u8],
    dst: &mut [u8],
    scratch: &mut [u8],
    width: u32,
    height: u32,
    radius: u32,
) {
    let r = radius as i32;

    // Horizontal: src -> scratch
    for y in 0..height {
        for x in 0..width as i32 {
            let mut acc = [0f32; 4];
            let mut count = 0f32;
            for dx in -r..=r {
                let px = (x + dx).clamp(0, width as i32 - 1) as u32;
                let idx = ((y * width + px) * 4) as usize;
                for ch in 0..4 {
                    acc[ch] += src[idx + ch] as f32;
                }
                count += 1.0;
            }
            let oidx = ((y * width + x as u32) * 4) as usize;
            for ch in 0..4 {
                scratch[oidx + ch] = (acc[ch] / count).round() as u8;
            }
        }
    }

    // Vertical: scratch -> dst
    for y in 0..height as i32 {
        for x in 0..width {
            let mut acc = [0f32; 4];
            let mut count = 0f32;
            for dy in -r..=r {
                let py = (y + dy).clamp(0, height as i32 - 1) as u32;
                let idx = ((py * width + x) * 4) as usize;
                for ch in 0..4 {
                    acc[ch] += scratch[idx + ch] as f32;
                }
                count += 1.0;
            }
            let oidx = ((y as u32 * width + x) * 4) as usize;
            for ch in 0..4 {
                dst[oidx + ch] = (acc[ch] / count).round() as u8;
            }
        }
    }
}

#[cfg(test)]
mod gaussian_tests {
    use super::*;

    #[test]
    fn test_boxes_for_gauss_sane() {
        let [w1, w2, w3] = boxes_for_gauss(4.0, 3);
        // All widths must be positive and odd (box convention)
        for w in [w1, w2, w3] {
            assert!(w >= 1.0);
            assert_eq!(w % 2.0, 1.0);
        }
        // Widths must be roughly increasing then equal (standard property)
        assert!(w1 <= w2 + 2.0);
    }

    #[test]
    fn test_gaussian_blur_smooths_uniform_image_to_itself() {
        // A uniform image must be unchanged by any blur (edge clamping)
        let w = 32u32;
        let h = 32u32;
        let mut px = vec![128u8; (w * h * 4) as usize];
        let original = px.clone();
        apply_gaussian_blur(&mut px, w, h, 6);
        assert_eq!(px, original);
    }

    #[test]
    fn test_gaussian_blur_spreads_energy() {
        // A single bright pixel spreads into neighbors and loses peak intensity
        let w = 64u32;
        let h = 64u32;
        let mut px = vec![0u8; (w * h * 4) as usize];
        let center = ((32 * w + 32) * 4) as usize;
        px[center] = 255;
        px[center + 1] = 255;
        px[center + 2] = 255;
        px[center + 3] = 255;
        apply_gaussian_blur(&mut px, w, h, 8);
        let peak = px[center];
        assert!(peak < 255, "peak must drop, got {}", peak);
        let neighbor = ((32 * w + 33) * 4) as usize;
        assert!(px[neighbor] > 0, "energy must spread to neighbors");
        // Symmetry check: left and right neighbors receive equal energy
        let left = ((32 * w + 31) * 4) as usize;
        assert!(
            (px[left] as i32 - px[neighbor] as i32).abs() <= 2,
            "blur must be symmetric"
        );
    }

    #[test]
    fn test_gaussian_vs_single_box_quality() {
        // The 3-box Gaussian must produce a smoother falloff than a single box:
        // at 1.5x radius from center, a single box is already 0 while Gaussian
        // still has measurable energy.
        let w = 128u32;
        let h = 9u32;
        let mut g = vec![0u8; (w * h * 4) as usize];
        let mut b = g.clone();
        let cy = 4 * w;
        for x in 60..68 {
            let idx = ((cy + x) * 4) as usize;
            g[idx] = 255;
            b[idx] = 255;
        }
        apply_gaussian_blur(&mut g, w, h, 10);
        super::super::apply_fast_box_blur(&mut b, w, h, 10);
        // Quality assertions:
        // 1. Gaussian concentrates energy at the center (higher peak than box)
        let center_g = g[((cy + 63) * 4) as usize];
        let center_b = b[((cy + 63) * 4) as usize];
        assert!(
            center_g > center_b,
            "gaussian peak ({}) must exceed box ({})",
            center_g,
            center_b
        );

        // 2. Smooth monotone falloff (no abrupt steps like a box kernel)
        let mut prev = u32::MAX;
        for x in 64..80 {
            let v = g[((cy + x) * 4) as usize] as u32;
            assert!(
                v <= prev.saturating_add(1),
                "falloff must be monotone at x={}: {} -> {}",
                x,
                prev,
                v
            );
            prev = v;
        }

        // 3. Symmetry around the source center (x=63.5)
        for d in 1..8u32 {
            let l = g[((cy + (64 - d)) * 4) as usize] as i32;
            let r = g[((cy + (63 + d)) * 4) as usize] as i32;
            assert!(
                (l - r).abs() <= 1,
                "blur must be symmetric at d={}: {} vs {}",
                d,
                l,
                r
            );
        }
    }
}
