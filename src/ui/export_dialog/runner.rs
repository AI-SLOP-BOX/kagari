//! Composition export task dispatcher and async renderer starter.

use super::util::{batch_output_path, export_frame_range};
use crate::ui::localization::{tr, tr_format};
use crate::ExportEvent;
use eframe::egui;

/// Spawn the async FFmpeg (or fallback) render worker for one composition.
/// Shared by the export dialog button and the Render Queue batch runner.
pub fn start_comp_export(app: &mut crate::KagariApp, ctx: &egui::Context, comp_id: &str) {
    if app.export.is_exporting {
        return;
    }
    let Some(comp_index) = app
        .history
        .current()
        .compositions
        .iter()
        .position(|composition| composition.id == comp_id)
    else {
        app.toasts
            .error(format!("Queue composition ID not found: {}", comp_id));
        app.render_item_status.insert(
            comp_id.to_string(),
            crate::app_state::QueueItemStatus::Failed,
        );
        return;
    };
    let comp = app.history.current().compositions[comp_index].clone();
    let production_document = app.production_document.clone();
    let comp_total = comp.duration_frames;
    // Range mode: 0 = Entire Comp, 1 = Work Area (falls back to Entire when unset)
    let range_mode =
        ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(egui::Id::new("ae_export_range"), || 0u8));
    let (frame_offset, total_frames) = export_frame_range(
        comp_total,
        range_mode,
        app.playback.work_area_in,
        app.playback.work_area_out,
    );

    app.export.is_exporting = true;
    app.export.export_progress = 0.0;
    app.export.export_comp_id = Some(comp.id.clone());
    app.export.export_comp_name = Some(comp.name.clone());
    app.render_item_status.insert(
        comp.id.clone(),
        crate::app_state::QueueItemStatus::Rendering,
    );

    // Render the same non-destructive audio correction used by preview before
    // muxing. This avoids exporting the untouched source while the preview
    // sounds corrected.
    let include_audio = ctx.data_mut(|d| {
        *d.get_temp_mut_or_insert_with(egui::Id::new("ae_export_include_audio"), || true)
    });
    let has_audio = crate::core::audio_engine::composition_contains_audio_sources(&comp);
    app.export.export_status = Some(if include_audio && has_audio {
        tr_format("Preparing audio for '{}'…", &[&comp.name])
    } else {
        tr_format("Rendering '{}'…", &[&comp.name])
    });
    let codec_idx = app.export_codec_idx;
    let export_fps = app.export.export_fps.clamp(1, 120);
    let res_scale = ctx.data_mut(|d| {
        *d.get_temp_mut_or_insert_with(egui::Id::new("ae_export_res_scale"), || 1.0f32)
    });

    let output_path = if app.batch_queue.len() > 1 {
        batch_output_path(&app.export.export_output_path, &comp.name, &comp.id)
    } else {
        app.export.export_output_path.clone()
    };
    app.last_export_path = Some(output_path.clone());
    // Final output size.
    let render_w = ((comp.width as f32 * res_scale) as u32).max(2);
    let render_h = ((comp.height as f32 * res_scale) as u32).max(2);
    // Supersampling is disabled when either doubled dimension exceeds the rasterizer limit.
    let ssaa =
        ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(egui::Id::new("ae_export_ssaa"), || true));
    let max_dim = crate::core::software_renderer::MAX_RENDER_DIMENSION;
    let ss_active = ssaa && render_w <= max_dim / 2 && render_h <= max_dim / 2;
    let (ss_w, ss_h) = if ss_active {
        (render_w * 2, render_h * 2)
    } else {
        (render_w, render_h)
    };

    // ── PNG Sequence branch: bypass FFmpeg, write numbered PNGs directly ──
    if codec_idx == 3 {
        if let Some(old_flag) = app.export_cancel_flag.take() {
            old_flag.store(true, std::sync::atomic::Ordering::SeqCst);
        }
        let cancel_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        app.export_cancel_flag = Some(cancel_flag.clone());

        let out = std::path::PathBuf::from(&output_path);
        let dir = out
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        let stem = out
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| comp.name.clone());

        let (tx_ui, rx_ui) = std::sync::mpsc::channel();
        app.export.export_rx = Some(rx_ui);
        let export_document = production_document.clone();
        crate::core::ffmpeg_export::start_png_sequence_export(
            dir,
            stem,
            render_w,
            render_h,
            total_frames.max(1),
            frame_offset,
            tx_ui,
            cancel_flag,
            move |frame| {
                let frame_comp = export_document
                    .as_ref()
                    .and_then(|document| document.composition_for_frame(comp_index, frame));
                let render_comp = frame_comp.as_ref().unwrap_or(&comp);
                let px = crate::core::software_renderer::render_frame_to_pixels(
                    render_comp,
                    frame,
                    ss_w,
                    ss_h,
                    0.0,
                    0,
                );
                if ss_active {
                    crate::core::supersample::downsample2x(&px, render_w, render_h)
                } else {
                    px
                }
            },
        );
        log::info!(
            "Spawned PNG sequence export for {}",
            app.export.export_output_path
        );
        return;
    }

    let codec = match codec_idx {
        1 => crate::core::ffmpeg_export::VideoCodec::ProRes422,
        2 => crate::core::ffmpeg_export::VideoCodec::ProRes4444,
        4 => crate::core::ffmpeg_export::VideoCodec::WebP,
        _ => crate::core::ffmpeg_export::VideoCodec::H264,
    };
    let config = crate::core::ffmpeg_export::ExportConfig {
        audio_wav: None,
        output_path: output_path.clone(),
        width: render_w,
        height: render_h,
        fps: export_fps,
        total_frames: total_frames.max(1),
        codec,
    };

    if let Some(old_flag) = app.export_cancel_flag.take() {
        old_flag.store(true, std::sync::atomic::Ordering::SeqCst);
    }
    let cancel_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    app.export_cancel_flag = Some(cancel_flag.clone());

    if crate::core::ffmpeg_export::is_ffmpeg_available() {
        let (tx_ui, rx_ui) = std::sync::mpsc::channel();
        app.export.export_rx = Some(rx_ui);
        let audio_comp = comp.clone();
        let mixer_channels = app.audio_mixer_channels.clone();
        let master_dsp = app.master_dsp_params();
        let audio_end_frame = frame_offset.saturating_add(total_frames);
        let frame_renderer = move |frame: u32| {
            let absolute_frame = frame.saturating_add(frame_offset);
            let frame_comp = production_document
                .as_ref()
                .and_then(|document| document.composition_for_frame(comp_index, absolute_frame));
            let render_comp = frame_comp.as_ref().unwrap_or(&comp);
            let px = crate::core::software_renderer::render_frame_to_pixels(
                render_comp,
                absolute_frame,
                ss_w,
                ss_h,
                0.0,
                0,
            );
            if ss_active {
                crate::core::supersample::downsample2x(&px, render_w, render_h)
            } else {
                px
            }
        };
        std::thread::spawn(move || {
            let mut config = config;
            if include_audio && has_audio {
                let audio_path = match crate::core::audio_engine::mix_composition_to_wav(
                    &audio_comp,
                    frame_offset,
                    audio_end_frame,
                    48_000,
                    Some(&mixer_channels),
                    &master_dsp,
                ) {
                    Ok(Some(path)) => path,
                    Ok(None) => {
                        let _ = tx_ui.send(ExportEvent::Error(tr(
                            "Audio sources were present, but no audio samples were produced",
                        )));
                        return;
                    }
                    Err(error) => {
                        let error = error.to_string();
                        let _ = tx_ui.send(ExportEvent::Error(tr_format(
                            "Unable to prepare the audio mix for export: {}",
                            &[&error],
                        )));
                        return;
                    }
                };
                config.audio_wav = Some(audio_path.to_string_lossy().into_owned());
            }

            let (tx_ff, rx_ff) = std::sync::mpsc::channel();
            let mixed_wav = config.audio_wav.clone();
            if let Err(error) = crate::core::ffmpeg_export::start_export_cancelable(
                config,
                tx_ff,
                cancel_flag,
                frame_renderer,
            ) {
                if let Some(path) = mixed_wav {
                    let _ = std::fs::remove_file(path);
                }
                let _ = tx_ui.send(ExportEvent::Error(error.to_string()));
                return;
            }
            while let Ok(evt) = rx_ff.recv() {
                let mapped = match evt {
                    crate::core::ffmpeg_export::ExportEvent::Progress(p, m) => {
                        ExportEvent::Progress(p, m)
                    }
                    crate::core::ffmpeg_export::ExportEvent::Finished(m) => {
                        ExportEvent::Finished(m)
                    }
                    crate::core::ffmpeg_export::ExportEvent::Error(m) => ExportEvent::Error(m),
                };
                if tx_ui.send(mapped).is_err() {
                    break;
                }
            }
            if let Some(path) = mixed_wav {
                let _ = std::fs::remove_file(path);
            }
        });
    } else {
        // A transform-only preview is not an export. Reporting success here
        // used to leave users with a green toast and no file on disk.
        app.export.is_exporting = false;
        let message = tr("FFmpeg is required for video export. Install it and try again.");
        app.export.export_status = Some(format!("Error: {message}"));
        app.toasts.error(message);
        return;
    }

    log::info!(
        "Spawned async render pipeline for {}",
        app.export.export_output_path
    );
}
