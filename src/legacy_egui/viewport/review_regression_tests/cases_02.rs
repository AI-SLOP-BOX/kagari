#[test]
fn roto_brush_stroke_propagation_survives_project_roundtrip() {
    let mut app = KagariApp::default();
    let mut project = app.history.current().clone();
    let comp = project.active_composition_mut();
    comp.width = 32;
    comp.height = 32;
    comp.duration_frames = 3;
    comp.layers.clear();
    let mut layer = crate::core::timeline::Layer::new(
        "roto-target".into(),
        "Roto target".into(),
        crate::core::timeline::LayerType::Solid {
            color: [0.8, 0.7, 0.6, 1.0],
        },
        3,
    );
    layer.out_frame = 2;
    comp.layers.push(layer);
    app.commit_project(project);

    let ctx = egui::Context::default();
    let stroke = EngineRotoStroke {
        stroke_type: RotoStrokeType::Foreground,
        points: vec![[8.0, 16.0], [24.0, 16.0]],
        radius: 4.0,
        frame: Some(1),
    };
    start_roto_segment_job(
        &app,
        &ctx,
        RotoSegmentRequest {
            layer_index: 0,
            frame: 1,
            previous_strokes: Vec::new(),
            strokes: vec![stroke],
            settings: RotoBrushSettings {
                feather_radius: 0.0,
                ..Default::default()
            },
            feather: 0.0,
        },
    )
    .unwrap();

    let started = std::time::Instant::now();
    while ctx
        .data(|data| {
            data.get_temp::<std::sync::Arc<RotoSegmentJob>>(egui::Id::new("roto_segment_job"))
        })
        .is_some()
    {
        poll_roto_segment_job(&mut app, &ctx);
        assert!(started.elapsed() < std::time::Duration::from_secs(5));
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    start_roto_flow_bake(&app, &ctx, 0, 1).unwrap();
    let started = std::time::Instant::now();
    while ctx
        .data(|data| {
            data.get_temp::<std::sync::Arc<RotoFlowJob>>(egui::Id::new("roto_flow_bake_job"))
        })
        .is_some()
    {
        poll_roto_flow_bake(&mut app, &ctx);
        assert!(started.elapsed() < std::time::Duration::from_secs(5));
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    let saved = serde_json::to_vec(app.history.current()).unwrap();
    let restored: crate::core::timeline::Project = serde_json::from_slice(&saved).unwrap();
    let restored_layer = &restored.active_composition().layers[0];
    assert_eq!(restored_layer.roto_brush_strokes.len(), 1);
    let matte = restored_layer
        .masks
        .iter()
        .find(|mask| mask.name == "Roto Brush Matte")
        .expect("generated roto matte should survive project save/reload");
    let Animatable::Animated(keyframes) = &matte.path.vertices else {
        panic!("propagated matte should contain per-frame keyframes");
    };
    assert_eq!(
        keyframes
            .iter()
            .map(|keyframe| keyframe.frame)
            .collect::<Vec<_>>(),
        [0, 1, 2]
    );
}
#[test]
fn semantically_incomplete_effect_uses_software_preview() {
    let mut comp = crate::core::timeline::Composition::new(
        "comp".to_string(),
        "Comp".to_string(),
        320,
        180,
        30,
        30,
    );
    let mut layer = crate::core::timeline::Layer::new(
        "layer".to_string(),
        "Layer".to_string(),
        crate::core::timeline::LayerType::Solid {
            color: [1.0, 1.0, 1.0, 1.0],
        },
        30,
    );
    layer.effects.push(crate::core::timeline::Effect {
        id: "hue-saturation".to_string(),
        name: "Hue/Saturation".to_string(),
        effect_type: crate::core::timeline::EffectType::HueSaturation {
            hue_shift: crate::core::property::Animatable::new_constant(45.0),
            saturation: crate::core::property::Animatable::new_constant(20.0),
            lightness: crate::core::property::Animatable::new_constant(0.0),
        },
        enabled: true,
    });
    comp.layers.push(layer);
    assert!(software_preview_required(&comp, 0));
}
#[test]
fn unsupported_shape_geometry_uses_software_preview() {
    let mut comp = crate::core::timeline::Composition::new(
        "comp".to_string(),
        "Comp".to_string(),
        320,
        180,
        30,
        30,
    );
    let mut layer = crate::core::timeline::Layer::new(
        "shape".to_string(),
        "Freeform".to_string(),
        crate::core::timeline::LayerType::Shape {
            shape_type: crate::core::timeline::ShapeType::FreeformBezier {
                points: vec![[0.0, 0.0], [20.0, 0.0], [10.0, 20.0]],
                tangents: vec![([0.0, 0.0], [0.0, 0.0]); 3],
                closed: true,
            },
            color: [1.0, 0.0, 0.0, 1.0],
            stroke_color: [0.0, 0.0, 0.0, 0.0],
            stroke_width: 0.0,
            fill_type: Default::default(),
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        },
        30,
    );
    comp.layers.push(layer.clone());
    assert!(software_preview_required(&comp, 0));

    layer.layer_type = crate::core::timeline::LayerType::Shape {
        shape_type: crate::core::timeline::ShapeType::Rectangle {
            width: crate::core::property::Animatable::new_constant(20.0),
            height: crate::core::property::Animatable::new_constant(20.0),
            corner_radius: crate::core::property::Animatable::new_constant(0.0),
        },
        color: [1.0, 0.0, 0.0, 1.0],
        stroke_color: [0.0, 0.0, 0.0, 0.0],
        stroke_width: 0.0,
        fill_type: Default::default(),
        extrusion_depth: 2.0,
        bevel_depth: 0.0,
    };
    comp.layers[0] = layer;
    assert!(software_preview_required(&comp, 0));

    if let crate::core::timeline::LayerType::Shape { stroke_width, .. } =
        &mut comp.layers[0].layer_type
    {
        *stroke_width = 3.0;
    }
    if let crate::core::timeline::LayerType::Shape {
        extrusion_depth, ..
    } = &mut comp.layers[0].layer_type
    {
        *extrusion_depth = 0.0;
    }
    assert!(software_preview_required(&comp, 0));

    let mut blend_comp = comp.clone();
    if let crate::core::timeline::LayerType::Shape { stroke_width, .. } =
        &mut blend_comp.layers[0].layer_type
    {
        *stroke_width = 0.0;
    }
    if let crate::core::timeline::LayerType::Shape {
        extrusion_depth, ..
    } = &mut blend_comp.layers[0].layer_type
    {
        *extrusion_depth = 0.0;
    }
    blend_comp.layers[0].blend_mode = crate::core::timeline::BlendMode::Difference;
    assert!(software_preview_required(&blend_comp, 0));
    blend_comp.layers[0].blend_mode = crate::core::timeline::BlendMode::Normal;
    assert!(!software_preview_required(&blend_comp, 0));
}
