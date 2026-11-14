use super::*;

impl AfterEffectsStudio {
    pub(crate) fn refresh_openfx_plugins(&mut self) {
        let search_paths = kagari_vfx::core::openfx_bridge::ofx_search_paths();
        self.openfx_search_paths = search_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join("\n");
        self.openfx_plugins = kagari_vfx::core::openfx_bridge::discover_all_ofx_plugins()
            .into_iter()
            .map(|plugin| DetectedOfxPlugin {
                name: plugin.name,
                path: plugin.bundle_root.display().to_string(),
            })
            .collect();
    }

    pub(crate) fn ensure_composition_project_items(&mut self) {
        use kagari_vfx::core::timeline::{ProjectItem, ProjectItemType};
        if self.project_file_path.is_none() && self.project_model.has_pristine_start_composition() {
            return;
        }
        let names = self
            .project_model
            .compositions
            .iter()
            .map(|composition| composition.name.clone())
            .collect::<Vec<_>>();
        let mut present = vec![false; names.len()];
        let mut changed = false;
        self.project_model.assets.retain_mut(|item| {
            let ProjectItemType::Composition { comp_idx } = &mut item.item_type else {
                return true;
            };
            if *comp_idx >= names.len() || present[*comp_idx] {
                changed = true;
                return false;
            }
            present[*comp_idx] = true;
            if item.name != names[*comp_idx] {
                item.name = names[*comp_idx].clone();
                changed = true;
            }
            true
        });
        for (comp_idx, name) in names.into_iter().enumerate() {
            if present[comp_idx] {
                continue;
            }
            let id = self.project_model.next_project_item_id("item_comp");
            self.project_model.assets.push(ProjectItem::new(
                id,
                name,
                ProjectItemType::Composition { comp_idx },
            ));
            changed = true;
        }
        if changed {
            self.mark_project_changed();
        }
    }

    pub(crate) fn set_document(&mut self, title: &str, body: &str) {
        self.screen = Screen::Documents;
        self.docs_detail_page = true;
        self.docs_title = title.to_owned();
        self.docs_body = body.to_owned();
    }

    pub(crate) fn settings_draft(&self) -> SettingsDraft {
        SettingsDraft {
            restore_previous_state: self.restore_previous_state,
            confirm_close_project: self.confirm_close_project,
            confirm_exit: self.confirm_exit,
            adaptive_preview: self.adaptive_preview,
            audio_preview: self.audio_preview,
            show_home_on_startup: self.show_home_on_startup,
            autosave_secs: self.autosave_secs,
            gpu_enabled: self.gpu_enabled,
            memory_limit_gb: self.memory_limit_gb,
            undo_steps: self.undo_steps,
            recovery_slots: self.recovery_slots,
            cache_path: self.cache_path.clone(),
            cache_limit: self.cache_limit.clone(),
            accent_color: self.accent_color.clone(),
            auto_project_names: self.auto_project_names,
            renderer: self.renderer.clone(),
            background_render: self.background_render,
            preview_quality: self.preview_quality.clone(),
            high_quality_preview: self.high_quality_preview,
            preview_resolution: self.preview_resolution.clone(),
            still_duration_secs: self.still_duration_secs,
            image_sequence_detect: self.image_sequence_detect,
            proxy_auto_generate: self.proxy_auto_generate,
            hardware_decode: self.hardware_decode,
            preferred_decoder: self.preferred_decoder.clone(),
            media_color_space: self.media_color_space.clone(),
            grid_visible: self.grid_visible,
            grid_spacing: self.grid_spacing,
            safe_area_visible: self.safe_area_visible,
            color_management: self.color_management.clone(),
            gamma: self.gamma.clone(),
            hdr_enabled: self.hdr_enabled,
            playback_quality: self.playback_quality.clone(),
            playback_start_mode: self.playback_start_mode.clone(),
            pre_roll_secs: self.pre_roll_secs,
            post_roll_secs: self.post_roll_secs,
            playback_volume: self.playback_volume,
            playback_cache: self.playback_cache,
            scrub_audio: self.scrub_audio,
            show_audio_peaks: self.show_audio_peaks,
            project_dir: self.project_dir.clone(),
            preview_cache_dir: self.preview_cache_dir.clone(),
            temp_dir: self.temp_dir.clone(),
            autosave_dir: self.autosave_dir.clone(),
            backup_dir: self.backup_dir.clone(),
            backup_generations: self.backup_generations,
            asset_library_dir: self.asset_library_dir.clone(),
            shortcut_filter: self.shortcut_filter.clone(),
        }
    }

