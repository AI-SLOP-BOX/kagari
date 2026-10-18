/// Regression: every EffectType variant must serialize without panic.
#[test]
fn effect_type_all_variants_serialize() {
    let effects = vec![
        EffectType::GaussianBlur {
            blur_radius: Animatable::new_constant(5.0),
        },
        EffectType::ColorTint {
            color: Animatable::new_constant([1.0, 0.0, 0.0, 1.0]),
            intensity: Animatable::new_constant(0.5),
        },
        EffectType::Glow {
            threshold: Animatable::new_constant(0.5),
            radius: Animatable::new_constant(20.0),
            intensity: Animatable::new_constant(1.0),
            color: Animatable::new_constant([1.0, 1.0, 1.0, 1.0]),
        },
        EffectType::Twirl {
            angle: Animatable::new_constant(45.0),
            radius: Animatable::new_constant(100.0),
        },
        EffectType::Bulge {
            amount: Animatable::new_constant(0.5),
            radius: Animatable::new_constant(100.0),
        },
        EffectType::Sharpen {
            amount: Animatable::new_constant(50.0),
        },
        EffectType::Threshold {
            threshold: Animatable::new_constant(128.0),
        },
        EffectType::Invert { invert_alpha: true },
        EffectType::FindEdges { invert: false },
        EffectType::RadialBlur {
            amount: Animatable::new_constant(10.0),
        },
        EffectType::DirectionalBlur {
            angle: Animatable::new_constant(0.0),
            length: Animatable::new_constant(10.0),
        },
    ];
    for effect in effects {
        let json = serde_json::to_string(&effect).unwrap();
        let _restored: EffectType = serde_json::from_str(&json).unwrap();
    }
}

// ─── Composition: Layer Management ──────────────────────────────────────────

/// Regression: Composition with many layers should not panic on operations.
#[test]
fn composition_many_layers_safe() {
    let mut comp = Composition::new("c1".into(), "Test".into(), 100, 100, 30, 300);
    for i in 0..100 {
        comp.add_layer(Layer::new(
            format!("l{}", i),
            format!("Layer {}", i),
            LayerType::Solid {
                color: [1.0, 0.0, 0.0, 1.0],
            },
            300,
        ));
    }
    assert_eq!(comp.layers.len(), 100);

    // All layers should be active at frame 0
    let active_count = comp.layers.iter().filter(|l| l.is_active(0)).count();
    assert_eq!(active_count, 100);
}

/// Regression: Layer with in_frame > out_frame should not be active.
#[test]
fn layer_in_gt_out_not_active() {
    let mut layer = Layer::new(
        "l1".into(),
        "Test".into(),
        LayerType::Solid {
            color: [1.0, 1.0, 1.0, 1.0],
        },
        100,
    );
    layer.in_frame = 50;
    layer.out_frame = 30; // in > out
    assert!(!layer.is_active(40));
    assert!(!layer.is_active(0));
    assert!(!layer.is_active(100));
}

// ─── Project: Default Value Invariants ──────────────────────────────────────

/// Regression: Project default must have at least one composition.
#[test]
fn project_default_has_composition() {
    let project = Project::default();
    assert!(
        !project.compositions.is_empty(),
        "Project::default() must have at least one composition"
    );
}

/// Regression: Composition default fps must be > 0.
#[test]
fn composition_default_fps_positive() {
    let comp = Composition::new("c".into(), "C".into(), 1920, 1080, 30, 100);
    assert!(comp.fps > 0);
}

/// Regression: Composition default dimensions must be > 0.
#[test]
fn composition_dimensions_positive() {
    let comp = Composition::new("c".into(), "C".into(), 1920, 1080, 30, 100);
    assert!(comp.width > 0);
    assert!(comp.height > 0);
}

// ─── MaterialOptions: Invariants ────────────────────────────────────────────

/// Regression: MaterialOptions ambient/diffuse/specular must be in [0,1].
#[test]
fn material_options_values_in_range() {
    let mat = MaterialOptions::default();
    assert!(mat.ambient >= 0.0 && mat.ambient <= 1.0);
    assert!(mat.diffuse >= 0.0 && mat.diffuse <= 1.0);
    assert!(mat.specular >= 0.0 && mat.specular <= 1.0);
    assert!(mat.emission >= 0.0 && mat.emission <= 1.0);
    assert!(mat.metalness >= 0.0 && mat.metalness <= 1.0);
}
