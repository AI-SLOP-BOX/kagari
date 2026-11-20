#![allow(clippy::too_many_arguments)]
use super::*;

#[derive(Clone, Copy, Debug)]
pub(crate) struct FlowchartDragWireView {
    pub(crate) start: (f64, f64),
    pub(crate) current: (f64, f64),
    pub(crate) is_matte: bool,
}

pub(crate) fn draw_flowchart(
    scene: &mut Scene,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    comp: &kagari_vfx::core::timeline::Composition,
    selected_layer_id: Option<&str>,
    drag_wire: Option<FlowchartDragWireView>,
) {
    let bounds = Rect::new(x, y, x + width, y + height);
    // Obsidian dot-grid canvas background with crisp 1px border
    scene.fill(
        Fill::NonZero,
        Affine::IDENTITY,
        Color::from_rgb8(11, 12, 14),
        None,
        &bounds,
    );
    scene.stroke(
        &Stroke::new(1.0),
        Affine::IDENTITY,
        Color::from_rgb8(34, 38, 46),
        None,
        &bounds,
    );

    let grid_size = 22.0;
    let dot_color = Color::from_rgba8(64, 72, 86, 120);
    let mut gy = y + grid_size;
    while gy < y + height {
        let mut gx = x + grid_size;
        while gx < x + width {
            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                dot_color,
                None,
                &Circle::new((gx, gy), 1.0),
            );
            gx += grid_size;
        }
        gy += grid_size;
    }

    let layers_count = comp.layers.len();
    if layers_count == 0 {
        return;
    }

    let graph = kagari_vfx::core::flow_graph_model::FlowGraphModel::build_structure(comp);
    let node_w = 150.0;
    let node_h = 38.0;
    let col_spacing = 180.0;
    let row_spacing = 52.0;

    let origin_x = x + 24.0;
    let origin_y = y + 20.0;
    let positions = graph.positions_f64(origin_x, origin_y, col_spacing, row_spacing);

    // 1. Draw Connecting Wires with bezier curves
    for (idx, node) in graph.nodes.iter().enumerate() {
        if idx >= positions.len() {
            continue;
        }
        let (cx, cy) = positions[idx];
        let child_input = (cx, cy + node_h * 0.5);

        // Parent Connection Wire (Cyan curve)
        if let Some(parent_idx) = node.parent_index {
            if parent_idx < positions.len() {
                let (px, py) = positions[parent_idx];
                let parent_output = (px + node_w, py + node_h * 0.5);

                let mut path = BezPath::new();
                path.move_to(parent_output);
                let ctrl1 = (parent_output.0 + 40.0, parent_output.1);
                let ctrl2 = (child_input.0 - 40.0, child_input.1);
                path.curve_to(ctrl1, ctrl2, child_input);

                // Glow stroke
                scene.stroke(
                    &Stroke::new(3.0),
                    Affine::IDENTITY,
                    Color::from_rgba8(0, 210, 255, 60),
                    None,
                    &path,
                );
                // Core stroke
                scene.stroke(
                    &Stroke::new(1.5),
                    Affine::IDENTITY,
                    Color::from_rgb8(0, 220, 255),
                    None,
                    &path,
                );
            }
        }

        // Track Matte Connection Wire (Amber / Orange curve)
        if let Some(matte_idx) = node.matte_index {
            if matte_idx < positions.len() {
                let (mx, my) = positions[matte_idx];
                let matte_output = (mx + node_w, my + node_h * 0.5);

                let mut path = BezPath::new();
                path.move_to(matte_output);
                let ctrl1 = (matte_output.0 + 35.0, matte_output.1);
                let ctrl2 = (child_input.0 - 35.0, child_input.1);
                path.curve_to(ctrl1, ctrl2, child_input);

                // Glow stroke
                scene.stroke(
                    &Stroke::new(3.0),
                    Affine::IDENTITY,
                    Color::from_rgba8(255, 170, 0, 60),
                    None,
                    &path,
                );
                // Core stroke
                scene.stroke(
                    &Stroke::new(1.5),
                    Affine::IDENTITY,
                    Color::from_rgb8(255, 180, 20),
                    None,
                    &path,
                );
            }
        }
    }

    // 2. Draw Layer Card Nodes
    for (idx, layer) in comp.layers.iter().enumerate() {
        if idx >= positions.len() {
            continue;
        }
        let (nx, ny) = positions[idx];
        if ny + node_h < y || ny > y + height || nx + node_w < x || nx > x + width {
            continue;
        }

        let is_selected = selected_layer_id == Some(layer.id.as_str());
        let node_rect = RoundedRect::new(nx, ny, nx + node_w, ny + node_h, 5.0);

        // Card background fill
        let bg_color = if is_selected {
            Color::from_rgba8(26, 44, 58, 230)
        } else {
            Color::from_rgba8(22, 25, 30, 210)
        };
        scene.fill(Fill::NonZero, Affine::IDENTITY, bg_color, None, &node_rect);

        // Card border stroke
        let border_color = if is_selected {
            Color::from_rgb8(0, 220, 255)
        } else {
            Color::from_rgb8(45, 52, 62)
        };
        let border_width = if is_selected { 1.8 } else { 1.0 };
        scene.stroke(
            &Stroke::new(border_width),
            Affine::IDENTITY,
            border_color,
            None,
            &node_rect,
        );

        // Layer Label Color Bar (left accent)
        let label_color = if layer.label == kagari_vfx::core::timeline::LabelColor::None {
            Color::from_rgb8(80, 90, 100)
        } else {
            let [r, g, b] = layer.label.to_rgb();
            Color::from_rgb8(
                (r.clamp(0.0, 1.0) * 255.0) as u8,
                (g.clamp(0.0, 1.0) * 255.0) as u8,
                (b.clamp(0.0, 1.0) * 255.0) as u8,
            )
        };
        let color_bar = RoundedRect::new(nx + 4.0, ny + 6.0, nx + 9.0, ny + node_h - 6.0, 2.0);
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            label_color,
            None,
            &color_bar,
        );

        // Layer type badge & status pills inside node card
        let badge_x = nx + 16.0;
        let badge_y = ny + 10.0;
        let badge_w = 18.0;
        let badge_h = 18.0;
        let badge_rect =
            RoundedRect::new(badge_x, badge_y, badge_x + badge_w, badge_y + badge_h, 3.0);

        let (badge_bg, badge_stroke_color) = match &layer.layer_type {
            kagari_vfx::core::timeline::LayerType::Text { .. } => (
                Color::from_rgba8(40, 100, 200, 180),
                Color::from_rgb8(80, 160, 255),
            ),
            kagari_vfx::core::timeline::LayerType::Shape { .. } => (
                Color::from_rgba8(180, 70, 20, 180),
                Color::from_rgb8(255, 120, 40),
            ),
            kagari_vfx::core::timeline::LayerType::AdjustmentLayer => (
                Color::from_rgba8(120, 40, 180, 180),
                Color::from_rgb8(190, 90, 255),
            ),
            kagari_vfx::core::timeline::LayerType::Particle { .. } => (
                Color::from_rgba8(20, 140, 120, 180),
                Color::from_rgb8(40, 220, 190),
            ),
            _ => (
                Color::from_rgba8(50, 60, 75, 180),
                Color::from_rgb8(100, 120, 145),
            ),
        };
        scene.fill(Fill::NonZero, Affine::IDENTITY, badge_bg, None, &badge_rect);
        scene.stroke(
            &Stroke::new(1.0),
            Affine::IDENTITY,
            badge_stroke_color,
            None,
            &badge_rect,
        );

        // Draw geometric symbol inside badge:
        match &layer.layer_type {
            kagari_vfx::core::timeline::LayerType::Text { .. } => {
                // "T" icon: horizontal bar + vertical stem
                let bar = Line::new(
                    (badge_x + 4.0, badge_y + 5.0),
                    (badge_x + badge_w - 4.0, badge_y + 5.0),
                );
                let stem = Line::new(
                    (badge_x + badge_w * 0.5, badge_y + 5.0),
                    (badge_x + badge_w * 0.5, badge_y + badge_h - 4.0),
                );
                scene.stroke(
                    &Stroke::new(1.5),
                    Affine::IDENTITY,
                    Color::WHITE,
                    None,
                    &bar,
                );
                scene.stroke(
                    &Stroke::new(1.5),
                    Affine::IDENTITY,
                    Color::WHITE,
                    None,
                    &stem,
                );
            }
            kagari_vfx::core::timeline::LayerType::Shape { .. } => {
                // Star/diamond polygon
                let diamond = RoundedRect::new(
                    badge_x + 4.0,
                    badge_y + 4.0,
                    badge_x + badge_w - 4.0,
                    badge_y + badge_h - 4.0,
                    2.0,
                );
                scene.stroke(
                    &Stroke::new(1.3),
                    Affine::IDENTITY,
                    Color::WHITE,
                    None,
                    &diamond,
                );
            }
            _ => {
                // Dot icon
                let dot = Circle::new((badge_x + badge_w * 0.5, badge_y + badge_h * 0.5), 3.0);
                scene.fill(Fill::NonZero, Affine::IDENTITY, Color::WHITE, None, &dot);
            }
        }

        // Layer Name Pill & Details Bar
        let name_bar = RoundedRect::new(nx + 40.0, ny + 11.0, nx + node_w - 20.0, ny + 19.0, 2.0);
        let name_bar_color = if is_selected {
            Color::from_rgba8(220, 235, 255, 230)
        } else {
            Color::from_rgba8(140, 155, 175, 180)
        };
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            name_bar_color,
            None,
            &name_bar,
        );

        // Subtitle / Effects count indicator line
        let has_effects = layer.effects_enabled && !layer.effects.is_empty();
        let sub_w = if has_effects { 55.0 } else { 35.0 };
        let sub_bar = RoundedRect::new(nx + 40.0, ny + 23.0, nx + 40.0 + sub_w, ny + 27.0, 1.5);
        let sub_color = if has_effects {
            Color::from_rgba8(0, 210, 255, 180)
        } else {
            Color::from_rgba8(80, 95, 115, 140)
        };
        scene.fill(Fill::NonZero, Affine::IDENTITY, sub_color, None, &sub_bar);

        // 3D layer badge indicator (cyan cube corner)
        if layer.is_3d {
            let cube_dot = Circle::new((nx + node_w - 28.0, ny + 25.0), 2.5);
            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                Color::from_rgb8(0, 220, 255),
                None,
                &cube_dot,
            );
        }

        // Input port pin (left side: Cyan)
        let in_pin = Circle::new((nx, ny + node_h * 0.5), 3.5);
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            Color::from_rgb8(0, 210, 255),
            None,
            &in_pin,
        );
        scene.stroke(
            &Stroke::new(1.0),
            Affine::IDENTITY,
            Color::from_rgb8(13, 15, 17),
            None,
            &in_pin,
        );

        // Parent Output port pin (right side top: Gold / Yellow)
        let out_pin = Circle::new((nx + node_w, ny + 12.0), 3.5);
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            Color::from_rgb8(255, 210, 60),
            None,
            &out_pin,
        );
        scene.stroke(
            &Stroke::new(1.0),
            Affine::IDENTITY,
            Color::from_rgb8(13, 15, 17),
            None,
            &out_pin,
        );

        // Matte Source Output port pin (right side bottom: Amber / Orange)
        let matte_pin = Circle::new((nx + node_w, ny + node_h - 12.0), 3.5);
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            Color::from_rgb8(255, 130, 20),
            None,
            &matte_pin,
        );
        scene.stroke(
            &Stroke::new(1.0),
            Affine::IDENTITY,
            Color::from_rgb8(13, 15, 17),
            None,
            &matte_pin,
        );
    }

    // 3. Draw Interactive Dragging Wire (Active Pin-to-Pin connection)
    if let Some(FlowchartDragWireView {
        start,
        current,
        is_matte,
    }) = drag_wire
    {
        let mut path = BezPath::new();
        path.move_to(start);
        let dx = (current.0 - start.0).abs().max(40.0) * 0.5;
        let ctrl1 = (start.0 + dx, start.1);
        let ctrl2 = (current.0 - dx, current.1);
        path.curve_to(ctrl1, ctrl2, current);

        let (glow_color, core_color, end_color) = if is_matte {
            (
                Color::from_rgba8(255, 130, 20, 90),
                Color::from_rgb8(255, 150, 40),
                Color::from_rgb8(255, 170, 60),
            )
        } else {
            (
                Color::from_rgba8(255, 230, 80, 80),
                Color::from_rgb8(255, 235, 100),
                Color::from_rgb8(255, 240, 120),
            )
        };

        scene.stroke(&Stroke::new(4.0), Affine::IDENTITY, glow_color, None, &path);
        scene.stroke(&Stroke::new(2.0), Affine::IDENTITY, core_color, None, &path);
        let end_point = Circle::new(current, 4.0);
        scene.fill(Fill::NonZero, Affine::IDENTITY, end_color, None, &end_point);
    }
}
