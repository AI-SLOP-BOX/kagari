//! Asynchronous job execution and polling for content-aware fill.

use crate::ui::localization::{tr, tr_format};
use crate::KagariApp;
use eframe::egui;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use super::bake::*;

pub fn start_caf_job(
    app: &KagariApp,
    ctx: &egui::Context,
    range_idx: usize,
    expansion: f32,
    method: crate::core::content_aware_engine::FillMethod,
) -> Result<(), String> {
    let job_id = egui::Id::new("content_aware_fill_job");
    if ctx
        .data(|data| data.get_temp::<Arc<CafJob>>(job_id))
        .is_some()
    {
        return Err("A Content-Aware Fill sequence is already running".into());
    }
    let layer_index = app
        .selection
        .selected_layer_idx
        .ok_or_else(|| "Select a layer with a mask first".to_owned())?;
    let project = app.history.current();
    let composition = project.active_composition().clone();
    let layer = composition
        .layers
        .get(layer_index)
        .ok_or_else(|| "Selected layer no longer exists".to_owned())?;
    let (mask_index, mask) = layer
        .masks
        .iter()
        .enumerate()
        .find(|(_, mask)| mask.enabled)
        .ok_or_else(|| "Selected layer has no enabled mask".to_owned())?;
    if !mask.path.is_closed || mask.path.to_polygon(app.playback.current_frame, 12).len() < 3 {
        return Err(
            "Content-Aware Fill requires a closed mask with at least three vertices".into(),
        );
    }
    let (start_frame, end_frame) = caf_range(
        range_idx,
        composition.duration_frames,
        app.playback.work_area_in,
        app.playback.work_area_out,
        layer.in_frame,
        layer.out_frame,
    )?;
    let source = prepare_caf_source(&composition, layer_index, mask_index)
        .ok_or_else(|| "Could not prepare the selected layer for fill generation".to_owned())?;
    let directory = caf_asset_directory(&app.project_path)?;
    let asset_dir = caf_sequence_directory(&directory)?;
    let composition_id = composition.id.clone();
    let source_layer_id = layer.id.clone();
    let mask_id = mask.id.clone();
    let original_source_layer = serde_json::to_vec(layer)
        .map_err(|error| format!("Could not snapshot source layer: {error}"))?;
    let mask = mask.clone();
    let cancelled = Arc::new(AtomicBool::new(false));
    // Let `on_exit()` cancel this bake before joining the worker.
    app.register_exit_cancel_flag(cancelled.clone());
    let progress = Arc::new(AtomicU32::new(0));
    let total = end_frame - start_frame + 1;
    let (sender, receiver) = std::sync::mpsc::channel();
    let job = Arc::new(CafJob {
        receiver: Mutex::new(receiver),
        cancelled: cancelled.clone(),
        progress: progress.clone(),
        total,
    });
    ctx.data_mut(|data| data.insert_temp(job_id, job));
    let worker_ctx = ctx.clone();
    let worker_asset_dir = asset_dir.clone();
    let spawn_result = std::thread::Builder::new()
        .name("kagari-content-aware-fill".into())
        .spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                bake_caf_sequence(CafBakeRequest {
                    source: &source,
                    layer_index,
                    mask: &mask,
                    start_frame,
                    end_frame,
                    expansion,
                    method,
                    asset_dir: &worker_asset_dir,
                    cancelled: &cancelled,
                    progress: &progress,
                })
            }))
            .unwrap_or_else(|_| {
                Err("Content-Aware Fill worker encountered an internal error".into())
            });
            let result = if cancelled.load(Ordering::Relaxed) {
                Err("Content-Aware Fill cancelled".into())
            } else {
                result
            };
            if result.is_err() {
                let _ = std::fs::remove_dir_all(&worker_asset_dir);
            }
            let _ = sender.send(CafCompletion {
                composition_id,
                source_layer_id,
                mask_id,
                original_source_layer,
                start_frame,
                end_frame,
                method,
                asset_dir: worker_asset_dir,
                result,
            });
            worker_ctx.request_repaint();
        });
    match spawn_result {
        Ok(handle) => app.track_background_thread(handle),
        Err(error) => {
            ctx.data_mut(|data| data.remove::<Arc<CafJob>>(job_id));
            let _ = std::fs::remove_dir_all(&asset_dir);
            return Err(format!("Could not start fill generation: {error}"));
        }
    }
    Ok(())
}

