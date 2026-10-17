//! User preferences reading, writing, path normalization, and recents management.

use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

pub const MAX_PREFS_BYTES: usize = 1024 * 1024;
pub const MAX_STARRED_PROJECTS: usize = 100;
static PREFS_WRITE_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static PREFS_WRITE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub(crate) fn prefs_path() -> std::path::PathBuf {
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
        {
            return std::path::PathBuf::from(home)
                .join(".config")
                .join("kagari")
                .join("prefs.json");
        }
        #[cfg(not(target_os = "linux"))]
        {
            return std::path::PathBuf::from(home).join(".kagari_prefs.json");
        }
    }

    std::env::current_dir()
        .unwrap_or_else(|_| std::env::temp_dir())
        .join(".kagari_prefs.json")
}

fn legacy_prefs_path() -> Option<std::path::PathBuf> {
    let home = std::env::var_os("HOME")?;
    let legacy = std::path::PathBuf::from(home).join(".kagari_prefs.json");
    (legacy != prefs_path()).then_some(legacy)
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct RecentsFile {
    #[serde(default)]
    pub recent_projects: Vec<String>,
    /// Starred project paths. Added later; defaults to empty for old files.
    #[serde(default)]
    pub starred_projects: Vec<String>,
    /// Whether the welcome screen shows on startup. Defaults to true for
    /// prefs files written before this flag existed.
    #[serde(default = "default_welcome_on_startup")]
    pub show_welcome_on_startup: bool,
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

pub fn read_prefs() -> RecentsFile {
    let mut prefs = load_prefs_value()
        .and_then(|value| serde_json::from_value::<RecentsFile>(value).ok())
        .unwrap_or_default();
    prefs.recent_projects = normalize_path_keys(prefs.recent_projects, 8);
    prefs.starred_projects = normalize_path_keys(prefs.starred_projects, MAX_STARRED_PROJECTS);
    prefs
}

pub(crate) fn load_prefs_value() -> Option<serde_json::Value> {
    let mut paths = vec![prefs_path()];
    if let Some(legacy) = legacy_prefs_path() {
        paths.push(legacy);
    }
    for path in paths {
        let Ok(text) =
            crate::core::project_migration::read_bounded_text_file(&path, MAX_PREFS_BYTES)
        else {
            continue;
        };
        match serde_json::from_str(&text) {
            Ok(serde_json::Value::Object(object)) => {
                return Some(serde_json::Value::Object(object));
            }
            Ok(_) | Err(_) => {
                let backup = path.with_file_name(format!(
                    ".kagari_prefs.corrupt.{}.{}.json",
                    std::process::id(),
                    PREFS_WRITE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
                ));
                let _ = std::fs::rename(path, backup);
            }
        }
    }
    None
}

/// Update only the requested preference fields while preserving fields owned
/// by the preferences dialog or newer app versions.
pub(crate) fn update_prefs<F>(update: F) -> Result<(), String>
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
    let json = serde_json::to_vec_pretty(&serde_json::Value::Object(root))
        .map_err(|error| format!("Could not serialize preferences: {error}"))?;
    let sequence = PREFS_WRITE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary = path.with_file_name(format!(
        ".kagari_prefs.json.tmp.{}.{}",
        std::process::id(),
        sequence
    ));
    let result = (|| -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = std::fs::File::create(&temporary)?;
        use std::io::Write;
        file.write_all(&json)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temporary, &path)
    })();
    if let Err(error) = result {
        let _ = std::fs::remove_file(temporary);
        return Err(format!(
            "Could not save preferences to {}: {error}",
            path.display()
        ));
    }
    Ok(())
}

