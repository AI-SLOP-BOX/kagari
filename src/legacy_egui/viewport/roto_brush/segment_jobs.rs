use super::*;

pub(in crate::ui::viewport) fn start_roto_segment_job(
    app: &KagariApp,
    ctx: &egui::Context,
    request: RotoSegmentRequest,
) -> Result<(), String> {
    let RotoSegmentRequest {
        layer_index,
        frame,
        previous_strokes,
        strokes,
        settings,
        feather,
    } = request;
    let job_id = egui::Id::new("roto_segment_job");
    if ctx
        .data(|data| data.get_temp::<std::sync::Arc<RotoSegmentJob>>(job_id))
        .is_some()
    {
        return Err("Roto Brush is still analyzing the previous stroke".into());
    }
    let composition = app.history.current().active_composition().clone();
    let layer = composition
        .layers
        .get(layer_index)
        .ok_or_else(|| "Selected layer no longer exists".to_owned())?;
    if !layer.is_active(frame) {
        return Err("Move the playhead inside the selected layer before painting".into());
    }
    let frame_strokes = crate::core::roto_brush_engine::strokes_for_frame(&strokes, frame);
    if !frame_strokes
        .iter()
        .any(|stroke| stroke.stroke_type == RotoStrokeType::Foreground)
    {
        return Err("Add a foreground stroke on this frame before refining its matte".into());
    }
    let source_layer_snapshot = serde_json::to_vec(layer)
        .map_err(|error| format!("Could not snapshot source layer: {error}"))?;
    let layer_id = layer.id.clone();
    let composition_id = composition.id.clone();
    let composition_width = composition.width;
    let composition_height = composition.height;
    let composition_fps = composition.fps;
    let source_comp = prepare_roto_brush_source(&composition, layer_index)
        .ok_or_else(|| "Selected source layer is unavailable".to_owned())?;
    let (sender, receiver) = std::sync::mpsc::channel();
    let job = std::sync::Arc::new(RotoSegmentJob {
        receiver: std::sync::Mutex::new(receiver),
    });
    ctx.data_mut(|data| data.insert_temp(job_id, job));
    let worker_ctx = ctx.clone();
    let worker_strokes = frame_strokes;
    let spawn_result = std::thread::Builder::new()
        .name("kagari-roto-segment".into())
        .spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let pixels = crate::core::software_renderer::render_frame_to_pixels_filtered(
                    &source_comp,
                    frame,
                    source_comp.width,
                    source_comp.height,
                    0.0,
                    0,
                    Some(layer_index),
                );
                let expected = (source_comp.width as usize)
                    .checked_mul(source_comp.height as usize)
                    .and_then(|pixels| pixels.checked_mul(4))
                    .ok_or_else(|| "Composition dimensions exceed supported limits".to_owned())?;
                if pixels.len() != expected {
                    return Err("Could not render the selected source frame for Roto Brush".into());
                }
                let matte = generate_rotobrush_matte(
                    &pixels,
                    source_comp.width,
                    source_comp.height,
                    &worker_strokes,
                    &settings,
                );
                let polygon =
                    trace_contour_to_polygon(&matte, source_comp.width, source_comp.height, 2.0);
                Ok((polygon.len() >= 3).then_some(polygon))
            }))
            .unwrap_or_else(|_| Err("Roto Brush worker encountered an internal error".into()));
            let _ = sender.send(RotoSegmentCompletion {
                composition_id,
                composition_width,
                composition_height,
                composition_fps,
                layer_id,
                source_layer_snapshot,
                previous_strokes,
                strokes,
                frame,
                feather,
                result,
            });
            worker_ctx.request_repaint();
        });
    match spawn_result {
        Ok(handle) => app.track_background_thread(handle),
        Err(error) => {
            ctx.data_mut(|data| data.remove::<std::sync::Arc<RotoSegmentJob>>(job_id));
            return Err(format!("Could not start Roto Brush analysis: {error}"));
        }
    }
    Ok(())
}

