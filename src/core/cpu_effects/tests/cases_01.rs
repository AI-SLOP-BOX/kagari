#[test]
fn source_layer_effects_are_noops_without_a_resolved_source() {
    let original = solid_layer(8, 8, 96, 128, 160);

    let mut displacement = original.clone();
    apply_layer_effects(
        None,
        None,
        &mut displacement,
        8,
        8,
        &[effect(
            "displacement",
            EffectType::DisplacementMap {
                source_layer: Animatable::new_constant(0.0),
                max_horizontal: Animatable::new_constant(24.0),
                max_vertical: Animatable::new_constant(18.0),
            },
        )],
        0,
        30,
    );
    assert_eq!(displacement, original);

    let mut compound_blur = original.clone();
    apply_layer_effects(
        None,
        None,
        &mut compound_blur,
        8,
        8,
        &[effect(
            "compound-blur",
            EffectType::CompoundBlur {
                source_layer: Animatable::new_constant(0.0),
                max_blur: Animatable::new_constant(8.0),
            },
        )],
        0,
        30,
    );
    assert_eq!(compound_blur, original);
}

fn patterned_layer(width: u32, height: u32) -> Vec<u8> {
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    for y in 0..height {
        for x in 0..width {
            let i = ((y * width + x) * 4) as usize;
            pixels[i] = (x * 13).min(255) as u8;
            pixels[i + 1] = (y * 13).min(255) as u8;
            pixels[i + 2] = 96;
            pixels[i + 3] = 255;
        }
    }
    pixels
}

