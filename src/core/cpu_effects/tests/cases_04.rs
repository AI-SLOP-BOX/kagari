#[test]
fn test_distort_pack2_neutral_settings_are_noops() {
    let base = solid_layer(8, 8, 90, 140, 200);

    let mut px = base.clone();
    apply_layer_effects(
        None,
        None,
        &mut px,
        8,
        8,
        &[effect(
            "pinch",
            EffectType::PinchPunch {
                radius: Animatable::new_constant(50.0),
                amount: Animatable::new_constant(0.0),
            },
        )],
        0,
        30,
    );
    assert_eq!(px, base, "zero-amount pinch/punch must be a no-op");

    let mut px = base.clone();
    apply_layer_effects(
        None,
        None,
        &mut px,
        8,
        8,
        &[effect(
            "sg",
            EffectType::ScanlineGlitch {
                jitter_amount: Animatable::new_constant(0.0),
                seed: Animatable::new_constant(1.0),
            },
        )],
        0,
        30,
    );
    assert_eq!(px, base, "zero-jitter scanline glitch must be a no-op");

    let mut px = base.clone();
    apply_layer_effects(
        None,
        None,
        &mut px,
        8,
        8,
        &[effect(
            "dsh",
            EffectType::DirectionalSharpen {
                angle_deg: Animatable::new_constant(45.0),
                strength: Animatable::new_constant(0.0),
            },
        )],
        0,
        30,
    );
    assert_eq!(
        px, base,
        "zero-strength directional sharpen must be a no-op"
    );
}
#[test]
fn test_generate_stylize_pack() {
    let base = solid_layer(8, 8, 128, 128, 128);

    // A flat single-colour gradient maps every pixel to that colour.
    let mut px = base.clone();
    apply_layer_effects(
        None,
        None,
        &mut px,
        8,
        8,
        &[effect(
            "gm",
            EffectType::GradientMap {
                low_color: Animatable::new_constant([1.0, 0.0, 0.0]),
                mid_color: Animatable::new_constant([1.0, 0.0, 0.0]),
                high_color: Animatable::new_constant([1.0, 0.0, 0.0]),
            },
        )],
        0,
        30,
    );
    for p in px.as_chunks::<4>().0 {
        assert_eq!(p[0], 255, "flat gradient map R");
        assert_eq!(p[1], 0, "flat gradient map G");
        assert_eq!(p[2], 0, "flat gradient map B");
    }

    // Zero-intensity light leak must be a no-op.
    let mut px = base.clone();
    apply_layer_effects(
        None,
        None,
        &mut px,
        8,
        8,
        &[effect(
            "leak",
            EffectType::LightLeak {
                pos_x: Animatable::new_constant(0.5),
                pos_y: Animatable::new_constant(0.5),
                intensity: Animatable::new_constant(0.0),
            },
        )],
        0,
        30,
    );
    assert_eq!(px, base, "zero-intensity light leak must be a no-op");
}
#[test]
fn test_reflection_generate_pack() {
    let base = solid_layer(8, 8, 128, 128, 128);

    // Zero-opacity reflection is a no-op.
    let mut px = base.clone();
    apply_layer_effects(
        None,
        None,
        &mut px,
        8,
        8,
        &[effect(
            "refl",
            EffectType::ReflectionMap {
                reflect_y: Animatable::new_constant(4.0),
                fade_dist: Animatable::new_constant(50.0),
                opacity: Animatable::new_constant(0.0),
            },
        )],
        0,
        30,
    );
    assert_eq!(px, base, "zero-opacity reflection must be a no-op");

    // Perlin flow and FBM turbulence are deterministic per frame.
    let run = |et: EffectType| {
        let mut px = base.clone();
        apply_layer_effects(None, None, &mut px, 8, 8, &[effect("gen", et)], 3, 30);
        px
    };
    let p1 = run(EffectType::PerlinFlow {
        scale: Animatable::new_constant(4.0),
    });
    let p1b = run(EffectType::PerlinFlow {
        scale: Animatable::new_constant(4.0),
    });
    assert_eq!(p1, p1b, "perlin flow must be deterministic");

    let f1 = run(EffectType::FbmTurbulence {
        octaves: Animatable::new_constant(3.0),
        amplitude: Animatable::new_constant(60.0),
    });
    let f1b = run(EffectType::FbmTurbulence {
        octaves: Animatable::new_constant(3.0),
        amplitude: Animatable::new_constant(60.0),
    });
    assert_eq!(f1, f1b, "fbm turbulence must be deterministic");
}
