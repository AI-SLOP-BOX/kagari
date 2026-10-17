use crate::core::proxy::ProxyResolution;
use crate::ui::localization::tr;
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

/// Draw proxy controls in the timeline header or viewport overlay.
pub fn draw_proxy_controls(app: &mut KagariApp, ui: &mut egui::Ui) {
    let (current, active_in_preview) = {
        let comp = app.history.current().active_composition();
        (
            comp.comp_proxy.global_resolution,
            comp.comp_proxy.active_in_preview,
        )
    };

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(tr("Proxy"))
                .small()
                .color(colors::TEXT_PRIMARY),
        );

        let btn = |ui: &mut egui::Ui, label: &str, res: ProxyResolution| {
            let active = current == res;
            ui.selectable_label(
                active,
                egui::RichText::new(label).small().color(if active {
                    colors::ACCENT_CYAN
                } else {
                    colors::TEXT_PRIMARY
                }),
            )
        };

        if btn(ui, &tr("Full"), ProxyResolution::Full).clicked() {
            app.modify_project(|project| {
                project
                    .active_composition_mut()
                    .comp_proxy
                    .global_resolution = ProxyResolution::Full;
            });
        }
        if btn(ui, &tr("Half"), ProxyResolution::Half).clicked() {
            app.modify_project(|project| {
                project
                    .active_composition_mut()
                    .comp_proxy
                    .global_resolution = ProxyResolution::Half;
            });
        }
        if btn(ui, "¼", ProxyResolution::Quarter).clicked() {
            app.modify_project(|project| {
                project
                    .active_composition_mut()
                    .comp_proxy
                    .global_resolution = ProxyResolution::Quarter;
            });
        }
        if btn(ui, "⅛", ProxyResolution::Eighth).clicked() {
            app.modify_project(|project| {
                project
                    .active_composition_mut()
                    .comp_proxy
                    .global_resolution = ProxyResolution::Eighth;
            });
        }

        ui.separator();

        if ui
            .selectable_label(
                active_in_preview,
                egui::RichText::new(if active_in_preview {
                    tr("Proxy ON")
                } else {
                    tr("Proxy OFF")
                })
                .small()
                .color(if active_in_preview {
                    colors::ACCENT_CYAN
                } else {
                    colors::TEXT_MUTED
                }),
            )
            .clicked()
        {
            app.modify_project(|project| {
                project
                    .active_composition_mut()
                    .comp_proxy
                    .active_in_preview = !active_in_preview;
            });
        }

        ui.separator();
        // The toggle writes the shared processing-side flag (read by FFmpeg
        // export, software-renderer frame ranges, and parallel export), so the
        // button reflects and controls actual rendering behavior.
        let mfr_enabled = crate::core::parallel_render::mfr_enabled();
        if ui
            .selectable_label(mfr_enabled, "MFR")
            .on_hover_text(tr("Multi-Frame Rendering (Rayon CPU Parallelism)"))
            .clicked()
        {
            crate::core::parallel_render::set_mfr_enabled(!mfr_enabled);
        }

        if ui
            .small_button(tr("Purge RAM"))
            .on_hover_text(tr("Purge all RAM Preview Frame Cache"))
            .clicked()
        {
            crate::core::frame_cache::bump_version();
            app.toasts.info(tr("RAM Preview Cache purged"));
        }
    });
}

/// Draw per-layer proxy toggle in the layer controls area.
pub fn draw_layer_proxy(app: &mut KagariApp, ui: &mut egui::Ui, layer_idx: usize) {
    let (enabled, res) = {
        let comp = app.history.current().active_composition();
        if layer_idx >= comp.layers.len() {
            return;
        }
        let proxy = &comp.layers[layer_idx].proxy;
        (proxy.enabled, proxy.resolution)
    };

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(tr("Proxy"))
                .small()
                .color(colors::TEXT_MUTED),
        );

        let mut next_enabled = enabled;
        if ui.checkbox(&mut next_enabled, "").changed() {
            app.modify_project(|project| {
                if let Some(layer) = project.active_composition_mut().layers.get_mut(layer_idx) {
                    layer.proxy.enabled = next_enabled;
                }
            });
        }

        if enabled {
            let btn = |ui: &mut egui::Ui, label: &str, r: ProxyResolution| {
                ui.selectable_label(res == r, egui::RichText::new(label).small())
            };
            if btn(ui, "½", ProxyResolution::Half).clicked() {
                app.modify_project(|project| {
                    if let Some(layer) = project.active_composition_mut().layers.get_mut(layer_idx)
                    {
                        layer.proxy.resolution = ProxyResolution::Half;
                    }
                });
            }
            if btn(ui, "¼", ProxyResolution::Quarter).clicked() {
                app.modify_project(|project| {
                    if let Some(layer) = project.active_composition_mut().layers.get_mut(layer_idx)
                    {
                        layer.proxy.resolution = ProxyResolution::Quarter;
                    }
                });
            }
            if btn(ui, "⅛", ProxyResolution::Eighth).clicked() {
                app.modify_project(|project| {
                    if let Some(layer) = project.active_composition_mut().layers.get_mut(layer_idx)
                    {
                        layer.proxy.resolution = ProxyResolution::Eighth;
                    }
                });
            }
        }

        ui.separator();
        draw_proxy_job_controls(app, ui, layer_idx);
    });
}

