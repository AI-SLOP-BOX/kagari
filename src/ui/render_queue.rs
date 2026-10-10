use eframe::egui;
use std::sync::Mutex;
use std::time::Instant;

use crate::ui::custom_widgets;
use crate::ui::localization::{tr, tr_format};
use crate::ui::theme::colors;
use crate::KagariApp;

const START_TIME_ID: &str = "render_queue_export_start";
const CANCEL_CONFIRM_ID: &str = "render_queue_cancel_confirm";

fn preset_name(preset: usize) -> &'static str {
    match preset {
        1 => "Apple ProRes 422 HQ (MOV)",
        2 => "PNG Image Sequence",
        _ => "H.264 High Bitrate (MP4)",
    }
}

fn format_duration(secs: f64) -> String {
    if !secs.is_finite() || secs <= 0.0 {
        return "--:--".to_string();
    }
    let total = secs.round() as u64;
    format!("{:02}:{:02}", total / 60, total % 60)
}

pub fn draw_render_queue_panel(app: &mut KagariApp, ui: &mut egui::Ui) {
    let comp = app.history.current().active_composition();
    let comp_id = comp.id.clone();
    let comp_name = comp.name.clone();

    ui.horizontal_wrapped(|ui| {
        if custom_widgets::ae_button_accent(ui, &tr("Render All Queue (Cmd+M)"))
            .on_hover_text(tr("Sequentially export every queued composition to video"))
            .clicked()
        {
            if app.render_queue_items.is_empty() {
                app.render_queue_items.push(comp_id.clone());
                app.render_item_status
                    .insert(comp_id.clone(), crate::app_state::QueueItemStatus::Queued);
            }
            // Sequential background batch: each item runs the real FFmpeg
            // export; the next starts automatically when the previous finishes.
            app.batch_queue = app.render_queue_items.clone();
            app.batch_idx = 0;
            app.export.show_export_dialog = true;
            let first = app.batch_queue[0].clone();
            crate::ui::export_dialog::start_comp_export(app, ui.ctx(), &first);
        }
        if custom_widgets::ae_button(ui, &tr("+ Add Active Comp"))
            .on_hover_text(tr("Add the active composition to the render queue"))
            .clicked()
            && !app.render_queue_items.contains(&comp_id)
        {
            app.render_queue_items.push(comp_id.clone());
            app.render_item_status
                .insert(comp_id.clone(), crate::app_state::QueueItemStatus::Queued);
        }
        if custom_widgets::ae_button(ui, &tr("Clear Queue"))
            .on_hover_text(tr("Remove all items from the queue"))
            .clicked()
            && !app.export.is_exporting
        {
            app.render_queue_items.clear();
            app.render_item_status.clear();
        }

        let failed_items: Vec<String> = app
            .render_queue_items
            .iter()
            .filter(|composition_id| {
                matches!(
                    app.render_item_status.get(*composition_id),
                    Some(crate::app_state::QueueItemStatus::Failed)
                )
            })
            .cloned()
            .collect();
        if custom_widgets::ae_button(ui, &tr("↻ Retry Failed"))
            .on_hover_text(tr("Requeue failed items and render them sequentially"))
            .clicked()
            && !failed_items.is_empty()
            && !app.export.is_exporting
        {
            for composition_id in &failed_items {
                app.render_item_status.insert(
                    composition_id.clone(),
                    crate::app_state::QueueItemStatus::Queued,
                );
            }
            app.batch_queue = failed_items;
            app.batch_idx = 0;
            app.export.show_export_dialog = true;
            if let Some(first) = app.batch_queue.first().cloned() {
                crate::ui::export_dialog::start_comp_export(app, ui.ctx(), &first);
            }
        }

        if custom_widgets::ae_button(ui, &tr("Export Farm Job Manifest"))
            .on_hover_text(tr(
                "Write the current render queue as a Deadline/OpenCue-compatible JSON job",
            ))
            .clicked()
        {
            let default_name = format!("{}_farm_job.json", comp_name.replace(['/', '\\'], "_"));
            let path = rfd::FileDialog::new()
                .add_filter(tr("Render job manifest"), &["json"])
                .set_file_name(default_name)
                .save_file();
            if let Some(path) = path {
                let items: Vec<serde_json::Value> = app
                    .render_queue_items
                    .iter()
                    .map(|composition_id| {
                        let name = app
                            .history
                            .current()
                            .compositions
                            .iter()
                            .find(|composition| composition.id == *composition_id)
                            .map(|composition| composition.name.as_str())
                            .unwrap_or("Missing composition");
                        let output = if app.render_queue_items.len() > 1 {
                            crate::ui::export_dialog::batch_output_path(
                                &app.export.export_output_path,
                                name,
                                composition_id,
                            )
                        } else {
                            app.export.export_output_path.clone()
                        };
                        serde_json::json!({
                            "composition": name,
                            "composition_id": composition_id,
                            "status": "queued",
                            "output": output,
                            "format": preset_name(app.export_format_preset),
                            "start_frame": app.playback.work_area_in.unwrap_or(0),
                            "end_frame": app.playback.work_area_out,
                            "chunk_size": 25,
                            "priority": 50,
                        })
                    })
                    .collect();
                let manifest = serde_json::json!({
                    "schema": "kagari.render-job/v1",
                    "application": "kagari-vfx",
                    "adapter": "deadline-or-opencue",
                    "job_name": comp_name,
                    "items": items,
                });
                match serde_json::to_string_pretty(&manifest)
                    .map_err(|error| error.to_string())
                    .and_then(|json| std::fs::write(&path, json).map_err(|error| error.to_string()))
                {
                    Ok(()) => {
                        crate::ui::project_io::reveal_in_file_manager(&path);
                        app.toasts.info(tr_format(
                            "Farm job manifest exported: {}",
                            &[&path.display().to_string()],
                        ));
                    }
                    Err(error) => app.toasts.error(tr_format(
                        "Could not write farm job manifest: {}",
                        &[&error],
                    )),
                }
            }
        }
    });

    ui.add_space(8.0);

    // Queue Items List Display
    if !app.render_queue_items.is_empty() {
        let (done, rendering, failed) = app.render_queue_items.iter().fold(
            (0, 0, 0),
            |(done, rendering, failed), composition_id| match app
                .render_item_status
                .get(composition_id)
                .copied()
                .unwrap_or(crate::app_state::QueueItemStatus::Queued)
            {
                crate::app_state::QueueItemStatus::Done => (done + 1, rendering, failed),
                crate::app_state::QueueItemStatus::Rendering => (done, rendering + 1, failed),
                crate::app_state::QueueItemStatus::Failed => (done, rendering, failed + 1),
                crate::app_state::QueueItemStatus::Queued => (done, rendering, failed),
            },
        );
        let queued = app.render_queue_items.len() - done - rendering - failed;
        ui.label(
            egui::RichText::new(tr_format(
                "QUEUED COMPOSITIONS ({}) — {} done · {} rendering · {} queued · {} failed",
                &[
                    &app.render_queue_items.len().to_string(),
                    &done.to_string(),
                    &rendering.to_string(),
                    &queued.to_string(),
                    &failed.to_string(),
                ],
            ))
            .small()
            .strong()
            .color(colors::ACCENT_CYAN),
        );
        let mut remove_idx: Option<usize> = None;
        let mut switch_comp_id: Option<String> = None;

        egui::ScrollArea::vertical()
            .max_height(100.0)
            .show(ui, |ui| {
                for (idx, q_id) in app.render_queue_items.iter().enumerate() {
                    let is_active = q_id == &comp_id;
                    let q_name = app
                        .history
                        .current()
                        .compositions
                        .iter()
                        .find(|composition| composition.id == *q_id)
                        .map(|composition| composition.name.as_str())
                        .unwrap_or("Missing composition");
                    let (status_text, status_color) = match app
                        .render_item_status
                        .get(q_id)
                        .copied()
                        .unwrap_or(crate::app_state::QueueItemStatus::Queued)
                    {
                        crate::app_state::QueueItemStatus::Queued => ("Queued", colors::TEXT_MUTED),
                        crate::app_state::QueueItemStatus::Rendering => {
                            ("Rendering", colors::ACCENT_GREEN)
                        }
                        crate::app_state::QueueItemStatus::Done => ("Done", colors::ACCENT_CYAN),
                        crate::app_state::QueueItemStatus::Failed => ("Failed", colors::ACCENT_RED),
                    };
                    ui.horizontal(|ui| {
                        let display_name = if q_name == "Missing composition" {
                            tr(q_name)
                        } else {
                            q_name.to_owned()
                        };
                        let text = format!("{}. {}", idx + 1, display_name);
                        let label = if is_active {
                            egui::RichText::new(text)
                                .strong()
                                .color(colors::ACCENT_GREEN)
                        } else {
                            egui::RichText::new(text).color(colors::TEXT_PRIMARY)
                        };
                        if ui.selectable_label(is_active, label).clicked() && !is_active {
                            switch_comp_id = Some(q_id.clone());
                        }
                        ui.label(
                            egui::RichText::new(tr(status_text))
                                .small()
                                .color(status_color),
                        );
                        if !app.export.is_exporting && ui.small_button("✖").clicked() {
                            remove_idx = Some(idx);
                        }
                    });
                }
            });

        if let Some(idx) = remove_idx {
            if idx < app.render_queue_items.len() {
                let removed = app.render_queue_items.remove(idx);
                app.render_item_status.remove(&removed);
            }
        }
        if let Some(composition_id) = switch_comp_id {
            let mut project = app.history.current().clone();
            if let Some(pos) = project
                .compositions
                .iter()
                .position(|composition| composition.id == composition_id)
            {
                project.active_composition_idx = pos;
                app.commit_project(project);
                app.selection.selected_layer_idx = None;
                app.selection.selected_layers.clear();
            }
        }

        ui.add_space(6.0);
    }

    // Frame range derived from the work area when one is set.
    let comp_duration = app.history.current().active_composition().duration_frames;
    let wa_in = app.playback.work_area_in.unwrap_or(0);
    let wa_out = app
        .playback
        .work_area_out
        .unwrap_or(comp_duration.saturating_sub(1))
        .min(comp_duration.saturating_sub(1));
    let range_frames = wa_out.saturating_sub(wa_in).saturating_add(1);

    // Editable output path persisted across frames via egui's temp store.
    let path_id = egui::Id::new("render_queue_output_path");

    let frame = egui::Frame::none()
        .fill(colors::BG_DARK)
        .inner_margin(egui::Margin::same(8.0))
        .stroke(egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE));

    frame.show(ui, |ui| {
        let (status_text, status_color) = if app.export.is_exporting {
            ("Rendering", colors::ACCENT_GREEN)
        } else {
            ("Queued", egui::Color32::YELLOW)
        };

        ui.horizontal(|ui| {
            let queue_pos = app
                .render_queue_items
                .iter()
                .position(|id| id == &comp_id)
                .map(|p| p + 1)
                .unwrap_or(app.render_queue_items.len().max(1));
            ui.label(
                egui::RichText::new(tr_format("Item {}", &[&queue_pos.to_string()]))
                    .strong()
                    .color(colors::TEXT_ACCENT),
            );
            ui.label(tr_format("Comp: {}", &[&comp_name]));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(tr_format("Status: {}", &[&tr(status_text)]))
                        .strong()
                        .color(status_color),
                );
            });
        });

        ui.separator();

        egui::Grid::new("render_queue_grid")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label(tr("Render Settings:"));
                ui.label(
                    egui::RichText::new(tr("Best Quality / Full Resolution"))
                        .color(egui::Color32::WHITE),
                );
                ui.end_row();

                ui.label(tr("Output Module:"));
                ui.label(
                    egui::RichText::new(tr(preset_name(app.export_format_preset)))
                        .color(egui::Color32::WHITE),
                );
                ui.end_row();

                ui.label(tr("Output To:"));
                {
                    let mut edit = ui.ctx().data_mut(|d| {
                        d.get_temp_mut_or_insert_with(path_id, || {
                            std::sync::Arc::new(Mutex::new(app.export.export_output_path.clone()))
                        })
                        .lock()
                        .map(|g| g.clone())
                        .unwrap_or_default()
                    });
                    let resp = ui.add_sized(
                        [ui.available_width().max(120.0), 18.0],
                        egui::TextEdit::singleline(&mut edit),
                    );
                    if resp.changed() || resp.lost_focus() {
                        ui.ctx().data_mut(|d| {
                            let arc = d.get_temp_mut_or_insert_with(path_id, || {
                                std::sync::Arc::new(Mutex::new(edit.clone()))
                            });
                            if let Ok(mut g) = arc.lock() {
                                *g = edit.clone();
                            }
                        });
                        app.export.export_output_path = edit;
                    }
                }
                ui.end_row();

                ui.label(tr("Frame Range:"));
                ui.horizontal(|ui| {
                    let mut in_v = wa_in as i32;
                    let mut out_v = wa_out as i32;
                    let comp_fps = app.history.current().active_composition().fps.max(1);
                    let mut changed = false;
                    if ui
                        .add(
                            egui::DragValue::new(&mut in_v)
                                .prefix(tr("In: "))
                                .speed(0.5)
                                .range(0..=(comp_duration.saturating_sub(1) as i32)),
                        )
                        .changed()
                    {
                        changed = true;
                    }
                    if ui
                        .add(
                            egui::DragValue::new(&mut out_v)
                                .prefix(tr("Out: "))
                                .speed(0.5)
                                .range(in_v..=(comp_duration.saturating_sub(1) as i32)),
                        )
                        .changed()
                    {
                        changed = true;
                    }
                    ui.label(
                        egui::RichText::new(tr_format(
                            "({} frames @ {} fps, {}s)",
                            &[
                                &range_frames.to_string(),
                                &comp_fps.to_string(),
                                &format!("{:.2}", range_frames as f64 / comp_fps as f64),
                            ],
                        ))
                        .color(egui::Color32::GRAY)
                        .small(),
                    );
                    if changed {
                        app.playback.work_area_in = Some(in_v.max(0) as u32);
                        app.playback.work_area_out = Some(out_v.max(in_v.max(0)) as u32);
                    }
                });
                ui.end_row();
            });

        ui.add_space(6.0);

        if app.export.is_exporting {
            // Track elapsed time for ETA estimation.
            let start: Instant = ui.ctx().data_mut(|d| {
                *d.get_temp_mut_or_insert_with(egui::Id::new(START_TIME_ID), Instant::now)
            });

            let progress = app.export.export_progress.clamp(0.0, 1.0);
            let elapsed = start.elapsed().as_secs_f64();
            let eta_secs = if progress > 0.01 {
                elapsed / progress as f64 * (1.0 - progress as f64)
            } else {
                f64::INFINITY
            };

            ui.add(egui::ProgressBar::new(progress).text(tr_format(
                "Rendering… {}%",
                &[&format!("{:.0}", progress * 100.0)],
            )));
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(tr_format("Elapsed: {}", &[&format_duration(elapsed)]))
                        .small()
                        .color(egui::Color32::GRAY),
                );
                ui.label(
                    egui::RichText::new(tr_format(
                        "Remaining (est.): {}",
                        &[&format_duration(eta_secs)],
                    ))
                    .small()
                    .color(egui::Color32::GRAY),
                );
                if let Some(status) = &app.export.export_status {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(status)
                                .small()
                                .color(colors::HUD_STATUS_TEXT),
                        );
                    });
                }
            });

            // Cancel with a lightweight confirmation step.
            let confirming = ui
                .ctx()
                .data_mut(|d| *d.get_temp_mut_or_default::<bool>(egui::Id::new(CANCEL_CONFIRM_ID)));
            ui.horizontal(|ui| {
                if confirming {
                    ui.label(
                        egui::RichText::new(tr("Cancel this render?"))
                            .small()
                            .color(egui::Color32::YELLOW),
                    );
                    if custom_widgets::ae_button(ui, &tr("Yes, Cancel")).clicked() {
                        if let Some(flag) = &app.export_cancel_flag {
                            flag.store(true, std::sync::atomic::Ordering::Relaxed);
                            log::info!("Render cancelled by user");
                        }
                        ui.ctx()
                            .data_mut(|d| d.insert_temp(egui::Id::new(CANCEL_CONFIRM_ID), false));
                    }
                    if custom_widgets::ae_button(ui, &tr("Keep Rendering")).clicked() {
                        ui.ctx()
                            .data_mut(|d| d.insert_temp(egui::Id::new(CANCEL_CONFIRM_ID), false));
                    }
                } else if custom_widgets::ae_button(ui, &tr("■ Cancel Render")).clicked() {
                    ui.ctx()
                        .data_mut(|d| d.insert_temp(egui::Id::new(CANCEL_CONFIRM_ID), true));
                }
            });
        } else {
            // Reset transient render state when idle.
            ui.ctx().data_mut(|d| {
                d.remove::<Instant>(egui::Id::new(START_TIME_ID));
                d.remove::<bool>(egui::Id::new(CANCEL_CONFIRM_ID));
            });
            ui.add(egui::ProgressBar::new(0.0).text(tr_format(
                "Ready to Render ({} frames)",
                &[&range_frames.to_string()],
            )));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_state::QueueItemStatus;

    #[test]
    fn queue_list_renders_mixed_statuses_without_panic() {
        let mut app = KagariApp::default();
        let comp_id = app.history.current().active_composition().id.clone();
        app.render_queue_items = vec![comp_id.clone(), "missing-id".into()];
        app.render_item_status
            .insert(comp_id, QueueItemStatus::Rendering);
        app.render_item_status
            .insert("missing-id".into(), QueueItemStatus::Failed);

        let ctx = egui::Context::default();
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(400.0, 600.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    draw_render_queue_panel(&mut app, ui);
                });
            },
        );
        // Statuses survive the draw; unknown names default to Queued.
        assert_eq!(
            app.render_item_status.get("missing-id"),
            Some(&QueueItemStatus::Failed)
        );
    }
}
