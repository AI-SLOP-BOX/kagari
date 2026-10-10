use std::time::{Duration, Instant};

use super::manager::MAX_AUTOSAVE_SLOTS;
use super::snapshot_writer::is_valid_project_file;
use super::types::AutosaveSnapshot;
use super::AutosaveManager;
use crate::core::production_document::ProductionDocument;
use crate::core::timeline::{Composition, Layer, LayerType, Project};

fn sample_project() -> Project {
    let mut comp = Composition::new("c1".into(), "AutoSaveComp".into(), 64, 64, 30, 30);
    let layer = Layer::new(
        "l1".into(),
        "Solid".into(),
        LayerType::Solid { color: [1.0; 4] },
        30,
    );
    comp.layers.push(layer);
    Project {
        compositions: vec![comp],
        active_composition_idx: 0,
        assets: Vec::new(),
        use_gpu_compute: false,
    }
}

#[test]
fn test_tick_writes_only_when_dirty_and_due() {
    let dir = std::env::temp_dir().join(format!("kagari_autosave_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut mgr = AutosaveManager::new(&dir).with_interval(Duration::from_secs(0));

    // Not dirty → no write
    assert!(mgr.tick(&sample_project()).is_none());
    // Dirty → writes
    mgr.mark_dirty();
    let path = mgr
        .tick(&sample_project())
        .expect("should write when dirty");
    assert!(path.exists());

    // Recovery loads back the same project
    assert!(mgr.has_recovery());
    let recovered = mgr.load_latest_recovery().expect("recovery should load");
    assert_eq!(recovered.compositions[0].name, "AutoSaveComp");

    // Clear removes everything
    mgr.clear_recovery();
    assert!(!mgr.has_recovery());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn production_autosave_uses_latest_project_with_metadata() {
    let dir =
        std::env::temp_dir().join(format!("kagari_autosave_production_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut manager = AutosaveManager::new(&dir).with_interval(Duration::from_secs(0));
    let mut document = ProductionDocument::new(sample_project());
    document.audio.sample_rate = 44_100;
    let mut latest = sample_project();
    latest.active_composition_mut().name = "Latest".into();
    manager.mark_dirty();
    manager
        .tick_production(&latest, &document)
        .expect("writes production snapshot");

    let recovered = manager.load_latest_recovery().expect("recovers project");
    assert_eq!(recovered.active_composition().name, "Latest");
    assert_eq!(
        manager.load_latest_production().unwrap().audio.sample_rate,
        44_100
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn async_autosave_preserves_newer_edits_while_worker_writes() {
    let dir = std::env::temp_dir().join(format!("kagari_autosave_async_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut manager = AutosaveManager::new(&dir).with_interval(Duration::from_secs(0));
    let mut project = sample_project();
    manager.mark_dirty();
    assert!(manager.tick_async(&project, None).is_none());
    project.active_composition_mut().name = "Edited during save".into();
    manager.mark_dirty();
    assert!(manager.wait_for_pending().is_some());
    assert!(manager.has_unsaved_snapshot());

    assert!(manager.tick_async(&project, None).is_none());
    assert!(manager.wait_for_pending().is_some());
    assert!(!manager.has_unsaved_snapshot());
    assert_eq!(
        manager
            .load_latest_recovery()
            .unwrap()
            .active_composition()
            .name,
        "Edited during save"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn async_production_snapshot_keeps_metadata_without_duplicate_project_payload() {
    let dir = std::env::temp_dir().join(format!(
        "kagari_autosave_async_production_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let mut manager = AutosaveManager::new(&dir).with_interval(Duration::from_secs(0));
    let project = sample_project();
    let mut document = ProductionDocument::new(project.clone());
    document.audio.sample_rate = 44_100;
    document.bindings.clear();
    manager.mark_dirty();
    assert!(manager.tick_async(&project, Some(&document)).is_none());
    assert!(manager.wait_for_pending().is_some());

    let recovered = manager.load_latest_snapshot().unwrap();
    assert_eq!(recovered.1.unwrap().audio.sample_rate, 44_100);
    let json = std::fs::read_to_string(manager.slot_path(0)).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(value["production_document"].get("project").is_none());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn production_snapshot_written_by_older_schema_remains_recoverable() {
    let dir = std::env::temp_dir().join(format!(
        "kagari_autosave_legacy_production_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let project = sample_project();
    let mut old_document = ProductionDocument::new(project.clone());
    old_document.audio.sample_rate = 44_100;
    let old_json = serde_json::json!({
        "project": project,
        "production_document": old_document,
    });
    std::fs::write(
        dir.join("recovery_0.json"),
        serde_json::to_vec(&old_json).unwrap(),
    )
    .unwrap();

    let recovered = AutosaveManager::new(&dir)
        .load_latest_snapshot()
        .expect("legacy production snapshot should load");
    assert_eq!(recovered.1.unwrap().audio.sample_rate, 44_100);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn legacy_project_snapshot_remains_recoverable() {
    let dir = std::env::temp_dir().join(format!("kagari_autosave_legacy_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let json = serde_json::to_string(&sample_project()).unwrap();
    std::fs::write(dir.join("recovery_0.json"), json).unwrap();
    let manager = AutosaveManager::new(&dir);
    assert_eq!(
        manager
            .load_latest_recovery()
            .unwrap()
            .active_composition()
            .name,
        "AutoSaveComp"
    );
    assert!(manager.load_latest_production().is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn invalid_production_snapshot_is_not_returned_as_recovery() {
    let dir = std::env::temp_dir().join(format!(
        "kagari_autosave_invalid_production_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut document = ProductionDocument::new(sample_project());
    document.audio.sample_rate = 0;
    let snapshot = AutosaveSnapshot {
        project: sample_project(),
        production_document: Some((&document).into()),
    };
    std::fs::write(
        dir.join("recovery_0.json"),
        serde_json::to_string(&snapshot).unwrap(),
    )
    .unwrap();

    let manager = AutosaveManager::new(&dir);
    assert!(manager.load_latest_production().is_none());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn project_file_validation_accepts_legacy_and_production_documents() {
    let dir = std::env::temp_dir().join(format!(
        "kagari_project_file_validation_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let legacy_path = dir.join("legacy.json");
    let production_path = dir.join("production.aura");
    let invalid_path = dir.join("invalid.json");
    std::fs::write(
        &legacy_path,
        serde_json::to_string(&sample_project()).unwrap(),
    )
    .unwrap();
    std::fs::write(
        &production_path,
        ProductionDocument::new(sample_project()).to_json().unwrap(),
    )
    .unwrap();
    std::fs::write(&invalid_path, "{\"project\": null}").unwrap();

    assert!(is_valid_project_file(&legacy_path));
    assert!(is_valid_project_file(&production_path));
    assert!(!is_valid_project_file(&invalid_path));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn recovery_skips_project_with_invalid_active_composition() {
    let dir = std::env::temp_dir().join(format!(
        "kagari_autosave_invalid_project_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut project = sample_project();
    project.active_composition_idx = project.compositions.len();
    std::fs::write(
        dir.join("recovery_0.json"),
        serde_json::to_string(&project).unwrap(),
    )
    .unwrap();

    assert!(AutosaveManager::new(&dir).load_latest_recovery().is_none());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn production_recovery_skips_document_with_invalid_project() {
    let dir = std::env::temp_dir().join(format!(
        "kagari_autosave_invalid_production_project_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut project = sample_project();
    project.active_composition_idx = project.compositions.len();
    let document = ProductionDocument::new(project.clone());
    let snapshot = AutosaveSnapshot {
        project,
        production_document: Some((&document).into()),
    };
    std::fs::write(
        dir.join("recovery_0.json"),
        serde_json::to_string(&snapshot).unwrap(),
    )
    .unwrap();

    assert!(AutosaveManager::new(&dir)
        .load_latest_production()
        .is_none());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn failed_tick_keeps_dirty_state_for_retry() {
    let path = std::env::temp_dir().join(format!("kagari_autosave_blocked_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::write(&path, "not a directory").unwrap();
    let mut manager = AutosaveManager::new(&path).with_interval(Duration::from_secs(0));
    manager.mark_dirty();
    assert!(manager.tick(&sample_project()).is_none());
    std::fs::remove_file(&path).unwrap();
    manager.retry_at = Some(Instant::now() - Duration::from_secs(1));
    assert!(manager.tick(&sample_project()).is_some());
    let _ = std::fs::remove_dir_all(&path);
}

#[test]
fn failed_snapshot_replace_removes_unique_temp_file() {
    let dir = std::env::temp_dir().join(format!(
        "kagari_autosave_replace_failure_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir(dir.join("recovery_0.json")).unwrap();

    let mut manager = AutosaveManager::new(&dir).with_interval(Duration::ZERO);
    manager.mark_dirty();
    assert!(manager.tick(&sample_project()).is_none());

    let temporary_files = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.file_name())
        .filter(|name| name.to_string_lossy().contains(".json."))
        .collect::<Vec<_>>();
    assert!(
        temporary_files.is_empty(),
        "temporary files: {temporary_files:?}"
    );

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_rotating_slots_never_exceed_limit() {
    let dir = std::env::temp_dir().join(format!("kagari_autosave_rot_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut mgr = AutosaveManager::new(&dir).with_interval(Duration::from_secs(0));

    for i in 0..(MAX_AUTOSAVE_SLOTS * 3) {
        mgr.mark_dirty();
        mgr.tick(&sample_project()).expect("each tick should write");
        assert_eq!(mgr.next_slot, (i + 1) % MAX_AUTOSAVE_SLOTS);
    }

    // Count recovery files on disk
    let count = std::fs::read_dir(&dir).unwrap().count();
    assert!(
        count <= MAX_AUTOSAVE_SLOTS,
        "must not exceed slot limit, got {}",
        count
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_corrupt_recovery_files_are_skipped() {
    let dir = std::env::temp_dir().join(format!("kagari_autosave_corrupt_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // Slot 0: truncated garbage; slot 1: valid project
    std::fs::write(dir.join("recovery_0.json"), "{\"compositions\": [").unwrap();
    let good = dir.join("recovery_1.json");
    serde_json::to_writer_pretty(std::fs::File::create(&good).unwrap(), &sample_project()).unwrap();

    let mgr = AutosaveManager::new(&dir);
    let recovered = mgr
        .load_latest_recovery()
        .expect("valid slot must be found");
    assert_eq!(recovered.compositions[0].name, "AutoSaveComp");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_atomic_write_leaves_no_tmp_files() {
    let dir = std::env::temp_dir().join(format!("kagari_autosave_tmp_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut mgr = AutosaveManager::new(&dir).with_interval(Duration::from_secs(0));
    mgr.mark_dirty();
    mgr.save_now(&sample_project()).expect("save_now works");

    let tmp_count = std::fs::read_dir(&dir)
        .unwrap()
        .filter(|e| {
            e.as_ref()
                .unwrap()
                .path()
                .extension()
                .is_some_and(|x| x == "tmp")
        })
        .count();
    assert_eq!(
        tmp_count, 0,
        "no .tmp files may remain after successful save"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
