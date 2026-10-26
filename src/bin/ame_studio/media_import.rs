use super::*;

impl AfterEffectsStudio {
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
                &["png", "jpg", "jpeg", "webp", "bmp", "tif", "tiff", "svg"],
            )
            .pick_file()
        else {
            return;
        };
        self.import_image_file(path);
    }

    pub(crate) fn import_image_file(&mut self, path: std::path::PathBuf) {
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
        let composition = self.project_model.active_composition_mut();
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
            self.asset_status = format!(
                "「{}」はロック中のため素材を再指定できません。",
                layer.name
            );
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
        if self.video_import.is_some() {
            return;
        }
        if !kagari_vfx::core::video_import::ffmpeg_available() {
            self.asset_status =
                "動画の読み込みにはFFmpegが必要です。PATHを確認してください。".to_owned();
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
        if self.video_import.is_some() {
            return;
        }
        let Some(path) = rfd::FileDialog::new()
            .add_filter("画像連番", &["png", "jpg", "jpeg", "webp"])
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
        std::thread::spawn(move || {
            let result = kagari_vfx::core::video_import::import_image_sequence_with_cancel(
                &path,
                &destination,
                fps as f32,
                &worker_cancel,
            );
            let _ = sender.send(result);
        });
        self.video_import = Some(VideoImportJob {
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
            "mp4" | "mov" | "mkv" | "avi" | "webm" | "m4v" | "mpeg" | "mpg" | "ts"
            | "m2ts" | "ivf" => self.start_video_import(path, None, None, true),
            "wav" | "mp3" | "m4a" | "aac" | "flac" | "ogg" | "aiff" | "aif" | "caf" => {
                self.import_audio_file(path);
            }
            "obj" | "gltf" | "glb" => self.import_model_file(path),
            _ => {
                self.asset_status = format!(
                    "対応していない素材形式です: {}",
                    path.extension()
                        .unwrap_or_default()
                        .to_string_lossy()
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
        if self.video_import.is_some() {
            return;
        }
        if !kagari_vfx::core::video_import::ffmpeg_available() {
            self.asset_status =
                "動画の読み込みにはFFmpegが必要です。PATHを確認してください。".to_owned();
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
        std::thread::spawn(move || {
            let result = kagari_vfx::core::video_import::import_video_with_cancel(
                &source,
                &destination,
                fps as f32,
                &worker_cancel,
            );
            let _ = sender.send(result);
        });
        self.video_import = Some(VideoImportJob {
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

    pub(crate) fn cycle_selected_layer_proxy_resolution(&mut self) {
        use kagari_vfx::core::proxy::ProxyResolution;
        let Some(id) = self.selected_layer_id.clone() else {
            return;
        };
        let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == id && !layer.locked)
        else {
            return;
        };
        layer.proxy.resolution = match layer.proxy.resolution {
            ProxyResolution::Full => ProxyResolution::Half,
            ProxyResolution::Half => ProxyResolution::Quarter,
            ProxyResolution::Quarter => ProxyResolution::Eighth,
            ProxyResolution::Eighth => ProxyResolution::Full,
        };
        self.mark_project_changed();
    }

    pub(crate) fn toggle_selected_layer_proxy(&mut self) {
        let Some(id) = self.selected_layer_id.clone() else {
            return;
        };
        let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == id && !layer.locked)
        else {
            return;
        };
        layer.proxy.enabled = !layer.proxy.enabled;
        self.mark_project_changed();
    }

    pub(crate) fn clear_selected_layer_proxy(&mut self) {
        let Some(id) = self.selected_layer_id.clone() else {
            return;
        };
        let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == id && !layer.locked)
        else {
            return;
        };
        if layer.proxy.proxy_path.take().is_some() {
            layer.proxy.enabled = false;
            self.mark_project_changed();
        }
    }

    pub(crate) fn start_selected_layer_proxy_generation(&mut self) {
        use kagari_vfx::core::proxy::{generate_preview_proxy_for_layer, ProxyResolution};
        if self.proxy_generation.is_some() {
            return;
        }
        let Some(layer_id) = self.selected_layer_id.clone() else {
            self.proxy_status = "プロキシを生成するレイヤーを選択してください。".to_owned();
            return;
        };
        let composition = self.project_model.active_composition();
        let Some(layer) = composition
            .layers
            .iter()
            .find(|layer| layer.id == layer_id && !layer.locked)
            .cloned()
        else {
            return;
        };
        let resolution = layer.proxy.resolution;
        if !matches!(
            &layer.layer_type,
            kagari_vfx::core::timeline::LayerType::Image { .. }
                | kagari_vfx::core::timeline::LayerType::Video { .. }
        ) {
            self.proxy_status = "プロキシ生成は画像と動画レイヤーに対応しています。".to_owned();
            return;
        }
        if resolution == ProxyResolution::Full {
            self.proxy_status = "生成前に解像度を1/2、1/4、1/8から選んでください。".to_owned();
            return;
        }
        let composition_id = composition.id.clone();
        let destination = std::env::temp_dir().join(format!(
            "kagari_proxy_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|time| time.as_nanos())
                .unwrap_or_default(),
        ));
        let (sender, receiver) = std::sync::mpsc::channel();
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        std::thread::spawn(move || {
            let last_sent = std::cell::Cell::new(None::<std::time::Instant>);
            let progress_sender = sender.clone();
            let mut progress = move |done: u32, total: u32| {
                let now = std::time::Instant::now();
                if done >= total
                    || last_sent.get().is_none_or(|previous| {
                        now.duration_since(previous) >= std::time::Duration::from_millis(100)
                    })
                {
                    last_sent.set(Some(now));
                    let _ =
                        progress_sender.send(LayerProxyGenerationMessage::Progress { done, total });
                }
            };
            let result = generate_preview_proxy_for_layer(
                &layer,
                resolution,
                &destination,
                &worker_cancel,
                &mut progress,
            );
            let _ = sender.send(LayerProxyGenerationMessage::Finished(result));
        });
        self.proxy_generation = Some(LayerProxyGenerationJob {
            receiver,
            cancel,
            layer_id,
            composition_id,
            project_file_path: self.project_file_path.clone(),
            project_name: self.project_name.clone(),
            project_generation: self.project_generation,
        });
        self.proxy_progress = 0;
        self.proxy_status = format!("{} プロキシを生成しています…", resolution.label());
    }

    pub(crate) fn cancel_layer_proxy_generation(&mut self) {
        if let Some(job) = &self.proxy_generation {
            job.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
            self.proxy_status = "プロキシ生成をキャンセルしています…".to_owned();
        }
    }

    pub(crate) fn receive_layer_proxy_generation(&mut self) -> bool {
        let Some(job) = self.proxy_generation.as_ref() else {
            return false;
        };
        let mut messages = Vec::new();
        let mut disconnected = false;
        loop {
            match job.receiver.try_recv() {
                Ok(message) => messages.push(message),
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    disconnected = true;
                    break;
                }
            }
        }
        if messages.is_empty() && !disconnected {
            return false;
        }
        let mut changed = false;
        for message in messages {
            match message {
                LayerProxyGenerationMessage::Progress { done, total } => {
                    self.proxy_progress = if total == 0 {
                        0
                    } else {
                        (u64::from(done).saturating_mul(100) / u64::from(total)).min(100) as i64
                    };
                    self.proxy_status =
                        format!("プロキシを生成しています… {done}/{total} フレーム");
                    changed = true;
                }
                LayerProxyGenerationMessage::Finished(result) => {
                    let Some(job) = self.proxy_generation.take() else {
                        return changed;
                    };
                    let cancelled = job.cancel.load(std::sync::atomic::Ordering::Relaxed);
                    let target_is_current = self.project_generation == job.project_generation
                        && self.project_name == job.project_name
                        && self.project_file_path == job.project_file_path
                        && self.project_model.active_composition().id == job.composition_id;
                    match result {
                        Ok(path) if !cancelled && target_is_current => {
                            let applied = self
                                .project_model
                                .active_composition_mut()
                                .layers
                                .iter_mut()
                                .find(|layer| layer.id == job.layer_id)
                                .map(|layer| {
                                    layer.proxy.proxy_path = Some(path.clone());
                                    layer.proxy.enabled = true;
                                })
                                .is_some();
                            if applied {
                                kagari_vfx::core::frame_cache::bump_version();
                                self.proxy_status =
                                    "プロキシを生成し、プレビューに適用しました。".to_owned();
                                self.proxy_progress = 100;
                                self.mark_project_changed();
                            } else {
                                remove_generated_proxy(&path);
                                self.proxy_status = "生成後に対象レイヤーがなくなったため、プロキシを適用しませんでした。".to_owned();
                            }
                        }
                        Ok(path) => {
                            remove_generated_proxy(&path);
                            self.proxy_status = if cancelled {
                                "プロキシ生成をキャンセルしました。"
                            } else {
                                "プロジェクトが切り替わったため、プロキシを適用しませんでした。"
                            }
                            .to_owned();
                        }
                        Err(error) if error == kagari_vfx::core::proxy::PROXY_CANCELLED => {
                            self.proxy_status = "プロキシ生成をキャンセルしました。".to_owned();
                        }
                        Err(error) => {
                            self.proxy_status = format!("プロキシを生成できませんでした: {error}")
                        }
                    }
                    changed = true;
                }
            }
        }
        if disconnected && self.proxy_generation.is_some() {
            self.proxy_generation = None;
            self.proxy_status = "プロキシ生成ワーカーが停止しました。".to_owned();
            changed = true;
        }
        changed
    }

    pub(crate) fn place_project_asset(&mut self, item_id: &str, fill_image: bool) {
        use kagari_vfx::core::timeline::{Layer, LayerType, ProjectItemType};
        let Some(item) = self
            .project_model
            .assets
            .iter()
            .find(|item| item.id == item_id)
            .cloned()
        else {
            return;
        };
        if let ProjectItemType::Video { path, .. } = &item.item_type {
            self.start_video_import(std::path::PathBuf::from(path), None, None, false);
            return;
        }
        if let ProjectItemType::Composition { comp_idx } = &item.item_type {
            self.place_project_composition_asset(*comp_idx, item.name);
            return;
        }
        let composition = self.project_model.active_composition();
        let (width, height, duration) = (
            composition.width as f32,
            composition.height as f32,
            composition.duration_frames,
        );
        let id = format!(
            "ameui_asset_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or_default(),
        );
        let item_name = item.name.clone();
        let stored_image_dimensions = match &item.item_type {
            ProjectItemType::Image { width, height, .. } => (*width, *height),
            _ => (0, 0),
        };
        let layer_type = match item.item_type {
            ProjectItemType::Image { path, .. } => LayerType::Image { path },
            ProjectItemType::Model3D { path } => LayerType::Model3D { path },
            ProjectItemType::Audio { path, .. } => LayerType::Audio {
                path,
                volume: kagari_vfx::core::timeline::Animatable::new_constant(0.0),
            },
            ProjectItemType::Solid { color } => LayerType::Solid { color },
            ProjectItemType::Video { .. }
            | ProjectItemType::Composition { .. }
            | ProjectItemType::Folder { .. } => return,
        };
        let is_model = matches!(&layer_type, LayerType::Model3D { .. });
        let is_image = matches!(&layer_type, LayerType::Image { .. });
        let mut layer = Layer::new(id.clone(), item_name.clone(), layer_type, duration);
        layer.is_3d = is_model;
        layer.transform.position =
            kagari_vfx::core::timeline::Animatable::new_constant([width * 0.5, height * 0.5]);
        if is_image {
            let image_dimensions = if stored_image_dimensions.0 > 0 && stored_image_dimensions.1 > 0
            {
                stored_image_dimensions
            } else if let LayerType::Image { path } = &layer.layer_type {
                kagari_vfx::core::image_cache::image_dimensions(path).unwrap_or((0, 0))
            } else {
                (0, 0)
            };
            if image_dimensions.0 > 0 && image_dimensions.1 > 0 {
                let scale = if fill_image {
                    (width / image_dimensions.0 as f32).max(height / image_dimensions.1 as f32)
                } else {
                    (width / image_dimensions.0 as f32)
                        .min(height / image_dimensions.1 as f32)
                        .min(1.0)
                } * 100.0;
                layer.transform.scale =
                    kagari_vfx::core::timeline::Animatable::new_constant([scale, scale]);
            }
        }
        self.project_model.active_composition_mut().add_layer(layer);
        self.selected_layer_id = Some(id);
        self.sync_selected_layer_controls();
        self.asset_status = format!("素材をシーンに配置しました: {}", item_name);
        self.mark_project_changed();
    }
}


