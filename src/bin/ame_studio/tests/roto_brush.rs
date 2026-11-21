use super::*;

#[test]
fn roto_brush_pointer_strokes_are_persisted_and_undoable() {
    let mut app = AfterEffectsStudio::new().unwrap();
    let layer_id = "roto_test_layer".to_owned();
    {
        let composition = app.project_model.active_composition_mut();
        composition
            .layers
            .push(kagari_vfx::core::timeline::Layer::new(
                layer_id.clone(),
                "Roto test source".to_owned(),
                kagari_vfx::core::timeline::LayerType::Image {
                    path: "missing-test-image.png".to_owned(),
                },
                composition.duration_frames,
            ));
    }
    app.mark_project_changed();
    app.selected_layer_id = Some(layer_id.clone());
    app.selected_tool = "RotoBrush";
    app.viewport_mapping = Some((0.0, 0.0, 1.0, 1.0));

    let pointer = |phase, x, y| ame_ui::CanvasPointerEvent {
        phase,
        kind: ame_ui::CanvasPointerKind::Mouse,
        pointer_id: None,
        click_count: 1,
        x,
        y,
        button: Some(ame_ui::event::PointerButton::Primary),
        modifiers: ame_ui::event::KeyModifiers::default(),
        pressure: None,
        tilt: None,
        coalesced_samples: Vec::new(),
        space_pressed: false,
    };
    app.on_canvas_pointer_event(
        "kagari.viewport",
        pointer(ame_ui::CanvasPointerPhase::Down, 30.0, 40.0),
    );
    app.on_canvas_pointer_event(
        "kagari.viewport",
        pointer(ame_ui::CanvasPointerPhase::Moved, 42.0, 50.0),
    );
    app.on_canvas_pointer_event(
        "kagari.viewport",
        pointer(ame_ui::CanvasPointerPhase::Up, 48.0, 54.0),
    );

    let strokes = &app
        .project_model
        .active_composition()
        .layers
        .iter()
        .find(|layer| layer.id == layer_id)
        .unwrap()
        .roto_brush_strokes;
    assert_eq!(strokes.len(), 1);
    assert_eq!(strokes[0].frame, Some(app.current_frame));
    assert!(strokes[0].points.len() > 3);
    assert!(app.roto_live_stroke.is_none());

    app.undo_project();
    let layer = app
        .project_model
        .active_composition()
        .layers
        .iter()
        .find(|layer| layer.id == layer_id)
        .unwrap();
    assert!(layer.roto_brush_strokes.is_empty());

    app.redo_project();
    let layer = app
        .project_model
        .active_composition()
        .layers
        .iter()
        .find(|layer| layer.id == layer_id)
        .unwrap();
    assert_eq!(layer.roto_brush_strokes.len(), 1);
}

#[test]
fn roto_brush_persists_interpolated_stylus_pressure_and_coalesced_samples() {
    let mut app = AfterEffectsStudio::new().unwrap();
    let layer_id = "roto_pressure_test_layer".to_owned();
    let duration = app.project_model.active_composition().duration_frames;
    app.project_model
        .active_composition_mut()
        .layers
        .push(kagari_vfx::core::timeline::Layer::new(
            layer_id.clone(),
            "Roto pressure test source".to_owned(),
            kagari_vfx::core::timeline::LayerType::Image {
                path: "missing-pressure-test-image.png".to_owned(),
            },
            duration,
        ));
    app.mark_project_changed();
    app.selected_layer_id = Some(layer_id.clone());
    app.selected_tool = "RotoBrush";
    app.viewport_mapping = Some((0.0, 0.0, 1.0, 1.0));

    let event = |phase, x, y, pressure, coalesced_samples| ame_ui::CanvasPointerEvent {
        phase,
        kind: ame_ui::CanvasPointerKind::Stylus,
        pointer_id: Some(17),
        click_count: 1,
        x,
        y,
        button: Some(ame_ui::event::PointerButton::Primary),
        modifiers: ame_ui::event::KeyModifiers::default(),
        pressure,
        tilt: Some([0.05, -0.1]),
        coalesced_samples,
        space_pressed: false,
    };

    app.on_canvas_pointer_event(
        "kagari.viewport",
        event(
            ame_ui::CanvasPointerPhase::Down,
            30.0,
            40.0,
            Some(0.25),
            vec![ame_ui::CanvasPointerSample {
                x: 29.0,
                y: 40.0,
                pressure: Some(0.15),
                tilt: None,
            }],
        ),
    );
    app.on_canvas_pointer_event(
        "kagari.viewport",
        event(
            ame_ui::CanvasPointerPhase::Moved,
            50.0,
            40.0,
            Some(0.9),
            vec![
                ame_ui::CanvasPointerSample {
                    x: 35.0,
                    y: 40.0,
                    pressure: Some(0.4),
                    tilt: None,
                },
                ame_ui::CanvasPointerSample {
                    x: 42.0,
                    y: 40.0,
                    pressure: Some(0.7),
                    tilt: None,
                },
            ],
        ),
    );
    let mut scene = vello::Scene::new();
    app.draw_viewport(&mut scene, 0.0, 0.0, 800.0, 600.0);
    assert!(!scene.encoding().is_empty());
    app.on_canvas_pointer_event(
        "kagari.viewport",
        event(
            ame_ui::CanvasPointerPhase::Up,
            51.0,
            40.0,
            Some(0.8),
            Vec::new(),
        ),
    );

    let stroke = &app
        .project_model
        .active_composition()
        .layers
        .iter()
        .find(|layer| layer.id == layer_id)
        .unwrap()
        .roto_brush_strokes[0];
    assert_eq!(stroke.pressures.len(), stroke.points.len());
    assert!((stroke.pressures[0] - 0.15).abs() < 1e-6);
    assert!((stroke.pressures.last().copied().unwrap() - 0.8).abs() < 1e-6);
    assert!(stroke
        .pressures
        .iter()
        .any(|pressure| (*pressure - 0.4).abs() < 1e-6));
    assert!(stroke
        .pressures
        .iter()
        .any(|pressure| (*pressure - 0.7).abs() < 1e-6));
    let encoded = serde_json::to_string(stroke).expect("pressure stroke should serialize");
    let restored: kagari_vfx::core::roto_brush_engine::RotoStroke =
        serde_json::from_str(&encoded).expect("pressure stroke should deserialize");
    assert_eq!(restored.pressures, stroke.pressures);
}

