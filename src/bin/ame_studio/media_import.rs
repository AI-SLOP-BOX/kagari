use super::*;

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

    pub(crate) fn import_video_layer(&mut self) {
        if self.reject_media_import_while_busy() {
            return;
        }
        let Some(path) = rfd::FileDialog::new()
            .add_filter(
                "動画",
                &[
                    "mp4", "mov", "mkv", "avi", "webm", "m4v", "mpeg", "mpg", "ts", "m2ts", "ivf",
                ],
            )
            .pick_file()
        else {
            return;
        };
        self.start_video_import(path, None, None, true);
    }

    pub(crate) fn import_image_sequence_layer(&mut self) {
        if self.reject_media_import_while_busy() {
            return;
        }
        let Some(path) = rfd::FileDialog::new()
            .add_filter(
                "画像連番",
                &["png", "jpg", "jpeg", "webp", "bmp", "tif", "tiff"],
            )
            .pick_file()
        else {
            return;
        };
        let Some(source_name) = path
            .file_stem()
            .map(|name| name.to_string_lossy().into_owned())
        else {
            self.asset_status = "連番画像のファイル名を読み取れませんでした。".to_owned();
            return;
        };
        let composition = self.project_model.active_composition();
        let composition_id = composition.id.clone();
        let fps = composition.fps.max(1);
        let import_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default()
            .to_string();
        let layer_id = format!("ameui_sequence_{import_id}");
        let project_file_path = self.project_file_path.clone();
        let project_name = self.project_name.clone();
        let project_generation = self.project_generation;
        let destination = std::env::temp_dir()
            .join("kagari_media")
            .join(format!("{}_{}", source_name, import_id));
        let (sender, receiver) = std::sync::mpsc::channel();
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        let worker = std::thread::Builder::new()
            .name("kagari-image-sequence-import".to_owned())
            .spawn(move || {
                let result = kagari_vfx::core::video_import::import_image_sequence_with_cancel(
                    &path,
                    &destination,
                    fps as f32,
                    &worker_cancel,
                );
                let _ = sender.send(result);
            });
        let worker = match worker {
            Ok(worker) => worker,
            Err(error) => {
                self.asset_status = format!("連番画像の読み込みを開始できませんでした: {error}");
                return;
            }
        };
        self.video_import = Some(VideoImportJob {
            worker: Some(worker),
            receiver,
            cancel,
            cancel_supported: true,
            is_image_sequence: true,
            source_name: source_name.clone(),
            layer_id,
            replace_layer_id: None,
            replace_project_asset_id: None,
            register_project_asset: true,
            composition_id,
            project_file_path,
            project_name,
            fps,
            project_generation,
            asset_parent_folder: self.asset_folder_id.clone(),
        });
        self.asset_status = format!("連番画像を展開中: {source_name}");
        self.sync_bindings_logged();
    }

    pub(crate) fn import_dropped_file(&mut self, source: &str) {
        let path = std::path::PathBuf::from(source);
        if !path.is_file() {
            self.asset_status = format!("ファイルが見つかりません: {}", path.display());
            return;
        }
        let extension = path
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or_default()
            .to_ascii_lowercase();
        match extension.as_str() {
            "png" | "jpg" | "jpeg" | "webp" | "bmp" | "tif" | "tiff" | "svg" => {
                self.import_image_file(path);
            }
            "mp4" | "mov" | "mkv" | "avi" | "webm" | "m4v" | "mpeg" | "mpg" | "ts" | "m2ts"
            | "ivf" => self.start_video_import(path, None, None, true),
            "wav" | "mp3" | "m4a" | "aac" | "flac" | "ogg" | "aiff" | "aif" | "caf" => {
                self.import_audio_file(path);
            }
            "obj" | "gltf" | "glb" => self.import_model_file(path),
            _ => {
                self.asset_status = format!(
                    "対応していない素材形式です: {}",
                    path.extension().unwrap_or_default().to_string_lossy()
                );
            }
        }
    }

    pub(crate) fn start_video_import(
        &mut self,
        path: std::path::PathBuf,
        replace_layer_id: Option<String>,
        replace_project_asset_id: Option<String>,
        register_project_asset: bool,
    ) {
        if self.reject_media_import_while_busy() {
            return;
        }
        let Some(source_name) = path
            .file_stem()
            .map(|name| name.to_string_lossy().into_owned())
        else {
            self.asset_status = "ファイル名を読み取れませんでした。".to_owned();
            return;
        };
        let composition = self.project_model.active_composition();
        let composition_id = composition.id.clone();
        let fps = composition.fps.max(1);
        let import_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default()
            .to_string();
        let layer_id = format!("ameui_video_{import_id}");
        let project_file_path = self.project_file_path.clone();
        let project_name = self.project_name.clone();
        let project_generation = self.project_generation;
        let source = path.display().to_string();
        let destination = std::env::temp_dir()
            .join("kagari_media")
            .join(format!("{}_{}", source_name, import_id));
        let (sender, receiver) = std::sync::mpsc::channel();
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        let worker = std::thread::Builder::new()
            .name("kagari-video-import".to_owned())
            .spawn(move || {
                let result = kagari_vfx::core::video_import::import_video_with_cancel(
                    &source,
                    &destination,
                    fps as f32,
                    &worker_cancel,
                );
                let _ = sender.send(result);
            });
        let worker = match worker {
            Ok(worker) => worker,
            Err(error) => {
                self.asset_status = format!("動画の読み込みを開始できませんでした: {error}");
                return;
            }
        };
        self.video_import = Some(VideoImportJob {
            worker: Some(worker),
            receiver,
            cancel,
            cancel_supported: true,
            is_image_sequence: false,
            source_name: source_name.clone(),
            layer_id,
            replace_layer_id,
            replace_project_asset_id,
            register_project_asset,
            composition_id,
            project_file_path,
            project_name,
            fps,
            project_generation,
            asset_parent_folder: self.asset_folder_id.clone(),
        });
        self.asset_status = format!("動画を展開中: {source_name}");
        self.sync_bindings_logged();
    }

    pub(crate) fn cancel_video_import(&mut self) {
        if let Some(job) = &self.video_import {
            if !job.cancel_supported {
                return;
            }
            job.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
            self.asset_status = "素材の読み込みを中止しています…".to_owned();
            self.sync_bindings_logged();
        }
    }

    pub(crate) fn receive_video_import_result(&mut self) -> bool {
        let result = match self
            .video_import
            .as_ref()
            .map(|job| job.receiver.try_recv())
        {
            Some(Ok(result)) => Some(result),
            Some(Err(std::sync::mpsc::TryRecvError::Disconnected)) => {
                Some(Err("素材の読み込み処理が途中で終了しました。".to_owned()))
            }
            Some(Err(std::sync::mpsc::TryRecvError::Empty)) | None => None,
        };
        let Some(result) = result else {
            return false;
        };
        let Some(job) = self.video_import.take() else {
            return false;
        };
        if let Some(worker) = job.worker {
            let _ = worker.join();
        }
        let target_is_current = self.screen == Screen::Editor
            && self.project_generation == job.project_generation
            && self.project_name == job.project_name
            && self.project_file_path == job.project_file_path
            && if let Some(asset_id) = job.replace_project_asset_id.as_deref() {
                self.project_model.assets.iter().any(|item| {
                    item.id == asset_id
                        && matches!(
                            &item.item_type,
                            kagari_vfx::core::timeline::ProjectItemType::Video { .. }
                        )
                })
            } else {
                self.project_model.active_composition().id == job.composition_id
                    && self.project_model.active_composition().fps.max(1) == job.fps
            };
        match result {
            Ok(asset) if target_is_current => {
                let audio_warning = asset.audio_warning.clone();
                let relinking_project_asset = if let Some(asset_id) =
                    job.replace_project_asset_id.as_deref()
                {
                    let old_path = self
                        .project_model
                        .assets
                        .iter()
                        .find(|item| item.id == asset_id)
                        .and_then(|item| item.media_path())
                        .map(str::to_owned);
                    if let Some(old_path) = old_path {
                        if let Some(item) = self
                            .project_model
                            .assets
                            .iter_mut()
                            .find(|item| item.id == asset_id)
                        {
                            item.item_type = kagari_vfx::core::timeline::ProjectItemType::Video {
                                path: asset.source_path.clone(),
                                duration_sec: asset.frame_count as f32 / job.fps.max(1) as f32,
                            };
                        }
                        replace_project_video_references(
                            &mut self.project_model.compositions,
                            &old_path,
                            &asset,
                        );
                        self.asset_status =
                            format!("プロジェクト動画を再指定しました: {}", job.source_name);
                        append_audio_warning(&mut self.asset_status, audio_warning.as_deref());
                        self.mark_project_changed();
                        true
                    } else {
                        false
                    }
                } else if job.register_project_asset {
                    self.register_project_asset_in_folder(
                        job.source_name.clone(),
                        kagari_vfx::core::timeline::ProjectItemType::Video {
                            path: asset.source_path.clone(),
                            duration_sec: asset.frame_count as f32 / job.fps.max(1) as f32,
                        },
                        job.asset_parent_folder.clone(),
                    );
                    false
                } else {
                    false
                };
                if relinking_project_asset {
                } else if let Some(replace_layer_id) = job.replace_layer_id.as_deref() {
                    let (playback_speed, audio_volume) = self
                        .project_model
                        .active_composition()
                        .layers
                        .iter()
                        .find(|layer| layer.id == replace_layer_id)
                        .and_then(|layer| match &layer.layer_type {
                            kagari_vfx::core::timeline::LayerType::Video {
                                speed,
                                audio_volume,
                                ..
                            } => Some((*speed, audio_volume.clone())),
                            _ => None,
                        })
                        .unwrap_or_else(|| {
                            (
                                1.0,
                                kagari_vfx::core::timeline::Animatable::new_constant(0.0),
                            )
                        });
                    let import_dir = std::path::Path::new(&asset.frames_dir)
                        .parent()
                        .map(std::path::Path::to_path_buf);
                    let replacement = kagari_vfx::core::timeline::LayerType::Video {
                        source: asset.source_path,
                        frames_dir: asset.frames_dir,
                        frame_count: asset.frame_count,
                        audio_wav: asset.audio_wav,
                        audio_volume,
                        speed: playback_speed,
                    };
                    let replaced = self
                        .project_model
                        .active_composition_mut()
                        .layers
                        .iter_mut()
                        .find(|layer| layer.id == replace_layer_id)
                        .filter(|layer| {
                            matches!(
                                &layer.layer_type,
                                kagari_vfx::core::timeline::LayerType::Video { .. }
                            )
                        })
                        .map(|layer| {
                            layer.layer_type = replacement;
                            layer.name = job.source_name.clone();
                        })
                        .is_some();
                    if replaced {
                        self.selected_layer_id = Some(replace_layer_id.to_owned());
                        self.asset_status = format!("動画を再指定しました: {}", job.source_name);
                        append_audio_warning(&mut self.asset_status, audio_warning.as_deref());
                        self.sync_selected_layer_controls();
                        self.mark_project_changed();
                    } else {
                        if let Some(import_dir) = import_dir {
                            let _ = std::fs::remove_dir_all(import_dir);
                        }
                        self.asset_status =
                            "再指定先の動画レイヤーが見つからなくなりました。".to_owned();
                    }
                } else {
                    let composition = self.project_model.active_composition_mut();
                    let comp_width = composition.width as f32;
                    let comp_height = composition.height as f32;
                    let fit = (comp_width / asset.width.max(1) as f32)
                        .min(comp_height / asset.height.max(1) as f32)
                        .min(1.0)
                        * 100.0;
                    let mut layer = kagari_vfx::core::timeline::Layer::new(
                        job.layer_id.clone(),
                        job.source_name.clone(),
                        kagari_vfx::core::timeline::LayerType::Video {
                            source: asset.source_path,
                            frames_dir: asset.frames_dir,
                            frame_count: asset.frame_count,
                            audio_wav: asset.audio_wav,
                            audio_volume: kagari_vfx::core::timeline::Animatable::new_constant(0.0),
                            speed: 1.0,
                        },
                        asset.frame_count.max(1),
                    );
                    layer.transform.position =
                        kagari_vfx::core::timeline::Animatable::new_constant([
                            comp_width * 0.5,
                            comp_height * 0.5,
                        ]);
                    layer.transform.scale =
                        kagari_vfx::core::timeline::Animatable::new_constant([fit, fit]);
                    composition.add_layer(layer);
                    self.selected_layer_id = Some(job.layer_id);
                    self.asset_status = if job.is_image_sequence {
                        format!("連番画像を追加しました: {}", job.source_name)
                    } else {
                        format!("動画を追加しました: {}", job.source_name)
                    };
                    append_audio_warning(&mut self.asset_status, audio_warning.as_deref());
                    self.sync_selected_layer_controls();
                    self.mark_project_changed();
                }
            }
            Ok(asset) => {
                if let Some(import_dir) = std::path::Path::new(&asset.frames_dir).parent() {
                    let _ = std::fs::remove_dir_all(import_dir);
                }
                self.asset_status = "読み込み中にプロジェクトまたはコンポジションが切り替わったため、素材を追加しませんでした。".to_owned();
            }
            Err(_error) if job.cancel.load(std::sync::atomic::Ordering::Relaxed) => {
                self.asset_status = "素材の読み込みを中止しました。".to_owned();
            }
            Err(error) => {
                self.asset_status = if job.is_image_sequence {
                    format!("連番画像を読み込めませんでした: {error}")
                } else {
                    format!("動画を読み込めませんでした: {error}")
                };
            }
        }
        self.sync_bindings_logged();
        true
    }
}

fn append_audio_warning(status: &mut String, warning: Option<&str>) {
    if let Some(warning) = warning {
        status.push_str(&format!(" 音声を取り込めませんでした: {warning}"));
    }
}
