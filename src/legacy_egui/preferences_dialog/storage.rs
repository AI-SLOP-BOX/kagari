use super::*;

impl Default for Prefs {
    fn default() -> Self {
        Self {
            language: crate::ui::localization::LanguagePreference::System,
            cache_mb: 12 * 1024,
            undo_steps: 50,
            autosave_secs: 300,
            recovery_slots: 10,
            audio_preview: true,
            adaptive_preview: true,
            disk_cache_gb: 100,
            disk_cache_directory: String::new(),
            custom_workspaces: Vec::new(),
            shortcut_bindings: std::collections::HashMap::new(),
            startup_home: true,
            confirm_exit: true,
            confirm_close_project: true,
            restore_previous_state: true,
            gpu_compute_enabled: None,
        }
    }
}

pub(crate) fn load() -> Prefs {
    crate::ui::project_io::load_prefs_value()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default()
}

pub(crate) fn save(p: &Prefs) -> Result<(), String> {
    let serde_json::Value::Object(fields) = serde_json::to_value(p)
        .map_err(|error| format!("Could not serialize preferences: {error}"))?
    else {
        return Err("Preferences did not serialize to an object".to_owned());
    };
    crate::ui::project_io::update_prefs(|root| {
        root.extend(fields);
    })
}

pub(crate) fn save_workspaces(
    workspaces: &[crate::ui::workspace_manager::SavedWorkspace],
) -> Result<(), String> {
    let mut prefs = load();
    prefs.custom_workspaces = workspaces.to_vec();
    save(&prefs)
}

/// Apply stored prefs to live app state. Called once at startup.
pub fn apply_loaded(app: &mut KagariApp) {
    let p = load();
    apply(app, &p);
    app.show_home = p.startup_home;
}

pub(crate) fn apply(app: &mut KagariApp, p: &Prefs) {
    crate::ui::localization::set_preference(p.language);
    crate::ui::shortcuts::defs::set_overrides(&p.shortcut_bindings);
    app.confirm_exit = p.confirm_exit;
    app.confirm_close_project = p.confirm_close_project;
    app.restore_previous_state = p.restore_previous_state;
    if let Some(enabled) = p.gpu_compute_enabled {
        if app.history.current().use_gpu_compute != enabled {
            app.modify_project(|project| project.use_gpu_compute = enabled);
        }
    }
    app.frame_cache.max_memory_bytes = p.cache_mb * 1024 * 1024;
    crate::core::frame_cache::disk_cache::set_max_disk_bytes(
        p.disk_cache_gb as u64 * 1024 * 1024 * 1024,
    );
    let cache_path = if p.disk_cache_directory.trim().is_empty() {
        None
    } else {
        Some(std::path::PathBuf::from(&p.disk_cache_directory))
    };
    if let Err(error) = crate::core::frame_cache::disk_cache::set_cache_directory(cache_path) {
        app.toasts
            .error(format!("Could not set disk cache folder: {error}"));
    }
    app.history.set_max_history_entries(p.undo_steps);
    app.autosave.set_interval_secs(p.autosave_secs);
    app.autosave.set_max_slots(p.recovery_slots);
    app.audio_preview_enabled = p.audio_preview;
    app.adaptive_preview_enabled = p.adaptive_preview;
    app.custom_workspaces = p.custom_workspaces.clone();
    if !p.adaptive_preview {
        app.playback.adaptive_preview_factor = 1.0;
    }
}

/// Restore the persisted preferences and the live runtime state to defaults.
pub fn reset_to_defaults(app: &mut KagariApp) {
    let defaults = Prefs::default();
    apply(app, &defaults);
    match save(&defaults) {
        Ok(()) => app.toasts.info("Preferences reset to defaults"),
        Err(error) => app.toasts.error(error),
    }
}
