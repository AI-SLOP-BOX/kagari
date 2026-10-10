//! Snapshot A/B interactive comparison split wipe line overlay.

use crate::ui::theme::colors;
use eframe::egui;

pub fn draw_split_wipe(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    origin_x: f32,
    origin_y: f32,
    draw_w: f32,
    draw_h: f32,
) {
    if !crate::ui::viewport_state::is_comparing(ctx) {
        return;
    }
    let wipe_pos = crate::ui::viewport_state::wipe_pos(ctx);
    let wipe_x = origin_x + wipe_pos * draw_w;
    let handle_center = egui::pos2(wipe_x, origin_y + draw_h * 0.5);

    ui.painter().line_segment(
        [
            egui::pos2(wipe_x, origin_y),
            egui::pos2(wipe_x, origin_y + draw_h),
        ],
        egui::Stroke::new(2.5_f32, colors::ACCENT_CYAN),
    );
    ui.painter()
        .circle_filled(handle_center, 10.0, colors::ACCENT_CYAN);
    ui.painter().circle_stroke(
        handle_center,
        10.0,
        egui::Stroke::new(1.5_f32, egui::Color32::WHITE),
    );
    ui.painter().text(
        egui::pos2(wipe_x - 15.0, origin_y + 15.0),
        egui::Align2::RIGHT_TOP,
        "[Snap A]",
        egui::FontId::proportional(11.0),
        colors::ACCENT_CYAN,
    );
    ui.painter().text(
        egui::pos2(wipe_x + 15.0, origin_y + 15.0),
        egui::Align2::LEFT_TOP,
        "[Live Frame]",
        egui::FontId::proportional(11.0),
        colors::ACCENT_ORANGE,
    );

    let handle_rect =
        egui::Rect::from_center_size(handle_center, egui::vec2(24.0, draw_h.max(24.0)));
    let wipe_response = ui.interact(
        handle_rect,
        ui.id().with("viewport_wipe_drag"),
        egui::Sense::drag(),
    );
    if wipe_response.dragged() {
        if let Some(ptr) = wipe_response.interact_pointer_pos() {
            crate::ui::viewport_state::set_wipe_pos(
                ctx,
                ((ptr.x - origin_x) / draw_w).clamp(0.05, 0.95),
            );
        }
    }
}
