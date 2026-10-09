use crate::ui::custom_widgets;
use crate::ui::localization::{tr, tr_format};
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub(crate) fn draw_point_tracker_section(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    idx: usize,
    current_frame: u32,
) {
    let (layer_name, tracker_count, has_media, layer_pos_at_head) = {
        let comp = app.history.current().active_composition();
        match comp.layers.get(idx) {
            Some(l) => (
                l.name.clone(),
                l.trackers.len(),
                matches!(
                    l.layer_type,
                    crate::core::timeline::LayerType::Video { .. }
                        | crate::core::timeline::LayerType::Image { .. }
                ),
                l.transform.position.evaluate(current_frame),
            ),
            None => return,
        }
    };

            ui.add_space(4.0);
            // ── Tracker point management ──
            let tracker_count_text = tracker_count.to_string();
            ui.label(tr_format("Tracker points: {}", &[&tracker_count_text]));
            if custom_widgets::ae_button_accent(ui, &tr("+ Add Tracker Point"))
                .on_hover_text(tr("Add a track point at the playhead position"))
                .clicked()
            {
                let pos = layer_pos_at_head;
                let tp = crate::core::timeline::TrackerPoint {
                    id: format!("tracker_{}", tracker_count),
                    name: format!("Tracker {}", tracker_count + 1),
                    position: crate::core::property::Animatable::new_constant(pos),
                    search_size: 32.0,
                    feature_size: 16.0,
                    reference_pattern: None,
                };
                app.modify_project(|p| {
                    p.active_composition_mut().layers[idx].trackers.push(tp);
                });
            }

            if !has_media {
                ui.label(
                    egui::RichText::new(tr("Note: pixel tracking needs a Video or Image layer; others extrapolate transform velocity."))
                        .small()
                        .color(colors::ACCENT_YELLOW),
                );
            }

            let mut mocap_max = ui.ctx().data(|d| {
                d.get_temp::<u32>(egui::Id::new("mocap_max_features"))
                    .unwrap_or(64)
            });
            let mut mocap_spacing = ui.ctx().data(|d| {
                d.get_temp::<u32>(egui::Id::new("mocap_feature_spacing"))
                    .unwrap_or(12)
            });
            let mut mocap_search = ui.ctx().data(|d| {
                d.get_temp::<u32>(egui::Id::new("mocap_search_radius"))
                    .unwrap_or(16)
            });
            let mut mocap_confidence = ui.ctx().data(|d| {
                d.get_temp::<f32>(egui::Id::new("mocap_min_confidence"))
                    .unwrap_or(0.05)
            });
            ui.collapsing(tr("⚙ Markerless Capture Settings"), |ui| {
                ui.add(egui::Slider::new(&mut mocap_max, 1..=512).text(tr("Features")));
                ui.add(egui::Slider::new(&mut mocap_spacing, 1..=128).text(tr("Spacing px")));
                ui.add(egui::Slider::new(&mut mocap_search, 1..=128).text(tr("Search px")));
                ui.add(
                    egui::Slider::new(&mut mocap_confidence, 0.0..=1.0).text(tr("Min confidence")),
                );
                ui.ctx().data_mut(|d| {
                    d.insert_temp(egui::Id::new("mocap_max_features"), mocap_max);
                    d.insert_temp(egui::Id::new("mocap_feature_spacing"), mocap_spacing);
                    d.insert_temp(egui::Id::new("mocap_search_radius"), mocap_search);
                    d.insert_temp(egui::Id::new("mocap_min_confidence"), mocap_confidence);
                });
            });
            let mut active_tk_idx = ui.ctx().data(|d| {
                d.get_temp::<usize>(egui::Id::new("ae_active_tracker_pt_idx"))
                    .unwrap_or(0)
            });
            if tracker_count > 0 {
                if active_tk_idx >= tracker_count {
                    active_tk_idx = tracker_count - 1;
                }
                ui.horizontal(|ui| {
                    ui.label(tr("Active Tracker Point:"));
                    egui::ComboBox::from_id_salt("tracker_point_select")
                        .selected_text(tr_format("Tracker {}", &[&(active_tk_idx + 1).to_string()]))
                        .show_ui(ui, |ui| {
                            for t_i in 0..tracker_count {
                                if ui
                                    .selectable_value(
                                        &mut active_tk_idx,
                                        t_i,
                                        tr_format("Tracker {}", &[&(t_i + 1).to_string()]),
                                    )
                                    .clicked()
                                {
                                    ui.ctx().data_mut(|d| {
                                        d.insert_temp(
                                            egui::Id::new("ae_active_tracker_pt_idx"),
                                            active_tk_idx,
                                        );
                                    });
                                }
                            }
                        });
                });
            }

            ui.horizontal(|ui| {
                if custom_widgets::ae_button(ui, &tr("Analyze Forward (Work Area)")).on_hover_text(tr("Track the feature through the work area using real SAD matching + subpixel refinement")).clicked() {
                    let wa_out = app.playback.work_area_out.unwrap_or_else(|| {
                        app.history.current().active_composition().duration_frames.saturating_sub(1)
                    });
                    let start = current_frame.max(1).saturating_sub(1);
                    if wa_out > start {
                        if tracker_count == 0 {
                            app.toasts.error(tr("Add a tracker point first"));
                        } else {
                            app.modify_project(|p| {
                                let comp = p.active_composition_mut();
                                crate::core::tracker_engine::TrackerEngine::analyze_track_cancellable(
                                    comp, idx, active_tk_idx, start, wa_out, None,
                                );
                            });
                            app.toasts.info(tr_format(
                                "Tracked Tracker {} frames {}..{} on '{}'",
                                &[
                                    &(active_tk_idx + 1).to_string(),
                                    &start.to_string(),
                                    &wa_out.to_string(),
                                    layer_name.as_str(),
                                ],
                            ));
                        }
                    } else {
                        app.toasts.error(tr("Nothing to analyze: extend the work area past the playhead"));
                    }
                }
                if custom_widgets::ae_button_accent(ui, &tr("Markerless Optical Flow")).on_hover_text(tr("Track the selected point with dense forward/backward optical flow and confidence filtering")).clicked() {
                    let wa_out = app.playback.work_area_out.unwrap_or_else(|| {
                        app.history.current().active_composition().duration_frames.saturating_sub(1)
                    });
                    let start = current_frame;
                    if wa_out > start {
                        app.modify_project(|p| {
                            let comp = p.active_composition_mut();
                            crate::core::tracker_engine::TrackerEngine::analyze_markerless_tracks(
                                &mut comp.layers[idx], start, wa_out, mocap_max as usize, mocap_spacing,
                                2, mocap_search as i32, mocap_confidence,
                            );
                        });
                        app.toasts.info(tr("Optical-flow mocap generated keyframes for all tracker points"));
                    } else {
                        app.toasts.error(tr("Nothing to track: extend the work area past the playhead"));
                    }
                }
            });
            if let Some(summary) = ui
                .ctx()
                .data(|d| d.get_temp::<String>(egui::Id::new("mocap_pose_summary")))
            {
                ui.label(
                    egui::RichText::new(summary)
                        .small()
                        .color(colors::ACCENT_GREEN),
                );
            }
            if custom_widgets::ae_button(ui, &tr("Estimate Markerless Pose"))
                .on_hover_text(tr(
                    "Estimate and stabilize a 2D humanoid pose from the work area",
                ))
                .clicked()
            {
                let wa_out = app.playback.work_area_out.unwrap_or_else(|| {
                    app.history
                        .current()
                        .active_composition()
                        .duration_frames
                        .saturating_sub(1)
                });
                if wa_out >= current_frame {
                    let pose = {
                        let comp = app.history.current().active_composition();
                        crate::core::tracker_engine::TrackerEngine::estimate_markerless_pose(
                            &comp.layers[idx],
                            current_frame,
                            wa_out,
                            mocap_max as usize,
                            mocap_spacing,
                            2,
                            mocap_search as i32,
                        )
                    };
                    if let Some(pose) = pose {
                        let valid = pose
                            .frames
                            .iter()
                            .flat_map(|frame| frame.joints.iter())
                            .filter(|point| point.iter().all(|value| value.is_finite()))
                            .count();
                        let total = pose.frames.len().saturating_mul(
                            pose.frames
                                .first()
                                .map(|frame| frame.joints.len())
                                .unwrap_or(0),
                        );
                        let confidence = if pose.frames.is_empty() {
                            0.0
                        } else {
                            pose.frames
                                .iter()
                                .map(|frame| frame.confidence)
                                .sum::<f32>()
                                / pose.frames.len() as f32
                        };
                        let pose_frames = pose.frames.len().to_string();
                        let valid_joints = valid.to_string();
                        let total_joints = total.to_string();
                        let pose_confidence = format!("{:.0}", confidence * 100.0);
                        let summary = tr_format(
                            "Pose: {} frames • {}/{} joints valid • confidence {}%",
                            &[&pose_frames, &valid_joints, &total_joints, &pose_confidence],
                        );
                        let mut added = 0;
                        app.modify_project(|p| {
                            added = crate::core::tracker_engine::TrackerEngine::apply_pose_as_tracker_points(
                                &mut p.active_composition_mut().layers[idx], &pose, mocap_confidence,
                            );
                        });
                        ui.ctx().data_mut(|d| {
                            d.insert_temp(egui::Id::new("mocap_pose_summary"), summary)
                        });
                        app.toasts.info(tr_format(
                            "Markerless pose completed: {} tracker points added",
                            &[&added.to_string()],
                        ));
                    } else {
                        app.toasts
                            .error(tr("Pose estimation failed: no valid media frames"));
                    }
                } else {
                    app.toasts.error(tr(
                        "Nothing to estimate: extend the work area past the playhead",
                    ));
                }
            }
            if custom_widgets::ae_button(ui, &tr("✕ Clear Generated Pose"))
                .on_hover_text(tr(
                    "Remove generated pose trackers while keeping manual tracker points",
                ))
                .clicked()
            {
                let mut removed = 0usize;
                app.modify_project(|p| {
                    let trackers = &mut p.active_composition_mut().layers[idx].trackers;
                    let before = trackers.len();
                    trackers.retain(|tracker| !tracker.id.starts_with("pose_"));
                    removed = before - trackers.len();
                });
                ui.ctx()
                    .data_mut(|d| d.remove::<String>(egui::Id::new("mocap_pose_summary")));
                app.toasts.info(tr_format(
                    "Removed {} generated pose tracker(s)",
                    &[&removed.to_string()],
                ));
            }


}
