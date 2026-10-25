//! Modal export dialog UI rendering and user interactions.

use super::runner::start_comp_export;
use super::util::{export_frame_range, localized_export_status};
use crate::ui::localization::{tr, tr_format};
use crate::ui::theme::colors;
use crate::ExportEvent;
use eframe::egui;

pub fn draw(app: &mut crate::KagariApp, ctx: &egui::Context) {
    // ── Non-blocking Channel Event Receiver ──
    let mut finished_export = false;
    let mut next_batch_comp: Option<String> = None;
    let mut export_channel_disconnected = false;
    if let Some(ref rx) = app.export.export_rx {
        loop {
            let event = match rx.try_recv() {
                Ok(event) => event,
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    export_channel_disconnected = true;
                    break;
                }
            };
            match event {
                ExportEvent::Progress(prog, msg) => {
                    app.export.export_progress = prog;
                    app.export.export_status = Some(msg);
                }
                ExportEvent::Finished(msg) => {
                    app.export.export_progress = 1.0;
                    app.export.export_status = Some(msg.clone());
                    app.export.is_exporting = false;
                    finished_export = true;
                    if let Some(id) = app.export.export_comp_id.clone() {
                        app.render_item_status
                            .insert(id, crate::app_state::QueueItemStatus::Done);
                    }
                    app.toasts.info(tr_format("Export complete: {}", &[&msg]));
                    // ── Batch runner: advance to the next queued comp ──
                    app.batch_idx += 1;
                    if app.batch_idx < app.batch_queue.len() {
                        next_batch_comp = Some(app.batch_queue[app.batch_idx].clone());
                    } else if !app.batch_queue.is_empty() {
                        app.toasts
                            .info(tr("Batch render complete: all queue items exported"));
                        app.batch_queue.clear();
                        app.batch_idx = 0;
                    }
                }
                ExportEvent::Error(msg) => {
                    app.export.export_status = Some(format!("Error: {}", msg));
                    app.export.is_exporting = false;
                    finished_export = true;
                    if let Some(id) = app.export.export_comp_id.clone() {
                        app.render_item_status
                            .insert(id, crate::app_state::QueueItemStatus::Failed);
                    }
                    app.toasts.error(tr_format("Export failed: {}", &[&msg]));
                    // A failed item aborts the batch to avoid cascading failures
                    if !app.batch_queue.is_empty() {
                        app.toasts.error(tr("Batch render aborted"));
                        app.batch_queue.clear();
                        app.batch_idx = 0;
                    }
                }
            }
        }
    }

    if export_channel_disconnected && app.export.is_exporting {
        let message = tr("Export worker stopped unexpectedly before reporting a result");
        app.export.export_status = Some(format!("Error: {}", message));
        app.export.is_exporting = false;
        finished_export = true;
        if let Some(id) = app.export.export_comp_id.clone() {
            app.render_item_status
                .insert(id, crate::app_state::QueueItemStatus::Failed);
        }
        app.toasts.error(message);
        if !app.batch_queue.is_empty() {
            app.toasts.error(tr("Batch render aborted"));
            app.batch_queue.clear();
            app.batch_idx = 0;
        }
    }

    if finished_export {
        app.export.export_rx = None;
    }
    if let Some(name) = next_batch_comp {
        start_comp_export(app, ctx, &name);
    }

    if !app.export.show_export_dialog {
        return;
    }

    let mut open = app.export.show_export_dialog;
    let screen_h = ctx.screen_rect().height();
    let max_dialog_h = (screen_h * 0.82).clamp(320.0, 700.0);
    crate::ui::modal::dialog(tr("Export Composition Video"))
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .default_width(480.0)
        .default_height(max_dialog_h)
        .max_size(egui::vec2(600.0, max_dialog_h))
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.heading(tr("Render Settings"));
            ui.separator();

            let scroll_h = (ui.available_height() - 56.0).max(180.0);
            egui::ScrollArea::vertical()
                .max_height(scroll_h)
                .auto_shrink([false, false])
                .show(ui, |ui| {

            let comp = app.history.current().active_composition();
            let comp_total = comp.duration_frames;
            // Display range length when Work Area mode is active.
            let range_mode = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(egui::Id::new("ae_export_range"), || 0u8));
            let (_, shown_total) = export_frame_range(
                comp_total,
                range_mode,
                app.playback.work_area_in,
                app.playback.work_area_out,
            );

            ui.label(tr_format("Composition: {} × {}", &[&comp.width.to_string(), &comp.height.to_string()]));
            ui.label(tr_format("Total Duration: {} frames", &[&shown_total.to_string()]));

            ui.add_space(8.0);

            // ── User Export Presets ──
            {
                let user_presets = crate::core::export_presets::load_user_presets();
                if !user_presets.is_empty() {
                    ui.horizontal(|ui| {
                        ui.label(tr("User Preset:"));
                        egui::ComboBox::from_id_salt("export_user_preset")
                            .selected_text(tr("Select preset..."))
                            .show_ui(ui, |ui| {
                                for preset in &user_presets {
                                    if ui.selectable_label(false, &preset.name).clicked() {
                                        app.export_format_preset = match preset.format {
                                            crate::core::export_presets::ExportFormat::Mp4 => 0,
                                            crate::core::export_presets::ExportFormat::ProRes => 1,
                                            crate::core::export_presets::ExportFormat::PngSequence => 2,
                                            _ => 0,
                                        };
                                        app.export_codec_idx = match app.export_format_preset {
                                            1 => 1,
                                            2 => 3,
                                            _ => 0,
                                        };
                                        app.export.export_fps = preset.fps;
                                        app.export_resolution_scale = if preset.width >= 1920 { 0 } else if preset.width >= 1280 { 1 } else { 2 };
                                        let preset_scale = match app.export_resolution_scale {
                                            0 => 1.0,
                                            1 => 0.5,
                                            _ => 0.25,
                                        };
                                        ctx.data_mut(|d| d.insert_temp(egui::Id::new("ae_export_res_scale"), preset_scale));
                                    }
                                }
                            });
                    });
                }
            }

            ui.horizontal(|ui| {
                ui.label(tr("Format Preset:"));
                egui::ComboBox::from_id_salt("export_fmt_combo")
                    .selected_text(match app.export_format_preset {
                        0 => tr("H.264 / MP4 (Standard)"),
                        1 => tr("Apple ProRes 422 HQ (MOV)"),
                        2 => tr("PNG Image Sequence"),
                        _ => tr("Lottie / Bodymovin (.json)"),
                    })
                    .show_ui(ui, |ui| {
                        if ui.selectable_value(&mut app.export_format_preset, 0, tr("H.264 / MP4 (Standard)")).clicked() {
                            app.export_codec_idx = 0;
                        }
                        if ui.selectable_value(&mut app.export_format_preset, 1, tr("Apple ProRes 422 HQ (MOV)")).clicked() {
                            app.export_codec_idx = 1;
                        }
                        if ui.selectable_value(&mut app.export_format_preset, 2, tr("PNG Image Sequence")).clicked() {
                            app.export_codec_idx = 3;
                        }
                        ui.selectable_value(&mut app.export_format_preset, 3, tr("Lottie / Bodymovin (.json)"));
                    });
            });

            ui.horizontal(|ui| {
                ui.label(tr("Render Resolution Scale:"));
                let scale_id = egui::Id::new("ae_export_res_scale");
                let mut scale: f32 = ctx.data_mut(|d| {
                    *d.get_temp_mut_or_insert_with(scale_id, || match app.export_resolution_scale {
                        0 => 1.0,
                        1 => 0.5,
                        _ => 0.25,
                    })
                });
                egui::ComboBox::from_id_salt("export_scale_combo")
                    .selected_text(match scale {
                        s if (s - 1.0).abs() < f32::EPSILON => tr("100% Full Resolution"),
                        s if (s - 0.5).abs() < f32::EPSILON => tr("50% Half Resolution"),
                        _ => tr("25% Quarter Resolution"),
                    })
                    .show_ui(ui, |ui| {
                        if ui.selectable_value(&mut scale, 1.0, tr("100% Full Resolution")).clicked() {
                            app.export_resolution_scale = 0;
                        }
                        if ui.selectable_value(&mut scale, 0.5, tr("50% Half Resolution")).clicked() {
                            app.export_resolution_scale = 1;
                        }
                        if ui.selectable_value(&mut scale, 0.25, tr("25% Quarter Resolution")).clicked() {
                            app.export_resolution_scale = 2;
                        }
                    });
                ctx.data_mut(|d| d.insert_temp(scale_id, scale));
            });

            ui.horizontal(|ui| {
                ui.label(tr("Target FPS:"));
                ui.add(egui::DragValue::new(&mut app.export.export_fps).range(1..=120));
            });

            ui.horizontal(|ui| {
                ui.label(tr("Output Path:"));
                ui.text_edit_singleline(&mut app.export.export_output_path);
                if ui.button(tr("Browse...")).clicked() {
                    let mut dialog = rfd::FileDialog::new();
                    let current_p = std::path::Path::new(&app.export.export_output_path);
                    if let Some(parent) = current_p.parent().filter(|p| p.is_dir()) {
                        dialog = dialog.set_directory(parent);
                    }
                    if let Some(file_name) = current_p.file_name().and_then(|n| n.to_str()) {
                        dialog = dialog.set_file_name(file_name);
                    }
                    if let Some(picked) = dialog.save_file() {
                        app.export.export_output_path = picked.to_string_lossy().to_string();
                    }
                }
            });
            let abs_path = std::path::Path::new(&app.export.export_output_path);
            let display_dir = if abs_path.is_absolute() {
                abs_path.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default()
            } else if let Ok(cwd) = std::env::current_dir() {
                cwd.to_string_lossy().to_string()
            } else {
                String::new()
            };
            if !display_dir.is_empty() {
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(format!("📁 {}", display_dir))
                            .small()
                            .color(colors::TEXT_MUTED),
                    );
                });
            }

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button(tr("Save Current as Preset")).clicked() {
                    let preset = crate::core::export_presets::ExportPreset {
                        name: format!("{} {}", tr("Custom"), { let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(); format!("{}", t) }),
                        format: match app.export_format_preset {
                            1 => crate::core::export_presets::ExportFormat::ProRes,
                            2 => crate::core::export_presets::ExportFormat::PngSequence,
                            _ => crate::core::export_presets::ExportFormat::Mp4,
                        },
                        width: comp.width,
                        height: comp.height,
                        fps: app.export.export_fps,
                        quality: 85,
                        codec: "h264".into(),
                        audio_enabled: true,
                        audio_bitrate: 192,
                    };
                    let _ = crate::core::export_presets::add_user_preset(preset);
                }
            });

            ui.add_space(10.0);
            if app.export.is_exporting {
                ctx.request_repaint_after(std::time::Duration::from_millis(100)); // Throttled: progress bar needs ~10fps, not 60+
            ui.label(tr("Rendering composition in background thread..."));
                ui.add(egui::ProgressBar::new(app.export.export_progress).show_percentage());
                if let Some(ref status) = app.export.export_status {
                ui.weak(localized_export_status(status));
                }
                if ui.button(tr("Cancel Export")).clicked() {
                    if let Some(flag) = app.export_cancel_flag.take() {
                        flag.store(true, std::sync::atomic::Ordering::SeqCst);
                    }
                    app.export.export_rx = None;
                    app.export.is_exporting = false;
                    app.export.export_status = Some(tr("Export cancelled"));
                    app.toasts.info(tr("Export cancelled"));
                }
            } else {
                if let Some(ref status) = app.export.export_status {
                    let is_error = status.starts_with("Error:");
                    let color = if is_error { egui::Color32::RED } else { egui::Color32::GREEN };
                    ui.label(egui::RichText::new(localized_export_status(status)).color(color));
                    ui.add_space(4.0);
                    if !is_error {
                        if let Some(lp) = &app.last_export_path {
                            ui.horizontal(|ui| {
                                if ui.small_button(tr("Show in Folder")).clicked() {
                                    crate::ui::project_io::reveal_in_file_manager(&std::path::PathBuf::from(lp));
                                }
                            });
                        }
                    }
                }
                // Include-audio toggle (only meaningful for MP4)
                let audio_toggle_id = egui::Id::new("ae_export_include_audio");
                let mut include_audio = ctx.data_mut(|d| {
                    *d.get_temp_mut_or_insert_with(audio_toggle_id, || true)
                });
                let has_wav = app.history.current().active_composition().layers.iter().any(|l| {
                    matches!(&l.layer_type, crate::core::timeline::LayerType::Video { audio_wav: Some(_), .. })
                });
                if !has_wav {
                    include_audio = false;
                }
                ui.horizontal(|ui| {
                    let resp = ui.add_enabled(
                        has_wav,
                        egui::Checkbox::new(&mut include_audio, tr("Include Audio")),
                    );
                    let resp = if has_wav {
                        resp
                    } else {
                        resp.on_disabled_hover_text(tr("No audio source: import a video with sound first"))
                    };
                    if resp.changed() {
                        ctx.data_mut(|d| d.insert_temp(audio_toggle_id, include_audio));
                    }
                });
                ui.separator();

                // Estimated file size
                {
                    let bitrate_mbps = match app.export_codec_idx {
                        1 => 147.0, // ProRes 422
                        2 => 330.0, // ProRes 4444
                        4 => 5.0,   // WebP Animation
                        _ => 10.0,   // H.264
                    };
                    let duration_sec = shown_total as f32 / app.export.export_fps.max(1) as f32;
                    let est_mb = bitrate_mbps * duration_sec / 8.0;
                    let size_text = if est_mb > 1024.0 {
                        format!("{:.1} GB", est_mb / 1024.0)
                    } else {
                        format!("{:.0} MB", est_mb)
                    };
                    ui.label(egui::RichText::new(tr_format("Estimated file size: ~{}", &[&size_text]))
                        .small()
                        .color(colors::TEXT_SECONDARY));
                }

                // Resolution scale
                let res_scale_id = egui::Id::new("ae_export_res_scale");
                let mut res_scale = ctx.data_mut(|d| {
                    *d.get_temp_mut_or_insert_with(res_scale_id, || 1.0f32)
                });
                ui.horizontal(|ui| {
                    ui.label(tr("Resolution:"));
                    let orig_w = comp.width;
                    let orig_h = comp.height;
                    for (label, scale) in [("Full", 1.0), ("Half", 0.5), ("Third", 1.0/3.0), ("Quarter", 0.25)] {
                        if ui.selectable_label((res_scale - scale).abs() < 0.01, tr(label)).clicked() {
                            res_scale = scale;
                            ctx.data_mut(|d| d.insert_temp(res_scale_id, res_scale));
                        }
                        ui.separator();
                    }
                    if res_scale != 1.0 {
                        ui.label(format!("{}x{}", (orig_w as f32 * res_scale) as u32, (orig_h as f32 * res_scale) as u32));
                    }
                });

                // Codec selection
                ui.horizontal(|ui| {
                    ui.label(tr("Video Codec:"));
                    ui.selectable_value(&mut app.export_codec_idx, 0, "H.264");
                    ui.selectable_value(&mut app.export_codec_idx, 1, "ProRes 422");
                    ui.selectable_value(&mut app.export_codec_idx, 2, "ProRes 4444");
                    ui.selectable_value(&mut app.export_codec_idx, 3, tr("PNG Sequence"));
                    ui.selectable_value(&mut app.export_codec_idx, 4, tr("WebP Animation (.webp)"));
                });
                // ── Quality: supersampled anti-aliasing ──
                ui.horizontal(|ui| {
                    let mut ssaa_now = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(egui::Id::new("ae_export_ssaa"), || true));
                    if ui.checkbox(&mut ssaa_now, tr("High Quality (2× SSAA)")).on_hover_text(tr("Renders at double resolution and downsamples — cleaner edges, ~4× slower")).changed() {
                        ctx.data_mut(|d| d.insert_temp(egui::Id::new("ae_export_ssaa"), ssaa_now));
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(tr("Range:"));
                    let mut range_mode = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(egui::Id::new("ae_export_range"), || 0u8));
                    let has_wa = app.playback.work_area_in.is_some() || app.playback.work_area_out.is_some();
                    let mut changed = ui.radio_value(&mut range_mode, 0u8, tr("Entire Comp")).changed();
                    changed |= ui.add_enabled(
                        has_wa,
                        egui::RadioButton::new(range_mode == 1, tr("Work Area")),
                    ).on_disabled_hover_text(tr("Set a work area first (B / N)")).changed();
                    if !has_wa && range_mode == 1 {
                        range_mode = 0;
                    }
                    if changed {
                        ctx.data_mut(|d| d.insert_temp(egui::Id::new("ae_export_range"), range_mode));
                    }
                });
            }
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                let active_comp = app.history.current().active_composition();
                let active_comp_id = active_comp.id.clone();
                let active_comp_name = active_comp.name.clone();
                if app.export_format_preset == 3 {
                        // Lottie: synchronous JSON write — fast, no ffmpeg required.
                        if ui.button(tr("Export Lottie JSON")).clicked() {
                            let project = app.history.current().clone();
                            match crate::core::lottie_exporter::export_project_to_json(&project) {
                                Err(error) => {
                                    let error = error.to_string();
                                    let message = tr_format("Lottie export failed: {}", &[&error]);
                                    app.export.export_status = Some(format!("Error: {message}"));
                                    app.toasts.error(message);
                                }
                                Ok(json) => {
                                    // The Lottie format cannot carry effects; warn instead of losing them silently.
                                    let effect_count: usize = project
                                        .compositions
                                        .iter()
                                        .flat_map(|c| c.layers.iter())
                                        .map(|layer| {
                                            layer.effects.iter().filter(|effect| effect.enabled).count()
                                        })
                                        .sum();
                                    let mut path = app.export.export_output_path.trim().to_string();
                                    if path.is_empty() {
                                        path = format!(
                                            "{}_lottie",
                                            active_comp_name.replace(' ', "_")
                                        );
                                    }
                                    if !path.ends_with(".json") {
                                        path.push_str(".json");
                                    }
                                    match std::fs::write(&path, &json) {
                                        Ok(()) => {
                                            app.export.export_status = Some(format!("Lottie export complete → {}", path));
                                            let path = path.to_string();
                                            let bytes = json.len().to_string();
                                            app.toasts.info(tr_format("Lottie exported → {} ({} bytes)", &[&path, &bytes]));
                                            if effect_count > 0 {
                                                let effect_count = effect_count.to_string();
                                                app.toasts.warning(tr_format(
                                                    "{} enabled effect(s) are NOT part of Lottie output (format limitation)",
                                                    &[&effect_count],
                                                ));
                                            }
                                        }
                                        Err(error) => {
                                            let error = error.to_string();
                                            let message = tr_format("Lottie export failed: {}", &[&error]);
                                            app.export.export_status = Some(format!("Error: {message}"));
                                            app.toasts.error(message);
                                        }
                                    }
                                }
                            }
                        }
                    } else if ui.button(tr("Start Async Render")).clicked() {
                        start_comp_export(app, ctx, &active_comp_id);
                    }
                    if ui.button(tr("Close")).clicked() {
                        app.export.show_export_dialog = false;
                    }
                });
        });

    app.export.show_export_dialog = open;
}
