use crate::core::timeline::{Composition, LayerType};
use crate::ui::flow_graph_model::FlowGraphModel;
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

#[allow(dead_code)]
pub fn draw_flowchart_view(app: &mut KagariApp, ui: &mut egui::Ui, comp: &Composition) {
    let graph = FlowGraphModel::build(comp, app.playback.current_frame);

    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Comp & Layer Flowchart View (Shift+F3)")
                    .strong()
                    .color(colors::ACCENT_CYAN),
            );
            ui.weak("— Visual Graph of Compositions, Layer Hierarchies & Mattes");
        });
        ui.separator();

        if graph.cycle_detected {
            ui.colored_label(
                egui::Color32::from_rgb(230, 90, 90),
                "⚠ Dependency cycle detected between layers — evaluation order may be incorrect",
            );
            ui.separator();
        }

        let avail_size = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(avail_size.x, avail_size.y.max(220.0)),
            egui::Sense::click_and_drag(),
        );

        let pan_id = egui::Id::new("flowchart_canvas_pan");
        let zoom_id = egui::Id::new("flowchart_canvas_zoom");

        let mut pan = ui
            .ctx()
            .data(|d| d.get_temp::<egui::Vec2>(pan_id).unwrap_or(egui::Vec2::ZERO));
        let mut zoom = ui.ctx().data(|d| d.get_temp::<f32>(zoom_id).unwrap_or(1.0));

        if response.hovered() {
            let scroll = ui.input(|i| i.raw_scroll_delta.y);
            if scroll != 0.0 {
                let factor = if scroll > 0.0 { 1.1 } else { 1.0 / 1.1 };
                zoom = (zoom * factor).clamp(0.2, 3.0);
                ui.ctx().data_mut(|d| d.insert_temp(zoom_id, zoom));
            }
        }

        if response.dragged() {
            pan += response.drag_delta();
            ui.ctx().data_mut(|d| d.insert_temp(pan_id, pan));
            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Grabbing);
        } else if response.hovered() {
            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Grab);
        }

        let painter = ui.painter().with_clip_rect(rect);
        painter.rect_filled(rect, 4.0, colors::BG_DEEPEST);
        painter.rect_stroke(rect, 4.0, egui::Stroke::new(1.0_f32, colors::BORDER_MEDIUM));

        // Grid Background with pan offset
        let grid_color = colors::GRID_LINE;
        let step = 30.0 * zoom;
        let mut x = rect.left() + (pan.x % step);
        while x < rect.right() {
            painter.line_segment(
                [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                egui::Stroke::new(1.0_f32, grid_color),
            );
            x += step;
        }
        let mut y = rect.top() + (pan.y % step);
        while y < rect.bottom() {
            painter.line_segment(
                [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                egui::Stroke::new(1.0_f32, grid_color),
            );
            y += step;
        }

        // Draw Composition Root Node
        let root_pos = egui::pos2(rect.left() + 90.0 * zoom + pan.x, rect.center().y + pan.y);
        let root_rect =
            egui::Rect::from_center_size(root_pos, egui::vec2(130.0 * zoom, 44.0 * zoom));
        painter.rect_filled(root_rect, 6.0, colors::BG_ACTIVE);
        painter.rect_stroke(
            root_rect,
            6.0,
            egui::Stroke::new(1.5_f32, egui::Color32::WHITE),
        );
        painter.text(
            root_pos,
            egui::Align2::CENTER_CENTER,
            format!("[COMP]\n{}", comp.name),
            egui::FontId::proportional(12.0),
            egui::Color32::WHITE,
        );

        // Draw Layer Nodes
        let layer_count = comp.layers.len();
        if layer_count == 0 {
            painter.text(
                egui::pos2(rect.center().x + 80.0, rect.center().y),
                egui::Align2::CENTER_CENTER,
                "No layers in composition",
                egui::FontId::proportional(14.0),
                colors::TEXT_MUTED,
            );
            return;
        }

        let start_x = rect.left() + 280.0 * zoom + pan.x;
        let spacing_y = ((rect.height() - 40.0) / (layer_count as f32 + 1.0).max(1.0)) * zoom;
        let node_positions = graph.positions(
            egui::pos2(start_x, rect.top() + (20.0 + spacing_y) + pan.y),
            30.0 * zoom,
            spacing_y,
        );

        for (i, layer) in comp.layers.iter().enumerate() {
            let npos = node_positions[i];
            let ny = npos.y;

            let is_selected = app.selection.selected_layers.contains(&i)
                || app.selection.selected_layer_idx == Some(i);
            let node_rect =
                egui::Rect::from_center_size(npos, egui::vec2(140.0 * zoom, 36.0 * zoom));

            let base_color = match layer.layer_type {
                LayerType::Null => egui::Color32::from_rgb(140, 80, 80),
                LayerType::Audio { .. } => egui::Color32::from_rgb(30, 120, 90),
                LayerType::Text { .. } => egui::Color32::from_rgb(160, 110, 40),
                LayerType::Shape { .. } => egui::Color32::from_rgb(130, 70, 150),
                _ => egui::Color32::from_rgb(50, 70, 100),
            };

            let stroke_color = if is_selected {
                colors::ACCENT_CYAN
            } else {
                colors::BORDER_STRONG
            };

            painter.rect_filled(node_rect, 4.0, base_color);
            painter.rect_stroke(
                node_rect,
                4.0,
                egui::Stroke::new(if is_selected { 2.5_f32 } else { 1.0_f32 }, stroke_color),
            );

            let tag = match layer.layer_type {
                LayerType::Text { .. } => "[TXT]",
                LayerType::Shape { .. } => "[SHP]",
                LayerType::Audio { .. } => "[AUD]",
                LayerType::Null => "[NUL]",
                _ => "[VID]",
            };

            painter.text(
                npos,
                egui::Align2::CENTER_CENTER,
                format!("{} {}\n#{}", tag, layer.name, i + 1),
                egui::FontId::proportional(11.0),
                egui::Color32::WHITE,
            );

            // Topological execution-order badge (from VfxGraphCompiler)
            if let Some(order) = graph.nodes[i].execution_order {
                painter.text(
                    egui::pos2(npos.x + 62.0, ny - 14.0),
                    egui::Align2::CENTER_CENTER,
                    format!("{}", order),
                    egui::FontId::proportional(9.0),
                    colors::ACCENT_CYAN,
                );
            }

            // Curve line from Root Comp to Layer
            let control1 = egui::pos2(root_pos.x + 80.0, root_pos.y);
            let control2 = egui::pos2(npos.x - 80.0, npos.y);
            let points = [root_pos, control1, control2, npos];
            let curve_pts: Vec<egui::Pos2> = (0..=20)
                .map(|t| {
                    let u = t as f32 / 20.0;
                    let u1 = 1.0 - u;
                    let x = u1 * u1 * u1 * points[0].x
                        + 3.0 * u1 * u1 * u * points[1].x
                        + 3.0 * u1 * u * u * points[2].x
                        + u * u * u * points[3].x;
                    let y = u1 * u1 * u1 * points[0].y
                        + 3.0 * u1 * u1 * u * points[1].y
                        + 3.0 * u1 * u * u * points[2].y
                        + u * u * u * points[3].y;
                    egui::pos2(x, y)
                })
                .collect();

            painter.add(egui::Shape::line(
                curve_pts,
                egui::Stroke::new(1.2_f32, colors::MOTION_PATH.linear_multiply(140.0 / 255.0)),
            ));

            // Draw Parent Connection Lines
            if let Some(p_idx) = graph.nodes[i].parent_index {
                if p_idx < node_positions.len() {
                    let parent_pos = node_positions[p_idx];
                    painter.line_segment(
                        [npos, parent_pos],
                        egui::Stroke::new(1.8_f32, colors::TIMELINE_KEYFRAME),
                    );
                }
            }
        }

        // Track-matte dependency arrows: matte layer → consumer layer.
        // Dashed cyan so they read differently from parenting lines.
        for i in 0..graph.nodes.len() {
            let Some(matte_idx) = graph.nodes[i].matte_index else {
                continue;
            };
            if let (Some(&matte_pos), Some(&consumer_pos)) =
                (node_positions.get(matte_idx), node_positions.get(i))
            {
                let stroke = egui::Stroke::new(1.6_f32, colors::ACCENT_CYAN);
                for seg in egui::Shape::dashed_line(&[matte_pos, consumer_pos], stroke, 6.0, 5.0) {
                    painter.add(seg);
                }
                painter.text(
                    egui::pos2(
                        (matte_pos.x + consumer_pos.x) * 0.5,
                        (matte_pos.y + consumer_pos.y) * 0.5 - 8.0,
                    ),
                    egui::Align2::CENTER_CENTER,
                    "matte",
                    egui::FontId::proportional(9.0),
                    colors::ACCENT_CYAN,
                );
            }
        }

        if response.clicked() {
            if let Some(ptr) = response.interact_pointer_pos() {
                for (i, npos) in node_positions.iter().enumerate() {
                    let nrect =
                        egui::Rect::from_center_size(*npos, egui::vec2(140.0 * zoom, 36.0 * zoom));
                    if nrect.contains(ptr) {
                        app.selection.selected_layers.clear();
                        app.selection.selected_layers.insert(i);
                        app.selection.selected_layer_idx = Some(i);
                        break;
                    }
                }
            }
        }
    });
}
