//! Asynchronous project opening, saving, and format migrating logic.

use super::prefs::push_recent;
use crate::KagariApp;
use std::sync::mpsc::Receiver;

pub struct PendingProjectLoad {
    pub(crate) path: std::path::PathBuf,
    pub(crate) receiver: Receiver<Result<ProjectLoadResult, String>>,
    pub(crate) project_session_id: u64,
    pub(crate) history_generation: u64,
}

pub struct PendingColorLutLoad {
    pub(crate) path: String,
    pub(crate) receiver: Receiver<(
        Option<std::sync::Arc<crate::core::ocio_color::Lut3D>>,
        Option<String>,
    )>,
    pub(crate) project_session_id: u64,
}

pub struct ProjectLoadResult {
    pub project: crate::core::timeline::Project,
    pub production_document: Option<crate::core::production_document::ProductionDocument>,
    pub active_lut: Option<std::sync::Arc<crate::core::ocio_color::Lut3D>>,
    pub lut_error: Option<String>,
    pub relinked: usize,
    pub refreshed: usize,
    pub cache_errors: Vec<String>,
}

/// Load a project file into app state. Returns Ok(()) or an error message.
pub fn open_project_from_path(app: &mut KagariApp, path: &std::path::Path) -> Result<(), String> {
    if app.pending_project_load.is_some() {
        return Err("別のプロジェクトを読み込み中です。完了後にもう一度お試しください。".into());
    }
    let path = path.to_path_buf();
    let worker_path = path.clone();
    let repaint = app.repaint_context.clone();
    let (sender, receiver) = std::sync::mpsc::channel();
    let handle = match std::thread::Builder::new()
        .name("kagari-project-open".into())
        .spawn(move || {
            let result = load_project_for_open(&worker_path);
            let _ = sender.send(result);
            if let Some(context) = repaint {
                context.request_repaint();
            }
        }) {
        Ok(handle) => handle,
        Err(error) => {
            return Err(format!(
                "プロジェクト読み込みを開始できませんでした: {error}"
            ));
        }
    };
    app.background_threads.push(handle);
    app.pending_project_load = Some(PendingProjectLoad {
        path,
        receiver,
        project_session_id: app.project_session_id,
        history_generation: app.history.generation(),
    });
    app.toasts.info("プロジェクトを読み込んでいます…");
    Ok(())
}

fn load_project_for_open(path: &std::path::Path) -> Result<ProjectLoadResult, String> {
    let json = crate::core::project_migration::read_project_json_file(path)
        .map_err(|e| format!("Could not access project file: {}", e))?;
    let mut production_document =
        crate::core::production_document::ProductionDocument::from_json(&json).ok();
    let mut project = match &production_document {
        Some(document) => document.project().clone(),
        None => crate::core::project_migration::load_project_migrated(&json)
            .map_err(|e| format!("Failed to parse project file: {}", e))?,
    };
    let saved_lut_path = production_document
        .as_ref()
        .and_then(|document| document.color_lut_path.as_deref());
    let (active_lut, lut_error) = load_project_lut(path, saved_lut_path);

    let mut relinked = 0;
    let mut refreshed = 0;
    let mut cache_errors = Vec::new();
    if let Some(project_dir) = path.parent() {
        relinked = project.resolve_relative_footage_paths(project_dir);
        let cache_root = project_dir.join(".kagari_media").join("relinked");
        let (count, errors) =
            crate::core::video_import::refresh_missing_video_caches(&mut project, &cache_root);
        refreshed = count;
        cache_errors = errors;
    }
    if let Some(document) = production_document.as_mut() {
        *document.project_mut() = project.clone();
    }

    Ok(ProjectLoadResult {
        project,
        production_document,
        active_lut,
        lut_error,
        relinked,
        refreshed,
        cache_errors,
    })
}

pub fn load_project_lut(
    project_path: &std::path::Path,
    saved_lut_path: Option<&str>,
) -> (
    Option<std::sync::Arc<crate::core::ocio_color::Lut3D>>,
    Option<String>,
) {
    let Some(saved_lut_path) = saved_lut_path else {
        return (None, None);
    };
    let saved_path = std::path::PathBuf::from(saved_lut_path);
    let candidate = if saved_path.is_absolute() {
        saved_path
    } else {
        project_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join(saved_path)
    };
    match crate::core::ocio_color::Lut3D::load_cube_file(&candidate) {
        Ok(lut) => (Some(std::sync::Arc::new(lut)), None),
        Err(error) => (None, Some(format!("{} ({error})", candidate.display()))),
    }
}