    pub(crate) fn restore_settings_draft(&mut self, draft: SettingsDraft) {
        self.restore_previous_state = draft.restore_previous_state;
        self.confirm_close_project = draft.confirm_close_project;
        self.confirm_exit = draft.confirm_exit;
        self.adaptive_preview = draft.adaptive_preview;
        self.audio_preview = draft.audio_preview;
        self.show_home_on_startup = draft.show_home_on_startup;
        self.autosave_secs = draft.autosave_secs;
        self.gpu_enabled = draft.gpu_enabled;
        self.memory_limit_gb = draft.memory_limit_gb;
        self.undo_steps = draft.undo_steps;
        self.recovery_slots = draft.recovery_slots;
        self.cache_path = draft.cache_path;
        self.cache_limit = draft.cache_limit;
        self.accent_color = draft.accent_color;
        self.auto_project_names = draft.auto_project_names;
        self.renderer = draft.renderer;
        self.background_render = draft.background_render;
        self.preview_quality = draft.preview_quality;
        self.high_quality_preview = draft.high_quality_preview;
        self.preview_resolution = draft.preview_resolution;
        self.still_duration_secs = draft.still_duration_secs;
        self.image_sequence_detect = draft.image_sequence_detect;
        self.proxy_auto_generate = draft.proxy_auto_generate;
        self.hardware_decode = draft.hardware_decode;
        self.preferred_decoder = draft.preferred_decoder;
        self.media_color_space = draft.media_color_space;
        self.grid_visible = draft.grid_visible;
        self.grid_spacing = draft.grid_spacing;
        self.safe_area_visible = draft.safe_area_visible;
        self.color_management = draft.color_management;
        self.gamma = draft.gamma;
        self.hdr_enabled = draft.hdr_enabled;
        self.playback_quality = draft.playback_quality;
        self.playback_start_mode = draft.playback_start_mode;
        self.pre_roll_secs = draft.pre_roll_secs;
        self.post_roll_secs = draft.post_roll_secs;
        self.playback_volume = draft.playback_volume;
        self.playback_cache = draft.playback_cache;
        self.scrub_audio = draft.scrub_audio;
        self.show_audio_peaks = draft.show_audio_peaks;
        self.project_dir = draft.project_dir;
        self.preview_cache_dir = draft.preview_cache_dir;
        self.temp_dir = draft.temp_dir;
        self.autosave_dir = draft.autosave_dir;
        self.backup_dir = draft.backup_dir;
        self.backup_generations = draft.backup_generations;
        self.asset_library_dir = draft.asset_library_dir;
        self.shortcut_filter = draft.shortcut_filter;
        kagari_vfx::core::compute_pipeline::set_gpu_effects_enabled(self.gpu_enabled);
        self.history.set_max_history_entries(self.undo_steps);
        let cache_directory = if self.cache_path.trim().is_empty() {
            None
        } else {
            Some(expand_home_path(self.cache_path.trim()))
        };
        let _ = kagari_vfx::core::frame_cache::disk_cache::set_cache_directory(cache_directory);
        let _ = write_preference(
            "disk_cache_directory",
            serde_json::Value::String(self.cache_path.clone()),
        );
    }

