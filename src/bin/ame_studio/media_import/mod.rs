use super::*;

mod video_and_sequence;


impl AfterEffectsStudio {
    fn reject_media_import_while_busy(&mut self) -> bool {
        let Some(job) = self.video_import.as_ref() else {
            return false;
        };
        let canceling = job.cancel.load(std::sync::atomic::Ordering::Relaxed);
        self.asset_status = if canceling {
            "前の素材の中止処理中です。完了してから再度追加してください。".to_owned()
        } else {
            "別の素材を読み込み中です。完了してから再度追加してください。".to_owned()
        };
        self.sync_bindings_logged();
        true
    }

    pub(crate) fn import_model_layer(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("3Dモデル", &["obj", "gltf", "glb"])
            .pick_file()
        else {
            return;
        };
        self.import_model_file(path);
    }

    pub(crate) fn import_model_file(&mut self, path: std::path::PathBuf) {
        let Some(name) = path
            .file_stem()
            .map(|name| name.to_string_lossy().into_owned())
        else {
            self.asset_status = "3Dモデルのファイル名を読み取れませんでした。".to_owned();
            return;
        };
        let source_path = path.display().to_string();
        self.register_project_asset(
            name.clone(),
            kagari_vfx::core::timeline::ProjectItemType::Model3D {
                path: source_path.clone(),
            },
        );
        let composition = self.project_model.active_composition_mut();
        let id = format!(
            "ameui_model_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or_default()
        );
        let mut layer = kagari_vfx::core::timeline::Layer::new(
            id.clone(),
            name,
            kagari_vfx::core::timeline::LayerType::Model3D { path: source_path },
            composition.duration_frames,
        );
        layer.is_3d = true;
        layer.transform.position = kagari_vfx::core::timeline::Animatable::new_constant([
            composition.width as f32 * 0.5,
            composition.height as f32 * 0.5,
        ]);
        composition.add_layer(layer);
        self.selected_layer_id = Some(id);
        self.sync_selected_layer_controls();
        self.asset_status = format!("3Dモデルを追加しました: {}", path.display());
        self.mark_project_changed();
    }

    pub(crate) fn import_image_layer(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter(
                "画像",
                &["png", "jpg", "jpeg", "webp", "bmp", "tif", "tiff", "svg", "psd"],
            )
            .pick_file()
        else {
            return;
        };
        self.import_image_file(path);
    }

