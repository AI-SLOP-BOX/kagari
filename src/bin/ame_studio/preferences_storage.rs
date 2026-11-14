use super::*;

/// Preset autosave intervals exposed by the preferences screen. Loading a
/// value outside this set would make the Select fail to find a matching
/// option, so persisted values are normalised on startup.
pub(crate) const AUTOSAVE_INTERVALS: [u64; 8] = [0, 30, 60, 180, 300, 600, 900, 1800];

pub(crate) fn normalize_autosave_secs(value: u64) -> u64 {
    if AUTOSAVE_INTERVALS.contains(&value) {
        return value;
    }
    AUTOSAVE_INTERVALS
        .into_iter()
        .min_by_key(|candidate| candidate.abs_diff(value))
        .unwrap_or(300)
}

fn home_subdir(relative: &str) -> String {
    std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .map(|home| home.join(relative).display().to_string())
        .unwrap_or_else(|| relative.to_owned())
}

pub(crate) fn default_project_dir() -> String {
    home_subdir("Documents/Kagari/Projects")
}

pub(crate) fn default_preview_cache_dir() -> String {
    #[cfg(target_os = "macos")]
    {
        home_subdir("Library/Caches/Kagari/Preview")
    }
    #[cfg(target_os = "linux")]
    {
        if let Some(cache_home) = std::env::var_os("XDG_CACHE_HOME")
            .filter(|path| std::path::PathBuf::from(path).is_absolute())
        {
            std::path::PathBuf::from(cache_home)
                .join("kagari/preview")
                .display()
                .to_string()
        } else {
            home_subdir(".cache/kagari/preview")
        }
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("LOCALAPPDATA")
            .map(std::path::PathBuf::from)
            .map(|path| path.join("Kagari/Cache").display().to_string())
            .unwrap_or_else(default_temp_dir)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    home_subdir(".cache/kagari/preview")
}

pub(crate) fn default_temp_dir() -> String {
    std::env::temp_dir().join("Kagari").display().to_string()
}

pub(crate) fn default_autosave_dir() -> String {
    home_subdir("Documents/Kagari/AutoSave")
}

pub(crate) fn default_backup_dir() -> String {
    home_subdir("Documents/Kagari/Backup")
}

pub(crate) fn default_asset_library_dir() -> String {
    home_subdir("Documents/Kagari/Assets")
}

pub(crate) fn default_accent_color() -> String {
    "#f39c29".to_owned()
}

pub(crate) fn parse_hex_color(value: &str) -> Option<vello::peniko::Color> {
    let hex = value.trim().trim_start_matches('#');
    if hex.len() != 6 || !hex.chars().all(|character| character.is_ascii_hexdigit()) {
        return None;
    }
    let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(vello::peniko::Color::from_rgb8(red, green, blue))
}

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

pub(crate) fn autosave_snapshot_prefix(path: &std::path::Path) -> String {
    let stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".to_owned());
    let identity_path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let identity = identity_path
        .as_os_str()
        .to_string_lossy()
        .bytes()
        .fold(0xcbf29ce484222325u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
        });
    format!("{stem}-{identity:016x}")
}

