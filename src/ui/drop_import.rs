//! Drag-and-drop media import: images become Image layers, videos spawn the
//! ffmpeg frame-extraction pipeline on a worker thread, WAVs become Audio
//! layers. Results stream back through a channel and are inserted above the
//! current selection.
use crate::KagariApp;
use eframe::egui;

pub enum ImportResult {
    Video(
        crate::core::video_import::VideoAsset,
        String,
        bool,
        ImportTarget,
    ),
    ImageSequence(crate::core::video_import::VideoAsset, String, ImportTarget),
    ReplaceFootage(Result<ReplaceFootageImport, String>),
    Err(String, bool),
}

pub struct ImportJob {
    pub receiver: std::sync::mpsc::Receiver<ImportResult>,
    pub label: String,
    pub cancel: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
}

pub struct ReplaceFootageImport {
    pub project_session_id: u64,
    pub asset_id: String,
    pub old_name: String,
    pub old_path: String,
    pub new_name: String,
    pub new_path: String,
    pub replacement_item_type: crate::core::timeline::ProjectItemType,
    pub imported_video_by_fps:
        std::collections::HashMap<u32, crate::core::video_import::VideoAsset>,
}

#[derive(Clone)]
pub struct ImportTarget {
    project_session_id: u64,
    composition_id: String,
    selected_layer_id: Option<String>,
}

pub fn start_video_import(
    app: &mut KagariApp,
    ctx: &egui::Context,
    path: std::path::PathBuf,
    name: String,
) {
    start_video_import_mode(app, ctx, path, name, false);
}

pub fn start_video_tutorial_practice(
    app: &mut KagariApp,
    ctx: &egui::Context,
    path: std::path::PathBuf,
    name: String,
) {
    start_video_import_mode(app, ctx, path, name, true);
}

pub fn start_image_sequence_import(
    app: &mut KagariApp,
    ctx: &egui::Context,
    path: std::path::PathBuf,
    name: String,
) {
    let comp = app.history.current().active_composition();
    let fps = comp.fps.max(1) as f32;
    let target = ImportTarget {
        project_session_id: app.project_session_id,
        composition_id: comp.id.clone(),
        selected_layer_id: app
            .selection
            .selected_layer_idx
            .and_then(|index| comp.layers.get(index))
            .map(|layer| layer.id.clone()),
    };
    let destination = std::env::temp_dir()
        .join("kagari_media")
        .join("image_sequences")
        .join(&name);
    let (tx, rx) = std::sync::mpsc::channel();
    app.import_rx.push(ImportJob {
        receiver: rx,
        label: name.clone(),
        cancel: None,
    });
    app.toasts
        .info(format!("画像シーケンス「{}」を読み込み中…", name));
    ctx.request_repaint();
    let handle = std::thread::spawn(move || {
        let result = crate::core::video_import::import_image_sequence(&path, &destination, fps)
            .map(|asset| ImportResult::ImageSequence(asset, name, target))
            .unwrap_or_else(|error| ImportResult::Err(error.to_string(), false));
        let _ = tx.send(result);
    });
    app.track_background_thread(handle);
}

fn start_video_import_mode(
    app: &mut KagariApp,
    ctx: &egui::Context,
    path: std::path::PathBuf,
    name: String,
    tutorial_practice: bool,
) {
    if !crate::core::video_import::ffmpeg_available() {
        app.toasts
            .error("動画の読み込みにはFFmpegが必要です。PATHを確認してください。");
        return;
    }
    let comp = app.history.current().active_composition();
    let fps = comp.fps.max(1) as f32;
    let target = ImportTarget {
        project_session_id: app.project_session_id,
        composition_id: comp.id.clone(),
        selected_layer_id: app
            .selection
            .selected_layer_idx
            .and_then(|index| comp.layers.get(index))
            .map(|layer| layer.id.clone()),
    };
    let dest = std::env::temp_dir().join("kagari_media").join(&name);
    let source = path.to_string_lossy().into_owned();
    let (tx, rx) = std::sync::mpsc::channel();
    let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    app.import_rx.push(ImportJob {
        receiver: rx,
        label: name.clone(),
        cancel: Some(cancel.clone()),
    });
    app.toasts
        .info(format!("「{}」のフレームを読み込み中…", name));
    ctx.request_repaint();
    let handle = std::thread::spawn(move || {
        let result =
            crate::core::video_import::import_video_with_cancel(&source, &dest, fps, &cancel)
                .map(|asset| ImportResult::Video(asset, name, tutorial_practice, target))
                .unwrap_or_else(|error| ImportResult::Err(error.to_string(), tutorial_practice));
        let _ = tx.send(result);
    });
    app.track_background_thread(handle);
}

