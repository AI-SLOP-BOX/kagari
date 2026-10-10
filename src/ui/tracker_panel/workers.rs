use crate::ui::localization::{tr, tr_format};
use crate::KagariApp;
use eframe::egui;

pub(crate) fn poll_camera_solve(app: &mut KagariApp, ctx: &egui::Context) {
    let Some(rx) = app.camera_solve_rx.as_ref() else {
        return;
    };
    let result = match rx.try_recv() {
        Ok(result) => Some(result),
        Err(std::sync::mpsc::TryRecvError::Empty) => {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
            None
        }
        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
            Some(Err(tr("Camera solve worker stopped unexpectedly")))
        }
    };
    let Some(result) = result else { return };
    app.camera_solve_rx = None;
    let Some((target_name, width, height, fov, feature_count)) = app.camera_solve_target.take()
    else {
        return;
    };
    match result {
        Ok(solution) => {
            let mut project = app.history.current().clone();
            let comp = project.active_composition_mut();
            if comp.name != target_name || comp.width != width || comp.height != height {
                app.toasts.error(tr("3D camera solve finished, but the active composition changed; result was not applied."));
                return;
            }
            let mut camera = crate::core::timeline::Camera3D {
                id: comp.next_camera_id(),
                name: "3D Tracked Camera 1".to_string(),
                active: true,
                fov_degrees: fov,
                ..Default::default()
            };
            camera.transform.position = crate::core::property::Animatable::new_animated(
                solution
                    .camera_frames
                    .iter()
                    .map(|frame| {
                        crate::core::keyframe::Keyframe::new(
                            frame.frame,
                            frame.pos.map(|value| value as f32),
                            crate::core::keyframe::InterpolationType::Linear,
                        )
                    })
                    .collect(),
            );
            camera.transform.rotation = crate::core::property::Animatable::new_animated(
                solution
                    .camera_frames
                    .iter()
                    .map(|frame| {
                        crate::core::keyframe::Keyframe::new(
                            frame.frame,
                            frame.rot_deg.map(|value| value as f32),
                            crate::core::keyframe::InterpolationType::Linear,
                        )
                    })
                    .collect(),
            );
            let camera_id = camera.id.clone();
            let camera_name = camera.name.clone();
            let camera_transform = camera.transform.clone();
            for existing in &mut comp.cameras {
                existing.active = false;
            }
            comp.cameras.push(camera);
            let camera_idx = comp.cameras.len() - 1;
            comp.set_active_camera(Some(camera_idx));
            let mut camera_layer = crate::core::timeline::Layer::new(
                comp.next_layer_id("camera"),
                camera_name,
                crate::core::timeline::LayerType::Null,
                comp.duration_frames,
            );
            camera_layer.is_3d = true;
            camera_layer.transform_3d = camera_transform;
            camera_layer.scene_object =
                Some(crate::core::timeline::SceneObjectRef::Camera { id: camera_id });
            comp.add_layer(camera_layer);
            app.commit_project(project);
            let count = feature_count.to_string();
            let error = format!("{:.2}", solution.average_reprojection_error);
            app.toasts.info(tr_format(
                "3D Camera solved from {} tracks (average error: {} px)",
                &[&count, &error],
            ));
        }
        Err(error) => app
            .toasts
            .error(tr_format("3D camera solve failed: {}", &[&error])),
    }
}

