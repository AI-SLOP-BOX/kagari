use super::adjustments::*;
use super::balance_mixer::*;
use super::curves::*;

fn gradient_rgb() -> Vec<u8> {
    let mut v = Vec::with_capacity(256 * 4);
    for i in 0..256u16 {
        let c = i as u8;
        v.extend_from_slice(&[c, 255 - c, (c / 2), 255]);
    }
    v
}

fn solid(w: u32, h: u32, rgba: [u8; 4]) -> Vec<u8> {
    let mut v = Vec::with_capacity((w * h * 4) as usize);
    for _ in 0..w * h {
        v.extend_from_slice(&rgba);
    }
    v
}

#[test]
fn test_tone_curve_identity_and_clamping() {
    let c = ToneCurve::linear();
    assert!((c.eval(0.0) - 0.0).abs() < 1e-6);
    assert!((c.eval(0.5) - 0.5).abs() < 1e-6);
    assert!((c.eval(1.0) - 1.0).abs() < 1e-6);
    assert!((c.eval(-0.5) - 0.0).abs() < 1e-6);
    assert!((c.eval(1.5) - 1.0).abs() < 1e-6);
    // Fewer than two points falls back to identity.
    assert!((ToneCurve::new(vec![[0.4, 0.7]]).eval(0.3) - 0.3).abs() < 1e-6);
}

#[test]
fn test_tone_curve_sorts_and_dedups_points() {
    let c = ToneCurve::new(vec![[1.0, 1.0], [0.0, 0.0], [0.5, 0.6], [0.5, 0.55]]);
    assert!(c.points.windows(2).all(|w| w[0][0] <= w[1][0]));
    assert_eq!(c.eval(0.5), 0.55); // dedup keeps the last point
}

#[test]
fn test_s_curve_increases_contrast() {
    let s = ToneCurve::new(vec![
        [0.0, 0.0],
        [0.25, 0.15],
        [0.5, 0.5],
        [0.75, 0.85],
        [1.0, 1.0],
    ]);
    assert!(s.eval(0.25) < 0.25, "shadow must darken");
    assert!(s.eval(0.75) > 0.75, "highlight must brighten");
    assert!((s.eval(0.5) - 0.5).abs() < 1e-4, "mid anchor preserved");
}

#[test]
fn test_monotone_curve_does_not_overshoot() {
    // Strictly increasing data → output must stay within neighbour bounds.
    let c = ToneCurve::new(vec![[0.0, 0.0], [0.33, 0.2], [0.66, 0.8], [1.0, 1.0]]);
    let mut prev = -0.001;
    for i in 0..=64 {
        let x = i as f32 / 64.0;
        let y = c.eval(x);
        assert!((-0.05..=1.05).contains(&y), "overshoot at {x}: {y}");
        assert!(y >= prev - 1e-6, "non-monotonic at {x}");
        prev = y;
    }
}

#[test]
fn test_apply_curves_identity_leaves_pixels() {
    let src = gradient_rgb();
    let mut out = src.clone();
    apply_curves(&mut out, &ChannelCurves::default());
    assert_eq!(src, out);
}

#[test]
fn test_apply_curves_per_channel_isolation() {
    let mut px = vec![128u8, 128, 128, 255];
    let curves = ChannelCurves {
        red: Some(ToneCurve::new(vec![[0.0, 0.0], [1.0, 0.0]])), // red → black
        ..Default::default()
    };
    apply_curves(&mut px, &curves);
    assert_eq!(px[0], 0);
    assert_eq!(px[1], 128);
    assert_eq!(px[2], 128);
}

#[test]
fn test_color_balance_zero_is_identity() {
    let src = gradient_rgb();
    let mut out = src.clone();
    apply_color_balance(&mut out, &ColorBalance::default());
    assert_eq!(src, out);
}

#[test]
fn test_color_balance_shifts_channels_directionally() {
    let mut px = vec![128u8, 128, 128, 255];
    apply_color_balance(
        &mut px,
        &ColorBalance {
            midtones: [50.0, 0.0, -50.0],
            preserve_luminosity: false,
            ..Default::default()
        },
    );
    assert!(px[0] > 150, "red pushed up: {}", px[0]);
    assert!(px[2] < 110, "blue pulled down: {}", px[2]);
}

#[test]
fn test_color_balance_preserve_luminosity_keeps_luma() {
    let src = gradient_rgb();
    let mut out = src.clone();
    apply_color_balance(
        &mut out,
        &ColorBalance {
            shadows: [-40.0, 20.0, 60.0],
            midtones: [30.0, -50.0, 10.0],
            highlights: [70.0, 0.0, -30.0],
            preserve_luminosity: true,
        },
    );
    for (src_px, out_px) in src.chunks(4).zip(out.chunks(4)) {
        let l_in = rec709_luma(src_px[0] as f32, src_px[1] as f32, src_px[2] as f32);
        let l_out = rec709_luma(out_px[0] as f32, out_px[1] as f32, out_px[2] as f32);
        assert!(
            (l_in - l_out).abs() < 12.0,
            "luma drift {} vs {}",
            l_in,
            l_out
        );
    }
}

