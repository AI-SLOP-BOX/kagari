use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use super::types::{AutosaveSnapshotRef, ProductionDocumentMetadata};
use crate::core::production_document::ProductionDocument;
use crate::core::timeline::Project;

static AUTOSAVE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(crate) fn write_snapshot_file(
    recovery_dir: &Path,
    path: &Path,
    project: &Project,
    production_document: Option<&ProductionDocumentMetadata>,
) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(recovery_dir)?;
    let sequence = AUTOSAVE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let tmp = path.with_extension(format!("json.{}.{}.tmp", std::process::id(), sequence));
    let result = (|| {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)?;
        let mut file = std::io::BufWriter::new(file);
        serde_json::to_writer(
            &mut file,
            &AutosaveSnapshotRef {
                project,
                production_document,
            },
        )?;
        use std::io::Write;
        file.flush()?;
        let file = file.into_inner().map_err(|error| error.into_error())?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&tmp, path)?;
        Ok(path.to_path_buf())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(tmp);
    }
    result
}

pub(crate) fn is_recoverable_project(project: &Project) -> bool {
    !project.compositions.is_empty() && project.active_composition_idx < project.compositions.len()
}

/// Returns true if the path looks like a valid project JSON (cheap sanity check
/// used by the recovery picker UI).
pub fn is_valid_project_file(path: &Path) -> bool {
    crate::core::project_migration::read_bounded_text_file(
        path,
        crate::core::project_migration::MAX_PROJECT_JSON_BYTES,
    )
    .ok()
    .and_then(|s| {
        ProductionDocument::from_json(&s)
            .map(|_| ())
            .or_else(|_| serde_json::from_str::<Project>(&s).map(|_| ()))
            .ok()
    })
    .is_some()
}