pub fn poll_caf_job(app: &mut KagariApp, ctx: &egui::Context) {
    let job_id = egui::Id::new("content_aware_fill_job");
    let Some(job) = ctx.data(|data| data.get_temp::<Arc<CafJob>>(job_id)) else {
        return;
    };
    let received = match job.receiver.lock() {
        Ok(receiver) => receiver.try_recv(),
        Err(_) => Err(std::sync::mpsc::TryRecvError::Disconnected),
    };
    let completion = match received {
        Ok(completion) => completion,
        Err(std::sync::mpsc::TryRecvError::Empty) => return,
        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
            ctx.data_mut(|data| data.remove::<Arc<CafJob>>(job_id));
            app.toasts
                .error(tr("Content-Aware Fill worker stopped unexpectedly"));
            return;
        }
    };
    ctx.data_mut(|data| data.remove::<Arc<CafJob>>(job_id));
    if let Err(error) = completion.result {
        app.toasts.error(tr(&error));
        return;
    }
    let mut project = app.history.current().clone();
    let comp = project.active_composition_mut();
    if comp.id != completion.composition_id {
        let _ = std::fs::remove_dir_all(&completion.asset_dir);
        app.toasts.error(tr(
            "Composition changed; generated fill sequence was discarded",
        ));
        return;
    }
    let Some(source_index) = comp
        .layers
        .iter()
        .position(|layer| layer.id == completion.source_layer_id)
    else {
        let _ = std::fs::remove_dir_all(&completion.asset_dir);
        app.toasts.error(tr(
            "Source layer changed; generated fill sequence was discarded",
        ));
        return;
    };
    let source_layer = &comp.layers[source_index];
    let source_layer_matches = serde_json::to_vec(source_layer).ok().as_deref()
        == Some(completion.original_source_layer.as_slice());
    if !source_layer_matches
        || !source_layer
            .masks
            .iter()
            .any(|mask| mask.id == completion.mask_id)
    {
        let _ = std::fs::remove_dir_all(&completion.asset_dir);
        app.toasts.error(tr(
            "Source layer changed during generation; fill sequence was discarded",
        ));
        return;
    }
    let end_frame = completion.end_frame;
    let start_frame = completion.start_frame;
    let duration = comp.duration_frames;
    let frame_count = end_frame - start_frame + 1;
    let mut fill = crate::core::timeline::Layer::new(
        format!("caf_fill_{}_{}", start_frame, std::process::id()),
        format!("Fill Layer [CAF] ({}–{})", start_frame + 1, end_frame + 1),
        crate::core::timeline::LayerType::Video {
            source: "Kagari Content-Aware Fill sequence".into(),
            frames_dir: completion.asset_dir.to_string_lossy().into_owned(),
            frame_count,
            audio_wav: None,
            audio_volume: crate::core::property::Animatable::new_constant(0.0),
            speed: 1.0,
        },
        duration,
    );
    fill.in_frame = start_frame;
    fill.out_frame = end_frame;
    fill.time_remap = Some(crate::core::property::Animatable::Animated(vec![
        crate::core::keyframe::Keyframe::new(
            start_frame,
            0.0,
            crate::core::keyframe::InterpolationType::Linear,
        ),
        crate::core::keyframe::Keyframe::new(
            end_frame.max(start_frame + 1),
            frame_count.saturating_sub(1) as f32,
            crate::core::keyframe::InterpolationType::Linear,
        ),
    ]));
    fill.frame_blending = false;
    let insert_at = if comp
        .layers
        .get(source_index + 1)
        .is_some_and(|layer| layer.track_matte != crate::core::timeline::TrackMatteMode::None)
    {
        source_index + 2
    } else {
        source_index + 1
    }
    .min(comp.layers.len());
    comp.layers.insert(insert_at, fill);
    app.commit_project(project);
    let frame_count = frame_count.to_string();
    let method = match completion.method {
        crate::core::content_aware_engine::FillMethod::Object => tr("Object"),
        crate::core::content_aware_engine::FillMethod::Surface => tr("Surface"),
        crate::core::content_aware_engine::FillMethod::EdgeBlend => tr("Edge Blend"),
    };
    app.toasts.info(tr_format(
        "Generated {} WebP fill frames ({})",
        &[&frame_count, &method],
    ));
}
