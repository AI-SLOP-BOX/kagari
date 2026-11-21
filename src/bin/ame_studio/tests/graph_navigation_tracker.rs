use super::*;

#[test]
fn graph_editor_keyframe_right_click_toggles_easing_and_draws_handles() {
    let mut app = AfterEffectsStudio::new().unwrap();
    invoke(&mut app, "timeline_view_graph", None);
    app.timeline_width = 800.0;
    app.graph_canvas_height = 300.0;
    app.graph_channel = GraphChannel::PositionX;

    app.add_particle_layer();
    // Ensure a keyframe exists on Position at frame 0 and frame 30
    let layer_id = {
        let comp = app.project_model.active_composition_mut();
        let layer = &mut comp.layers[0];
        layer
            .transform
            .position
            .add_keyframe(kagari_vfx::core::keyframe::Keyframe::new(
                0,
                [100.0, 100.0],
                kagari_vfx::core::keyframe::InterpolationType::Linear,
            ));
        layer
            .transform
            .position
            .add_keyframe(kagari_vfx::core::keyframe::Keyframe::new(
                30,
                [400.0, 100.0],
                kagari_vfx::core::keyframe::InterpolationType::Linear,
            ));
        layer.id.clone()
    };
    app.selected_layer_id = Some(layer_id.clone());

    // Right-click on keyframe at frame 0 to cycle interpolation from Linear to Bezier
    let (timeline_start, visible_frames) = app.timeline_window(100);
    let plot_left = 14.0;
    let plot_right = (app.timeline_width - 14.0).max(plot_left + 1.0);
    let px_0 = plot_left + (0.0 - timeline_start) / visible_frames * (plot_right - plot_left);

    // Calculate plot_bottom and py
    let comp = app.project_model.active_composition();
    let layer = comp.layers.iter().find(|l| l.id == layer_id).unwrap();
    let keys = graph_channel_keys(layer, None, None, None, None, None, GraphChannel::PositionX);
    let (min_v, max_v) = graph_value_range(&keys);
    let plot_top = 12.0;
    let plot_bottom = app.graph_canvas_height - 12.0;
    let py_0 = plot_bottom
        - (100.0 - f64::from(min_v)) / f64::from((max_v - min_v).max(f32::EPSILON))
            * (plot_bottom - plot_top);

    let right_click_event = ame_ui::CanvasPointerEvent {
        phase: ame_ui::CanvasPointerPhase::Down,
        kind: ame_ui::CanvasPointerKind::Mouse,
        pointer_id: None,
        click_count: 1,
        x: px_0,
        y: py_0,
        button: Some(ame_ui::event::PointerButton::Secondary),
        modifiers: ame_ui::event::KeyModifiers::default(),
        pressure: None,
        tilt: None,
        coalesced_samples: Vec::new(),
        space_pressed: false,
    };

    // Trigger right-click toggle
    let handled = app.on_canvas_pointer_event("kagari.graph", right_click_event);
    assert!(handled, "Right click on keyframe should be handled");

    // Verify that interpolation changed to Bezier
    let comp = app.project_model.active_composition();
    let layer = comp.layers.iter().find(|l| l.id == layer_id).unwrap();
    let key_0 = layer
        .transform
        .position
        .keyframes()
        .unwrap()
        .iter()
        .find(|k| k.frame == 0)
        .unwrap();
    assert!(
        matches!(
            key_0.interpolation,
            kagari_vfx::core::keyframe::InterpolationType::Bezier { .. }
        ),
        "Keyframe 0 should now have Bezier easing"
    );

    // Verify draw_value_graph renders Bezier handles without errors
    let mut graph_scene = Scene::new();
    draw_value_graph(
        &mut graph_scene,
        0.0,
        0.0,
        800.0,
        300.0,
        0,
        100,
        0.0,
        100.0,
        GraphChannel::PositionX,
        Some(layer),
        None,
        None,
        None,
        None,
        None,
        &keys,
    );
    assert!(!graph_scene.encoding().is_empty());
}

