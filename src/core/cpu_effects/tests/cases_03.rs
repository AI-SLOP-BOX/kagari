#[test]
fn test_stylize_transition_pack() {
    // Night vision output must be phosphor-green dominant.
    let mut px = solid_layer(4, 4, 90, 140, 200);
    apply_layer_effects(
        None,
        None,
        &mut px,
        4,
        4,
        &[effect(
            "nv",
            EffectType::NightVision {
                amplification: Animatable::new_constant(2.0),
            },
        )],
        0,
        30,
    );
    for p in px.as_chunks::<4>().0 {
        assert!(p[1] >= p[0] && p[1] >= p[2], "green channel must dominate");
    }
    // Determinism: same frame → identical output.
    let mut again = solid_layer(4, 4, 90, 140, 200);
    apply_layer_effects(
        None,
        None,
        &mut again,
        4,
        4,
        &[effect(
            "nv",
            EffectType::NightVision {
                amplification: Animatable::new_constant(2.0),
            },
        )],
        0,
        30,
    );
    assert_eq!(px, again, "night vision must be deterministic per frame");

    // Iris wipe at 0% must leave the image untouched (fully covered).
    let base = solid_layer(8, 8, 90, 140, 200);
    let mut px = base.clone();
    apply_layer_effects(
        None,
        None,
        &mut px,
        8,
        8,
        &[effect(
            "iris",
            EffectType::IrisWipe {
                completion: Animatable::new_constant(0.0),
            },
        )],
        0,
        30,
    );
    assert_eq!(px, base, "0% iris wipe must be a no-op");

    // Same for radial wipe.
    let mut px = base.clone();
    apply_layer_effects(
        None,
        None,
        &mut px,
        8,
        8,
        &[effect(
            "rad",
            EffectType::RadialWipe {
                completion: Animatable::new_constant(0.0),
            },
        )],
        0,
        30,
    );
    assert_eq!(px, base, "0% radial wipe must be a no-op");
}
#[test]
fn test_grade_light_pack() {
    let base = solid_layer(8, 8, 90, 140, 200);

    // Neutral ASC CDL grade must stay within rounding distance of identity.
    let mut px = base.clone();
    apply_layer_effects(
        None,
        None,
        &mut px,
        8,
        8,
        &[effect(
            "film",
            EffectType::FilmEmulation {
                lift: Animatable::new_constant(0.0),
                gamma: Animatable::new_constant(1.0),
                gain: Animatable::new_constant(1.0),
                hue_shift_deg: Animatable::new_constant(0.0),
            },
        )],
        0,
        30,
    );
    for (out, inp) in px
        .as_chunks::<4>()
        .0
        .iter()
        .zip(base.as_chunks::<4>().0.iter())
    {
        assert!(
            (out[0] as i32 - inp[0] as i32).abs() <= 2,
            "R drift {}",
            out[0]
        );
        assert!(
            (out[1] as i32 - inp[1] as i32).abs() <= 2,
            "G drift {}",
            out[1]
        );
        assert!(
            (out[2] as i32 - inp[2] as i32).abs() <= 2,
            "B drift {}",
            out[2]
        );
    }

    // God rays with zero weight adds no light.
    let mut px = base.clone();
    apply_layer_effects(
        None,
        None,
        &mut px,
        8,
        8,
        &[effect(
            "rays",
            EffectType::GodRays {
                sun_x: Animatable::new_constant(0.5),
                sun_y: Animatable::new_constant(0.0),
                samples: Animatable::new_constant(8.0),
                decay: Animatable::new_constant(0.9),
                weight: Animatable::new_constant(0.0),
            },
        )],
        0,
        30,
    );
    assert_eq!(px, base, "zero-weight god rays must be a no-op");

    // Zero-amount zoom blur must be a no-op.
    let mut px = base.clone();
    apply_layer_effects(
        None,
        None,
        &mut px,
        8,
        8,
        &[effect(
            "zb",
            EffectType::RadialBlurZoom {
                amount: Animatable::new_constant(0.0),
            },
        )],
        0,
        30,
    );
    assert_eq!(px, base, "zero-amount zoom blur must be a no-op");

    // Positive zoom blur must keep the centre bright on a uniform image.
    let mut px = vec![255u8; 16 * 16 * 4];
    apply_layer_effects(
        None,
        None,
        &mut px,
        16,
        16,
        &[effect(
            "zb",
            EffectType::RadialBlurZoom {
                amount: Animatable::new_constant(40.0),
            },
        )],
        0,
        30,
    );
    let center = ((8 * 16 + 8) * 4) as usize;
    assert!(px[center] > 0, "zoom blur keeps centre bright");
}
#[test]
fn test_sharpen_stylize_pack_uniform_safety() {
    // Median filter of a uniform image must be the image itself.
    let base = solid_layer(8, 8, 90, 140, 200);
    let mut px = base.clone();
    apply_layer_effects(
        None,
        None,
        &mut px,
        8,
        8,
        &[effect(
            "med",
            EffectType::MedianFilter {
                radius: Animatable::new_constant(2.0),
            },
        )],
        0,
        30,
    );
    assert_eq!(px, base, "median of uniform image is identity");

    // Mosaic with block size 1x1 must be identity.
    let mut px = base.clone();
    apply_layer_effects(
        None,
        None,
        &mut px,
        8,
        8,
        &[effect(
            "mos",
            EffectType::Mosaic {
                block_w: Animatable::new_constant(1.0),
                block_h: Animatable::new_constant(1.0),
            },
        )],
        0,
        30,
    );
    assert_eq!(px, base, "1x1 mosaic is identity");

    // Sobel on a uniform image must produce no edges inside the frame.
    // (The kernel leaves the 1px border untouched, so only check interior.)
    let mut px = base.clone();
    apply_layer_effects(
        None,
        None,
        &mut px,
        8,
        8,
        &[effect("sobel", EffectType::SobelEdges { invert: false })],
        0,
        30,
    );
    for y in 1..7usize {
        for x in 1..7usize {
            let i = (y * 8 + x) * 4;
            assert!(
                px[i] < 16,
                "uniform image must have no sobel edges (got {})",
                px[i]
            );
        }
    }
}
#[test]
fn test_simulation_pack_determinism_and_noop() {
    let base = solid_layer(8, 8, 90, 140, 200);

    // Fire with zero intensity is a no-op.
    let mut px = base.clone();
    apply_layer_effects(
        None,
        None,
        &mut px,
        8,
        8,
        &[effect(
            "fire",
            EffectType::FireAutomaton {
                intensity: Animatable::new_constant(0.0),
            },
        )],
        0,
        30,
    );
    assert_eq!(px, base, "zero-intensity fire must be a no-op");

    // Star field is deterministic for a given frame.
    let run_stars = || {
        let mut px = base.clone();
        apply_layer_effects(
            None,
            None,
            &mut px,
            8,
            8,
            &[effect(
                "stars",
                EffectType::StarField {
                    num_stars: Animatable::new_constant(20.0),
                    depth_speed: Animatable::new_constant(1.0),
                },
            )],
            5,
            30,
        );
        px
    };
    assert_eq!(run_stars(), run_stars(), "star field must be deterministic");

    // Lightning is deterministic per frame as well.
    let run_bolt = || {
        let mut px = base.clone();
        apply_layer_effects(
            None,
            None,
            &mut px,
            8,
            8,
            &[effect(
                "bolt",
                EffectType::LightningArc {
                    start_x: Animatable::new_constant(0.1),
                    start_y: Animatable::new_constant(0.1),
                    end_x: Animatable::new_constant(0.9),
                    end_y: Animatable::new_constant(0.9),
                    seed: Animatable::new_constant(42.0),
                    glow: Animatable::new_constant(1.0),
                },
            )],
            7,
            30,
        );
        px
    };
    assert_eq!(run_bolt(), run_bolt(), "lightning must be deterministic");
}
#[test]
fn test_key_stylize_pack() {
    let base = solid_layer(8, 8, 90, 140, 200);
    // Rec.601-ish integer luma of the base colour.
    let luma_of_base = 131u32;

    // Solarize with threshold 255 never triggers — identity.
    let mut px = base.clone();
    apply_layer_effects(
        None,
        None,
        &mut px,
        8,
        8,
        &[effect(
            "sol",
            EffectType::Solarize {
                threshold: Animatable::new_constant(255.0),
            },
        )],
        0,
        30,
    );
    assert_eq!(px, base, "solarize above all values must be a no-op");

    // Luma key non-inverted: pixels inside the band become transparent.
    let mut px = base.clone();
    apply_layer_effects(
        None,
        None,
        &mut px,
        8,
        8,
        &[effect(
            "lk",
            EffectType::LumaKeyRange {
                low_threshold: Animatable::new_constant((luma_of_base - 10) as f32),
                high_threshold: Animatable::new_constant((luma_of_base + 10) as f32),
                invert: false,
            },
        )],
        0,
        30,
    );
    assert_eq!(px[3], 0, "in-band pixel must key to transparent");

    // Inverted: pixels outside the band become transparent instead.
    let mut px = base.clone();
    apply_layer_effects(
        None,
        None,
        &mut px,
        8,
        8,
        &[effect(
            "lk",
            EffectType::LumaKeyRange {
                low_threshold: Animatable::new_constant(0.0),
                high_threshold: Animatable::new_constant(50.0),
                invert: true,
            },
        )],
        0,
        30,
    );
    assert_eq!(px[3], 0, "out-of-band pixel must key when inverted");
}
