//! Shared project open/save helpers + recent-projects list persisted in
//! the prefs file. Used by the File menu, the welcome screen, and the
//! command palette.
use crate::KagariApp;
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

const MAX_PREFS_BYTES: usize = 1024 * 1024;
const MAX_STARRED_PROJECTS: usize = 100;
static PREFS_WRITE_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static PREFS_WRITE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub(crate) fn prefs_path() -> std::path::PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        return std::path::PathBuf::from(home).join(".kagari_prefs.json");
    }
    if let Ok(config_home) = std::env::var("XDG_CONFIG_HOME") {
        return std::path::PathBuf::from(config_home)
            .join("kagari")
            .join("prefs.json");
    }
    if let Ok(app_data) = std::env::var("APPDATA") {
        return std::path::PathBuf::from(app_data)
            .join("Kagari")
            .join("prefs.json");
    }
    std::env::current_dir()
        .unwrap_or_else(|_| std::env::temp_dir())
        .join(".kagari_prefs.json")
}

#[derive(serde::Serialize, serde::Deserialize)]
struct RecentsFile {
    #[serde(default)]
    recent_projects: Vec<String>,
    /// Starred project paths. Added later; defaults to empty for old files.
    #[serde(default)]
    starred_projects: Vec<String>,
    /// Whether the welcome screen shows on startup. Defaults to true for
    /// prefs files written before this flag existed.
    #[serde(default = "default_welcome_on_startup")]
    show_welcome_on_startup: bool,
}

impl Default for RecentsFile {
    fn default() -> Self {
        Self {
            recent_projects: Vec::new(),
            starred_projects: Vec::new(),
            show_welcome_on_startup: true,
        }
    }
}

fn default_welcome_on_startup() -> bool {
    true
}

fn read_prefs() -> RecentsFile {
    let mut prefs = load_prefs_value()
        .and_then(|value| serde_json::from_value::<RecentsFile>(value).ok())
        .unwrap_or_default();
    prefs.recent_projects = normalize_path_keys(prefs.recent_projects, 8);
    prefs.starred_projects = normalize_path_keys(prefs.starred_projects, MAX_STARRED_PROJECTS);
    prefs
}

pub(crate) fn load_prefs_value() -> Option<serde_json::Value> {
    let path = prefs_path();
    let text =
        crate::core::project_migration::read_bounded_text_file(&path, MAX_PREFS_BYTES).ok()?;
    match serde_json::from_str(&text) {
        Ok(serde_json::Value::Object(object)) => Some(serde_json::Value::Object(object)),
        Ok(_) | Err(_) => {
            let backup = path.with_file_name(format!(
                ".kagari_prefs.corrupt.{}.{}.json",
                std::process::id(),
                PREFS_WRITE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ));
            let _ = std::fs::rename(path, backup);
            None
        }
    }
}

/// Update only the requested preference fields while preserving fields owned
/// by the preferences dialog or newer app versions.
pub(crate) fn update_prefs<F>(update: F)
where
    F: FnOnce(&mut serde_json::Map<String, serde_json::Value>),
{
    let _write_guard = PREFS_WRITE_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let path = prefs_path();
    let mut root = load_prefs_value()
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();
    update(&mut root);
    let Ok(json) = serde_json::to_vec_pretty(&serde_json::Value::Object(root)) else {
        return;
    };
    let sequence = PREFS_WRITE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let temporary = path.with_file_name(format!(
        ".kagari_prefs.json.tmp.{}.{}",
        std::process::id(),
        sequence
    ));
    let result = (|| {
        let mut file = std::fs::File::create(&temporary).ok()?;
        use std::io::Write;
        file.write_all(&json).ok()?;
        file.sync_all().ok()?;
        drop(file);
        std::fs::rename(&temporary, &path).ok()
    })();
    if result.is_none() {
        let _ = std::fs::remove_file(temporary);
    }
}

pub(crate) fn path_key(path: &std::path::Path) -> String {
    if path.as_os_str().is_empty() {
        return String::new();
    }
    let normalized = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    normalized.to_string_lossy().into_owned()
}

fn path_list(
    root: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    limit: usize,
) -> Vec<String> {
    root.get(key)
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .map(|paths| normalize_path_keys(paths, limit))
        .unwrap_or_default()
}

fn set_path_list(
    root: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    paths: &[String],
) {
    root.insert(key.to_string(), serde_json::json!(paths));
}

