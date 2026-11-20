use super::*;

fn view_tree_has_button_label(node: &ame_ui::view::ViewNode, label: &str) -> bool {
    matches!(
        &node.kind,
        ame_ui::view::ViewKind::Button { label: actual, .. } if actual == label
    ) || node
        .children
        .iter()
        .any(|child| view_tree_has_button_label(child, label))
}

#[test]
fn video_frame_blending_action_updates_binding_and_survives_project_roundtrip() {
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(|| {
    let mut app = AfterEffectsStudio::new().unwrap();
    invoke(&mut app, "back_to_editor", None);
    let video = kagari_vfx::core::timeline::Layer::new(
        "video_frame_mix_test".to_owned(),
        "Frame Mix Test".to_owned(),
        kagari_vfx::core::timeline::LayerType::Video {
            source: "fixture.mp4".to_owned(),
            frames_dir: "fixture_frames".to_owned(),
            frame_count: 2,
            audio_wav: None,
            audio_volume: kagari_vfx::core::property::Animatable::new_constant(0.0),
            speed: 1.0,
        },
        30,
    );
    let layer_id = video.id.clone();
    app.project_model
        .active_composition_mut()
        .layers
        .push(video);
    app.selected_layer_id = Some(layer_id.clone());

    invoke(&mut app, "toggle_frame_blending", None);
    assert!(
        app.project_model
            .active_composition()
            .layers
            .last()
            .unwrap()
            .frame_blending
    );
    app.sync_graph_layer_bindings().unwrap();
    invoke(&mut app, "properties_section_advanced", None);
    assert_eq!(
        app.ui.state("layer_frame_blending"),
        Some(&Value::Bool(true))
    );
    let tree = app.shared_view_tree().unwrap();
    assert!(view_tree_has_button_label(&tree.root, "フレーム補間: ON"));

    let serialized =
        kagari_vfx::core::project_migration::save_project_versioned(&app.project_model).unwrap();
    let reopened = kagari_vfx::core::project_migration::load_project_migrated(&serialized).unwrap();
    let reopened_layer = reopened
        .active_composition()
        .layers
        .iter()
        .find(|layer| layer.id == layer_id)
        .unwrap();
    assert!(reopened_layer.frame_blending);

    invoke(&mut app, "toggle_frame_blending", None);
    assert!(
        !app.project_model
            .active_composition()
            .layers
            .last()
            .unwrap()
            .frame_blending
    );
    app.sync_graph_layer_bindings().unwrap();
    invoke(&mut app, "properties_section_advanced", None);
    assert_eq!(
        app.ui.state("layer_frame_blending"),
        Some(&Value::Bool(false))
    );
    let tree = app.shared_view_tree().unwrap();
    assert!(
        view_tree_has_button_label(&tree.root, "フレーム補間: OFF"),
        "selected_is_video={:?}, properties_section_advanced={:?}, layer_frame_blending={:?}",
        app.ui.state("selected_is_video"),
        app.ui.state("properties_section_advanced"),
        app.ui.state("layer_frame_blending"),
    );
    let serialized =
        kagari_vfx::core::project_migration::save_project_versioned(&app.project_model).unwrap();
    let reopened = kagari_vfx::core::project_migration::load_project_migrated(&serialized).unwrap();
    let reopened_layer = reopened
        .active_composition()
        .layers
        .iter()
        .find(|layer| layer.id == layer_id)
        .unwrap();
    assert!(!reopened_layer.frame_blending);
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn workspace_and_timeline_tabs_change_the_active_panel() {
    let mut app = AfterEffectsStudio::new().unwrap();
    assert_eq!(app.screen, Screen::Home);
    invoke(&mut app, "back_to_editor", None);
    assert_eq!(app.screen, Screen::Editor);
    invoke(&mut app, "workspace_effects", None);
    assert_eq!(app.workspace, Workspace::Effects);
    invoke(&mut app, "go_export", None);
    assert_eq!(app.screen, Screen::Export);
    assert!(app.render_queue);
    invoke(&mut app, "back_to_editor", None);
    assert_eq!(app.screen, Screen::Editor);
    assert!(!app.render_queue);
}

#[test]
fn committed_japanese_text_and_resize_state_are_preserved() {
    let mut app = AfterEffectsStudio::new().unwrap();
    invoke(
        &mut app,
        "layer_changed",
        Some(ActionValue::Text("夕焼けのタイトル".to_owned())),
    );
    assert_eq!(app.layer_name, "夕焼けのタイトル");
    app.on_event(&AmeEvent::Resized {
        width: 640,
        height: 480,
    });
    assert!(app.compact);
    assert_eq!(
        app.ui.state("layer_name"),
        Some(&Value::String("夕焼けのタイトル".to_owned()))
    );
}

#[test]
fn draw_canvas_handles_viewport_and_timeline_with_full_render_commands() {
    let mut app = AfterEffectsStudio::new().unwrap();
    let mut scene = Scene::new();

    // 1. Draw Viewport
    app.draw_viewport(&mut scene, 0.0, 0.0, 800.0, 450.0);
    // Ensure scene is non-empty
    assert!(!scene.encoding().is_empty());

    // 1b. Verify 3D Light Gizmo renders when Point, Spot, and Parallel lights are present
    {
        let comp = app.project_model.active_composition_mut();
        comp.lights
            .push(kagari_vfx::core::timeline::Light3D::default());
        let spot = kagari_vfx::core::timeline::Light3D {
            light_type: kagari_vfx::core::timeline::LightType::Spot {
                cone_angle_deg: 60.0,
                cone_feather_pct: 20.0,
            },
            position: kagari_vfx::core::property::Animatable::new_constant([400.0, 200.0, -300.0]),
            ..Default::default()
        };
        comp.lights.push(spot);

        let parallel = kagari_vfx::core::timeline::Light3D {
            light_type: kagari_vfx::core::timeline::LightType::Parallel,
            ..Default::default()
        };
        comp.lights.push(parallel);

        // Add a 3D camera
        comp.cameras
            .push(kagari_vfx::core::timeline::Camera3D::default());
    }
    let mut scene_with_light_and_cam = Scene::new();
    app.draw_viewport(&mut scene_with_light_and_cam, 0.0, 0.0, 800.0, 450.0);
    assert!(!scene_with_light_and_cam.encoding().is_empty());

    // 2. Draw Timeline
    let mut timeline_scene = Scene::new();
    let composition = app.project_model.active_composition();
    draw_timeline(
        &mut timeline_scene,
        0.0,
        0.0,
        800.0,
        190.0,
        132,
        None,
        composition.duration_frames,
        0.0,
        composition.duration_frames as f64,
        app.timeline_vertical_scroll,
        &composition.layers,
        app.selected_layer_id.as_deref(),
        None,
        &std::collections::HashMap::new(),
        composition.fps,
    );
    assert!(!timeline_scene.encoding().is_empty());

    // 3. Draw Flowchart
    let mut flowchart_scene = Scene::new();
    draw_flowchart(
        &mut flowchart_scene,
        0.0,
        0.0,
        800.0,
        300.0,
        composition,
        app.selected_layer_id.as_deref(),
        Some(FlowchartDragWireView {
            start: (200.0, 50.0),
            current: (350.0, 120.0),
            is_matte: false,
        }),
    );
    assert!(!flowchart_scene.encoding().is_empty());
}

#[test]
fn flowchart_pin_wiring_connects_parent_layer() {
    let mut app = AfterEffectsStudio::new().unwrap();
    invoke(&mut app, "timeline_view_flowchart", None);

    let (parent_id, child_id) = {
        let comp = app.project_model.active_composition_mut();
        while comp.layers.len() < 2 {
            let idx = comp.layers.len() + 1;
            comp.layers.push(kagari_vfx::core::timeline::Layer::new(
                format!("layer-{idx}"),
                format!("Layer {idx}"),
                kagari_vfx::core::timeline::LayerType::Solid {
                    color: [0.5, 0.5, 0.5, 1.0],
                },
                comp.duration_frames,
            ));
        }
        comp.layers[0].parent_id = None;
        comp.layers[1].parent_id = None;
        (comp.layers[0].id.clone(), comp.layers[1].id.clone())
    };

    let graph = kagari_vfx::core::flow_graph_model::FlowGraphModel::build_structure(
        app.project_model.active_composition(),
    );
    let positions = graph.positions_f64(24.0, 20.0, 180.0, 52.0);
    let node_w = 150.0;
    let node_h = 38.0;

    let make_event = |phase, x, y| ame_ui::CanvasPointerEvent {
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

    // Simulate Mouse Down on Layer 0's parent output pin (top pin)
    let (p0_x, p0_y) = positions[0];
    let p0_parent_out = (p0_x + node_w, p0_y + 12.0);
    app.on_canvas_pointer_event(
        "kagari.flowchart",
        make_event(
            ame_ui::CanvasPointerPhase::Down,
            p0_parent_out.0,
            p0_parent_out.1,
        ),
    );
    assert!(
        app.flowchart_wire_drag.is_some(),
        "Flowchart drag should start from parent output pin"
    );
    assert_eq!(
        app.flowchart_wire_drag.as_ref().map(|d| d.kind),
        Some(FlowchartWireKind::Parent)
    );

    // Simulate Mouse Move to stretch wire
    app.on_canvas_pointer_event(
        "kagari.flowchart",
        make_event(
            ame_ui::CanvasPointerPhase::Moved,
            positions[1].0,
            positions[1].1 + node_h * 0.5,
        ),
    );

    // Simulate Mouse Up on Layer 1's input pin / card
    let (p1_x, p1_y) = positions[1];
    let p1_in = (p1_x, p1_y + node_h * 0.5);
    app.on_canvas_pointer_event(
        "kagari.flowchart",
        make_event(ame_ui::CanvasPointerPhase::Up, p1_in.0, p1_in.1),
    );
    assert!(app.flowchart_wire_drag.is_none(), "Drag should clear on Up");

    // Verify that layer 1's parent_id is now connected to layer 0!
    let comp = app.project_model.active_composition();
    let target = comp.layers.iter().find(|l| l.id == child_id).unwrap();
    assert_eq!(
        target.parent_id.as_deref(),
        Some(parent_id.as_str()),
        "Child layer parent_id should be wired to source layer"
    );

    // Recompute positions because layer 1 is now a child (depth 1 instead of depth 0)
    let graph2 = kagari_vfx::core::flow_graph_model::FlowGraphModel::build_structure(
        app.project_model.active_composition(),
    );
    let positions2 = graph2.positions_f64(24.0, 20.0, 180.0, 52.0);
    let (p1_new_x, p1_new_y) = positions2[1];
    let p1_new_in = (p1_new_x, p1_new_y + node_h * 0.5);

    // Now test Matte pin wiring: Drag from layer 0's matte pin to layer 1
    let p0_matte_out = (p0_x + node_w, p0_y + node_h - 12.0);
    app.on_canvas_pointer_event(
        "kagari.flowchart",
        make_event(
            ame_ui::CanvasPointerPhase::Down,
            p0_matte_out.0,
            p0_matte_out.1,
        ),
    );
    assert!(
        app.flowchart_wire_drag.is_some(),
        "Flowchart drag should start from matte pin"
    );
    assert_eq!(
        app.flowchart_wire_drag.as_ref().map(|d| d.kind),
        Some(FlowchartWireKind::Matte)
    );

    app.on_canvas_pointer_event(
        "kagari.flowchart",
        make_event(ame_ui::CanvasPointerPhase::Up, p1_new_in.0, p1_new_in.1),
    );
    assert!(app.flowchart_wire_drag.is_none());

    let comp = app.project_model.active_composition();
    let target = comp.layers.iter().find(|l| l.id == child_id).unwrap();
    assert_eq!(
        target.track_matte_layer_id.as_deref(),
        Some(parent_id.as_str()),
        "Child layer track_matte_layer_id should be wired to source layer"
    );
    assert_eq!(
        target.track_matte,
        kagari_vfx::core::timeline::TrackMatteMode::AlphaMatte,
        "Target track_matte should become AlphaMatte"
    );
}

#[test]
fn viewport_pen_tool_draws_live_guide_and_creates_shape() {
    let mut app = AfterEffectsStudio::new().unwrap();
    app.selected_tool = "Pen";
    app.viewport_mapping = Some((0.0, 0.0, 1.0, 1.0));

    let make_event = |phase, x, y| ame_ui::CanvasPointerEvent {
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

    let initial_layer_count = app.project_model.active_composition().layers.len();

    // Down
    app.on_canvas_pointer_event(
        "kagari.viewport",
        make_event(ame_ui::CanvasPointerPhase::Down, 50.0, 50.0),
    );
    assert_eq!(app.viewport_pen_start, Some([50.0, 50.0]));

    // Moved
    app.on_canvas_pointer_event(
        "kagari.viewport",
        make_event(ame_ui::CanvasPointerPhase::Moved, 150.0, 200.0),
    );
    assert_eq!(app.viewport_pen_current, Some([150.0, 200.0]));

    // Verify draw_viewport renders live pen guide without panics
    let mut scene = Scene::new();
    app.draw_viewport(&mut scene, 0.0, 0.0, 800.0, 600.0);
    assert!(!scene.encoding().is_empty());

    // Up -> creates bezier shape layer
    app.on_canvas_pointer_event(
        "kagari.viewport",
        make_event(ame_ui::CanvasPointerPhase::Up, 150.0, 200.0),
    );
    assert_eq!(app.viewport_pen_start, None);
    assert_eq!(app.viewport_pen_current, None);
    assert_eq!(
        app.project_model.active_composition().layers.len(),
        initial_layer_count + 1,
        "Pen tool should have added a freeform bezier shape layer"
    );
}

#[test]
fn command_palette_context_menus_and_navigation_e2e() {
    let mut app = AfterEffectsStudio::new().unwrap();
    assert_eq!(app.screen, Screen::Home);
    assert_eq!(app.ui.state("home_page"), Some(&Value::Bool(true)));

    // 1. Navigation & Settings screens sync both state fields and AmeUI bindings
    invoke(&mut app, "go_settings", None);
    assert!(app.settings_open);
    assert_eq!(app.ui.state("settings_page"), Some(&Value::Bool(true)));
    invoke(&mut app, "close_settings", None);
    assert!(!app.settings_open);
    assert_eq!(app.ui.state("settings_page"), Some(&Value::Bool(false)));

    invoke(&mut app, "go_projects", None);
    assert_eq!(app.screen, Screen::Projects);
    assert_eq!(app.ui.state("projects_page"), Some(&Value::Bool(true)));

    invoke(&mut app, "go_activity", None);
    assert_eq!(app.screen, Screen::Activity);
    assert_eq!(app.ui.state("activity_page"), Some(&Value::Bool(true)));

    invoke(&mut app, "go_documents", None);
    assert_eq!(app.screen, Screen::Documents);
    assert_eq!(app.ui.state("documents_page"), Some(&Value::Bool(true)));

    invoke(&mut app, "go_home", None);
    assert_eq!(app.screen, Screen::Home);
    assert_eq!(app.ui.state("home_page"), Some(&Value::Bool(true)));

    invoke(&mut app, "new_project", None);
    assert_eq!(app.screen, Screen::ProjectSetup);
    assert_eq!(app.ui.state("project_setup_page"), Some(&Value::Bool(true)));

    invoke(&mut app, "back_to_editor", None);
    assert_eq!(app.screen, Screen::Editor);
    assert_eq!(app.ui.state("editor_page"), Some(&Value::Bool(true)));

    // 2. Command Palette open / execute / close lifecycle
    invoke(&mut app, "toggle_command_palette", None);
    assert!(app.command_palette_open);
    assert_eq!(
        app.ui.state("command_palette_open"),
        Some(&Value::Bool(true))
    );
    invoke(
        &mut app,
        "execute_command",
        Some(ActionValue::Text("workspace_effects".to_owned())),
    );
    assert!(!app.command_palette_open);
    assert_eq!(
        app.ui.state("command_palette_open"),
        Some(&Value::Bool(false))
    );
    assert_eq!(app.workspace, Workspace::Effects);
    assert_eq!(
        app.ui.state("is_effects_workspace"),
        Some(&Value::Bool(true))
    );

    invoke(&mut app, "toggle_command_palette", None);
    assert!(app.command_palette_open);
    invoke(
        &mut app,
        "execute_command",
        Some(ActionValue::Text("tool_roto".to_owned())),
    );
    assert!(!app.command_palette_open);
    assert_eq!(app.selected_tool, "RotoBrush");
    assert_eq!(app.ui.state("selected_tool_roto"), Some(&Value::Bool(true)));

    // 3. Context menu actions across timeline, graph editor, and playback range
    let initial_layers = app.project_model.active_composition().layers.len();
    invoke(&mut app, "add_solid_layer", None);
    assert_eq!(
        app.project_model.active_composition().layers.len(),
        initial_layers + 1
    );
    assert!(app.selected_layer_id.is_some());

    invoke(&mut app, "duplicate_selected_layer", None);
    assert_eq!(
        app.project_model.active_composition().layers.len(),
        initial_layers + 2
    );

    app.current_frame = 15;
    invoke(&mut app, "split_selected_layer_at_playhead", None);
    assert_eq!(
        app.project_model.active_composition().layers.len(),
        initial_layers + 3
    );

    invoke(&mut app, "set_selected_layer_in_to_playhead", None);
    app.current_frame = 45;
    invoke(&mut app, "set_selected_layer_out_to_playhead", None);

    invoke(&mut app, "graph_add_keyframe", None);
    invoke(&mut app, "graph_set_easy_ease", None);
    invoke(&mut app, "graph_cycle_interpolation", None);
    assert_eq!(app.timeline_view, TimelineView::Graph);

    invoke(&mut app, "clear_playback_range", None);

    let before_remove = app.project_model.active_composition().layers.len();
    invoke(&mut app, "request_remove_selected_layer", None);
    assert!(app.layer_remove_confirm_open);
    assert_eq!(
        app.ui.state("layer_remove_confirm_open"),
        Some(&Value::Bool(true))
    );
    invoke(&mut app, "confirm_remove_selected_layer", None);
    assert!(!app.layer_remove_confirm_open);
    assert_eq!(
        app.ui.state("layer_remove_confirm_open"),
        Some(&Value::Bool(false))
    );
    assert_eq!(
        app.project_model.active_composition().layers.len(),
        before_remove - 1
    );
}

