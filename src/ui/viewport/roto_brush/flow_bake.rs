use super::*;

pub(in crate::ui::viewport) fn start_roto_flow_bake(
    app: &KagariApp,
    ctx: &egui::Context,
    layer_index: usize,
    anchor_frame: u32,
) -> Result<(), String> {
    let job_id = egui::Id::new("roto_flow_bake_job");
    if ctx
        .data(|data| data.get_temp::<std::sync::Arc<RotoFlowJob>>(job_id))
        .is_some()
    {
        return Err("Roto optical-flow propagation is already running".into());
    }
    let composition = app.history.current().active_composition().clone();
    let layer = composition
        .layers
        .get(layer_index)
        .ok_or_else(|| format!("Selected layer {} no longer exists", layer_index + 1))?;
    let mask = layer
        .masks
        .iter()
        .find(|mask| mask.name == "Roto Brush Matte")
        .ok_or_else(|| "Create a Roto Brush Matte before propagation".to_owned())?;
    let start_frame = layer.in_frame;
    let end_frame = layer.out_frame.max(start_frame.saturating_add(1));
    if anchor_frame < start_frame || anchor_frame > end_frame {
        return Err("Move the playhead inside the selected layer before propagation".into());
    }
    let mask_id = mask.id.clone();
    let original_path = serde_json::to_vec(&mask.path.vertices)
        .map_err(|error| format!("Could not snapshot roto matte: {error}"))?;
    let source_layer_snapshot = serde_json::to_vec(layer)
        .map_err(|error| format!("Could not snapshot roto source layer: {error}"))?;
    let source_strokes = layer.roto_brush_strokes.clone();
    let layer_id = layer.id.clone();
    let composition_id = composition.id.clone();
    let composition_width = composition.width;
    let composition_height = composition.height;
    let composition_fps = composition.fps;
    let Some(source_comp) = prepare_roto_brush_source(&composition, layer_index) else {
        return Err("Selected source layer is unavailable".into());
    };
    let mask = mask.clone();
    let settings = crate::core::roto_brush_engine::RotoBrushSettings {
        feather_radius: ctx.data(|data| {
            data.get_temp::<f32>(egui::Id::new("roto_feather_radius"))
                .unwrap_or(3.0)
        }),
        contrast: ctx.data(|data| {
            data.get_temp::<f32>(egui::Id::new("roto_contrast"))
                .unwrap_or(1.0)
        }),
        ..Default::default()
    };
    let width = source_comp.width;
    let height = source_comp.height;
    let cancelled = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let (sender, receiver) = std::sync::mpsc::channel();
    let job = std::sync::Arc::new(RotoFlowJob {
        receiver: std::sync::Mutex::new(receiver),
        cancelled: cancelled.clone(),
    });
    ctx.data_mut(|data| data.insert_temp(job_id, job));

    let worker_ctx = ctx.clone();
    let spawn_result = std::thread::Builder::new()
        .name("kagari-roto-flow".into())
        .spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                crate::core::roto_assist::bake_optical_flow_roto_matte(
                    &mask,
                    &source_strokes,
                    &settings,
                    anchor_frame,
                    start_frame,
                    end_frame,
                    width,
                    height,
                    |frame| {
                        if cancelled.load(std::sync::atomic::Ordering::Relaxed) {
                            return None;
                        }
                        let pixels =
                            crate::core::software_renderer::render_frame_to_pixels_filtered(
                                &source_comp,
                                frame,
                                width,
                                height,
                                0.0,
                                0,
                                Some(layer_index),
                            );
                        (!pixels.is_empty()).then_some(pixels)
                    },
                )
            }))
            .unwrap_or_else(|_| {
                Err("Roto optical-flow worker encountered an internal error".into())
            });
            let result = if cancelled.load(std::sync::atomic::Ordering::Relaxed) {
                Err("Roto optical-flow propagation cancelled".to_owned())
            } else {
                result
            };
            let _ = sender.send((
                composition_id,
                composition_width,
                composition_height,
                composition_fps,
                layer_id,
                mask_id,
                original_path,
                source_layer_snapshot,
                result,
            ));
            worker_ctx.request_repaint();
        });
    match spawn_result {
        Ok(handle) => app.track_background_thread(handle),
        Err(error) => {
            ctx.data_mut(|data| data.remove::<std::sync::Arc<RotoFlowJob>>(job_id));
            return Err(format!("Could not start roto propagation: {error}"));
        }
    }
    Ok(())
}

pub(in crate::ui::viewport) fn poll_roto_flow_bake(app: &mut KagariApp, ctx: &egui::Context) {
    let job_id = egui::Id::new("roto_flow_bake_job");
    let job = ctx.data(|data| data.get_temp::<std::sync::Arc<RotoFlowJob>>(job_id));
    let Some(job) = job else { return };
    let received = match job.receiver.lock() {
        Ok(receiver) => receiver.try_recv(),
        Err(_) => Err(std::sync::mpsc::TryRecvError::Disconnected),
    };
    let completion = match received {
        Ok(completion) => completion,
        Err(std::sync::mpsc::TryRecvError::Empty) => return,
        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
            ctx.data_mut(|data| data.remove::<std::sync::Arc<RotoFlowJob>>(job_id));
            app.toasts
                .error("Roto optical-flow worker stopped unexpectedly");
            return;
        }
    };
    ctx.data_mut(|data| data.remove::<std::sync::Arc<RotoFlowJob>>(job_id));
    let (
        composition_id,
        composition_width,
        composition_height,
        composition_fps,
        layer_id,
        mask_id,
        original_path,
        source_layer_snapshot,
        result,
    ) = completion;
    let baked = match result {
        Ok(baked) => baked,
        Err(error) => {
            app.toasts.error(error);
            return;
        }
    };
    let mut project = app.history.current().clone();
    let comp = project.active_composition_mut();
    if comp.id != composition_id
        || comp.width != composition_width
        || comp.height != composition_height
        || comp.fps != composition_fps
    {
        app.toasts
            .error("Composition changed; propagated matte was discarded");
        return;
    }
    let Some(layer) = comp.layers.iter_mut().find(|layer| layer.id == layer_id) else {
        app.toasts
            .error("Source layer changed; propagated matte was discarded");
        return;
    };
    if serde_json::to_vec(layer).ok().as_deref() != Some(source_layer_snapshot.as_slice()) {
        app.toasts
            .error("Source layer changed during propagation; result was discarded");
        return;
    }
    let Some(mask) = layer.masks.iter_mut().find(|mask| mask.id == mask_id) else {
        app.toasts
            .error("Roto Brush Matte was removed; propagated result was discarded");
        return;
    };
    if serde_json::to_vec(&mask.path.vertices).ok().as_deref() != Some(original_path.as_slice()) {
        app.toasts
            .error("Roto Brush Matte changed during propagation; result was discarded");
        return;
    }
    mask.path.vertices = baked;
    app.commit_project(project);
    app.toasts
        .info("Roto matte propagated through adjacent-frame optical flow");
}