fn insert_layer_into_target(
    app: &mut KagariApp,
    layer: crate::core::timeline::Layer,
    target: &ImportTarget,
    label: &str,
) -> Option<usize> {
    let mut inserted_at = None;
    app.modify_project(|project| {
        let Some(comp) = project
            .compositions
            .iter_mut()
            .find(|comp| comp.id == target.composition_id)
        else {
            return;
        };
        let comp_duration = comp.duration_frames.max(1);
        let mut layer = layer;
        layer.in_frame = layer.in_frame.min(comp_duration.saturating_sub(1));
        layer.out_frame = layer.out_frame.min(comp_duration).max(layer.in_frame + 1);
        let index = target
            .selected_layer_id
            .as_ref()
            .and_then(|selected_id| {
                comp.layers
                    .iter()
                    .position(|candidate| candidate.id == *selected_id)
            })
            .map(|index| index + 1)
            .unwrap_or(0)
            .min(comp.layers.len());
        comp.layers.insert(index, layer);
        inserted_at = Some(index);
    });
    if inserted_at.is_some() {
        app.toasts.info(format!("読み込みました: {}", label));
    }
    inserted_at
}

fn insert_layer(
    app: &mut KagariApp,
    layer: crate::core::timeline::Layer,
    label: &str,
    duration_override: Option<u32>,
) -> usize {
    let comp_dur = duration_override
        .unwrap_or(app.history.current().active_composition().duration_frames)
        .max(1);
    let insert_at = app.selection.selected_layer_idx.map(|i| i + 1).unwrap_or(0);
    let mut l = layer;
    l.in_frame = l.in_frame.min(comp_dur.saturating_sub(1));
    l.out_frame = l.out_frame.min(comp_dur).max(l.in_frame + 1);
    let inserted_at = insert_at.min(app.history.current().active_composition().layers.len());
    app.modify_project(|proj| {
        let comp = proj.active_composition_mut();
        if let Some(duration) = duration_override {
            comp.duration_frames = duration;
        }
        let at = insert_at.min(comp.layers.len());
        comp.layers.insert(at, l);
    });
    app.toasts.info(format!("読み込みました: {}", label));
    inserted_at
}

