use super::schema::{load_project_migrated, save_project_versioned, MAX_PROJECT_JSON_BYTES};
use crate::core::timeline::Project;
use std::io::Write;

fn replace_backup_atomically(
    source: &std::path::Path,
    backup: &std::path::Path,
) -> Result<(), String> {
    use std::sync::atomic::{AtomicU64, Ordering};

    static BACKUP_COUNTER: AtomicU64 = AtomicU64::new(0);
    let file_name = backup
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or("project.json.bak");
    let temporary = backup.with_file_name(format!(
        ".{file_name}.tmp.{}.{}",
        std::process::id(),
        BACKUP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));

    let result = (|| {
        std::fs::copy(source, &temporary)
            .map_err(|error| format!("Failed to stage project backup: {error}"))?;
        std::fs::File::open(&temporary)
            .and_then(|file| file.sync_all())
            .map_err(|error| format!("Failed to sync staged project backup: {error}"))?;
        std::fs::rename(&temporary, backup)
            .map_err(|error| format!("Failed to atomically replace project backup: {error}"))
    })();

    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

/// Save a project file atomically using temporary file creation and OS atomic replacement.
pub fn save_project_atomic<P: AsRef<std::path::Path>>(
    proj: &Project,
    path: P,
) -> Result<(), String> {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Mutex, OnceLock};

    static SAVE_COUNTER: AtomicU64 = AtomicU64::new(0);
    static SAVE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    let _save_guard = SAVE_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let json_str = save_project_versioned(proj)?;
    let target_path = path.as_ref();
    let temp_suffix = format!(
        "json.tmp.{}.{}",
        std::process::id(),
        SAVE_COUNTER.fetch_add(1, Ordering::Relaxed)
    );
    let tmp_path = target_path.with_extension(temp_suffix);
    let bak_path = target_path.with_extension("json.bak");

    if target_path.exists() {
        replace_backup_atomically(target_path, &bak_path)?;
    }

    let write_result = (|| {
        let mut tmp_file = std::fs::File::create(&tmp_path)
            .map_err(|e| format!("Failed to create temporary project file: {}", e))?;
        tmp_file
            .write_all(json_str.as_bytes())
            .map_err(|e| format!("Failed to write temporary project file: {}", e))?;
        tmp_file
            .sync_all()
            .map_err(|e| format!("Failed to sync temporary project file: {}", e))?;
        drop(tmp_file);

        std::fs::rename(&tmp_path, target_path)
            .map_err(|e| format!("Failed to atomically replace project file: {}", e))
    })();

    if write_result.is_err() {
        let _ = std::fs::remove_file(&tmp_path);
    }
    write_result?;

    if let Some(parent) = target_path.parent() {
        if let Ok(dir) = std::fs::File::open(parent) {
            let _ = dir.sync_all();
        }
    }

    Ok(())
}

/// Loads `<path>` or, if missing/corrupt, falls back to `<path>.bak`.
pub fn load_project_with_backup<P: AsRef<std::path::Path>>(
    path: P,
) -> Result<(Project, bool), String> {
    let target = path.as_ref();
    let bak = target.with_extension("json.bak");

    if let Ok(json) = read_project_json_file(target) {
        match load_project_migrated(&json) {
            Ok(p) => return Ok((p, false)),
            Err(e) => log::warn!("[Project] Primary file corrupt ({}); trying backup", e),
        }
    }
    if let Ok(json) = read_project_json_file(&bak) {
        match load_project_migrated(&json) {
            Ok(p) => return Ok((p, true)),
            Err(e) => log::warn!("[Project] Backup also corrupt: {}", e),
        }
    }
    Err(format!(
        "Could not load '{}' and no valid backup at '{}'",
        target.display(),
        bak.display()
    ))
}

/// Read a project JSON file only after enforcing size limits.
pub fn read_project_json_file(path: &std::path::Path) -> Result<String, String> {
    read_bounded_text_file(path, MAX_PROJECT_JSON_BYTES)
}

/// Read a text file up to a declared max size in bytes.
pub fn read_bounded_text_file(path: &std::path::Path, max_bytes: usize) -> Result<String, String> {
    use std::io::Read;

    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    if metadata.len() > max_bytes as u64 {
        return Err(format!("File exceeds the {} byte limit", max_bytes));
    }
    let mut text = String::new();
    file.take(max_bytes.saturating_add(1) as u64)
        .read_to_string(&mut text)
        .map_err(|e| e.to_string())?;
    if text.len() > max_bytes {
        return Err(format!("File exceeds the {} byte limit", max_bytes));
    }
    Ok(text)
}
