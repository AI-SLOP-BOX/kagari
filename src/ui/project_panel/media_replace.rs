use crate::core::timeline::{Composition, LayerType, ProjectItemType};
use crate::KagariApp;
use eframe::egui;

pub(crate) fn classify_imported_media(path: &std::path::Path) -> Result<ProjectItemType, String> {
    let file_path = path.to_string_lossy().into_owned();
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    if matches!(
        extension.as_str(),
        "mp4"
            | "mov"
            | "mkv"
            | "webm"
            | "avi"
            | "m4v"
            | "mpeg"
            | "mpg"
            | "ts"
            | "m2ts"
            | "av1"
            | "ivf"
    ) {
        Ok(ProjectItemType::Video {
            path: file_path,
            duration_sec: 0.0,
        })
    } else if matches!(
        extension.as_str(),
        "wav" | "mp3" | "m4a" | "aac" | "flac" | "ogg" | "aiff" | "aif"
    ) {
        Ok(ProjectItemType::Audio {
            path: file_path,
            duration_sec: 0.0,
        })
    } else if extension == "obj" {
        Ok(ProjectItemType::Model3D { path: file_path })
    } else if matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "webp") {
        let (width, height) = image::image_dimensions(path)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        Ok(ProjectItemType::Image {
            path: file_path,
            width,
            height,
        })
    } else {
        Err(format!("未対応の素材形式です: {}", path.display()))
    }
}

