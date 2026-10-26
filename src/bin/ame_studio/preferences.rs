use super::*;

pub(crate) fn format_cache_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    let size = bytes as f64;
    if size < KB {
        format!("{bytes} B")
    } else if size < KB * KB {
        format!("{:.1} KB", size / KB)
    } else if size < KB * KB * KB {
        format!("{:.1} MB", size / (KB * KB))
    } else {
        format!("{:.2} GB", size / (KB * KB * KB))
    }
}

pub(crate) fn load_recent_items() -> Vec<RecentProjectItem> {
    let candidate_paths = read_recent_project_paths();

    let mut out = Vec::new();
    for p_str in candidate_paths {
        let path = std::path::PathBuf::from(&p_str);
        if !path.is_file() {
            continue;
        }

        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "project".to_string());

        let modified_str = std::fs::metadata(&path)
            .ok()
            .and_then(|meta| meta.modified().ok())
            .map(fmt_timestamp)
            .unwrap_or_else(|| "—".to_string());

        // Try reading project JSON for real metadata
        let mut dur_str = "—".to_string();
        let mut fps_str = "—".to_string();
        let mut res_str = "—".to_string();
        let mut comps_count = 0usize;
        let mut layers_count = 0usize;

        if let Ok(content) = kagari_vfx::core::project_migration::read_bounded_text_file(
            &path,
            kagari_vfx::core::project_migration::MAX_PROJECT_JSON_BYTES,
        ) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
                let project_root = v
                    .get("project")
                    .or_else(|| v.get("project_data"))
                    .unwrap_or(&v);
                let comps = project_root
                    .get("compositions")
                    .and_then(serde_json::Value::as_array);

                if let Some(arr) = comps {
                    comps_count = arr.len();
                    let mut total_layers = 0usize;
                    for c in arr {
                        if let Some(layers) = c.get("layers").and_then(serde_json::Value::as_array)
                        {
                            total_layers += layers.len();
                        }
                    }
                    layers_count = total_layers;

                    let active_idx = project_root
                        .get("active_composition_idx")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0) as usize;
                    if let Some(c) = arr.get(active_idx).or_else(|| arr.first()) {
                        let width = c.get("width").and_then(serde_json::Value::as_u64);
                        let height = c.get("height").and_then(serde_json::Value::as_u64);
                        let fps = c.get("fps").and_then(serde_json::Value::as_u64);
                        let duration_frames =
                            c.get("duration_frames").and_then(serde_json::Value::as_u64);
                        if let (Some(width), Some(height)) = (width, height) {
                            res_str = format!("{width} × {height}");
                        }
                        if let Some(fps) = fps {
                            fps_str = format!("{fps} fps");
                            if let Some(duration_frames) = duration_frames {
                                let total_secs = duration_frames / fps.max(1);
                                let hours = total_secs / 3600;
                                let minutes = (total_secs / 60) % 60;
                                let seconds = total_secs % 60;
                                dur_str = format!("{hours:02}:{minutes:02}:{seconds:02}");
                            }
                        }
                    }
                }
            }
        }

        let comps_info = format!("{}コンポジション · {}レイヤー", comps_count, layers_count);

        out.push(RecentProjectItem {
            path_label: compact_project_path(&path),
            path,
            name,
            date: modified_str,
            duration: dur_str,
            fps: fps_str,
            resolution: res_str,
            layer_count: layers_count,
            comps_info,
        });

        if out.len() >= 32 {
            break;
        }
    }
    out
}

pub(crate) fn compact_project_path(path: &std::path::Path) -> String {
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_default();
    let parent = path
        .parent()
        .and_then(std::path::Path::file_name)
        .map(|name| name.to_string_lossy())
        .unwrap_or_default();
    if parent.is_empty() {
        file_name.into_owned()
    } else {
        format!("…/{parent}/{file_name}")
    }
}

pub(crate) fn expand_home_path(value: &str) -> std::path::PathBuf {
    let Some(home) = std::env::var_os("HOME").map(std::path::PathBuf::from) else {
        return std::path::PathBuf::from(value);
    };
    if value == "~" {
        home
    } else if let Some(suffix) = value.strip_prefix("~/") {
        home.join(suffix)
    } else {
        std::path::PathBuf::from(value)
    }
}