pub fn handle_dropped_files(app: &mut KagariApp, ctx: &egui::Context) {
    // Drain completed workers before processing new drops; each import owns a
    // channel so concurrent imports cannot overwrite one another.
    let jobs = std::mem::take(&mut app.import_rx);
    for job in jobs {
        let rx = job.receiver;
        let mut disconnected = false;
        let mut received_result = false;
        loop {
            let res = match rx.try_recv() {
                Ok(result) => {
                    received_result = true;
                    result
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    disconnected = true;
                    break;
                }
            };
            match res {
                ImportResult::Video(asset, name, tutorial_practice, target) => {
                    if target.project_session_id != app.project_session_id {
                        app.toasts.warning(format!(
                            "プロジェクト切替後に「{}」の読み込みが完了したため、追加を取り消しました。",
                            name
                        ));
                        continue;
                    }
                    let dur = asset.frame_count.max(1);
                    let Some((comp_w, comp_h, layer_id, target_is_active)) = app
                        .history
                        .current()
                        .compositions
                        .iter()
                        .enumerate()
                        .find(|(_, comp)| comp.id == target.composition_id)
                        .map(|(index, comp)| {
                            (
                                comp.width as f32,
                                comp.height as f32,
                                comp.next_layer_id("video"),
                                index == app.history.current().active_composition_idx,
                            )
                        })
                    else {
                        app.toasts.error(format!(
                            "動画「{}」の読み込み先コンポジションが見つかりません。",
                            name
                        ));
                        continue;
                    };
                    let fit = (comp_w / asset.width.max(1) as f32)
                        .min(comp_h / asset.height.max(1) as f32);
                    let fit = if tutorial_practice { fit } else { fit.min(1.0) } * 100.0;
                    let mut layer = crate::core::timeline::Layer::new(
                        layer_id,
                        name.clone(),
                        crate::core::timeline::LayerType::Video {
                            source: asset.source_path.clone(),
                            frames_dir: asset.frames_dir.clone(),
                            frame_count: asset.frame_count,
                            audio_wav: asset.audio_wav.clone(),
                            speed: 1.0,
                        },
                        dur,
                    );
                    layer.transform.position = crate::core::property::Animatable::new_constant([
                        comp_w * 0.5,
                        comp_h * 0.5,
                    ]);
                    layer.transform.scale =
                        crate::core::property::Animatable::new_constant([fit, fit]);
                    if tutorial_practice {
                        layer.effects.push(crate::core::timeline::Effect {
                            id: "tutorial_color_tint".to_string(),
                            name: "Color Tint".to_string(),
                            effect_type: crate::core::timeline::EffectType::ColorTint {
                                color: crate::core::property::Animatable::new_constant([
                                    0.12, 0.72, 0.92, 1.0,
                                ]),
                                intensity: crate::core::property::Animatable::new_constant(65.0),
                            },
                            enabled: true,
                        });
                    }
                    let Some(inserted_at) =
                        insert_layer_into_target(app, layer, &target, &format!("動画「{}」", name))
                    else {
                        app.toasts.error(format!(
                            "動画「{}」を読み込み先へ追加できませんでした。",
                            name
                        ));
                        continue;
                    };
                    if tutorial_practice && target_is_active {
                        app.selection.select_single(inserted_at);
                        app.playback.current_frame = 0;
                        app.loop_playback = true;
                        app.playback.start_playing();
                        crate::core::frame_cache::bump_version();
                        app.toasts.info(
                            "動画にColor Tintを適用しました。再生しながらInspectorで強さを調整できます。",
                        );
                    }
                }
                ImportResult::ImageSequence(asset, name, target) => {
                    if target.project_session_id != app.project_session_id {
                        app.toasts.warning(format!(
                            "プロジェクト切替後に「{}」の読み込みが完了したため、追加を取り消しました。",
                            name
                        ));
                        continue;
                    }
                    let dur = asset.frame_count.max(1);
                    let Some((comp_w, comp_h, layer_id)) = app
                        .history
                        .current()
                        .compositions
                        .iter()
                        .find(|comp| comp.id == target.composition_id)
                        .map(|comp| {
                            (
                                comp.width as f32,
                                comp.height as f32,
                                comp.next_layer_id("image_sequence"),
                            )
                        })
                    else {
                        app.toasts.error(format!(
                            "画像シーケンス「{}」の読み込み先コンポジションが見つかりません。",
                            name
                        ));
                        continue;
                    };
                    let fit = (comp_w / asset.width.max(1) as f32)
                        .min(comp_h / asset.height.max(1) as f32)
                        .min(1.0)
                        * 100.0;
                    let mut layer = crate::core::timeline::Layer::new(
                        layer_id,
                        name.clone(),
                        crate::core::timeline::LayerType::Video {
                            source: asset.source_path.clone(),
                            frames_dir: asset.frames_dir.clone(),
                            frame_count: asset.frame_count,
                            audio_wav: None,
                            speed: 1.0,
                        },
                        dur,
                    );
                    layer.transform.position = crate::core::property::Animatable::new_constant([
                        comp_w * 0.5,
                        comp_h * 0.5,
                    ]);
                    layer.transform.scale =
                        crate::core::property::Animatable::new_constant([fit, fit]);
                    if insert_layer_into_target(
                        app,
                        layer,
                        &target,
                        &format!("画像シーケンス「{}」", name),
                    )
                    .is_none()
                    {
                        app.toasts.error(format!(
                            "画像シーケンス「{}」を読み込み先へ追加できませんでした。",
                            name
                        ));
                    }
                }
                ImportResult::ReplaceFootage(Ok(result)) => {
                    crate::ui::project_panel::complete_replace_footage(app, result);
                }
                ImportResult::ReplaceFootage(Err(error)) => {
                    if error.contains("video import cancelled") {
                        app.toasts.info("動画の置き換えをキャンセルしました。");
                    } else {
                        app.toasts
                            .error(format!("素材を置き換えられませんでした: {}", error));
                    }
                }
                ImportResult::Err(e, tutorial_practice) => {
                    if e.contains("video import cancelled") {
                        app.toasts.info("動画の読み込みをキャンセルしました。");
                        continue;
                    }
                    if tutorial_practice {
                        app.show_guided_tutorial = true;
                        app.tutorial = None;
                    }
                    app.toasts
                        .error(format!("メディアを読み込めませんでした: {}", e));
                }
            }
        }
        if disconnected && !received_result {
            app.toasts
                .error("動画の読み込み処理が予期せず終了しました。");
        }
        if !disconnected {
            app.import_rx.push(ImportJob {
                receiver: rx,
                label: job.label,
                cancel: job.cancel,
            });
        }
    }
    if !app.import_rx.is_empty() {
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }

    // 2) Pick up newly dropped files.
    let dropped = ctx.input(|i| i.raw.dropped_files.clone());
    if dropped.is_empty() {
        return;
    }
    for f in dropped {
        let Some(path) = f.path else { continue };
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "media".into());
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let path_str = path.to_string_lossy().to_string();

        match ext.as_str() {
            "png" | "jpg" | "jpeg" | "webp" | "bmp" | "gif" => {
                match image::image_dimensions(&path) {
                    Ok((iw, ih)) => {
                        let (cw, ch) = {
                            let c = app.history.current().active_composition();
                            (c.width as f32, c.height as f32)
                        };
                        let fit = ((cw / iw as f32).min(ch / ih as f32)).min(1.0) * 100.0;
                        let mut layer = crate::core::timeline::Layer::new(
                            app.history
                                .current()
                                .active_composition()
                                .next_layer_id("image"),
                            name.clone(),
                            crate::core::timeline::LayerType::Image {
                                path: path_str.clone(),
                            },
                            1,
                        );
                        layer.transform.position =
                            crate::core::property::Animatable::new_constant([cw / 2.0, ch / 2.0]);
                        layer.transform.scale =
                            crate::core::property::Animatable::new_constant([fit, fit]);
                        layer.out_frame =
                            app.history.current().active_composition().duration_frames;
                        insert_layer(
                            app,
                            layer,
                            &format!("image '{}' ({}×{})", name, iw, ih),
                            None,
                        );
                    }
                    Err(e) => app.toasts.error(format!("{}を読み取れません: {}", name, e)),
                }
            }
            "wav" | "mp3" | "m4a" | "aac" | "flac" | "ogg" | "aiff" | "aif" | "caf" => {
                let (cw, ch) = {
                    let c = app.history.current().active_composition();
                    (c.width as f32, c.height as f32)
                };
                let mut layer = crate::core::timeline::Layer::new(
                    format!("aud_{}", name),
                    name.to_string(),
                    crate::core::timeline::LayerType::Audio {
                        path: path_str.clone(),
                        volume: crate::core::property::Animatable::new_constant(1.0),
                    },
                    1,
                );
                layer.transform.position =
                    crate::core::property::Animatable::new_constant([cw / 2.0, ch / 2.0]);
                layer.out_frame = app.history.current().active_composition().duration_frames;
                insert_layer(app, layer, &format!("audio '{}'", name), None);
            }
            "mp4" | "mov" | "mkv" | "avi" | "webm" | "m4v" | "mpeg" | "mpg" | "ts" | "m2ts"
            | "av1" | "ivf" => {
                start_video_import(app, ctx, path, name);
            }
            other => {
                app.toasts
                    .error(format!("未対応のファイル形式です: .{}", other));
            }
        }
    }
}

pub fn draw_import_progress(app: &mut KagariApp, ctx: &egui::Context) {
    let cancellable = app
        .import_rx
        .iter()
        .filter_map(|job| job.cancel.clone().map(|cancel| (job.label.clone(), cancel)))
        .collect::<Vec<_>>();
    if cancellable.is_empty() {
        return;
    }
    egui::Window::new("メディア読み込み")
        .id(egui::Id::new("media-import-progress"))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .default_width(360.0)
        .show(ctx, |ui| {
            ui.label("メディアの準備中");
            for (index, (label, cancel)) in cancellable.iter().enumerate() {
                if index > 0 {
                    ui.separator();
                }
                ui.horizontal(|ui| {
                    if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                        ui.spinner();
                        ui.label(format!("{} — 中断しています…", label));
                    } else {
                        ui.spinner();
                        ui.label(format!("{} — フレームを読み込み中", label));
                        if ui.button("キャンセル").clicked() {
                            cancel.store(true, std::sync::atomic::Ordering::Relaxed);
                        }
                    }
                });
            }
        });
}
