use super::*;

impl AfterEffectsStudio {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let ui = compile_with_sources(
            "src/ui/ame/kagari_studio.ame",
            ame_sources::STUDIO_ROOT,
            ame_sources::STUDIO_MODULES,
        )?;
        let recents = load_recent_items();
        let saved = read_preferences();
        let workspace_left_ratios = read_workspace_ratios(
            &saved,
            "workspace_left_ratios",
            Some("workspace_left_ratio"),
            WORKSPACE_LEFT_DEFAULTS,
            0.16,
            0.36,
        );
        let workspace_center_ratios = read_workspace_ratios(
            &saved,
            "workspace_center_ratios",
            Some("workspace_center_ratio"),
            WORKSPACE_CENTER_DEFAULTS,
            0.54,
            0.84,
        );
        let workspace_vertical_ratios = read_workspace_ratios(
            &saved,
            "workspace_vertical_ratios",
            None,
            [WORKSPACE_VERTICAL_DEFAULT; 15],
            0.20,
            0.80,
        );
        let show_home_on_startup = saved
            .get("startup_home")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);
        let autosave_secs = saved
            .get("autosave_secs")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(300);
        let memory_limit_gb = saved
            .get("cache_mb")
            .and_then(serde_json::Value::as_u64)
            .map(|mb| (mb as f64 / 1024.0).clamp(1.0, 64.0))
            .unwrap_or(12.0);
        let undo_steps = saved
            .get("undo_steps")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(50)
            .clamp(10, 500) as usize;
        let recovery_slots = saved
            .get("recovery_slots")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(10)
            .clamp(1, 50) as usize;
        let restore_previous_state = saved
            .get("restore_previous_state")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);
        let confirm_close_project = saved
            .get("confirm_close_project")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);
        let confirm_exit = saved
            .get("confirm_exit")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);
        let adaptive_preview = saved
            .get("adaptive_preview")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);
        let audio_preview = saved
            .get("audio_preview")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);
        let gpu_enabled = saved
            .get("gpu_compute_enabled")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);
        let configured_cache_path = saved
            .get("disk_cache_directory")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let cache_path = if configured_cache_path.is_empty() {
            kagari_vfx::core::frame_cache::disk_cache::cache_directory()
                .display()
                .to_string()
        } else {
            configured_cache_path.clone()
        };
        let cache_limit = saved
            .get("disk_cache_gb")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(100)
            .clamp(10, 500)
            .to_string();
        kagari_vfx::core::frame_cache::disk_cache::set_max_disk_bytes(
            cache_limit.parse::<u64>().unwrap_or(100) * 1024 * 1024 * 1024,
        );
        let cache_directory = if configured_cache_path.trim().is_empty() {
            None
        } else {
            Some(expand_home_path(&configured_cache_path))
        };
        let _ = kagari_vfx::core::frame_cache::disk_cache::set_cache_directory(cache_directory);
        let cache_usage_bytes = kagari_vfx::core::frame_cache::disk_cache::current_disk_usage();
        let mut assets = AssetStore::default();
        for index in 0..4 {
            let preview = recent_preview(recents.get(index).map(|item| item.path.as_path()), index);
            assets.insert(
                AssetKey::new(format!("kagari.home.recent.{}", index + 1), 1)?,
                Ok(AssetData::new(preview.0, preview.1)?),
            );
        }
        assets.insert(
            AssetKey::new("kagari.logo", 1)?,
            Ok(AssetData::new(
                AssetMime::Webp,
                include_bytes!("../../../assets/kagari_logo.webp").as_slice(),
            )?),
        );
        for (id, svg, color) in [
            ("kagari.nav.home", ame_studio_icons::SVG_HOME, "#aeb7bd"),
            (
                "kagari.nav.home.active",
                ame_studio_icons::SVG_HOME,
                "#ff8a36",
            ),
            (
                "kagari.nav.projects",
                ame_studio_icons::SVG_FOLDER,
                "#aeb7bd",
            ),
            (
                "kagari.nav.projects.active",
                ame_studio_icons::SVG_FOLDER,
                "#ff8a36",
            ),
            (
                "kagari.nav.documents",
                ame_studio_icons::SVG_DOCUMENT,
                "#aeb7bd",
            ),
            (
                "kagari.nav.documents.active",
                ame_studio_icons::SVG_DOCUMENT,
                "#ff8a36",
            ),
            (
                "kagari.nav.settings",
                ame_studio_icons::SVG_SETTINGS,
                "#aeb7bd",
            ),
            (
                "kagari.nav.settings.active",
                ame_studio_icons::SVG_SETTINGS,
                "#ff8a36",
            ),
            ("kagari.nav.exit", ame_studio_icons::SVG_POWER, "#aeb7bd"),
            ("kagari.home.plus", ame_studio_icons::SVG_PLUS, "#ffffff"),
            (
                "kagari.home.open",
                ame_studio_icons::SVG_OPEN_FOLDER,
                "#dce0e4",
            ),
            ("kagari.docs.intro", ame_studio_icons::SVG_HOME, "#aeb7bd"),
            (
                "kagari.docs.layers",
                ame_studio_icons::SVG_LAYERS,
                "#aeb7bd",
            ),
            (
                "kagari.docs.effects",
                ame_studio_icons::SVG_EFFECTS,
                "#aeb7bd",
            ),
            (
                "kagari.docs.tracking",
                ame_studio_icons::SVG_TRACKING,
                "#aeb7bd",
            ),
            (
                "kagari.docs.export",
                ame_studio_icons::SVG_EXPORT,
                "#aeb7bd",
            ),
            (
                "kagari.preferences.general",
                ame_studio_icons::SVG_SETTINGS,
                "#aeb7bd",
            ),
            (
                "kagari.preferences.general.active",
                ame_studio_icons::SVG_SETTINGS,
                "#ff8a36",
            ),
            (
                "kagari.preferences.projects",
                ame_studio_icons::SVG_FOLDER,
                "#aeb7bd",
            ),
            (
                "kagari.preferences.projects.active",
                ame_studio_icons::SVG_FOLDER,
                "#ff8a36",
            ),
            (
                "kagari.preferences.cache",
                ame_studio_icons::SVG_DATABASE,
                "#aeb7bd",
            ),
            (
                "kagari.preferences.cache.active",
                ame_studio_icons::SVG_DATABASE,
                "#ff8a36",
            ),
            (
                "kagari.preferences.performance",
                ame_studio_icons::SVG_BOLT,
                "#aeb7bd",
            ),
            (
                "kagari.preferences.performance.active",
                ame_studio_icons::SVG_BOLT,
                "#ff8a36",
            ),
            (
                "kagari.preferences.shortcuts",
                ame_studio_icons::SVG_KEYBOARD,
                "#aeb7bd",
            ),
            (
                "kagari.preferences.shortcuts.active",
                ame_studio_icons::SVG_KEYBOARD,
                "#ff8a36",
            ),
            (
                "kagari.preferences.interface",
                ame_studio_icons::SVG_MONITOR,
                "#aeb7bd",
            ),
            (
                "kagari.preferences.interface.active",
                ame_studio_icons::SVG_MONITOR,
                "#ff8a36",
            ),
            (
                "kagari.preferences.plugins",
                ame_studio_icons::SVG_PUZZLE,
                "#aeb7bd",
            ),
            (
                "kagari.preferences.plugins.active",
                ame_studio_icons::SVG_PUZZLE,
                "#ff8a36",
            ),
            (
                "kagari.preferences.license",
                ame_studio_icons::SVG_LOCK,
                "#aeb7bd",
            ),
            (
                "kagari.preferences.license.active",
                ame_studio_icons::SVG_LOCK,
                "#ff8a36",
            ),
            ("kagari.icon.undo", ame_studio_icons::SVG_UNDO, "#d0d7de"),
            ("kagari.icon.redo", ame_studio_icons::SVG_REDO, "#d0d7de"),
            ("kagari.icon.refresh", ame_studio_icons::SVG_REFRESH, "#d0d7de"),
            ("kagari.icon.keyframe", ame_studio_icons::SVG_KEYFRAME_DIAMOND, "#58a6ff"),
            ("kagari.icon.keyframe.prev", ame_studio_icons::SVG_KEYFRAME_PREV, "#d0d7de"),
            ("kagari.icon.keyframe.next", ame_studio_icons::SVG_KEYFRAME_NEXT, "#d0d7de"),
            ("kagari.icon.ease", ame_studio_icons::SVG_EASE_CURVE, "#f58c4b"),
            ("kagari.icon.adjustment", ame_studio_icons::SVG_ADJUSTMENT, "#79c0ff"),
            ("kagari.icon.particle", ame_studio_icons::SVG_PARTICLE, "#d2a8ff"),
            ("kagari.icon.camera", ame_studio_icons::SVG_CAMERA, "#7ee787"),
            ("kagari.icon.light", ame_studio_icons::SVG_LIGHT, "#ffa657"),
            ("kagari.icon.graph", ame_studio_icons::SVG_GRAPH, "#58a6ff"),
            ("kagari.icon.motion_blur", ame_studio_icons::SVG_MOTION_BLUR, "#d0d7de"),
            ("kagari.icon.solo", ame_studio_icons::SVG_SOLO, "#f58c4b"),
            ("kagari.icon.play", ame_studio_icons::SVG_PLAY, "#ffffff"),
            ("kagari.icon.pause", ame_studio_icons::SVG_PAUSE, "#f58c4b"),
            ("kagari.icon.stop", ame_studio_icons::SVG_STOP, "#f85149"),
            ("kagari.icon.rewind", ame_studio_icons::SVG_REWIND, "#d0d7de"),
            ("kagari.icon.forward", ame_studio_icons::SVG_FORWARD, "#d0d7de"),
            ("kagari.icon.step_back", ame_studio_icons::SVG_PREV_FRAME, "#d0d7de"),
            ("kagari.icon.step_forward", ame_studio_icons::SVG_NEXT_FRAME, "#d0d7de"),
            ("kagari.icon.repeat", ame_studio_icons::SVG_REPEAT, "#d0d7de"),
            ("kagari.icon.repeat.active", ame_studio_icons::SVG_REPEAT, "#ff8a36"),
        ] {
            let svg = svg.replace("stroke=\"white\"", &format!("stroke=\"{color}\""));
            assets.insert(
                AssetKey::new(id, 1)?,
                Ok(AssetData::new(AssetMime::Svg, svg.into_bytes())?),
            );
        }
        let (waveform_request_tx, waveform_result_rx) = spawn_waveform_decoder();
        let (asset_render_preview_request_tx, asset_render_preview_result_rx) =
            spawn_asset_preview_renderer();
        let (viewport_preview_request_tx, viewport_preview_result_rx) =
            spawn_viewport_preview_renderer();
        let text_font_families = kagari_vfx::core::font_rasterizer::with_font_rasterizer(|rasterizer| {
            rasterizer.available_families()
        });
        let mut app = Self {
            ui,
            assets,
            asset_preview_image: None,
            asset_preview_failed_asset_id: None,
            asset_render_preview_cache: std::collections::HashMap::new(),
            asset_render_preview_requested: std::collections::HashSet::new(),
            asset_render_preview_pending: std::collections::HashMap::new(),
            asset_render_preview_request_tx,
            asset_render_preview_result_rx,
            text_font_families,
            viewport_preview_request_tx,
            viewport_preview_result_rx,
            viewport_preview_inflight: false,
            viewport_preview_failed: None,
            screen: if show_home_on_startup {
                Screen::Home
            } else {
                Screen::Editor
            },
            docs_detail_page: false,
            docs_title: "基本操作".to_owned(),
            docs_body: "プロジェクトの作成、コンポジションの設定、レイヤーの追加について"
                .to_owned(),
            clear_cache_confirm: false,
            settings_category: 0,
            openfx_plugins: Vec::new(),
            openfx_search_paths: String::new(),
            settings_open: false,
            exit_confirm_open: false,
            project_switch_confirm_open: false,
            composition_remove_confirm_open: false,
            layer_remove_confirm_open: false,
            pending_project_path: None,
            pending_new_project: false,
            composition_settings_status: String::new(),
            settings_snapshot: None,
            restore_previous_state,
            confirm_close_project,
            confirm_exit,
            adaptive_preview,
            audio_preview,
            show_home_on_startup,
            autosave_secs,
            gpu_enabled,
            memory_limit_gb,
            undo_steps,
            recovery_slots,
            cache_path,
            cache_path_status: String::new(),
            settings_feedback: String::new(),
            cache_limit,
            cache_usage_bytes,
            compact: false,
            narrow: false,
            wide: true,
            render_queue: false,
            workspace: Workspace::Default,
            selected_tool: "Selection",
            layer_name: "Emitter · Main".to_owned(),
            search: String::new(),
            project_search: String::new(),
            current_time: format_timecode(132, 30),
            zoom_manually_adjusted: false,
            current_frame: 132,
            timeline_width: 0.0,
            graph_canvas_height: 180.0,
            timeline_zoom: 1.0,
            timeline_start_frame: 0.0,
            workspace_left_ratios,
            workspace_center_ratios,
            workspace_vertical_ratios,
            preview_revision: 0,
            preview_cache: std::collections::HashMap::new(),
            viewport_mapping: None,
            viewport_drag: None,
            mask_vertex_drag: None,
            viewport_pan: [0.0, 0.0],
            viewport_pan_drag: None,
            viewport_pen_start: None,
            viewport_pen_current: None,
            light_gizmo_drag: None,
            camera_nav_drag: None,
            timeline_scrubbing: false,
            timeline_clip_drag: None,
            timeline_work_area_drag: None,
            timeline_snap_frame: None,
            timeline_view: TimelineView::All,
            properties_section: PropertiesSection::Transform,
            timeline_snap_enabled: true,
            graph_channel: GraphChannel::PositionX,
            graph_key_drag: None,
            flowchart_wire_drag: None,
            hide_shy_layers: false,
            waveform_cache: std::collections::HashMap::new(),
            waveform_requested: std::collections::HashSet::new(),
            waveform_failed: std::collections::HashSet::new(),
            waveform_request_tx,
            waveform_result_rx,
            playback_anchor: None,
            last_autosave_at: std::time::Instant::now(),
            zoom: "100%".to_owned(),
            position_x: 960.0,
            position_y: 540.0,
            anchor_x: 0.0,
            anchor_y: 0.0,
            rotation_degrees: 0.0,
            scale_x: 20.0,
            scale_y: 20.0,
            opacity: 100.0,
            effect_search: String::new(),
            text_font_search: String::new(),
            asset_status: "素材を読み込むとシーンに追加されます。".to_owned(),
            asset_folder_id: None,
            new_asset_folder_name: String::new(),
            project_asset_search: String::new(),
            project_asset_search_all_folders: false,
            project_assets_sort_by_type: false,
            selected_project_asset_id: None,
            selected_layer: true,
            selected_layer_id: None,
            selected_mask_id: None,
            selected_tracker_idx: 0,
            selected_tracker_target_id: None,
            tracker_status: "トラッカーを追加して解析を開始".to_owned(),
            collapsed_scene_layers: std::collections::HashSet::new(),
            align_selected_layer_ids: std::collections::HashSet::new(),
            align_to_composition: false,
            selected_effect_id: None,
            graph_effect_parameter_id: None,
            selected_marker_frame: None,
            selected_layer_marker_frame: None,
            output_path: "~/Movies/Particle_Study.png".to_owned(),
            still_overwrite_pending: false,
            still_status: String::new(),
            video_output_path: "~/Movies/Kagari_export.mp4".to_owned(),
            video_export_codec_idx: 0,
            video_overwrite_pending: false,
            project_name: "無題のエフェクト".to_owned(),
            project_file_path: None,
            save_status: String::new(),
            setup_name: "無題のプロジェクト".to_owned(),
            setup_width: "1920".to_owned(),
            setup_height: "1080".to_owned(),
            setup_fps: "30".to_owned(),
            setup_duration_seconds: "10".to_owned(),
            setup_error: String::new(),
            setup_return_screen: if show_home_on_startup {
                Screen::Home
            } else {
                Screen::Editor
            },
            project_model: kagari_vfx::core::timeline::Project::empty_for_new_project(),
            project_generation: 0,
            project_fps: 30,
            history: kagari_vfx::core::history::ProjectHistory::new(
                kagari_vfx::core::timeline::Project::empty_for_new_project(),
            ),
            history_interaction_snapshot: None,
            is_playing: false,
            playback_loop: false,
            playback_range: None,
            render_progress: 0,
            render_status: "未実行".to_owned(),
            render_events: None,
            render_cancel: None,
            render_refresh_ticks: 0,
            video_import: None,
            proxy_generation: None,
            proxy_status: String::new(),
            proxy_progress: 0,
            recent_projects: recents,
            exit_requested: false,
        };
        app.history.set_max_history_entries(app.undo_steps);
        if !app.show_home_on_startup && app.restore_previous_state {
            let recent_paths = app
                .recent_projects
                .iter()
                .map(|item| item.path.clone())
                .collect::<Vec<_>>();
            for path in recent_paths {
                let name = path
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .unwrap_or("Project");
                if app.load_project_file(&path, name).is_ok() {
                    app.screen = Screen::Editor;
                    break;
                }
            }
        }
        kagari_vfx::core::compute_pipeline::set_gpu_effects_enabled(app.gpu_enabled);
        app.sync_bindings()?;
        Ok(app)
    }
}