    pub(crate) fn reset_settings_draft(&mut self) {
        self.restore_previous_state = true;
        self.confirm_close_project = true;
        self.confirm_exit = true;
        self.adaptive_preview = true;
        self.audio_preview = true;
        self.show_home_on_startup = true;
        self.autosave_secs = 300;
        self.gpu_enabled = true;
        self.memory_limit_gb = 12.0;
        self.undo_steps = 50;
        self.recovery_slots = 10;
        self.cache_path = kagari_vfx::core::frame_cache::disk_cache::cache_directory()
            .display()
            .to_string();
        self.cache_limit = "100".to_owned();
        self.accent_color = default_accent_color();
        self.auto_project_names = true;
        self.renderer = "gpu".to_owned();
        self.background_render = true;
        self.preview_quality = "auto".to_owned();
        self.high_quality_preview = true;
        self.preview_resolution = "auto".to_owned();
        self.still_duration_secs = 5.0;
        self.image_sequence_detect = true;
        self.proxy_auto_generate = false;
        self.hardware_decode = true;
        self.preferred_decoder = "auto".to_owned();
        self.media_color_space = "project".to_owned();
        self.grid_visible = false;
        self.grid_spacing = 100.0;
        self.safe_area_visible = false;
        self.color_management = "project".to_owned();
        self.gamma = "2.2".to_owned();
        self.hdr_enabled = false;
        self.playback_quality = "auto".to_owned();
        self.playback_start_mode = "current".to_owned();
        self.pre_roll_secs = 1.0;
        self.post_roll_secs = 1.0;
        self.playback_volume = 100.0;
        self.playback_cache = true;
        self.scrub_audio = false;
        self.show_audio_peaks = false;
        self.project_dir = default_project_dir();
        self.preview_cache_dir = default_preview_cache_dir();
        self.temp_dir = default_temp_dir();
        self.autosave_dir = default_autosave_dir();
        self.backup_dir = default_backup_dir();
        self.backup_generations = 10;
        self.asset_library_dir = default_asset_library_dir();
        self.shortcut_filter = "all".to_owned();
    }

