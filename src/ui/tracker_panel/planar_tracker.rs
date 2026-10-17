use super::workers::start_quad_track;
use crate::ui::custom_widgets;
use crate::ui::localization::{tr, tr_format};
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub(crate) fn draw_planar_tracker_section(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    idx: usize,
    current_frame: u32,
) {
    let tracker_count = app
        .history
        .current()
        .active_composition()
        .layers
        .get(idx)
        .map(|l| l.trackers.len())
        .unwrap_or(0);
    let mut active_tk_idx = ui.ctx().data(|d| {
        d.get_temp::<usize>(egui::Id::new("ae_active_tracker_pt_idx"))
            .unwrap_or(0)
    });
    if tracker_count > 0 && active_tk_idx >= tracker_count {
        active_tk_idx = tracker_count - 1;
    }
    ui.separator();
    ui.collapsing(tr("☕ Mocha Planar Surface Tracker"), |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 3.0);
            ui.label(tr("Track Motion:"));
            let mut track_trans = ui.ctx().data(|d| {
                d.get_temp::<bool>(egui::Id::new("planar_trans"))
                    .unwrap_or(true)
            });
            let mut track_rot_scale = ui.ctx().data(|d| {
                d.get_temp::<bool>(egui::Id::new("planar_rot_scale"))
                    .unwrap_or(true)
            });
            let mut track_shear = ui.ctx().data(|d| {
                d.get_temp::<bool>(egui::Id::new("planar_shear"))
                    .unwrap_or(true)
            });
            let mut track_persp = ui.ctx().data(|d| {
                d.get_temp::<bool>(egui::Id::new("planar_persp"))
                    .unwrap_or(true)
            });

            if ui.checkbox(&mut track_trans, tr("Translation")).changed() {
                ui.ctx()
                    .data_mut(|d| d.insert_temp(egui::Id::new("planar_trans"), track_trans));
            }
            if ui.checkbox(&mut track_rot_scale, tr("Scale/Rot")).changed() {
                ui.ctx().data_mut(|d| {
                    d.insert_temp(egui::Id::new("planar_rot_scale"), track_rot_scale)
                });
            }
            if ui.checkbox(&mut track_shear, tr("Shear")).changed() {
                ui.ctx()
                    .data_mut(|d| d.insert_temp(egui::Id::new("planar_shear"), track_shear));
            }
            if ui.checkbox(&mut track_persp, tr("Perspective")).changed() {
                ui.ctx()
                    .data_mut(|d| d.insert_temp(egui::Id::new("planar_persp"), track_persp));
            }
        });

        ui.horizontal(|ui| {
            let mut show_surf = ui.ctx().data(|d| {
                d.get_temp::<bool>(egui::Id::new("planar_show_surf"))
                    .unwrap_or(true)
            });
            let mut show_grid = ui.ctx().data(|d| {
                d.get_temp::<bool>(egui::Id::new("planar_show_grid"))
                    .unwrap_or(false)
            });
            if ui
                .checkbox(&mut show_surf, tr("Show Surface (Blue)"))
                .changed()
            {
                ui.ctx()
                    .data_mut(|d| d.insert_temp(egui::Id::new("planar_show_surf"), show_surf));
            }
            if ui
                .checkbox(&mut show_grid, tr("Show Planar Grid"))
                .changed()
            {
                ui.ctx()
                    .data_mut(|d| d.insert_temp(egui::Id::new("planar_show_grid"), show_grid));
            }
        });

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 3.0);
            if custom_widgets::ae_button_accent(ui, &tr("☕ Track Planar Surface"))
                .on_hover_text(tr("Track the four surface corners on a background worker"))
                .clicked()
            {
                if tracker_count < 4 {
                    app.toasts
                        .error(tr("Planar tracking needs four tracker points"));
                } else if app.quad_track_rx.is_some() {
                    app.toasts
                        .info(tr("A four-corner tracking job is already running"));
                } else {
                    let end_frame = app.playback.work_area_out.unwrap_or_else(|| {
                        app.history
                            .current()
                            .active_composition()
                            .duration_frames
                            .saturating_sub(1)
                    });
                    if end_frame <= current_frame {
                        app.toasts.error(tr(
                            "Nothing to track: extend the work area past the playhead",
                        ));
                    } else {
                        start_quad_track(
                            app,
                            ui.ctx(),
                            idx,
                            current_frame,
                            end_frame,
                            crate::app_state::QuadTrackPurpose::PlanarSurface,
                        );
                    }
                }
            }
            if custom_widgets::ae_button(ui, &tr("Align Surface Corners"))
                .on_hover_text(tr("Snap planar surface to layer bounding box"))
                .clicked()
            {
                app.modify_project(|p| {
                    let comp = p.active_composition_mut();
                    if let Some(layer) = comp.layers.get_mut(idx) {
                        let [width, height] = layer.bounding_size();
                        let position = layer.transform.position.evaluate(current_frame);
                        let scale = layer.transform.scale.evaluate(current_frame);
                        let half_width = width * scale[0].abs() / 200.0;
                        let half_height = height * scale[1].abs() / 200.0;
                        let corners = [
                            [position[0] - half_width, position[1] - half_height],
                            [position[0] + half_width, position[1] - half_height],
                            [position[0] + half_width, position[1] + half_height],
                            [position[0] - half_width, position[1] + half_height],
                        ];
                        for (tracker, corner) in layer.trackers.iter_mut().take(4).zip(corners) {
                            tracker
                                .position
                                .set_keyframe(crate::core::keyframe::Keyframe::new(
                                    current_frame,
                                    corner,
                                    crate::core::keyframe::InterpolationType::Linear,
                                ));
                        }
                    }
                });
                app.toasts
                    .info(tr("Planar surface corners aligned to layer bounds"));
            }
        });
    });

    // ── Perspective Corner Pin (4-point) tracking ──
    ui.add_space(8.0);
    ui.separator();
    ui.label(
        egui::RichText::new(tr("Perspective Corner Pin (4-point)"))
            .strong()
            .color(colors::ACCENT_CYAN),
    );
    ui.label(tr("Corners = Tracker 1..4 in order: TL, TR, BR, BL."));
    let quad_conf_id = egui::Id::new("kagari_quad_conf");
    if let Some(prev) = ui.memory(|m| m.data.get_temp::<String>(quad_conf_id)) {
        ui.label(
            egui::RichText::new(tr_format("Last run lock quality: {}", &[&prev]))
                .small()
                .color(colors::ACCENT_GREEN),
        );
    }
    if app.quad_track_rx.is_some() {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(tr("Analyzing quad track…"));
            if custom_widgets::ae_button(ui, &tr("Cancel")).clicked() {
                if let Some(flag) = &app.quad_track_cancel_flag {
                    flag.store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
        });
    } else if ui
        .add_enabled(
            tracker_count >= 4,
            egui::Button::new(tr("Analyze Quad (Work Area)")),
        )
        .on_disabled_hover_text(tr("Add at least 4 tracker points first"))
        .clicked()
    {
        let wa_out = app.playback.work_area_out.unwrap_or_else(|| {
            app.history
                .current()
                .active_composition()
                .duration_frames
                .saturating_sub(1)
        });
        if wa_out <= current_frame {
            app.toasts.error(tr(
                "Nothing to analyze: extend the work area past the playhead",
            ));
        } else if !start_quad_track(
            app,
            ui.ctx(),
            idx,
            current_frame,
            wa_out,
            crate::app_state::QuadTrackPurpose::Perspective,
        ) {
            app.toasts.error(tr("Four tracker points are required"));
        }
    }

    ui.horizontal(|ui| {
        if custom_widgets::ae_button(ui, &tr("◀◀ 1f"))
            .on_hover_text(tr("Analyze 1 Frame Backward"))
            .clicked()
            && current_frame > 0
        {
            let f = current_frame - 1;
            app.modify_project(|p| {
                let comp = p.active_composition_mut();
                crate::core::tracker_engine::TrackerEngine::analyze_track_cancellable(
                    comp,
                    idx,
                    0,
                    f.saturating_sub(1),
                    f,
                    None,
                );
            });
        }
        if custom_widgets::ae_button(ui, &tr("1f ▶▶"))
            .on_hover_text(tr("Analyze 1 Frame Forward"))
            .clicked()
        {
            let total = app.history.current().active_composition().duration_frames;
            let f = (current_frame + 1).min(total.saturating_sub(1));
            app.modify_project(|p| {
                let comp = p.active_composition_mut();
                crate::core::tracker_engine::TrackerEngine::analyze_track_cancellable(
                    comp,
                    idx,
                    0,
                    current_frame,
                    f,
                    None,
                );
            });
        }
    });

    ui.add_space(8.0);
    ui.separator();
    ui.label(tr("Apply:"));

    ui.horizontal(|ui| {
        if custom_widgets::ae_button(ui, &tr("Reset Track"))
            .on_hover_text(tr("Remove all tracked keyframes from this tracker"))
            .clicked()
        {
            app.modify_project(|p| {
                let comp = p.active_composition_mut();
                if let Some(tp) = comp.layers.get_mut(idx) {
                    for t in tp.trackers.iter_mut() {
                        t.position = crate::core::property::Animatable::new_constant(
                            t.position.evaluate(current_frame),
                        );
                    }
                }
            });
            app.toasts.info(tr("Tracker keyframes reset"));
        }
        if custom_widgets::ae_button(ui, &tr("Apply to Position"))
            .on_hover_text(tr(
                "Bake tracker motion into a target layer's position (pick below)",
            ))
            .clicked()
        {
            if let Some(target_idx) = app.tracker_apply_target {
                app.modify_project(|p| {
                    let comp = p.active_composition_mut();
                    crate::core::tracker_engine::TrackerEngine::apply_tracker_to_target(
                        comp,
                        idx,
                        active_tk_idx,
                        target_idx,
                        true,
                        false,
                    );
                });
                app.toasts.info(tr_format(
                    "Applied Tracker {} position to layer {}",
                    &[
                        &(active_tk_idx + 1).to_string(),
                        &(target_idx + 1).to_string(),
                    ],
                ));
            } else {
                app.toasts
                    .info(tr("Select the target layer in 'Apply to Layer' dropdown"));
            }
        }
    });

    // Target picker + apply
    egui::ComboBox::from_id_salt("tracker_apply_target")
        .selected_text(
            app.tracker_apply_target
                .map(|i| tr_format("Layer {}", &[&(i + 1).to_string()]))
                .unwrap_or_else(|| tr("Target layer...")),
        )
        .show_ui(ui, |ui| {
            let names: Vec<(usize, String)> = app
                .history
                .current()
                .active_composition()
                .layers
                .iter()
                .enumerate()
                .map(|(i, l)| (i, l.name.clone()))
                .collect();
            for (ti, tname) in names {
                ui.selectable_value(
                    &mut app.tracker_apply_target,
                    Some(ti),
                    tr_format("Layer {}. {}", &[&(ti + 1).to_string(), &tname]),
                );
            }
        });
    if let Some(target_idx) = app.tracker_apply_target {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 3.0);
            if custom_widgets::ae_button_accent(ui, &tr("Apply Motion → Target")).clicked() {
                app.modify_project(|p| {
                    let comp = p.active_composition_mut();
                    crate::core::tracker_engine::TrackerEngine::apply_tracker_to_target(
                        comp,
                        idx,
                        active_tk_idx,
                        target_idx,
                        true,
                        false,
                    );
                });
                app.toasts.info(tr_format(
                    "Applied tracking to layer {}",
                    &[&(target_idx + 1).to_string()],
                ));
            }
            if custom_widgets::ae_button(ui, &tr("Apply as Corner Pin → Target"))
                .on_hover_text(tr(
                    "Create a CornerPin effect on the target using the tracked quad corners",
                ))
                .clicked()
            {
                let end_frame = app.playback.work_area_out.unwrap_or_else(|| {
                    app.history
                        .current()
                        .active_composition()
                        .duration_frames
                        .saturating_sub(1)
                });
                let mut applied = false;
                app.modify_project(|p| {
                    let comp = p.active_composition_mut();
                    let Some(src_layer) = comp.layers.get(idx) else {
                        return;
                    };
                    if src_layer.trackers.len() < 4 || end_frame < current_frame {
                        return;
                    }
                    let track = crate::core::tracker_engine::QuadTrackData {
                        frames: (current_frame..=end_frame).collect(),
                        corners: (current_frame..=end_frame)
                            .map(|frame| {
                                [
                                    src_layer.trackers[0].position.evaluate(frame),
                                    src_layer.trackers[1].position.evaluate(frame),
                                    src_layer.trackers[2].position.evaluate(frame),
                                    src_layer.trackers[3].position.evaluate(frame),
                                ]
                            })
                            .collect(),
                    };
                    if let Some(target) = comp.layers.get_mut(target_idx) {
                        applied = crate::core::tracker_engine::apply_quad_track_to_corner_pin(
                            target, &track,
                        );
                    }
                });
                if applied {
                    app.toasts.info(tr_format(
                        "Applied animated Corner Pin to layer {}",
                        &[&(target_idx + 1).to_string()],
                    ));
                } else {
                    app.toasts
                        .error(tr("Could not apply Corner Pin: need four tracked points"));
                }
            }
            if custom_widgets::ae_button(ui, &tr("Stabilize Motion"))
                .on_hover_text(tr(
                    "Cancel camera shake by inverting motion onto target anchor/position",
                ))
                .clicked()
            {
                app.modify_project(|p| {
                    let comp = p.active_composition_mut();
                    crate::core::tracker_engine::TrackerEngine::apply_stabilize_tracker_to_target(
                        comp,
                        idx,
                        active_tk_idx,
                        target_idx,
                        true,
                        true,
                    );
                });
                app.toasts.info(tr_format(
                    "Stabilized motion applied to layer {}",
                    &[&(target_idx + 1).to_string()],
                ));
            }
        });
    }

    ui.horizontal(|ui| {
        if custom_widgets::ae_button_accent(ui, &tr("Apply Motion → New Null"))
            .on_hover_text(tr(
                "Create a new Null layer and bind the tracked motion keyframes to it",
            ))
            .clicked()
        {
            app.modify_project(|p| {
                let comp = p.active_composition_mut();
                let null_idx = comp.layers.len();
                let dur = comp.duration_frames;
                let null_layer = crate::core::timeline::Layer::new(
                    format!("null_track_{}", null_idx + 1),
                    format!("Tracker {} Null", idx + 1),
                    crate::core::timeline::LayerType::Null,
                    dur,
                );
                comp.add_layer(null_layer);
                crate::core::tracker_engine::TrackerEngine::apply_tracker_to_target(
                    comp,
                    idx,
                    active_tk_idx,
                    null_idx,
                    true,
                    false,
                );
            });
            app.toasts
                .info(tr("Created new Null layer with tracked motion!"));
        }
        if custom_widgets::ae_button(ui, &tr("Smooth Track"))
            .on_hover_text(tr(
                "Apply Gaussian temporal filter to reduce jitter in tracked keyframes",
            ))
            .clicked()
        {
            let mut smoothed = false;
            app.modify_project(|p| {
                let comp = p.active_composition_mut();
                if let Some(src_layer) = comp.layers.get_mut(idx) {
                    for tracker in &mut src_layer.trackers {
                        if let crate::core::property::Animatable::Animated(ref kfs) =
                            tracker.position
                        {
                            let new_kfs =
                                crate::core::tracker_engine::smooth_tracker_keyframes(kfs, 2);
                            tracker.position = crate::core::property::Animatable::Animated(new_kfs);
                            smoothed = true;
                        }
                    }
                }
            });
            if smoothed {
                crate::core::frame_cache::bump_version();
                app.toasts.info(tr(
                    "Tracked keyframes smoothed with Gaussian temporal filter",
                ));
            } else {
                app.toasts
                    .error(tr("No animated keyframes found on tracker"));
            }
        }
    });
}
