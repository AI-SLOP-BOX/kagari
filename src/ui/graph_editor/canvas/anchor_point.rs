use eframe::egui;
use crate::core::timeline::Layer;
use crate::ui::graph_editor::channel_ops::{
    axis_3d, is_effect_property, move_and_set_channel, pin_anim_mut,
};
use crate::ui::graph_editor::interpolation_ops::{
    move_effect_channel_keyframe, move_effect_scalar_keyframe, set_effect_channel_at_frame,
};
use crate::ui::graph_editor::keyframe_access::*;
use crate::ui::graph_editor::GraphKeyframeDrag;
use crate::ui::theme::colors;

pub fn handle_anchor_interaction(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    total_f: u32,
    val_range: f32,
    layer: &mut Layer,
    graph_prop: &str,
    kf_idx: usize,
    kf_frame: u32,
    kf_val: f32,
    drag_display: Option<GraphKeyframeDrag>,
    _display_frame: u32,
    pt: egui::Pos2,
    drag_state_id: egui::Id,
    project_changed: &mut bool,
) -> Option<usize> {
    let anchor_rect = egui::Rect::from_center_size(pt, egui::vec2(14.0, 14.0));
    #[cfg(test)]
    ui.ctx().data_mut(|d| {
        d.insert_temp(
            egui::Id::new(("ae_graph_anchor_rect", &layer.id, graph_prop, kf_frame)),
            anchor_rect,
        );
    });
    if !ui.is_rect_visible(anchor_rect) && drag_display.is_none() {
        return None;
    }
    let anchor_token = drag_display
        .map(|drag| drag.original_frame as usize)
        .unwrap_or(kf_frame as usize);
    let anchor_resp = ui.interact(
        anchor_rect,
        egui::Id::new(("graph_anchor", &layer.id, graph_prop, anchor_token)),
        egui::Sense::click_and_drag(),
    );
    if anchor_resp.hovered() {
        ui.ctx().data_mut(|d| {
            d.insert_temp(
                egui::Id::new(("ae_graph_hovered_kf", &layer.id, graph_prop)),
                kf_idx,
            )
        });
    }
    if anchor_resp.secondary_clicked() {
        let prop = graph_prop.to_string();
        if prop.starts_with("3D ") {
            if let Some(kfs) = layer_keyframes_of_vec3(layer, &prop) {
                if let Some(index) = kfs.iter().position(|key| key.frame == kf_frame) {
                    kfs.remove(index);
                    *project_changed = true;
                }
            }
        } else if matches!(
            prop.as_str(),
            "Position X" | "Position Y" | "Scale X" | "Scale Y"
        ) {
            if let Some(kfs) = layer_keyframes_of_vec2(layer, &prop) {
                if let Some(index) = kfs.iter().position(|key| key.frame == kf_frame) {
                    kfs.remove(index);
                    *project_changed = true;
                }
            }
        } else if is_effect_property(&prop) {
            if let Some(kfs) = effect_keyframes_of_f32(layer, &prop) {
                if let Some(index) = kfs.iter().position(|key| key.frame == kf_frame) {
                    kfs.remove(index);
                    *project_changed = true;
                }
            }
        } else if let Some(kfs) = layer_keyframes_of_f32(layer, &prop) {
            if let Some(index) = kfs.iter().position(|key| key.frame == kf_frame) {
                kfs.remove(index);
                *project_changed = true;
            }
        }
        return None;
    }
    if anchor_resp.drag_started() {
        let state = GraphKeyframeDrag {
            original_frame: kf_frame,
            current_frame: kf_frame,
            original_value: kf_val,
            current_value: kf_val,
        };
        ui.ctx().data_mut(|d| d.insert_temp(drag_state_id, state));
    }
    if anchor_resp.dragged() {
        let state = ui
            .ctx()
            .data(|d| d.get_temp::<GraphKeyframeDrag>(drag_state_id))
            .unwrap_or(GraphKeyframeDrag {
                original_frame: kf_frame,
                current_frame: kf_frame,
                original_value: kf_val,
                current_value: kf_val,
            });
        let delta_frames =
            (anchor_resp.drag_delta().x / rect.width() * total_f as f32).round() as i32;
        let new_frame = (state.original_frame as i32 + delta_frames)
            .clamp(0, total_f as i32) as u32;
        let delta_val = -anchor_resp.drag_delta().y / (rect.height() - 8.0) * val_range;
        let new_value = state.original_value + delta_val;
        let axis = if graph_prop.starts_with("3D ") {
            axis_3d(graph_prop)
        } else {
            usize::from(graph_prop.ends_with('Y'))
        };

        let changed = if is_effect_property(graph_prop) && !graph_prop.contains('|') {
            let moved = state.current_frame != new_frame
                && move_effect_scalar_keyframe(
                    layer,
                    graph_prop,
                    state.current_frame,
                    new_frame,
                );
            let updated =
                set_effect_channel_at_frame(layer, graph_prop, new_frame, new_value);
            moved || updated
        } else if graph_prop == "Rotation" {
            move_and_set_channel(
                &mut layer.transform.rotation,
                state.current_frame,
                new_frame,
                0,
                new_value,
            )
        } else if graph_prop == "Opacity" {
            move_and_set_channel(
                &mut layer.transform.opacity,
                state.current_frame,
                new_frame,
                0,
                new_value.clamp(0.0, 100.0),
            )
        } else if graph_prop.starts_with("3D Position") {
            move_and_set_channel(
                &mut layer.transform_3d.position,
                state.current_frame,
                new_frame,
                axis,
                new_value,
            )
        } else if graph_prop.starts_with("3D Rotation") {
            move_and_set_channel(
                &mut layer.transform_3d.rotation,
                state.current_frame,
                new_frame,
                axis,
                new_value,
            )
        } else if graph_prop.starts_with("3D Scale") {
            move_and_set_channel(
                &mut layer.transform_3d.scale,
                state.current_frame,
                new_frame,
                axis,
                new_value,
            )
        } else if graph_prop.starts_with("Position") {
            move_and_set_channel(
                &mut layer.transform.position,
                state.current_frame,
                new_frame,
                axis,
                new_value,
            )
        } else if graph_prop.starts_with("Scale") {
            move_and_set_channel(
                &mut layer.transform.scale,
                state.current_frame,
                new_frame,
                axis,
                new_value,
            )
        } else if graph_prop.starts_with("PinX:") || graph_prop.starts_with("PinY:") {
            pin_anim_mut(layer, graph_prop)
                .map(|pin| {
                    move_and_set_channel(
                        pin,
                        state.current_frame,
                        new_frame,
                        axis,
                        new_value,
                    )
                })
                .unwrap_or(false)
        } else {
            false
        };
        if changed {
            *project_changed = true;
        }
        ui.ctx().data_mut(|d| {
            d.insert_temp(
                drag_state_id,
                GraphKeyframeDrag {
                    current_frame: new_frame,
                    current_value: new_value,
                    ..state
                },
            )
        });
    }
    if anchor_resp.drag_stopped() {
        ui.ctx()
            .data_mut(|d| d.remove::<GraphKeyframeDrag>(drag_state_id));
    }
    let anchor_color = if anchor_resp.dragged() {
        colors::HANDLE_NORMAL
    } else if anchor_resp.hovered() {
        colors::HANDLE_HOVER_FILL
    } else {
        colors::TIMELINE_KEYFRAME
    };
    ui.painter().circle_filled(pt, 4.0, anchor_color);

    // ── Double-click anchor → numeric value popup ──
    let dbl_id =
        ui.make_persistent_id(("graph_kf_popup", &layer.id, graph_prop, anchor_token));
    let mut show_popup: bool = ui
        .ctx()
        .data_mut(|d| *d.get_temp_mut_or_insert_with(dbl_id, || false));
    if anchor_resp.double_clicked() {
        show_popup = true;
        let frame_edit_id = egui::Id::new((
            "graph_kf_frame_text",
            &layer.id,
            graph_prop,
            anchor_token,
        ));
        let value_edit_id = egui::Id::new((
            "graph_kf_value_text",
            &layer.id,
            graph_prop,
            anchor_token,
        ));
        ui.ctx().data_mut(|data| {
            data.insert_temp(frame_edit_id, kf_frame.to_string());
            data.insert_temp(value_edit_id, format!("{:.2}", kf_val));
        });
    }
    if show_popup {
        let popup_id =
            egui::Id::new(("graph_kf_val_popup", &layer.id, graph_prop, anchor_token));
        let resp = egui::Area::new(popup_id)
            .fixed_pos(pt + egui::vec2(12.0, -20.0))
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                ui.group(|ui| {
                    ui.set_min_width(140.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Frame:").small());
                        let frame_edit_id = egui::Id::new((
                            "graph_kf_frame_text",
                            &layer.id,
                            graph_prop,
                            anchor_token,
                        ));
                        let mut frame_str = ui.ctx().data(|data| {
                            data.get_temp::<String>(frame_edit_id)
                                .unwrap_or_else(|| kf_frame.to_string())
                        });
                        let frame_response = ui.add(
                            egui::TextEdit::singleline(&mut frame_str)
                                .desired_width(50.0),
                        );
                        if frame_response.changed() {
                            ui.ctx().data_mut(|data| {
                                data.insert_temp(frame_edit_id, frame_str.clone())
                            });
                        }
                        if frame_response.lost_focus()
                            || ui.input(|input| input.key_pressed(egui::Key::Enter))
                        {
                            if let Ok(f) = frame_str.parse::<u32>() {
                                let new_f = f.min(total_f);
                                let handled_effect_move =
                                    if is_effect_property(graph_prop)
                                        && new_f != kf_frame
                                    {
                                        if graph_prop.contains('|') {
                                            move_effect_channel_keyframe(
                                                layer,
                                                graph_prop,
                                                kf_frame,
                                                new_f,
                                            )
                                        } else {
                                            move_effect_scalar_keyframe(
                                                layer,
                                                graph_prop,
                                                kf_frame,
                                                new_f,
                                            )
                                        }
                                    } else if graph_prop == "Rotation"
                                        && new_f != kf_frame
                                    {
                                        layer
                                            .transform
                                            .rotation
                                            .move_keyframe(kf_frame, new_f)
                                    } else if graph_prop == "Opacity"
                                        && new_f != kf_frame
                                    {
                                        layer
                                            .transform
                                            .opacity
                                            .move_keyframe(kf_frame, new_f)
                                    } else if graph_prop == "Position X"
                                        || graph_prop == "Position Y"
                                    {
                                        layer
                                            .transform
                                            .position
                                            .move_keyframe(kf_frame, new_f)
                                    } else if graph_prop == "Scale X"
                                        || graph_prop == "Scale Y"
                                    {
                                        layer
                                            .transform
                                            .scale
                                            .move_keyframe(kf_frame, new_f)
                                    } else if graph_prop.starts_with("3D Position") {
                                        layer
                                            .transform_3d
                                            .position
                                            .move_keyframe(kf_frame, new_f)
                                    } else if graph_prop.starts_with("3D Rotation") {
                                        layer
                                            .transform_3d
                                            .rotation
                                            .move_keyframe(kf_frame, new_f)
                                    } else if graph_prop.starts_with("3D Scale") {
                                        layer
                                            .transform_3d
                                            .scale
                                            .move_keyframe(kf_frame, new_f)
                                    } else if graph_prop.starts_with("PinX:")
                                        || graph_prop.starts_with("PinY:")
                                    {
                                        pin_anim_mut(layer, graph_prop).is_some_and(
                                            |pin| pin.move_keyframe(kf_frame, new_f),
                                        )
                                    } else {
                                        false
                                    };
                                if handled_effect_move {
                                    *project_changed = true;
                                }
                                ui.ctx().data_mut(|data| {
                                    data.insert_temp(frame_edit_id, new_f.to_string())
                                });
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Value:").small());
                        let value_edit_id = egui::Id::new((
                            "graph_kf_value_text",
                            &layer.id,
                            graph_prop,
                            anchor_token,
                        ));
                        let mut val_str = ui.ctx().data(|data| {
                            data.get_temp::<String>(value_edit_id)
                                .unwrap_or_else(|| format!("{:.2}", kf_val))
                        });
                        let value_response = ui.add(
                            egui::TextEdit::singleline(&mut val_str)
                                .desired_width(70.0),
                        );
                        if value_response.changed() {
                            ui.ctx().data_mut(|data| {
                                data.insert_temp(value_edit_id, val_str.clone())
                            });
                        }
                        if value_response.lost_focus()
                            || ui.input(|input| input.key_pressed(egui::Key::Enter))
                        {
                            if let Ok(v) = val_str.parse::<f32>() {
                                if graph_prop.starts_with("Position") {
                                    let ci =
                                        if graph_prop.ends_with('Y') { 1 } else { 0 };
                                    if let Some(kfs) =
                                        layer.transform.position.keyframes_mut()
                                    {
                                        if let Some(kf) = kfs
                                            .iter_mut()
                                            .find(|kf| kf.frame == kf_frame)
                                        {
                                            kf.value[ci] = v;
                                            *project_changed = true;
                                        }
                                    }
                                } else if graph_prop.starts_with("Scale") {
                                    let ci =
                                        if graph_prop.ends_with('Y') { 1 } else { 0 };
                                    if let Some(kfs) =
                                        layer.transform.scale.keyframes_mut()
                                    {
                                        if let Some(kf) = kfs
                                            .iter_mut()
                                            .find(|kf| kf.frame == kf_frame)
                                        {
                                            kf.value[ci] = v;
                                            *project_changed = true;
                                        }
                                    }
                                } else if graph_prop == "Rotation" {
                                    if let Some(kfs) =
                                        layer.transform.rotation.keyframes_mut()
                                    {
                                        if let Some(kf) = kfs
                                            .iter_mut()
                                            .find(|kf| kf.frame == kf_frame)
                                        {
                                            kf.value = v;
                                            *project_changed = true;
                                        }
                                    }
                                } else if graph_prop == "Opacity" {
                                    if let Some(kfs) =
                                        layer.transform.opacity.keyframes_mut()
                                    {
                                        if let Some(kf) = kfs
                                            .iter_mut()
                                            .find(|kf| kf.frame == kf_frame)
                                        {
                                            kf.value = v.clamp(0.0, 100.0);
                                            *project_changed = true;
                                        }
                                    }
                                } else if graph_prop.starts_with("3D Position") {
                                    if let Some(kfs) =
                                        layer.transform_3d.position.keyframes_mut()
                                    {
                                        if let Some(kf) = kfs
                                            .iter_mut()
                                            .find(|kf| kf.frame == kf_frame)
                                        {
                                            kf.value[axis_3d(graph_prop)] = v;
                                            *project_changed = true;
                                        }
                                    }
                                } else if graph_prop.starts_with("3D Rotation") {
                                    if let Some(kfs) =
                                        layer.transform_3d.rotation.keyframes_mut()
                                    {
                                        if let Some(kf) = kfs
                                            .iter_mut()
                                            .find(|kf| kf.frame == kf_frame)
                                        {
                                            kf.value[axis_3d(graph_prop)] = v;
                                            *project_changed = true;
                                        }
                                    }
                                } else if graph_prop.starts_with("3D Scale") {
                                    if let Some(kfs) =
                                        layer.transform_3d.scale.keyframes_mut()
                                    {
                                        if let Some(kf) = kfs
                                            .iter_mut()
                                            .find(|kf| kf.frame == kf_frame)
                                        {
                                            kf.value[axis_3d(graph_prop)] = v;
                                            *project_changed = true;
                                        }
                                    }
                                } else if is_effect_property(graph_prop)
                                    && set_effect_channel_at_frame(
                                        layer,
                                        graph_prop,
                                        kf_frame,
                                        v,
                                    )
                                {
                                    *project_changed = true;
                                }
                            }
                        }
                    });
                    if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
                        show_popup = false;
                    }
                    if ui.button("Done").clicked() {
                        show_popup = false;
                    }
                });
            });
        if ui.input(|i| i.pointer.any_click()) && !resp.response.contains_pointer() {
            show_popup = false;
        }
    }
    ui.ctx().data_mut(|d| d.insert_temp(dbl_id, show_popup));
    if anchor_resp.hovered() {
        ui.painter().circle_stroke(
            pt,
            7.0,
            egui::Stroke::new(1.0_f32, colors::TIMELINE_KEYFRAME),
        );
    }
    Some(anchor_token)
}