#[test]
fn test_viewport_alt_camera_navigation() {
    let mut app = AfterEffectsStudio::new().unwrap();
    app.screen = Screen::Editor;
    app.viewport_mapping = Some((0.0, 0.0, 1.0, 1.0));

    let initial_pos = app
        .project_model
        .active_composition()
        .resolve_camera()
        .transform
        .position
        .evaluate(0);
    let initial_rot = app
        .project_model
        .active_composition()
        .resolve_camera()
        .transform
        .rotation
        .evaluate(0);

    // 1. Alt + Left Drag -> Orbit (Changes camera rotation yaw and pitch)
    let alt_mod = ame_ui::event::KeyModifiers {
        alt: true,
        shift: false,
        control: false,
        super_key: false,
    };
    let down_orbit = ame_ui::CanvasPointerEvent {
        phase: ame_ui::CanvasPointerPhase::Down,
        kind: ame_ui::CanvasPointerKind::Mouse,
        pointer_id: None,
        click_count: 1,
        x: 200.0,
        y: 200.0,
        button: Some(ame_ui::event::PointerButton::Primary),
        modifiers: alt_mod,
        pressure: None,
        tilt: None,
        coalesced_samples: Vec::new(),
        space_pressed: false,
    };
    assert!(app.on_canvas_pointer_event("kagari.viewport", down_orbit));
    assert!(app.camera_nav_drag.is_some());

    let move_orbit = ame_ui::CanvasPointerEvent {
        phase: ame_ui::CanvasPointerPhase::Moved,
        kind: ame_ui::CanvasPointerKind::Mouse,
        pointer_id: None,
        click_count: 1,
        x: 250.0,
        y: 230.0,
        button: Some(ame_ui::event::PointerButton::Primary),
        modifiers: alt_mod,
        pressure: None,
        tilt: None,
        coalesced_samples: Vec::new(),
        space_pressed: false,
    };
    assert!(app.on_canvas_pointer_event("kagari.viewport", move_orbit));

    let up_orbit = ame_ui::CanvasPointerEvent {
        phase: ame_ui::CanvasPointerPhase::Up,
        kind: ame_ui::CanvasPointerKind::Mouse,
        pointer_id: None,
        click_count: 1,
        x: 250.0,
        y: 230.0,
        button: Some(ame_ui::event::PointerButton::Primary),
        modifiers: alt_mod,
        pressure: None,
        tilt: None,
        coalesced_samples: Vec::new(),
        space_pressed: false,
    };
    assert!(app.on_canvas_pointer_event("kagari.viewport", up_orbit));
    assert!(app.camera_nav_drag.is_none());

    let rot_after = app
        .project_model
        .active_composition()
        .resolve_camera()
        .transform
        .rotation
        .evaluate(0);
    assert!(
        (rot_after[1] - (initial_rot[1] + 50.0 * 0.4)).abs() < 1e-3,
        "Yaw should rotate with X delta"
    );
    assert!(
        (rot_after[0] - (initial_rot[0] + 30.0 * 0.4)).abs() < 1e-3,
        "Pitch should rotate with Y delta"
    );

    // 2. Alt + Right Drag -> Pan (Changes camera X/Y position)
    let down_pan = ame_ui::CanvasPointerEvent {
        phase: ame_ui::CanvasPointerPhase::Down,
        kind: ame_ui::CanvasPointerKind::Mouse,
        pointer_id: None,
        click_count: 1,
        x: 200.0,
        y: 200.0,
        button: Some(ame_ui::event::PointerButton::Secondary),
        modifiers: alt_mod,
        pressure: None,
        tilt: None,
        coalesced_samples: Vec::new(),
        space_pressed: false,
    };
    assert!(app.on_canvas_pointer_event("kagari.viewport", down_pan));

    let move_pan = ame_ui::CanvasPointerEvent {
        phase: ame_ui::CanvasPointerPhase::Moved,
        kind: ame_ui::CanvasPointerKind::Mouse,
        pointer_id: None,
        click_count: 1,
        x: 240.0,
        y: 220.0,
        button: Some(ame_ui::event::PointerButton::Secondary),
        modifiers: alt_mod,
        pressure: None,
        tilt: None,
        coalesced_samples: Vec::new(),
        space_pressed: false,
    };
    assert!(app.on_canvas_pointer_event("kagari.viewport", move_pan));

    let up_pan = ame_ui::CanvasPointerEvent {
        phase: ame_ui::CanvasPointerPhase::Up,
        kind: ame_ui::CanvasPointerKind::Mouse,
        pointer_id: None,
        click_count: 1,
        x: 240.0,
        y: 220.0,
        button: Some(ame_ui::event::PointerButton::Secondary),
        modifiers: alt_mod,
        pressure: None,
        tilt: None,
        coalesced_samples: Vec::new(),
        space_pressed: false,
    };
    assert!(app.on_canvas_pointer_event("kagari.viewport", up_pan));

    let pos_after_pan = app
        .project_model
        .active_composition()
        .resolve_camera()
        .transform
        .position
        .evaluate(0);
    assert!(
        (pos_after_pan[0] - (initial_pos[0] - 40.0 * 1.5)).abs() < 1e-3,
        "Camera should pan along X"
    );
    assert!(
        (pos_after_pan[1] - (initial_pos[1] - 20.0 * 1.5)).abs() < 1e-3,
        "Camera should pan along Y"
    );

    // 3. Alt + Shift + Drag -> Dolly (Changes camera Z position)
    let alt_shift_mod = ame_ui::event::KeyModifiers {
        alt: true,
        shift: true,
        control: false,
        super_key: false,
    };
    let down_dolly = ame_ui::CanvasPointerEvent {
        phase: ame_ui::CanvasPointerPhase::Down,
        kind: ame_ui::CanvasPointerKind::Mouse,
        pointer_id: None,
        click_count: 1,
        x: 200.0,
        y: 200.0,
        button: Some(ame_ui::event::PointerButton::Primary),
        modifiers: alt_shift_mod,
        pressure: None,
        tilt: None,
        coalesced_samples: Vec::new(),
        space_pressed: false,
    };
    assert!(app.on_canvas_pointer_event("kagari.viewport", down_dolly));

    let move_dolly = ame_ui::CanvasPointerEvent {
        phase: ame_ui::CanvasPointerPhase::Moved,
        kind: ame_ui::CanvasPointerKind::Mouse,
        pointer_id: None,
        click_count: 1,
        x: 200.0,
        y: 250.0,
        button: Some(ame_ui::event::PointerButton::Primary),
        modifiers: alt_shift_mod,
        pressure: None,
        tilt: None,
        coalesced_samples: Vec::new(),
        space_pressed: false,
    };
    assert!(app.on_canvas_pointer_event("kagari.viewport", move_dolly));

    let up_dolly = ame_ui::CanvasPointerEvent {
        phase: ame_ui::CanvasPointerPhase::Up,
        kind: ame_ui::CanvasPointerKind::Mouse,
        pointer_id: None,
        click_count: 1,
        x: 200.0,
        y: 250.0,
        button: Some(ame_ui::event::PointerButton::Primary),
        modifiers: alt_shift_mod,
        pressure: None,
        tilt: None,
        coalesced_samples: Vec::new(),
        space_pressed: false,
    };
    assert!(app.on_canvas_pointer_event("kagari.viewport", up_dolly));

    let pos_after_dolly = app
        .project_model
        .active_composition()
        .resolve_camera()
        .transform
        .position
        .evaluate(0);
    assert!(
        (pos_after_dolly[2] - (initial_pos[2] + 50.0 * 3.0)).abs() < 1e-3,
        "Camera should dolly along Z"
    );
}

