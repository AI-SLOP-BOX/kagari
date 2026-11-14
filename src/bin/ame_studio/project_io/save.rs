use super::*;

fn create_backup_snapshot_atomically(
    source: &std::path::Path,
    destination: &std::path::Path,
) -> Result<(), String> {
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    let parent = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."));
    let name = destination
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or("project.json");
    let mut temporary = None;
    let mut published_identity = None;
    let result = (|| {
        let mut input = std::fs::File::open(source)
            .map_err(|error| format!("Cannot open project for backup: {error}"))?;
        let source_metadata = input
            .metadata()
            .map_err(|error| format!("Cannot inspect project for backup: {error}"))?;
        let mut staged = None;
        for _ in 0..100 {
            let candidate = parent.join(format!(
                ".{name}.tmp.{}.{}",
                std::process::id(),
                TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ));
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            match options.open(&candidate) {
                Ok(file) => {
                    temporary = Some(candidate);
                    staged = Some(file);
                    break;
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(format!("Cannot create temporary backup snapshot: {error}"));
                }
            }
        }
        let Some(mut output) = staged else {
            return Err("Cannot reserve a unique temporary backup snapshot".to_owned());
        };
        let temporary_path = temporary
            .as_deref()
            .expect("staged backup file has a temporary path");
        std::io::copy(&mut input, &mut output)
            .map_err(|error| format!("Cannot copy project into backup snapshot: {error}"))?;
        output
            .flush()
            .and_then(|()| output.sync_all())
            .map_err(|error| format!("Cannot sync backup snapshot: {error}"))?;
        drop(output);
        let identity =
            kagari_vfx::core::ffmpeg_export::publish_file_no_replace(temporary_path, destination)
                .map_err(|error| format!("Cannot publish backup snapshot: {error}"))?;
        published_identity = Some(identity);
        let _ = std::fs::remove_file(temporary_path);
        let published = std::fs::File::open(destination)
            .map_err(|error| format!("Cannot verify published backup snapshot: {error}"))?;
        let current = kagari_vfx::core::ffmpeg_export::types::file_identity(&published)
            .map_err(|error| format!("Cannot inspect published backup snapshot: {error}"))?;
        if current != identity {
            return Err("Backup destination changed while it was being published".to_owned());
        }
        published
            .set_permissions(source_metadata.permissions())
            .map_err(|error| format!("Cannot preserve backup permissions: {error}"))?;
        if let Ok(directory) = std::fs::File::open(parent) {
            let _ = directory.sync_all();
        }
        Ok(())
    })();

    if result.is_err() {
        if let Some(identity) = published_identity {
            let _ = kagari_vfx::core::atomic_file::remove_file_if_identity(destination, identity);
        }
        if let Some(temporary) = temporary {
            let _ = std::fs::remove_file(temporary);
        }
    }
    result
}

impl AfterEffectsStudio {
    pub(crate) fn activate_project_file(&mut self, path: &std::path::Path) {
        self.wait_for_autosave();
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
        if self.confirm_close_project && self.project_is_dirty {
            self.pending_project_path = Some(path.to_path_buf());
            self.project_switch_confirm_open = true;
        } else {
            self.activate_project_file(path);
        }
    }

