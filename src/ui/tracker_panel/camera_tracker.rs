use crate::ui::custom_widgets;
use crate::ui::localization::tr;
use crate::KagariApp;
use eframe::egui;

pub(crate) fn draw_camera_tracker_section(app: &mut KagariApp, ui: &mut egui::Ui, idx: usize) {
    ui.separator();
    ui.collapsing(tr("3D Camera Tracker (Scene Reconstruction)"), |ui| {
        ui.horizontal(|ui| {
            ui.label(tr("Shot Type:"));
            let mut shot_type = ui.ctx().data(|d| {
                d.get_temp::<i32>(egui::Id::new("3d_cam_shot_type"))
                    .unwrap_or(0)
            });
            egui::ComboBox::from_id_salt("3d_cam_shot_combo")
                .selected_text(if shot_type == 0 {
                    tr("Fixed Angle of View")
                } else {
                    tr("Variable Zoom")
                })
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_value(&mut shot_type, 0, tr("Fixed Angle of View"))
                        .clicked()
                    {
                        ui.ctx()
                            .data_mut(|d| d.insert_temp(egui::Id::new("3d_cam_shot_type"), 0));
                    }
                    if ui
                        .selectable_value(&mut shot_type, 1, tr("Variable Zoom"))
                        .clicked()
                    {
                        ui.ctx()
                            .data_mut(|d| d.insert_temp(egui::Id::new("3d_cam_shot_type"), 1));
                    }
                });
        });

        let mut track_pts = ui.ctx().data(|d| {
            d.get_temp::<u32>(egui::Id::new("3d_cam_track_pts"))
                .unwrap_or(250)
        });
        ui.horizontal(|ui| {
            ui.label(tr("Track Points:"));
            if ui
                .add(egui::Slider::new(&mut track_pts, 50..=1000).suffix(" pts"))
                .changed()
            {
                ui.ctx()
                    .data_mut(|d| d.insert_temp(egui::Id::new("3d_cam_track_pts"), track_pts));
            }
        });

        ui.horizontal(|ui| {
            let solve_label = if app.camera_solve_rx.is_some() {
                tr("Solving 3D Camera…")
            } else {
                tr("Track & Solve 3D Camera")
            };
            if custom_widgets::ae_button_accent(ui, &solve_label)
                .on_hover_text(tr(
                    "Analyze 3D optical flow and solve virtual 3D camera trajectory",
                ))
                .clicked()
            {
                if app.camera_solve_rx.is_some() {
                    app.toasts.info(tr("A 3D camera solve is already running."));
                    return;
                }
                let mut temp_proj = app.history.current().clone();
                let comp_mut = temp_proj.active_composition_mut();
                let features: Vec<crate::core::camera_track::CameraTrackFeaturePoint> = comp_mut
                    .layers
                    .get(idx)
                    .into_iter()
                    .flat_map(|layer| layer.trackers.iter().take(track_pts as usize))
                    .enumerate()
                    .filter_map(|(feature_id, tracker)| {
                        let keyframes = tracker.position.keyframes()?;
                        if keyframes.len() < 2 {
                            return None;
                        }
                        Some(crate::core::camera_track::CameraTrackFeaturePoint {
                            id: format!("tracker_{feature_id}"),
                            frames: keyframes.iter().map(|keyframe| keyframe.frame).collect(),
                            coords_2d: keyframes.iter().map(|keyframe| keyframe.value).collect(),
                            world_3d: [0.0; 3],
                            confidence: 1.0,
                        })
                    })
                    .collect();
                if features.is_empty() {
                    app.toasts.error(tr(
                        "No valid 2D track points found with >= 2 keyframes. Track points first.",
                    ));
                    return;
                }
                let fov = comp_mut.resolve_camera().fov_degrees;
                let width = comp_mut.width;
                let height = comp_mut.height;
                let target_name = comp_mut.name.clone();
                let feature_count = features.len();
                let (tx, rx) = std::sync::mpsc::channel();
                let repaint_ctx = ui.ctx().clone();
                app.camera_solve_rx = Some(rx);
                app.camera_solve_target = Some((target_name, width, height, fov, feature_count));
                app.toasts
                    .info(tr("Solving 3D camera on a background worker…"));
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(100));
                let handle = std::thread::spawn(move || {
                    let result = crate::core::camera_track::solve_3d_camera_from_tracks(
                        &features, width, height, fov,
                    );
                    let _ = tx.send(result);
                    repaint_ctx.request_repaint();
                });
                app.track_background_thread(handle);
            }
        });
    });
}
