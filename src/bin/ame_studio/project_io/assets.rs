use super::*;

impl AfterEffectsStudio {
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
        let disk_module_refs: Vec<(&str, &str)> =
            disk_modules.iter().map(|(p, c)| (*p, c.as_str())).collect();

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
