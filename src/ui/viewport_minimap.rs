use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub const SHOW_NAVIGATOR_ID: &str = "viewport_show_navigator";

pub fn show_navigator(ctx: &egui::Context) -> bool {
    ctx.data(|data| {
        data.get_temp(egui::Id::new(SHOW_NAVIGATOR_ID))
            .unwrap_or(false)
    })
}

pub fn draw_minimap(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    current_frame: u32,
    comp_w: f32,
    comp_h: f32,
    viewport_rect: egui::Rect,
    draw_rect: egui::Rect,
) {
    if comp_w <= 0.0
        || comp_h <= 0.0
        || viewport_rect.width() <= 0.0
        || viewport_rect.height() <= 0.0
    {
        return;
    }

    let aspect = comp_w / comp_h;
    let max_map_width = (viewport_rect.width() - 24.0).clamp(48.0, 140.0);
    let max_map_height = (viewport_rect.height() - 24.0).clamp(48.0, 140.0);
    let map_w = max_map_width.min(max_map_height * aspect);
    let map_h = map_w / aspect.max(f32::EPSILON);
    let rect = egui::Rect::from_min_size(
        egui::pos2(
            viewport_rect.right() - map_w - 12.0,
            viewport_rect.top() + 12.0,
        ),
        egui::vec2(map_w, map_h),
    );
    let response = ui.interact(
        rect,
        ui.id().with("composition_navigator"),
        egui::Sense::click_and_drag(),
    );
    let drag_id = ui.id().with("composition_navigator_drag");
    if response.drag_started() {
        let start_pointer = ui
            .input(|input| input.pointer.press_origin())
            .or_else(|| response.interact_pointer_pos());
        if let Some(pointer) = start_pointer {
            ui.ctx().data_mut(|data| {
                data.insert_temp(drag_id, (pointer, app.playback.viewport_pan));
            });
        }
    }
    let painter = ui.painter().with_clip_rect(rect);
    painter.rect_filled(rect, 4.0, colors::HUD_BG);
    painter.rect_stroke(rect, 4.0, egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE));

    let map_point = |point: [f32; 2]| {
        egui::pos2(
            rect.left() + (point[0] / comp_w) * map_w,
            rect.top() + (point[1] / comp_h) * map_h,
        )
    };
    let comp = app.history.current().active_composition();
    for layer in &comp.layers {
        if !layer.is_active(current_frame) || !layer.visible {
            continue;
        }
        let center = map_point(layer.transform.position.evaluate(current_frame));
        let scale = layer.transform.scale.evaluate(current_frame);
        let layer_size = egui::vec2(
            (scale[0].abs() * 0.8 * map_w / comp_w).clamp(3.0, map_w),
            (scale[1].abs() * 0.8 * map_h / comp_h).clamp(3.0, map_h),
        );
        let rgb = layer.label.to_rgb();
        let color = egui::Color32::from_rgba_unmultiplied(
            (rgb[0] * 255.0) as u8,
            (rgb[1] * 255.0) as u8,
            (rgb[2] * 255.0) as u8,
            180,
        );
        painter.rect_filled(egui::Rect::from_center_size(center, layer_size), 1.0, color);
    }

    let transform = crate::ui::viewport_state::ViewportTransform::new(draw_rect, comp_w, comp_h);
    let visible_min = transform.screen_to_comp(viewport_rect.left_top());
    let visible_max = transform.screen_to_comp(viewport_rect.right_bottom());
    let left = visible_min[0].min(visible_max[0]).clamp(0.0, comp_w);
    let top = visible_min[1].min(visible_max[1]).clamp(0.0, comp_h);
    let right = visible_min[0].max(visible_max[0]).clamp(left, comp_w);
    let bottom = visible_min[1].max(visible_max[1]).clamp(top, comp_h);
    let view_rect = egui::Rect::from_min_max(map_point([left, top]), map_point([right, bottom]));
    painter.rect_stroke(
        view_rect,
        2.0,
        egui::Stroke::new(1.5_f32, colors::ACCENT_CYAN),
    );

    if app.ui_tabs.viewport_mag_ratio > 0.0 && response.dragged() {
        if let (Some(pointer), Some((start_pointer, start_pan))) = (
            response.interact_pointer_pos(),
            ui.ctx()
                .data(|data| data.get_temp::<(egui::Pos2, egui::Vec2)>(drag_id)),
        ) {
            let screen_per_map_x = draw_rect.width() / map_w.max(f32::EPSILON);
            let screen_per_map_y = draw_rect.height() / map_h.max(f32::EPSILON);
            let delta = pointer - start_pointer;
            app.playback.viewport_pan =
                start_pan - egui::vec2(delta.x * screen_per_map_x, delta.y * screen_per_map_y);
        }
    } else if app.ui_tabs.viewport_mag_ratio > 0.0 && response.clicked() {
        if let Some(pointer) = response.interact_pointer_pos() {
            let normalized_x = ((pointer.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
            let normalized_y = ((pointer.y - rect.top()) / rect.height()).clamp(0.0, 1.0);
            let target_comp = [normalized_x * comp_w, normalized_y * comp_h];
            let current_screen = transform.comp_to_screen(target_comp);
            app.playback.viewport_pan += viewport_rect.center() - current_screen;
        }
    }
    if response.drag_stopped() {
        ui.ctx()
            .data_mut(|data| data.remove::<(egui::Pos2, egui::Vec2)>(drag_id));
    }
    response.on_hover_text(if app.ui_tabs.viewport_mag_ratio > 0.0 {
        "ドラッグしてビューアーを移動"
    } else {
        "拡大表示するとドラッグで移動できます"
    });
}