pub(crate) fn autosave_snapshot_name(path: &std::path::Path) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};

    static SNAPSHOT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    let prefix = autosave_snapshot_prefix(path);
    let extension = path
        .extension()
        .map(|extension| extension.to_string_lossy().to_string())
        .unwrap_or_else(|| "json".to_owned());
    #[cfg(unix)]
    {
        let Ok(duration) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        else {
            return format!(
                "{prefix}-{}-{}.{}",
                std::process::id(),
                SNAPSHOT_SEQUENCE.fetch_add(1, Ordering::Relaxed),
                extension
            );
        };
        let seconds = duration.as_secs() as libc::time_t;
        let mut local: libc::tm = unsafe { std::mem::zeroed() };
        if unsafe { libc::localtime_r(&seconds, &mut local) }.is_null() {
            return format!(
                "{prefix}-{}-{}-{}.{}",
                duration.as_millis(),
                std::process::id(),
                SNAPSHOT_SEQUENCE.fetch_add(1, Ordering::Relaxed),
                extension
            );
        }
        format!(
            "{prefix}-{:04}{:02}{:02}-{:02}{:02}{:02}-{:03}-{}-{}.{}",
            local.tm_year + 1900,
            local.tm_mon + 1,
            local.tm_mday,
            local.tm_hour,
            local.tm_min,
            local.tm_sec,
            duration.subsec_millis(),
            std::process::id(),
            SNAPSHOT_SEQUENCE.fetch_add(1, Ordering::Relaxed),
            extension
        )
    }
    #[cfg(not(unix))]
    {
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or(0);
        format!(
            "{prefix}-{millis:013}-{}-{}.{}",
            std::process::id(),
            SNAPSHOT_SEQUENCE.fetch_add(1, Ordering::Relaxed),
            extension
        )
    }
}

pub(crate) fn prune_backup_files(
    directory: &std::path::Path,
    snapshot_prefix: &str,
    extension: &str,
    generations: u64,
) -> Result<(), String> {
    let mut snapshots = Vec::new();
    let snapshot_prefix = format!("{snapshot_prefix}-");
    let snapshot_suffix = format!(".{extension}");
    for entry in std::fs::read_dir(directory).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        let metadata = match std::fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_file() => metadata,
            Ok(_) => continue,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.to_string()),
        };
        let modified = metadata
            .modified()
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let is_timestamped_backup = name
            .strip_prefix(&snapshot_prefix)
            .and_then(|name| name.strip_suffix(&snapshot_suffix))
            .is_some_and(is_snapshot_generation_suffix);
        if is_timestamped_backup {
            snapshots.push((modified, name.to_owned(), path));
        }
    }
    snapshots.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    let keep = generations.max(1) as usize;
    let excess = snapshots.len().saturating_sub(keep);
    if excess > 0 {
        for (_, _, path) in snapshots.into_iter().take(excess) {
            match std::fs::remove_file(path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.to_string()),
            }
        }
    }
    Ok(())
}

fn is_snapshot_generation_suffix(value: &str) -> bool {
    let parts = value.split('-').collect::<Vec<_>>();
    let digits = |part: &str| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());
    if parts.iter().all(|part| digits(part)) {
        return parts.len() == 1 || parts.len() == 3;
    }
    matches!(parts.as_slice(), [date, time, millis] if date.len() == 8 && digits(date) && time.len() == 6 && digits(time) && millis.len() == 3 && digits(millis))
        || matches!(parts.as_slice(), [date, time, millis, pid, sequence] if date.len() == 8 && digits(date) && time.len() == 6 && digits(time) && millis.len() == 3 && digits(millis) && digits(pid) && digits(sequence))
}