pub(in crate::ui::viewport) fn poll_roto_segment_job(app: &mut KagariApp, ctx: &egui::Context) {
    let job_id = egui::Id::new("roto_segment_job");
    let Some(job) = ctx.data(|data| data.get_temp::<std::sync::Arc<RotoSegmentJob>>(job_id)) else {
        return;
    };
    let received = match job.receiver.lock() {
        Ok(receiver) => receiver.try_recv(),
        Err(_) => Err(std::sync::mpsc::TryRecvError::Disconnected),
    };
    let completion = match received {
        Ok(completion) => completion,
        Err(std::sync::mpsc::TryRecvError::Empty) => {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
            return;
        }
        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
            ctx.data_mut(|data| data.remove::<std::sync::Arc<RotoSegmentJob>>(job_id));
            app.toasts.error("Roto Brush worker stopped unexpectedly");
            return;
        }
    };
    ctx.data_mut(|data| data.remove::<std::sync::Arc<RotoSegmentJob>>(job_id));
    let stroke_state_id = egui::Id::new(("roto_strokes", completion.layer_id.as_str()));
    let restore_strokes = |ctx: &egui::Context| {
        ctx.data_mut(|data| data.insert_temp(stroke_state_id, completion.previous_strokes.clone()));
    };
    let mut project = app.history.current().clone();
    let comp = project.active_composition_mut();
    if comp.id != completion.composition_id
        || comp.width != completion.composition_width
        || comp.height != completion.composition_height
        || comp.fps != completion.composition_fps
    {
        restore_strokes(ctx);
        app.toasts
            .error("Composition changed; Roto Brush result was discarded");
        return;
    }
    let Some(layer_index) = comp
        .layers
        .iter()
        .position(|layer| layer.id == completion.layer_id)
    else {
        restore_strokes(ctx);
        app.toasts
            .error("Source layer changed; Roto Brush result was discarded");
        return;
    };
    let layer_matches = serde_json::to_vec(&comp.layers[layer_index])
        .ok()
        .as_deref()
        == Some(completion.source_layer_snapshot.as_slice());
    if !layer_matches {
        restore_strokes(ctx);
        app.toasts
            .error("Source layer changed during analysis; Roto Brush result was discarded");
        return;
    }
    let polygon = match completion.result {
        Ok(polygon) => polygon,
        Err(error) => {
            restore_strokes(ctx);
            app.toasts.error(error);
            return;
        }
    };
    let frame_stroke_count =
        crate::core::roto_brush_engine::strokes_for_frame(&completion.strokes, completion.frame)
            .len();
    let has_polygon = polygon.is_some();
    let layer = &mut comp.layers[layer_index];
    layer.roto_brush_strokes = completion.strokes;
    if let Some(polygon) = polygon {
        if let Some(mask) = layer
            .masks
            .iter_mut()
            .find(|mask| mask.name == "Roto Brush Matte")
        {
            crate::core::roto_assist::set_roto_matte_keyframe(mask, completion.frame, polygon);
            mask.feather = Animatable::new_constant(completion.feather);
        } else {
            let mut mask = crate::core::mask::Mask::new_closed(
                "roto_brush_matte".to_owned(),
                "Roto Brush Matte".to_owned(),
                polygon.clone(),
            );
            crate::core::roto_assist::set_roto_matte_keyframe(&mut mask, completion.frame, polygon);
            mask.feather = Animatable::new_constant(completion.feather);
            layer.masks.push(mask);
        }
    }
    app.commit_project(project);
    if has_polygon {
        app.toasts.info(if frame_stroke_count == 1 {
            "Roto Brush matte updated from foreground stroke"
        } else {
            "Roto Brush matte refined from foreground/background strokes"
        });
    } else {
        app.toasts
            .info("Roto Brush stroke saved; add marks around the subject to create a matte");
    }
}
