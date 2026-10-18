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
    apply_fast_box_blur(&mut b, w, h, 10);
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
