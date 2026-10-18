#[test]
fn blur_and_vignette_preserve_effect_specific_pixel_relationships() {
    let width = 9;
    let height = 9;
    let center = ((height / 2) * width + width / 2) as usize * 4;
    let neighbor = ((height / 2) * width + width / 2 + 1) as usize * 4;
    let corner = 0;

    let mut impulse = vec![0u8; (width * height * 4) as usize];
    impulse[center..center + 4].copy_from_slice(&[255, 255, 255, 255]);
    kagari_vfx::core::cpu_effects::apply_layer_effects(
        None,
        None,
        &mut impulse,
        width,
        height,
        &[fx(EffectType::GaussianBlur {
            blur_radius: c32(2.0),
        })],
        0,
        30,
    );
    assert!(
        impulse[neighbor] > 0,
        "blur must spread energy to a neighbor"
    );
    assert!(impulse[center] < 255, "blur must reduce the impulse peak");
    assert!(impulse[center + 3] > 0, "blur must retain alpha coverage");
    assert!(impulse[center + 3] < 255, "blur must soften alpha coverage");

    let mut vignette = vec![200u8; (width * height * 4) as usize];
    for pixel in vignette.as_chunks_mut::<4>().0 {
        pixel[3] = 255;
    }
    kagari_vfx::core::cpu_effects::apply_layer_effects(
        None,
        None,
        &mut vignette,
        width,
        height,
        &[fx(EffectType::Vignette {
            intensity: c32(100.0),
            roundness: c32(1.0),
            feather: c32(50.0),
            color: c32a4([0.0, 0.0, 0.0, 1.0]),
        })],
        0,
        30,
    );
    assert!(
        vignette[corner] < vignette[center],
        "vignette must darken the corner more than the center"
    );
    assert_eq!(vignette[corner + 3], 255, "vignette must preserve alpha");
}
#[test]
fn effect_endpoint_contracts_match_pixel_semantics() {
    let base = vec![
        10, 20, 30, 255, 40, 50, 60, 255, 70, 80, 90, 255, 100, 110, 120, 255,
    ];

    let mut wipe_zero = base.clone();
    kagari_vfx::core::cpu_effects::apply_layer_effects(
        None,
        None,
        &mut wipe_zero,
        4,
        1,
        &[fx(EffectType::LinearWipe {
            completion: c32(0.0),
            angle: c32(0.0),
        })],
        0,
        30,
    );
    assert_eq!(wipe_zero, base, "0% wipe must preserve source pixels");

    let mut wipe_full = base.clone();
    kagari_vfx::core::cpu_effects::apply_layer_effects(
        None,
        None,
        &mut wipe_full,
        4,
        1,
        &[fx(EffectType::LinearWipe {
            completion: c32(100.0),
            angle: c32(0.0),
        })],
        0,
        30,
    );
    assert!(
        wipe_full
            .as_chunks::<4>()
            .0
            .iter()
            .all(|pixel| pixel[3] == 0),
        "100% wipe must make every pixel transparent"
    );

    let mut shifted = vec![10, 0, 0, 255, 20, 0, 0, 255, 30, 0, 0, 255, 40, 0, 0, 255];
    kagari_vfx::core::cpu_effects::apply_layer_effects(
        None,
        None,
        &mut shifted,
        4,
        1,
        &[fx(EffectType::Offset {
            shift_x: c32(1.0),
            shift_y: c32(0.0),
        })],
        0,
        30,
    );
    assert_eq!(
        shifted
            .as_chunks::<4>()
            .0
            .iter()
            .map(|pixel| pixel[0])
            .collect::<Vec<_>>(),
        vec![40, 10, 20, 30],
        "offset must wrap source pixels in the documented direction"
    );

    let mut threshold = vec![10, 10, 10, 77, 240, 240, 240, 88];
    kagari_vfx::core::cpu_effects::apply_layer_effects(
        None,
        None,
        &mut threshold,
        2,
        1,
        &[fx(EffectType::Threshold {
            threshold: c32(128.0),
        })],
        0,
        30,
    );
    assert_eq!(&threshold[0..4], &[0, 0, 0, 77]);
    assert_eq!(&threshold[4..8], &[255, 255, 255, 88]);
}
#[test]
fn all_effects_with_zero_size_buffer_no_panic() {
    let effects = vec![
        fx(EffectType::GaussianBlur {
            blur_radius: c32(5.0),
        }),
        fx(EffectType::Glow {
            threshold: c32(0.5),
            radius: c32(5.0),
            intensity: c32(1.0),
            color: c32a4([1.0; 4]),
        }),
        fx(EffectType::Twirl {
            angle: c32(30.0),
            radius: c32(50.0),
        }),
        fx(EffectType::Sharpen { amount: c32(50.0) }),
    ];
    let mut empty = vec![0u8; 0];
    kagari_vfx::core::cpu_effects::apply_layer_effects(
        None, None, &mut empty, 0, 0, &effects, 0, 30,
    );
}
#[test]
fn all_effects_on_1x1_buffer_no_panic() {
    let effects = vec![
        fx(EffectType::GaussianBlur {
            blur_radius: c32(999.0),
        }),
        fx(EffectType::Glow {
            threshold: c32(0.0),
            radius: c32(999.0),
            intensity: c32(999.0),
            color: c32a4([1.0; 4]),
        }),
        fx(EffectType::Twirl {
            angle: c32(9999.0),
            radius: c32(9999.0),
        }),
        fx(EffectType::Vignette {
            intensity: c32(999.0),
            roundness: c32(999.0),
            feather: c32(999.0),
            color: c32a4([1.0; 4]),
        }),
        fx(EffectType::Sharpen {
            amount: c32(9999.0),
        }),
    ];
    let mut tiny = vec![128u8; 4];
    kagari_vfx::core::cpu_effects::apply_layer_effects(
        None, None, &mut tiny, 1, 1, &effects, 0, 30,
    );
}
#[test]
fn all_effects_idempotent() {
    let w = 16u32;
    let h = 16u32;
    let effects = vec![
        fx(EffectType::GaussianBlur {
            blur_radius: c32(3.0),
        }),
        fx(EffectType::Sharpen { amount: c32(50.0) }),
        fx(EffectType::Threshold {
            threshold: c32(128.0),
        }),
        fx(EffectType::Invert {
            invert_alpha: false,
        }),
    ];

    let mut pixels1 = make_pixels(w, h);
    let mut pixels2 = pixels1.clone();

    kagari_vfx::core::cpu_effects::apply_layer_effects(
        None,
        None,
        &mut pixels1,
        w,
        h,
        &effects,
        0,
        30,
    );
    kagari_vfx::core::cpu_effects::apply_layer_effects(
        None,
        None,
        &mut pixels2,
        w,
        h,
        &effects,
        0,
        30,
    );
    assert_eq!(
        pixels1, pixels2,
        "Same effects on same input must produce identical output"
    );
}
#[test]
fn effect_order_matters() {
    let w = 16u32;
    let h = 16u32;

    let effects_a = vec![
        fx(EffectType::Invert {
            invert_alpha: false,
        }),
        fx(EffectType::Threshold {
            threshold: c32(128.0),
        }),
    ];
    let effects_b = vec![
        fx(EffectType::Threshold {
            threshold: c32(128.0),
        }),
        fx(EffectType::Invert {
            invert_alpha: false,
        }),
    ];

    let mut px_a = make_pixels(w, h);
    let mut px_b = px_a.clone();

    kagari_vfx::core::cpu_effects::apply_layer_effects(
        None, None, &mut px_a, w, h, &effects_a, 0, 30,
    );
    kagari_vfx::core::cpu_effects::apply_layer_effects(
        None, None, &mut px_b, w, h, &effects_b, 0, 30,
    );

    // Invert+Threshold != Threshold+Invert (order matters in AE)
    assert_ne!(px_a, px_b, "Effect order should change output");
}

