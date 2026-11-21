use super::*;

fn find_view_nodes<'a>(
    root: &'a ame_ui::view::ViewNode,
    predicate: &impl Fn(&ame_ui::view::ViewNode) -> bool,
) -> Vec<&'a ame_ui::view::ViewNode> {
    let mut stack = vec![root];
    let mut matches = Vec::new();
    while let Some(node) = stack.pop() {
        if predicate(node) {
            matches.push(node);
        }
        for child in node.children.iter().rev() {
            stack.push(child);
        }
    }
    matches
}

#[test]
fn ame_ui_virtualization_motion_and_app_delegation_e2e() {
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(|| {
            let mut app = AfterEffectsStudio::new().unwrap();

            // 1. Accessibility motion flags & App delegation
            let adaptation = App::accessibility_adaptation(&app);
            assert!(adaptation.exit_retention);
            assert!(adaptation.layout_motion);
            assert!(App::layout_motion_interpolation(&app));
            assert_eq!(App::window_title(&app), Some("KAGARI VFX".to_owned()));
            assert_eq!(
                App::state_json(&app, "home_page"),
                Some(serde_json::Value::Bool(true))
            );
            assert!(!App::window_modified(&app));

            // 2. CommandPaletteModal renders with GlassCard LiquidGlass Material
            invoke(&mut app, "back_to_editor", None);
            invoke(&mut app, "toggle_command_palette", None);
            let palette_tree = app.shared_view_tree().unwrap();
            let glass_nodes = find_view_nodes(&palette_tree.root, &|node| {
                matches!(
                    &node.kind,
                    ame_ui::view::ViewKind::Material { spec }
                        if spec.kind == ame_ui::lang::MaterialKind::LiquidGlass
                )
            });
            assert!(
                !glass_nodes.is_empty(),
                "CommandPaletteModal must emit a LiquidGlass Material node"
            );
            invoke(&mut app, "toggle_command_palette", None);

            // 3. HorizontalVirtualList for timeline_markers, layer_markers, and parent_candidates
            invoke(&mut app, "add_solid_layer", None);
            let parent_layer_id = app.selected_layer_id.clone().unwrap();
            invoke(&mut app, "add_solid_layer", None);
            let child_layer_id = app.selected_layer_id.clone().unwrap();
            assert_ne!(parent_layer_id, child_layer_id);

            app.current_frame = 24;
            invoke(&mut app, "add_timeline_marker", None);
            app.current_frame = 48;
            invoke(&mut app, "add_timeline_marker", None);
            app.current_frame = 12;
            invoke(&mut app, "add_layer_marker", None);
            app.current_frame = 36;
            invoke(&mut app, "add_layer_marker", None);

            invoke(&mut app, "workspace_scene", None);
            invoke(&mut app, "timeline_view_markers", None);

            let tree = app.shared_view_tree().unwrap();
            let list_with_action = |action_name: &str| {
                find_view_nodes(&tree.root, &|node| {
                    matches!(
                        &node.kind,
                        ame_ui::view::ViewKind::HorizontalVirtualList {
                            source_id: Some(_),
                            variable_width: true,
                            ..
                        }
                    ) && !find_view_nodes(node, &|child| {
                        matches!(
                            &child.kind,
                            ame_ui::view::ViewKind::Button { action_id: Some(act), .. }
                                if act == action_name
                        )
                    })
                    .is_empty()
                })
            };
            for (expected_source, action_name) in [
                ("timeline_markers", "go_timeline_marker"),
                ("layer_markers", "go_layer_marker"),
                ("parent_candidates", "set_layer_parent"),
            ] {
                let nodes = list_with_action(action_name);
                assert_eq!(
                    nodes.len(),
                    1,
                    "Expected one variable-width HorizontalVirtualList for `{expected_source}`"
                );
                assert!(
                    !nodes[0].children.is_empty(),
                    "HorizontalVirtualList `{expected_source}` should lower visible rows"
                );
            }

            let find_first_row_target = |action_name: &str| {
                let list_nodes = list_with_action(action_name);
                let first_child = &list_nodes[0].children[0];
                assert!(first_child.identity.segments.iter().any(|segment| matches!(
                    segment,
                    NodePathSegment::Key(StableKey::String(_))
                )));
                first_child.identity.clone()
            };

            let marker_target = find_first_row_target("go_timeline_marker");
            app.current_frame = 0;
            app.handle_action(HostAction {
                name: "go_timeline_marker".to_owned(),
                value: None,
                target: Some(marker_target),
            })
            .unwrap();
            assert_eq!(app.current_frame, 24);
            assert_eq!(app.selected_marker_frame, Some(24));

            let layer_marker_target = find_first_row_target("go_layer_marker");
            app.handle_action(HostAction {
                name: "go_layer_marker".to_owned(),
                value: None,
                target: Some(layer_marker_target),
            })
            .unwrap();
            assert_eq!(app.current_frame, 12);
            assert_eq!(app.selected_layer_marker_frame, Some(12));

            let parent_target = find_first_row_target("set_layer_parent");
            app.handle_action(HostAction {
                name: "set_layer_parent".to_owned(),
                value: None,
                target: Some(parent_target),
            })
            .unwrap();
            let child_layer = app
                .project_model
                .active_composition()
                .layers
                .iter()
                .find(|layer| layer.id == child_layer_id)
                .unwrap();
            assert_eq!(child_layer.parent_id.as_deref(), Some(parent_layer_id.as_str()));

            // 4. AudioMixerPanel host-windowed VirtualList + App::set_virtual_list_ranges
            {
                let comp = app.project_model.active_composition_mut();
                for i in 0..25 {
                    comp.layers.push(kagari_vfx::core::timeline::Layer::new(
                        format!("audio-track-{i}"),
                        format!("Audio Track {i}"),
                        kagari_vfx::core::timeline::LayerType::Audio {
                            path: format!("audio_{i}.wav"),
                            volume: kagari_vfx::core::property::Animatable::new_constant(0.0),
                        },
                        comp.duration_frames,
                    ));
                }
            }
            invoke(&mut app, "workspace_audio", None);
            assert_eq!(app.ui.virtual_list_size("audio_tracks"), Some(25));
            let Some(Value::List(initial_items)) = app.ui.state("audio_tracks") else {
                panic!("audio_tracks binding must be a List");
            };
            assert_eq!(
                initial_items.len(),
                10,
                "AudioMixerPanel must only materialize the initial window (0..10), not all 25 rows"
            );

            App::set_virtual_list_ranges(
                &mut app,
                &std::collections::HashMap::from([("audio_tracks".to_owned(), 10..22)]),
            );
            let is_audio_mixer_list = |node: &ame_ui::view::ViewNode| {
                matches!(
                    &node.kind,
                    ame_ui::view::ViewKind::VirtualList {
                        height: 300,
                        row_height: 54,
                        variable_height: false,
                        ..
                    }
                )
            };
            let audio_tree = app.shared_view_tree().unwrap();
            let audio_nodes = find_view_nodes(&audio_tree.root, &is_audio_mixer_list);
            assert_eq!(audio_nodes.len(), 1);
            let runtime_source_id = match &audio_nodes[0].kind {
                ame_ui::view::ViewKind::VirtualList {
                    total_count,
                    start_index,
                    source_id: Some(source_id),
                    ..
                } => {
                    assert_eq!(*total_count, 25);
                    assert_eq!(*start_index, 10);
                    source_id.clone()
                }
                other => panic!("unexpected AudioMixerPanel node kind: {other:?}"),
            };
            assert!(
                runtime_source_id.starts_with("ame-vl-"),
                "expected AmeRuntime-assigned virtual list source_id, got {runtime_source_id}"
            );
            assert_eq!(audio_nodes[0].children.len(), 12);

            App::set_virtual_list_ranges(
                &mut app,
                &std::collections::HashMap::from([(runtime_source_id, 12..25)]),
            );
            let scrolled_audio_tree = app.shared_view_tree().unwrap();
            let scrolled_audio_nodes =
                find_view_nodes(&scrolled_audio_tree.root, &is_audio_mixer_list);
            assert_eq!(scrolled_audio_nodes.len(), 1);
            assert!(matches!(
                &scrolled_audio_nodes[0].kind,
                ame_ui::view::ViewKind::VirtualList {
                    total_count: 25,
                    start_index: 12,
                    ..
                }
            ));
            assert_eq!(scrolled_audio_nodes[0].children.len(), 13);
            assert_eq!(
                App::state_json(&app, "has_audio_tracks"),
                Some(serde_json::Value::Bool(true))
            );

            // 4. UI_REDRAW_MESSAGE wakes AmeApplication cleanly without altering bindings
            let before_audio = App::state_json(&app, "has_audio_tracks");
            App::on_event(
                &mut app,
                &ame_ui::event::AmeEvent::AppMessage(
                    crate::ame_studio_app::UI_REDRAW_MESSAGE.to_owned(),
                ),
            );
            assert_eq!(App::state_json(&app, "has_audio_tracks"), before_audio);
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn quick_start_templates_wire_project_setup_presets_e2e() {
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(|| {
            let mut app = AfterEffectsStudio::new().unwrap();
            for (action, expected_w, expected_h, expected_fps) in [
                ("quick_start_1080p60", 1920, 1080, 60),
                ("quick_start_4k24", 3840, 2160, 24),
                ("quick_start_vertical", 1080, 1920, 60),
                ("quick_start_square", 1080, 1080, 30),
            ] {
                invoke(&mut app, "go_home", None);
                invoke(&mut app, action, None);
                assert_eq!(app.screen, Screen::Editor);
                let comp = app.project_model.active_composition();
                assert_eq!(comp.width, expected_w);
                assert_eq!(comp.height, expected_h);
                assert_eq!(comp.fps, expected_fps);
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn settings_search_filters_categories_and_selects_match() {
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(|| {
            let mut app = AfterEffectsStudio::new().unwrap();
            invoke(&mut app, "go_settings", None);
            assert_eq!(app.settings_category, 0);
            invoke(
                &mut app,
                "settings_search_changed",
                Some(ActionValue::Text("gpu".to_owned())),
            );
            assert_eq!(app.settings_category, 1);
            assert_eq!(
                app.ui.state("settings_show_performance"),
                Some(&Value::Bool(true))
            );
            assert_eq!(
                app.ui.state("settings_show_general"),
                Some(&Value::Bool(false))
            );
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn locale_switch_updates_home_translations_between_ja_and_en() {
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(|| {
            let view_tree_has_text = |root: &ame_ui::view::ViewNode, target: &str| {
                !find_view_nodes(root, &|node| match &node.kind {
                    ame_ui::view::ViewKind::Text(text)
                    | ame_ui::view::ViewKind::TextSized { text, .. }
                    | ame_ui::view::ViewKind::TextWeighted { text, .. } => text == target,
                    ame_ui::view::ViewKind::Button { label, .. } => label == target,
                    _ => false,
                })
                .is_empty()
            };

            let mut app = AfterEffectsStudio::new().unwrap();
            invoke(&mut app, "switch_locale_en", None);
            assert_eq!(app.ui_locale, "en");
            assert_eq!(
                app.ui.state("ui_locale"),
                Some(&Value::String("en".to_owned()))
            );
            let en_tree = app.shared_view_tree().unwrap();
            assert!(
                view_tree_has_text(&en_tree.root, "Start Creating"),
                "English locale should render `Start Creating` on Home"
            );
            assert!(
                view_tree_has_text(&en_tree.root, "New Project"),
                "English locale should render `New Project` on Home"
            );

            invoke(&mut app, "switch_locale_ja", None);
            assert_eq!(app.ui_locale, "ja");
            assert_eq!(
                app.ui.state("ui_locale"),
                Some(&Value::String("ja".to_owned()))
            );
            let ja_tree = app.shared_view_tree().unwrap();
            assert!(
                !view_tree_has_text(&ja_tree.root, "Start Creating"),
                "Japanese locale should no longer show `Start Creating` on Home"
            );
            assert!(
                view_tree_has_text(&ja_tree.root, "制作を始める"),
                "Japanese locale should restore `制作を始める` on Home"
            );
            assert!(
                view_tree_has_text(&ja_tree.root, "新規プロジェクト"),
                "Japanese locale should restore `新規プロジェクト` on Home"
            );
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn supported_locales_are_loaded_from_json_catalogs() {
    assert!(ame_sources::i18n::is_supported_locale("ja"));
    assert!(ame_sources::i18n::is_supported_locale("en"));
    assert!(!ame_sources::i18n::is_supported_locale("unknown_locale"));
    assert!(ame_sources::i18n::SUPPORTED_LOCALES.len() >= 2);
}

#[test]
fn canvas_accessibility_items_expose_composition_and_playhead() {
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(|| {
            let app = AfterEffectsStudio::new().unwrap();
            let viewport_items = App::canvas_accessibility_items(&app, "kagari.viewport");
            assert!(
                !viewport_items.is_empty(),
                "kagari.viewport should expose at least the active composition label"
            );
            assert_eq!(viewport_items[0].role, ame_ui::app::CanvasItemRole::Label);

            let timeline_items = App::canvas_accessibility_items(&app, "kagari.timeline");
            assert!(
                timeline_items.iter().any(|item| item.label == "Playhead"),
                "kagari.timeline should expose the Playhead slider item"
            );
        })
        .unwrap()
        .join()
        .unwrap();
}

