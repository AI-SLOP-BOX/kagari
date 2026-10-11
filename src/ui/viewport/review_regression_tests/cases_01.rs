#[test]
fn multi_vertex_drag_does_not_accumulate_previous_frames() {
    let mut path = crate::core::mask::MaskPath::new_rect(100.0, 200.0, 50.0, 60.0);
    let original = path.vertices_at_frame(0);
    let starts = [(0, original[0]), (1, original[1])];
    move_mask_vertices(&mut path, 0, &starts, [10.0, 5.0]);
    move_mask_vertices(&mut path, 0, &starts, [20.0, 15.0]);
    let actual = path.vertices_at_frame(0);
    assert_eq!(actual[0], [120.0, 215.0]);
    assert_eq!(actual[1], [170.0, 215.0]);
    assert_eq!(&actual[2..], &original[2..]);
}
#[test]
fn tangent_drag_is_relative_and_preserves_unlinked_handle() {
    let mut path = crate::core::mask::MaskPath::new_rect(100.0, 200.0, 50.0, 60.0);
    path.set_tangents_at_vertex(0, [-10.0, -5.0], [20.0, 15.0], false);
    move_mask_tangent(&mut path, 0, true, [25.0, 20.0], false);
    assert_eq!(
        path.tangents.as_ref().unwrap()[0],
        ([-10.0, -5.0], [25.0, 20.0])
    );
    move_mask_tangent(&mut path, 0, false, [-15.0, -7.0], false);
    assert_eq!(
        path.tangents.as_ref().unwrap()[0],
        ([-15.0, -7.0], [25.0, 20.0])
    );
    move_mask_tangent(&mut path, 0, true, [30.0, 40.0], true);
    assert_eq!(
        path.tangents.as_ref().unwrap()[0],
        ([-30.0, -40.0], [30.0, 40.0])
    );
}
#[test]
fn pen_drag_release_records_start_vertex_and_correct_curve_direction() {
    let ctx = egui::Context::default();
    let mut app = KagariApp::default();
    let start = egui::pos2(100.0, 100.0);
    let end = egui::pos2(130.0, 120.0);
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    for events in [
        vec![],
        vec![egui::Event::PointerMoved(start), button(start, true)],
        vec![egui::Event::PointerMoved(end)],
        vec![button(end, false)],
    ] {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(400.0, 400.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let response = ui.interact(
                        egui::Rect::from_min_max(egui::pos2(20.0, 20.0), egui::pos2(300.0, 300.0)),
                        egui::Id::new("pen_test"),
                        egui::Sense::click_and_drag(),
                    );
                    handle_pen_pointer(&mut app, &response, [20.0, 20.0], [2.0, 2.0]);
                });
            },
        );
    }
    assert_eq!(app.pen_points, vec![[160.0, 160.0]]);
    assert!(app.pen_bezier_drag.is_none());
    let path = pen_path(app.pen_points, app.pen_tangents);
    assert_eq!(path.tangents.unwrap()[0], ([-60.0, -40.0], [60.0, 40.0]));
}
#[test]
fn multiple_gpu_effects_use_software_preview_to_preserve_stack_order() {
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
        id: "invert".to_string(),
        name: "Invert".to_string(),
        effect_type: crate::core::timeline::EffectType::Invert {
            invert_alpha: false,
        },
        enabled: true,
    });
    comp.layers.push(layer);
    assert!(!software_preview_required(&comp, 0));

    comp.layers[0].effects.push(crate::core::timeline::Effect {
        id: "posterize".to_string(),
        name: "Posterize".to_string(),
        effect_type: crate::core::timeline::EffectType::Posterize {
            levels: crate::core::property::Animatable::new_constant(8.0),
        },
        enabled: true,
    });
    assert!(software_preview_required(&comp, 0));
}
#[test]
fn puppet_deformation_uses_software_preview_until_gpu_warp_is_supported() {
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
    layer
        .puppet_pins
        .push(crate::core::timeline::PuppetPin::new(
            "pin".to_string(),
            "Pin 1".to_string(),
            [160.0, 90.0],
        ));
    comp.layers.push(layer);

    assert!(software_preview_required(&comp, 0));
}
#[test]
fn unsupported_gpu_mask_geometry_uses_software_preview() {
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
    layer.masks.push(crate::core::mask::Mask::new_rect(
        "mask".to_string(),
        "Mask".to_string(),
        20.0,
        20.0,
        100.0,
        80.0,
    ));
    comp.layers.push(layer);
    assert!(!software_preview_required(&comp, 0));

    comp.layers[0].masks[0].expansion = crate::core::property::Animatable::new_constant(4.0);
    assert!(software_preview_required(&comp, 0));

    comp.layers[0].masks[0].mode = crate::core::mask::MaskMode::None;
    assert!(!software_preview_required(&comp, 0));

    comp.layers[0].masks[0].mode = crate::core::mask::MaskMode::Add;
    comp.layers[0].masks[0].expansion = crate::core::property::Animatable::new_constant(0.0);
    comp.layers[0].masks[0].wiggle = Some(crate::core::wiggle_paths::WigglePathsOptions::default());
    assert!(software_preview_required(&comp, 0));
}
#[test]
fn roto_source_frame_isolated_from_other_layers_background_and_existing_masks() {
    let mut comp = crate::core::timeline::Composition::new(
        "roto-source".to_string(),
        "Roto source".to_string(),
        32,
        32,
        30,
        30,
    );
    comp.background_color = [0.0, 1.0, 0.0, 1.0];
    let mut target = crate::core::timeline::Layer::new(
        "target".to_string(),
        "Target".to_string(),
        crate::core::timeline::LayerType::Solid {
            color: [1.0, 0.0, 0.0, 1.0],
        },
        30,
    );
    target.masks.push(crate::core::mask::Mask::new_rect(
        "existing".to_string(),
        "Existing mask".to_string(),
        0.0,
        0.0,
        2.0,
        2.0,
    ));
    target.track_matte = crate::core::timeline::TrackMatteMode::AlphaMatte;
    let mut unrelated = crate::core::timeline::Layer::new(
        "unrelated".to_string(),
        "Unrelated".to_string(),
        crate::core::timeline::LayerType::Solid {
            color: [0.0, 0.0, 1.0, 1.0],
        },
        30,
    );
    unrelated.track_matte = crate::core::timeline::TrackMatteMode::LumaMatte;
    comp.layers.extend([target, unrelated]);

    let pixels = render_roto_brush_source_frame(&comp, 0, 0);
    let center = ((16 * 32 + 16) * 4) as usize;
    assert_eq!(&pixels[center..center + 4], &[255, 0, 0, 255]);
}
#[test]
fn roto_stroke_analysis_runs_async_and_commits_strokes_with_the_matte() {
    let mut app = KagariApp::default();
    let mut project = app.history.current().clone();
    let comp = project.active_composition_mut();
    comp.width = 32;
    comp.height = 32;
    comp.duration_frames = 3;
    comp.layers.clear();
    comp.layers.push(crate::core::timeline::Layer::new(
        "roto-target".into(),
        "Roto target".into(),
        crate::core::timeline::LayerType::Solid {
            color: [0.8, 0.7, 0.6, 1.0],
        },
        3,
    ));
    app.commit_project(project);
    app.selection.selected_layer_idx = Some(0);

    let frame = 1;
    let stroke = EngineRotoStroke {
        stroke_type: RotoStrokeType::Foreground,
        points: vec![[8.0, 16.0], [24.0, 16.0]],
        radius: 4.0,
        frame: Some(frame),
    };
    let ctx = egui::Context::default();
    start_roto_segment_job(
        &app,
        &ctx,
        RotoSegmentRequest {
            layer_index: 0,
            frame,
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
    loop {
        poll_roto_segment_job(&mut app, &ctx);
        if ctx
            .data(|data| {
                data.get_temp::<std::sync::Arc<RotoSegmentJob>>(egui::Id::new("roto_segment_job"))
            })
            .is_none()
        {
            break;
        }
        assert!(started.elapsed() < std::time::Duration::from_secs(5));
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    let layer = &app.history.current().active_composition().layers[0];
    assert_eq!(layer.roto_brush_strokes.len(), 1);
    let matte = layer
        .masks
        .iter()
        .find(|mask| mask.name == "Roto Brush Matte")
        .expect("background worker should commit the generated matte");
    assert!(matte.path.vertices_at_frame(frame).len() >= 3);
}
#[test]
fn roto_stroke_analysis_discards_results_if_the_source_layer_changes() {
    let mut app = KagariApp::default();
    let mut project = app.history.current().clone();
    let comp = project.active_composition_mut();
    comp.width = 32;
    comp.height = 32;
    comp.duration_frames = 3;
    comp.layers.clear();
    comp.layers.push(crate::core::timeline::Layer::new(
        "roto-target".into(),
        "Roto target".into(),
        crate::core::timeline::LayerType::Solid {
            color: [0.8, 0.7, 0.6, 1.0],
        },
        3,
    ));
    app.commit_project(project);
    app.selection.selected_layer_idx = Some(0);

    let frame = 1;
    let stroke = EngineRotoStroke {
        stroke_type: RotoStrokeType::Foreground,
        points: vec![[8.0, 16.0], [24.0, 16.0]],
        radius: 4.0,
        frame: Some(frame),
    };
    let ctx = egui::Context::default();
    start_roto_segment_job(
        &app,
        &ctx,
        RotoSegmentRequest {
            layer_index: 0,
            frame,
            previous_strokes: Vec::new(),
            strokes: vec![stroke],
            settings: RotoBrushSettings::default(),
            feather: 3.0,
        },
    )
    .unwrap();

    let mut changed_project = app.history.current().clone();
    changed_project.active_composition_mut().layers[0]
        .transform
        .position = Animatable::new_constant([3.0, 4.0]);
    app.commit_project(changed_project);

    let started = std::time::Instant::now();
    loop {
        poll_roto_segment_job(&mut app, &ctx);
        if ctx
            .data(|data| {
                data.get_temp::<std::sync::Arc<RotoSegmentJob>>(egui::Id::new("roto_segment_job"))
            })
            .is_none()
        {
            break;
        }
        assert!(started.elapsed() < std::time::Duration::from_secs(5));
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    let layer = &app.history.current().active_composition().layers[0];
    assert!(layer.roto_brush_strokes.is_empty());
    assert!(!layer
        .masks
        .iter()
        .any(|mask| mask.name == "Roto Brush Matte"));
}
#[test]
fn roto_flow_discards_bake_when_source_layer_changes() {
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
    layer.masks.push(crate::core::mask::Mask::new_rect(
        "roto_brush_matte".into(),
        "Roto Brush Matte".into(),
        8.0,
        8.0,
        16.0,
        16.0,
    ));
    comp.layers.push(layer);
    app.commit_project(project);

    let ctx = egui::Context::default();
    start_roto_flow_bake(&app, &ctx, 0, 1).unwrap();

    let mut changed_project = app.history.current().clone();
    changed_project.active_composition_mut().layers[0].layer_type =
        crate::core::timeline::LayerType::Solid {
            color: [0.1, 0.2, 0.9, 1.0],
        };
    app.commit_project(changed_project);

    let started = std::time::Instant::now();
    loop {
        poll_roto_flow_bake(&mut app, &ctx);
        if ctx
            .data(|data| {
                data.get_temp::<std::sync::Arc<RotoFlowJob>>(egui::Id::new("roto_flow_bake_job"))
            })
            .is_none()
        {
            break;
        }
        assert!(started.elapsed() < std::time::Duration::from_secs(5));
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    let mask = &app.history.current().active_composition().layers[0].masks[0];
    assert!(matches!(mask.path.vertices, Animatable::Constant(_)));
}
