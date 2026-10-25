//! Forward & Backward Compatible Project Schema Migration Engine.
//!
//! Ensures saved project files (.json) remain 100% loadable even as
//! fields are added, renamed, or refactored across application versions.

pub mod bundle;
pub mod schema;
pub mod storage;

pub use bundle::collect_production_document;
pub use schema::{
    load_project_migrated, migrate_schema_json, save_project_versioned, VersionedProjectFile,
    CURRENT_SCHEMA_VERSION, MAX_PROJECT_JSON_BYTES,
};
pub use storage::{
    load_project_with_backup, read_bounded_text_file, read_project_json_file, save_project_atomic,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::timeline::Project;

    #[test]
    fn test_save_and_load_versioned_project() {
        let proj = Project::default();
        let json = save_project_versioned(&proj).unwrap();
        let loaded = load_project_migrated(&json).unwrap();
        assert_eq!(loaded.compositions.len(), proj.compositions.len());
    }

    #[test]
    fn test_migrate_v0_json() {
        let proj = Project::default();
        let mut proj_val = serde_json::to_value(&proj).unwrap();
        if let Some(obj) = proj_val.as_object_mut() {
            obj.remove("active_composition_idx");
        }
        let v0_json = serde_json::to_string(&proj_val).unwrap();

        let loaded = load_project_migrated(&v0_json).unwrap();
        assert_eq!(loaded.active_composition_idx, 0);
    }

    #[test]
    fn test_rejects_future_schema_without_discarding_unknown_data() {
        let wrapper = VersionedProjectFile {
            schema_version: CURRENT_SCHEMA_VERSION + 1,
            project_data: serde_json::to_value(Project::default()).unwrap(),
        };
        let json = serde_json::to_string(&wrapper).unwrap();

        let error = load_project_migrated(&json).unwrap_err();
        assert!(error.contains("newer than supported"));
    }

    #[test]
    fn test_rejects_unrenderable_legacy_composition() {
        let mut project = Project::default();
        project.compositions[0].width = 20_000;
        let json = serde_json::to_string(&project).unwrap();

        let error = load_project_migrated(&json).unwrap_err();
        assert!(error.contains("validation"));
    }

    #[test]
    fn bounded_project_reader_returns_explicit_errors() {
        let dir = std::env::temp_dir().join(format!(
            "kagari_bounded_reader_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let oversized = dir.join("oversized.json");
        std::fs::write(&oversized, b"123456789").unwrap();

        let error = read_bounded_text_file(&oversized, 8).unwrap_err();
        assert!(error.contains("8 byte limit"), "unexpected error: {error}");

        let invalid_utf8 = dir.join("invalid-utf8.json");
        std::fs::write(&invalid_utf8, [0xff, 0xfe]).unwrap();
        let error = read_bounded_text_file(&invalid_utf8, 8).unwrap_err();
        assert!(
            !error.is_empty(),
            "invalid text must be reported to the caller"
        );

        let _ = std::fs::remove_dir_all(dir);
    }
}

#[cfg(test)]
mod collect_tests {
    use super::*;
    use crate::core::timeline::{Layer, LayerType, Project};

    #[test]
    fn collect_bundle_copies_and_rewrites_project_media_paths() {
        let root = std::env::temp_dir().join(format!(
            "kagari_collect_bundle_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let source_dir = root.join("source");
        let destination = root.join("output");
        std::fs::create_dir_all(&source_dir).unwrap();
        std::fs::create_dir_all(&destination).unwrap();
        let source = source_dir.join("shot plate.png");
        std::fs::write(&source, b"collected-media").unwrap();

        let mut project = Project::empty_for_new_project();
        project.assets.push(crate::core::timeline::ProjectItem::new(
            "image_0",
            "Shot Plate",
            crate::core::timeline::ProjectItemType::Image {
                path: source.to_string_lossy().into_owned(),
                width: 1,
                height: 1,
            },
        ));
        let document = crate::core::production_document::ProductionDocument::new(project);
        let (project_path, copied) =
            collect_production_document(&document, &destination, &source_dir).unwrap();
        assert_eq!(copied, 1);
        let collected =
            crate::core::production_document::ProductionDocument::load(&project_path).unwrap();
        let media_path = collected.project.assets[0].media_path().unwrap();
        assert!(!std::path::Path::new(media_path).is_absolute());
        assert_eq!(
            std::fs::read(
                project_path
                    .parent()
                    .unwrap()
                    .join("Footage")
                    .join(media_path)
            )
            .unwrap(),
            b"collected-media"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn collect_bundle_includes_and_rewrites_obj_material_and_texture_files() {
        let root = std::env::temp_dir().join(format!(
            "kagari_collect_obj_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let source_dir = root.join("source");
        let destination = root.join("output");
        std::fs::create_dir_all(&source_dir).unwrap();
        std::fs::create_dir_all(&destination).unwrap();
        let model = source_dir.join("scene.obj");
        let material = source_dir.join("scene.mtl");
        let texture = source_dir.join("albedo.png");
        std::fs::write(&model, "mtllib scene.mtl\no Scene\n").unwrap();
        std::fs::write(&material, "newmtl Surface\nmap_Kd albedo.png\n").unwrap();
        std::fs::write(&texture, b"texture").unwrap();

        let mut project = Project::empty_for_new_project();
        project.assets.push(crate::core::timeline::ProjectItem::new(
            "model_0",
            "Scene",
            crate::core::timeline::ProjectItemType::Model3D {
                path: model.to_string_lossy().into_owned(),
            },
        ));
        let document = crate::core::production_document::ProductionDocument::new(project);
        let (project_path, copied) =
            collect_production_document(&document, &destination, &source_dir).unwrap();
        assert_eq!(copied, 3);

        let footage = project_path.parent().unwrap().join("Footage");
        let collected_model = std::fs::read_dir(&footage)
            .unwrap()
            .flatten()
            .map(|entry| entry.path())
            .find(|path| {
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .ends_with("_scene.obj")
            })
            .unwrap();
        let model_text = std::fs::read_to_string(&collected_model).unwrap();
        let material_name = model_text
            .lines()
            .find_map(|line| line.strip_prefix("mtllib "))
            .unwrap();
        let collected_material = footage.join(material_name);
        assert!(collected_material.is_file());
        let material_text = std::fs::read_to_string(&collected_material).unwrap();
        let texture_name = material_text
            .lines()
            .find_map(|line| line.strip_prefix("map_Kd "))
            .unwrap();
        assert!(footage.join(texture_name).is_file());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn collect_bundle_keeps_video_metadata_valid_for_cache_rebuild() {
        let root = std::env::temp_dir().join(format!(
            "kagari_collect_video_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let source_dir = root.join("source");
        let destination = root.join("output");
        std::fs::create_dir_all(&source_dir).unwrap();
        std::fs::create_dir_all(&destination).unwrap();
        let source = source_dir.join("shot.mp4");
        std::fs::write(&source, b"video-source").unwrap();

        let mut project = Project::empty_for_new_project();
        let (layer_id, duration_frames) = {
            let composition = project.active_composition_mut();
            (
                composition.next_layer_id("layer_video"),
                composition.duration_frames,
            )
        };
        let composition = project.active_composition_mut();
        composition.layers.push(Layer::new(
            layer_id,
            "Shot".to_string(),
            LayerType::Video {
                source: source.to_string_lossy().into_owned(),
                frames_dir: source_dir.join("cache").to_string_lossy().into_owned(),
                frame_count: 12,
                audio_wav: None,
                audio_volume: crate::core::property::Animatable::new_constant(0.0),
                speed: 1.0,
            },
            duration_frames,
        ));

        let document = crate::core::production_document::ProductionDocument::new(project);
        let (project_path, copied) =
            collect_production_document(&document, &destination, &source_dir).unwrap();
        assert_eq!(copied, 1);
        let collected =
            crate::core::production_document::ProductionDocument::load(&project_path).unwrap();
        let LayerType::Video {
            source: collected_source,
            frames_dir,
            frame_count,
            audio_wav,
            ..
        } = &collected.project.compositions[0].layers[0].layer_type
        else {
            panic!("collected video layer must remain a video");
        };
        assert_eq!(*frame_count, 12);
        assert_eq!(frames_dir, "cache_pending");
        assert!(audio_wav.is_none());
        assert!(project_path
            .parent()
            .unwrap()
            .join("Footage")
            .join(collected_source)
            .is_file());
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod backup_tests {
    use super::*;
    use crate::core::timeline::{Composition, Layer, LayerType, Project};

    fn project_named(name: &str) -> Project {
        let mut comp = Composition::new("c1".into(), name.into(), 32, 32, 30, 30);
        comp.layers.push(Layer::new(
            "l".into(),
            "S".into(),
            LayerType::Solid { color: [1.0; 4] },
            30,
        ));
        Project {
            compositions: vec![comp],
            active_composition_idx: 0,
            assets: Vec::new(),
            use_gpu_compute: false,
        }
    }

    #[test]
    fn test_atomic_save_creates_backup_of_previous_generation() {
        let dir = std::env::temp_dir().join(format!("kagari_bak_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("proj.json");

        save_project_atomic(&project_named("v1"), &path).unwrap();
        assert!(!path.with_extension("json.bak").exists());

        save_project_atomic(&project_named("v2"), &path).unwrap();
        assert!(path.with_extension("json.bak").exists());

        let (loaded, used_backup) = load_project_with_backup(&path).unwrap();
        assert_eq!(loaded.compositions[0].name, "v2");
        assert!(!used_backup);

        let bak_loaded = load_project_migrated(
            &std::fs::read_to_string(path.with_extension("json.bak")).unwrap(),
        )
        .unwrap();
        assert_eq!(bak_loaded.compositions[0].name, "v1");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_corrupt_primary_falls_back_to_backup() {
        let dir = std::env::temp_dir().join(format!("kagari_bak2_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("proj.json");

        save_project_atomic(&project_named("older_generation"), &path).unwrap();
        save_project_atomic(&project_named("newer_generation"), &path).unwrap();
        std::fs::write(&path, "{ truncated garbage").unwrap();

        let (loaded, used_backup) = load_project_with_backup(&path).unwrap();
        assert!(used_backup);
        assert_eq!(loaded.compositions[0].name, "older_generation");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_both_missing_is_an_error() {
        let dir = std::env::temp_dir().join(format!("kagari_bak3_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let result = load_project_with_backup(dir.join("nonexistent.json"));
        assert!(result.is_err());
    }
}
