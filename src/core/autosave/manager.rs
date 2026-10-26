use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use super::snapshot_writer::{is_recoverable_project, write_snapshot_file};
use super::types::{AutosaveSnapshot, ProductionDocumentMetadata};
use crate::core::production_document::ProductionDocument;
use crate::core::timeline::Project;

/// Number of rotating recovery files kept on disk.
pub const MAX_AUTOSAVE_SLOTS: usize = 5;
/// Maximum retention exposed by Preferences.
pub const MAX_RECOVERY_SLOTS: usize = 50;

/// One recoverable snapshot generation, listed newest-first by the picker.
#[derive(Debug, Clone)]
pub struct RecoveryGeneration {
    pub path: PathBuf,
    /// Unix seconds of the snapshot write (metadata may be missing on old files).
    pub saved_at_secs: u64,
    /// Short provenance label, e.g. "autosave" or "manual".
    pub label: String,
}

impl AutosaveManager {
    /// Fetch the version-history list on a background thread instead of the
    /// caller's thread, so snapshot JSON parsing does not stall the UI.
    #[allow(clippy::type_complexity)]
    pub fn fetch_generations_async(
        &self,
    ) -> (
        std::sync::mpsc::Sender<Vec<RecoveryGeneration>>,
        std::sync::mpsc::Receiver<Vec<RecoveryGeneration>>,
        Option<std::thread::JoinHandle<()>>,
    ) {
        let (tx, rx) = std::sync::mpsc::channel();
        let tx_clone = tx.clone();
        let paths = self.recovery_paths();
        let handle = std::thread::Builder::new()
            .name("kagari-autosave-list".into())
            .spawn(move || {
                let generations = Self::list_generations_from_paths(&paths);
                let _ = tx_clone.send(generations);
            })
            .ok();
        (tx, rx, handle)
    }
}

pub struct AutosaveManager {
    /// Directory where recovery files are written.
    recovery_dir: PathBuf,
    /// Base file stem, e.g. "recovery" -> recovery_0.json .. recovery_4.json
    stem: String,
    /// How often to save while there are unsaved changes.
    interval: Duration,
    /// Last time an autosave was written.
    last_save: Instant,
    /// Round-robin slot for the next write.
    pub(super) next_slot: usize,
    /// Number of recovery generations retained across sessions.
    max_slots: usize,
    /// Set true by `mark_dirty` when the project changes.
    dirty: bool,
    pub(super) retry_at: Option<Instant>,
    last_error: Option<String>,
    loaded_recovery_path: Mutex<Option<PathBuf>>,
    dirty_generation: u64,
    async_write: Option<std::sync::mpsc::Receiver<(u64, std::io::Result<PathBuf>)>>,
    /// Join handle for the in-flight async write worker, so `wait_for_pending`
    /// can join it instead of leaving the thread detached.
    join_handle: Mutex<Option<std::thread::JoinHandle<()>>>,
}

/// Read and validate one recovery snapshot file **without touching manager
/// state**, so callers can parse on a background thread while the UI keeps
/// running. Returns `(project, production_document)` for both the
/// `AutosaveSnapshot` format and the legacy bare-`Project` format; `None`
/// for corrupt or non-recoverable files.
pub fn load_snapshot_file(path: &Path) -> Option<(Project, Option<ProductionDocument>)> {
    let json = crate::core::project_migration::read_bounded_text_file(
        path,
        crate::core::project_migration::MAX_PROJECT_JSON_BYTES,
    )
    .ok()?;
    // Preferred format: AutosaveSnapshot (project + optional production doc
    // + generation metadata). Fall back to bare Project JSON for old files.
    if let Ok(snapshot) = serde_json::from_str::<AutosaveSnapshot>(&json) {
        if !is_recoverable_project(&snapshot.project) {
            log::warn!("[Autosave] Skipping invalid project {:?}", path);
            return None;
        }
        let document = match snapshot.production_document {
            Some(metadata) => {
                let doc = metadata.into_document(snapshot.project.clone());
                if doc.validate().is_ok() {
                    Some(doc)
                } else {
                    return None;
                }
            }
            None => None,
        };
        return Some((snapshot.project, document));
    }
    match serde_json::from_str::<Project>(&json) {
        Ok(project) if is_recoverable_project(&project) => Some((project, None)),
        _ => {
            log::warn!("[Autosave] Skipping corrupt snapshot {:?}", path);
            None
        }
    }
}

