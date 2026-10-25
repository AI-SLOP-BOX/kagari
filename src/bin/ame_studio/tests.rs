use super::*;

#[cfg(test)]
mod tests {
    use super::*;

    fn invoke(app: &mut AfterEffectsStudio, name: &str, value: Option<ActionValue>) {
        app.handle_action(HostAction {
            name: name.to_owned(),
            value,
            target: None,
        })
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
            comp.lights.push(kagari_vfx::core::timeline::Light3D::default());
            let mut spot = kagari_vfx::core::timeline::Light3D::default();
            spot.light_type = kagari_vfx::core::timeline::LightType::Spot {
                cone_angle_deg: 60.0,
                cone_feather_pct: 20.0,
            };
            spot.position = kagari_vfx::core::property::Animatable::new_constant([400.0, 200.0, -300.0]);
            comp.lights.push(spot);

            let mut parallel = kagari_vfx::core::timeline::Light3D::default();
            parallel.light_type = kagari_vfx::core::timeline::LightType::Parallel;
            comp.lights.push(parallel);

            // Add a 3D camera
            comp.cameras.push(kagari_vfx::core::timeline::Camera3D::default());
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
                    kagari_vfx::core::timeline::LayerType::Solid { color: [0.5, 0.5, 0.5, 1.0] },
                    comp.duration_frames,
                ));
            }
            comp.layers[0].parent_id = None;
            comp.layers[1].parent_id = None;
            (comp.layers[0].id.clone(), comp.layers[1].id.clone())
        };

        let graph = kagari_vfx::core::flow_graph_model::FlowGraphModel::build_structure(app.project_model.active_composition());
        let positions = graph.positions_f64(24.0, 20.0, 180.0, 52.0);
        let node_w = 150.0;
        let node_h = 38.0;

        let make_event = |phase, x, y| ame_ui::CanvasPointerEvent {
            phase,
            kind: ame_ui::CanvasPointerKind::Mouse,
            pointer_id: None,
            x,
            y,
            button: Some(ame_ui::event::PointerButton::Primary),
            modifiers: ame_ui::event::KeyModifiers::default(),
            space_pressed: false,
        };

        // Simulate Mouse Down on Layer 0's parent output pin (top pin)
        let (p0_x, p0_y) = positions[0];
        let p0_parent_out = (p0_x + node_w, p0_y + 12.0);
        app.on_canvas_pointer_event("kagari.flowchart", make_event(ame_ui::CanvasPointerPhase::Down, p0_parent_out.0, p0_parent_out.1));
        assert!(app.flowchart_wire_drag.is_some(), "Flowchart drag should start from parent output pin");
        assert_eq!(app.flowchart_wire_drag.as_ref().map(|d| d.kind), Some(FlowchartWireKind::Parent));

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
        app.on_canvas_pointer_event("kagari.flowchart", make_event(ame_ui::CanvasPointerPhase::Up, p1_in.0, p1_in.1));
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
        let graph2 = kagari_vfx::core::flow_graph_model::FlowGraphModel::build_structure(app.project_model.active_composition());
        let positions2 = graph2.positions_f64(24.0, 20.0, 180.0, 52.0);
        let (p1_new_x, p1_new_y) = positions2[1];
        let p1_new_in = (p1_new_x, p1_new_y + node_h * 0.5);

        // Now test Matte pin wiring: Drag from layer 0's matte pin to layer 1
        let p0_matte_out = (p0_x + node_w, p0_y + node_h - 12.0);
        app.on_canvas_pointer_event("kagari.flowchart", make_event(ame_ui::CanvasPointerPhase::Down, p0_matte_out.0, p0_matte_out.1));
        assert!(app.flowchart_wire_drag.is_some(), "Flowchart drag should start from matte pin");
        assert_eq!(app.flowchart_wire_drag.as_ref().map(|d| d.kind), Some(FlowchartWireKind::Matte));

        app.on_canvas_pointer_event("kagari.flowchart", make_event(ame_ui::CanvasPointerPhase::Up, p1_new_in.0, p1_new_in.1));
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
            x,
            y,
            button: Some(ame_ui::event::PointerButton::Primary),
            modifiers: ame_ui::event::KeyModifiers::default(),
            space_pressed: false,
        };

        let initial_layer_count = app.project_model.active_composition().layers.len();

        // Down
        app.on_canvas_pointer_event("kagari.viewport", make_event(ame_ui::CanvasPointerPhase::Down, 50.0, 50.0));
        assert_eq!(app.viewport_pen_start, Some([50.0, 50.0]));

        // Moved
        app.on_canvas_pointer_event("kagari.viewport", make_event(ame_ui::CanvasPointerPhase::Moved, 150.0, 200.0));
        assert_eq!(app.viewport_pen_current, Some([150.0, 200.0]));

        // Verify draw_viewport renders live pen guide without panics
        let mut scene = Scene::new();
        app.draw_viewport(&mut scene, 0.0, 0.0, 800.0, 600.0);
        assert!(!scene.encoding().is_empty());

        // Up -> creates bezier shape layer
        app.on_canvas_pointer_event("kagari.viewport", make_event(ame_ui::CanvasPointerPhase::Up, 150.0, 200.0));
        assert_eq!(app.viewport_pen_start, None);
        assert_eq!(app.viewport_pen_current, None);
        assert_eq!(
            app.project_model.active_composition().layers.len(),
            initial_layer_count + 1,
            "Pen tool should have added a freeform bezier shape layer"
        );
    }

    #[test]
    fn viewport_light_gizmo_drag_updates_light_position() {
        let mut app = AfterEffectsStudio::new().unwrap();
        app.selected_tool = "Selection";
        app.viewport_mapping = Some((0.0, 0.0, 1.0, 1.0));

        // Add a Point light at (200, 150, -500)
        let light_id = {
            let comp = app.project_model.active_composition_mut();
            let mut light = kagari_vfx::core::timeline::Light3D::default();
            light.id = "interactive_light".to_string();
            light.position = kagari_vfx::core::property::Animatable::new_constant([200.0, 150.0, -500.0]);
            comp.lights.push(light);
            "interactive_light".to_string()
        };

        let make_event = |phase, x, y| ame_ui::CanvasPointerEvent {
            phase,
            kind: ame_ui::CanvasPointerKind::Mouse,
            pointer_id: None,
            x,
            y,
            button: Some(ame_ui::event::PointerButton::Primary),
            modifiers: ame_ui::event::KeyModifiers::default(),
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
            (f64::from(norm[0]) * f64::from(src_w), f64::from(norm[1]) * f64::from(src_h))
        } else {
            (200.0, 150.0)
        };

        // Click directly on light gizmo
        let handled = app.on_canvas_pointer_event("kagari.viewport", make_event(ame_ui::CanvasPointerPhase::Down, lx, ly));
        assert!(handled, "Click on light gizmo should be handled");
        assert!(app.light_gizmo_drag.is_some(), "Light gizmo drag should be active");

        // Drag by (+50, +30)
        app.on_canvas_pointer_event("kagari.viewport", make_event(ame_ui::CanvasPointerPhase::Moved, lx + 50.0, ly + 30.0));

        // Release mouse
        app.on_canvas_pointer_event("kagari.viewport", make_event(ame_ui::CanvasPointerPhase::Up, lx + 50.0, ly + 30.0));
        assert!(app.light_gizmo_drag.is_none(), "Drag should be cleared on Up");

        // Verify that light's 3D position updated
        let comp = app.project_model.active_composition();
        let light = comp.lights.iter().find(|l| l.id == light_id).expect("light exists");
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

    #[test]
    fn timecode_respects_project_fps() {
        let mut app = AfterEffectsStudio::new().unwrap();
        assert_eq!(app.project_fps, 30);

        // 00:00:02:12 at 30 fps is 2 * 30 + 12 = 72 frames
        invoke(
            &mut app,
            "time_changed",
            Some(ActionValue::Text("00:00:02:12".to_owned())),
        );
        assert_eq!(app.current_frame, 72);
        assert_eq!(app.current_time, "00:00:02:12");

        // Now set project_fps to 60 and test time_changed
        app.project_fps = 60;
        invoke(
            &mut app,
            "time_changed",
            Some(ActionValue::Text("00:00:02:12".to_owned())),
        );
        // at 60 fps, 2 * 60 + 12 = 132 frames
        assert_eq!(app.current_frame, 132);
        assert_eq!(app.current_time, "00:00:02:12");
    }

    #[test]
    fn duration_format_rolls_over_minutes_and_hours() {
        // Test durations > 60 seconds
        let dur_75_secs = 75u64;
        let formatted = format!(
            "{:02}:{:02}:{:02}",
            dur_75_secs / 3600,
            (dur_75_secs / 60) % 60,
            dur_75_secs % 60
        );
        assert_eq!(formatted, "00:01:15");

        let dur_3665_secs = 3665u64;
        let formatted_h = format!(
            "{:02}:{:02}:{:02}",
            dur_3665_secs / 3600,
            (dur_3665_secs / 60) % 60,
            dur_3665_secs % 60
        );
        assert_eq!(formatted_h, "01:01:05");
    }

    #[test]
    fn playback_actions_work() {
        let mut app = AfterEffectsStudio::new().unwrap();
        assert!(!app.is_playing);
        invoke(&mut app, "play_start", None);
        assert!(app.is_playing);

        invoke(&mut app, "play_stop", None);
        assert!(!app.is_playing);

        invoke(&mut app, "play_rewind", None);
        assert_eq!(app.current_frame, 0);
        assert_eq!(app.current_time, "00:00:00:00");

        invoke(&mut app, "play_forward", None);
        assert_eq!(app.current_frame, 299);
        assert_eq!(app.current_time, format_timecode(299, app.project_fps));
    }

    #[test]
    fn core_ame_studio_actions_coverage() {
        // 1. Scan src/ui/ame for all actions declared in .ame files
        let ame_dir = std::path::Path::new("src/ui/ame");
        if !ame_dir.exists() {
            return;
        }

        let mut declared_actions = std::collections::BTreeSet::new();

        fn extract_action(line: &str) -> Option<String> {
            let idx = line.find("action:")?;
            let rest = line[idx + 7..].trim_start();
            let end = rest.find(|c: char| !c.is_alphanumeric() && c != '_').unwrap_or(rest.len());
            let action = &rest[..end];
            if !action.is_empty() {
                Some(action.to_string())
            } else {
                None
            }
        }

        fn visit_dirs(dir: &std::path::Path, acc: &mut std::collections::BTreeSet<String>) {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        visit_dirs(&path, acc);
                    } else if path.extension().and_then(|s| s.to_str()) == Some("ame") {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            for line in content.lines() {
                                let mut curr = line;
                                while let Some(action) = extract_action(curr) {
                                    acc.insert(action);
                                    if let Some(pos) = curr.find("action:") {
                                        curr = &curr[pos + 7..];
                                        if let Some(next_pos) = curr.find(|c: char| !c.is_alphanumeric() && c != '_') {
                                            curr = &curr[next_pos..];
                                        } else {
                                            break;
                                        }
                                    } else {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        visit_dirs(ame_dir, &mut declared_actions);
        assert!(!declared_actions.is_empty(), "Expected to find actions declared in .ame files");

        // 2. Scan Rust handler files for handled action names
        let handler_files = [
            "src/bin/ame_studio/event_handlers.rs",
            "src/bin/ame_studio/preferences.rs",
            "src/bin/ame_studio/composition_action_ops.rs",
            "src/bin/ame_studio/playback_ops.rs",
        ];
        let mut handled_actions = std::collections::BTreeSet::new();

        for file_path in handler_files {
            if let Ok(content) = std::fs::read_to_string(file_path) {
                for line in content.lines() {
                    let mut curr = line;
                    while let Some(start) = curr.find("(\"") {
                        let rest = &curr[start + 2..];
                        if let Some(end) = rest.find('"') {
                            let action = &rest[..end];
                            if !action.is_empty() && rest[end + 1..].trim_start().starts_with(',') {
                                handled_actions.insert(action.to_string());
                            }
                            curr = &rest[end + 1..];
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        let mut unhandled = Vec::new();
        for action in &declared_actions {
            if !handled_actions.contains(action) {
                unhandled.push(action.clone());
            }
        }

        assert!(
            unhandled.is_empty(),
            "Found {} unhandled .ame actions in Rust handlers: {:?}",
            unhandled.len(),
            unhandled
        );
    }

    #[test]
    fn particle_emitter_extended_controls_update_state() {
        let mut app = AfterEffectsStudio::new().unwrap();
        app.add_particle_layer();
        let selected_id = app.selected_layer_id.clone().expect("particle layer should be selected");

        // Test updating extended properties via update_selected_emitter
        app.update_selected_emitter("spread", "120.0");
        app.update_selected_emitter("gravity_y", "980.0");
        app.update_selected_emitter("wind_x", "150.0");
        app.update_selected_emitter("turbulence", "45.0");
        app.update_selected_emitter("drag", "0.25");

        let comp = app.project_model.active_composition();
        let layer = comp.layers.iter().find(|l| l.id == selected_id).expect("layer exists");
        if let kagari_vfx::core::timeline::LayerType::Particle { emitter } = &layer.layer_type {
            assert_eq!(emitter.spread_degrees, 120.0);
            assert_eq!(emitter.gravity[1], 980.0);
            assert_eq!(emitter.wind[0], 150.0);
            assert_eq!(emitter.turbulence, 45.0);
            assert_eq!(emitter.drag, 0.25);
        } else {
            panic!("Expected particle layer");
        }
    }

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
            layer.transform.position.add_keyframe(kagari_vfx::core::keyframe::Keyframe::new(0, [100.0, 100.0], kagari_vfx::core::keyframe::InterpolationType::Linear));
            layer.transform.position.add_keyframe(kagari_vfx::core::keyframe::Keyframe::new(30, [400.0, 100.0], kagari_vfx::core::keyframe::InterpolationType::Linear));
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
        let py_0 = plot_bottom - (100.0 - f64::from(min_v)) / f64::from((max_v - min_v).max(f32::EPSILON)) * (plot_bottom - plot_top);

        let right_click_event = ame_ui::CanvasPointerEvent {
            phase: ame_ui::CanvasPointerPhase::Down,
            kind: ame_ui::CanvasPointerKind::Mouse,
            pointer_id: None,
            x: px_0,
            y: py_0,
            button: Some(ame_ui::event::PointerButton::Secondary),
            modifiers: ame_ui::event::KeyModifiers::default(),
            space_pressed: false,
        };

        // Trigger right-click toggle
        let handled = app.on_canvas_pointer_event("kagari.graph", right_click_event);
        assert!(handled, "Right click on keyframe should be handled");

        // Verify that interpolation changed to Bezier
        let comp = app.project_model.active_composition();
        let layer = comp.layers.iter().find(|l| l.id == layer_id).unwrap();
        let key_0 = layer.transform.position.keyframes().unwrap().iter().find(|k| k.frame == 0).unwrap();
        assert!(
            matches!(key_0.interpolation, kagari_vfx::core::keyframe::InterpolationType::Bezier { .. }),
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

        let initial_pos = app.project_model.active_composition().resolve_camera().transform.position.evaluate(0);
        let initial_rot = app.project_model.active_composition().resolve_camera().transform.rotation.evaluate(0);

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
            x: 200.0,
            y: 200.0,
            button: Some(ame_ui::event::PointerButton::Primary),
            modifiers: alt_mod,
            space_pressed: false,
        };
        assert!(app.on_canvas_pointer_event("kagari.viewport", down_orbit));
        assert!(app.camera_nav_drag.is_some());

        let move_orbit = ame_ui::CanvasPointerEvent {
            phase: ame_ui::CanvasPointerPhase::Moved,
            kind: ame_ui::CanvasPointerKind::Mouse,
            pointer_id: None,
            x: 250.0,
            y: 230.0,
            button: Some(ame_ui::event::PointerButton::Primary),
            modifiers: alt_mod,
            space_pressed: false,
        };
        assert!(app.on_canvas_pointer_event("kagari.viewport", move_orbit));

        let up_orbit = ame_ui::CanvasPointerEvent {
            phase: ame_ui::CanvasPointerPhase::Up,
            kind: ame_ui::CanvasPointerKind::Mouse,
            pointer_id: None,
            x: 250.0,
            y: 230.0,
            button: Some(ame_ui::event::PointerButton::Primary),
            modifiers: alt_mod,
            space_pressed: false,
        };
        assert!(app.on_canvas_pointer_event("kagari.viewport", up_orbit));
        assert!(app.camera_nav_drag.is_none());

        let rot_after = app.project_model.active_composition().resolve_camera().transform.rotation.evaluate(0);
        assert!((rot_after[1] - (initial_rot[1] + 50.0 * 0.4)).abs() < 1e-3, "Yaw should rotate with X delta");
        assert!((rot_after[0] - (initial_rot[0] + 30.0 * 0.4)).abs() < 1e-3, "Pitch should rotate with Y delta");

        // 2. Alt + Right Drag -> Pan (Changes camera X/Y position)
        let down_pan = ame_ui::CanvasPointerEvent {
            phase: ame_ui::CanvasPointerPhase::Down,
            kind: ame_ui::CanvasPointerKind::Mouse,
            pointer_id: None,
            x: 200.0,
            y: 200.0,
            button: Some(ame_ui::event::PointerButton::Secondary),
            modifiers: alt_mod,
            space_pressed: false,
        };
        assert!(app.on_canvas_pointer_event("kagari.viewport", down_pan));

        let move_pan = ame_ui::CanvasPointerEvent {
            phase: ame_ui::CanvasPointerPhase::Moved,
            kind: ame_ui::CanvasPointerKind::Mouse,
            pointer_id: None,
            x: 240.0,
            y: 220.0,
            button: Some(ame_ui::event::PointerButton::Secondary),
            modifiers: alt_mod,
            space_pressed: false,
        };
        assert!(app.on_canvas_pointer_event("kagari.viewport", move_pan));

        let up_pan = ame_ui::CanvasPointerEvent {
            phase: ame_ui::CanvasPointerPhase::Up,
            kind: ame_ui::CanvasPointerKind::Mouse,
            pointer_id: None,
            x: 240.0,
            y: 220.0,
            button: Some(ame_ui::event::PointerButton::Secondary),
            modifiers: alt_mod,
            space_pressed: false,
        };
        assert!(app.on_canvas_pointer_event("kagari.viewport", up_pan));

        let pos_after_pan = app.project_model.active_composition().resolve_camera().transform.position.evaluate(0);
        assert!((pos_after_pan[0] - (initial_pos[0] - 40.0 * 1.5)).abs() < 1e-3, "Camera should pan along X");
        assert!((pos_after_pan[1] - (initial_pos[1] - 20.0 * 1.5)).abs() < 1e-3, "Camera should pan along Y");

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
            x: 200.0,
            y: 200.0,
            button: Some(ame_ui::event::PointerButton::Primary),
            modifiers: alt_shift_mod,
            space_pressed: false,
        };
        assert!(app.on_canvas_pointer_event("kagari.viewport", down_dolly));

        let move_dolly = ame_ui::CanvasPointerEvent {
            phase: ame_ui::CanvasPointerPhase::Moved,
            kind: ame_ui::CanvasPointerKind::Mouse,
            pointer_id: None,
            x: 200.0,
            y: 250.0,
            button: Some(ame_ui::event::PointerButton::Primary),
            modifiers: alt_shift_mod,
            space_pressed: false,
        };
        assert!(app.on_canvas_pointer_event("kagari.viewport", move_dolly));

        let up_dolly = ame_ui::CanvasPointerEvent {
            phase: ame_ui::CanvasPointerPhase::Up,
            kind: ame_ui::CanvasPointerKind::Mouse,
            pointer_id: None,
            x: 200.0,
            y: 250.0,
            button: Some(ame_ui::event::PointerButton::Primary),
            modifiers: alt_shift_mod,
            space_pressed: false,
        };
        assert!(app.on_canvas_pointer_event("kagari.viewport", up_dolly));

        let pos_after_dolly = app.project_model.active_composition().resolve_camera().transform.position.evaluate(0);
        assert!((pos_after_dolly[2] - (initial_pos[2] + 50.0 * 3.0)).abs() < 1e-3, "Camera should dolly along Z");
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
                kagari_vfx::core::timeline::LayerType::Solid { color: [1.0, 0.0, 0.0, 1.0] },
                comp.duration_frames,
            );
            let l2 = kagari_vfx::core::timeline::Layer::new(
                "target-layer".to_owned(),
                "Target Layer".to_owned(),
                kagari_vfx::core::timeline::LayerType::Solid { color: [0.0, 1.0, 0.0, 1.0] },
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
        assert_eq!(app.project_model.active_composition().layers[0].trackers.len(), 1);
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
        invoke(&mut app, "select_motion_tracker_target", Some(ActionValue::Text(target_layer_id.clone())));
        assert_eq!(app.selected_tracker_target_id.as_ref(), Some(&target_layer_id));

        // 4. Apply motion track to target
        invoke(&mut app, "apply_motion_track_to_target", None);
        let target_layer = app.project_model.active_composition().layers.iter().find(|l| l.id == target_layer_id).unwrap();
        match &target_layer.transform.position {
            kagari_vfx::core::timeline::Animatable::Animated(kfs) => {
                assert!(kfs.len() > 1, "Target layer should now have animated position keyframes from tracker");
            }
            _ => panic!("Target layer position should be animated after track apply"),
        }

        // 5. Add 3 more trackers to reach 4 points for Planar Corner Pin
        invoke(&mut app, "add_motion_tracker", None);
        invoke(&mut app, "add_motion_tracker", None);
        invoke(&mut app, "add_motion_tracker", None);
        assert_eq!(app.project_model.active_composition().layers[0].trackers.len(), 4);

        // 6. Apply 4-point planar track to target Corner Pin effect
        invoke(&mut app, "apply_planar_track_corner_pin", None);
        assert!(app.tracker_status.contains("4点プラナー コーナーピンを適用しました"));
        let target_layer_after_pin = app.project_model.active_composition().layers.iter().find(|l| l.id == target_layer_id).unwrap();
        let has_corner_pin = target_layer_after_pin.effects.iter().any(|e| matches!(e.effect_type, kagari_vfx::core::timeline::EffectType::CornerPin { .. }));
        assert!(has_corner_pin, "Target layer should have a Corner Pin effect created by planar tracker");
    }

    #[test]
    fn test_reload_ame_ui_runtime() {
        let mut app = AfterEffectsStudio::new().unwrap();
        invoke(&mut app, "back_to_editor", None);
        assert_eq!(app.screen, Screen::Editor);

        // Invoke reload_ame_ui
        invoke(&mut app, "reload_ame_ui", None);
        assert_eq!(app.asset_status, "AmeUI ホットリロード成功");
        assert_eq!(app.screen, Screen::Editor);
    }
}

