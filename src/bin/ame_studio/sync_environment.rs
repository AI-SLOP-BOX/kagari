use super::*;
use ame_studio_preferences::format_cache_size;

impl AfterEffectsStudio {
    pub(crate) fn sync_environment_bindings(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for (name, screen) in [
            ("home_page", Screen::Home),
            ("projects_page", Screen::Projects),
            ("activity_page", Screen::Activity),
            ("documents_page", Screen::Documents),
            ("project_setup_page", Screen::ProjectSetup),
            ("editor_page", Screen::Editor),
            ("export_page", Screen::Export),
        ] {
            self.ui
                .set_binding(name, Value::Bool(self.screen == screen))?;
        }
        self.ui
            .set_binding("settings_page", Value::Bool(self.settings_open))?;
        self.ui
            .set_binding("exit_confirm_open", Value::Bool(self.exit_confirm_open))?;
        self.ui.set_binding(
            "project_switch_confirm_open",
            Value::Bool(self.project_switch_confirm_open),
        )?;
        self.ui.set_binding(
            "composition_remove_confirm_open",
            Value::Bool(self.composition_remove_confirm_open),
        )?;
        self.ui.set_binding(
            "layer_remove_confirm_open",
            Value::Bool(self.layer_remove_confirm_open),
        )?;
        self.ui
            .set_binding("settings_nav_active", Value::Bool(self.settings_open))?;
        self.ui.set_binding(
            "restore_previous_state",
            Value::Bool(self.restore_previous_state),
        )?;
        self.ui.set_binding(
            "confirm_close_project",
            Value::Bool(self.confirm_close_project),
        )?;
        self.ui
            .set_binding("confirm_exit", Value::Bool(self.confirm_exit))?;
        self.ui
            .set_binding("adaptive_preview", Value::Bool(self.adaptive_preview))?;
        self.ui
            .set_binding("audio_preview", Value::Bool(self.audio_preview))?;

        let cache_limit_bytes = self
            .cache_limit
            .parse::<u64>()
            .unwrap_or(100)
            .saturating_mul(1024 * 1024 * 1024);
        let usage_percent = self
            .cache_usage_bytes
            .saturating_mul(100)
            .checked_div(cache_limit_bytes)
            .unwrap_or(0)
            .min(100) as i64;
        self.ui
            .set_binding("cache_usage_progress", Value::Int(usage_percent))?;
        self.ui.set_binding(
            "cache_usage_text",
            Value::String(format_cache_size(self.cache_usage_bytes)),
        )?;
        self.ui.set_binding("compact", Value::Bool(self.compact))?;
        self.ui.set_binding("narrow", Value::Bool(self.narrow))?;
        self.ui.set_binding("wide", Value::Bool(self.wide))?;
        self.ui
            .set_binding("docs_detail_page", Value::Bool(self.docs_detail_page))?;
        self.ui
            .set_binding("docs_title", Value::String(self.docs_title.clone()))?;
        self.ui
            .set_binding("docs_body", Value::String(self.docs_body.clone()))?;
        self.ui
            .set_binding("clear_cache_confirm", Value::Bool(self.clear_cache_confirm))?;
        for (name, index) in [
            ("settings_general", 0),
            ("settings_projects", 1),
            ("settings_cache", 2),
            ("settings_performance", 3),
            ("settings_shortcuts", 4),
            ("settings_interface", 5),
            ("settings_plugins", 6),
            ("settings_license", 7),
        ] {
            self.ui
                .set_binding(name, Value::Bool(self.settings_category == index))?;
        }

        let openfx_plugins = self
            .openfx_plugins
            .iter()
            .map(|plugin| {
                Value::Record(std::collections::BTreeMap::from([
                    ("name".to_owned(), Value::String(plugin.name.clone())),
                    ("path".to_owned(), Value::String(plugin.path.clone())),
                ]))
            })
            .collect();
        self.ui
            .set_binding("openfx_plugins", Value::List(openfx_plugins))?;
        self.ui.set_binding(
            "has_openfx_plugins",
            Value::Bool(!self.openfx_plugins.is_empty()),
        )?;
        self.ui.set_binding(
            "openfx_search_paths",
            Value::String(self.openfx_search_paths.clone()),
        )?;
        self.ui.set_binding(
            "show_home_on_startup",
            Value::Bool(self.show_home_on_startup),
        )?;
        self.ui.set_binding(
            "autosave_interval",
            Value::String(self.autosave_secs.to_string()),
        )?;
        self.ui
            .set_binding("memory_limit_gb", Value::Float(self.memory_limit_gb))?;
        self.ui
            .set_binding("undo_steps", Value::Float(self.undo_steps as f64))?;
        self.ui
            .set_binding("recovery_slots", Value::Float(self.recovery_slots as f64))?;
        self.ui
            .set_binding("gpu_enabled", Value::Bool(self.gpu_enabled))?;
        self.ui
            .set_binding("cache_path", Value::String(self.cache_path.clone()))?;
        self.ui.set_binding(
            "cache_path_status",
            Value::String(self.cache_path_status.clone()),
        )?;
        self.ui.set_binding(
            "settings_feedback",
            Value::String(self.settings_feedback.clone()),
        )?;
        self.ui
            .set_binding("cache_limit", Value::String(self.cache_limit.clone()))?;
        self.ui
            .set_binding("output_path", Value::String(self.output_path.clone()))?;
        self.ui.set_binding(
            "still_overwrite_pending",
            Value::Bool(self.still_overwrite_pending),
        )?;
        self.ui
            .set_binding("still_status", Value::String(self.still_status.clone()))?;
        self.ui
            .set_binding("project_name", Value::String(self.project_name.clone()))?;
        self.ui
            .set_binding("save_status", Value::String(self.save_status.clone()))?;

        // Setup page bindings
        self.ui
            .set_binding("setup_name", Value::String(self.setup_name.clone()))?;
        self.ui
            .set_binding("setup_width", Value::String(self.setup_width.clone()))?;
        self.ui
            .set_binding("setup_height", Value::String(self.setup_height.clone()))?;
        self.ui
            .set_binding("setup_fps", Value::String(self.setup_fps.clone()))?;
        self.ui.set_binding(
            "setup_duration_seconds",
            Value::String(self.setup_duration_seconds.clone()),
        )?;
        self.ui
            .set_binding("setup_error", Value::String(self.setup_error.clone()))?;
        self.ui.set_binding(
            "setup_hd_selected",
            Value::Bool(self.setup_width == "1280" && self.setup_height == "720"),
        )?;
        self.ui.set_binding(
            "setup_full_hd_selected",
            Value::Bool(self.setup_width == "1920" && self.setup_height == "1080"),
        )?;
        self.ui.set_binding(
            "setup_uhd_selected",
            Value::Bool(self.setup_width == "3840" && self.setup_height == "2160"),
        )?;
        self.ui.set_binding(
            "setup_qhd_selected",
            Value::Bool(self.setup_width == "2560" && self.setup_height == "1440"),
        )?;
        self.ui.set_binding(
            "setup_vertical_selected",
            Value::Bool(self.setup_width == "1080" && self.setup_height == "1920"),
        )?;
        self.ui.set_binding(
            "setup_square_selected",
            Value::Bool(self.setup_width == "1080" && self.setup_height == "1080"),
        )?;
        self.ui.set_binding(
            "setup_portrait_4_5_selected",
            Value::Bool(self.setup_width == "1080" && self.setup_height == "1350"),
        )?;
        for (name, rate) in [
            ("setup_fps_24_selected", "24"),
            ("setup_fps_25_selected", "25"),
            ("setup_fps_30_selected", "30"),
            ("setup_fps_50_selected", "50"),
            ("setup_fps_60_selected", "60"),
        ] {
            self.ui
                .set_binding(name, Value::Bool(self.setup_fps.trim() == rate))?;
        }

        // Workspace layouts and tab views
        self.ui.set_binding(
            "workspace_left_ratio",
            Value::Float(self.workspace_left_ratios[self.workspace.index()]),
        )?;
        self.ui.set_binding(
            "workspace_center_ratio",
            Value::Float(self.workspace_center_ratios[self.workspace.index()]),
        )?;
        self.ui.set_binding(
            "workspace_vertical_ratio",
            Value::Float(self.workspace_vertical_ratios[self.workspace.index()]),
        )?;
        for (name, workspace) in [
            ("is_default_workspace", Workspace::Default),
            ("is_composition_workspace", Workspace::Composition),
            ("is_scene_workspace", Workspace::Scene),
            ("is_timeline_workspace", Workspace::Timeline),
            ("is_animation_workspace", Workspace::Animation),
            ("is_graph_workspace", Workspace::Graph),
            ("is_effects_workspace", Workspace::Effects),
            ("is_audio_workspace", Workspace::Audio),
            ("is_time_remap_workspace", Workspace::TimeRemap),
            ("is_mask_workspace", Workspace::Mask),
            ("is_assets_workspace", Workspace::Assets),
            ("is_text_workspace", Workspace::Text),
            ("is_layer_style_workspace", Workspace::LayerStyle),
            ("is_color_workspace", Workspace::Color),
            ("is_camera_light_workspace", Workspace::CameraLight),
        ] {
            self.ui
                .set_binding(name, Value::Bool(self.workspace == workspace))?;
        }
        for (name, view) in [
            ("timeline_view_all", TimelineView::All),
            ("timeline_view_animation", TimelineView::Animation),
            ("timeline_view_current_keyframes", TimelineView::CurrentKeyframes),
            ("timeline_view_audio", TimelineView::Audio),
            ("timeline_view_media", TimelineView::Media),
            ("timeline_view_text", TimelineView::Text),
            ("timeline_view_shapes", TimelineView::Shapes),
            ("timeline_view_3d", TimelineView::ThreeD),
            ("timeline_view_cameras", TimelineView::Cameras),
            ("timeline_view_lights", TimelineView::Lights),
            ("timeline_view_markers", TimelineView::Markers),
            ("timeline_view_graph", TimelineView::Graph),
            ("timeline_view_flowchart", TimelineView::Flowchart),
        ] {
            self.ui
                .set_binding(name, Value::Bool(self.timeline_view == view))?;
        }
        for (name, section) in [
            ("properties_section_transform", PropertiesSection::Transform),
            ("properties_section_content", PropertiesSection::Content),
            ("properties_section_composite", PropertiesSection::Composite),
            ("properties_section_advanced", PropertiesSection::Advanced),
        ] {
            self.ui
                .set_binding(name, Value::Bool(self.properties_section == section))?;
        }
        self.ui
            .set_binding("can_undo", Value::Bool(self.history.can_undo()))?;
        self.ui
            .set_binding("can_redo", Value::Bool(self.history.can_redo()))?;
        self.ui.set_binding(
            "timeline_snap_enabled",
            Value::Bool(self.timeline_snap_enabled),
        )?;
        for (name, tool) in [
            ("selected_tool_select", "Selection"),
            ("selected_tool_hand", "Hand"),
            ("selected_tool_zoom", "Zoom"),
            ("selected_tool_text", "Text"),
            ("selected_tool_shape", "Shape"),
            ("selected_tool_pen", "Pen"),
        ] {
            self.ui
                .set_binding(name, Value::Bool(self.selected_tool == tool))?;
        }
        self.ui.set_binding(
            "selected_tool",
            Value::String(self.selected_tool.to_owned()),
        )?;
        let selected_tool_label = match self.selected_tool {
            "Selection" => "選択",
            "Hand" => "手のひら",
            "Zoom" => "ズーム",
            "Text" => "文字",
            "Shape" => "図形",
            "Pen" => "ペン",
            _ => "選択",
        };
        self.ui.set_binding(
            "selected_tool_label",
            Value::String(selected_tool_label.to_owned()),
        )?;

        Ok(())
    }
}