impl AutosaveManager {
    pub fn new(recovery_dir: impl Into<PathBuf>) -> Self {
        Self {
            recovery_dir: recovery_dir.into(),
            stem: "recovery".to_string(),
            interval: Duration::from_secs(30),
            last_save: Instant::now(),
            next_slot: 0,
            max_slots: MAX_AUTOSAVE_SLOTS,
            dirty: false,
            retry_at: None,
            last_error: None,
            loaded_recovery_path: Mutex::new(None),
            dirty_generation: 0,
            async_write: None,
            join_handle: Mutex::new(None),
        }
    }

    pub fn with_process_namespace(mut self, process_id: u32) -> Self {
        self.stem = format!("recovery_{process_id}");
        self
    }

    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// Runtime interval adjustment (seconds), used by the Preferences dialog.
    pub fn set_interval_secs(&mut self, secs: u64) {
        self.interval = Duration::from_secs(secs.clamp(5, 3600));
    }

    /// Sets the retained recovery-generation count and removes older files
    /// beyond the newly selected limit.
    pub fn set_max_slots(&mut self, slots: usize) {
        self.max_slots = slots.clamp(1, MAX_RECOVERY_SLOTS);
        self.next_slot %= self.max_slots;
        let paths = self.all_recovery_paths();
        for path in paths.into_iter().skip(self.max_slots) {
            let _ = std::fs::remove_file(path);
        }
    }

    pub fn max_slots(&self) -> usize {
        self.max_slots
    }

    /// Current autosave interval in seconds.
    pub fn interval_secs(&self) -> u64 {
        self.interval.as_secs()
    }

