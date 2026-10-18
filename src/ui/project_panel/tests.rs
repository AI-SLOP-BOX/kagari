#[cfg(test)]
#[allow(clippy::items_after_test_module)]
#[allow(clippy::module_inception)]
mod tests {
    use crate::core::timeline::{Composition, Layer, LayerType, ProjectItemType};
    use crate::ui::project_panel::media_replace::{
        classify_imported_media, replace_media_in_composition,
    };
    use std::collections::HashMap;
    use std::path::Path;

    #[test]
    fn project_panel_classifies_video_containers_as_video_assets() {
        for extension in ["mp4", "MOV", "webm", "mkv", "av1"] {
            assert!(matches!(
                classify_imported_media(Path::new(&format!("clip.{extension}"))),
                Ok(ProjectItemType::Video { .. })
            ));
        }
    }

    #[test]
    fn project_panel_classifies_audio_formats_without_case_sensitive_suffixes() {
        for extension in ["wav", "MP3", "m4a", "flac", "aiff"] {
            assert!(matches!(
                classify_imported_media(Path::new(&format!("take.{extension}"))),
                Ok(ProjectItemType::Audio { .. })
            ));
        }
    }

    #[test]
    fn project_panel_rejects_unknown_media_instead_of_fabricating_an_image() {
        assert!(classify_imported_media(Path::new("still.avif")).is_err());
        assert!(classify_imported_media(Path::new("notes.txt")).is_err());
    }

    #[test]
    fn project_panel_classifies_obj_as_3d_model_asset() {
        assert!(matches!(
            classify_imported_media(Path::new("hero.OBJ")),
            Ok(ProjectItemType::Model3D { .. })
        ));
    }

    #[test]
    fn replace_footage_updates_nested_precomp_video_layers() {
        let mut root = Composition::new("root".into(), "Root".into(), 64, 64, 24, 24);
        let mut nested = Composition::new("nested".into(), "Nested".into(), 64, 64, 24, 24);
        nested.layers.push(Layer::new(
            "video".into(),
            "Shot".into(),
            LayerType::Video {
                source: "/old/shot.mp4".into(),
                frames_dir: "/old/cache".into(),
                frame_count: 4,
                audio_wav: Some("/old/audio.wav".into()),
                speed: 1.0,
            },
            24,
        ));
        root.sub_compositions.push(nested);

        let mut imported = HashMap::new();
        imported.insert(
            24,
            crate::core::video_import::VideoAsset {
                source_path: "/new/shot.mp4".into(),
                frames_dir: "/new/cache".into(),
                frame_count: 8,
                fps: 24.0,
                width: 64,
                height: 64,
                audio_wav: Some("/new/audio.wav".into()),
            },
        );

        replace_media_in_composition(
            &mut root,
            "Shot",
            Some("/old/shot.mp4"),
            "Shot Replaced",
            "/new/shot.mp4",
            true,
            &imported,
        );

        let LayerType::Video {
            source,
            frames_dir,
            frame_count,
            audio_wav,
            ..
        } = &root.sub_compositions[0].layers[0].layer_type
        else {
            panic!("expected nested video layer");
        };
        assert_eq!(root.sub_compositions[0].layers[0].name, "Shot Replaced");
        assert_eq!(source, "/new/shot.mp4");
        assert_eq!(frames_dir, "/new/cache");
        assert_eq!(*frame_count, 8);
        assert_eq!(audio_wav.as_deref(), Some("/new/audio.wav"));
    }

    #[test]
    fn replacing_model_footage_updates_bound_layer_without_renaming_unrelated_layers() {
        let mut comp = Composition::new("root".into(), "Root".into(), 64, 64, 24, 24);
        comp.layers.push(Layer::new(
            "model".into(),
            "fire.obj".into(),
            LayerType::Model3D {
                path: "/old/fire.obj".into(),
            },
            24,
        ));
        comp.layers.push(Layer::new(
            "other-model".into(),
            "fire.obj".into(),
            LayerType::Model3D {
                path: "/other/fire.obj".into(),
            },
            24,
        ));
        comp.layers.push(Layer::new(
            "solid".into(),
            "fire.obj".into(),
            LayerType::Solid { color: [1.0; 4] },
            24,
        ));

        replace_media_in_composition(
            &mut comp,
            "fire.obj",
            Some("/old/fire.obj"),
            "fire-new.obj",
            "/new/fire.obj",
            false,
            &HashMap::new(),
        );

        assert_eq!(comp.layers[0].name, "fire-new.obj");
        assert!(matches!(
            &comp.layers[0].layer_type,
            LayerType::Model3D { path } if path == "/new/fire.obj"
        ));
        assert_eq!(comp.layers[1].name, "fire.obj");
        assert!(matches!(
            &comp.layers[1].layer_type,
            LayerType::Model3D { path } if path == "/other/fire.obj"
        ));
        assert_eq!(comp.layers[2].name, "fire.obj");
    }
}