/// Background proxy-generation controls: shows Generate while idle, progress +
/// Cancel while a job runs, and polls the worker channel every frame.
fn draw_proxy_job_controls(app: &mut KagariApp, ui: &mut egui::Ui, layer_idx: usize) {
    poll_proxy_job(app, ui.ctx());

    if app.proxy_job_rx.is_some() {
        let label = app
            .proxy_job_label
            .clone()
            .unwrap_or_else(|| tr("Generating proxy…"));
        ui.spinner();
        if let Some((done, total)) = app.proxy_job_progress {
            let fraction = if total == 0 { 0.0 } else { done as f32 / total as f32 };
            ui.add(
                egui::ProgressBar::new(fraction)
                    .show_percentage()
                    .desired_width(160.0)
                    .text(format!("{done}/{total} frames")),
            );
        }
        ui.label(egui::RichText::new(label).small().color(colors::TEXT_MUTED));
        let cancel_requested = app
            .proxy_job_cancel_flag
            .as_ref()
            .is_some_and(|flag| flag.load(std::sync::atomic::Ordering::Relaxed));
        if cancel_requested {
            ui.label(
                egui::RichText::new(tr("Cancelling…"))
                    .small()
                    .color(colors::TEXT_MUTED),
            );
        } else if ui.small_button(tr("Cancel")).clicked() {
            if let Some(flag) = app.proxy_job_cancel_flag.as_ref() {
                flag.store(true, std::sync::atomic::Ordering::Relaxed);
            }
        }
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(100));
        return;
    }

    let can_generate = {
        let comp = app.history.current().active_composition();
        comp.layers.get(layer_idx).is_some_and(|layer| {
            matches!(
                layer.layer_type,
                crate::core::timeline::LayerType::Image { .. }
                    | crate::core::timeline::LayerType::Video { .. }
            )
        })
    };
    if !can_generate {
        return;
    }
    if ui
        .small_button(tr("Generate proxy"))
        .on_hover_text(tr(
            "Render a low-resolution WebP proxy on a background worker",
        ))
        .clicked()
    {
        start_proxy_job(app, ui.ctx(), layer_idx);
    }
}