#[test]
fn test_channel_mixer_identity_and_swap() {
    let src = vec![200u8, 40, 90, 255];

    let mut out = src.clone();
    apply_channel_mixer(&mut out, &ChannelMixer::default());
    assert_eq!(out, src);

    let swap = ChannelMixer {
        matrix: [[0.0, 100.0, 0.0], [100.0, 0.0, 0.0], [0.0, 0.0, 100.0]],
        monochrome: false,
    };
    let mut swapped = src.clone();
    apply_channel_mixer(&mut swapped, &swap);
    assert_eq!((swapped[0], swapped[1]), (src[1], src[0]));
}

#[test]
fn test_channel_mixer_monochrome_grays() {
    let mut px = vec![220u8, 30, 60, 255];
    apply_channel_mixer(
        &mut px,
        &ChannelMixer {
            matrix: [[30.0, 59.0, 11.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
            monochrome: true,
        },
    );
    assert_eq!(px[0], px[1]);
    assert_eq!(px[1], px[2]);
    let expected = ((220.0f32 * 30.0 + 30.0 * 59.0 + 60.0 * 11.0) / 100.0).round() as u8;
    assert_eq!(px[0], expected);
}

#[test]
fn test_all_kernels_handle_empty_buffers() {
    let mut empty: Vec<u8> = vec![];
    apply_curves(&mut empty, &ChannelCurves::default());
    apply_color_balance(&mut empty, &ColorBalance::default());
    apply_channel_mixer(&mut empty, &ChannelMixer::default());
    assert!(empty.is_empty());
}

#[test]
fn test_luma_histogram_counts_every_pixel() {
    // Two pixels: pure red (luma ≈ 54) and mid gray (128).
    let mut buf = Vec::new();
    buf.extend_from_slice(&[255, 0, 0, 255]);
    buf.extend_from_slice(&[128, 128, 128, 255]);
    let h = compute_luma_histogram(&buf);
    let total: u32 = h.iter().sum();
    assert_eq!(total, 2);
    assert!(h[54] >= 1 || h[55] >= 1, "red luma bin missing");
    assert!(h[128] == 1, "gray bin missing");
    assert_eq!(compute_luma_histogram(&[]), [0u32; 256]);
}

#[test]
fn test_rgb_histograms_separate_channels() {
    let mut buf = Vec::new();
    buf.extend_from_slice(&[10, 200, 30, 255]);
    buf.extend_from_slice(&[20, 210, 40, 255]);
    let [r, g, b] = compute_rgb_histograms(&buf);
    assert_eq!(r[10], 1);
    assert_eq!(r[20], 1);
    assert_eq!(g[200], 1);
    assert_eq!(g[210], 1);
    assert_eq!(b[30], 1);
    assert_eq!(b[40], 1);
    let total_r: u32 = r.iter().sum();
    assert_eq!(total_r, 2);
}

#[test]
fn test_extreme_inputs_do_not_panic_or_nan() {
    let mut buf = vec![255u8; 64 * 4];
    let wild = ColorBalance {
        shadows: [-500.0, 500.0, 0.0],
        midtones: [1000.0, -1000.0, 0.0],
        highlights: [0.0, 250.0, -250.0],
        preserve_luminosity: true,
    };
    apply_color_balance(&mut buf, &wild);
    // White pixel: shadow/midtone weights are 0, highlight shift clamps
    // blue to 0, then luminosity preservation lifts it back to ~18.
    // A missing intermediate clamp (or dropped preserve step) yields 0.
    assert!(
        buf.as_chunks::<4>()
            .0
            .iter()
            .all(|px| px[0] == 255 && px[1] == 255 && px[3] == 255),
        "red/green/alpha must survive extreme balance, got {:?}",
        &buf[..4]
    );
    assert!(
        buf.as_chunks::<4>().0.iter().all(|px| px[2] == 18),
        "clamped blue channel must be luminosity-lifted to 18, got {:?}",
        &buf[..4]
    );

    let huge = ChannelMixer {
        matrix: [[900.0, -400.0, 300.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
        monochrome: false,
    };
    apply_channel_mixer(&mut buf, &huge);
    // (900*255 - 400*255 + 300*18)/100 saturates red; green/blue rows are 0.
    assert!(
        buf.as_chunks::<4>()
            .0
            .iter()
            .all(|px| *px == [255, 0, 0, 255]),
        "huge mixer must saturate to pure red, got {:?}",
        &buf[..4]
    );
}

#[test]
fn test_vibrance_leaves_gray_unchanged() {
    // Pure gray has zero chroma: boosting vibrance must not shift it.
    let mut img = solid(8, 8, [128, 128, 128, 255]);
    let before = img.clone();
    apply_vibrance(&mut img, 100.0);
    assert_eq!(img, before, "gray must survive max vibrance");
}

#[test]
fn test_vibrance_boosts_muted_more_than_saturated() {
    // Muted teal vs fully saturated red.
    let mut img = vec![80u8, 120, 130, 255, 255, 0, 0, 255];
    let before = img.clone();
    apply_vibrance(&mut img, 60.0);
    let muted_drift = (img[0] as i32 - before[0] as i32).abs()
        + (img[1] as i32 - before[1] as i32).abs()
        + (img[2] as i32 - before[2] as i32).abs();
    let sat_drift = (img[4] as i32 - before[4] as i32).abs()
        + (img[5] as i32 - before[5] as i32).abs()
        + (img[6] as i32 - before[6] as i32).abs();
    assert!(
        muted_drift > sat_drift,
        "muted {muted_drift} must move more than saturated {sat_drift}"
    );
}

#[test]
fn test_negative_vibrance_desaturates_and_zero_is_identity() {
    let mut img = solid(4, 4, [200, 40, 40, 255]);
    apply_vibrance(&mut img, -90.0);
    let spread = img[0].max(img[1]) - img[0].min(img[1]);
    assert!(spread < 120, "spread should shrink, got {}", spread);

    let mut id = solid(4, 4, [200, 40, 40, 255]);
    apply_vibrance(&mut id, 0.0);
    assert_eq!(id, solid(4, 4, [200, 40, 40, 255]));
}

#[test]
fn test_vibrance_deterministic_and_safe() {
    let run = || {
        let mut img = solid(16, 16, [90, 140, 200, 255]);
        apply_vibrance(&mut img, 45.0);
        img
    };
    assert_eq!(run(), run());
    apply_vibrance(&mut [], 50.0); // empty buffer safe
}

#[test]
fn test_white_balance_warms_and_cools() {
    let base = solid(4, 4, [128, 128, 128, 255]);

    let mut warm = base.clone();
    apply_white_balance(
        &mut warm,
        &WhiteBalance {
            temperature: 80.0,
            tint: 0.0,
        },
    );
    assert!(warm[0] > 128, "warm raises R: {}", warm[0]);
    assert!(warm[2] < 128, "warm lowers B: {}", warm[2]);
    assert_eq!(warm[1], 128, "tint 0 keeps G");

    let mut cool = base.clone();
    apply_white_balance(
        &mut cool,
        &WhiteBalance {
            temperature: -80.0,
            tint: 0.0,
        },
    );
    assert!(cool[0] < 128 && cool[2] > 128);

    let mut mag = base.clone();
    apply_white_balance(
        &mut mag,
        &WhiteBalance {
            temperature: 0.0,
            tint: 60.0,
        },
    );
    assert!(mag[1] < 128, "positive tint reduces G (magenta)");

    let mut neutral = base.clone();
    apply_white_balance(&mut neutral, &WhiteBalance::default());
    assert_eq!(neutral, base);
}

#[test]
fn test_hsl_identity_at_defaults() {
    let src = solid(8, 8, [200, 90, 30, 255]);
    let mut img = src.clone();
    apply_hsl_adjust(&mut img, &HslAdjust::default());
    assert_eq!(img, src);
}

#[test]
fn test_hsl_full_desaturation_grays_out() {
    let mut img = solid(4, 4, [200, 40, 40, 255]);
    apply_hsl_adjust(
        &mut img,
        &HslAdjust {
            saturation: -100.0,
            ..Default::default()
        },
    );
    assert_eq!(img[0], img[1], "R==G");
    assert_eq!(img[1], img[2], "G==B");
}

#[test]
fn test_hsl_hue_rotation_moves_red_to_cyan_family() {
    // Pure red rotated +180° lands in the cyan family (low R, high G+B).
    let mut img = solid(2, 2, [255, 0, 0, 255]);
    apply_hsl_adjust(
        &mut img,
        &HslAdjust {
            hue_deg: 180.0,
            ..Default::default()
        },
    );
    assert!(img[0] < 60, "R dropped: {}", img[0]);
    assert!(img[1] > 180 && img[2] > 180, "G/B high: {:?}", &img[..3]);
    // Rotation wraps: +360° returns to the original hue.
    let mut wrapped = solid(2, 2, [255, 0, 0, 255]);
    apply_hsl_adjust(
        &mut wrapped,
        &HslAdjust {
            hue_deg: 360.0,
            ..Default::default()
        },
    );
    assert!(
        wrapped[0] > 230 && wrapped[1] < 25,
        "wrap keeps red: {:?}",
        &wrapped[..3]
    );
}

#[test]
fn test_hsl_lightness_extremes_and_determinism() {
    let run = |light: f32| {
        let mut img = solid(4, 4, [120, 120, 120, 255]);
        apply_hsl_adjust(
            &mut img,
            &HslAdjust {
                lightness: light,
                ..Default::default()
            },
        );
        img
    };
    let bright = run(100.0);
    assert!(
        bright[0] > 240,
        "+100 lightness → near white: {}",
        bright[0]
    );
    let dark = run(-100.0);
    assert!(dark[0] < 15, "-100 lightness → near black: {}", dark[0]);
    assert_eq!(run(20.0), run(20.0), "deterministic");
    // Empty buffer safe.
    apply_hsl_adjust(
        &mut [],
        &HslAdjust {
            hue_deg: 90.0,
            saturation: 50.0,
            lightness: 10.0,
        },
    );
}
