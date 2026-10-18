use eframe::egui;
use crate::core::keyframe::{BezierControlPoint, InterpolationType};
use crate::core::timeline::Layer;
use crate::ui::graph_editor::channel_ops::clamp_bezier_x;
use crate::ui::graph_editor::keyframe_access::bezier_control_points;
use crate::ui::theme::colors;

pub fn handle_tangent_handles(
    ui: &mut egui::Ui,
    layer: &mut Layer,
    graph_prop: &str,
    anchor_token: usize,
    display_frame: u32,
    pt: egui::Pos2,
    linked_tangent: &mut bool,
    project_changed: &mut bool,
) {
    // Extract current bezier points (default Easy Ease if linear/hold)
    let (bx1, by1, bx2, by2): (f32, f32, f32, f32) = {
        let prop = graph_prop.to_string();
        if prop.starts_with("3D ") {
            crate::ui::graph_editor::keyframe_access::layer_keyframes_of_vec3(layer, &prop)
                .and_then(|kfs| {
                    kfs.iter()
                        .find(|key| key.frame == display_frame)
                        .map(bezier_control_points)
                })
                .unwrap_or((0.33, 0.0, 0.67, 1.0))
        } else if matches!(
            prop.as_str(),
            "Position X" | "Position Y" | "Scale X" | "Scale Y"
        ) {
            crate::ui::graph_editor::keyframe_access::layer_keyframes_of_vec2(layer, &prop)
                .and_then(|kfs| {
                    kfs.iter()
                        .find(|key| key.frame == display_frame)
                        .map(bezier_control_points)
                })
                .unwrap_or((0.33, 0.0, 0.67, 1.0))
        } else if crate::ui::graph_editor::channel_ops::is_effect_property(&prop) {
            crate::ui::graph_editor::keyframe_access::effect_keyframes_of_f32(layer, &prop)
                .and_then(|kfs| {
                    kfs.iter()
                        .find(|key| key.frame == display_frame)
                        .map(bezier_control_points)
                })
                .unwrap_or((0.33, 0.0, 0.67, 1.0))
        } else {
            crate::ui::graph_editor::keyframe_access::layer_keyframes_of_f32(layer, &prop)
                .and_then(|kfs| {
                    kfs.iter()
                        .find(|key| key.frame == display_frame)
                        .map(bezier_control_points)
                })
                .unwrap_or((0.33, 0.0, 0.67, 1.0))
        }
    };

    let h_out = egui::pos2(pt.x + bx2 * 44.0, pt.y - by2 * 24.0);
    let h_in = egui::pos2(pt.x - bx1 * 44.0, pt.y + by1 * 24.0);

    let h_out_rect = egui::Rect::from_center_size(h_out, egui::vec2(14.0, 14.0));
    let h_in_rect = egui::Rect::from_center_size(h_in, egui::vec2(14.0, 14.0));
    let h_out_id = egui::Id::new(("graph_h_out", &layer.id, graph_prop, anchor_token));
    let h_in_id = egui::Id::new(("graph_h_in", &layer.id, graph_prop, anchor_token));
    let h_out_drag_id = egui::Id::new((
        "graph_bezier_drag_out",
        &layer.id,
        graph_prop,
        anchor_token,
    ));
    let h_in_drag_id = egui::Id::new((
        "graph_bezier_drag_in",
        &layer.id,
        graph_prop,
        anchor_token,
    ));
    let h_out_resp = ui.interact(h_out_rect, h_out_id, egui::Sense::drag());
    let h_in_resp = ui.interact(h_in_rect, h_in_id, egui::Sense::drag());

    let mut new_pts: Option<[f32; 4]> = None;
    if h_out_resp.drag_started() {
        ui.ctx()
            .data_mut(|data| data.insert_temp(h_out_drag_id, [bx1, by1, bx2, by2]));
    }
    if h_in_resp.drag_started() {
        ui.ctx()
            .data_mut(|data| data.insert_temp(h_in_drag_id, [bx1, by1, bx2, by2]));
    }
    let out_base = ui
        .ctx()
        .data(|data| data.get_temp::<[f32; 4]>(h_out_drag_id))
        .unwrap_or([bx1, by1, bx2, by2]);
    let in_base = ui
        .ctx()
        .data(|data| data.get_temp::<[f32; 4]>(h_in_drag_id))
        .unwrap_or([bx1, by1, bx2, by2]);
    if h_out_resp.dragged() {
        let d = h_out_resp.drag_delta();
        let nx2 = clamp_bezier_x(out_base[2] + d.x / 44.0, out_base[0] + 0.01, 1.0);
        let ny2 = (out_base[3] - d.y / 24.0).clamp(-1.5, 2.5);
        if *linked_tangent {
            let mir_x = clamp_bezier_x(1.0 - nx2, 0.0, nx2 - 0.01);
            new_pts = Some([mir_x, -ny2, nx2, ny2]);
        } else {
            new_pts = Some([out_base[0], out_base[1], nx2, ny2]);
        }
    }
    if h_in_resp.dragged() {
        let d = h_in_resp.drag_delta();
        let nx1 = clamp_bezier_x(in_base[0] - d.x / 44.0, 0.0, in_base[2] - 0.01);
        let ny1 = (in_base[1] + d.y / 24.0).clamp(-1.5, 2.5);
        if *linked_tangent {
            let mir_x = clamp_bezier_x(1.0 - nx1, nx1 + 0.01, 1.0);
            new_pts = Some([nx1, ny1, mir_x, -ny1]);
        } else {
            new_pts = Some([nx1, ny1, in_base[2], in_base[3]]);
        }
    }
    if let Some(pts) = new_pts {
        let prop = graph_prop.to_string();
        let interp = InterpolationType::Bezier {
            outgoing: BezierControlPoint {
                influence: 0.333,
                speed: 0.0,
            },
            incoming: BezierControlPoint {
                influence: 0.333,
                speed: 0.0,
            },
            custom_bezier: Some(pts),
        };
        let mut updated = false;
        if prop.starts_with("3D ") {
            if let Some(kfs) = crate::ui::graph_editor::keyframe_access::layer_keyframes_of_vec3(layer, &prop) {
                if let Some(kf) = kfs.iter_mut().find(|key| key.frame == display_frame) {
                    kf.interpolation = interp;
                    updated = true;
                }
            }
        } else if matches!(
            prop.as_str(),
            "Position X" | "Position Y" | "Scale X" | "Scale Y"
        ) {
            if let Some(kfs) = crate::ui::graph_editor::keyframe_access::layer_keyframes_of_vec2(layer, &prop) {
                if let Some(kf) = kfs.iter_mut().find(|key| key.frame == display_frame) {
                    kf.interpolation = interp;
                    updated = true;
                }
            }
        } else if crate::ui::graph_editor::channel_ops::is_effect_property(&prop) {
            if let Some(kfs) = crate::ui::graph_editor::keyframe_access::effect_keyframes_of_f32(layer, &prop) {
                if let Some(kf) = kfs.iter_mut().find(|key| key.frame == display_frame) {
                    kf.interpolation = interp;
                    updated = true;
                }
            }
        } else if let Some(kfs) = crate::ui::graph_editor::keyframe_access::layer_keyframes_of_f32(layer, &prop) {
            if let Some(kf) = kfs.iter_mut().find(|key| key.frame == display_frame) {
                kf.interpolation = interp;
                updated = true;
            }
        }
        if updated {
            *project_changed = true;
        }
    }
    if h_out_resp.drag_stopped() {
        ui.ctx()
            .data_mut(|data| data.remove::<[f32; 4]>(h_out_drag_id));
    }
    if h_in_resp.drag_stopped() {
        ui.ctx()
            .data_mut(|data| data.remove::<[f32; 4]>(h_in_drag_id));
    }

    let any_hover = h_out_resp.hovered()
        || h_in_resp.dragged()
        || h_in_resp.hovered()
        || h_out_resp.dragged();
    let stroke_color = if any_hover {
        colors::ACCENT_ORANGE
    } else {
        colors::MOTION_PATH
    };
    ui.painter()
        .line_segment([pt, h_out], egui::Stroke::new(1.2_f32, stroke_color));
    ui.painter()
        .line_segment([pt, h_in], egui::Stroke::new(1.2_f32, stroke_color));

    let h_out_color = if h_out_resp.hovered() || h_out_resp.dragged() {
        colors::HANDLE_NORMAL
    } else {
        colors::MOTION_PATH
    };
    let h_in_color = if h_in_resp.hovered() || h_in_resp.dragged() {
        colors::HANDLE_NORMAL
    } else {
        colors::MOTION_PATH
    };
    ui.painter().circle_filled(h_out, 4.0, h_out_color);
    ui.painter().circle_filled(h_in, 4.0, h_in_color);
}