/// Spawn the background worker for `layer_idx`. Returns false when no layer or
/// a job is already running.
pub(crate) fn start_proxy_job(app: &mut KagariApp, ctx: &egui::Context, layer_idx: usize) -> bool {
    if app.proxy_job_rx.is_some() {
        return false;
    }
    let Some((layer, composition_name, resolution)) = ({
        let comp = app.history.current().active_composition();
        let layer = comp.layers.get(layer_idx).cloned();
        let name = comp.name.clone();
        layer.map(|layer| {
            let resolution = layer.proxy.resolution;
            (layer, name, resolution)
        })
    }) else {
        return false;
    };
    if !matches!(
        layer.layer_type,
        crate::core::timeline::LayerType::Image { .. }
            | crate::core::timeline::LayerType::Video { .. }
    ) {
        return false;
    }
    if resolution == ProxyResolution::Full {
        app.toasts
            .info(tr("Set Half, Quarter, or Eighth before generating a proxy"));
        return false;
    }
    let destination =
        std::env::temp_dir().join(format!("kagari_proxy_{}_{layer_idx}", std::process::id()));
    let layer_id = layer.id.clone();
    let label = format!("Generating proxy… {layer_id} ({})", resolution.label());
    let cancel_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let worker_cancel = cancel_flag.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    app.proxy_job_rx = Some(rx);
    app.proxy_job_cancel_flag = Some(cancel_flag);
    app.proxy_job_label = Some(label);
    app.proxy_job_progress = None;
    app.toasts
        .info(tr("Rendering proxy on a background worker…"));
    ctx.request_repaint_after(std::time::Duration::from_millis(100));

    std::thread::spawn(move || {
        let finish = |proxy_path: Result<String, String>| {
            let _ = tx.send(crate::app_state::ProxyJobMessage::Finished(
                crate::app_state::ProxyJobResult {
                    composition_name,
                    layer_id,
                    layer_idx,
                    resolution,
                    proxy_path,
                },
            ));
        };
        if worker_cancel.load(std::sync::atomic::Ordering::Relaxed) {
            finish(Err(crate::core::proxy::PROXY_CANCELLED.to_string()));
            return;
        }
        // Throttled progress: at most one message per 100 ms (plus a final
        // update) so long video jobs cannot flood the UI channel.
        let last_sent: std::cell::Cell<Option<std::time::Instant>> =
            std::cell::Cell::new(None);
        let tx_progress = tx.clone();
        let mut progress = move |done: u32, total: u32| {
            let now = std::time::Instant::now();
            let due = done >= total
                || last_sent
                    .get()
                    .is_none_or(|prev| now.duration_since(prev) >= std::time::Duration::from_millis(100));
            if due {
                last_sent.set(Some(now));
                let _ = tx_progress.send(crate::app_state::ProxyJobMessage::Progress {
                    done,
                    total,
                });
            }
        };
        let proxy_path = crate::core::proxy::generate_preview_proxy_for_layer(
            &layer,
            resolution,
            &destination,
            &worker_cancel,
            &mut progress,
        );
        finish(proxy_path);
    });
    true
}

/// Poll the proxy worker channel; progress messages update the bar, and the
/// final message attaches the proxy path to the matching layer (unless the
/// user requested cancel — that always wins over a racing completion).
pub(crate) fn poll_proxy_job(app: &mut KagariApp, ctx: &egui::Context) {
    loop {
        let message = match app.proxy_job_rx.as_ref() {
            Some(rx) => rx.try_recv(),
            None => return,
        };
        match message {
            Ok(crate::app_state::ProxyJobMessage::Progress { done, total }) => {
                app.proxy_job_progress = Some((done, total));
            }
            Ok(crate::app_state::ProxyJobMessage::Finished(result)) => {
                handle_proxy_job_finished(app, result);
                return;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
                return;
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                clear_proxy_job_state(app);
                app.toasts.error(tr("Proxy worker stopped unexpectedly"));
                return;
            }
        }
    }
}

fn clear_proxy_job_state(app: &mut KagariApp) {
    app.proxy_job_rx = None;
    app.proxy_job_cancel_flag = None;
    app.proxy_job_label = None;
    app.proxy_job_progress = None;
}

