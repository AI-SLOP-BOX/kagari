#[test]
fn test_camera_shake_is_deterministic() {
    let a1 = super::camera_shake(5.0, 10.0, 8.0, 42);
    let a2 = super::camera_shake(5.0, 10.0, 8.0, 42);
    assert_eq!(a1[0], a2[0]);
    assert_eq!(a1[1], a2[1]);
}

#[test]
fn test_camera_shake_varies_with_seed() {
    let a = super::camera_shake(5.0, 10.0, 8.0, 42);
    let b = super::camera_shake(5.0, 10.0, 8.0, 99);
    assert!(a != b, "different seeds must produce different shake");
}

#[test]
fn test_camera_shake_zero_intensity() {
    let a = super::camera_shake(5.0, 0.0, 8.0, 42);
    assert_eq!(a[0], 0.0);
    assert_eq!(a[1], 0.0);
}

#[test]
fn test_lens_flare_brightens_center() {
    let w = 64u32;
    let h = 64u32;
    let mut px = vec![10u8; (w * h * 4) as usize];
    super::apply_lens_flare(
        &mut px,
        w,
        h,
        &super::LensFlareParams {
            pos_x: 0.5,
            pos_y: 0.5,
            intensity: 1.0,
            threshold: 1.0,
            color: [255, 255, 255],
        },
    );
    let center = ((32 * w + 32) * 4) as usize;
    assert!(
        px[center] > 200,
        "flare core must brighten center, got {}",
        px[center]
    );
    // Far corner should stay near original
    let corner = ((2 * w + 2) * 4) as usize;
    assert!(
        px[corner] < 60,
        "corner must be barely affected, got {}",
        px[corner]
    );
}

#[test]
fn test_lens_flare_zero_intensity_noop() {
    let mut px = vec![40u8; 16 * 16 * 4];
    let before = px.clone();
    super::apply_lens_flare(
        &mut px,
        16,
        16,
        &super::LensFlareParams {
            pos_x: 0.5,
            pos_y: 0.5,
            intensity: 0.0,
            threshold: 1.0,
            color: [255, 255, 255],
        },
    );
    assert_eq!(px, before);
}

#[test]
fn test_lens_flare_clamps_to_white() {
    let w = 32u32;
    let h = 32u32;
    let mut px = vec![250u8; (w * h * 4) as usize];
    super::apply_lens_flare(
        &mut px,
        w,
        h,
        &super::LensFlareParams {
            pos_x: 0.5,
            pos_y: 0.5,
            intensity: 5.0,
            threshold: 2.0,
            color: [255, 255, 255],
        },
    );
    // Center region must clamp to pure white
    for y in 12..20u32 {
        for x in 12..20u32 {
            let i = ((y * w + x) * 4) as usize;
            assert_eq!(px[i], 255, "pixel ({x},{y}) must clamp to white");
        }
    }
}

#[test]
fn test_lens_flare_deterministic() {
    let run = || {
        let mut px = vec![0u8; 32 * 32 * 4];
        super::apply_lens_flare(
            &mut px,
            32,
            32,
            &super::LensFlareParams {
                pos_x: 0.3,
                pos_y: 0.7,
                intensity: 1.0,
                threshold: 1.0,
                color: [200, 150, 100],
            },
        );
        px
    };
    assert_eq!(run(), run());
}

#[test]
fn test_color_overlay_blends_opaque_pixels() {
    let mut px = vec![0u8; 16];
    // One opaque red pixel
    px[0] = 255;
    px[1] = 0;
    px[2] = 0;
    px[3] = 255;
    // One transparent pixel (must stay untouched)
    px[4] = 200;
    px[5] = 100;
    px[6] = 50;
    px[7] = 0;
    let before = px.clone();
    super::apply_color_overlay(&mut px, 2, 2, [0, 0, 255, 255], 50.0);
    assert!(
        px[0] >= 127 && px[0] <= 128,
        "red blended halfway toward blue, got {}",
        px[0]
    );
    assert!(px[2] >= 127 && px[2] <= 128);
    assert_eq!(&px[4..8], &before[4..8], "transparent pixel untouched");
}

#[test]
fn test_gradient_overlay_direction() {
    // 8x1 layer, gradient left→right (angle 0), white→black
    let mut px = vec![0u8; 8 * 4];
    for i in 0..8 {
        px[i * 4 + 3] = 255;
    }
    super::apply_gradient_overlay(
        &mut px,
        8,
        1,
        &super::GradientOverlayParams {
            angle_deg: 0.0,
            scale_pct: 100.0,
            start: [255, 255, 255, 255],
            end: [0, 0, 0, 255],
            opacity: 100.0,
        },
    );
    let left = px[0];
    let right = px[(7 * 4) as usize];
    assert!(left > 200, "left edge near start color, got {left}");
    assert!(right < 55, "right edge near end color, got {right}");
    assert!(left > right + 100, "strong horizontal ramp expected");
}

#[test]
fn test_gradient_overlay_zero_opacity_noop() {
    let mut px = vec![80u8; 64];
    let before = px.clone();
    super::apply_gradient_overlay(
        &mut px,
        4,
        4,
        &super::GradientOverlayParams {
            angle_deg: 90.0,
            scale_pct: 100.0,
            start: [255, 255, 255, 255],
            end: [0, 0, 0, 255],
            opacity: 0.0,
        },
    );
    assert_eq!(px, before);
}

