use super::*;

#[test]
fn test_directional_blur_spreads_along_angle_only() {
    // A single bright pixel smears along the direction vector; the
    // perpendicular neighbor must stay dark.
    let w = 64u32;
    let h = 64u32;
    let mut px = vec![0u8; (w * h * 4) as usize];
    let c = ((32 * w + 32) * 4) as usize;
    px[c] = 255;
    px[c + 1] = 255;
    px[c + 2] = 255;
    px[c + 3] = 255;
    apply_directional_blur(&mut px, w, h, 90.0, 20.0);
    // 90deg => horizontal smear
    let right = ((32 * w + 38) * 4) as usize;
    let below = ((36 * w + 32) * 4) as usize;
    assert!(px[right] > 0, "energy must spread horizontally");
    assert!(
        px[below] < 10,
        "perpendicular axis must stay dark, got {}",
        px[below]
    );
}

#[test]
fn test_directional_blur_zero_length_is_noop() {
    let mut px = vec![128u8; 16 * 16 * 4];
    let orig = px.clone();
    apply_directional_blur(&mut px, 16, 16, 45.0, 0.0);
    assert_eq!(px, orig);
}

#[test]
fn test_radial_blur_smears_angularly() {
    // An off-center bright pixel spreads angularly around center.
    let w = 64u32;
    let h = 64u32;
    let mut px = vec![0u8; (w * h * 4) as usize];
    let p = ((32 * w + 44) * 4) as usize;
    px[p] = 255;
    px[p + 1] = 255;
    px[p + 2] = 255;
    px[p + 3] = 255;
    apply_radial_blur(&mut px, w, h, 100.0);
    // Energy must spread to multiple pixels around the original radius
    let nonzero = px.as_chunks::<4>().0.iter().filter(|p| p[0] > 0).count();
    assert!(
        nonzero > 1,
        "radial blur must spread angularly, got {} lit pixels",
        nonzero
    );
    let peak = px.as_chunks::<4>().0.iter().map(|p| p[0]).max().unwrap();
    assert!(peak < 255, "peak must drop as energy spreads, got {}", peak);
}

#[test]
fn test_radial_blur_uniform_image_is_stable() {
    // A uniform image has no angular structure: blur must be a no-op.
    let w = 33u32;
    let h = 33u32;
    let mut px = vec![200u8; (w * h * 4) as usize];
    for i in (3..px.len()).step_by(4) {
        px[i] = 255;
    }
    let orig = px.clone();
    apply_radial_blur(&mut px, w, h, 50.0);
    assert_eq!(px, orig);
}