fn normalize_path_keys(paths: Vec<String>, limit: usize) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::with_capacity(paths.len().min(limit));
    for path in paths {
        let key = path_key(std::path::Path::new(&path));
        if !key.is_empty() && seen.insert(key.clone()) {
            normalized.push(key);
        }
    }
    if normalized.len() > limit {
        let excess = normalized.len() - limit;
        normalized.drain(..excess);
    }
    normalized
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
    update_prefs(|root| {
        root.insert(
            "show_welcome_on_startup".into(),
            serde_json::Value::Bool(show),
        );
    });
}

/// Insert path at front of recents (deduped, capped at 8) and persist.
pub fn push_recent(path: &std::path::Path) {
    let s = path_key(path);
    update_prefs(|root| {
        let mut recents = path_list(root, "recent_projects", 8);
        recents.retain(|p| p != &s);
        recents.insert(0, s);
        recents.truncate(8);
        set_path_list(root, "recent_projects", &recents);
    });
}

/// Starred project paths shown in Home > Starred.
pub fn starred_projects() -> Vec<String> {
    read_prefs().starred_projects
}

/// Toggle the starred flag for a project path. Returns true if now starred.
#[must_use]
pub fn toggle_starred(path: &std::path::Path) -> bool {
    let s = path_key(path);
    let mut now_starred = false;
    update_prefs(|root| {
        let mut stars = path_list(root, "starred_projects", MAX_STARRED_PROJECTS);
        let mut prefs = RecentsFile::default();
        prefs.starred_projects = std::mem::take(&mut stars);
        now_starred = toggle_starred_key(&mut prefs, s);
        set_path_list(root, "starred_projects", &prefs.starred_projects);
    });
    now_starred
}

fn toggle_starred_key(prefs: &mut RecentsFile, key: String) -> bool {
    if prefs.starred_projects.iter().any(|path| path == &key) {
        prefs.starred_projects.retain(|path| path != &key);
        false
    } else {
        prefs.starred_projects.push(key);
        if prefs.starred_projects.len() > MAX_STARRED_PROJECTS {
            let excess = prefs.starred_projects.len() - MAX_STARRED_PROJECTS;
            prefs.starred_projects.drain(..excess);
        }
        true
    }
}

/// Load a project file into app state. Returns Ok(()) or an error message.
pub fn open_project_from_path(app: &mut KagariApp, path: &std::path::Path) -> Result<(), String> {
    let json = crate::core::project_migration::read_project_json_file(path)
        .map_err(|e| format!("Could not access project file: {}", e))?;
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
        let cache_root = project_dir.join(".kagari_media").join("relinked");
        let (refreshed, errors) =
            crate::core::video_import::refresh_missing_video_caches(&mut project, &cache_root);
        if refreshed > 0 {
            app.toasts.info(format!(
                "Rebuilt {} relinked video cache{}",
                refreshed,
                if refreshed == 1 { "" } else { "s" }
            ));
        }
        for error in errors {
            app.toasts
                .warning(format!("Could not rebuild relinked video cache: {error}"));
        }
    }

    app.history = crate::core::history::ProjectHistory::new(project);
    app.mogrt_properties.clear();
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
        let mut document =
            crate::core::production_document::ProductionDocument::new(project_snapshot.clone());
        document.audio.correction = audio_correction;
        document.audio.channels = audio_channels;
        document.audio.master_gain = master_gain;
        document
            .save_atomic(path)
            .map_err(|e| format!("Failed to upgrade and save production document: {}", e))?;
        app.production_document = Some(document);
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

    #[test]
    fn starred_projects_toggle_is_bounded_and_roundtrips() {
        let parsed: RecentsFile = serde_json::from_str(r#"{"recent_projects":[]}"#).unwrap();
        assert!(parsed.starred_projects.is_empty());

        let mut prefs = RecentsFile::default();
        assert!(toggle_starred_key(&mut prefs, "first.json".into()));
        assert!(!toggle_starred_key(&mut prefs, "first.json".into()));
        assert!(prefs.starred_projects.is_empty());

        for index in 0..=MAX_STARRED_PROJECTS {
            assert!(toggle_starred_key(
                &mut prefs,
                format!("project-{index}.json")
            ));
        }
        assert_eq!(prefs.starred_projects.len(), MAX_STARRED_PROJECTS);
        assert!(!prefs.starred_projects.iter().any(|p| p == "project-0.json"));
        assert!(prefs
            .starred_projects
            .iter()
            .any(|p| p == "project-100.json"));
    }
}
