use crate::core::editor_assist::{analyze_exposure_clipping, sharpness_score};
use crate::ui::kui::{self, ButtonKind, StatusKind};
use crate::ui::localization::{tr, tr_format};
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct QualityCheckResult {
    pub frame: u32,
    pub shadow_clipping: f32,
    pub highlight_clipping: f32,
    pub sharpness: f32,
    pub width: u32,
    pub height: u32,
}

struct QualityCheckCompletion {
    session_id: u64,
    generation: u64,
    composition_id: String,
    frame: u32,
    result: Result<QualityCheckResult, String>,
}

struct QualityCheckJob {
    receiver: Mutex<std::sync::mpsc::Receiver<QualityCheckCompletion>>,
}

pub fn draw(app: &mut KagariApp, ctx: &egui::Context) {
    poll_analysis(app, ctx);
    if !app.show_quality_check_panel {
        return;
    }
    let mut open = app.show_quality_check_panel;
    crate::ui::modal::window(tr("Analyze / Quality Check"))
        .open(&mut open)
        .default_width(380.0)
        .default_height(440.0)
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-12.0, 48.0))
        .show(ctx, |ui| {
            draw_header(ui);
            ui.add_space(kui::space::XS);
            let analyzing = current_job(ctx).is_some();
            let analyze_button_size = egui::vec2(ui.available_width(), 30.0);
            if kui::button_enabled_min_size(
                ui,
                if analyzing {
                    "Analyzing current frame…"
                } else {
                    "▶ Analyze Current Frame"
                },
                ButtonKind::Accent,
                !analyzing,
                analyze_button_size,
            )
            .on_hover_text(tr(
                "Render and analyze the full-resolution frame in the background",
            ))
            .clicked()
            {
                if let Err(error) = start_analysis(app, ctx) {
                    app.toasts.error(tr(&error));
                }
            }
            ui.add_space(kui::space::SM);
            if let Some(result) = app.quality_check_result.clone() {
                draw_result(app, ui, &result);
            } else {
                kui::empty_state(ui, &tr("Run Analyze to render the current frame."), 72.0);
                kui::caption(
                    ui,
                    "Rendering runs in the background at composition resolution.",
                );
            }
        });
    app.show_quality_check_panel = open;
}

fn draw_header(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        kui::section_title(ui, "Frame QC");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            kui::badge(ui, "NON-DESTRUCTIVE", colors::ACCENT_INFORMATION);
        });
    });
    kui::caption(ui, "Exposure clipping and focus diagnostics");
}

fn current_job(ctx: &egui::Context) -> Option<Arc<QualityCheckJob>> {
    ctx.data(|data| {
        data.get_temp::<Option<Arc<QualityCheckJob>>>(egui::Id::new("frame_qc_job"))
            .flatten()
    })
}

fn start_analysis(app: &mut KagariApp, ctx: &egui::Context) -> Result<(), String> {
    if current_job(ctx).is_some() {
        return Err(tr("A frame quality check is already running"));
    }
    let frame = app.playback.current_frame;
    let composition_index = app.history.current().active_composition_idx;
    let composition = if app.doc_sync_generation == app.history.generation() {
        app.production_document
            .as_ref()
            .and_then(|document| document.composition_for_frame(composition_index, frame))
            .or_else(|| {
                app.history
                    .current()
                    .compositions
                    .get(composition_index)
                    .cloned()
            })
    } else {
        app.history
            .current()
            .compositions
            .get(composition_index)
            .cloned()
    }
    .ok_or_else(|| tr("Active composition is unavailable"))?;
    let byte_size =
        crate::core::software_renderer::rgba_buffer_size(composition.width, composition.height)
            .ok_or_else(|| tr("Composition frame exceeds the 512 MiB render limit"))?;
    let composition_id = composition.id.clone();
    let session_id = app.project_session_id;
    let generation = app.history.generation();
    app.quality_check_result = None;
    let exposure_ev = ctx
        .data(|data| data.get_temp::<f32>(egui::Id::new("ae_exposure_ev")))
        .unwrap_or(0.0);
    let lut_mode = app.color_lut_mode;
    let (sender, receiver) = std::sync::mpsc::channel();
    let job = Arc::new(QualityCheckJob {
        receiver: Mutex::new(receiver),
    });
    ctx.data_mut(|data| data.insert_temp(egui::Id::new("frame_qc_job"), Some(job)));
    let worker_ctx = ctx.clone();
    let spawn_result = std::thread::Builder::new()
        .name("kagari-frame-qc".into())
        .spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let pixels = crate::core::software_renderer::render_frame_to_pixels(
                    &composition,
                    frame,
                    composition.width,
                    composition.height,
                    exposure_ev,
                    lut_mode,
                );
                if pixels.len() != byte_size {
                    return Err(tr("Frame renderer returned an incomplete image"));
                }
                let exposure = analyze_exposure_clipping(
                    &pixels,
                    composition.width,
                    composition.height,
                    4,
                    250,
                )
                .ok_or_else(|| tr("Could not analyze the rendered frame"))?;
                let sharpness = sharpness_score(&pixels, composition.width, composition.height)
                    .ok_or_else(|| tr("Could not measure frame sharpness"))?;
                Ok(QualityCheckResult {
                    frame,
                    shadow_clipping: exposure.shadow_clipped_fraction,
                    highlight_clipping: exposure.highlight_clipped_fraction,
                    sharpness,
                    width: composition.width,
                    height: composition.height,
                })
            }))
            .unwrap_or_else(|_| Err(tr("Frame quality analysis failed unexpectedly")));
            let _ = sender.send(QualityCheckCompletion {
                session_id,
                generation,
                composition_id,
                frame,
                result,
            });
            worker_ctx.request_repaint();
        });
    if let Err(error) = spawn_result {
        ctx.data_mut(|data| {
            data.remove_temp::<Option<Arc<QualityCheckJob>>>(egui::Id::new("frame_qc_job"))
        });
        return Err(tr_format(
            "Could not start frame analysis: {}",
            &[&error.to_string()],
        ));
    }
    ctx.request_repaint_after(std::time::Duration::from_millis(100));
    Ok(())
}

