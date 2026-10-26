use crate::core::timeline::Composition;
use crate::ui::inspector_property::draw_property_ui;
use crate::ui::kui;
use crate::ui::theme::colors;
use eframe::egui;

fn remove_key_at<T>(track: &mut Option<crate::core::property::Animatable<T>>, frame: u32) {
    if let Some(crate::core::property::Animatable::Animated(keys)) = track {
        keys.retain(|key| key.frame != frame);
        if keys.is_empty() {
            *track = None;
        }
    }
}

pub fn draw_camera_settings(
    ui: &mut egui::Ui,
    comp: &mut Composition,
    current_frame: u32,
    project_changed: &mut bool,
    next_frame: &mut Option<u32>,
) {
    ui.collapsing("Active Camera Settings", |ui| {
        let duration_frames = comp.duration_frames;
        let cam = comp.resolve_camera_mut();
        kui::toggle(ui, &mut cam.active, "Camera Active");

        let (changed, key, remove, fov_value) = kui::property(ui, "Field of View", |ui| {
            let mut fov_value = cam.fov_at(current_frame);
            let changed = kui::slider(ui, &mut fov_value, 10.0..=120.0, "°").changed();
            let key = kui::button(ui, "◆", kui::ButtonKind::Quiet).clicked();
            let remove = kui::button(ui, "×", kui::ButtonKind::Quiet)
                .on_hover_text("Remove FOV keyframe at current frame")
                .clicked();
            (changed, key, remove, fov_value)
        });
        if remove {
            remove_key_at(&mut cam.fov_animation, current_frame);
            *project_changed = true;
        } else if changed || key {
            cam.set_fov_at(current_frame, fov_value);
            *project_changed = true;
        }

        let (changed, key, remove, focus_value) = kui::property(ui, "Focus Distance", |ui| {
            let mut focus_value = cam.focus_distance_at(current_frame);
            let changed = ui
                .add(kui::drag(&mut focus_value).speed(10.0).unit(" mm"))
                .changed();
            let key = kui::button(ui, "◆", kui::ButtonKind::Quiet).clicked();
            let remove = kui::button(ui, "×", kui::ButtonKind::Quiet)
                .on_hover_text("Remove focus-distance keyframe at current frame")
                .clicked();
            (changed, key, remove, focus_value)
        });
        {
            if remove {
                remove_key_at(&mut cam.focus_distance_animation, current_frame);
                *project_changed = true;
            } else if changed || key {
                cam.set_focus_distance_at(current_frame, focus_value);
                *project_changed = true;
            }
        }

        let (changed, key, remove, aperture_value) = kui::property(ui, "Aperture f/", |ui| {
            let mut aperture_value = cam.aperture_at(current_frame);
            let changed = kui::slider(ui, &mut aperture_value, 0.95..=22.0, "").changed();
            let key = kui::button(ui, "◆", kui::ButtonKind::Quiet).clicked();
            let remove = kui::button(ui, "×", kui::ButtonKind::Quiet)
                .on_hover_text("Remove aperture keyframe at current frame")
                .clicked();
            (changed, key, remove, aperture_value)
        });
        {
            if remove {
                remove_key_at(&mut cam.aperture_animation, current_frame);
                *project_changed = true;
            } else if changed || key {
                cam.set_aperture_at(current_frame, aperture_value);
                *project_changed = true;
            }
        }

        let lens_tracks = [
            ("FOV", cam.fov_animation.as_ref()),
            ("Focus", cam.focus_distance_animation.as_ref()),
            ("Aperture", cam.aperture_animation.as_ref()),
            ("DOF Blur", cam.dof_max_blur_animation.as_ref()),
            ("DOF Enabled", cam.dof_enabled_animation.as_ref()),
        ];
        if lens_tracks.iter().any(|(_, track)| track.is_some()) {
            ui.label(egui::RichText::new("Lens Animation").small().strong());
            let (rect, _) = ui
                .allocate_exact_size(egui::vec2(ui.available_width(), 68.0), egui::Sense::hover());
            ui.painter()
                .rect_filled(rect, 3.0, egui::Color32::from_gray(28));
            for (row, (_, track)) in lens_tracks.iter().enumerate() {
                ui.painter().text(
                    egui::pos2(rect.left() + 4.0, rect.top() + row as f32 * 13.0 + 9.0),
                    egui::Align2::LEFT_CENTER,
                    lens_tracks[row].0,
                    egui::FontId::proportional(8.0),
                    colors::TEXT_MUTED,
                );
                if let Some(track) = track {
                    if let Some(keys) = track.keyframes() {
                        let min_frame = keys.first().map(|k| k.frame).unwrap_or(0);
                        let max_frame = keys
                            .last()
                            .map(|k| k.frame)
                            .unwrap_or(min_frame)
                            .max(min_frame + 1);
                        let min_value = keys.iter().map(|k| k.value).fold(f32::INFINITY, f32::min);
                        let max_value = keys
                            .iter()
                            .map(|k| k.value)
                            .fold(f32::NEG_INFINITY, f32::max);
                        let range = (max_value - min_value).max(0.001);
                        let points: Vec<_> = keys
                            .iter()
                            .map(|key| {
                                let x = rect.left()
                                    + ((key.frame - min_frame) as f32
                                        / (max_frame - min_frame) as f32)
                                        * rect.width();
                                let y = rect.top() + row as f32 * 13.0 + 11.0
                                    - ((key.value - min_value) / range) * 8.0;
                                egui::pos2(x, y)
                            })
                            .collect();
                        for pair in points.windows(2) {
                            ui.painter().line_segment(
                                [pair[0], pair[1]],
                                egui::Stroke::new(1.0_f32, colors::ACCENT_CYAN),
                            );
                        }
                    }
                }
            }
            let end_frame = lens_tracks
                .iter()
                .filter_map(|(_, track)| {
                    track
                        .and_then(|t| t.keyframes())
                        .and_then(|keys| keys.last().map(|key| key.frame))
                })
                .max()
                .unwrap_or(1)
                .max(current_frame)
                .max(1);
            let x = rect.left() + (current_frame as f32 / end_frame as f32) * rect.width();
            ui.painter().line_segment(
                [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                egui::Stroke::new(1.0_f32, colors::HANDLE_HOVER_FILL),
            );
        }

        crate::ui::graph_editor::draw_camera_lens_graph(
            ui,
            cam,
            duration_frames,
            current_frame,
            project_changed,
        );

        // ── Depth of Field ──
        ui.separator();
        ui.label(
            egui::RichText::new("Depth of Field")
                .small()
                .strong()
                .color(colors::ACCENT_CYAN),
        );

        let mut enabled = cam.dof_enabled_at(current_frame);
        if kui::property(ui, "Depth of Field", |ui| {
            kui::toggle(ui, &mut enabled, "Enabled")
        })
        .changed()
        {
            cam.set_dof_enabled_at(current_frame, enabled);
            *project_changed = true;
        }

        if cam.dof_enabled_at(current_frame) {
            let (changed, key, remove, blur) = kui::property(ui, "Max Blur Radius", |ui| {
                let mut blur = cam.dof_max_blur_at(current_frame);
                let changed = kui::slider(ui, &mut blur, 1.0..=64.0, " px").changed();
                let key = kui::button(ui, "◆", kui::ButtonKind::Quiet).clicked();
                let remove = kui::button(ui, "×", kui::ButtonKind::Quiet).clicked();
                (changed, key, remove, blur)
            });
            {
                if remove {
                    remove_key_at(&mut cam.dof_max_blur_animation, current_frame);
                    *project_changed = true;
                } else if changed || key {
                    cam.set_dof_max_blur_at(current_frame, blur);
                    *project_changed = true;
                }
            }

            if kui::property(ui, "Iris Shape", |ui| {
                kui::combo(
                    ui,
                    "iris_shape",
                    &mut cam.dof_iris_sides,
                    &[
                        (0, "Circle"),
                        (3, "Triangle"),
                        (5, "Pentagon"),
                        (6, "Hexagon"),
                        (8, "Octagon"),
                    ],
                )
            }) {
                *project_changed = true;
            }
        }

        ui.label("Camera Transform:");
        let cam_pos_before = cam.transform.position.clone();
        if let Some(nf) = draw_property_ui(
            current_frame,
            ui,
            "  Pos",
            &mut cam.transform.position,
            |ui, val| {
                kui::vector(ui, val, f32::NEG_INFINITY..=f32::INFINITY, 1.0, "");
            },
        ) {
            *next_frame = Some(nf);
        }
        if cam_pos_before != cam.transform.position {
            *project_changed = true;
        }

        let cam_rot_before = cam.transform.rotation.clone();
        if let Some(nf) = draw_property_ui(
            current_frame,
            ui,
            "  Rot",
            &mut cam.transform.rotation,
            |ui, val| {
                kui::vector_axes(
                    ui,
                    val,
                    f32::NEG_INFINITY..=f32::INFINITY,
                    1.0,
                    "",
                    ["P", "Y", "R"],
                );
            },
        ) {
            *next_frame = Some(nf);
        }
        if cam_rot_before != cam.transform.rotation {
            *project_changed = true;
        }
    });
}