#[test]
fn test_inner_shadow_darkens_offset_edge() {
    // Opaque rectangle x∈[6..26) inside a 32x8 buffer; angle 90°
    // (light from left) must darken only the LEFT inner band.
    let (w, h) = (32u32, 8u32);
    let mut px = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 6..26u32 {
            let i = ((y * w + x) * 4) as usize;
            px[i] = 200;
            px[i + 1] = 200;
            px[i + 2] = 200;
            px[i + 3] = 255;
        }
    }
    super::apply_inner_shadow(&mut px, w, h, 6.0, 90.0, 1, [0, 0, 0, 255]);
    let left = px[(2 * w as usize + 7) * 4];
    let right = px[(2 * w as usize + 22) * 4];
    assert!(
        left < right,
        "left band must darken: left={left} right={right}"
    );
    assert!(right >= 195, "far side stays bright: {right}");
}

#[test]
fn test_inner_glow_brightens_edges_not_center() {
    // Opaque rectangle with transparent margins so alpha edges exist
    let (w, h) = (16u32, 16u32);
    let mut px = vec![0u8; (w * h * 4) as usize];
    for y in 3..13usize {
        for x in 3..13usize {
            let i = (y * w as usize + x) * 4;
            px[i] = 40;
            px[i + 1] = 40;
            px[i + 2] = 40;
            px[i + 3] = 255;
        }
    }
    super::apply_inner_glow(&mut px, w, h, 2, [255, 255, 0, 255], 100.0);
    let edge = (4 * w as usize + 4) * 4;
    let center = (8 * w as usize + 8) * 4;
    assert!(px[edge] > 60, "inner edge must glow, got {}", px[edge]);
    assert_eq!(px[center], 40, "deep interior untouched");
}

fn opaque_rect(w: u32, h: u32, x0: u32, x1: u32, y0: u32, y1: u32, gray: u8) -> Vec<u8> {
    let mut px = vec![0u8; (w * h * 4) as usize];
    for y in y0..y1 {
        for x in x0..x1 {
            let i = ((y * w + x) * 4) as usize;
            px[i] = gray;
            px[i + 1] = gray;
            px[i + 2] = gray;
            px[i + 3] = 255;
        }
    }
    px
}

#[test]
fn test_satin_bands_one_side() {
    // Wide rect; angle 90° (dx=+distance): sheen band near LEFT interior.
    let (w, h) = (48u32, 16u32);
    let mut px = opaque_rect(w, h, 6, 42, 2, 14, 120);
    super::apply_satin(
        &mut px,
        w,
        h,
        &super::SatinParams {
            distance: 10.0,
            angle_deg: 90.0,
            size: 4,
            color: [30, 30, 60, 255],
            opacity: 100.0,
        },
    );
    let band = px[(7 * w as usize + 9) * 4];
    let far = px[(7 * w as usize + 38) * 4];
    assert!(band < far, "left band tinted: band={band} far={far}");
    assert!((110..=120).contains(&far), "far side ~original: {far}");
}

#[test]
fn test_satin_zero_opacity_noop() {
    let (w, h) = (16u32, 16u32);
    let mut px = opaque_rect(w, h, 2, 14, 2, 14, 90);
    let before = px.clone();
    super::apply_satin(
        &mut px,
        w,
        h,
        &super::SatinParams {
            distance: 5.0,
            angle_deg: 45.0,
            size: 4,
            color: [200, 0, 0, 255],
            opacity: 0.0,
        },
    );
    assert_eq!(px, before);
}

#[test]
fn test_bevel_highlights_light_side_shadows_far_side() {
    // Wide rect; light from the LEFT (angle 270° in compass = left).
    let (w, h) = (48u32, 16u32);
    let mut px = opaque_rect(w, h, 6, 42, 2, 14, 128);
    super::apply_bevel_emboss(
        &mut px,
        w,
        h,
        &super::BevelEmbossParams {
            angle_deg: 90.0,
            depth_px: 4.0,
            size_px: 2,
            color_light: [255, 255, 255, 255],
            color_dark: [0, 0, 0, 255],
            highlight_strength: 100.0,
            shadow_strength: 100.0,
        },
    );
    let left_edge = px[(7 * w as usize + 8) * 4];
    let right_edge = px[(7 * w as usize + 40) * 4];
    assert!(
        left_edge > right_edge,
        "light side brighter: {left_edge} vs {right_edge}"
    );
}

#[test]
fn test_bevel_zero_strengths_noop() {
    let (w, h) = (16u32, 16u32);
    let mut px = opaque_rect(w, h, 2, 14, 2, 14, 90);
    let before = px.clone();
    super::apply_bevel_emboss(
        &mut px,
        w,
        h,
        &super::BevelEmbossParams {
            angle_deg: 135.0,
            depth_px: 3.0,
            size_px: 2,
            color_light: [255, 255, 255, 255],
            color_dark: [0, 0, 0, 255],
            highlight_strength: 0.0,
            shadow_strength: 0.0,
        },
    );
    assert_eq!(px, before);
}