fn source_map_composition(width: u32, height: u32) -> Composition {
    let mut comp = Composition::new(
        "source-map-effects".into(),
        "Source map effects".into(),
        width,
        height,
        30,
        30,
    );
    comp.background_color = [0.0, 0.0, 0.0, 0.0];
    let mut source = crate::core::timeline::Layer::new(
        "source".into(),
        "Source map".into(),
        crate::core::timeline::LayerType::Shape {
            shape_type: crate::core::timeline::ShapeType::Rectangle {
                width: Animatable::new_constant(width as f32 * 0.5),
                height: Animatable::new_constant(height as f32),
                corner_radius: Animatable::new_constant(0.0),
            },
            color: [1.0, 1.0, 1.0, 1.0],
            stroke_color: [0.0, 0.0, 0.0, 0.0],
            stroke_width: 0.0,
            fill_type: Default::default(),
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        },
        30,
    );
    source.transform.position =
        Animatable::new_constant([width as f32 * 0.25, height as f32 * 0.5]);
    comp.layers.push(source);
    comp.layers.push(crate::core::timeline::Layer::new(
        "target".into(),
        "Target".into(),
        crate::core::timeline::LayerType::Solid {
            color: [1.0, 0.0, 0.0, 1.0],
        },
        30,
    ));
    comp
}
#[test]
fn displacement_map_resolves_a_real_source_layer() {
    let width = 16;
    let height = 16;
    let comp = source_map_composition(width, height);
    let original = patterned_layer(width, height);
    let mut rendered = original.clone();
    apply_layer_effects(
        Some(&comp),
        Some(1),
        &mut rendered,
        width,
        height,
        &[effect(
            "displacement",
            EffectType::DisplacementMap {
                source_layer: Animatable::new_constant(0.0),
                max_horizontal: Animatable::new_constant(5.0),
                max_vertical: Animatable::new_constant(0.0),
            },
        )],
        0,
        30,
    );
    assert_ne!(
        rendered, original,
        "resolved source map must affect target pixels"
    );
    assert!(rendered
        .as_chunks::<4>()
        .0
        .iter()
        .all(|pixel| pixel[3] == 255));
}
#[test]
fn compound_blur_resolves_a_real_source_layer() {
    let width = 16;
    let height = 16;
    let comp = source_map_composition(width, height);
    let original = patterned_layer(width, height);
    let mut rendered = original.clone();
    apply_layer_effects(
        Some(&comp),
        Some(1),
        &mut rendered,
        width,
        height,
        &[effect(
            "compound-blur",
            EffectType::CompoundBlur {
                source_layer: Animatable::new_constant(0.0),
                max_blur: Animatable::new_constant(4.0),
            },
        )],
        0,
        30,
    );
    assert_ne!(
        rendered, original,
        "resolved blur map must affect target pixels"
    );
    assert!(rendered
        .as_chunks::<4>()
        .0
        .iter()
        .all(|pixel| pixel[3] == 255));
}
#[test]
fn custom_shader_template_is_applied_through_cpu_effect_pipeline() {
    let original = solid_layer(8, 8, 160, 96, 64);
    let mut rendered = original.clone();
    apply_layer_effects(
        None,
        None,
        &mut rendered,
        8,
        8,
        &[effect(
            "custom-crt",
            EffectType::CustomShader {
                wgsl_source: "// CRT Scanlines Shader".to_string(),
                uniform_values: vec![1.0, 1.0],
            },
        )],
        0,
        30,
    );
    assert_ne!(rendered, original);
    assert!(rendered
        .as_chunks::<4>()
        .0
        .iter()
        .all(|pixel| pixel[3] == 255));
}
#[test]
fn test_vignette_slider_units_do_not_crush_to_black() {
    // Slider units are 0..100: intensity 45 must darken corners gently,
    // not multiply the frame by 45 (which crushed everything but the
    // exact center pixel to black).
    let mut px = solid_layer(64, 64, 128, 128, 128);
    apply_layer_effects(
        None,
        None,
        &mut px,
        64,
        64,
        &[effect(
            "vig",
            EffectType::Vignette {
                intensity: Animatable::new_constant(45.0),
                roundness: Animatable::new_constant(0.55),
                feather: Animatable::new_constant(60.0),
                color: Animatable::new_constant([0.0, 0.0, 0.0, 1.0]),
            },
        )],
        0,
        30,
    );
    let at = |x: u32, y: u32| px[((y * 64 + x) * 4) as usize];
    let center = at(32, 32);
    let corner = at(0, 0);
    assert!(
        (120..=128).contains(&center),
        "center must survive, got {}",
        center
    );
    assert!(
        (20..=115).contains(&corner),
        "corner must be darkened but not crushed, got {}",
        corner
    );
    assert!(corner < center, "vignette must darken outward");
}
#[test]
fn test_invert_flips_values() {
    let mut px = solid_layer(4, 4, 100, 150, 200);
    let before = px[1];
    apply_layer_effects(
        None,
        None,
        &mut px,
        4,
        4,
        &[effect(
            "inv",
            EffectType::Invert {
                invert_alpha: false,
            },
        )],
        0,
        30,
    );
    assert_eq!(px[0], 255 - 100);
    assert_eq!(px[1], 255 - before);
}
#[test]
fn test_posterize_reduces_levels() {
    let mut px = solid_layer(8, 8, 123, 45, 67);
    apply_layer_effects(
        None,
        None,
        &mut px,
        8,
        8,
        &[effect(
            "post",
            EffectType::Posterize {
                levels: Animatable::new_constant(2.0),
            },
        )],
        0,
        30,
    );
    // With 2 levels, each channel becomes either 0 or 255.
    for &c in &[px[0], px[1], px[2]] {
        assert!(c == 0 || c == 255, "channel {} not posterized", c);
    }
}
#[test]
fn test_threshold_binaries() {
    let mut px = solid_layer(8, 8, 123, 200, 10);
    apply_layer_effects(
        None,
        None,
        &mut px,
        8,
        8,
        &[effect(
            "thr",
            EffectType::Threshold {
                threshold: Animatable::new_constant(128.0),
            },
        )],
        0,
        30,
    );
    for p in px.as_chunks::<4>().0 {
        let luma = (p[0] as u32 * 299 + p[1] as u32 * 587 + p[2] as u32 * 114) / 1000;
        let expected = if luma >= 128 { 255 } else { 0 };
        assert_eq!(p[0], expected);
        assert_eq!(p[1], expected);
        assert_eq!(p[2], expected);
    }
}
#[test]
fn test_blur_changes_pixels() {
    // A single bright pixel in an otherwise dark buffer should spread when blurred.
    let mut px = vec![0u8; 8 * 8 * 4];
    for p in px.as_chunks_mut::<4>().0 {
        p[3] = 255;
    }
    let cx = (4 * 8 + 4) * 4;
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
            "blur",
            EffectType::GaussianBlur {
                blur_radius: Animatable::new_constant(3.0),
            },
        )],
        0,
        30,
    );
    // Center stays bright, a near neighbor should now be non-zero (spread).
    assert!(px[cx] > 0);
    let neighbor = ((4 * 8 + 5) * 4) as usize;
    assert!(px[neighbor] > 0, "blur did not spread to neighbor");
}
#[test]
fn test_disabled_effect_is_noop() {
    let mut px = solid_layer(4, 4, 100, 100, 100);
    let mut eff = effect(
        "inv",
        EffectType::Invert {
            invert_alpha: false,
        },
    );
    eff.enabled = false;
    apply_layer_effects(None, None, &mut px, 4, 4, &[eff], 0, 30);
    assert_eq!(px[0], 100);
}
#[test]
fn test_offset_shifts_content() {
    // Place a small bright patch at center of a transparent buffer, then offset.
    let mut px = vec![0u8; 8 * 8 * 4];
    for p in px.as_chunks_mut::<4>().0 {
        p[3] = 255;
    }
    // Paint pixel (4,4) red.
    let idx = ((4 * 8 + 4) * 4) as usize;
    px[idx] = 255;
    px[idx + 1] = 0;
    px[idx + 2] = 0;

    apply_layer_effects(
        None,
        None,
        &mut px,
        8,
        8,
        &[effect(
            "off",
            EffectType::Offset {
                shift_x: Animatable::new_constant(2.0),
                shift_y: Animatable::new_constant(0.0),
            },
        )],
        0,
        30,
    );
    // After toroidal shift right by 2, red pixel moved from (4,4) to (6,4).
    let new_idx = ((4 * 8 + 6) * 4) as usize;
    assert_eq!(px[new_idx], 255, "red channel at shifted position");
    // Original position should now be transparent (or at least not red).
    assert_eq!(px[idx], 0, "original position cleared after offset");
}
#[test]
fn test_vibrance_boosts_chroma_but_not_gray() {
    let mut gray = solid_layer(4, 4, 128, 128, 128);
    apply_layer_effects(
        None,
        None,
        &mut gray,
        4,
        4,
        &[effect(
            "vib",
            EffectType::Vibrance {
                amount: Animatable::new_constant(100.0),
            },
        )],
        0,
        30,
    );
    assert_eq!(
        (gray[0], gray[1], gray[2]),
        (128, 128, 128),
        "zero-chroma pixels must survive max vibrance"
    );

    let mut colored = solid_layer(4, 4, 90, 140, 200);
    let before = colored.clone();
    apply_layer_effects(
        None,
        None,
        &mut colored,
        4,
        4,
        &[effect(
            "vib",
            EffectType::Vibrance {
                amount: Animatable::new_constant(80.0),
            },
        )],
        0,
        30,
    );
    assert_ne!(
        &colored[0..3],
        &before[0..3],
        "chroma must shift under vibrance"
    );
}