pub(crate) fn fmt_timestamp(time: std::time::SystemTime) -> String {
    #[cfg(unix)]
    {
        let Ok(duration) = time.duration_since(std::time::UNIX_EPOCH) else {
            return "—".to_owned();
        };
        let timestamp = duration.as_secs() as libc::time_t;
        let mut local: libc::tm = unsafe { std::mem::zeroed() };
        if unsafe { libc::localtime_r(&timestamp, &mut local) }.is_null() {
            return "—".to_owned();
        }
        format!(
            "{:04}/{:02}/{:02} {:02}:{:02}",
            local.tm_year + 1900,
            local.tm_mon + 1,
            local.tm_mday,
            local.tm_hour,
            local.tm_min,
        )
    }
    #[cfg(not(unix))]
    {
        time.duration_since(std::time::UNIX_EPOCH)
            .map(|duration| format!("{}", duration.as_secs()))
            .unwrap_or_else(|_| "—".to_owned())
    }
}

pub(crate) fn read_recent_project_paths() -> Vec<String> {
    read_preferences()
        .get("recent_projects")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|path| path.as_str().map(str::to_owned))
        .collect()
}

pub(crate) fn read_preferences() -> serde_json::Value {
    const MAX_PREFERENCES_BYTES: usize = 1024 * 1024;
    let primary = preferences_path();
    let mut paths = vec![primary.clone()];
    if let Some(home) = std::env::var_os("HOME") {
        let legacy = std::path::PathBuf::from(home).join(".kagari_prefs.json");
        if legacy != primary {
            paths.push(legacy);
        }
    }

    for path in paths {
        let Ok(text) = kagari_vfx::core::project_migration::read_bounded_text_file(
            &path,
            MAX_PREFERENCES_BYTES,
        ) else {
            continue;
        };
        if let Ok(serde_json::Value::Object(preferences)) =
            serde_json::from_str::<serde_json::Value>(&text)
        {
            return serde_json::Value::Object(preferences);
        }
    }

    serde_json::Value::Object(serde_json::Map::new())
}

pub(crate) fn write_preference(key: &str, value: serde_json::Value) -> Result<(), String> {
    let path = preferences_path();
    let mut prefs = read_preferences().as_object().cloned().unwrap_or_default();
    prefs.insert(key.to_owned(), value);
    let serialized = serde_json::to_vec_pretty(&serde_json::Value::Object(prefs))
        .map_err(|error| error.to_string())?;
    let parent = path.parent().ok_or("Preferences path has no parent")?;
    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let temporary = path.with_file_name(format!(".prefs.json.tmp.{}", std::process::id()));
    std::fs::write(&temporary, serialized).map_err(|error| error.to_string())?;
    std::fs::rename(&temporary, &path).map_err(|error| {
        let _ = std::fs::remove_file(&temporary);
        error.to_string()
    })
}

pub(crate) fn read_workspace_ratios(
    saved: &serde_json::Value,
    key: &str,
    legacy_key: Option<&str>,
    defaults: [f64; 15],
    minimum: f64,
    maximum: f64,
) -> [f64; 15] {
    let ratios = saved.get(key).and_then(serde_json::Value::as_array);
    let legacy = legacy_key
        .and_then(|key| saved.get(key))
        .and_then(serde_json::Value::as_f64)
        .filter(|ratio| ratio.is_finite());
    std::array::from_fn(|index| {
        ratios
            .and_then(|values| values.get(index))
            .and_then(serde_json::Value::as_f64)
            .filter(|ratio| ratio.is_finite())
            .or(legacy)
            .map(|ratio| ratio.clamp(minimum, maximum))
            .unwrap_or(defaults[index])
    })
}

