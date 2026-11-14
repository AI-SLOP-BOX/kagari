use super::*;

#[test]
fn viewport_light_gizmo_drag_updates_light_position() {
    let mut app = AfterEffectsStudio::new().unwrap();
    app.selected_tool = "Selection";
    app.viewport_mapping = Some((0.0, 0.0, 1.0, 1.0));

    // Add a Point light at (200, 150, -500)
    let light_id = {
        let comp = app.project_model.active_composition_mut();
        let light = kagari_vfx::core::timeline::Light3D {
            id: "interactive_light".to_string(),
            position: kagari_vfx::core::property::Animatable::new_constant([200.0, 150.0, -500.0]),
            ..Default::default()
        };
        comp.lights.push(light);
        "interactive_light".to_string()
    };

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

    // Project initial position to find expected 2D screen coordinate
    let comp = app.project_model.active_composition();
    let active_cam = comp.resolve_camera();
    let src_w = comp.width.max(1) as f32;
    let src_h = comp.height.max(1) as f32;
    let (lx, ly) = if let Some(norm) = kagari_vfx::core::timeline::project_point_to_screen_at_frame(
        active_cam,
        [200.0, 150.0, -500.0],
        src_w,
        src_h,
        0,
    ) {
        (
            f64::from(norm[0]) * f64::from(src_w),
            f64::from(norm[1]) * f64::from(src_h),
        )
    } else {
        (200.0, 150.0)
    };

    // Click directly on light gizmo
    let handled = app.on_canvas_pointer_event(
        "kagari.viewport",
        make_event(ame_ui::CanvasPointerPhase::Down, lx, ly),
    );
    assert!(handled, "Click on light gizmo should be handled");
    assert!(
        app.light_gizmo_drag.is_some(),
        "Light gizmo drag should be active"
    );

    // Drag by (+50, +30)
    app.on_canvas_pointer_event(
        "kagari.viewport",
        make_event(ame_ui::CanvasPointerPhase::Moved, lx + 50.0, ly + 30.0),
    );

    // Release mouse
    app.on_canvas_pointer_event(
        "kagari.viewport",
        make_event(ame_ui::CanvasPointerPhase::Up, lx + 50.0, ly + 30.0),
    );
    assert!(
        app.light_gizmo_drag.is_none(),
        "Drag should be cleared on Up"
    );

    // Verify that light's 3D position updated
    let comp = app.project_model.active_composition();
    let light = comp
        .lights
        .iter()
        .find(|l| l.id == light_id)
        .expect("light exists");
    let updated_pos = light.position.evaluate(0);
    assert_eq!(updated_pos[0], 250.0);
    assert_eq!(updated_pos[1], 180.0);
    assert_eq!(updated_pos[2], -500.0);
}

#[test]
fn recent_projects_bind_real_data_or_empty_state() {
    let mut app = AfterEffectsStudio::new().unwrap();
    // Check default recent projects binding state
    assert_eq!(
        app.ui.state("has_recents"),
        Some(&Value::Bool(!app.recent_projects.is_empty()))
    );

    // Verify empty state switching
    app.recent_projects.clear();
    app.sync_bindings().unwrap();
    assert_eq!(app.ui.state("has_recents"), Some(&Value::Bool(false)));
    assert_eq!(app.ui.state("has_recent_1"), Some(&Value::Bool(false)));
}

#[test]
fn open_recent_loads_project_file_and_updates_editor_state() {
    let mut app = AfterEffectsStudio::new().unwrap();
    assert_eq!(app.screen, Screen::Home);

    // Ensure there is a recent project entry to open
    let dummy_proj = std::path::PathBuf::from("examples/projects/glitch_project.json");
    app.recent_projects.push(RecentProjectItem {
        path: dummy_proj.clone(),
        name: "glitch_project".to_owned(),
        path_label: dummy_proj.display().to_string(),
        date: "今日".to_owned(),
        duration: "10s".to_owned(),
        fps: "30fps".to_owned(),
        resolution: "1920x1080".to_owned(),
        layer_count: 3,
        comps_info: "1コンポジション".to_owned(),
    });

    // Invoke open_recent_1
    invoke(&mut app, "open_recent_1", None);
    assert_eq!(app.screen, Screen::Editor);

    // Verify that project_name and output_path were updated from recent project item
    assert!(!app.project_name.is_empty());
    assert!(app.output_path.contains(&app.project_name));

    // Test loading an actual JSON project directly
    let test_path = std::path::PathBuf::from("examples/projects/glitch_project.json");
    if test_path.is_file() {
        let _ = app.load_project_file(&test_path, "glitch_project");
        assert_eq!(app.project_name, "glitch_project");
        assert!(!app.layer_name.is_empty());
    }
}