#[test]
fn roto_brush_discards_live_stroke_when_composition_resizes() {
    let mut app = AfterEffectsStudio::new().unwrap();
    let layer_id = "roto_resize_test_layer".to_owned();
    let old_width = app.project_model.active_composition().width;
    let duration = app.project_model.active_composition().duration_frames;
    app.project_model
        .active_composition_mut()
        .layers
        .push(kagari_vfx::core::timeline::Layer::new(
            layer_id.clone(),
            "Roto resize test source".to_owned(),
            kagari_vfx::core::timeline::LayerType::Image {
                path: "missing-test-image.png".to_owned(),
            },
            duration,
        ));
    app.mark_project_changed();
    app.selected_layer_id = Some(layer_id.clone());
    app.selected_tool = "RotoBrush";
    app.viewport_mapping = Some((0.0, 0.0, 1.0, 1.0));
    let pointer = |phase, x, y| ame_ui::CanvasPointerEvent {
        phase,
        kind: ame_ui::CanvasPointerKind::Mouse,
        pointer_id: None,
        click_count: 1,
        x,
        y,
        button: Some(ame_ui::event::PointerButton::Primary),
        modifiers: ame_ui::event::KeyModifiers::default(),
        pressure: None,
        tilt: None,
        coalesced_samples: Vec::new(),
        space_pressed: false,
    };

    app.on_canvas_pointer_event(
        "kagari.viewport",
        pointer(ame_ui::CanvasPointerPhase::Down, 30.0, 40.0),
    );
    assert!(app.roto_live_stroke.is_some());
    app.project_model.active_composition_mut().width = old_width / 2;
    app.on_canvas_pointer_event(
        "kagari.viewport",
        pointer(ame_ui::CanvasPointerPhase::Moved, 42.0, 50.0),
    );
    assert!(app.roto_live_stroke.is_none());
    app.on_canvas_pointer_event(
        "kagari.viewport",
        pointer(ame_ui::CanvasPointerPhase::Up, 48.0, 54.0),
    );
    let layer = app
        .project_model
        .active_composition()
        .layers
        .iter()
        .find(|layer| layer.id == layer_id)
        .unwrap();
    assert!(layer.roto_brush_strokes.is_empty());
    assert!(app.roto_status.contains("設定が変わった"));
}

#[test]
fn roto_clear_preserves_user_mask_with_generated_matte_name() {
    let mut app = AfterEffectsStudio::new().unwrap();
    let layer_id = "roto_clear_test_layer".to_owned();
    let duration = app.project_model.active_composition().duration_frames;
    let layer = kagari_vfx::core::timeline::Layer::new(
        layer_id.clone(),
        "Roto clear test source".to_owned(),
        kagari_vfx::core::timeline::LayerType::Image {
            path: "missing-test-image.png".to_owned(),
        },
        duration,
    );
    let mut layer = layer;
    let polygon = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]];
    layer.masks.push(kagari_vfx::core::mask::Mask::new_closed(
        "user_mask_id".to_owned(),
        "Roto Brush Matte".to_owned(),
        polygon.clone(),
    ));
    layer.masks.push(kagari_vfx::core::mask::Mask::new_closed(
        kagari_vfx::core::roto_assist::ROTO_BRUSH_MATTE_ID.to_owned(),
        "Roto Brush Matte".to_owned(),
        polygon,
    ));
    app.project_model
        .active_composition_mut()
        .layers
        .push(layer);
    app.selected_layer_id = Some(layer_id.clone());

    app.clear_roto_brush();

    let layer = app
        .project_model
        .active_composition()
        .layers
        .iter()
        .find(|layer| layer.id == layer_id)
        .unwrap();
    assert_eq!(layer.masks.len(), 1);
    assert_eq!(layer.masks[0].id, "user_mask_id");
    assert_eq!(layer.masks[0].name, "Roto Brush Matte");
}
