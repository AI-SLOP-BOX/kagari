use super::*;

fn find_slider_nodes(
    node: &ame_ui::view::ViewNode,
    action_name: &str,
    out: &mut Vec<ame_ui::view::ViewNode>,
) {
    if matches!(
        &node.kind,
        ame_ui::view::ViewKind::Slider {
            action_id: Some(id),
            ..
        } if id == action_name
    ) {
        out.push(node.clone());
    }
    for child in &node.children {
        find_slider_nodes(child, action_name, out);
    }
}

#[test]
fn color_workspace_scopes_color_wheels_and_effect_sliders_e2e() {
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(|| {
            let mut app = AfterEffectsStudio::new().unwrap();
            invoke(&mut app, "new_project", None);
            invoke(&mut app, "back_to_editor", None);
            invoke(&mut app, "add_solid_layer", None);
            invoke(&mut app, "workspace_color", None);

            let mut scene = vello::Scene::new();
            app.draw_scopes_canvas(&mut scene, 0.0, 0.0, 320.0, 112.0);
            assert!(!scene.encoding().is_empty());

            let mut wheel_scene = vello::Scene::new();
            app.draw_color_wheels_canvas(&mut wheel_scene, 0.0, 0.0, 320.0, 148.0);
            assert!(!wheel_scene.encoding().is_empty());

            let make_pointer = |phase, x, y| ame_ui::app::CanvasPointerEvent {
                phase,
                kind: ame_ui::CanvasPointerKind::Mouse,
                pointer_id: None,
                click_count: 1,
                x,
                y,
                button: Some(ame_ui::event::PointerButton::Primary),
                modifiers: Default::default(),
                pressure: None,
                tilt: None,
                coalesced_samples: Vec::new(),
                space_pressed: false,
            };
            assert_eq!(
                App::state_json(&app, "color_scope_mode"),
                Some(serde_json::Value::String("parade".to_owned()))
            );
            assert!(app.on_canvas_pointer_event(
                "kagari.scopes",
                make_pointer(ame_ui::app::CanvasPointerPhase::Down, 40.0, 40.0),
            ));
            assert!(app.on_canvas_pointer_event(
                "kagari.scopes",
                make_pointer(ame_ui::app::CanvasPointerPhase::Up, 40.0, 40.0),
            ));
            assert_eq!(
                App::state_json(&app, "color_scope_mode"),
                Some(serde_json::Value::String("waveform".to_owned()))
            );
            assert!(app.on_canvas_pointer_event(
                "kagari.scopes",
                make_pointer(ame_ui::app::CanvasPointerPhase::Down, 40.0, 40.0),
            ));
            assert!(app.on_canvas_pointer_event(
                "kagari.scopes",
                make_pointer(ame_ui::app::CanvasPointerPhase::Up, 40.0, 40.0),
            ));
            assert_eq!(
                App::state_json(&app, "color_scope_mode"),
                Some(serde_json::Value::String("vectorscope".to_owned()))
            );

            assert!(app.on_canvas_pointer_event(
                "kagari.color_wheels",
                make_pointer(ame_ui::app::CanvasPointerPhase::Down, 50.0, 80.0),
            ));
            assert!(app.on_canvas_pointer_event(
                "kagari.color_wheels",
                make_pointer(ame_ui::app::CanvasPointerPhase::Moved, 50.0, 40.0),
            ));
            assert!(app.on_canvas_pointer_event(
                "kagari.color_wheels",
                make_pointer(ame_ui::app::CanvasPointerPhase::Up, 50.0, 40.0),
            ));
            let (lift, _, _, _) = app.current_color_wheel_values();
            assert!(
                lift > 0.1,
                "expected Lift wheel drag to raise lift, got {lift}"
            );

            invoke(&mut app, "reset_color_wheels", None);
            let (lift_after_reset, _, _, _) = app.current_color_wheel_values();
            assert!(lift_after_reset.abs() < 1e-4);

            invoke(&mut app, "add_gaussian_blur", None);
            let blur_tree = app.shared_view_tree().unwrap();
            let mut slider_nodes = Vec::new();
            find_slider_nodes(
                &blur_tree.root,
                "effect_parameter_slider_changed",
                &mut slider_nodes,
            );
            assert!(!slider_nodes.is_empty());
            app.handle_action(HostAction {
                name: "effect_parameter_slider_changed".to_owned(),
                value: Some(ame_ui::event::ActionValue::Number(0.25)),
                target: Some(slider_nodes[0].identity.clone()),
            })
            .unwrap();
            let layer_id = app.selected_layer_id.as_deref().unwrap();
            let blur_effect = app
                .project_model
                .active_composition()
                .layers
                .iter()
                .find(|l| l.id == layer_id)
                .unwrap()
                .effects
                .last()
                .unwrap();
            let blur_val = match &blur_effect.effect_type {
                kagari_vfx::core::timeline::EffectType::GaussianBlur { blur_radius } => {
                    blur_radius.evaluate(0)
                }
                other => panic!("unexpected effect type: {other:?}"),
            };
            assert!((blur_val - 50.0).abs() < 0.1, "expected 50.0, got {blur_val}");

            invoke(&mut app, "add_color_tint", None);
            app.handle_action(HostAction {
                name: "effect_color_changed".to_owned(),
                value: Some(ame_ui::event::ActionValue::Text("#12AB34".to_owned())),
                target: None,
            })
            .unwrap();
            assert_eq!(
                App::state_json(&app, "selected_effect_color"),
                Some(serde_json::Value::String("#12AB34".to_owned()))
            );
        })
        .unwrap()
        .join()
        .unwrap();
}