fn poll_analysis(app: &mut KagariApp, ctx: &egui::Context) {
    let Some(job) = current_job(ctx) else {
        return;
    };
    let completion = job
        .receiver
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .try_recv();
    match completion {
        Ok(completion) => {
            ctx.data_mut(|data| {
                data.remove_temp::<Option<Arc<QualityCheckJob>>>(egui::Id::new("frame_qc_job"))
            });
            let Some(current_comp) = app
                .history
                .current()
                .compositions
                .get(app.history.current().active_composition_idx)
            else {
                return;
            };
            if completion.session_id != app.project_session_id
                || completion.generation != app.history.generation()
                || completion.composition_id != current_comp.id
                || completion.frame != app.playback.current_frame
            {
                app.toasts.warning(tr(
                    "Frame analysis was discarded because the project or frame changed",
                ));
                return;
            }
            match completion.result {
                Ok(result) => {
                    app.quality_check_result = Some(result);
                    app.toasts.info(tr_format(
                        "Frame {} quality check complete",
                        &[&completion.frame.to_string()],
                    ));
                }
                Err(error) => app.toasts.error(tr(&error)),
            }
        }
        Err(std::sync::mpsc::TryRecvError::Empty) => {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
            ctx.data_mut(|data| {
                data.remove_temp::<Option<Arc<QualityCheckJob>>>(egui::Id::new("frame_qc_job"))
            });
            app.toasts
                .error(tr("Frame analysis worker stopped unexpectedly"));
        }
    }
}

fn draw_result(app: &mut KagariApp, ui: &mut egui::Ui, result: &QualityCheckResult) {
    ui.horizontal(|ui| {
        kui::readout(
            ui,
            tr_format("FRAME {}", &[&result.frame.to_string()]),
            colors::TIMELINE_PLAYHEAD,
        );
        kui::caption(ui, format!("{} × {}", result.width, result.height));
        if kui::button(ui, "Go to frame", ButtonKind::Quiet).clicked() {
            app.playback.current_frame = result.frame;
        }
    });
    kui::separator(ui);
    metric_row(ui, "Shadow clipping", result.shadow_clipping, 0.02, true);
    metric_row(
        ui,
        "Highlight clipping",
        result.highlight_clipping,
        0.02,
        true,
    );
    ui.add_space(kui::space::SM);
    let focus_color = if result.sharpness < 20.0 {
        colors::ACCENT_WARNING
    } else {
        colors::ACCENT_INFORMATION
    };
    ui.horizontal(|ui| {
        kui::secondary(ui, "Sharpness");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            kui::readout(ui, format!("{:.1}", result.sharpness), focus_color);
        });
    });
    if result.sharpness < 20.0 {
        kui::status(ui, StatusKind::Warning, "⚠ Possible soft-focus frame");
    }
}

fn metric_row(ui: &mut egui::Ui, label: &str, fraction: f32, warning: f32, lower_is_better: bool) {
    let warned = if lower_is_better {
        fraction > warning
    } else {
        fraction < warning
    };
    let color = if warned {
        colors::ACCENT_WARNING
    } else {
        colors::ACCENT_INFORMATION
    };
    ui.horizontal(|ui| {
        kui::secondary(ui, label);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            kui::readout(ui, format!("{:.2}%", fraction * 100.0), color);
        });
    });
    kui::progress_bar(ui, fraction.clamp(0.0, 1.0), None);
    if warned {
        kui::status(ui, StatusKind::Warning, "⚠ Above recommended 2% threshold");
    }
    ui.add_space(kui::space::XS);
}
