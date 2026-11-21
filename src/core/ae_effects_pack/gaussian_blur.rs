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
    let Some(expected_len) = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
    else {
        return;
    };
    if radius == 0 || width == 0 || height == 0 || pixels.len() != expected_len {
        return;
    }
    let cancel_flag = crate::core::software_renderer::get_render_cancel_flag();
    let is_cancelled = || {
        cancel_flag
            .as_ref()
            .is_some_and(|flag| flag.load(std::sync::atomic::Ordering::Relaxed))
    };
    if is_cancelled() {
        return;
    }

    // The input buffer is one side of the ping-pong pair. This needs only two
    // additional full-frame buffers (temporary output + horizontal scratch),
    // instead of cloning the input and allocating two more full buffers.
    // Effects in Kagari expose blur controls up to 1000 px. Clamp the public
    // helper too, avoiding integer overflow and unbounded work for bad callers.
    let radius = radius.min(1000);
    let sigma = radius as f32 / 2.0;
    let boxes = boxes_for_gauss(sigma, 3);
    let mut temp = vec![0u8; pixels.len()];
    let mut scratch = vec![0u8; pixels.len()];
    let mut source_is_pixels = true;

    for &box_w in &boxes {
        let r = ((box_w - 1.0) / 2.0).round().max(1.0) as u32;
        let completed = if source_is_pixels {
            box_blur_separable(
                pixels,
                &mut temp,
                &mut scratch,
                width,
                height,
                r,
                cancel_flag.as_ref(),
            )
        } else {
            box_blur_separable(
                &temp,
                pixels,
                &mut scratch,
                width,
                height,
                r,
                cancel_flag.as_ref(),
            )
        };
        if !completed {
            return;
        }
        source_is_pixels = !source_is_pixels;
    }
    if is_cancelled() {
        return;
    }
    if !source_is_pixels {
        pixels.copy_from_slice(&temp);
    }
}

