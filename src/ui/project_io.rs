//! Shared project open/save helpers + recent-projects list persisted in
//! the prefs file. Used by the File menu, the welcome screen, and the
//! command palette.
use crate::KagariApp;

fn prefs_path() -> std::path::PathBuf {
    std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir())
        .join(".kagari_prefs.json")
}

#[derive(serde::Serialize, serde::Deserialize)]
struct RecentsFile {
    #[serde(default)]
    recent_projects: Vec<String>,
    /// Whether the welcome screen shows on startup. Defaults to true for
    /// prefs files written before this flag existed.
    #[serde(default = "default_welcome_on_startup")]
    show_welcome_on_startup: bool,
}

impl Default for RecentsFile {
    fn default() -> Self {
        Self {
            recent_projects: Vec::new(),
            show_welcome_on_startup: true,
        }
    }
}

fn default_welcome_on_startup() -> bool {
    true
}

fn read_prefs() -> RecentsFile {
    std::fs::read_to_string(prefs_path())
        .ok()
        .and_then(|s| serde_json::from_str::<RecentsFile>(&s).ok())
        .unwrap_or_default()
}

fn write_prefs(prefs: &RecentsFile) {
    if let Ok(json) = serde_json::to_string_pretty(prefs) {
        let _ = std::fs::write(prefs_path(), json);
    }
}

pub fn recent_projects() -> Vec<String> {
    read_prefs().recent_projects
}

/// Whether the welcome screen should show on startup (persisted).
pub fn welcome_on_startup() -> bool {
    read_prefs().show_welcome_on_startup
}

/// Persist the welcome-on-startup preference ("Don't show again" writes false).
pub fn set_welcome_on_startup(show: bool) {
    let mut prefs = read_prefs();
    prefs.show_welcome_on_startup = show;
    write_prefs(&prefs);
}

/// Insert path at front of recents (deduped, capped at 8) and persist.
pub fn push_recent(path: &std::path::Path) {
    let s = path.to_string_lossy().to_string();
    let mut r = read_prefs();
    r.recent_projects.retain(|p| p != &s);
    r.recent_projects.insert(0, s);
    r.recent_projects.truncate(8);
    write_prefs(&r);
}

/// Load a project file into app state. Returns Ok(()) or an error message.
pub fn open_project_from_path(app: &mut KagariApp, path: &std::path::Path) -> Result<(), String> {
    let json = std::fs::read_to_string(path).map_err(|e| format!("Could not read file: {}", e))?;
    let production_document =
        crate::core::production_document::ProductionDocument::from_json(&json).ok();
    let mut project = match &production_document {
        Some(document) => document.project().clone(),
        None => crate::core::project_migration::load_project_migrated(&json)
            .map_err(|e| format!("Failed to parse project file: {}", e))?,
    };

    // Auto-resolve relative/missing external footage paths
    if let Some(project_dir) = path.parent() {
        let relinked = project.resolve_relative_footage_paths(project_dir);
        if relinked > 0 {
            app.toasts
                .info(format!("Auto-relinked {} missing footage items", relinked));
        }
    }

    app.history = crate::core::history::ProjectHistory::new(project);
    app.production_document = production_document;
    if let Some(document) = app.production_document.as_ref() {
        let correction = document.audio.correction;
        let channels = document.audio.channels.clone();
        let master_gain = document.audio.master_gain;
        app.apply_audio_correction_settings(correction);
        app.audio_mixer_channels = channels;
        app.playback.master_volume = master_gain.clamp(0.0, 1.0);
    }
    app.doc_sync_generation = 0;
    app.clear_automation_history();
    // Restore the persisted GPU-compute preference (respects adapter availability)
    let gpu_pref = app.history.current().use_gpu_compute;
    crate::core::compute_pipeline::set_gpu_effects_enabled(gpu_pref);
    app.selection.selected_layer_idx = None;
    app.selection.selected_layers.clear();
    app.project_path = path.to_string_lossy().to_string();
    push_recent(path);
    crate::core::frame_cache::bump_version();
    app.toasts.info(format!(
        "Project opened: {}",
        path.file_name().unwrap_or_default().to_string_lossy()
    ));
    Ok(())
}

/// Atomically save the current project. Returns Ok(()) or an error message.
pub fn save_project_to_path(app: &mut KagariApp, path: &std::path::Path) -> Result<(), String> {
    let project_snapshot = app.history.current().clone();
    let audio_correction = app.audio_correction_settings();
    let audio_channels = app.audio_mixer_channels.clone();
    let master_gain = app.playback.master_volume;
    if let Some(existing) = app.production_document.as_mut() {
        *existing.project_mut() = project_snapshot.clone();
        existing.audio.correction = audio_correction;
        existing.audio.channels = audio_channels;
        existing.audio.master_gain = master_gain;
        existing
            .save_atomic(path)
            .map_err(|e| format!("Failed to save production document: {}", e))?;
    } else {
        crate::core::project_migration::save_project_atomic(&project_snapshot, path)?;
    }
    app.project_path = path.to_string_lossy().to_string();
    let _ = app.autosave.save_now(&project_snapshot);
    push_recent(path);
    app.toasts.info(format!(
        "Project saved: {}",
        path.file_name().unwrap_or_default().to_string_lossy()
    ));
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_prefs_without_flag_default_to_showing_welcome() {
        // Prefs files written before the flag existed must keep old behavior.
        let parsed: RecentsFile = serde_json::from_str(r#"{"recent_projects":[]}"#).unwrap();
        assert!(parsed.show_welcome_on_startup);
        assert!(RecentsFile::default().show_welcome_on_startup);
        // Explicit false survives a serde roundtrip.
        let parsed: RecentsFile =
            serde_json::from_str(r#"{"recent_projects":[],"show_welcome_on_startup":false}"#)
                .unwrap();
        assert!(!parsed.show_welcome_on_startup);
    }
}