pub fn begin_color_lut_restore(app: &mut KagariApp, path: String) {
    let project_path = std::path::PathBuf::from(&app.project_path);
    let worker_path = path.clone();
    let repaint = app.repaint_context.clone();
    let (sender, receiver) = std::sync::mpsc::channel();
    match std::thread::Builder::new()
        .name("kagari-lut-restore".into())
        .spawn(move || {
            let result = load_project_lut(&project_path, Some(&worker_path));
            let _ = sender.send(result);
            if let Some(context) = repaint {
                context.request_repaint();
            }
        }) {
        Ok(handle) => {
            app.background_threads.push(handle);
            app.color_lut_path = Some(path.clone());
            app.color_lut_mode = 0;
            app.pending_color_lut_load = Some(PendingColorLutLoad {
                path,
                receiver,
                project_session_id: app.project_session_id,
            });
        }
        Err(error) => app.toasts.warning(format!(
            "保存されたLUTの読み込みを開始できませんでした: {error}"
        )),
    }
}

pub fn poll_color_lut_restore(app: &mut KagariApp, ctx: &eframe::egui::Context) {
    let Some(pending) = app.pending_color_lut_load.take() else {
        return;
    };
    match pending.receiver.try_recv() {
        Ok((lut, error)) => {
            if pending.project_session_id != app.project_session_id
                || app.color_lut_path.as_deref() != Some(pending.path.as_str())
            {
                return;
            }
            app.color_lut_mode = if lut.is_some() { 3 } else { 0 };
            crate::core::ocio_color::set_active_lut(lut);
            if let Some(error) = error {
                app.toasts.warning(format!(
                    "保存されていたLUTを読み込めませんでした。パスを確認してください: {error}"
                ));
            }
            crate::core::frame_cache::bump_version();
        }
        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
            app.toasts
                .warning("保存されたLUTの読み込みworkerが終了しました。");
        }
        Err(std::sync::mpsc::TryRecvError::Empty) => {
            app.pending_color_lut_load = Some(pending);
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }
}

fn apply_loaded_project(app: &mut KagariApp, path: &std::path::Path, loaded: ProjectLoadResult) {
    let ProjectLoadResult {
        project,
        production_document,
        active_lut,
        lut_error,
        relinked,
        refreshed,
        cache_errors,
    } = loaded;
    app.replace_project_history(project);
    app.mogrt_properties.clear();
    app.color_lut_path = production_document
        .as_ref()
        .and_then(|document| document.color_lut_path.clone());
    app.color_lut_mode = if active_lut.is_some() { 3 } else { 0 };
    crate::core::ocio_color::set_active_lut(active_lut);
    app.production_document = production_document;
    app.playback.cached_audio_path = None;
    app.playback.cached_audio_key = 0;
    if let Some(document) = app.production_document.as_ref() {
        let correction = document.audio.correction;
        let channels = document.audio.channels.clone();
        let master_gain = document.audio.master_gain;
        app.apply_audio_correction_settings(correction);
        app.audio_mixer_channels = channels;
        app.playback.master_volume = master_gain.clamp(0.0, 1.0);
    } else {
        app.apply_audio_correction_settings(Default::default());
        app.audio_mixer_channels.clear();
        app.playback.master_volume = 0.8;
    }
    app.doc_sync_generation = 0;
    app.clear_automation_history();
    // Restore the persisted GPU-compute preference (respects adapter availability)
    let gpu_pref = app.history.current().use_gpu_compute;
    crate::core::compute_pipeline::set_gpu_effects_enabled(gpu_pref);
    app.selection.selected_layer_idx = None;
    app.selection.selected_layers.clear();
    app.project_path = path.to_string_lossy().to_string();
    app.show_home = false;
    app.show_welcome = false;
    app.ui_tabs.left_tab_idx = 0;
    app.ui_tabs.right_panel = crate::app_state::RightPanel::Properties;
    if let Some(lut_error) = lut_error {
        app.toasts.warning(format!(
            "保存されていたLUTを読み込めませんでした。パスを確認してください: {lut_error}"
        ));
    }
    if let Err(error) = push_recent(path) {
        app.toasts.warning(crate::ui::localization::tr_format(
            "Project opened, but recent-project settings were not saved: {}",
            &[&error.to_string()],
        ));
    }
    crate::core::frame_cache::bump_version();
    if relinked > 0 {
        app.toasts.info(crate::ui::localization::tr_format(
            "Auto-relinked {} missing footage items",
            &[&relinked.to_string()],
        ));
    }
    if refreshed > 0 {
        app.toasts.info(crate::ui::localization::tr_format(
            "Rebuilt {} relinked video caches",
            &[&refreshed.to_string()],
        ));
    }
    for error in cache_errors {
        app.toasts.warning(crate::ui::localization::tr_format(
            "Could not rebuild relinked video cache: {}",
            &[&error],
        ));
    }
    let filename = path.file_name().unwrap_or_default().to_string_lossy();
    app.toasts.info(crate::ui::localization::tr_format(
        "Project opened: {}",
        &[&filename],
    ));
}