    pub(crate) fn commit_settings(&mut self) -> Result<(), String> {
        let disk_cache_gb = self
            .cache_limit
            .trim()
            .parse::<u64>()
            .map_err(|_| "キャッシュ上限は10〜500 GBの整数で入力してください。".to_owned())?;
        if !(10..=500).contains(&disk_cache_gb) {
            return Err("キャッシュ上限は10〜500 GBで指定してください。".to_owned());
        }
        self.cache_limit = disk_cache_gb.to_string();
        self.still_duration_secs = self.still_duration_secs.clamp(0.1, 600.0);
        self.grid_spacing = self.grid_spacing.clamp(8.0, 512.0);
        self.pre_roll_secs = self.pre_roll_secs.clamp(0.0, 10.0);
        self.post_roll_secs = self.post_roll_secs.clamp(0.0, 10.0);
        self.playback_volume = self.playback_volume.clamp(0.0, 100.0);
        self.backup_generations = self.backup_generations.clamp(1, 100);
        self.autosave_secs = normalize_autosave_secs(self.autosave_secs);
        if parse_hex_color(&self.accent_color).is_none() {
            self.accent_color = default_accent_color();
        }
        self.gpu_enabled = self.renderer != "cpu";
        let cache_directory = if self.cache_path.trim().is_empty() {
            None
        } else {
            Some(expand_home_path(self.cache_path.trim()))
        };
        kagari_vfx::core::frame_cache::disk_cache::validate_cache_directory(
            cache_directory.as_deref(),
        )
        .map_err(|error| format!("保存先を設定できませんでした: {error}"))?;
        let previous_cache_path = self
            .settings_snapshot
            .as_ref()
            .map(|draft| draft.cache_path.as_str())
            .unwrap_or(self.cache_path.as_str());
        let previous_cache_directory = if previous_cache_path.trim().is_empty() {
            None
        } else {
            Some(expand_home_path(previous_cache_path.trim()))
        };
        kagari_vfx::core::frame_cache::disk_cache::set_cache_directory(cache_directory.clone())
            .map_err(|error| format!("保存先を設定できませんでした: {error}"))?;
        if let Err(error) = write_preferences([
            (
                "restore_previous_state",
                serde_json::Value::Bool(self.restore_previous_state),
            ),
            (
                "confirm_close_project",
                serde_json::Value::Bool(self.confirm_close_project),
            ),
            ("confirm_exit", serde_json::Value::Bool(self.confirm_exit)),
            (
                "adaptive_preview",
                serde_json::Value::Bool(self.adaptive_preview),
            ),
            ("audio_preview", serde_json::Value::Bool(self.audio_preview)),
            (
                "startup_home",
                serde_json::Value::Bool(self.show_home_on_startup),
            ),
            ("autosave_secs", serde_json::json!(self.autosave_secs)),
            (
                "gpu_compute_enabled",
                serde_json::Value::Bool(self.gpu_enabled),
            ),
            (
                "cache_mb",
                serde_json::json!((self.memory_limit_gb.round() as u64) * 1024),
            ),
            (
                "disk_cache_directory",
                serde_json::Value::String(self.cache_path.clone()),
            ),
            ("undo_steps", serde_json::json!(self.undo_steps)),
            ("recovery_slots", serde_json::json!(self.recovery_slots)),
            ("disk_cache_gb", serde_json::json!(disk_cache_gb)),
            (
                "accent_color",
                serde_json::Value::String(self.accent_color.clone()),
            ),
            (
                "auto_project_names",
                serde_json::Value::Bool(self.auto_project_names),
            ),
            ("renderer", serde_json::Value::String(self.renderer.clone())),
            (
                "background_render",
                serde_json::Value::Bool(self.background_render),
            ),
            (
                "preview_quality",
                serde_json::Value::String(self.preview_quality.clone()),
            ),
            (
                "high_quality_preview",
                serde_json::Value::Bool(self.high_quality_preview),
            ),
            (
                "preview_resolution",
                serde_json::Value::String(self.preview_resolution.clone()),
            ),
            (
                "still_duration_secs",
                serde_json::json!(self.still_duration_secs),
            ),
            (
                "image_sequence_detect",
                serde_json::Value::Bool(self.image_sequence_detect),
            ),
            (
                "proxy_auto_generate",
                serde_json::Value::Bool(self.proxy_auto_generate),
            ),
            (
                "hardware_decode",
                serde_json::Value::Bool(self.hardware_decode),
            ),
            (
                "preferred_decoder",
                serde_json::Value::String(self.preferred_decoder.clone()),
            ),
            (
                "media_color_space",
                serde_json::Value::String(self.media_color_space.clone()),
            ),
            ("grid_visible", serde_json::Value::Bool(self.grid_visible)),
            ("grid_spacing", serde_json::json!(self.grid_spacing)),
            (
                "safe_area_visible",
                serde_json::Value::Bool(self.safe_area_visible),
            ),
            (
                "color_management",
                serde_json::Value::String(self.color_management.clone()),
            ),
            ("gamma", serde_json::Value::String(self.gamma.clone())),
            ("hdr_enabled", serde_json::Value::Bool(self.hdr_enabled)),
            (
                "playback_quality",
                serde_json::Value::String(self.playback_quality.clone()),
            ),
            (
                "playback_start_mode",
                serde_json::Value::String(self.playback_start_mode.clone()),
            ),
            ("pre_roll_secs", serde_json::json!(self.pre_roll_secs)),
            ("post_roll_secs", serde_json::json!(self.post_roll_secs)),
            ("playback_volume", serde_json::json!(self.playback_volume)),
            (
                "playback_cache",
                serde_json::Value::Bool(self.playback_cache),
            ),
            ("scrub_audio", serde_json::Value::Bool(self.scrub_audio)),
            (
                "show_audio_peaks",
                serde_json::Value::Bool(self.show_audio_peaks),
            ),
            (
                "project_dir",
                serde_json::Value::String(self.project_dir.clone()),
            ),
            (
                "preview_cache_dir",
                serde_json::Value::String(self.preview_cache_dir.clone()),
            ),
            ("temp_dir", serde_json::Value::String(self.temp_dir.clone())),
            (
                "autosave_dir",
                serde_json::Value::String(self.autosave_dir.clone()),
            ),
            (
                "backup_dir",
                serde_json::Value::String(self.backup_dir.clone()),
            ),
            (
                "backup_generations",
                serde_json::json!(self.backup_generations),
            ),
            (
                "asset_library_dir",
                serde_json::Value::String(self.asset_library_dir.clone()),
            ),
            (
                "shortcut_filter",
                serde_json::Value::String(self.shortcut_filter.clone()),
            ),
        ]) {
            let _ = kagari_vfx::core::frame_cache::disk_cache::set_cache_directory(
                previous_cache_directory,
            );
            return Err(error);
        }
        self.cache_path_status = if self.cache_path.trim().is_empty() {
            "既定の保存先を使用します".to_owned()
        } else {
            "保存先を適用しました".to_owned()
        };
        kagari_vfx::core::compute_pipeline::set_gpu_effects_enabled(self.gpu_enabled);
        self.history.set_max_history_entries(self.undo_steps);
        kagari_vfx::core::frame_cache::disk_cache::set_max_disk_bytes(
            disk_cache_gb * 1024 * 1024 * 1024,
        );
        self.cache_usage_bytes = kagari_vfx::core::frame_cache::disk_cache::current_disk_usage();
        Ok(())
    }
}