fn handle_proxy_job_finished(app: &mut KagariApp, result: crate::app_state::ProxyJobResult) {
    // Snapshot the cancel flag before clearing job state: if the user pressed
    // Cancel while the worker was finishing, the completed result must NOT be
    // applied — the cancel request always wins.
    let cancel_requested = app
        .proxy_job_cancel_flag
        .as_ref()
        .is_some_and(|flag| flag.load(std::sync::atomic::Ordering::Relaxed));
    clear_proxy_job_state(app);

    match result.proxy_path {
        Ok(_) if cancel_requested => {
            app.toasts.info(tr("Proxy generation cancelled"));
        }
        Ok(path) => {
            let mut applied = false;
            app.modify_project(|project| {
                let comp = project.active_composition_mut();
                if comp.name != result.composition_name {
                    return;
                }
                let matches = comp
                    .layers
                    .get(result.layer_idx)
                    .is_some_and(|layer| layer.id == result.layer_id);
                if !matches {
                    return;
                }
                if let Some(layer) = comp.layers.get_mut(result.layer_idx) {
                    layer.proxy.proxy_path = Some(path.clone());
                    layer.proxy.enabled = true;
                    applied = true;
                }
            });
            if applied {
                crate::core::frame_cache::bump_version();
                app.toasts
                    .info(tr("Proxy ready — preview uses the low-resolution media"));
            } else {
                app.toasts.error(tr(
                    "Proxy finished, but the layer changed; result was not applied.",
                ));
            }
        }
        Err(error) if error == crate::core::proxy::PROXY_CANCELLED => {
            app.toasts.info(tr("Proxy generation cancelled"));
        }
        Err(error) => {
            app.toasts.error(error);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_resolution_labels() {
        assert_eq!(ProxyResolution::Full.label(), "Full");
        assert_eq!(ProxyResolution::Half.label(), "Half");
        assert_eq!(ProxyResolution::Quarter.label(), "Quarter");
        assert_eq!(ProxyResolution::Eighth.label(), "Eighth");
    }

    #[test]
    fn test_proxy_resolution_factors() {
        assert_eq!(ProxyResolution::Full.factor(), 1.0);
        assert_eq!(ProxyResolution::Half.factor(), 0.5);
        assert_eq!(ProxyResolution::Quarter.factor(), 0.25);
        assert_eq!(ProxyResolution::Eighth.factor(), 0.125);
    }

    /// Prepare an app whose active composition has exactly one layer, and
    /// return the identity needed to build a `ProxyJobResult`.
    fn app_with_one_layer() -> (KagariApp, String, String) {
        let mut app = KagariApp::default();
        app.modify_project(|project| {
            let comp = project.active_composition_mut();
            if comp.layers.is_empty() {
                comp.layers.push(crate::core::timeline::Layer::new_null(
                    "proxy-test-layer".into(),
                    "Null".into(),
                    30,
                ));
            }
        });
        let (comp_name, layer_id) = {
            let comp = app.history.current().active_composition();
            (
                comp.name.clone(),
                comp.layers.first().expect("one layer").id.clone(),
            )
        };
        (app, comp_name, layer_id)
    }

    fn finished_message(comp_name: String, layer_id: String) -> crate::app_state::ProxyJobMessage {
        crate::app_state::ProxyJobMessage::Finished(crate::app_state::ProxyJobResult {
            composition_name: comp_name,
            layer_id,
            layer_idx: 0,
            resolution: ProxyResolution::Half,
            proxy_path: Ok("/tmp/kagari_test_proxy.webp".into()),
        })
    }

    #[test]
    fn finished_proxy_job_applies_to_matching_layer() {
        let (mut app, comp_name, layer_id) = app_with_one_layer();
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(finished_message(comp_name, layer_id)).unwrap();
        app.proxy_job_rx = Some(rx);
        app.proxy_job_cancel_flag = Some(std::sync::Arc::new(
            std::sync::atomic::AtomicBool::new(false),
        ));
        app.proxy_job_label = Some("job".into());

        poll_proxy_job(&mut app, &egui::Context::default());

        assert!(app.proxy_job_rx.is_none(), "job state must clear");
        let comp = app.history.current().active_composition();
        let layer = &comp.layers[0];
        assert_eq!(
            layer.proxy.proxy_path.as_deref(),
            Some("/tmp/kagari_test_proxy.webp"),
            "completed proxy must be applied"
        );
        assert!(layer.proxy.enabled);
    }

    /// Cancel is authoritative: if the user pressed Cancel while the worker
    /// was already finishing, the completed result must be discarded.
    #[test]
    fn cancel_requested_discards_racing_finished_result() {
        let (mut app, comp_name, layer_id) = app_with_one_layer();
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(finished_message(comp_name, layer_id)).unwrap();
        app.proxy_job_rx = Some(rx);
        app.proxy_job_cancel_flag = Some(std::sync::Arc::new(
            std::sync::atomic::AtomicBool::new(true),
        ));
        app.proxy_job_label = Some("job".into());

        poll_proxy_job(&mut app, &egui::Context::default());

        assert!(app.proxy_job_rx.is_none(), "job state must clear");
        assert!(app.proxy_job_cancel_flag.is_none());
        assert!(app.proxy_job_progress.is_none());
        let comp = app.history.current().active_composition();
        assert!(
            comp.layers[0].proxy.proxy_path.is_none(),
            "a cancelled job's result must never be applied"
        );
    }

    #[test]
    fn progress_messages_update_proxy_job_progress() {
        let mut app = KagariApp::default();
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(crate::app_state::ProxyJobMessage::Progress {
            done: 5,
            total: 10,
        })
        .unwrap();
        app.proxy_job_rx = Some(rx);

        poll_proxy_job(&mut app, &egui::Context::default());
        assert_eq!(app.proxy_job_progress, Some((5, 10)));
        assert!(app.proxy_job_rx.is_some(), "job still running after progress");

        // Worker exits without a Finished message → state must reset.
        drop(tx);
        poll_proxy_job(&mut app, &egui::Context::default());
        assert!(app.proxy_job_rx.is_none());
        assert!(app.proxy_job_progress.is_none());
    }
}