    pub(crate) fn import_image_file(&mut self, path: std::path::PathBuf) {
        if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("psd"))
        {
            self.import_psd_file(&path);
            return;
        }
        let Some(file_name) = path
            .file_stem()
            .map(|name| name.to_string_lossy().into_owned())
        else {
            self.asset_status = "ファイル名を読み取れませんでした。".to_owned();
            return;
        };
        let source_path = path.display().to_string();
        let Some((image_width, image_height)) =
            kagari_vfx::core::image_cache::image_dimensions(&source_path)
        else {
            self.asset_status = "選択した画像を読み込めませんでした。SVGは外部画像の参照を含まない形式で読み込んでください。".to_owned();
            return;
        };
        self.register_project_asset(
            file_name.clone(),
            kagari_vfx::core::timeline::ProjectItemType::Image {
                path: source_path.clone(),
                width: image_width,
                height: image_height,
            },
        );
        let still_secs = self.still_duration_secs.clamp(0.1, 600.0);
        let composition = self.project_model.active_composition_mut();
        let still_frames = (still_secs * f64::from(composition.fps.max(1)))
            .round()
            .max(1.0) as u32;
        let layer_frames = still_frames.min(composition.duration_frames.max(1));
        let id = format!(
            "ameui_image_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or_default()
        );
        let mut layer = kagari_vfx::core::timeline::Layer::new(
            id.clone(),
            file_name,
            kagari_vfx::core::timeline::LayerType::Image { path: source_path },
            layer_frames,
        );
        layer.transform.position = kagari_vfx::core::timeline::Animatable::new_constant([
            composition.width as f32 * 0.5,
            composition.height as f32 * 0.5,
        ]);
        composition.add_layer(layer);
        self.selected_layer_id = Some(id);
        self.sync_selected_layer_controls();
        self.asset_status = format!(
            "追加しました: {}",
            path.file_name().unwrap_or_default().to_string_lossy()
        );
        self.mark_project_changed();
    }

    pub(crate) fn import_psd_file(&mut self, path: &std::path::Path) {
        let Ok(bytes) = std::fs::read(path) else {
            self.asset_status = "PSD ファイルを読み込めませんでした。".to_owned();
            return;
        };
        let doc = match kagari_vfx::core::psd_importer::parse_psd(&bytes) {
            Ok(d) => d,
            Err(err) => {
                self.asset_status = format!("PSD 解析エラー: {err}");
                return;
            }
        };
        if doc.layers.is_empty() {
            self.asset_status = "PSD に読み込めるレイヤーがありませんでした。".to_owned();
            return;
        }

        let cache_dir = std::env::temp_dir().join("kagari_psd_layers");
        let _ = std::fs::create_dir_all(&cache_dir);
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();

        let still_secs = self.still_duration_secs.clamp(0.1, 600.0);
        let mut imported_count = 0usize;

        for (idx, psd_layer) in doc.layers.into_iter().enumerate() {
            if psd_layer.width == 0 || psd_layer.height == 0 {
                continue;
            }
            let Some(img_buf) =
                image::RgbaImage::from_raw(psd_layer.width, psd_layer.height, psd_layer.rgba8)
            else {
                continue;
            };
            let out_png = cache_dir.join(format!("psd_{stamp}_{idx}.png"));
            if img_buf.save(&out_png).is_err() {
                continue;
            }
            let source_path = out_png.display().to_string();
            self.register_project_asset(
                psd_layer.name.clone(),
                kagari_vfx::core::timeline::ProjectItemType::Image {
                    path: source_path.clone(),
                    width: psd_layer.width,
                    height: psd_layer.height,
                },
            );

            let composition = self.project_model.active_composition_mut();
            let still_frames = (still_secs * f64::from(composition.fps.max(1)))
                .round()
                .max(1.0) as u32;
            let layer_frames = still_frames.min(composition.duration_frames.max(1));
            let id = format!("ameui_psd_{stamp}_{idx}");
            let mut layer = kagari_vfx::core::timeline::Layer::new(
                id.clone(),
                psd_layer.name,
                kagari_vfx::core::timeline::LayerType::Image { path: source_path },
                layer_frames,
            );
            layer.visible = psd_layer.visible;
            layer.transform.opacity = kagari_vfx::core::timeline::Animatable::new_constant(
                (f32::from(psd_layer.opacity) / 255.0) * 100.0,
            );
            layer.transform.position = kagari_vfx::core::timeline::Animatable::new_constant([
                psd_layer.left as f32 + psd_layer.width as f32 * 0.5,
                psd_layer.top as f32 + psd_layer.height as f32 * 0.5,
            ]);
            composition.add_layer(layer);
            self.selected_layer_id = Some(id);
            imported_count += 1;
        }

        self.sync_selected_layer_controls();
        if imported_count > 0 {
            self.mark_project_changed();
        }
        self.asset_status = format!("PSD から {imported_count} レイヤーを読み込みました。");
    }

    pub(crate) fn import_audio_layer(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter(
                "音声",
                &[
                    "wav", "mp3", "m4a", "aac", "flac", "ogg", "aiff", "aif", "caf",
                ],
            )
            .pick_file()
        else {
            return;
        };
        self.import_audio_file(path);
    }

    pub(crate) fn import_audio_file(&mut self, path: std::path::PathBuf) {
        let Some(file_name) = path
            .file_stem()
            .map(|name| name.to_string_lossy().into_owned())
        else {
            self.asset_status = "ファイル名を読み取れませんでした。".to_owned();
            return;
        };
        let source_path = path.display().to_string();
        self.register_project_asset(
            file_name.clone(),
            kagari_vfx::core::timeline::ProjectItemType::Audio {
                path: source_path.clone(),
                duration_sec: 0.0,
            },
        );
        let composition = self.project_model.active_composition_mut();
        let id = format!(
            "ameui_audio_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or_default()
        );
        let mut layer = kagari_vfx::core::timeline::Layer::new(
            id.clone(),
            file_name,
            kagari_vfx::core::timeline::LayerType::Audio {
                path: source_path,
                volume: kagari_vfx::core::timeline::Animatable::new_constant(0.0),
            },
            composition.duration_frames,
        );
        layer.transform.position = kagari_vfx::core::timeline::Animatable::new_constant([
            composition.width as f32 * 0.5,
            composition.height as f32 * 0.5,
        ]);
        composition.add_layer(layer);
        self.selected_layer_id = Some(id);
        self.sync_selected_layer_controls();
        self.asset_status = format!(
            "追加しました: {}",
            path.file_name().unwrap_or_default().to_string_lossy()
        );
        self.mark_project_changed();
    }

    pub(crate) fn relink_scene_asset(&mut self, layer_id: &str) {
        use kagari_vfx::core::timeline::LayerType;

        if let Some(layer) = self
            .project_model
            .active_composition()
            .layers
            .iter()
            .find(|layer| layer.id == layer_id && layer.locked)
        {
            self.asset_status =
                format!("「{}」はロック中のため素材を再指定できません。", layer.name);
            return;
        }

        let Some((kind, old_path)) = self
            .project_model
            .active_composition()
            .layers
            .iter()
            .find(|layer| layer.id == layer_id)
            .map(|layer| match &layer.layer_type {
                LayerType::Image { path } => ("image", path.clone()),
                LayerType::Video { source, .. } => ("video", source.clone()),
                LayerType::Audio { path, .. } => ("audio", path.clone()),
                LayerType::Model3D { path } => ("model", path.clone()),
                _ => ("", String::new()),
            })
            .filter(|(kind, _)| !kind.is_empty())
        else {
            return;
        };

        let path = match kind {
            "image" => rfd::FileDialog::new()
                .add_filter(
                    "画像",
                    &["png", "jpg", "jpeg", "webp", "bmp", "tif", "tiff", "svg"],
                )
                .pick_file(),
            "audio" => rfd::FileDialog::new()
                .add_filter(
                    "音声",
                    &[
                        "wav", "mp3", "m4a", "aac", "flac", "ogg", "aiff", "aif", "caf",
                    ],
                )
                .pick_file(),
            "video" => rfd::FileDialog::new()
                .add_filter(
                    "動画",
                    &[
                        "mp4", "mov", "mkv", "avi", "webm", "m4v", "mpeg", "mpg", "ts", "m2ts",
                        "ivf",
                    ],
                )
                .pick_file(),
            "model" => rfd::FileDialog::new()
                .add_filter("3Dモデル", &["obj", "gltf", "glb"])
                .pick_file(),
            _ => None,
        };
        let Some(path) = path else {
            return;
        };
        if kind == "video" {
            let project_asset_id = self
                .project_model
                .assets
                .iter()
                .find(|item| item.media_path() == Some(old_path.as_str()))
                .map(|item| item.id.clone());
            if let Some(asset_id) = project_asset_id {
                self.start_video_import(path, None, Some(asset_id), false);
            } else {
                self.start_video_import(path, Some(layer_id.to_owned()), None, false);
            }
            return;
        }

        let path = path.display().to_string();
        let dimensions = if kind == "image" {
            let Some(dimensions) = kagari_vfx::core::image_cache::image_dimensions(&path) else {
                self.asset_status =
                    "選択した画像を読み込めませんでした。元の素材は変更していません。".to_owned();
                return;
            };
            dimensions
        } else {
            (0, 0)
        };
        let mut project_item_updated = false;
        for item in &mut self.project_model.assets {
            if item.media_path() != Some(old_path.as_str()) {
                continue;
            }
            match &mut item.item_type {
                kagari_vfx::core::timeline::ProjectItemType::Image {
                    path: source,
                    width,
                    height,
                } => {
                    *source = path.clone();
                    *width = dimensions.0;
                    *height = dimensions.1;
                    project_item_updated = true;
                }
                kagari_vfx::core::timeline::ProjectItemType::Audio { path: source, .. }
                | kagari_vfx::core::timeline::ProjectItemType::Model3D { path: source } => {
                    *source = path.clone();
                    project_item_updated = true;
                }
                _ => {}
            }
        }
        if project_item_updated {
            replace_project_media_path(&mut self.project_model.compositions, &old_path, &path);
        }
        let changed = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == layer_id)
            .map(|layer| match &mut layer.layer_type {
                LayerType::Image { path: source }
                | LayerType::Audio { path: source, .. }
                | LayerType::Model3D { path: source } => {
                    *source = path.clone();
                    true
                }
                _ => false,
            })
            .unwrap_or(false);
        if changed || project_item_updated {
            self.selected_layer_id = Some(layer_id.to_owned());
            self.asset_status = format!("素材を再指定しました: {}", path);
            self.sync_selected_layer_controls();
            self.mark_project_changed();
        }
    }

}
