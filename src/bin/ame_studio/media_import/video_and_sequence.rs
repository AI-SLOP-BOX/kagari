use super::*;

impl AfterEffectsStudio {
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