    pub(crate) fn begin_new_project_setup(&mut self) {
        self.wait_for_autosave();
        self.cancel_video_import();
        self.invalidate_roto_operations();
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
        if self.confirm_close_project && self.project_is_dirty {
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
            let file_name = if self.auto_project_names {
                format!("{suggested_stem}.json")
            } else {
                "project.json".to_owned()
            };
            let mut dialog = rfd::FileDialog::new()
                .add_filter("Kagari Project", &["json"])
                .set_file_name(&file_name);
            let directory = expand_home_path(self.project_dir.trim());
            if directory.is_dir() {
                dialog = dialog.set_directory(directory);
            }
            dialog.save_file()
        });
        let Some(path) = path else {
            return;
        };
        self.wait_for_autosave();
        match kagari_vfx::core::project_migration::save_project_atomic_from(
            &self.project_model,
            self.project_file_path.as_deref(),
            &path,
        ) {
            Ok(()) => {
                self.project_name = path
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .unwrap_or("Project")
                    .to_owned();
                self.project_file_path = Some(path.clone());
                self.record_recent_project(&path);
                self.project_is_dirty = false;
                self.save_status = "保存しました".to_owned();
                self.last_autosave_at = std::time::Instant::now();
            }
            Err(error) => self.save_status = format!("保存できませんでした: {error}"),
        }
    }

    pub(crate) fn maybe_autosave(&mut self) {
        self.poll_autosave();
        if self.autosave_job.is_some()
            || self.autosave_secs == 0
            || !self.project_is_dirty
            || self.last_autosave_at.elapsed().as_secs() < self.autosave_secs
        {
            return;
        }
        let Some(path) = self.project_file_path.clone() else {
            return;
        };
        let autosave_dir = expand_home_path(self.autosave_dir.trim());
        let target = if self.autosave_dir.trim().is_empty() {
            path.clone()
        } else {
            autosave_dir.join(autosave_snapshot_name(&path))
        };
        let project = self.project_model.clone();
        let backup_dir = self.backup_dir.trim().to_owned();
        let backup_generations = self.backup_generations.clamp(1, 100);
        let autosave_dir_configured = !self.autosave_dir.trim().is_empty();
        let source_path = path.clone();
        let target_path = target.clone();
        let preview_revision = self.preview_revision;
        let project_generation = self.project_generation;
        let (sender, receiver) = std::sync::mpsc::channel();
        let worker = std::thread::Builder::new()
            .name("kagari-project-autosave".into())
            .spawn(move || {
                let result = (|| {
                    let mut backup_status = None;
                    if !backup_dir.is_empty() {
                        let directory = expand_home_path(backup_dir.trim());
                        match std::fs::create_dir_all(&directory) {
                            Ok(()) => {
                                let snapshot = directory.join(autosave_snapshot_name(&source_path));
                                if let Err(error) =
                                    create_backup_snapshot_atomically(&source_path, &snapshot)
                                {
                                    backup_status =
                                        Some(format!("バックアップの作成に失敗しました: {error}"));
                                } else {
                                    let extension = source_path
                                        .extension()
                                        .and_then(|extension| extension.to_str())
                                        .unwrap_or("json");
                                    let prefix = autosave_snapshot_prefix(&source_path);
                                    if let Err(error) = prune_backup_files(
                                        &directory,
                                        &prefix,
                                        extension,
                                        backup_generations,
                                    ) {
                                        backup_status = Some(format!(
                                            "バックアップの整理に失敗しました: {error}"
                                        ));
                                    }
                                }
                            }
                            Err(error) => {
                                backup_status = Some(format!(
                                    "バックアップフォルダを作成できませんでした: {error}"
                                ));
                            }
                        }
                    }
                    if autosave_dir_configured {
                        std::fs::create_dir_all(&autosave_dir).map_err(|error| {
                            format!("自動保存の保存先を作成できませんでした: {error}")
                        })?;
                    }
                    kagari_vfx::core::project_migration::save_project_atomic_from(
                        &project,
                        Some(source_path.as_path()),
                        &target_path,
                    )?;
                    Ok(backup_status)
                })();
                let (save_result, backup_status) = match result {
                    Ok(backup_status) => (Ok(()), backup_status),
                    Err(error) => (Err(error), None),
                };
                let _ = sender.send((save_result, backup_status));
            });
        match worker {
            Ok(worker) => {
                self.autosave_job = Some(AutosaveJob {
                    preview_revision,
                    project_generation,
                    project_path: path,
                    target_path: target,
                    receiver,
                    worker: Some(worker),
                });
                self.save_status = "自動保存中…".to_owned();
            }
            Err(error) => {
                self.last_autosave_at = std::time::Instant::now()
                    - std::time::Duration::from_secs(self.autosave_secs.saturating_sub(5));
                self.save_status = format!("自動保存を開始できませんでした: {error}");
            }
        }
    }

    fn poll_autosave(&mut self) {
        let Some(job) = self.autosave_job.as_ref() else {
            return;
        };
        let outcome = match job.receiver.try_recv() {
            Ok((result, backup_status)) => Ok((result, backup_status)),
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                Err("自動保存ワーカーが結果を返さず終了しました".to_owned())
            }
        };
        let mut job = self.autosave_job.take().expect("autosave job was present");
        if let Some(worker) = job.worker.take() {
            let _ = worker.join();
        }
        self.apply_autosave_result(job, outcome);
    }

    fn wait_for_autosave(&mut self) {
        let Some(mut job) = self.autosave_job.take() else {
            return;
        };
        let outcome = job
            .receiver
            .recv()
            .map_err(|error| format!("自動保存ワーカーが結果を返しませんでした: {error}"));
        if let Some(worker) = job.worker.take() {
            let _ = worker.join();
        }
        self.apply_autosave_result(job, outcome);
    }

    fn apply_autosave_result(
        &mut self,
        job: AutosaveJob,
        outcome: Result<(Result<(), String>, Option<String>), String>,
    ) {
        self.last_autosave_at = std::time::Instant::now();
        let (result, backup_status) = match outcome {
            Ok((result, backup_status)) => (result, backup_status),
            Err(error) => (Err(error), None),
        };
        match result {
            Ok(()) => {
                let matches_current_project = self.project_generation == job.project_generation
                    && self.project_file_path.as_ref() == Some(&job.project_path);
                let saved_current_revision =
                    matches_current_project && self.preview_revision == job.preview_revision;
                if job.target_path == job.project_path && saved_current_revision {
                    self.project_is_dirty = false;
                }
                self.save_status = match (backup_status, saved_current_revision) {
                    (Some(message), _) => format!("自動保存しました（{message}）"),
                    (None, false) if matches_current_project => {
                        "自動保存しました（追加の変更があります）".to_owned()
                    }
                    (None, _) if job.target_path == job.project_path => {
                        "自動保存しました".to_owned()
                    }
                    (None, _) => format!("自動保存しました: {}", job.target_path.display()),
                };
            }
            Err(error) => {
                self.last_autosave_at = std::time::Instant::now()
                    - std::time::Duration::from_secs(self.autosave_secs.saturating_sub(5));
                self.save_status = format!("自動保存できませんでした: {error}");
            }
        }
    }

    pub fn load_project_file(
        &mut self,
        path: &std::path::Path,
        default_name: &str,
    ) -> Result<(), String> {
        self.stop_playback();
        let is_kagari_template = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                extension.eq_ignore_ascii_case("kgtpl") || extension.eq_ignore_ascii_case("mogrt")
            });
        let is_otio = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("otio"));
        let primary_result = (|| -> Result<kagari_vfx::core::timeline::Project, String> {
            let content = kagari_vfx::core::project_migration::read_bounded_text_file(
                path,
                kagari_vfx::core::project_migration::MAX_PROJECT_JSON_BYTES,
            )
            .map_err(|error| format!("プロジェクトを読み込めません: {error}"))?;
            if is_otio {
                let timeline = kagari_vfx::core::integration::OtioTimeline::from_json(&content)
                    .map_err(|error| format!("OTIO を解析できませんでした: {error}"))?;
                let composition = timeline.to_composition().map_err(|error| {
                    format!("OTIO のコンポジションを復元できませんでした: {error}")
                })?;
                let project = kagari_vfx::core::timeline::Project {
                    compositions: vec![composition],
                    active_composition_idx: 0,
                    assets: vec![kagari_vfx::core::timeline::ProjectItem::new(
                        "item_otio_comp",
                        "OTIO Import",
                        kagari_vfx::core::timeline::ProjectItemType::Composition { comp_idx: 0 },
                    )],
                    ..Default::default()
                };
                return Ok(project);
            }
            let embedded_json = if is_kagari_template {
                kagari_vfx::core::mogrt_engine::MogrtPackage::from_json(&content)
                    .map(|package| package.project_json)
                    .map_err(|error| {
                        format!("Kagari テンプレートを解析できませんでした: {error}")
                    })?
            } else {
                content
            };
            kagari_vfx::core::project_migration::load_project_document_or_legacy(&embedded_json)
                .map(|(project, _)| project)
                .map_err(|error| format!("プロジェクト形式を読み込めません: {error}"))
        })();

        let (project, recovered_from_backup) = match primary_result {
            Ok(project) => (project, false),
            Err(primary_error) if !is_otio && !is_kagari_template => {
                match kagari_vfx::core::project_migration::load_project_with_backup(path) {
                    Ok((project, used_backup)) => (project, used_backup),
                    Err(backup_error) => {
                        return Err(format!(
                            "{primary_error}; バックアップからも復元できませんでした: {backup_error}"
                        ));
                    }
                }
            }
            Err(error) => return Err(error),
        };

        let file_path = if is_kagari_template || is_otio {
            None
        } else {
            Some(path.to_path_buf())
        };
        self.install_loaded_project(project, default_name, file_path)?;
        if recovered_from_backup {
            self.project_is_dirty = true;
            self.save_status =
                "バックアップから復元しました。保存して修復してください。".to_owned();
        }
        Ok(())
    }

    pub(crate) fn install_loaded_project(
        &mut self,
        project: kagari_vfx::core::timeline::Project,
        default_name: &str,
        file_path: Option<std::path::PathBuf>,
    ) -> Result<(), String> {
        self.invalidate_roto_operations();
        self.project_name = default_name.to_owned();
        self.project_file_path = file_path;
        self.project_is_dirty = false;
        self.save_status.clear();
        self.last_autosave_at = std::time::Instant::now();
        self.output_path = format!("~/Movies/{default_name}.png");
        self.project_model = project;
        self.history = kagari_vfx::core::history::ProjectHistory::new(self.project_model.clone());
        self.history.set_max_history_entries(self.undo_steps);
        kagari_vfx::core::frame_cache::bump_version();
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
        if let Some(path) = self.project_file_path.clone() {
            self.record_recent_project(&path);
        }
        Ok(())
    }
}