pub fn poll_pending_project_load(app: &mut KagariApp, ctx: &eframe::egui::Context) {
    let Some(pending) = app.pending_project_load.take() else {
        return;
    };
    match pending.receiver.try_recv() {
        Ok(Ok(loaded)) => {
            if pending.project_session_id != app.project_session_id
                || pending.history_generation != app.history.generation()
            {
                app.toasts.warning(
                    "読み込み中に現在のプロジェクトが変更されたため、読み込み結果を適用しませんでした。",
                );
            } else {
                apply_loaded_project(app, &pending.path, loaded);
            }
        }
        Ok(Err(error)) => app
            .toasts
            .error(format!("プロジェクトを開けませんでした: {error}")),
        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
            app.toasts
                .error("プロジェクト読み込みworkerが終了しました。");
        }
        Err(std::sync::mpsc::TryRecvError::Empty) => {
            app.pending_project_load = Some(pending);
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }
}

pub fn draw_project_load_modal(app: &KagariApp, ctx: &eframe::egui::Context) {
    let Some(pending) = app.pending_project_load.as_ref() else {
        return;
    };
    crate::ui::modal::dialog("プロジェクト読み込み")
        .title_bar(false)
        .fixed_size([360.0, 104.0])
        .anchor(eframe::egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("プロジェクトを読み込んでいます…");
            });
            ui.add_space(8.0);
            ui.label(
                pending
                    .path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy(),
            );
        });
}

/// Atomically save the current project. Returns Ok(()) or an error message.
pub fn save_project_to_path(app: &mut KagariApp, path: &std::path::Path) -> Result<(), String> {
    let project_snapshot = app.history.current().clone();
    let audio_correction = app.audio_correction_settings();
    let audio_channels = app.audio_mixer_channels.clone();
    let master_gain = app.playback.master_volume;
    if let Some(existing) = app.production_document.as_mut() {
        *existing.project_mut() = project_snapshot.clone();
        existing.color_lut_path = app.color_lut_path.clone();
        existing.audio.correction = audio_correction;
        existing.audio.channels = audio_channels;
        existing.audio.master_gain = master_gain;
        existing
            .save_atomic(path)
            .map_err(|e| format!("Failed to save production document: {}", e))?;
    } else {
        let mut document =
            crate::core::production_document::ProductionDocument::new(project_snapshot.clone());
        document.color_lut_path = app.color_lut_path.clone();
        document.audio.correction = audio_correction;
        document.audio.channels = audio_channels;
        document.audio.master_gain = master_gain;
        document
            .save_atomic(path)
            .map_err(|e| format!("Failed to upgrade and save production document: {}", e))?;
        app.production_document = Some(document);
    }
    app.project_path = path.to_string_lossy().to_string();
    let autosave_result = if let Some(document) = app.production_document.as_ref() {
        app.autosave
            .save_now_production(&project_snapshot, document)
    } else {
        app.autosave.save_now(&project_snapshot)
    };
    if let Err(error) = autosave_result {
        log::warn!("Could not update recovery snapshot after project save: {error}");
    }
    if let Err(error) = push_recent(path) {
        app.toasts.warning(format!(
            "Project saved, but recent-project settings were not saved: {error}"
        ));
    }
    app.toasts.info(format!(
        "Project saved: {}",
        path.file_name().unwrap_or_default().to_string_lossy()
    ));
    Ok(())
}

pub fn poll_collect_files(app: &mut KagariApp, ctx: &eframe::egui::Context) {
    use std::sync::mpsc::TryRecvError;
    let Some(receiver) = app.collect_files_rx.as_ref() else {
        return;
    };
    match receiver.try_recv() {
        Ok(Ok((project_path, file_count))) => {
            app.collect_files_rx = None;
            reveal_in_file_manager(&project_path);
            app.toasts.info(format!(
                "素材を{}件収集しました: {}",
                file_count,
                project_path.parent().unwrap_or(&project_path).display()
            ));
        }
        Ok(Err(error)) => {
            app.collect_files_rx = None;
            app.toasts
                .error(format!("素材を収集できませんでした: {error}"));
        }
        Err(TryRecvError::Disconnected) => {
            app.collect_files_rx = None;
            app.toasts.error("素材収集workerが予期せず終了しました。");
        }
        Err(TryRecvError::Empty) => {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }
}

/// Reveal a file (or its folder) in the OS file manager.
pub fn reveal_in_file_manager(path: &std::path::Path) {
    let target = path.to_path_buf();
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg("-R")
            .arg(&target)
            .spawn();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let dir = if target.is_file() {
            target
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or(target.clone())
        } else {
            target.clone()
        };
        let _ = std::process::Command::new("xdg-open").arg(dir).spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer")
            .arg(format!("/select,{}", target.display()))
            .spawn();
    }
}
