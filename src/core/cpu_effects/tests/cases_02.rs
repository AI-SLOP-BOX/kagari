#[test]
fn test_white_balance_warms_and_cools() {
    let mut warm = solid_layer(4, 4, 120, 120, 120);
    apply_layer_effects(
        None,
        None,
        &mut warm,
        4,
        4,
        &[effect(
            "wb",
            EffectType::WhiteBalance {
                temperature: Animatable::new_constant(100.0),
                tint: Animatable::new_constant(0.0),
            },
        )],
        0,
        30,
    );
    assert!(
        warm[0] > warm[2],
        "positive temperature must push R above B"
    );

    let mut cool = solid_layer(4, 4, 120, 120, 120);
    apply_layer_effects(
        None,
        None,
        &mut cool,
        4,
        4,
        &[effect(
            "wb",
            EffectType::WhiteBalance {
                temperature: Animatable::new_constant(-100.0),
                tint: Animatable::new_constant(0.0),
            },
        )],
        0,
        30,
    );
    assert!(
        cool[2] > cool[0],
        "negative temperature must push B above R"
    );
}
#[test]
fn test_hsl_adjust_neutral_noop_and_hue_rotation() {
    let mut px = solid_layer(4, 4, 90, 140, 200);
    let before = px.clone();
    apply_layer_effects(
        None,
        None,
        &mut px,
        4,
        4,
        &[effect(
            "hsl",
            EffectType::HslAdjust {
                hue_deg: Animatable::new_constant(0.0),
                saturation: Animatable::new_constant(0.0),
                lightness: Animatable::new_constant(0.0),
            },
        )],
        0,
        30,
    );
    assert_eq!(px, before, "neutral HSL must be a no-op");

    let mut red = solid_layer(4, 4, 255, 40, 40);
    apply_layer_effects(
        None,
        None,
        &mut red,
        4,
        4,
        &[effect(
            "hsl",
            EffectType::HslAdjust {
                hue_deg: Animatable::new_constant(180.0),
                saturation: Animatable::new_constant(0.0),
                lightness: Animatable::new_constant(0.0),
            },
        )],
        0,
        30,
    );
    assert!(
        red[0] < red[1].max(red[2]),
        "180° rotation must move red toward cyan"
    );
}
#[test]
fn test_glow_pro_bleeds_bright_pixels() {
    // Dark 8x8 buffer with a single white pixel: glow must spread brightness.
    let mut px = vec![0u8; 8 * 8 * 4];
    for p in px.as_chunks_mut::<4>().0 {
        p[3] = 255;
    }
    let cx = ((4 * 8 + 4) * 4) as usize;
    px[cx] = 255;
    px[cx + 1] = 255;
    px[cx + 2] = 255;

    apply_layer_effects(
        None,
        None,
        &mut px,
        8,
        8,
        &[effect(
            "glow",
            EffectType::GlowPro {
                threshold: Animatable::new_constant(0.5),
                radius: Animatable::new_constant(2.0),
                intensity: Animatable::new_constant(1.5),
            },
        )],
        0,
        30,
    );
    let neighbor = ((4 * 8 + 6) * 4) as usize;
    assert!(px[neighbor] > 0, "glow did not bleed to neighbor pixel");
    // Zero intensity must be a no-op.
    let mut idle = solid_layer(4, 4, 200, 200, 200);
    let before = idle.clone();
    apply_layer_effects(
        None,
        None,
        &mut idle,
        4,
        4,
        &[effect(
            "glow",
            EffectType::GlowPro {
                threshold: Animatable::new_constant(0.5),
                radius: Animatable::new_constant(4.0),
                intensity: Animatable::new_constant(0.0),
            },
        )],
        0,
        30,
    );
    assert_eq!(idle, before, "zero-intensity glow must be a no-op");
}
#[test]
fn test_stylize_distort_pack_noops_and_effects() {
    // Zero-strength / zero-angle settings must be exact no-ops.
    let base = solid_layer(8, 8, 90, 140, 200);

    let mut px = base.clone();
    apply_layer_effects(
        None,
        None,
        &mut px,
        8,
        8,
        &[effect(
            "heat",
            EffectType::HeatDistortion {
                strength: Animatable::new_constant(0.0),
                speed: Animatable::new_constant(1.0),
            },
        )],
        0,
        30,
    );
    assert_eq!(px, base, "zero-strength heat must be a no-op");

    let mut px = base.clone();
    apply_layer_effects(
        None,
        None,
        &mut px,
        8,
        8,
        &[effect(
            "vortex",
            EffectType::Vortex {
                radius: Animatable::new_constant(50.0),
                angle_deg: Animatable::new_constant(0.0),
            },
        )],
        0,
        30,
    );
    assert_eq!(px, base, "zero-angle vortex must be a no-op");

    // CRT scanlines must darken even rows only.
    let mut px = base.clone();
    apply_layer_effects(
        None,
        None,
        &mut px,
        8,
        8,
        &[effect(
            "crt",
            EffectType::CrtScanlines {
                line_spacing: Animatable::new_constant(2.0),
                intensity: Animatable::new_constant(0.5),
            },
        )],
        0,
        30,
    );
    let row0 = (px[0] as u32 + px[4] as u32) / 2;
    let row1 = (px[32] as u32 + px[36] as u32) / 2;
    assert_eq!(row0, 45, "row 0 (y%2==0) darkened to 50%");
    assert_eq!(row1, 90, "odd rows untouched");
}
#[test]
fn test_distort_pack_neutral_settings_are_noops() {
    let base = solid_layer(8, 8, 90, 140, 200);

    let mut px = base.clone();
    apply_layer_effects(
        None,
        None,
        &mut px,
        8,
        8,
        &[effect(
            "fish",
            EffectType::Fisheye {
                strength: Animatable::new_constant(0.0),
            },
        )],
        0,
        30,
    );
    assert_eq!(px, base, "zero-strength fisheye must be a no-op");

    let mut px = base.clone();
    apply_layer_effects(
        None,
        None,
        &mut px,
        8,
        8,
        &[effect(
            "lc",
            EffectType::LensCorrection {
                k1: Animatable::new_constant(0.0),
                k2: Animatable::new_constant(0.0),
            },
        )],
        0,
        30,
    );
    assert_eq!(px, base, "k1=k2=0 lens correction must be identity");

    let mut px = base.clone();
    apply_layer_effects(
        None,
        None,
        &mut px,
        8,
        8,
        &[effect(
            "glitch",
            EffectType::GlitchDisplacement {
                seed: Animatable::new_constant(1.0),
                amount: Animatable::new_constant(0.0),
            },
        )],
        0,
        30,
    );
    assert_eq!(px, base, "zero-amount glitch must be a no-op");
}
#[test]
fn test_matte_alpha_pack_behaviour() {
    // Opaque square in a transparent 8x8 buffer (alpha edge at x=1..6).
    let mut base = vec![0u8; 8 * 8 * 4];
    for y in 1..7usize {
        for x in 1..7usize {
            let i = (y * 8 + x) * 4;
            base[i] = 200;
            base[i + 1] = 100;
            base[i + 2] = 50;
            base[i + 3] = 255;
        }
    }

    // Choke with radius 1 must shrink the matte: corner pixel (1,1) loses alpha.
    let mut px = base.clone();
    apply_layer_effects(
        None,
        None,
        &mut px,
        8,
        8,
        &[effect(
            "choke",
            EffectType::MatteChokeSpread {
                radius: Animatable::new_constant(1.0),
                expand: false,
            },
        )],
        0,
        30,
    );
    let corner = (8 + 1) * 4;
    assert_eq!(px[corner + 3], 0, "choke must erode the alpha corner");

    // Spread must re-dilate it back.
    let mut px = base.clone();
    apply_layer_effects(
        None,
        None,
        &mut px,
        8,
        8,
        &[effect(
            "spread",
            EffectType::MatteChokeSpread {
                radius: Animatable::new_constant(1.0),
                expand: true,
            },
        )],
        0,
        30,
    );
    assert_eq!(px[(8 * 4) + 3], 255, "spread must dilate alpha outward");

    // Luma→alpha: white = opaque, black = transparent; invert flips both.
    let mut px = vec![0u8; 2 * 4];
    px[0] = 255;
    px[1] = 255;
    px[2] = 255;
    px[3] = 255;
    px[4] = 0;
    px[5] = 0;
    px[6] = 0;
    px[7] = 255;
    apply_layer_effects(
        None,
        None,
        &mut px,
        2,
        1,
        &[effect(
            "luma",
            EffectType::AlphaFromLuminance { invert: false },
        )],
        0,
        30,
    );
    assert_eq!(px[3], 255, "white luma must become full alpha");
    assert_eq!(px[7], 0, "black luma must become zero alpha");

    let mut px = vec![0u8; 2 * 4];
    px[0] = 255;
    px[1] = 255;
    px[2] = 255;
    px[3] = 255;
    px[4] = 0;
    px[5] = 0;
    px[6] = 0;
    px[7] = 255;
    apply_layer_effects(
        None,
        None,
        &mut px,
        2,
        1,
        &[effect(
            "lumainv",
            EffectType::AlphaFromLuminance { invert: true },
        )],
        0,
        30,
    );
    assert_eq!(px[3], 0, "inverted white must become transparent");
    assert_eq!(px[7], 255, "inverted black must become opaque");
}