pub(crate) fn preferences_path() -> std::path::PathBuf {
    #[cfg(target_os = "windows")]
    if let Some(app_data) = std::env::var_os("APPDATA") {
        return std::path::PathBuf::from(app_data)
            .join("Kagari")
            .join("prefs.json");
    }
    #[cfg(target_os = "macos")]
    if let Some(home) = std::env::var_os("HOME") {
        return std::path::PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("Kagari")
            .join("prefs.json");
    }
    #[cfg(target_os = "linux")]
    if let Some(config_home) = std::env::var_os("XDG_CONFIG_HOME") {
        let path = std::path::PathBuf::from(config_home);
        if path.is_absolute() {
            return path.join("kagari").join("prefs.json");
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        #[cfg(target_os = "linux")]
        return std::path::PathBuf::from(home)
            .join(".config")
            .join("kagari")
            .join("prefs.json");
        #[cfg(not(target_os = "linux"))]
        return std::path::PathBuf::from(home).join(".kagari_prefs.json");
    }
    std::env::current_dir()
        .unwrap_or_else(|_| std::env::temp_dir())
        .join(".kagari_prefs.json")
}

pub(crate) fn recent_preview(path: Option<&std::path::Path>, index: usize) -> (AssetMime, Vec<u8>) {
    let candidate = path.and_then(|project_path| {
        let text = kagari_vfx::core::project_migration::read_bounded_text_file(
            project_path,
            kagari_vfx::core::project_migration::MAX_PROJECT_JSON_BYTES,
        )
        .ok()?;
        let value = serde_json::from_str::<serde_json::Value>(&text).ok()?;
        let root = value
            .get("project")
            .or_else(|| value.get("project_data"))
            .unwrap_or(&value);
        let comps = root.get("compositions")?.as_array()?;
        let idx = root
            .get("active_composition_idx")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0) as usize;
        let layers = comps
            .get(idx)
            .or_else(|| comps.first())?
            .get("layers")?
            .as_array()?;
        let media = layers
            .iter()
            .filter(|layer| {
                layer.get("visible").and_then(serde_json::Value::as_bool) != Some(false)
            })
            .find_map(|layer| {
                let kind = layer.get("layer_type")?.as_object()?;
                kind.get("Image")?.get("path")?.as_str()
            })?;
        let media_path = std::path::PathBuf::from(media);
        let media_path = if media_path.is_absolute() {
            media_path
        } else {
            project_path.parent()?.join(media_path)
        };
        std::fs::read(media_path).ok()
    });

    let fallback = [
        include_bytes!("../../../assets/home/recent_project_eclipse.webp").as_slice(),
        include_bytes!("../../../assets/home/recent_project_citadel.webp").as_slice(),
        include_bytes!("../../../assets/home/recent_project_rift.webp").as_slice(),
        include_bytes!("../../../assets/home/recent_project_atlas.webp").as_slice(),
    ];
    if let Some(bytes) = candidate {
        if let Ok(image) = image::load_from_memory(&bytes) {
            let image = image.thumbnail(960, 540).to_rgb8();
            let mut jpeg = std::io::Cursor::new(Vec::new());
            if image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 82)
                .encode_image(&image)
                .is_ok()
                && jpeg.get_ref().len() <= ame_ui::asset::MAX_ASSET_BYTES
            {
                return (AssetMime::Jpeg, jpeg.into_inner());
            }
        }
    }
    (AssetMime::Webp, fallback[index % fallback.len()].to_vec())
}