#[test]
fn test_motion_tracker_ui_integration() {
    let mut app = AfterEffectsStudio::new().unwrap();
    invoke(&mut app, "back_to_editor", None);

    // Ensure layers exist
    let (layer_id, target_layer_id) = {
        let comp = app.project_model.active_composition_mut();
        let l1 = kagari_vfx::core::timeline::Layer::new(
            "source-layer".to_owned(),
            "Source Layer".to_owned(),
            kagari_vfx::core::timeline::LayerType::Solid {
                color: [1.0, 0.0, 0.0, 1.0],
            },
            comp.duration_frames,
        );
        let l2 = kagari_vfx::core::timeline::Layer::new(
            "target-layer".to_owned(),
            "Target Layer".to_owned(),
            kagari_vfx::core::timeline::LayerType::Solid {
                color: [0.0, 1.0, 0.0, 1.0],
            },
            comp.duration_frames,
        );
        comp.layers.push(l1);
        comp.layers.push(l2);
        ("source-layer".to_owned(), "target-layer".to_owned())
    };

    // Select the source layer
    app.selected_layer_id = Some(layer_id.clone());

    // 1. Add tracker
    invoke(&mut app, "add_motion_tracker", None);
    assert_eq!(
        app.project_model.active_composition().layers[0]
            .trackers
            .len(),
        1
    );
    assert!(app.tracker_status.contains("トラッカー 1 を作成しました"));

    // 2. Forward analyze
    invoke(&mut app, "analyze_motion_track_forward", None);
    let tracker = &app.project_model.active_composition().layers[0].trackers[0];
    match &tracker.position {
        kagari_vfx::core::timeline::Animatable::Animated(kfs) => {
            assert!(kfs.len() > 1, "Track analysis should produce keyframes");
        }
        _ => panic!("Expected animated keyframes after track analysis"),
    }

    // 3. Select target layer
    invoke(
        &mut app,
        "select_motion_tracker_target",
        Some(ActionValue::Text(target_layer_id.clone())),
    );
    assert_eq!(
        app.selected_tracker_target_id.as_ref(),
        Some(&target_layer_id)
    );

    // 4. Apply motion track to target
    invoke(&mut app, "apply_motion_track_to_target", None);
    let target_layer = app
        .project_model
        .active_composition()
        .layers
        .iter()
        .find(|l| l.id == target_layer_id)
        .unwrap();
    match &target_layer.transform.position {
        kagari_vfx::core::timeline::Animatable::Animated(kfs) => {
            assert!(
                kfs.len() > 1,
                "Target layer should now have animated position keyframes from tracker"
            );
        }
        _ => panic!("Target layer position should be animated after track apply"),
    }

    // 5. Add 3 more trackers to reach 4 points for Planar Corner Pin
    invoke(&mut app, "add_motion_tracker", None);
    invoke(&mut app, "add_motion_tracker", None);
    invoke(&mut app, "add_motion_tracker", None);
    assert_eq!(
        app.project_model.active_composition().layers[0]
            .trackers
            .len(),
        4
    );

    // 6. Apply 4-point planar track to target Corner Pin effect
    invoke(&mut app, "apply_planar_track_corner_pin", None);
    assert!(app
        .tracker_status
        .contains("4点プラナー コーナーピンを適用しました"));
    let target_layer_after_pin = app
        .project_model
        .active_composition()
        .layers
        .iter()
        .find(|l| l.id == target_layer_id)
        .unwrap();
    let has_corner_pin = target_layer_after_pin.effects.iter().any(|e| {
        matches!(
            e.effect_type,
            kagari_vfx::core::timeline::EffectType::CornerPin { .. }
        )
    });
    assert!(
        has_corner_pin,
        "Target layer should have a Corner Pin effect created by planar tracker"
    );
}
