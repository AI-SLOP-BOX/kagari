use crate::core::timeline::{Composition, LayerType};
use crate::ui::flow_graph_model::FlowGraphModel;
use crate::ui::kui;
use crate::ui::theme::colors;
use eframe::egui;

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;

    fn shape_count(layer_count: usize) -> usize {
        let mut comp = Composition::new("comp".into(), "Graph".into(), 1920, 1080, 30, 300);
        for index in 0..layer_count {
            comp.layers.push(crate::core::timeline::Layer::new_null(
                format!("layer-{index}"),
                format!("Long layer name {index} in the dependency graph"),
                300,
            ));
        }
        let ctx = egui::Context::default();
        let mut selected = None;
        let mut selection = std::collections::HashSet::new();
        let mut open = true;
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(600.0, 360.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    draw_node_graph_panel(ui, &comp, &mut selected, &mut selection, &mut open);
                });
            },
        );
        assert!(open && selected.is_none() && selection.is_empty());
        output.shapes.len()
    }

    #[test]
    fn offscreen_nodes_do_not_grow_the_paint_list() {
        let short = shape_count(300);
        let long = shape_count(5000);
        assert!(long <= short + 10, "short={short}, long={long}");
    }
}

pub fn draw_node_graph_panel(
    ui: &mut egui::Ui,
    comp: &Composition,
    selected_layer_idx: &mut Option<usize>,
    selected_layers: &mut std::collections::HashSet<usize>,
    show_graph_editor: &mut bool,
) {
    ui.group(|ui: &mut egui::Ui| {
        ui.horizontal(|ui: &mut egui::Ui| {
            ui.label(
                egui::RichText::new("Hybrid Node Graph View")
                    .strong()
                    .color(colors::ACCENT_CYAN),
            );
            ui.weak("— Visual Pipeline & Layer Dependencies");
            ui.with_layout(
                egui::Layout::right_to_left(egui::Align::Center),
                |ui: &mut egui::Ui| {
                    if kui::button(ui, "Close Node View (Tab)", kui::ButtonKind::Default).clicked()
                    {
                        *show_graph_editor = false;
                    }
                },
            );
        });
        ui.separator();

        let layers_count = comp.layers.len();
        let graph = FlowGraphModel::build_structure(comp);

        if layers_count == 0 {
            ui.centered_and_justified(|ui| {
                ui.weak("No layers in active composition");
            });
            return;
        }

        // Calculate node positions in canvas space
        let node_w = 140.0;
        let node_h = 36.0;
        let col_spacing = 170.0;
        let row_spacing = 52.0;

        let total_h = (layers_count as f32 * row_spacing + 60.0).max(220.0);
        let max_depth = graph.nodes.iter().map(|node| node.depth).max().unwrap_or(0);

        let total_w = (max_depth as f32 * col_spacing + 300.0).max(ui.available_width());

        egui::ScrollArea::both()
            .max_height(260.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let (rect, _response) = ui.allocate_exact_size(
                    egui::vec2(total_w, total_h),
                    egui::Sense::click_and_drag(),
                );

                // Draw Dark Canvas Grid
                ui.painter().rect_filled(rect, 4.0, colors::BG_DEEPEST);
                ui.painter().rect_stroke(
                    rect,
                    4.0,
                    egui::Stroke::new(1.0_f32, colors::BORDER_MEDIUM),
                );

                let visible = rect.intersect(ui.clip_rect());
                let grid_size = 24.0;
                let mut gx =
                    rect.left() + ((visible.left() - rect.left()) / grid_size).ceil() * grid_size;
                while gx < visible.right() {
                    ui.painter().line_segment(
                        [
                            egui::pos2(gx, visible.top()),
                            egui::pos2(gx, visible.bottom()),
                        ],
                        egui::Stroke::new(0.5_f32, colors::GRID_LINE),
                    );
                    gx += grid_size;
                }
                let mut gy =
                    rect.top() + ((visible.top() - rect.top()) / grid_size).ceil() * grid_size;
                while gy < visible.bottom() {
                    ui.painter().line_segment(
                        [
                            egui::pos2(visible.left(), gy),
                            egui::pos2(visible.right(), gy),
                        ],
                        egui::Stroke::new(0.5_f32, colors::GRID_LINE),
                    );
                    gy += grid_size;
                }

                let node_positions = graph.positions(
                    egui::pos2(rect.left() + 30.0, rect.top() + 24.0),
                    col_spacing,
                    row_spacing,
                );

                // 1. Draw Connecting Wires (Parent-Child Blue & Track Matte Amber)
                for idx in 0..layers_count {
                    let child_pos = node_positions[idx];
                    let child_input = egui::pos2(child_pos.x, child_pos.y + node_h * 0.5);

                    // Parent Connection Wire (Blue Curve)
                    if let Some(parent_idx) = graph.nodes[idx].parent_index {
                        let parent_pos = node_positions[parent_idx];
                        let parent_output =
                            egui::pos2(parent_pos.x + node_w, parent_pos.y + node_h * 0.5);

                        let ctrl1 = egui::pos2(parent_output.x + 40.0, parent_output.y);
                        let ctrl2 = egui::pos2(child_input.x - 40.0, child_input.y);

                        if egui::Rect::from_points(&[parent_output, ctrl1, ctrl2, child_input])
                            .expand(2.0)
                            .intersects(visible)
                        {
                            let mut wire_pts = Vec::with_capacity(16);
                            for step in 0..=15 {
                                let t = step as f32 / 15.0;
                                let inv_t = 1.0 - t;
                                let wx = inv_t.powi(3) * parent_output.x
                                    + 3.0 * inv_t.powi(2) * t * ctrl1.x
                                    + 3.0 * inv_t * t.powi(2) * ctrl2.x
                                    + t.powi(3) * child_input.x;
                                let wy = inv_t.powi(3) * parent_output.y
                                    + 3.0 * inv_t.powi(2) * t * ctrl1.y
                                    + 3.0 * inv_t * t.powi(2) * ctrl2.y
                                    + t.powi(3) * child_input.y;
                                wire_pts.push(egui::pos2(wx, wy));
                            }

                            for win in wire_pts.windows(2) {
                                ui.painter().line_segment(
                                    [win[0], win[1]],
                                    egui::Stroke::new(2.0_f32, colors::ACCENT_BLUE),
                                );
                            }
                        }
                    }

                    // Track Matte Connection Wire (Amber/Orange Curve)
                    if let Some(matte_idx) = graph.nodes[idx].matte_index {
                        let matte_pos = node_positions[matte_idx];
                        let matte_output =
                            egui::pos2(matte_pos.x + node_w, matte_pos.y + node_h * 0.5);

                        let ctrl1 = egui::pos2(matte_output.x + 30.0, matte_output.y);
                        let ctrl2 = egui::pos2(child_input.x - 30.0, child_input.y);

                        if egui::Rect::from_points(&[matte_output, ctrl1, ctrl2, child_input])
                            .expand(2.0)
                            .intersects(visible)
                        {
                            let mut wire_pts = Vec::with_capacity(16);
                            for step in 0..=15 {
                                let t = step as f32 / 15.0;
                                let inv_t = 1.0 - t;
                                let wx = inv_t.powi(3) * matte_output.x
                                    + 3.0 * inv_t.powi(2) * t * ctrl1.x
                                    + 3.0 * inv_t * t.powi(2) * ctrl2.x
                                    + t.powi(3) * child_input.x;
                                let wy = inv_t.powi(3) * matte_output.y
                                    + 3.0 * inv_t.powi(2) * t * ctrl1.y
                                    + 3.0 * inv_t * t.powi(2) * ctrl2.y
                                    + t.powi(3) * child_input.y;
                                wire_pts.push(egui::pos2(wx, wy));
                            }

                            for win in wire_pts.windows(2) {
                                ui.painter().line_segment(
                                    [win[0], win[1]],
                                    egui::Stroke::new(
                                        1.5_f32,
                                        egui::Color32::from_rgb(245, 158, 11),
                                    ),
                                );
                            }
                        }
                    }
                }

                // 2. Render Layer Card Nodes
                for (idx, layer) in comp.layers.iter().enumerate() {
                    let pos = node_positions[idx];
                    let is_selected =
                        *selected_layer_idx == Some(idx) || selected_layers.contains(&idx);

                    let node_rect = egui::Rect::from_min_size(pos, egui::vec2(node_w, node_h));

                    if !ui.is_rect_visible(node_rect) {
                        continue;
                    }
                    let node_response = ui.interact(
                        node_rect,
                        ui.id().with(("node_click", idx)),
                        egui::Sense::click(),
                    );
                    let bg_c = match (is_selected, node_response.hovered()) {
                        (true, true) => colors::BG_ACTIVE,
                        (true, false) => colors::BG_SELECTION,
                        (false, true) => colors::BG_HOVER,
                        (false, false) => colors::BG_DARK,
                    };

                    let stroke_c = if is_selected {
                        colors::ACCENT_CYAN
                    } else {
                        colors::BORDER_STRONG
                    };

                    ui.painter().rect_filled(node_rect, 4.0, bg_c);
                    ui.painter()
                        .rect_stroke(node_rect, 4.0, egui::Stroke::new(1.5_f32, stroke_c));

                    // Layer Label Color Square
                    let rgb = layer.label.to_rgb();
                    let label_c = egui::Color32::from_rgb(
                        (rgb[0] * 255.0) as u8,
                        (rgb[1] * 255.0) as u8,
                        (rgb[2] * 255.0) as u8,
                    );
                    let indicator_rect = egui::Rect::from_min_size(
                        egui::pos2(pos.x + 4.0, pos.y + 6.0),
                        egui::vec2(6.0, node_h - 12.0),
                    );
                    ui.painter().rect_filled(indicator_rect, 1.0, label_c);

                    // Icon prefix
                    let icon = match &layer.layer_type {
                        crate::core::timeline::LayerType::Video { .. } => "Video",
                        LayerType::Solid { .. } => "█",
                        LayerType::Image { .. } => "IMG",
                        LayerType::Model3D { .. } => "3D",
                        LayerType::Text { .. } => "T",
                        LayerType::Shape { .. } => "⬡",
                        LayerType::Null => "⌖",
                        LayerType::PreComp { .. } => "PRE",
                        LayerType::AdjustmentLayer => "◐",
                        LayerType::Audio { .. } => "AUD",
                        LayerType::Particle { .. } => "✦",
                    };

                    let title_text = format!("{} {}", icon, layer.name);
                    let mut job = egui::text::LayoutJob::simple_singleline(
                        title_text.clone(),
                        egui::FontId::proportional(11.0),
                        if is_selected {
                            colors::TEXT_PRIMARY
                        } else {
                            colors::TEXT_SECONDARY
                        },
                    );
                    job.wrap.max_width = node_w - 24.0;
                    job.wrap.max_rows = 1;
                    let galley = ui.fonts(|fonts| fonts.layout_job(job));
                    let text_pos =
                        egui::pos2(pos.x + 16.0, node_rect.center().y - galley.size().y * 0.5);
                    ui.painter()
                        .with_clip_rect(node_rect.intersect(ui.clip_rect()))
                        .galley(text_pos, galley, colors::TEXT_PRIMARY);
                    if node_response.clicked() {
                        selected_layers.clear();
                        selected_layers.insert(idx);
                        *selected_layer_idx = Some(idx);
                    }
                    node_response.on_hover_text(title_text);
                }
            });
    });
}
