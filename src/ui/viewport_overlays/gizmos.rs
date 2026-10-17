//! Transform handles, 3D axis gizmos, and vector mask vertex overlays.

use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub fn draw_transform_gizmos(
    ui: &mut egui::Ui,
    app: &KagariApp,
    ctx: &egui::Context,
    origin_x: f32,
    origin_y: f32,
    draw_w: f32,
    draw_h: f32,
    comp_w: f32,
    comp_h: f32,
    current_frame: u32,
) {
    if !app.show_handles {
        return;
    }
    let Some(s_idx) = app.selection.selected_layer_idx else {
        return;
    };
    let comp = app.history.current().active_composition();
    if s_idx >= comp.layers.len() {
        return;
    }
    let s_layer = &comp.layers[s_idx];
    let (pos, _scale, _rot, _op) = comp.resolve_world_transform(s_layer, current_frame);
    let rx = origin_x + (pos[0] / comp_w) * draw_w;
    let ry = origin_y + (pos[1] / comp_h) * draw_h;
    let center = egui::pos2(rx, ry);

    // 3D Axis Gizmo (only for 3D layers)
    if s_layer.is_3d {
        // X-Axis (Red)
        let x_end = egui::pos2(rx + 65.0, ry);
        ui.painter()
            .line_segment([center, x_end], egui::Stroke::new(2.5_f32, colors::GIZMO_X));
        ui.painter().text(
            egui::pos2(x_end.x + 8.0, x_end.y),
            egui::Align2::LEFT_CENTER,
            "X",
            egui::FontId::proportional(12.0),
            colors::GIZMO_X,
        );

        // Y-Axis (Green)
        let y_end = egui::pos2(rx, ry + 65.0);
        ui.painter()
            .line_segment([center, y_end], egui::Stroke::new(2.5_f32, colors::GIZMO_Y));
        ui.painter().text(
            egui::pos2(y_end.x, y_end.y + 8.0),
            egui::Align2::CENTER_TOP,
            "Y",
            egui::FontId::proportional(12.0),
            colors::GIZMO_Y,
        );

        // Z-Axis (Blue Diagonal)
        let z_end = egui::pos2(rx - 45.0, ry + 45.0);
        ui.painter()
            .line_segment([center, z_end], egui::Stroke::new(2.5_f32, colors::GIZMO_Z));
        ui.painter().text(
            egui::pos2(z_end.x - 8.0, z_end.y + 4.0),
            egui::Align2::RIGHT_TOP,
            "Z",
            egui::FontId::proportional(12.0),
            colors::GIZMO_Z,
        );
    }

    // 8-Point Bounding Box Transform Handles
    let scale = s_layer.transform.scale.evaluate(current_frame);
    let hw = (scale[0].abs() / 100.0 * 100.0 * 0.5) * (draw_w / comp_w);
    let hh = (scale[1].abs() / 100.0 * 100.0 * 0.5) * (draw_h / comp_h);
    let bbox_corners = [
        egui::pos2(rx - hw, ry - hh), // Top-Left
        egui::pos2(rx, ry - hh),      // Top-Center
        egui::pos2(rx + hw, ry - hh), // Top-Right
        egui::pos2(rx + hw, ry),      // Mid-Right
        egui::pos2(rx + hw, ry + hh), // Bottom-Right
        egui::pos2(rx, ry + hh),      // Bottom-Center
        egui::pos2(rx - hw, ry + hh), // Bottom-Left
        egui::pos2(rx - hw, ry),      // Mid-Left
    ];

    let bbox_rect = egui::Rect::from_center_size(center, egui::vec2(hw * 2.0, hh * 2.0));
    ui.painter().rect_stroke(
        bbox_rect,
        0.0,
        egui::Stroke::new(1.5_f32, colors::BBOX_STROKE),
    );

    let hover_pos = ctx.input(|i| i.pointer.hover_pos());

    for hp in bbox_corners {
        let dist = hover_pos.map_or(999.0, |p| p.distance(hp));
        let is_hovered = dist <= 14.0;

        let h_size = if is_hovered { 12.0 } else { 7.0 };
        let fill_c = if is_hovered {
            colors::HANDLE_HOVER_FILL
        } else {
            colors::HANDLE_NORMAL
        };
        let stroke_c = if is_hovered {
            colors::HANDLE_HOVER_STROKE
        } else {
            colors::BORDER_ACTIVE
        };

        let h_rect = egui::Rect::from_center_size(hp, egui::vec2(h_size, h_size));
        ui.painter().rect_filled(h_rect, 1.0, fill_c);
        ui.painter().rect_stroke(
            h_rect,
            1.0,
            egui::Stroke::new(if is_hovered { 2.0_f32 } else { 1.0_f32 }, stroke_c),
        );
    }

    // Center Target Circle with Magnetic Hover
    let center_dist = hover_pos.map_or(999.0, |p| p.distance(center));
    let is_center_hovered = center_dist <= 14.0;
    let c_radius = if is_center_hovered { 8.0 } else { 5.0 };

    ui.painter()
        .circle_filled(center, c_radius, colors::CENTER_DOT);
    ui.painter().circle_stroke(
        center,
        c_radius,
        egui::Stroke::new(1.5_f32, egui::Color32::BLACK),
    );
    if is_center_hovered {
        ui.painter().circle_stroke(
            center,
            30.0,
            egui::Stroke::new(1.5_f32, colors::CENTER_HOVER_RING),
        );
    }

    // Vector Mask Bezier Tangents & Path Overlay
    for mask in &s_layer.masks {
        if mask.enabled {
            let verts = mask.path.vertices_at_frame(current_frame);
            for v in &verts {
                let vx = origin_x + (v[0] / comp_w) * draw_w;
                let vy = origin_y + (v[1] / comp_h) * draw_h;
                let v_pos = egui::pos2(vx, vy);

                ui.painter().rect_filled(
                    egui::Rect::from_center_size(v_pos, egui::vec2(7.0, 7.0)),
                    1.0,
                    colors::KEYFRAME_DOT,
                );
                ui.painter().rect_stroke(
                    egui::Rect::from_center_size(v_pos, egui::vec2(7.0, 7.0)),
                    1.0,
                    egui::Stroke::new(1.0_f32, egui::Color32::BLACK),
                );
            }
        }
    }

    // Dragging Floating HUD Badge next to cursor
    if app.viewport_drag_state.is_some() || app.viewport_mask_drag_state.is_some() {
        if let Some(ptr) = hover_pos {
            let hud_pos = egui::pos2(ptr.x + 16.0, ptr.y + 16.0);
            let hud_rect = egui::Rect::from_min_size(hud_pos, egui::vec2(130.0, 22.0));
            ui.painter().rect_filled(hud_rect, 4.0, colors::HUD_BG);
            ui.painter().rect_stroke(
                hud_rect,
                4.0,
                egui::Stroke::new(1.0_f32, colors::HUD_STROKE),
            );
            ui.painter().text(
                hud_rect.center(),
                egui::Align2::CENTER_CENTER,
                format!("Scale: ({:.0}%, {:.0}%)", scale[0], scale[1]),
                egui::FontId::monospace(10.0),
                colors::HUD_TEXT,
            );
        }
    }


}