impl AfterEffectsStudio {
    pub(crate) fn handle_settings_action(&mut self, action_name: &str, action_value: Option<&ActionValue>) -> bool {
        match (action_name, action_value) {
            ("go_settings", _) => {
                self.settings_open = true;
                self.settings_category = 0;
                self.settings_feedback.clear();
                self.settings_snapshot = Some(self.settings_draft());
                true
            }
            ("close_settings", _) | ("settings_cancel", _) => {
                if let Some(draft) = self.settings_snapshot.take() {
                    self.restore_settings_draft(draft);
                    self.sync_bindings_logged();
                }
                self.settings_feedback.clear();
                self.settings_open = false;
                true
            }
            ("settings_apply", _) => {
                match self.commit_settings() {
                    Ok(()) => {
                        self.settings_feedback.clear();
                        self.settings_snapshot = None;
                        self.settings_open = false;
                    }
                    Err(error) => {
                        self.settings_feedback = format!("設定を保存できませんでした: {error}");
                    }
                }
                true
            }
            ("settings_general_select", _) => {
                self.settings_category = 0;
                true
            }
            ("settings_projects_select", _) => {
                self.settings_category = 1;
                true
            }
            ("settings_cache_select", _) => {
                self.settings_category = 2;
                true
            }
            ("settings_performance_select", _) => {
                self.settings_category = 3;
                true
            }
            ("settings_shortcuts_select", _) => {
                self.settings_category = 4;
                true
            }
            ("settings_interface_select", _) => {
                self.settings_category = 5;
                true
            }
            ("settings_plugins_select", _) => {
                self.settings_category = 6;
                self.refresh_openfx_plugins();
                true
            }
            ("settings_plugins_refresh", _) => {
                self.refresh_openfx_plugins();
                true
            }
            ("settings_license_select", _) => {
                self.settings_category = 7;
                true
            }
            ("setting_restore_changed", Some(ActionValue::Checked(value))) => {
                self.restore_previous_state = *value;
                true
            }
            ("setting_confirm_close_changed", Some(ActionValue::Checked(value))) => {
                self.confirm_close_project = *value;
                true
            }
            ("setting_confirm_exit_changed", Some(ActionValue::Checked(value))) => {
                self.confirm_exit = *value;
                true
            }
            ("setting_adaptive_preview_changed", Some(ActionValue::Checked(value))) => {
                self.adaptive_preview = *value;
                true
            }
            ("setting_audio_preview_changed", Some(ActionValue::Checked(value))) => {
                self.audio_preview = *value;
                true
            }
            ("setting_home_changed", Some(ActionValue::Checked(value))) => {
                self.show_home_on_startup = *value;
                true
            }
            ("setting_autosave_interval_changed", Some(ActionValue::Text(value))) => {
                if let Ok(seconds) = value.parse::<u64>() {
                    self.autosave_secs = seconds;
                }
                true
            }
            ("setting_memory_changed", Some(ActionValue::Number(value))) if value.is_finite() => {
                self.memory_limit_gb = value.round().clamp(1.0, 64.0);
                true
            }
            ("setting_undo_changed", Some(ActionValue::Number(value))) if value.is_finite() => {
                self.undo_steps = (value.round() as usize).clamp(10, 500);
                self.history.set_max_history_entries(self.undo_steps);
                true
            }
            ("setting_recovery_changed", Some(ActionValue::Number(value))) if value.is_finite() => {
                self.recovery_slots = (value.round() as usize).clamp(1, 50);
                true
            }
            ("setting_gpu_changed", Some(ActionValue::Checked(value))) => {
                self.gpu_enabled = *value;
                kagari_vfx::core::compute_pipeline::set_gpu_effects_enabled(*value);
                true
            }
            ("setting_cache_path_changed", Some(ActionValue::Text(value))) => {
                self.cache_path = value.clone();
                self.cache_path_status.clear();
                true
            }
            ("setting_cache_path_browse", _) => {
                let initial = expand_home_path(self.cache_path.trim());
                let initial = if initial.is_dir() {
                    initial
                } else {
                    std::env::var_os("HOME")
                        .map(std::path::PathBuf::from)
                        .unwrap_or_else(std::env::temp_dir)
                };
                if let Some(path) = rfd::FileDialog::new().set_directory(initial).pick_folder() {
                    self.cache_path = path.display().to_string();
                    self.cache_path_status = "変更は「適用」で保存されます。".to_owned();
                }
                true
            }
            ("setting_cache_limit_changed", Some(ActionValue::Text(value))) => {
                self.cache_limit = value.clone();
                true
            }
            ("clear_cache", _) => {
                self.clear_cache_confirm = true;
                true
            }
            ("dismiss_clear_cache", _) => {
                self.clear_cache_confirm = false;
                true
            }
            ("confirm_clear_cache", _) => {
                kagari_vfx::core::frame_cache::disk_cache::clear_all();
                kagari_vfx::core::frame_cache::bump_version();
                self.cache_usage_bytes =
                    kagari_vfx::core::frame_cache::disk_cache::current_disk_usage();
                self.clear_cache_confirm = false;
                true
            }
            ("exit_app", _) => {
                if self.confirm_exit {
                    self.exit_confirm_open = true;
                } else {
                    self.exit_requested = true;
                }
                true
            }
            ("confirm_exit_app", _) => {
                self.exit_confirm_open = false;
                self.exit_requested = true;
                true
            }
            ("cancel_exit_app", _) => {
                self.exit_confirm_open = false;
                true
            }
            ("confirm_project_switch", _) => {
                self.complete_pending_project_switch();
                true
            }
            ("save_and_confirm_project_switch", _) => {
                self.save_current_project();
                if self.save_status == "保存しました" {
                    self.complete_pending_project_switch();
                }
                true
            }
            ("cancel_project_switch", _) => {
                self.project_switch_confirm_open = false;
                self.pending_project_path = None;
                self.pending_new_project = false;
                true
            }
            _ => false,
        }
    }
}