pub(crate) fn pick_directory(current: &str) -> Option<String> {
    let initial = expand_home_path(current.trim());
    let initial = if initial.is_dir() {
        initial
    } else {
        std::env::var_os("HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
    };
    rfd::FileDialog::new()
        .set_directory(initial)
        .pick_folder()
        .map(|path| path.display().to_string())
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
    match read_preferences_from_disk() {
        Ok(preferences) => preferences,
        Err(error) => {
            log::warn!("Could not load preferences: {error}");
            serde_json::Value::Object(serde_json::Map::new())
        }
    }
}

fn read_preferences_from_disk() -> Result<serde_json::Value, String> {
    const MAX_PREFERENCES_BYTES: usize = 1024 * 1024;
    use std::io::Read;

    let primary = preferences_path();
    let mut paths = vec![primary.clone()];
    if let Some(home) = std::env::var_os("HOME") {
        let legacy = std::path::PathBuf::from(home).join(".kagari_prefs.json");
        if legacy != primary {
            paths.push(legacy);
        }
    }

    for path in paths {
        let file = match std::fs::File::open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(format!("Could not read {}: {error}", path.display())),
        };
        if file
            .metadata()
            .map_err(|error| format!("Could not inspect {}: {error}", path.display()))?
            .len()
            > MAX_PREFERENCES_BYTES as u64
        {
            return Err(format!(
                "Preferences file {} exceeds the {MAX_PREFERENCES_BYTES} byte limit",
                path.display()
            ));
        }
        let mut text = String::new();
        file.take(MAX_PREFERENCES_BYTES.saturating_add(1) as u64)
            .read_to_string(&mut text)
            .map_err(|error| format!("Could not read {}: {error}", path.display()))?;
        if text.len() > MAX_PREFERENCES_BYTES {
            return Err(format!(
                "Preferences file {} exceeds the {MAX_PREFERENCES_BYTES} byte limit",
                path.display()
            ));
        }
        let serde_json::Value::Object(preferences) = serde_json::from_str(&text)
            .map_err(|error| format!("Could not parse {}: {error}", path.display()))?
        else {
            return Err(format!(
                "Preferences file {} is not a JSON object",
                path.display()
            ));
        };
        return Ok(serde_json::Value::Object(preferences));
    }

    Ok(serde_json::Value::Object(serde_json::Map::new()))
}

pub(crate) fn write_preference(key: &str, value: serde_json::Value) -> Result<(), String> {
    write_preferences([(key, value)])
}

pub(crate) fn write_preferences<'a>(
    values: impl IntoIterator<Item = (&'a str, serde_json::Value)>,
) -> Result<(), String> {
    let path = preferences_path();
    let mut prefs = read_preferences_from_disk()?
        .as_object()
        .cloned()
        .unwrap_or_default();
    for (key, value) in values {
        prefs.insert(key.to_owned(), value);
    }
    let serialized = serde_json::to_vec_pretty(&serde_json::Value::Object(prefs))
        .map_err(|error| error.to_string())?;
    let parent = path.parent().ok_or("Preferences path has no parent")?;
    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    kagari_vfx::core::atomic_file::write_atomic(&path, &serialized)
        .map_err(|error| error.to_string())
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
        let reader = image::ImageReader::open(&media_path)
            .ok()?
            .with_guessed_format()
            .ok()?;
        let dimensions = reader.into_dimensions().ok()?;
        let decoded_bytes = u64::from(dimensions.0)
            .checked_mul(u64::from(dimensions.1))?
            .checked_mul(4)?;
        const MAX_RECENT_PREVIEW_SOURCE_BYTES: u64 = 256 * 1024 * 1024;
        if dimensions.0 == 0
            || dimensions.1 == 0
            || dimensions.0 > kagari_vfx::core::software_renderer::MAX_RENDER_DIMENSION
            || dimensions.1 > kagari_vfx::core::software_renderer::MAX_RENDER_DIMENSION
            || decoded_bytes > MAX_RECENT_PREVIEW_SOURCE_BYTES
        {
            return None;
        }
        let mut reader = image::ImageReader::open(&media_path)
            .ok()?
            .with_guessed_format()
            .ok()?;
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(kagari_vfx::core::software_renderer::MAX_RENDER_DIMENSION);
        limits.max_image_height = Some(kagari_vfx::core::software_renderer::MAX_RENDER_DIMENSION);
        limits.max_alloc = Some(MAX_RECENT_PREVIEW_SOURCE_BYTES);
        reader.limits(limits);
        let image = reader.decode().ok()?;
        ((image.width(), image.height()) == dimensions).then_some(image)
    });

    let fallback = [
        include_bytes!("../../../assets/home/recent_project_eclipse.webp").as_slice(),
        include_bytes!("../../../assets/home/recent_project_citadel.webp").as_slice(),
        include_bytes!("../../../assets/home/recent_project_rift.webp").as_slice(),
        include_bytes!("../../../assets/home/recent_project_atlas.webp").as_slice(),
    ];
    if let Some(image) = candidate {
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
    (AssetMime::Webp, fallback[index % fallback.len()].to_vec())
}
