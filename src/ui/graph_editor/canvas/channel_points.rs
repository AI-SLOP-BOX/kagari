use eframe::egui;
use crate::core::timeline::Layer;
use crate::ui::graph_editor::channel_ops::clamp_bezier_x;
use crate::ui::graph_editor::interpolation_ops::{
    effect_channel_bezier_points, move_effect_channel_keyframe, remove_effect_channel_at_frame,
    set_effect_channel_at_frame, set_effect_channel_bezier,
};
use crate::ui::graph_editor::GraphKeyframeDrag;
use crate::ui::theme::colors;

pub fn handle_channel_points_interaction(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    total_f: u32,
    val_range: f32,
    layer: &mut Layer,
    graph_prop: &str,
    channel_keys: &[(u32, f32)],
    frame_to_x: impl Fn(u32) -> f32,
    val_to_y: impl Fn(f32) -> f32,
    project_changed: &mut bool,
) {
    let channel_drag_id = egui::Id::new(("effect_channel_drag", &layer.id, graph_prop));
    let active_channel_drag: Option<GraphKeyframeDrag> =
        ui.ctx().data(|d| d.get_temp(channel_drag_id));
    for (frame, value) in channel_keys {
        let drag_display = active_channel_drag.filter(|drag| drag.current_frame == *frame);
        let display_frame = drag_display
            .map(|drag| drag.current_frame)
            .unwrap_or(*frame);
        let display_value = drag_display
            .map(|drag| drag.current_value)
            .unwrap_or(*value);
        let key_token = drag_display
            .map(|drag| drag.original_frame as usize)
            .unwrap_or(*frame as usize);
        let key_pos = egui::pos2(frame_to_x(display_frame), val_to_y(display_value));
        let key_rect = egui::Rect::from_center_size(key_pos, egui::vec2(14.0, 14.0));
        if !ui.is_rect_visible(key_rect) && drag_display.is_none() {
            continue;
        }
        let key_response = ui.interact(
            key_rect,
            egui::Id::new(("effect_channel_key", &layer.id, graph_prop, key_token)),
            egui::Sense::click_and_drag(),
        );
        ui.painter()
            .circle_filled(key_pos, 4.0, colors::TIMELINE_KEYFRAME);
        if let Some(points) = effect_channel_bezier_points(layer, graph_prop, display_frame) {
            let out = egui::pos2(key_pos.x + points[2] * 44.0, key_pos.y - points[3] * 24.0);
            let incoming = egui::pos2(key_pos.x - points[0] * 44.0, key_pos.y + points[1] * 24.0);
            let out_drag_id =
                egui::Id::new(("effect_bezier_drag_out", &layer.id, graph_prop, key_token));
            let in_drag_id =
                egui::Id::new(("effect_bezier_drag_in", &layer.id, graph_prop, key_token));
            let out_resp = ui.interact(
                egui::Rect::from_center_size(out, egui::vec2(14.0, 14.0)),
                egui::Id::new(("effect_bezier_out", &layer.id, graph_prop, key_token)),
                egui::Sense::drag(),
            );
            let in_resp = ui.interact(
                egui::Rect::from_center_size(incoming, egui::vec2(14.0, 14.0)),
                egui::Id::new(("effect_bezier_in", &layer.id, graph_prop, key_token)),
                egui::Sense::drag(),
            );
            ui.painter().line_segment(
                [key_pos, out],
                egui::Stroke::new(1.0_f32, colors::MOTION_PATH),
            );
            ui.painter().line_segment(
                [key_pos, incoming],
                egui::Stroke::new(1.0_f32, colors::MOTION_PATH),
            );
            ui.painter().circle_filled(out, 3.0, colors::HANDLE_NORMAL);
            ui.painter()
                .circle_filled(incoming, 3.0, colors::HANDLE_NORMAL);
            if out_resp.drag_started() {
                ui.ctx()
                    .data_mut(|data| data.insert_temp(out_drag_id, points));
            }
            if in_resp.drag_started() {
                ui.ctx()
                    .data_mut(|data| data.insert_temp(in_drag_id, points));
            }
            let out_base = ui
                .ctx()
                .data(|data| data.get_temp::<[f32; 4]>(out_drag_id))
                .unwrap_or(points);
            let in_base = ui
                .ctx()
                .data(|data| data.get_temp::<[f32; 4]>(in_drag_id))
                .unwrap_or(points);
            let mut next = points;
            if out_resp.dragged() {
                next[2] = clamp_bezier_x(
                    out_base[2] + out_resp.drag_delta().x / 44.0,
                    out_base[0] + 0.01,
                    1.0,
                );
                next[3] = (out_base[3] - out_resp.drag_delta().y / 24.0).clamp(-1.5, 2.5);
            }
            if in_resp.dragged() {
                next[0] = clamp_bezier_x(
                    in_base[0] - in_resp.drag_delta().x / 44.0,
                    0.0,
                    in_base[2] - 0.01,
                );
                next[1] = (in_base[1] + in_resp.drag_delta().y / 24.0).clamp(-1.5, 2.5);
            }
            if (out_resp.dragged() || in_resp.dragged())
                && set_effect_channel_bezier(layer, graph_prop, display_frame, next)
            {
                *project_changed = true;
            }
            if out_resp.drag_stopped() {
                ui.ctx()
                    .data_mut(|data| data.remove::<[f32; 4]>(out_drag_id));
            }
            if in_resp.drag_stopped() {
                ui.ctx()
                    .data_mut(|data| data.remove::<[f32; 4]>(in_drag_id));
            }
        }
        if key_response.drag_started() {
            ui.ctx().data_mut(|d| {
                d.insert_temp(
                    channel_drag_id,
                    GraphKeyframeDrag {
                        original_frame: *frame,
                        current_frame: *frame,
                        original_value: *value,
                        current_value: *value,
                    },
                )
            });
        }
        if key_response.secondary_clicked() {
            if remove_effect_channel_at_frame(layer, graph_prop, display_frame) {
                *project_changed = true;
            }
            continue;
        }
        if key_response.dragged() {
            let state = ui
                .ctx()
                .data(|d| d.get_temp::<GraphKeyframeDrag>(channel_drag_id))
                .unwrap_or(GraphKeyframeDrag {
                    original_frame: *frame,
                    current_frame: *frame,
                    original_value: *value,
                    current_value: *value,
                });
            let next_frame = (state.original_frame as i32
                + (key_response.drag_delta().x / rect.width() * total_f as f32).round() as i32)
                .clamp(0, total_f as i32) as u32;
            let next_value =
                state.original_value - key_response.drag_delta().y / (rect.height() - 8.0) * val_range;
            let target_frame = if next_frame == state.current_frame
                || move_effect_channel_keyframe(
                    layer,
                    graph_prop,
                    state.current_frame,
                    next_frame,
                ) {
                next_frame
            } else {
                state.current_frame
            };
            let value_changed =
                set_effect_channel_at_frame(layer, graph_prop, target_frame, next_value);
            *project_changed |= target_frame != state.current_frame || value_changed;
            ui.ctx().data_mut(|d| {
                d.insert_temp(
                    channel_drag_id,
                    GraphKeyframeDrag {
                        current_frame: target_frame,
                        current_value: next_value,
                        ..state
                    },
                )
            });
        }
        if key_response.drag_stopped() {
            ui.ctx()
                .data_mut(|d| d.remove::<GraphKeyframeDrag>(channel_drag_id));
        }
    }
}
