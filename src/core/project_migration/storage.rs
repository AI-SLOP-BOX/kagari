use super::schema::{load_project_migrated, save_project_versioned, MAX_PROJECT_JSON_BYTES};
use crate::core::production_document::ProductionDocument;
use crate::core::timeline::Project;

fn resolve_project_backup_base(path: &std::path::Path) -> std::path::PathBuf {
    if let Ok(canonical) = std::fs::canonicalize(path) {
        return canonical;
    }
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return path.to_path_buf();
    };
    if !metadata.file_type().is_symlink() {
        return path.to_path_buf();
    }
    let Ok(link_target) = std::fs::read_link(path) else {
        return path.to_path_buf();
    };
    let candidate = if link_target.is_absolute() {
        link_target
    } else {
        path.parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join(link_target)
    };
    if let Ok(canonical) = std::fs::canonicalize(&candidate) {
        return canonical;
    }
    let (Some(parent), Some(file_name)) = (candidate.parent(), candidate.file_name()) else {
        return candidate;
    };
    std::fs::canonicalize(parent)
        .map(|parent| parent.join(file_name))
        .unwrap_or(candidate)
}

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

    let mut owns_temporary = false;
    let result = (|| {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut staged = options
            .open(&temporary)
            .map_err(|error| format!("Failed to create staged project backup: {error}"))?;
        owns_temporary = true;
        let mut original = std::fs::File::open(source)
            .map_err(|error| format!("Failed to open project for backup: {error}"))?;
        let permissions = original
            .metadata()
            .map_err(|error| format!("Failed to inspect project permissions: {error}"))?
            .permissions();
        std::io::copy(&mut original, &mut staged)
            .map_err(|error| format!("Failed to stage project backup: {error}"))?;
        // Keep the staging file private until the complete backup is present.
        staged
            .set_permissions(permissions)
            .map_err(|error| format!("Failed to preserve project backup permissions: {error}"))?;
        staged
            .sync_all()
            .map_err(|error| format!("Failed to sync staged project backup: {error}"))?;
        drop(staged);
        crate::core::ffmpeg_export::types::replace_existing_file(&temporary, backup)
            .map_err(|error| format!("Failed to atomically replace project backup: {error}"))?;
        owns_temporary = false;
        Ok(())
    })();

    if result.is_err() && owns_temporary {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

/// Save a project file atomically using temporary file creation and OS atomic replacement.
pub fn save_project_atomic<P: AsRef<std::path::Path>>(
    proj: &Project,
    path: P,
) -> Result<(), String> {
    save_project_atomic_from(proj, None, path)
}

/// Save a project while carrying document-level data forward from its source.
/// This is used by Save As and external autosave destinations, where the target
/// file cannot provide the open document's audio, tempo, and automation data.
pub fn save_project_atomic_from<P: AsRef<std::path::Path>>(
    proj: &Project,
    source_path: Option<&std::path::Path>,
    path: P,
) -> Result<(), String> {
    use std::sync::{Mutex, OnceLock};

    static SAVE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    let _save_guard = SAVE_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let requested_path = path.as_ref();
    let target_path = match std::fs::symlink_metadata(requested_path) {
        Ok(_) => std::fs::canonicalize(requested_path)
            .map_err(|error| format!("Failed to resolve project save path: {error}"))?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => requested_path.to_path_buf(),
        Err(error) => return Err(format!("Failed to inspect project save path: {error}")),
    };
    let bak_path = target_path.with_extension("json.bak");

    let current_json = read_project_json_file(&target_path).ok();
    let current_document = current_json
        .as_deref()
        .and_then(|json| ProductionDocument::from_json(json).ok());
    let current_file_is_valid = current_document.is_some()
        || current_json
            .as_deref()
            .is_some_and(|json| load_project_migrated(json).is_ok());
    let source_document = source_path
        .filter(|source| *source != target_path.as_path())
        .and_then(|source| read_project_json_file(source).ok())
        .and_then(|json| ProductionDocument::from_json(&json).ok());
    let backup_document = if current_file_is_valid {
        None
    } else {
        read_project_json_file(&bak_path)
            .ok()
            .and_then(|json| ProductionDocument::from_json(&json).ok())
    };
    if current_file_is_valid {
        replace_backup_atomically(&target_path, &bak_path)?;
    }

    let json_str =
        if let Some(mut document) = source_document.or(current_document).or(backup_document) {
            document.project = proj.clone();
            document.to_json()?
        } else {
            save_project_versioned(proj)?
        };

    crate::core::atomic_file::write_atomic(&target_path, json_str.as_bytes())
        .map_err(|error| format!("Failed to atomically replace project file: {error}"))?;

    Ok(())
}

/// Loads `<path>` or, if missing/corrupt, falls back to `<path>.bak`.
pub fn load_project_with_backup<P: AsRef<std::path::Path>>(
    path: P,
) -> Result<(Project, bool), String> {
    let target = path.as_ref();
    // Saves resolve existing symlinked destinations before placing the backup.
    // Resolve the primary here too so recovery follows the same location.
    let backup_base = resolve_project_backup_base(target);
    let bak = backup_base.with_extension("json.bak");

    if let Ok(json) = read_project_json_file(target) {
        match load_project_document(&json) {
            Ok(p) => return Ok((p, false)),
            Err(e) => log::warn!("[Project] Primary file corrupt ({}); trying backup", e),
        }
    }
    if let Ok(json) = read_project_json_file(&bak) {
        match load_project_document(&json) {
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

fn load_project_document(json: &str) -> Result<Project, String> {
    load_project_document_or_legacy(json).map(|(project, _)| project)
}

/// Load a production document without silently downgrading it to a legacy
/// project when its production-only data fails validation.
pub fn load_project_document_or_legacy(
    json: &str,
) -> Result<(Project, Option<ProductionDocument>), String> {
    let root: serde_json::Value = serde_json::from_str(json)
        .map_err(|error| format!("Invalid JSON project file: {error}"))?;
    let is_production_document = ["project", "audio", "tempo", "bindings", "color_lut_path"]
        .iter()
        .any(|field| root.get(field).is_some());
    if is_production_document {
        let document = ProductionDocument::from_json(json)
            .map_err(|error| format!("Invalid production document: {error}"))?;
        return Ok((document.project.clone(), Some(document)));
    }
    load_project_migrated(json).map(|project| (project, None))
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
