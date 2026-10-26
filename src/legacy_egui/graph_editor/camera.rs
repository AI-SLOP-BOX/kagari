use crate::ui::theme::colors;
use eframe::egui;

use super::{
    remove_camera_key, set_camera_key_ease, set_camera_key_interpolation, GraphKeyframeDrag,
};

fn camera_animation_mut<'a>(
    camera: &'a mut crate::core::timeline::Camera3D,
    property: &str,
) -> &'a mut Option<crate::core::property::Animatable<f32>> {
    match property {
        "FOV" => &mut camera.fov_animation,
        "Focus Distance" => &mut camera.focus_distance_animation,
        "Aperture" => &mut camera.aperture_animation,
        "DOF Enabled" => &mut camera.dof_enabled_animation,
        _ => &mut camera.dof_max_blur_animation,
    }
}

pub fn draw_camera_lens_graph(
    ui: &mut egui::Ui,
    camera: &mut crate::core::timeline::Camera3D,
    duration_frames: u32,
    current_frame: u32,
    project_changed: &mut bool,
) {
    let id = egui::Id::new("camera_lens_graph_property");
    let mut property = ui
        .ctx()
        .data(|d| d.get_temp::<String>(id))
        .unwrap_or_else(|| "FOV".into());
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Camera Lens Graph").strong());
        egui::ComboBox::from_id_salt(id)
            .selected_text(&property)
            .show_ui(ui, |ui| {
                for name in [
                    "FOV",
                    "Focus Distance",
                    "Aperture",
                    "DOF Blur",
                    "DOF Enabled",
                ] {
                    if ui.selectable_label(property == name, name).clicked() {
                        property = name.into();
                    }
                }
            });
    });
    ui.ctx().data_mut(|d| d.insert_temp(id, property.clone()));
    ui.horizontal(|ui| {
        if ui.small_button("× Remove Key").clicked() {
            let removed = match property.as_str() {
                "FOV" => remove_camera_key(&mut camera.fov_animation, current_frame),
                "Focus Distance" => {
                    remove_camera_key(&mut camera.focus_distance_animation, current_frame)
                }
                "Aperture" => remove_camera_key(&mut camera.aperture_animation, current_frame),
                "DOF Enabled" => {
                    remove_camera_key(&mut camera.dof_enabled_animation, current_frame)
                }
                _ => remove_camera_key(&mut camera.dof_max_blur_animation, current_frame),
            };
            *project_changed |= removed;
        }
    });
    ui.horizontal(|ui| {
        ui.label("Interpolation:");
        for (label, interpolation) in [
            ("Linear", crate::core::keyframe::InterpolationType::Linear),
            ("Hold", crate::core::keyframe::InterpolationType::Hold),
        ] {
            if ui.small_button(label).clicked() {
                let changed = match property.as_str() {
                    "FOV" => set_camera_key_interpolation(
                        &mut camera.fov_animation,
                        current_frame,
                        interpolation,
                    ),
                    "Focus Distance" => set_camera_key_interpolation(
                        &mut camera.focus_distance_animation,
                        current_frame,
                        interpolation,
                    ),
                    "Aperture" => set_camera_key_interpolation(
                        &mut camera.aperture_animation,
                        current_frame,
                        interpolation,
                    ),
                    "DOF Enabled" => set_camera_key_interpolation(
                        &mut camera.dof_enabled_animation,
                        current_frame,
                        interpolation,
                    ),
                    _ => set_camera_key_interpolation(
                        &mut camera.dof_max_blur_animation,
                        current_frame,
                        interpolation,
                    ),
                };
                *project_changed |= changed;
            }
        }
    });
    ui.horizontal(|ui| {
        ui.label("Ease:");
        for (label, preset) in [
            ("F9", crate::core::keyframe::EasePreset::Standard),
            ("Ease In", crate::core::keyframe::EasePreset::EaseIn),
            ("Ease Out", crate::core::keyframe::EasePreset::EaseOut),
        ] {
            if ui.small_button(label).clicked() {
                let changed = match property.as_str() {
                    "FOV" => set_camera_key_ease(&mut camera.fov_animation, current_frame, preset),
                    "Focus Distance" => set_camera_key_ease(
                        &mut camera.focus_distance_animation,
                        current_frame,
                        preset,
                    ),
                    "Aperture" => {
                        set_camera_key_ease(&mut camera.aperture_animation, current_frame, preset)
                    }
                    "DOF Enabled" => set_camera_key_ease(
                        &mut camera.dof_enabled_animation,
                        current_frame,
                        preset,
                    ),
                    _ => set_camera_key_ease(
                        &mut camera.dof_max_blur_animation,
                        current_frame,
                        preset,
                    ),
                };
                *project_changed |= changed;
            }
        }
    });
    let track = match property.as_str() {
        "FOV" => camera.fov_animation.as_ref(),
        "Focus Distance" => camera.focus_distance_animation.as_ref(),
        "Aperture" => camera.aperture_animation.as_ref(),
        "DOF Enabled" => camera.dof_enabled_animation.as_ref(),
        _ => camera.dof_max_blur_animation.as_ref(),
    };
    let Some(track) = track else {
        ui.label("No keyframes yet — use in Camera Settings.");
        return;
    };
    let Some(keys) = track.keyframes().map(|keys| keys.to_vec()) else {
        return;
    };
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 110.0),
        egui::Sense::click_and_drag(),
    );
    let min = keys.iter().map(|k| k.value).fold(f32::INFINITY, f32::min);
    let max = keys
        .iter()
        .map(|k| k.value)
        .fold(f32::NEG_INFINITY, f32::max);
    let range = (max - min).max(0.001);
    let end = duration_frames
        .max(keys.last().map(|k| k.frame).unwrap_or(0))
        .max(1);
    let point = |frame: u32, value: f32| {
        egui::pos2(
            rect.left() + frame as f32 / end as f32 * rect.width(),
            rect.bottom() - 6.0 - (value - min) / range * (rect.height() - 12.0),
        )
    };
    let clamp_value = |value: f32| match property.as_str() {
        "FOV" => value.clamp(1.0, 179.0),
        "Focus Distance" | "Aperture" => value.max(0.0),
        "DOF Enabled" => value.clamp(0.0, 1.0),
        _ => value.clamp(1.0, 64.0),
    };
    for pair in keys.windows(2) {
        let start = &pair[0];
        let end_key = &pair[1];
        let bezier = match start.interpolation {
            crate::core::keyframe::InterpolationType::Bezier {
                custom_bezier: Some(points),
                ..
            } => Some(points),
            _ => None,
        };
        let mut previous = point(start.frame, start.value);
        for step in 1..=24 {
            let normalized = step as f32 / 24.0;
            let eased = bezier
                .map(|[x1, y1, x2, y2]| {
                    let t =
                        crate::core::keyframe::solve_bezier_eased_time(normalized, x1, y1, x2, y2);
                    let omt = 1.0 - t;
                    3.0 * omt * omt * t * y1 + 3.0 * omt * t * t * y2 + t * t * t
                })
                .unwrap_or_else(|| match start.interpolation {
                    crate::core::keyframe::InterpolationType::Hold => 0.0,
                    _ => normalized,
                });
            let frame = start.frame as f32
                + (end_key.frame.saturating_sub(start.frame) as f32) * normalized;
            let value = start.value + (end_key.value - start.value) * eased;
            let next = point(frame.round() as u32, value);
            ui.painter().line_segment(
                [previous, next],
                egui::Stroke::new(2.0_f32, colors::ACCENT_CYAN),
            );
            previous = next;
        }
    }
    for key in &keys {
        ui.painter()
            .circle_filled(point(key.frame, key.value), 4.0, colors::TIMELINE_KEYFRAME);
    }
    let x = rect.left() + current_frame.min(end) as f32 / end as f32 * rect.width();
    ui.painter().line_segment(
        [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
        egui::Stroke::new(1.0_f32, colors::HANDLE_HOVER_FILL),
    );
    let drag_id = egui::Id::new(("camera_lens_graph_drag", property.as_str()));
    if response.drag_started() {
        if let Some(pointer) = response.interact_pointer_pos() {
            let nearest = keys
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    point(a.frame, a.value)
                        .distance(pointer)
                        .partial_cmp(&point(b.frame, b.value).distance(pointer))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .filter(|(_, key)| point(key.frame, key.value).distance(pointer) <= 12.0)
                .map(|(index, _)| index);
            let drag = nearest.and_then(|index| {
                keys.get(index).map(|key| GraphKeyframeDrag {
                    original_frame: key.frame,
                    current_frame: key.frame,
                    original_value: key.value,
                    current_value: key.value,
                })
            });
            ui.ctx().data_mut(|data| data.insert_temp(drag_id, drag));
        }
    }
    if response.dragged() {
        if let Some(pointer) = response.interact_pointer_pos() {
            let drag = ui
                .ctx()
                .data(|data| data.get_temp::<GraphKeyframeDrag>(drag_id));
            if let Some(state) = drag {
                let frame = ((pointer.x - rect.left()) / rect.width() * end as f32)
                    .round()
                    .clamp(0.0, end as f32) as u32;
                let value = clamp_value(
                    min + ((rect.bottom() - 6.0 - pointer.y) / (rect.height() - 12.0) * range),
                );
                let track = camera_animation_mut(camera, &property);
                let moved = track
                    .as_mut()
                    .is_some_and(|track| track.move_keyframe(state.current_frame, frame));
                let updated = track.as_mut().is_some_and(|track| {
                    let before = track.evaluate(frame);
                    track.set_value_at_frame(frame, value);
                    before != value
                });
                *project_changed |= moved || updated;
                ui.ctx().data_mut(|data| {
                    data.insert_temp(
                        drag_id,
                        GraphKeyframeDrag {
                            current_frame: frame,
                            current_value: value,
                            ..state
                        },
                    )
                });
            }
        }
    }
    if response.drag_stopped() {
        ui.ctx()
            .data_mut(|data| data.remove::<GraphKeyframeDrag>(drag_id));
    }
    if response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            let frame = ((pos.x - rect.left()) / rect.width() * end as f32)
                .round()
                .clamp(0.0, end as f32) as u32;
            let value =
                clamp_value(min + ((rect.bottom() - 6.0 - pos.y) / (rect.height() - 12.0) * range));
            match property.as_str() {
                "FOV" => camera.set_fov_at(frame, value),
                "Focus Distance" => camera.set_focus_distance_at(frame, value),
                "Aperture" => camera.set_aperture_at(frame, value),
                "DOF Enabled" => camera.set_dof_enabled_at(frame, value >= 0.5),
                _ => camera.set_dof_max_blur_at(frame, value),
            }
            *project_changed = true;
        }
    }
}