    /// Call whenever the project is modified.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
        self.dirty_generation = self.dirty_generation.wrapping_add(1);
    }

    pub fn has_unsaved_snapshot(&self) -> bool {
        self.dirty
    }

    pub fn take_last_error(&mut self) -> Option<String> {
        self.last_error.take()
    }

    /// Returns the path of the recovery file for a slot.
    pub(super) fn slot_path(&self, slot: usize) -> PathBuf {
        self.recovery_dir
            .join(format!("{}_{}.json", self.stem, slot))
    }

    fn all_recovery_paths(&self) -> Vec<PathBuf> {
        let mut paths = std::fs::read_dir(&self.recovery_dir)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("recovery_") && name.ends_with(".json"))
            })
            .collect::<Vec<_>>();
        paths.sort_by_key(|path| {
            std::fs::metadata(path)
                .and_then(|meta| meta.modified())
                .unwrap_or(std::time::UNIX_EPOCH)
        });
        paths.reverse();
        paths
    }

    fn recovery_paths(&self) -> Vec<PathBuf> {
        self.all_recovery_paths()
            .into_iter()
            .take(self.max_slots)
            .collect()
    }

    /// Writes a recovery snapshot if the dirty flag is set and the interval elapsed.
    /// Returns the path written, if any.
    pub fn tick(&mut self, project: &Project) -> Option<PathBuf> {
        if !self.dirty
            || self.last_save.elapsed() < self.interval
            || self
                .retry_at
                .is_some_and(|retry_at| Instant::now() < retry_at)
        {
            return None;
        }
        let path = self.write_snapshot(project, None, "autosave");
        match &path {
            Ok(_) => {
                self.dirty = false;
                self.last_save = Instant::now();
                self.retry_at = None;
                self.last_error = None;
            }
            Err(error) => {
                self.retry_at = Some(Instant::now() + Duration::from_secs(30));
                self.last_error = Some(error.to_string());
            }
        }
        path.ok()
    }

    /// Forces an immediate snapshot (e.g. on app exit or before risky operations).
    pub fn save_now(&mut self, project: &Project) -> std::io::Result<PathBuf> {
        let _ = self.wait_for_pending();
        let path = self.write_snapshot(project, None, "manual")?;
        self.dirty = false;
        self.last_save = Instant::now();
        self.retry_at = None;
        self.last_error = None;
        Ok(path)
    }

    pub fn save_now_production(
        &mut self,
        project: &Project,
        document: &ProductionDocument,
    ) -> std::io::Result<PathBuf> {
        let _ = self.wait_for_pending();
        let path = self.write_snapshot(project, Some(document), "manual")?;
        self.dirty = false;
        self.last_save = Instant::now();
        self.retry_at = None;
        self.last_error = None;
        Ok(path)
    }

    pub fn tick_production(
        &mut self,
        project: &Project,
        document: &ProductionDocument,
    ) -> Option<PathBuf> {
        if !self.dirty
            || self.last_save.elapsed() < self.interval
            || self
                .retry_at
                .is_some_and(|retry_at| Instant::now() < retry_at)
        {
            return None;
        }
        let path = self.write_snapshot(project, Some(document), "autosave");
        match &path {
            Ok(_) => {
                self.dirty = false;
                self.last_save = Instant::now();
                self.retry_at = None;
                self.last_error = None;
            }
            Err(error) => {
                self.retry_at = Some(Instant::now() + Duration::from_secs(30));
                self.last_error = Some(error.to_string());
            }
        }
        path.ok()
    }

    /// Starts a recovery snapshot without serializing or writing on the caller
    /// thread. Project cloning remains necessary to hand an immutable snapshot
    /// to the worker, but the ProductionDocument metadata reuses that snapshot.
    pub fn tick_async(
        &mut self,
        project: &Project,
        production_document: Option<&ProductionDocument>,
    ) -> Option<PathBuf> {
        let completed_path = self.poll_async_write();
        if self.async_write.is_some()
            || !self.dirty
            || self.last_save.elapsed() < self.interval
            || self
                .retry_at
                .is_some_and(|retry_at| Instant::now() < retry_at)
        {
            return completed_path;
        }

        let project = project.clone();
        let metadata = production_document.map(ProductionDocumentMetadata::from);
        let generation = self.dirty_generation;
        let path = self.slot_path(self.next_slot % self.max_slots);
        self.next_slot = (self.next_slot + 1) % self.max_slots;
        let recovery_dir = self.recovery_dir.clone();
        let (sender, receiver) = std::sync::mpsc::channel();
        let worker = std::thread::Builder::new()
            .name("kagari-autosave".into())
            .spawn(move || {
                let result = write_snapshot_file(
                    &recovery_dir,
                    &path,
                    &project,
                    metadata.as_ref(),
                    "autosave",
                );
                let _ = sender.send((generation, result));
            });
        match worker {
            Ok(handle) => {
                *self
                    .join_handle
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(handle);
                self.async_write = Some(receiver);
            }
            Err(error) => {
                self.retry_at = Some(Instant::now() + Duration::from_secs(30));
                self.last_error = Some(format!("could not start autosave worker: {error}"));
            }
        }
        completed_path
    }

    /// Waits for an in-flight autosave, primarily for orderly application exit.
    pub fn wait_for_pending(&mut self) -> Option<PathBuf> {
        let receiver = self.async_write.take()?;
        let result = receiver.recv();
        // Join the worker thread after receiving its result so the autosave
        // thread does not outlive the process as a detached thread.
        if let Some(handle) = self
            .join_handle
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
        {
            let _ = handle.join();
        }
        match result {
            Ok((generation, Ok(path))) => {
                self.last_save = Instant::now();
                self.retry_at = None;
                self.last_error = None;
                if self.dirty_generation == generation {
                    self.dirty = false;
                }
                Some(path)
            }
            Ok((_, Err(error))) => {
                self.retry_at = Some(Instant::now() + Duration::from_secs(30));
                self.last_error = Some(error.to_string());
                None
            }
            Err(error) => {
                self.retry_at = Some(Instant::now() + Duration::from_secs(30));
                self.last_error = Some(format!("autosave worker stopped: {error}"));
                None
            }
        }
    }

    fn poll_async_write(&mut self) -> Option<PathBuf> {
        let outcome = self.async_write.as_ref()?.try_recv();
        match outcome {
            Err(std::sync::mpsc::TryRecvError::Empty) => None,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.async_write = None;
                self.retry_at = Some(Instant::now() + Duration::from_secs(30));
                self.last_error = Some("autosave worker stopped before returning a result".into());
                None
            }
            Ok((generation, result)) => {
                self.async_write = None;
                match result {
                    Ok(path) => {
                        self.last_save = Instant::now();
                        self.retry_at = None;
                        self.last_error = None;
                        if self.dirty_generation == generation {
                            self.dirty = false;
                        }
                        Some(path)
                    }
                    Err(error) => {
                        self.retry_at = Some(Instant::now() + Duration::from_secs(30));
                        self.last_error = Some(error.to_string());
                        None
                    }
                }
            }
        }
    }

    /// Writes to the next rotating slot, tolerating individual write failures.
    fn write_snapshot(
        &mut self,
        project: &Project,
        production_document: Option<&ProductionDocument>,
        label: &str,
    ) -> std::io::Result<PathBuf> {
        let path = self.slot_path(self.next_slot % self.max_slots);
        self.next_slot = (self.next_slot + 1) % self.max_slots;
        let metadata = production_document.map(ProductionDocumentMetadata::from);
        write_snapshot_file(&self.recovery_dir, &path, project, metadata.as_ref(), label)
    }

    /// Loads the newest valid recovery snapshot, if one exists.
    /// Corrupt/truncated slots are skipped automatically.
    pub fn load_latest_snapshot(&self) -> Option<(Project, Option<ProductionDocument>)> {
        for path in self.recovery_paths() {
            if let Ok(json) = crate::core::project_migration::read_bounded_text_file(
                &path,
                crate::core::project_migration::MAX_PROJECT_JSON_BYTES,
            ) {
                if let Ok(snapshot) = serde_json::from_str::<AutosaveSnapshot>(&json) {
                    if is_recoverable_project(&snapshot.project) {
                        let project = snapshot.project;
                        let production_document = snapshot
                            .production_document
                            .map(|metadata| metadata.into_document(project.clone()))
                            .filter(|document| document.validate().is_ok());
                        *self
                            .loaded_recovery_path
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(path);
                        return Some((project, production_document));
                    }
                    log::warn!(
                        "[Autosave] Skipping invalid project recovery file {:?}",
                        path
                    );
                }
                if let Ok(project) = serde_json::from_str::<Project>(&json) {
                    if is_recoverable_project(&project) {
                        *self
                            .loaded_recovery_path
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(path);
                        return Some((project, None));
                    }
                    log::warn!(
                        "[Autosave] Skipping invalid project recovery file {:?}",
                        path
                    );
                }
                log::warn!("[Autosave] Skipping corrupt recovery file {:?}", path);
            }
        }
        None
    }

    pub fn load_latest_recovery(&self) -> Option<Project> {
        self.load_latest_snapshot().map(|(project, _)| project)
    }

    pub fn load_latest_production(&self) -> Option<ProductionDocument> {
        for path in self.recovery_paths() {
            if let Ok(json) = crate::core::project_migration::read_bounded_text_file(
                &path,
                crate::core::project_migration::MAX_PROJECT_JSON_BYTES,
            ) {
                if let Ok(snapshot) = serde_json::from_str::<AutosaveSnapshot>(&json) {
                    if let Some(metadata) = snapshot.production_document {
                        let document = metadata.into_document(snapshot.project);
                        if document.validate().is_ok() && is_recoverable_project(&document.project)
                        {
                            return Some(document);
                        }
                        log::warn!("[Autosave] Skipping invalid production document {:?}", path);
                    }
                }
            }
        }
        None
    }

    /// True if at least one recovery snapshot exists on disk.
    pub fn has_recovery(&self) -> bool {
        !self.recovery_paths().is_empty()
    }

    /// Lists every recoverable snapshot newest-first with its generation
    /// metadata (save time + label), for the version-history picker.
    pub fn list_generations(&self) -> Vec<RecoveryGeneration> {
        Self::list_generations_from_paths(&self.recovery_paths())
    }

    /// Internal helper that parses snapshots from explicit paths off-thread.
    pub fn list_generations_from_paths(paths: &[PathBuf]) -> Vec<RecoveryGeneration> {
        let mut generations: Vec<RecoveryGeneration> = paths
            .iter()
            .filter_map(|path| {
                let modified = std::fs::metadata(path)
                    .ok()
                    .and_then(|meta| meta.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                // Snapshots are written as AutosaveSnapshot; older files may be
                // bare Project JSON. Corrupt slots are skipped in both cases.
                let json = crate::core::project_migration::read_bounded_text_file(
                    path,
                    crate::core::project_migration::MAX_PROJECT_JSON_BYTES,
                )
                .ok()?;
                let (saved_at_secs, label) =
                    if let Ok(snapshot) = serde_json::from_str::<AutosaveSnapshot>(&json) {
                        if !is_recoverable_project(&snapshot.project) {
                            return None;
                        }
                        (snapshot.saved_at_secs, snapshot.label)
                    } else if serde_json::from_str::<Project>(&json)
                        .map(|project| is_recoverable_project(&project))
                        .unwrap_or(false)
                    {
                        (modified, String::new())
                    } else {
                        return None;
                    };
                Some(RecoveryGeneration {
                    path: path.clone(),
                    saved_at_secs: saved_at_secs.max(modified),
                    label,
                })
            })
            .collect();
        generations.sort_by_key(|generation| std::cmp::Reverse(generation.saved_at_secs));
        generations
    }

    /// Loads the snapshot at an explicit path chosen from `list_generations`.
    pub fn load_snapshot_at(&self, path: &Path) -> Option<(Project, Option<ProductionDocument>)> {
        let loaded = load_snapshot_file(path);
        if loaded.is_some() {
            self.note_loaded_recovery(path);
        }
        loaded
    }

    /// Remember which snapshot file a restore came from. `clear_recovery` uses
    /// this to target the matching recovery namespace, and it must be called
    /// after a background [`load_snapshot_file`] succeeds (the stateless
    /// loader cannot record it itself).
    pub fn note_loaded_recovery(&self, path: &Path) {
        *self
            .loaded_recovery_path
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(path.to_path_buf());
    }

    pub fn latest_recovery_modified(&self) -> Option<std::time::SystemTime> {
        self.recovery_paths()
            .first()
            .and_then(|path| std::fs::metadata(path).ok()?.modified().ok())
    }

    /// Removes all recovery files (e.g. after a clean, successful save).
    pub fn clear_recovery(&self) {
        let loaded_path = self
            .loaded_recovery_path
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        if let Some(path) = loaded_path {
            let namespace = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .and_then(|stem| stem.rsplit_once('_').map(|(prefix, _)| prefix.to_owned()));
            if let Some(namespace) = namespace {
                if let Ok(entries) = std::fs::read_dir(&self.recovery_dir) {
                    for entry in entries.flatten() {
                        let candidate = entry.path();
                        let belongs_to_snapshot = candidate
                            .file_name()
                            .and_then(|name| name.to_str())
                            .is_some_and(|name| {
                                name.starts_with(&format!("{namespace}_"))
                                    && name.ends_with(".json")
                            });
                        if belongs_to_snapshot {
                            let _ = std::fs::remove_file(candidate);
                        }
                    }
                }
            } else {
                let _ = std::fs::remove_file(path);
            }
        } else {
            for i in 0..self.max_slots {
                let _ = std::fs::remove_file(self.slot_path(i));
            }
        }
    }
}