pub(crate) fn path_key(path: &std::path::Path) -> String {
    if path.as_os_str().is_empty() {
        return String::new();
    }
    let normalized = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    if let Some(path) = normalized.to_str() {
        if !path.starts_with("kagari-path-raw-v1:") {
            return path.to_string();
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        format!(
            "kagari-path-raw-v1:u:{}",
            encode_hex(normalized.as_os_str().as_bytes())
        )
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let bytes = normalized
            .as_os_str()
            .encode_wide()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();
        format!("kagari-path-raw-v1:w:{}", encode_hex(&bytes))
    }
    #[cfg(not(any(unix, windows)))]
    normalized.to_string_lossy().into_owned()
}

pub(crate) fn path_from_key(value: &str) -> std::path::PathBuf {
    let Some(encoded) = value.strip_prefix("kagari-path-raw-v1:") else {
        return std::path::PathBuf::from(value);
    };
    let Some((kind, hex)) = encoded.split_once(':') else {
        return std::path::PathBuf::from(value);
    };
    let Some(bytes) = decode_hex(hex) else {
        return std::path::PathBuf::from(value);
    };
    #[cfg(unix)]
    if kind == "u" {
        use std::os::unix::ffi::OsStringExt;
        return std::path::PathBuf::from(std::ffi::OsString::from_vec(bytes));
    }
    #[cfg(windows)]
    if kind == "w" && bytes.len().is_multiple_of(2) {
        use std::os::windows::ffi::OsStringExt;
        let wide = bytes
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>();
        return std::path::PathBuf::from(std::ffi::OsString::from_wide(&wide));
    }
    std::path::PathBuf::from(value)
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len().saturating_mul(2));
    for &byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

fn decode_hex(encoded: &str) -> Option<Vec<u8>> {
    if !encoded.len().is_multiple_of(2) {
        return None;
    }
    fn digit(value: u8) -> Option<u8> {
        match value {
            b'0'..=b'9' => Some(value - b'0'),
            b'a'..=b'f' => Some(value - b'a' + 10),
            b'A'..=b'F' => Some(value - b'A' + 10),
            _ => None,
        }
    }
    encoded
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| Some((digit(pair[0])? << 4) | digit(pair[1])?))
        .collect()
}

pub(crate) fn path_list(
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

pub(crate) fn set_path_list(
    root: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    paths: &[String],
) {
    root.insert(key.to_string(), serde_json::json!(paths));
}

pub(crate) fn normalize_path_keys(paths: Vec<String>, limit: usize) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::with_capacity(paths.len().min(limit));
    for path in paths {
        let native_path = path_from_key(&path);
        let key = path_key(&native_path);
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
pub fn set_welcome_on_startup(show: bool) -> Result<(), String> {
    update_prefs(|root| {
        root.insert(
            "show_welcome_on_startup".into(),
            serde_json::Value::Bool(show),
        );
    })
}

/// Insert path at front of recents (deduped, capped at 8) and persist.
pub fn push_recent(path: &std::path::Path) -> Result<(), String> {
    let s = path_key(path);
    update_prefs(|root| {
        let mut recents = path_list(root, "recent_projects", 8);
        recents.retain(|p| p != &s);
        recents.insert(0, s);
        recents.truncate(8);
        set_path_list(root, "recent_projects", &recents);
    })
}

/// Remove path from recents and persist.
pub fn remove_recent(path: &std::path::Path) -> Result<(), String> {
    let s = path_key(path);
    update_prefs(|root| {
        let mut recents = path_list(root, "recent_projects", 8);
        recents.retain(|p| p != &s);
        set_path_list(root, "recent_projects", &recents);
    })
}

/// Starred project paths shown in Home > Starred.
pub fn starred_projects() -> Vec<String> {
    read_prefs().starred_projects
}

/// Toggle the starred flag for a project path. Returns true if now starred.
pub fn toggle_starred(path: &std::path::Path) -> Result<bool, String> {
    let s = path_key(path);
    let mut now_starred = false;
    update_prefs(|root| {
        let mut stars = path_list(root, "starred_projects", MAX_STARRED_PROJECTS);
        let mut prefs = RecentsFile {
            starred_projects: std::mem::take(&mut stars),
            ..Default::default()
        };
        now_starred = toggle_starred_key(&mut prefs, s);
        set_path_list(root, "starred_projects", &prefs.starred_projects);
    })?;
    Ok(now_starred)
}

pub(crate) fn toggle_starred_key(prefs: &mut RecentsFile, key: String) -> bool {
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
