//! Drag-and-drop media import: images become Image layers, videos spawn the
//! ffmpeg frame-extraction pipeline on a worker thread, WAVs become Audio
//! layers. Results stream back through a channel and are inserted above the
//! current selection.
use crate::KagariApp;
use eframe::egui;

pub enum ImportResult {
    Video(crate::core::video_import::VideoAsset, String),
    Err(String),
}

pub fn start_video_import(
    app: &mut KagariApp,
    ctx: &egui::Context,
    path: std::path::PathBuf,
    name: String,
) {
    if !crate::core::video_import::ffmpeg_available() {
        app.toasts.error("Video import needs ffmpeg on PATH");
        return;
    }
    let fps = app.history.current().active_composition().fps as f32;
    let dest = std::env::temp_dir().join("kagari_media").join(&name);
    let source = path.to_string_lossy().into_owned();
    let (tx, rx) = std::sync::mpsc::channel();
    app.import_rx.push(rx);
    app.toasts.info(format!("Extracting frames from '{}'…", name));
    ctx.request_repaint();
    std::thread::spawn(move || {
        let result = crate::core::video_import::import_video(&source, &dest, fps)
            .map(|asset| ImportResult::Video(asset, name))
            .unwrap_or_else(|error| ImportResult::Err(error.to_string()));
        let _ = tx.send(result);
    });
}

fn insert_layer(app: &mut KagariApp, layer: crate::core::timeline::Layer, label: &str) {
    let comp_dur = app.history.current().active_composition().duration_frames;
    let insert_at = app.selection.selected_layer_idx.map(|i| i + 1).unwrap_or(0);
    let mut l = layer;
    l.in_frame = l.in_frame.min(comp_dur.saturating_sub(1));
    l.out_frame = comp_dur.max(l.in_frame + 1);
    app.modify_project(|proj| {
        let comp = proj.active_composition_mut();
        let at = insert_at.min(comp.layers.len());
        comp.layers.insert(at, l);
    });
    app.toasts.info(format!("Imported {}", label));
}

pub fn handle_dropped_files(app: &mut KagariApp, ctx: &egui::Context) {
    // Drain completed workers before processing new drops; each import owns a
    // channel so concurrent imports cannot overwrite one another.
    let receivers = std::mem::take(&mut app.import_rx);
    for rx in receivers {
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
                ImportResult::Video(asset, name) => {
                    let dur = asset.frame_count.max(1);
                    let (comp_w, comp_h) = {
                        let comp = app.history.current().active_composition();
                        (comp.width as f32, comp.height as f32)
                    };
                    let fit = ((comp_w / asset.width.max(1) as f32)
                        .min(comp_h / asset.height.max(1) as f32))
                    .min(1.0)
                        * 100.0;
                    let mut layer = crate::core::timeline::Layer::new(
                        app.history
                            .current()
                            .active_composition()
                            .next_layer_id("video"),
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
                    insert_layer(app, layer, &format!("video '{}'", name));
                }
                ImportResult::Err(e) => {
                    app.toasts.error(format!("Import failed: {}", e));
                }
            }
        }
        if disconnected && !received_result {
            app.toasts.error("Video import worker stopped unexpectedly.");
        }
        if !disconnected {
            app.import_rx.push(rx);
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
                        insert_layer(app, layer, &format!("image '{}' ({}×{})", name, iw, ih));
                    }
                    Err(e) => app.toasts.error(format!("Cannot read {}: {}", name, e)),
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
                insert_layer(app, layer, &format!("audio '{}'", name));
            }
            "mp4" | "mov" | "mkv" | "avi" | "webm" | "m4v" | "mpeg" | "mpg" | "ts" | "m2ts"
            | "av1" | "ivf" => {
                start_video_import(app, ctx, path, name);
            }
            other => {
                app.toasts
                    .error(format!("Unsupported file type: .{}", other));
            }
        }
    }
}
