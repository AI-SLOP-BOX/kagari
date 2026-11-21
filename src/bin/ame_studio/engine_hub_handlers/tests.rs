use super::*;

#[cfg(test)]
mod engine_hub_tests {
    use super::*;

    #[test]
    fn engine_hub_actions_exercise_all_subsystems() {
        let mut app = AfterEffectsStudio::new().expect("studio app");
        for act in [
            "engine_analyze_color_scopes",
            "engine_analyze_film_grain",
            "engine_run_pro_keyer",
            "engine_run_deep_and_node_graph",
            "engine_run_roto_refine",
            "engine_run_mograph_cloner",
            "engine_run_sfm_and_lens",
            "engine_run_temporal_cleanplate",
            "engine_apply_composer_preset",
            "engine_apply_shape_pucker_twist",
            "engine_auto_proxy_step",
            "engine_run_cryptomatte",
            "engine_run_aces_check",
        ] {
            app.handle_action(HostAction {
                name: act.to_owned(),
                value: None,
                target: None,
            })
            .unwrap();
            assert!(
                !app.engine_hub_status.is_empty()
                    && app.engine_hub_status != "エンジン統合ハブ待機中",
                "action {act} did not update engine_hub_status"
            );
        }
    }

    #[test]
    fn node_graph_add_evaluate_and_remove_works() {
        let mut app = AfterEffectsStudio::new().expect("studio app");
        let base_nodes = app.compositor_graph.nodes.len();
        app.handle_action(HostAction {
            name: "node_graph_add_blur".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert_eq!(app.compositor_graph.nodes.len(), base_nodes + 1);
        app.handle_action(HostAction {
            name: "node_graph_evaluate".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert!(app.node_graph_status.contains("DAG評価完了"));
        app.handle_action(HostAction {
            name: "node_graph_remove_selected".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert_eq!(app.compositor_graph.nodes.len(), base_nodes);
    }

    #[test]
    fn generator_layers_and_optical_flow_time_remap_work() {
        let mut app = AfterEffectsStudio::new().expect("studio app");
        let base_layers = app.project_model.active_composition().layers.len();
        for act in [
            "add_generator_smpte",
            "add_generator_grid",
            "add_generator_gradient",
            "add_generator_cells",
            "add_generator_light_leak",
            "add_generator_film_grain",
            "add_generator_anamorphic_flare",
        ] {
            app.handle_action(HostAction {
                name: act.to_owned(),
                value: None,
                target: None,
            })
            .unwrap();
        }
        assert_eq!(
            app.project_model.active_composition().layers.len(),
            base_layers + 7
        );
        app.handle_action(HostAction {
            name: "time_remap_optical_flow".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert!(app.engine_hub_status.contains("Optical Flow"));
    }

    #[test]
    fn saber_beam_and_mograph_clone_layers_work() {
        let mut app = AfterEffectsStudio::new().expect("studio app");
        let base_layers = app.project_model.active_composition().layers.len();
        app.handle_action(HostAction {
            name: "add_saber_beam_layer".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert_eq!(
            app.project_model.active_composition().layers.len(),
            base_layers + 1
        );
        app.handle_action(HostAction {
            name: "mograph_clone_selected_layer".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert_eq!(
            app.project_model.active_composition().layers.len(),
            base_layers + 7
        );
    }

    #[test]
    fn extruded_3d_shape_and_matte_choker_work() {
        let mut app = AfterEffectsStudio::new().expect("studio app");
        app.handle_action(HostAction {
            name: "add_3d_extruded_shape".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        let lid = app.selected_layer_id.clone().unwrap();
        let layer = app
            .project_model
            .active_composition()
            .layers
            .iter()
            .find(|l| l.id == lid)
            .unwrap();
        assert!(layer.is_3d);
        let eff_before = layer.effects.len();
        app.handle_action(HostAction {
            name: "apply_matte_choker".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        let layer_after = app
            .project_model
            .active_composition()
            .layers
            .iter()
            .find(|l| l.id == lid)
            .unwrap();
        assert_eq!(layer_after.effects.len(), eff_before + 1);
    }

    #[test]
    fn tracked_3d_camera_and_sapphire_saber_nodes_work() {
        let mut app = AfterEffectsStudio::new().expect("studio app");
        let base_cams = app.project_model.active_composition().cameras.len();
        app.handle_action(HostAction {
            name: "add_tracked_3d_camera".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert_eq!(
            app.project_model.active_composition().cameras.len(),
            base_cams + 1
        );
        let cam_id = app.selected_layer_id.clone().unwrap();
        let cam_layer = app
            .project_model
            .active_composition()
            .layers
            .iter()
            .find(|l| l.id == cam_id)
            .unwrap();
        assert_eq!(cam_layer.name, "SfM トラックカメラ");
        assert!(cam_layer.transform_3d.position.keyframes().is_some());

        let base_nodes = app.compositor_graph.nodes.len();
        app.handle_action(HostAction {
            name: "node_graph_add_sapphire".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        app.handle_action(HostAction {
            name: "node_graph_add_saber".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        #[cfg(feature = "wgpu")]
        assert_eq!(app.compositor_graph.nodes.len(), base_nodes + 2);
    }

    #[test]
    fn studio_rig_deep_merge_and_wiggle_keyframes_work() {
        let mut app = AfterEffectsStudio::new().expect("studio app");
        let base_layers = app.project_model.active_composition().layers.len();
        app.handle_action(HostAction {
            name: "add_3d_studio_rig".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert!(app.project_model.active_composition().layers.len() >= base_layers + 3);

        app.handle_action(HostAction {
            name: "add_solid_layer".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        app.handle_action(HostAction {
            name: "apply_wiggle_keyframes".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        let lid = app.selected_layer_id.clone().unwrap();
        let layer = app
            .project_model
            .active_composition()
            .layers
            .iter()
            .find(|l| l.id == lid)
            .unwrap();
        assert!(layer.transform.position.keyframes().is_some());
        assert!(layer.transform.rotation.keyframes().is_some());

        let base_nodes = app.compositor_graph.nodes.len();
        app.handle_action(HostAction {
            name: "node_graph_add_deep_merge".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert_eq!(app.compositor_graph.nodes.len(), base_nodes + 1);
    }

    #[test]
    fn node_graph_param_tuning_and_mesh_warp_work() {
        let mut app = AfterEffectsStudio::new().expect("studio app");
        app.handle_action(HostAction {
            name: "node_graph_add_blur".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        app.handle_action(HostAction {
            name: "node_graph_param_up".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert!(app.node_graph_status.contains("radius=6.0px"));

        app.handle_action(HostAction {
            name: "add_solid_layer".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        let lid = app.selected_layer_id.clone().unwrap();
        let before = app
            .project_model
            .active_composition()
            .layers
            .iter()
            .find(|l| l.id == lid)
            .unwrap()
            .effects
            .len();
        app.handle_action(HostAction {
            name: "add_mesh_warp".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        let after = app
            .project_model
            .active_composition()
            .layers
            .iter()
            .find(|l| l.id == lid)
            .unwrap()
            .effects
            .len();
        assert_eq!(after, before + 1);
    }

    #[test]
    fn node_graph_bake_and_extra_effects_work() {
        let mut app = AfterEffectsStudio::new().expect("studio app");
        let base_layers = app.project_model.active_composition().layers.len();
        app.handle_action(HostAction {
            name: "node_graph_bake_to_layer".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert_eq!(
            app.project_model.active_composition().layers.len(),
            base_layers + 1
        );
        assert!(app.node_graph_status.contains("書き出し完了"));

        for act in ["add_displacement_map", "add_compound_blur"] {
            app.handle_action(HostAction {
                name: act.to_owned(),
                value: None,
                target: None,
            })
            .unwrap();
        }
        let lid = app.selected_layer_id.clone().unwrap();
        let eff_len = app
            .project_model
            .active_composition()
            .layers
            .iter()
            .find(|l| l.id == lid)
            .unwrap()
            .effects
            .len();
        assert!(eff_len >= 2);
    }

    #[test]
    fn cleanplate_bake_and_eight_effects_work() {
        let mut app = AfterEffectsStudio::new().expect("studio app");
        let base_layers = app.project_model.active_composition().layers.len();
        app.handle_action(HostAction {
            name: "bake_cleanplate_layer".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert_eq!(
            app.project_model.active_composition().layers.len(),
            base_layers + 1
        );
        for act in [
            "add_crt_scanlines",
            "add_polar_coordinates",
            "add_wave_warp",
            "add_mosaic",
            "add_emboss",
            "add_bevel_alpha",
            "add_median_filter",
            "add_sobel_edges",
        ] {
            app.handle_action(HostAction {
                name: act.to_owned(),
                value: None,
                target: None,
            })
            .unwrap();
        }
        let lid = app.selected_layer_id.clone().unwrap();
        let eff_len = app
            .project_model
            .active_composition()
            .layers
            .iter()
            .find(|l| l.id == lid)
            .unwrap()
            .effects
            .len();
        assert_eq!(eff_len, 8);
    }

    #[test]
    fn effect_category_tabs_switch_cleanly() {
        let mut app = AfterEffectsStudio::new().expect("studio app");
        for (act, expected) in [
            ("effect_cat_color", "color"),
            ("effect_cat_blur", "blur"),
            ("effect_cat_distort", "distort"),
            ("effect_cat_key", "key"),
            ("effect_cat_all", "all"),
        ] {
            app.handle_action(HostAction {
                name: act.to_owned(),
                value: None,
                target: None,
            })
            .unwrap();
            assert_eq!(app.effect_category, expected);
        }
        app.handle_action(HostAction {
            name: "node_graph_add_blur".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert_eq!(app.compositor_graph.nodes.len(), 4);
        app.handle_action(HostAction {
            name: "node_graph_reset".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert_eq!(app.compositor_graph.nodes.len(), 3);
    }

    #[test]
    fn node_graph_reorder_works() {
        let mut app = AfterEffectsStudio::new().expect("studio app");
        app.handle_action(HostAction {
            name: "node_graph_add_blur".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        let before = app.compositor_graph.topological_order().unwrap();
        assert_eq!(before, vec![1, 2, 3, 999]);
        app.handle_action(HostAction {
            name: "node_graph_move_left".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        let after = app.compositor_graph.topological_order().unwrap();
        assert_eq!(after, vec![1, 3, 2, 999]);
        app.handle_action(HostAction {
            name: "node_graph_move_right".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        let back = app.compositor_graph.topological_order().unwrap();
        assert_eq!(back, vec![1, 2, 3, 999]);
    }

    #[test]
    fn node_graph_openfx_node_works() {
        let mut app = AfterEffectsStudio::new().expect("studio app");
        let base_nodes = app.compositor_graph.nodes.len();
        app.handle_action(HostAction {
            name: "node_graph_add_openfx".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert_eq!(app.compositor_graph.nodes.len(), base_nodes + 1);
        assert!(app.node_graph_status.contains("OFX:"));
    }

    #[test]
    fn export_qc_and_corner_pin_colorama_work() {
        let mut app = AfterEffectsStudio::new().expect("studio app");
        app.handle_action(HostAction {
            name: "add_solid_layer".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        let lid = app.selected_layer_id.clone().unwrap();
        app.handle_action(HostAction {
            name: "add_corner_pin".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        app.handle_action(HostAction {
            name: "add_colorama".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        let eff_len = app
            .project_model
            .active_composition()
            .layers
            .iter()
            .find(|l| l.id == lid)
            .unwrap()
            .effects
            .len();
        assert_eq!(eff_len, 2);

        app.handle_action(HostAction {
            name: "export_run_qc".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert!(app.render_status.contains("DataWindow="));
    }

    #[test]
    fn node_graph_drag_wire_over_merge_and_live_preview_work() {
        let mut app = AfterEffectsStudio::new().expect("studio app");
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
        app.node_graph_bounds = Some((0.0, 0.0, 600.0, 120.0));
        let rects = app.node_graph_layout_rects(0.0, 0.0, 600.0, 120.0);
        let (id1, r1) = rects[0];
        let (id2, r2) = rects[1];
        // Drag node 2 freely
        assert!(app.handle_node_graph_canvas_pointer(make_event(
            CanvasPointerPhase::Down,
            r2.x0 + 20.0,
            r2.y0 + 20.0,
        )));
        assert!(app.handle_node_graph_canvas_pointer(make_event(
            CanvasPointerPhase::Moved,
            260.0,
            70.0,
        )));
        assert!(app.handle_node_graph_canvas_pointer(make_event(
            CanvasPointerPhase::Up,
            260.0,
            70.0,
        )));
        assert!(app.node_graph_positions.contains_key(&id2));

        // Wire drag from node 1 output to node 2 input
        let cy1 = (r1.y0 + r1.y1) * 0.5;
        assert!(app.handle_node_graph_canvas_pointer(make_event(
            CanvasPointerPhase::Down,
            r1.x1,
            cy1,
        )));
        assert!(app.node_graph_drag_wire.is_some());
        assert!(app.handle_node_graph_canvas_pointer(make_event(
            CanvasPointerPhase::Up,
            245.0,
            60.0,
        )));
        assert!(app.node_graph_status.contains("結線") || app.node_graph_status.contains("接続"));

        // Auto layout, OverMerge, and Live Preview toggle
        app.handle_action(HostAction {
            name: "node_graph_auto_layout".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert!(app.node_graph_positions.is_empty());
        app.handle_action(HostAction {
            name: "node_graph_add_over_merge".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        app.handle_action(HostAction {
            name: "toggle_node_graph_preview".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert!(app.node_graph_live_preview);
        app.handle_action(HostAction {
            name: "export_run_qc".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert!(app.render_status.contains("DataWindow="));
        let _ = id1;
    }

    #[test]
    fn demo_vfx_scene_and_hub_toggle_work() {
        let mut app = AfterEffectsStudio::new().expect("studio app");
        assert!(!app.engine_hub_expanded);
        app.handle_action(HostAction {
            name: "engine_hub_toggle".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert!(app.engine_hub_expanded);

        let base_layers = app.project_model.active_composition().layers.len();
        app.handle_action(HostAction {
            name: "load_demo_vfx_scene".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert_eq!(app.screen, Screen::Editor);
        assert!(app.project_model.active_composition().layers.len() >= base_layers + 4);
        assert!(app.engine_hub_status.contains("VFX サンプルシーン構築完了"));
    }

    #[test]
    fn corner_pin_viewport_gizmo_drag_and_tutorial_work() {
        let mut app = AfterEffectsStudio::new().expect("studio app");
        app.handle_action(HostAction {
            name: "load_demo_vfx_scene".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert_eq!(app.tutorial_step, 1);
        for expected in [2usize, 3, 4, 0] {
            app.handle_action(HostAction {
                name: "tutorial_next".to_owned(),
                value: None,
                target: None,
            })
            .unwrap();
            assert_eq!(app.tutorial_step, expected);
        }

        app.handle_action(HostAction {
            name: "add_solid_layer".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        app.handle_action(HostAction {
            name: "add_corner_pin".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
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
        // Grab top-left corner at (0, 0) and drag to (192, 108)
        assert!(app.handle_viewport_canvas_pointer(make_event(
            CanvasPointerPhase::Down,
            0.0,
            0.0,
        )));
        assert_eq!(app.corner_pin_drag, Some(0));
        assert!(app.handle_viewport_canvas_pointer(make_event(
            CanvasPointerPhase::Moved,
            192.0,
            108.0,
        )));
        assert!(app.handle_viewport_canvas_pointer(make_event(
            CanvasPointerPhase::Up,
            192.0,
            108.0,
        )));
        assert_eq!(app.corner_pin_drag, None);

        // Live compositor viewport helper check
        let sample = vec![128u8; 8 * 8 * 4];
        let out = app.evaluate_compositor_graph_rgba8(8, 8, sample.clone());
        assert_eq!(out.len(), sample.len());
    }
}