pub(crate) fn replacement_filter(
    item_type: &ProjectItemType,
) -> Option<(&'static str, &'static [&'static str])> {
    match item_type {
        ProjectItemType::Image { .. } => Some(("Image Footage", &["png", "jpg", "jpeg", "webp"])),
        ProjectItemType::Video { .. } => Some((
            "Video Footage",
            &[
                "mp4", "mov", "mkv", "webm", "avi", "m4v", "mpeg", "mpg", "ts", "m2ts", "av1",
                "ivf",
            ],
        )),
        ProjectItemType::Audio { .. } => Some((
            "Audio Footage",
            &["wav", "mp3", "m4a", "aac", "flac", "ogg", "aif", "aiff"],
        )),
        ProjectItemType::Model3D { .. } => Some(("3D Model", &["obj"])),
        _ => None,
    }
}

pub(crate) fn same_media_kind(left: &ProjectItemType, right: &ProjectItemType) -> bool {
    matches!(
        (left, right),
        (ProjectItemType::Image { .. }, ProjectItemType::Image { .. })
            | (ProjectItemType::Video { .. }, ProjectItemType::Video { .. })
            | (ProjectItemType::Audio { .. }, ProjectItemType::Audio { .. })
            | (
                ProjectItemType::Model3D { .. },
                ProjectItemType::Model3D { .. }
            )
    )
}

pub(crate) fn media_path_matches(candidate: &str, old_path: Option<&str>) -> bool {
    old_path.is_some_and(|path| candidate == path)
}

pub(crate) fn collect_video_replacement_fps(
    comp: &Composition,
    old_path: Option<&str>,
    fps_values: &mut std::collections::HashSet<u32>,
) {
    let has_video = comp.layers.iter().any(|layer| {
        matches!(&layer.layer_type, LayerType::Video { source, .. }
            if media_path_matches(source, old_path))
    });
    if has_video {
        fps_values.insert(comp.fps.max(1));
    }
    for sub in &comp.sub_compositions {
        collect_video_replacement_fps(sub, old_path, fps_values);
    }
}

pub(crate) fn replace_media_in_composition(
    comp: &mut Composition,
    old_name: &str,
    old_path: Option<&str>,
    new_name: &str,
    new_path: &str,
    is_video_asset: bool,
    imported_video_by_fps: &std::collections::HashMap<u32, crate::core::video_import::VideoAsset>,
) {
    for layer in &mut comp.layers {
        let matched_name = layer.name == old_name;
        let matches_asset = match &mut layer.layer_type {
            LayerType::Image { path } if media_path_matches(path, old_path) => {
                *path = new_path.to_string();
                true
            }
            LayerType::Model3D { path } if media_path_matches(path, old_path) => {
                *path = new_path.to_string();
                true
            }
            LayerType::Audio { path, .. } if media_path_matches(path, old_path) => {
                *path = new_path.to_string();
                true
            }
            LayerType::Video {
                source,
                frames_dir,
                frame_count,
                audio_wav,
                ..
            } if is_video_asset && media_path_matches(source, old_path) => {
                if let Some(imported) = imported_video_by_fps.get(&comp.fps) {
                    *source = imported.source_path.clone();
                    *frames_dir = imported.frames_dir.clone();
                    *frame_count = imported.frame_count;
                    *audio_wav = imported.audio_wav.clone();
                }
                true
            }
            _ => false,
        };
        if matches_asset && matched_name {
            layer.name = new_name.to_string();
        }
    }
    for sub in &mut comp.sub_compositions {
        replace_media_in_composition(
            sub,
            old_name,
            old_path,
            new_name,
            new_path,
            is_video_asset,
            imported_video_by_fps,
        );
    }
}

pub(crate) fn discard_replacement_video_caches(
    videos: &std::collections::HashMap<u32, crate::core::video_import::VideoAsset>,
) {
    for video in videos.values() {
        if let Some(import_dir) = std::path::Path::new(&video.frames_dir).parent() {
            let _ = std::fs::remove_dir_all(import_dir);
        }
    }
}

pub(crate) fn start_replace_footage_import(
    app: &mut KagariApp,
    ctx: &egui::Context,
    asset_idx: usize,
    new_path: std::path::PathBuf,
) {
    let Some(asset) = app.history.current().assets.get(asset_idx).cloned() else {
        app.toasts.error("置き換える素材が見つかりません。");
        return;
    };
    let Some(old_path) = asset.media_path().map(str::to_string) else {
        app.toasts
            .error("この種類の項目は素材置換に対応していません。");
        return;
    };
    let new_path = new_path.to_string_lossy().into_owned();
    let new_name = std::path::Path::new(&new_path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let replacement_item_type = match classify_imported_media(std::path::Path::new(&new_path)) {
        Ok(item_type) if same_media_kind(&asset.item_type, &item_type) => item_type,
        Ok(_) => {
            app.toasts
                .error("置き換える素材は元と同じ種類を選んでください。");
            return;
        }
        Err(error) => {
            app.toasts.error(error);
            return;
        }
    };
    let is_video = matches!(&asset.item_type, ProjectItemType::Video { .. });
    if is_video && !crate::core::video_import::ffmpeg_available() {
        app.toasts
            .error("動画の置き換えにはFFmpegが必要です。PATHを確認してください。");
        return;
    }
    let mut required_fps = std::collections::HashSet::new();
    if is_video {
        for comp in &app.history.current().compositions {
            collect_video_replacement_fps(comp, Some(&old_path), &mut required_fps);
        }
    }
    let mut required_fps: Vec<_> = required_fps.into_iter().collect();
    required_fps.sort_unstable();
    let (tx, rx) = std::sync::mpsc::channel();
    let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    app.import_rx.push(crate::ui::drop_import::ImportJob {
        receiver: rx,
        label: new_name.clone(),
        cancel: is_video.then(|| cancel.clone()),
    });
    app.toasts
        .info(format!("素材「{}」を読み込み中…", new_name));
    ctx.request_repaint();

    let payload = crate::ui::drop_import::ReplaceFootageImport {
        project_session_id: app.project_session_id,
        asset_id: asset.id,
        old_name: asset.name,
        old_path,
        new_name,
        new_path,
        replacement_item_type,
        imported_video_by_fps: std::collections::HashMap::new(),
    };
    std::thread::spawn(move || {
        let result = (|| {
            let mut payload = payload;
            if is_video {
                let stem = std::path::Path::new(&payload.new_path)
                    .file_stem()
                    .map(|value| value.to_string_lossy())
                    .unwrap_or_else(|| std::borrow::Cow::Borrowed("video"));
                let safe_stem: String = stem
                    .chars()
                    .map(|ch| {
                        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                            ch
                        } else {
                            '_'
                        }
                    })
                    .collect();
                for fps in required_fps {
                    let destination = std::env::temp_dir()
                        .join("kagari_media")
                        .join("relinked")
                        .join(format!(
                            "asset_{}_{}fps_{}",
                            payload.asset_id, fps, safe_stem
                        ));
                    let imported = match crate::core::video_import::import_video_with_cancel(
                        &payload.new_path,
                        &destination,
                        fps as f32,
                        &cancel,
                    ) {
                        Ok(imported) => imported,
                        Err(error) => {
                            discard_replacement_video_caches(&payload.imported_video_by_fps);
                            return Err(error.to_string());
                        }
                    };
                    payload.imported_video_by_fps.insert(fps, imported);
                }
            }
            Ok(payload)
        })();
        let _ = tx.send(crate::ui::drop_import::ImportResult::ReplaceFootage(result));
    });
}

pub(crate) fn complete_replace_footage(
    app: &mut KagariApp,
    replacement: crate::ui::drop_import::ReplaceFootageImport,
) {
    if replacement.project_session_id != app.project_session_id {
        discard_replacement_video_caches(&replacement.imported_video_by_fps);
        app.toasts
            .warning("プロジェクト切替後に素材読込が完了したため、置き換えを取り消しました。");
        return;
    }
    let mut project = app.history.current().clone();
    let Some(asset) = project
        .assets
        .iter_mut()
        .find(|asset| asset.id == replacement.asset_id)
    else {
        discard_replacement_video_caches(&replacement.imported_video_by_fps);
        app.toasts
            .warning("読み込み中に置換対象の素材が削除されたため、置き換えを取り消しました。");
        return;
    };
    if asset.name != replacement.old_name
        || asset.media_path() != Some(replacement.old_path.as_str())
        || !same_media_kind(&asset.item_type, &replacement.replacement_item_type)
    {
        discard_replacement_video_caches(&replacement.imported_video_by_fps);
        app.toasts
            .warning("置換対象が読み込み中に変更されたため、素材の置き換えを取り消しました。");
        return;
    }
    let is_video = matches!(&asset.item_type, ProjectItemType::Video { .. });
    asset.name = replacement.new_name.clone();
    asset.item_type = replacement.replacement_item_type;
    for comp in &mut project.compositions {
        replace_media_in_composition(
            comp,
            &replacement.old_name,
            Some(&replacement.old_path),
            &replacement.new_name,
            &replacement.new_path,
            is_video,
            &replacement.imported_video_by_fps,
        );
    }
    app.commit_project(project);
    app.toasts.info(format!(
        "素材「{}」を「{}」に置き換えました。",
        replacement.old_name, replacement.new_name
    ));
}

