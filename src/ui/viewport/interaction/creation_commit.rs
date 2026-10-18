use eframe::egui;
use crate::app_state::KagariApp;
use crate::core::timeline::Layer;

pub fn handle_creation_tools_commit(
    app: &mut KagariApp,
    viewport_response: &egui::Response,
    pointer_pos: egui::Pos2,
    origin_x: f32,
    origin_y: f32,
    draw_w: f32,
    draw_h: f32,
    comp_w: f32,
    comp_h: f32,
) {
    if app.active_tool == crate::ui::toolbar::ActiveTool::Rectangle {
        let (cx, cy, sw, sh) = if let Some(start) = app.rect_drag_start.take() {
            let mid_x = (pointer_pos.x + start.x) / 2.0;
            let mid_y = (pointer_pos.y + start.y) / 2.0;
            let cx = ((mid_x - origin_x) / draw_w * comp_w).clamp(0.0, comp_w);
            let cy = ((mid_y - origin_y) / draw_h * comp_h).clamp(0.0, comp_h);
            let dx = ((pointer_pos.x - start.x) / draw_w * comp_w).abs().max(4.0);
            let dy = ((pointer_pos.y - start.y) / draw_h * comp_h).abs().max(4.0);
            (cx, cy, dx, dy)
        } else {
            let cx = ((pointer_pos.x - origin_x) / draw_w * comp_w).clamp(0.0, comp_w);
            let cy = ((pointer_pos.y - origin_y) / draw_h * comp_h).clamp(0.0, comp_h);
            (cx, cy, 220.0, 160.0)
        };
        let (n, dur) = {
            let c = app.history.current().active_composition();
            (c.layers.len(), c.duration_frames)
        };
        let mut shape_layer = Layer::new(
            format!("shape_{}", n),
            format!("Rectangle {}", n + 1),
            crate::core::timeline::LayerType::Shape {
                shape_type: crate::core::timeline::ShapeType::Rectangle {
                    width: crate::core::property::Animatable::new_constant(sw),
                    height: crate::core::property::Animatable::new_constant(sh),
                    corner_radius: crate::core::property::Animatable::new_constant(0.0),
                },
                color: [0.25, 0.55, 1.0, 1.0],
                stroke_color: [1.0, 1.0, 1.0, 1.0],
                stroke_width: 0.0,
                fill_type: Default::default(),
                extrusion_depth: 0.0,
                bevel_depth: 0.0,
            },
            dur,
        );
        shape_layer.transform.position = crate::core::property::Animatable::new_constant([cx, cy]);
        let comp_mut = app.history.current_mut().active_composition_mut();
        comp_mut.layers.push(shape_layer);
        let new_idx = comp_mut.layers.len() - 1;
        app.selection.selected_layer_idx = Some(new_idx);
        app.selection.selected_layers.clear();
        app.selection.selected_layers.insert(new_idx);
        crate::core::frame_cache::bump_version();
        app.toasts.info("Rectangle created (Q tool)");
    } else if app.active_tool == crate::ui::toolbar::ActiveTool::Text && viewport_response.clicked() {
        let cx = ((pointer_pos.x - origin_x) / draw_w * comp_w).clamp(0.0, comp_w);
        let cy = ((pointer_pos.y - origin_y) / draw_h * comp_h).clamp(0.0, comp_h);
        let (n, dur) = {
            let c = app.history.current().active_composition();
            (c.layers.len(), c.duration_frames)
        };
        let mut text_layer = Layer::new(
            format!("text_{}", n),
            format!("Text {}", n + 1),
            crate::core::timeline::LayerType::new_text("Text", 72, [1.0, 1.0, 1.0, 1.0]),
            dur,
        );
        text_layer.transform.position = crate::core::property::Animatable::new_constant([cx, cy]);
        let comp_mut = app.history.current_mut().active_composition_mut();
        comp_mut.layers.push(text_layer);
        let new_idx = comp_mut.layers.len() - 1;
        app.selection.selected_layer_idx = Some(new_idx);
        app.selection.selected_layers.clear();
        app.selection.selected_layers.insert(new_idx);
        app.inline_text_edit_layer = Some(new_idx);
        crate::core::frame_cache::bump_version();
    }
}
