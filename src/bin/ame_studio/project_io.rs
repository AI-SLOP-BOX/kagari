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
        self.apply_cache_path()?;
        for (key, value) in [
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
            ("undo_steps", serde_json::json!(self.undo_steps)),
            ("recovery_slots", serde_json::json!(self.recovery_slots)),
            ("disk_cache_gb", serde_json::json!(disk_cache_gb)),
        ] {
            write_preference(key, value)?;
        }
        kagari_vfx::core::frame_cache::disk_cache::set_max_disk_bytes(
            disk_cache_gb * 1024 * 1024 * 1024,
        );
        self.cache_usage_bytes = kagari_vfx::core::frame_cache::disk_cache::current_disk_usage();
        Ok(())
    }


    pub(crate) fn apply_cache_path(&mut self) -> Result<(), String> {
        let value = self.cache_path.trim();
        let directory = if value.is_empty() {
            None
        } else {
            Some(expand_home_path(value))
        };
        kagari_vfx::core::frame_cache::disk_cache::set_cache_directory(directory)
            .map_err(|error| format!("保存先を設定できませんでした: {error}"))?;
        write_preference(
            "disk_cache_directory",
            serde_json::Value::String(self.cache_path.clone()),
        )?;
        self.cache_path_status = if value.is_empty() {
            "既定の保存先を使用します".to_owned()
        } else {
            "保存先を適用しました".to_owned()
        };
        Ok(())
    }

    pub(crate) fn activate_project_file(&mut self, path: &std::path::Path) {
        self.cancel_video_import();
        let name = path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("Project");
        match self.load_project_file(path, name) {
            Ok(()) => {
                self.screen = Screen::Editor;
                self.render_queue = false;
            }
            Err(error) => self.set_document("プロジェクトを開けませんでした", &error),
        }
    }

    pub(crate) fn request_project_open(&mut self, path: &std::path::Path) {
        if self.confirm_close_project && self.save_status == "未保存の変更があります" {
            self.pending_project_path = Some(path.to_path_buf());
            self.project_switch_confirm_open = true;
        } else {
            self.activate_project_file(path);
        }
    }

    pub(crate) fn begin_new_project_setup(&mut self) {
        self.cancel_video_import();
        self.project_generation = self.project_generation.wrapping_add(1);
        self.setup_return_screen = self.screen;
        self.setup_name = "無題のプロジェクト".to_owned();
        self.setup_width = "1920".to_owned();
        self.setup_height = "1080".to_owned();
        self.setup_fps = "30".to_owned();
        self.setup_duration_seconds = "10".to_owned();
        self.setup_error.clear();
        self.screen = Screen::ProjectSetup;
        self.render_queue = false;
    }

    pub(crate) fn request_new_project(&mut self) {
        if self.confirm_close_project && self.save_status == "未保存の変更があります" {
            self.pending_new_project = true;
            self.project_switch_confirm_open = true;
        } else {
            self.begin_new_project_setup();
        }
    }

    pub(crate) fn reset_project_asset_browser(&mut self) {
        self.asset_folder_id = None;
        self.new_asset_folder_name.clear();
        self.project_asset_search.clear();
        self.selected_project_asset_id = None;
    }

    pub(crate) fn complete_pending_project_switch(&mut self) {
        self.project_switch_confirm_open = false;
        if self.pending_new_project {
            self.pending_new_project = false;
            self.pending_project_path = None;
            self.begin_new_project_setup();
        } else if let Some(path) = self.pending_project_path.take() {
            self.activate_project_file(&path);
        }
    }

    pub(crate) fn record_recent_project(&mut self, path: &std::path::Path) {
        let path = path.to_string_lossy().into_owned();
        let mut recent = read_recent_project_paths();
        recent.retain(|entry| entry != &path);
        recent.insert(0, path);
        recent.truncate(32);
        if write_preference("recent_projects", serde_json::json!(recent)).is_ok() {
            self.recent_projects = load_recent_items();
        }
    }

    pub(crate) fn save_current_project(&mut self) {
        let suggested_stem = self
            .project_name
            .trim()
            .chars()
            .map(|character| match character {
                '/' | '\\' | ':' => '_',
                character if character.is_control() => '_',
                character => character,
            })
            .collect::<String>();
        let suggested_stem = if suggested_stem.is_empty() {
            "無題のプロジェクト"
        } else {
            &suggested_stem
        };
        let path = self.project_file_path.clone().or_else(|| {
            let file_name = format!("{suggested_stem}.json");
            rfd::FileDialog::new()
                .add_filter("Kagari Project", &["json"])
                .set_file_name(&file_name)
                .save_file()
        });
        let Some(path) = path else {
            return;
        };
        match kagari_vfx::core::project_migration::save_project_atomic(&self.project_model, &path) {
            Ok(()) => {
                self.project_name = path
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .unwrap_or("Project")
                    .to_owned();
                self.project_file_path = Some(path.clone());
                self.record_recent_project(&path);
                self.save_status = "保存しました".to_owned();
                self.last_autosave_at = std::time::Instant::now();
            }
            Err(error) => self.save_status = format!("保存できませんでした: {error}"),
        }
    }

    pub(crate) fn maybe_autosave(&mut self) {
        if self.autosave_secs == 0
            || self.project_file_path.is_none()
            || self.save_status != "未保存の変更があります"
            || self.last_autosave_at.elapsed().as_secs() < self.autosave_secs
        {
            return;
        }
        let Some(path) = self.project_file_path.clone() else {
            return;
        };
        self.last_autosave_at = std::time::Instant::now();
        match kagari_vfx::core::project_migration::save_project_atomic(&self.project_model, &path) {
            Ok(()) => self.save_status = "自動保存しました".to_owned(),
            Err(error) => self.save_status = format!("自動保存できませんでした: {error}"),
        }
    }

    pub fn load_project_file(
        &mut self,
        path: &std::path::Path,
        default_name: &str,
    ) -> Result<(), String> {
        self.stop_playback();
        let content = kagari_vfx::core::project_migration::read_bounded_text_file(
            path,
            kagari_vfx::core::project_migration::MAX_PROJECT_JSON_BYTES,
        )
        .map_err(|error| format!("プロジェクトを読み込めません: {error}"))?;
        let project =
            kagari_vfx::core::production_document::ProductionDocument::from_json(&content)
                .map(|document| document.project)
                .or_else(|_| kagari_vfx::core::project_migration::load_project_migrated(&content))
                .map_err(|error| format!("プロジェクト形式を読み込めません: {error}"))?;

        self.project_name = default_name.to_owned();
        self.project_file_path = Some(path.to_path_buf());
        self.save_status.clear();
        self.last_autosave_at = std::time::Instant::now();
        self.output_path = format!("~/Movies/{default_name}.png");
        self.project_model = project;
        self.history = kagari_vfx::core::history::ProjectHistory::new(
            self.project_model.clone(),
        );
        self.history.set_max_history_entries(self.undo_steps);
        self.history_interaction_snapshot = None;
        self.reset_project_asset_browser();
        self.collapsed_scene_layers.clear();
        self.align_selected_layer_ids.clear();
        self.project_generation = self.project_generation.wrapping_add(1);
        self.preview_revision = self.preview_revision.wrapping_add(1);
        self.reset_active_composition_view_state();
        self.selected_layer_id = self
            .project_model
            .active_composition()
            .layers
            .first()
            .map(|layer| layer.id.clone());
        if let Some(composition) = self
            .project_model
            .compositions
            .get(self.project_model.active_composition_idx)
            .or_else(|| self.project_model.compositions.first())
        {
            self.project_fps = composition.fps.clamp(1, 240);
            self.current_time = format_timecode(self.current_frame, self.project_fps);
            self.position_x = f64::from(composition.width) * 0.5;
            self.position_y = f64::from(composition.height) * 0.5;
        }
        self.sync_selected_layer_controls();
        self.record_recent_project(path);
        Ok(())
    }

    pub(crate) fn register_project_asset(
        &mut self,
        name: String,
        item_type: kagari_vfx::core::timeline::ProjectItemType,
    ) -> String {
        self.register_project_asset_in_folder(name, item_type, self.asset_folder_id.clone())
    }

    pub(crate) fn register_project_asset_in_folder(
        &mut self,
        name: String,
        item_type: kagari_vfx::core::timeline::ProjectItemType,
        parent_folder: Option<String>,
    ) -> String {
        let path = match &item_type {
            kagari_vfx::core::timeline::ProjectItemType::Image { path, .. }
            | kagari_vfx::core::timeline::ProjectItemType::Model3D { path }
            | kagari_vfx::core::timeline::ProjectItemType::Video { path, .. }
            | kagari_vfx::core::timeline::ProjectItemType::Audio { path, .. } => {
                Some(path.as_str())
            }
            _ => None,
        };
        if let Some(existing) = path.and_then(|path| {
            self.project_model
                .assets
                .iter()
                .find(|item| item.media_path() == Some(path))
        }) {
            return existing.id.clone();
        }
        let id = self.project_model.next_project_item_id("footage");
        let mut project_item =
            kagari_vfx::core::timeline::ProjectItem::new(id.clone(), name, item_type);
        project_item.parent_folder = parent_folder;
        self.project_model.assets.push(project_item);
        self.mark_project_changed();
        id
    }

    pub(crate) fn relink_project_asset(&mut self, item_id: &str) {
        use kagari_vfx::core::timeline::ProjectItemType;
        let Some(item) = self
            .project_model
            .assets
            .iter()
            .find(|item| item.id == item_id)
            .cloned()
        else {
            return;
        };
        let Some(old_path) = item.media_path().map(str::to_owned) else {
            return;
        };
        let path = match item.item_type {
            ProjectItemType::Image { .. } => rfd::FileDialog::new()
                .add_filter(
                    "画像",
                    &["png", "jpg", "jpeg", "webp", "bmp", "tif", "tiff", "svg"],
                )
                .pick_file(),
            ProjectItemType::Audio { .. } => rfd::FileDialog::new()
                .add_filter(
                    "音声",
                    &[
                        "wav", "mp3", "m4a", "aac", "flac", "ogg", "aiff", "aif", "caf",
                    ],
                )
                .pick_file(),
            ProjectItemType::Model3D { .. } => rfd::FileDialog::new()
                .add_filter("3Dモデル", &["obj", "gltf", "glb"])
                .pick_file(),
            ProjectItemType::Video { .. } => rfd::FileDialog::new()
                .add_filter(
                    "動画",
                    &[
                        "mp4", "mov", "mkv", "avi", "webm", "m4v", "mpeg", "mpg", "ts", "m2ts",
                        "ivf",
                    ],
                )
                .pick_file(),
            _ => None,
        };
        let Some(path) = path else {
            return;
        };
        if matches!(item.item_type, ProjectItemType::Video { .. }) {
            self.start_video_import(path, None, Some(item.id), false);
            return;
        }
        let new_path = path.display().to_string();
        let dimensions = if matches!(item.item_type, ProjectItemType::Image { .. }) {
            let Some(dimensions) = kagari_vfx::core::image_cache::image_dimensions(&new_path)
            else {
                self.asset_status =
                    "選択した画像を読み込めませんでした。元の素材は変更していません。".to_owned();
                return;
            };
            dimensions
        } else {
            (0, 0)
        };
        let mut updated = false;
        if let Some(asset) = self
            .project_model
            .assets
            .iter_mut()
            .find(|asset| asset.id == item_id)
        {
            match &mut asset.item_type {
                ProjectItemType::Image {
                    path,
                    width,
                    height,
                } => {
                    *path = new_path.clone();
                    *width = dimensions.0;
                    *height = dimensions.1;
                    updated = true;
                }
                ProjectItemType::Audio { path, .. } | ProjectItemType::Model3D { path } => {
                    *path = new_path.clone();
                    updated = true;
                }
                _ => {}
            }
        }
        if updated {
            replace_project_media_path(&mut self.project_model.compositions, &old_path, &new_path);
            self.asset_status = format!("プロジェクト素材を再指定しました: {}", new_path);
            self.mark_project_changed();
        }
    }

    /// Hot-reloads all AmeUI sources at runtime on demand (e.g. via F5 or hotkey)
    pub(crate) fn reload_ame_ui(&mut self) -> Result<(), String> {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default());
        let root_path = manifest_dir.join("src/ui/ame/kagari_studio.ame");
        
        let root_source = std::fs::read_to_string(&root_path)
            .unwrap_or_else(|_| ame_sources::STUDIO_ROOT.to_owned());
        
        let mut disk_modules = Vec::new();
        for (module_path, fallback_content) in ame_sources::STUDIO_MODULES {
            let full_path = manifest_dir.join("src/ui/ame").join(module_path);
            let content = std::fs::read_to_string(&full_path)
                .unwrap_or_else(|_| (*fallback_content).to_owned());
            disk_modules.push((*module_path, content));
        }
        let disk_module_refs: Vec<(&str, &str)> = disk_modules
            .iter()
            .map(|(p, c)| (*p, c.as_str()))
            .collect();

        match compile_with_sources(
            "src/ui/ame/kagari_studio.ame",
            &root_source,
            &disk_module_refs,
        ) {
            Ok(new_ui) => {
                self.ui = new_ui;
                self.sync_bindings_logged();
                self.asset_status = "AmeUI ホットリロード成功".to_owned();
                log::info!("[Kagari] AmeUI hot-reloaded successfully from disk sources");
                Ok(())
            }
            Err(err) => {
                let err_msg = format!("AmeUI ホットリロード構文エラー: {err}");
                self.asset_status = err_msg.clone();
                log::error!("[Kagari] AmeUI hot-reload failed: {err}");
                Err(err_msg)
            }
        }
    }
}
