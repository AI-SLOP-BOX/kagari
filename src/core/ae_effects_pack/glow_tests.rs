use super::*;

#[test]
fn test_glow_adds_energy_near_bright_areas() {
    let w = 64u32;
    let h = 64u32;
    let mut px = vec![0u8; (w * h * 4) as usize];
    // Bright white square in the center
    for y in 28..36 {
        for x in 28..36 {
            let idx = ((y * w + x) * 4) as usize;
            px[idx] = 255;
            px[idx + 1] = 255;
            px[idx + 2] = 255;
            px[idx + 3] = 255;
        }
    }
    let before = px.clone();
    apply_glow(&mut px, w, h, 0.5, 8, 50.0);

    // A pixel just outside the square must receive bloom energy
    let outside = ((30 * w + 40) * 4) as usize;
    assert!(
        px[outside] > before[outside],
        "glow must add energy outside bright regions, got {}",
        px[outside]
    );
    // Dark corners far away stay dark
    let corner = ((2 * w + 2) * 4) as usize;
    assert!(
        px[corner] < 20,
        "far dark areas must stay dark, got {}",
        px[corner]
    );
}

#[test]
fn test_soft_threshold_no_hard_band() {
    // With a hard threshold, a gradient produces a binary mask with a sharp
    // edge; soft thresholding must produce a gradual ramp.
    let w = 64u32;
    let h = 8u32;
    let mut px = vec![0u8; (w * h * 4) as usize];
    for x in 0..w {
        let v = ((x as f32 / w as f32) * 255.0) as u8;
        for y in 0..h {
            let idx = ((y * w + x) * 4) as usize;
            px[idx] = v;
            px[idx + 1] = v;
            px[idx + 2] = v;
            px[idx + 3] = 255;
        }
    }
    // Dark gradient (max 60/255) with threshold 0.15 and modest intensity:
    // nothing saturates, so any hard banding in the mask would show as a
    // large single-step jump across the threshold boundary.
    for i in (0..px.len()).step_by(4) {
        let src_x = (i / 4 % w as usize) as f32 / w as f32;
        let v = (src_x * 60.0) as u8;
        px[i] = v;
        px[i + 1] = v;
        px[i + 2] = v;
    }
    let mut out = px.clone();
    apply_glow(&mut out, w, h, 0.15, 6, 30.0);
    let row = 4 * w;
    let mut max_delta = 0i32;
    for x in 1..w {
        let d =
            (out[((row + x) * 4) as usize] as i32 - out[((row + x - 1) * 4) as usize] as i32).abs();
        max_delta = max_delta.max(d);
    }
    // With blur radius 6 the steepest step should be gentle (< 60/px)
    assert!(
        max_delta < 60,
        "soft threshold should avoid hard banding, max step was {}",
        max_delta
    );
}