pub(crate) fn poll_quad_track(app: &mut KagariApp, ctx: &egui::Context) {
    let Some(rx) = app.quad_track_rx.as_ref() else {
        return;
    };
    let result = match rx.try_recv() {
        Ok(result) => Some(Ok(result)),
        Err(std::sync::mpsc::TryRecvError::Empty) => {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
            None
        }
        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
            Some(Err(tr("Quad tracking worker stopped unexpectedly")))
        }
    };
    let Some(result) = result else { return };
    app.quad_track_rx = None;
    let was_cancelled = app
        .quad_track_cancel_flag
        .take()
        .is_some_and(|flag| flag.load(std::sync::atomic::Ordering::Relaxed));
    if was_cancelled {
        app.toasts.info(tr("Four-corner tracking canceled"));
        return;
    }
    let result = match result {
        Ok(result) => result,
        Err(error) => {
            app.toasts.error(error);
            return;
        }
    };
    let current_comp = app.history.current().active_composition();
    if current_comp.name != result.composition_name
        || (current_comp.width, current_comp.height) != result.composition_size
        || current_comp
            .layers
            .get(result.layer_idx)
            .is_none_or(|layer| layer.id != result.layer_id || layer.trackers.len() < 4)
    {
        app.toasts.error(tr(
            "Tracking finished, but its target changed; result was not applied.",
        ));
        return;
    }
    if result.track.frames.is_empty() || result.track.corners.len() != result.track.frames.len() {
        app.toasts.error(tr("Quad tracking produced no frames"));
        return;
    }
    let baked_frames = result.track.frames.len();
    app.modify_project(|project| {
        let comp = project.active_composition_mut();
        let Some(layer) = comp.layers.get_mut(result.layer_idx) else {
            return;
        };
        for slot in 0..4 {
            let keyframes = result
                .track
                .frames
                .iter()
                .zip(&result.track.corners)
                .map(|(frame, corners)| {
                    crate::core::keyframe::Keyframe::new(
                        *frame,
                        corners[slot],
                        crate::core::keyframe::InterpolationType::Linear,
                    )
                })
                .collect();
            layer.trackers[slot].position = crate::core::property::Animatable::Animated(keyframes);
        }
    });
    crate::core::frame_cache::bump_version();
    let quality_percent = format!("{:.0}", result.average_confidence * 100.0);
    let baked_frames_text = baked_frames.to_string();
    let quality = tr_format(
        "{}% over {} frames",
        &[&quality_percent, &baked_frames_text],
    );
    ctx.data_mut(|data| data.insert_temp(egui::Id::new("kagari_quad_conf"), quality.clone()));
    let task_name = match result.purpose {
        crate::app_state::QuadTrackPurpose::Perspective => tr("Quad tracking"),
        crate::app_state::QuadTrackPurpose::PlanarSurface => tr("Planar surface tracking"),
    };
    app.toasts.info(tr_format(
        "{} complete — avg lock {}",
        &[&task_name, &quality],
    ));
}

pub(crate) fn start_quad_track(
    app: &mut KagariApp,
    ctx: &egui::Context,
    layer_idx: usize,
    start_frame: u32,
    end_frame: u32,
    purpose: crate::app_state::QuadTrackPurpose,
) -> bool {
    let comp = app.history.current().active_composition();
    let Some(layer) = comp
        .layers
        .get(layer_idx)
        .filter(|layer| layer.trackers.len() >= 4)
    else {
        return false;
    };
    let src_rect =
        std::array::from_fn(|corner| layer.trackers[corner].position.evaluate(start_frame));
    let snapshot = comp.clone();
    let composition_name = comp.name.clone();
    let composition_size = (comp.width, comp.height);
    let layer_id = layer.id.clone();
    let cancel_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let worker_cancel_flag = cancel_flag.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    let repaint_ctx = ctx.clone();
    app.quad_track_cancel_flag = Some(cancel_flag);
    app.quad_track_rx = Some(rx);
    app.toasts
        .info(tr("Analyzing quad tracking on a background worker…"));
    ctx.request_repaint_after(std::time::Duration::from_millis(100));

    std::thread::spawn(move || {
        let track = crate::core::tracker_engine::TrackerEngine::analyze_quad_track_cancellable(
            &snapshot,
            layer_idx,
            [0, 1, 2, 3],
            start_frame,
            end_frame,
            Some(worker_cancel_flag),
        );
        let average_confidence =
            crate::core::tracker_engine::quad_track_confidence(&track, src_rect)
                .iter()
                .sum::<f32>()
                / track.frames.len().max(1) as f32;
        let _ = tx.send(crate::app_state::QuadTrackJobResult {
            purpose,
            composition_name,
            composition_size,
            layer_id,
            layer_idx,
            track,
            average_confidence,
        });
        repaint_ctx.request_repaint();
    });
    true
}