/// One separable box blur: horizontal src -> scratch, vertical scratch -> dst.
/// Returns false when the render cancellation flag is set.
fn box_blur_separable(
    src: &[u8],
    dst: &mut [u8],
    scratch: &mut [u8],
    width: u32,
    height: u32,
    radius: u32,
    cancel_flag: Option<&std::sync::Arc<std::sync::atomic::AtomicBool>>,
) -> bool {
    let r = i64::from(radius.min(1000));
    let count = (2 * r + 1) as f32;
    let cancelled =
        || cancel_flag.is_some_and(|flag| flag.load(std::sync::atomic::Ordering::Relaxed));

    // Horizontal: src -> scratch. Maintain a rolling channel sum so each
    // pixel costs O(1), independent of blur radius.
    for y in 0..height {
        if cancelled() {
            return false;
        }
        let mut sum = [0u64; 4];
        for dx in -r..=r {
            let px = dx.clamp(0, i64::from(width) - 1) as u32;
            let idx = ((y * width + px) * 4) as usize;
            for ch in 0..4 {
                sum[ch] += u64::from(src[idx + ch]);
            }
        }
        for x in 0..width {
            let oidx = ((y * width + x) * 4) as usize;
            for ch in 0..4 {
                scratch[oidx + ch] = (sum[ch] as f32 / count).round() as u8;
            }
            if x + 1 < width {
                let outgoing_x = (i64::from(x) - r).clamp(0, i64::from(width) - 1) as u32;
                let incoming_x = (i64::from(x) + r + 1).clamp(0, i64::from(width) - 1) as u32;
                let outgoing_idx = ((y * width + outgoing_x) * 4) as usize;
                let incoming_idx = ((y * width + incoming_x) * 4) as usize;
                for ch in 0..4 {
                    sum[ch] -= u64::from(src[outgoing_idx + ch]);
                    sum[ch] += u64::from(src[incoming_idx + ch]);
                }
            }
        }
    }

    // Vertical: scratch -> dst, using the same rolling-window update.
    for x in 0..width {
        if cancelled() {
            return false;
        }
        let mut sum = [0u64; 4];
        for dy in -r..=r {
            let py = dy.clamp(0, i64::from(height) - 1) as u32;
            let idx = ((py * width + x) * 4) as usize;
            for ch in 0..4 {
                sum[ch] += u64::from(scratch[idx + ch]);
            }
        }
        for y in 0..height {
            if y % 64 == 0 && cancelled() {
                return false;
            }
            let oidx = ((y * width + x) * 4) as usize;
            for ch in 0..4 {
                dst[oidx + ch] = (sum[ch] as f32 / count).round() as u8;
            }
            if y + 1 < height {
                let outgoing_y = (i64::from(y) - r).clamp(0, i64::from(height) - 1) as u32;
                let incoming_y = (i64::from(y) + r + 1).clamp(0, i64::from(height) - 1) as u32;
                let outgoing_idx = ((outgoing_y * width + x) * 4) as usize;
                let incoming_idx = ((incoming_y * width + x) * 4) as usize;
                for ch in 0..4 {
                    sum[ch] -= u64::from(scratch[outgoing_idx + ch]);
                    sum[ch] += u64::from(scratch[incoming_idx + ch]);
                }
            }
        }
    }
    true
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
    fn cancelled_blur_leaves_input_untouched() {
        let mut pixels = vec![127u8; 32 * 32 * 4];
        let original = pixels.clone();
        let flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));

        crate::core::software_renderer::with_render_cancel_flag(flag, || {
            apply_gaussian_blur(&mut pixels, 32, 32, 8);
        });

        assert_eq!(pixels, original);
    }

    #[test]
    fn rolling_box_blur_matches_the_naive_reference() {
        for (width, height, radius) in [(1u32, 1u32, 3u32), (5, 3, 1), (5, 3, 4)] {
            let mut source = (0..width * height * 4)
                .map(|index| (index * 37 % 256) as u8)
                .collect::<Vec<_>>();
            let mut expected = vec![0u8; source.len()];
            let mut scratch = vec![0u8; source.len()];
            naive_box_blur(&source, &mut expected, &mut scratch, width, height, radius);
            box_blur_separable(
                &source.clone(),
                &mut source,
                &mut scratch,
                width,
                height,
                radius,
                None,
            );
            assert_eq!(source, expected, "{width}x{height} radius {radius}");
        }
    }

    fn naive_box_blur(
        src: &[u8],
        dst: &mut [u8],
        scratch: &mut [u8],
        width: u32,
        height: u32,
        radius: u32,
    ) {
        let radius = radius as i32;
        for y in 0..height {
            for x in 0..width as i32 {
                let mut sum = [0.0f32; 4];
                let mut count = 0.0f32;
                for dx in -radius..=radius {
                    let px = (x + dx).clamp(0, width as i32 - 1) as u32;
                    let idx = ((y * width + px) * 4) as usize;
                    for ch in 0..4 {
                        sum[ch] += src[idx + ch] as f32;
                    }
                    count += 1.0;
                }
                let idx = ((y * width + x as u32) * 4) as usize;
                for ch in 0..4 {
                    scratch[idx + ch] = (sum[ch] / count).round() as u8;
                }
            }
        }
        for y in 0..height as i32 {
            for x in 0..width {
                let mut sum = [0.0f32; 4];
                let mut count = 0.0f32;
                for dy in -radius..=radius {
                    let py = (y + dy).clamp(0, height as i32 - 1) as u32;
                    let idx = ((py * width + x) * 4) as usize;
                    for ch in 0..4 {
                        sum[ch] += scratch[idx + ch] as f32;
                    }
                    count += 1.0;
                }
                let idx = ((y as u32 * width + x) * 4) as usize;
                for ch in 0..4 {
                    dst[idx + ch] = (sum[ch] / count).round() as u8;
                }
            }
        }
    }

    #[test]
    fn malformed_dimensions_leave_blur_input_untouched() {
        let mut pixels = vec![73u8; 12];
        apply_gaussian_blur(&mut pixels, 4, 4, 8);
        assert_eq!(pixels, vec![73u8; 12]);
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