// ─────────────────────────────────────────────────────────────────────────────
// §6  Integration: Software Renderer + Effects + Cache
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn full_pipeline_render_with_effects() {
    let mut comp = Composition::new("test".into(), "Test".into(), 64, 64, 30, 10);
    let mut layer = Layer::new(
        "layer".into(),
        "Layer".into(),
        LayerType::Solid {
            color: [1.0, 0.5, 0.25, 1.0],
        },
        10,
    );
    layer.effects = vec![
        fx(EffectType::GaussianBlur {
            blur_radius: c32(3.0),
        }),
        fx(EffectType::Vignette {
            intensity: c32(50.0),
            roundness: c32(0.5),
            feather: c32(50.0),
            color: c32a4([0.0, 0.0, 0.0, 1.0]),
        }),
    ];
    comp.layers.push(layer);

    // Render multiple frames
    for frame in 0..10 {
        let pixels = software_renderer::render_frame_to_pixels(&comp, frame, 64, 64, 0.0, 0);
        assert_eq!(pixels.len(), 64 * 64 * 4, "Frame {frame} wrong size");
        // All pixels must be valid u8
    }
}
#[test]
fn multi_layer_composite_render() {
    let mut comp = Composition::new("test".into(), "Test".into(), 32, 32, 30, 5);

    // Red layer
    comp.layers.push(Layer::new(
        "red".into(),
        "Red".into(),
        LayerType::Solid {
            color: [1.0, 0.0, 0.0, 1.0],
        },
        5,
    ));
    // Green layer (should composite on top)
    comp.layers.push(Layer::new(
        "green".into(),
        "Green".into(),
        LayerType::Solid {
            color: [0.0, 1.0, 0.0, 1.0],
        },
        5,
    ));

    let pixels = software_renderer::render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);
    assert_eq!(pixels.len(), 32 * 32 * 4);
    // All pixels must be valid
    // Output should not be empty or all-zero
    let total: u64 = pixels.iter().map(|&p| p as u64).sum();
    assert!(
        total > 0,
        "Multi-layer render should produce non-zero output"
    );
}
#[test]
fn blend_mode_composite_render() {
    let mut comp = Composition::new("test".into(), "Test".into(), 16, 16, 30, 1);

    let bottom = Layer::new(
        "bottom".into(),
        "Bottom".into(),
        LayerType::Solid {
            color: [0.5, 0.5, 0.5, 1.0],
        },
        1,
    );
    comp.layers.push(bottom);

    let mut top = Layer::new(
        "top".into(),
        "Top".into(),
        LayerType::Solid {
            color: [0.5, 0.2, 0.2, 1.0],
        },
        1,
    );
    top.blend_mode = kagari_vfx::core::timeline::BlendMode::Add;
    comp.layers.push(top);

    let pixels = software_renderer::render_frame_to_pixels(&comp, 0, 16, 16, 0.0, 0);
    assert_eq!(pixels.len(), 16 * 16 * 4);

    // Add blend with background color factored in — verify non-zero output
    let total: u64 = pixels.iter().map(|&p| p as u64).sum();
    assert!(total > 0, "Add blend render should produce non-zero output");
}
