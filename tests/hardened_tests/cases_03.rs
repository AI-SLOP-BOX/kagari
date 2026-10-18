#[test]
fn blend_mode_all_variants_roundtrip() {
    let modes = vec![
        BlendMode::Normal,
        BlendMode::Multiply,
        BlendMode::Screen,
        BlendMode::Overlay,
        BlendMode::Add,
        BlendMode::Darken,
        BlendMode::Lighten,
        BlendMode::SoftLight,
        BlendMode::HardLight,
        BlendMode::Difference,
        BlendMode::Exclusion,
        BlendMode::Divide,
        BlendMode::Subtract,
        BlendMode::ColorBurn,
        BlendMode::LinearBurn,
        BlendMode::VividLight,
        BlendMode::ColorDodge,
        BlendMode::LinearDodge,
        BlendMode::Color,
        BlendMode::Hue,
        BlendMode::Saturation,
        BlendMode::Luminosity,
        BlendMode::StencilAlpha,
        BlendMode::StencilLuma,
        BlendMode::SilhouetteAlpha,
        BlendMode::SilhouetteLuma,
        BlendMode::Behind,
        BlendMode::AlphaAdd,
        BlendMode::LinearLight,
    ];
    for mode in &modes {
        let json = serde_json::to_string(mode).unwrap();
        let loaded: BlendMode = serde_json::from_str(&json).unwrap();
        assert_eq!(format!("{mode:?}"), format!("{loaded:?}"));
    }
}
#[test]
fn track_matte_all_variants_roundtrip() {
    let modes = vec![
        TrackMatteMode::None,
        TrackMatteMode::AlphaMatte,
        TrackMatteMode::AlphaMatteInverted,
        TrackMatteMode::LumaMatte,
        TrackMatteMode::LumaMatteInverted,
    ];
    for mode in &modes {
        let json = serde_json::to_string(mode).unwrap();
        let loaded: TrackMatteMode = serde_json::from_str(&json).unwrap();
        assert_eq!(format!("{mode:?}"), format!("{loaded:?}"));
    }
}
#[test]
fn layer_type_solid_roundtrip() {
    let lt = LayerType::Solid {
        color: [1.0, 0.5, 0.25, 1.0],
    };
    let json = serde_json::to_string(&lt).unwrap();
    let loaded: LayerType = serde_json::from_str(&json).unwrap();
    assert_eq!(format!("{lt:?}"), format!("{loaded:?}"));
}
#[test]
fn layer_type_text_roundtrip() {
    let lt = LayerType::Text {
        text: "Hello".into(),
        font_size: 48,
        color: [1.0, 1.0, 1.0, 1.0],
        font_family: "Arial".into(),
        tracking: 0.0,
        leading: 1.2,
        align: 0,
        stroke_color: [0.0, 0.0, 0.0, 1.0],
        stroke_width: 0.0,
        text_on_path: false,
    };
    let json = serde_json::to_string(&lt).unwrap();
    let loaded: LayerType = serde_json::from_str(&json).unwrap();
    assert_eq!(format!("{lt:?}"), format!("{loaded:?}"));
}
#[test]
fn layer_type_precomp_roundtrip() {
    let lt = LayerType::PreComp {
        comp_id: "abc".into(),
    };
    let json = serde_json::to_string(&lt).unwrap();
    let loaded: LayerType = serde_json::from_str(&json).unwrap();
    assert_eq!(format!("{lt:?}"), format!("{loaded:?}"));
}
#[test]
fn layer_visibility_flag_controls_rendering() {
    let mut comp = Composition::new("t".into(), "T".into(), 64, 64, 30, 100);
    comp.layers.push(Layer::new(
        "layer".into(),
        "Layer".into(),
        LayerType::Solid {
            color: [1.0, 0.0, 0.0, 1.0],
        },
        100,
    ));

    // Visible layer renders red
    let p = software_renderer::render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
    let red_sum: u64 = p.chunks(4).map(|c| c[0] as u64).sum();
    assert!(red_sum > 0, "Visible layer should render red");

    // Hidden layer must not render red
    comp.layers[0].visible = false;
    let p = software_renderer::render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
    let red_sum: u64 = p.chunks(4).map(|c| c[0] as u64).sum();
    let expected_bg: u64 = (0.05 * 255.0) as u64 * 64 * 64;
    assert!(
        red_sum <= expected_bg + 10000,
        "Hidden layer must not render red (got red_sum={red_sum}, expected_bg~{expected_bg})"
    );
}
#[test]
fn layer_opacity_proportional() {
    let mut c100 = Composition::new("a".into(), "A".into(), 4, 4, 30, 1);
    let mut l100 = Layer::new(
        "l".into(),
        "L".into(),
        LayerType::Solid {
            color: [1.0, 0.0, 0.0, 1.0],
        },
        1,
    );
    l100.transform.opacity = Animatable::new_constant(100.0);
    c100.layers.push(l100);

    let mut c50 = Composition::new("b".into(), "B".into(), 4, 4, 30, 1);
    let mut l50 = Layer::new(
        "l".into(),
        "L".into(),
        LayerType::Solid {
            color: [1.0, 0.0, 0.0, 1.0],
        },
        1,
    );
    l50.transform.opacity = Animatable::new_constant(50.0);
    c50.layers.push(l50);

    let px100 = software_renderer::render_frame_to_pixels(&c100, 0, 4, 4, 0.0, 0);
    let px50 = software_renderer::render_frame_to_pixels(&c50, 0, 4, 4, 0.0, 0);

    let red_100: u64 = px100.chunks(4).map(|c| c[0] as u64).sum();
    let red_50: u64 = px50.chunks(4).map(|c| c[0] as u64).sum();
    assert!(
        red_100 > red_50,
        "100% opacity (red={red_100}) should be brighter than 50% (red={red_50})"
    );
}
#[test]
fn all_blend_modes_render_safely() {
    let modes = vec![
        BlendMode::Normal,
        BlendMode::Multiply,
        BlendMode::Screen,
        BlendMode::Overlay,
        BlendMode::Add,
        BlendMode::Darken,
        BlendMode::Lighten,
        BlendMode::SoftLight,
        BlendMode::HardLight,
        BlendMode::Difference,
        BlendMode::Exclusion,
    ];

    for mode in modes {
        let mut comp = Composition::new("t".into(), "T".into(), 16, 16, 30, 1);
        let mut layer = Layer::new(
            "l".into(),
            "L".into(),
            LayerType::Solid {
                color: [1.0, 0.5, 0.25, 1.0],
            },
            1,
        );
        layer.blend_mode = mode;
        comp.layers.push(layer);

        let p = software_renderer::render_frame_to_pixels(&comp, 0, 16, 16, 0.0, 0);
        assert_eq!(p.len(), 16 * 16 * 4, "Blend mode {mode:?} wrong size");
    }
}
#[test]
fn all_track_matte_modes_render_safely() {
    let modes = vec![
        TrackMatteMode::None,
        TrackMatteMode::AlphaMatte,
        TrackMatteMode::AlphaMatteInverted,
        TrackMatteMode::LumaMatte,
        TrackMatteMode::LumaMatteInverted,
    ];

    for mode in modes {
        let mut comp = Composition::new("t".into(), "T".into(), 16, 16, 30, 1);
        comp.layers.push(Layer::new(
            "matte".into(),
            "Matte".into(),
            LayerType::Solid {
                color: [1.0, 1.0, 1.0, 1.0],
            },
            1,
        ));
        let mut content = Layer::new(
            "content".into(),
            "Content".into(),
            LayerType::Solid {
                color: [1.0, 0.0, 0.0, 1.0],
            },
            1,
        );
        content.track_matte = mode;
        comp.layers.push(content);

        let p = software_renderer::render_frame_to_pixels(&comp, 0, 16, 16, 0.0, 0);
        assert_eq!(p.len(), 16 * 16 * 4, "Track matte {mode:?} wrong size");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// §7  Keyframe Interpolation Correctness
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn keyframe_linear_interpolation_exact() {
    let kfs = vec![
        kagari_vfx::core::keyframe::Keyframe {
            frame: 0,
            value: 0.0,
            interpolation: InterpolationType::Linear,
        },
        kagari_vfx::core::keyframe::Keyframe {
            frame: 10,
            value: 100.0,
            interpolation: InterpolationType::Linear,
        },
    ];
    let prop = Animatable::Animated(kfs);

    let v0 = prop.evaluate(0);
    let v5 = prop.evaluate(5);
    let v10 = prop.evaluate(10);

    assert!((v0 - 0.0).abs() < 0.001, "v0 = {v0}");
    assert!((v5 - 50.0).abs() < 0.001, "v5 = {v5}");
    assert!((v10 - 100.0).abs() < 0.001, "v10 = {v10}");
}
#[test]
fn keyframe_hold_interpolation_exact() {
    let kfs = vec![
        kagari_vfx::core::keyframe::Keyframe {
            frame: 0,
            value: 10.0,
            interpolation: InterpolationType::Hold,
        },
        kagari_vfx::core::keyframe::Keyframe {
            frame: 10,
            value: 20.0,
            interpolation: InterpolationType::Hold,
        },
    ];
    let prop = Animatable::Animated(kfs);

    assert!((prop.evaluate(0) - 10.0).abs() < 0.001);
    assert!((prop.evaluate(5) - 10.0).abs() < 0.001);
    assert!((prop.evaluate(9) - 10.0).abs() < 0.001);
    assert!((prop.evaluate(10) - 20.0).abs() < 0.001);
    assert!((prop.evaluate(15) - 20.0).abs() < 0.001);
}
#[test]
fn property_constant_evaluate() {
    let prop = Animatable::new_constant(42.0);
    assert!((prop.evaluate(0) - 42.0).abs() < 0.001);
    assert!((prop.evaluate(100) - 42.0).abs() < 0.001);
}

// ─────────────────────────────────────────────────────────────────────────────
// §8  Precomp & Nesting Safety
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn precomp_self_reference_terminates() {
    let mut comp = Composition::new("self_ref".into(), "Self Ref".into(), 32, 32, 30, 1);
    comp.layers.push(Layer::new(
        "precomp".into(),
        "Precomp".into(),
        LayerType::PreComp {
            comp_id: "self_ref".into(),
        },
        1,
    ));
    let p = software_renderer::render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);
    assert_eq!(p.len(), 32 * 32 * 4);
}
#[test]
fn precomp_deep_nesting_terminates() {
    let mut comps: Vec<Composition> = (0..30)
        .map(|i| Composition::new(format!("comp_{i}"), format!("Comp {i}"), 16, 16, 30, 1))
        .collect();
    for (i, comp) in comps.iter_mut().take(29).enumerate() {
        comp.layers.push(Layer::new(
            format!("ref_{i}"),
            format!("Ref {i}"),
            LayerType::PreComp {
                comp_id: format!("comp_{}", i + 1),
            },
            1,
        ));
    }
    let p = software_renderer::render_frame_to_pixels(&comps[0], 0, 16, 16, 0.0, 0);
    assert_eq!(p.len(), 16 * 16 * 4);
}
