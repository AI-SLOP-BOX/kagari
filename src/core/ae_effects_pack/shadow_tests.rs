use super::*;

fn solid_square(w: u32, h: u32, x0: usize, x1: usize, y0: usize, y1: usize) -> Vec<u8> {
    let mut px = vec![0u8; (w * h * 4) as usize];
    for y in y0..y1 {
        for x in x0..x1 {
            let idx = (y * w as usize + x) * 4;
            px[idx] = 255;
            px[idx + 1] = 255;
            px[idx + 2] = 255;
            px[idx + 3] = 255;
        }
    }
    px
}

#[test]
fn test_drop_shadow_places_shadow_beyond_shape() {
    // Square on the left half; shadow offset to the right must appear
    // in the empty right region.
    let w = 64u32;
    let h = 64u32;
    let mut px = solid_square(w, h, 8, 24, 24, 40);
    apply_drop_shadow(&mut px, w, h, 16.0, 90.0, 4, [0, 0, 0, 255]);
    let right = ((32 * w + 30) * 4) as usize;
    assert!(
        px[right + 3] > 0,
        "shadow alpha must extend past shape edge"
    );
    assert_eq!(px[right], 0, "shadow must be black");
}

#[test]
fn test_drop_shadow_soft_edges_are_smooth() {
    // With Gaussian softness, the shadow falloff across the edge must be
    // monotone and gradual (no hard box-blur steps).
    let w = 96u32;
    let h = 64u32;
    let mut px = solid_square(w, h, 8, 24, 24, 40);
    apply_drop_shadow(&mut px, w, h, 20.0, 90.0, 10, [0, 0, 0, 255]);
    // Sample a horizontal run of shadow-only pixels right of the shape.
    // The profile must rise then fall smoothly (no hard box-blur steps).
    let row = 32usize;
    let vals: Vec<i32> = (28..90)
        .map(|x| px[(row * w as usize + x) * 4 + 3] as i32)
        .collect();
    let peak = vals.iter().enumerate().max_by_key(|(_, v)| **v).unwrap().0;
    assert!(
        peak > 0 && peak < vals.len() - 1,
        "shadow must have an interior peak"
    );
    let mut prev = i32::MAX;
    for (i, &v) in vals.iter().enumerate().skip(peak) {
        assert!(
            v <= prev.saturating_add(1),
            "shadow falloff must be monotone at x={}: {} -> {}",
            peak + i,
            prev,
            v
        );
        prev = v;
    }
}

#[test]
fn test_drop_shadow_preserves_foreground() {
    let w = 32u32;
    let h = 32u32;
    let mut px = solid_square(w, h, 12, 20, 12, 20);
    apply_drop_shadow(&mut px, w, h, 5.0, 45.0, 3, [0, 0, 0, 200]);
    // Foreground center stays fully opaque white
    let c = ((16 * w + 16) * 4) as usize;
    assert_eq!(px[c], 255);
    assert_eq!(px[c + 1], 255);
    assert_eq!(px[c + 2], 255);
    assert_eq!(px[c + 3], 255);
}
