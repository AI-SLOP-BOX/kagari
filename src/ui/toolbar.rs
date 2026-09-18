use crate::ui::custom_widgets;
use eframe::egui;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActiveTool {
    #[default]
    Selection,
    Hand,
    Zoom,
    Camera3D,
    Rotation,
    AnchorPoint,
    Rectangle,
    Pen,
    Text,
    Brush,
    CloneStamp,
    Eraser,
    RotoBrush,
    PuppetPin,
}

/// Compact tool strip for the composition Viewer. The full editing tool set
/// remains available through the overflow menu on narrow workspaces.
pub fn draw_viewer_tool_strip(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    use crate::ui::icons::*;
    use crate::ui::theme::colors;

    let tools: [(ActiveTool, &'static str, &'static str); 8] = [
        (ActiveTool::Selection, SVG_TOOL_SELECT, "Select (V)"),
        (ActiveTool::Hand, SVG_TOOL_HAND, "Hand (H)"),
        (ActiveTool::Zoom, SVG_TOOL_ZOOM, "Zoom (Z)"),
        (ActiveTool::Rotation, SVG_TOOL_ROTATE, "Rotate (W)"),
        (ActiveTool::AnchorPoint, SVG_TOOL_ANCHOR, "Anchor Point (Y)"),
        (ActiveTool::Pen, SVG_TOOL_PEN, "Pen (G)"),
        (ActiveTool::Text, SVG_TOOL_TEXT, "Text (Cmd+T)"),
        (ActiveTool::Camera3D, SVG_TOOL_CAMERA, "Camera (C)"),
    ];
    let compact = ui.available_width() < 680.0;
    let visible_count = if compact { 3 } else { tools.len() };

    for (index, (tool, svg, tooltip)) in tools.into_iter().enumerate() {
        if (index == 3 || index == 7) && !compact {
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);
        }
        if index >= visible_count {
            break;
        }
        let selected = app.active_tool == tool;
        let (rect, response) = ui.allocate_exact_size(egui::vec2(25.0, 25.0), egui::Sense::click());
        let fill = if selected || response.hovered() {
            colors::BG_HOVER
        } else {
            egui::Color32::TRANSPARENT
        };
        ui.painter().rect_filled(rect, 3.0, fill);
        if selected {
            ui.painter().line_segment(
                [egui::pos2(rect.left() + 4.0, rect.bottom() - 1.0), egui::pos2(rect.right() - 4.0, rect.bottom() - 1.0)],
                egui::Stroke::new(2.0_f32, colors::ACCENT_ORANGE),
            );
        }
        let _ = crate::ui::icons::render_svg_at(
            ui,
            format!("viewer_tool_{tool:?}"),
            svg,
            rect.shrink(4.0).size(),
            if selected { colors::ACCENT_ORANGE } else { colors::TEXT_SECONDARY },
            rect.shrink(4.0).min,
        );
        if response.clicked() {
            app.active_tool = tool;
        }
        response.on_hover_text(tooltip);
    }

    if compact {
        ui.menu_button("...", |ui| {
            for (tool, _svg, tooltip) in tools.into_iter().skip(3) {
                if ui.selectable_label(app.active_tool == tool, tooltip).clicked() {
                    app.active_tool = tool;
                    ui.close_menu();
                }
            }
        });
    }
}

#[allow(dead_code)]
pub fn draw(app: &mut crate::KagariApp, ctx: &egui::Context) {
    use crate::ui::theme::colors;
    let frame = egui::Frame::none()
        .fill(colors::BG_DARK)
        .inner_margin(egui::Margin::symmetric(8.0, 5.0))
        .stroke(egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE));

    egui::TopBottomPanel::top("ae_toolbar")
        .frame(frame)
        .default_height(42.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.style_mut().spacing.item_spacing.x = 3.0;

                // Vector tool icons with hover tooltips
                let tools: [(ActiveTool, &'static str, &'static str); 14] = [
                    (ActiveTool::Selection, SVG_TOOL_SELECT, "Select (V)"),
                    (ActiveTool::Hand, SVG_TOOL_HAND, "Hand (H)"),
                    (ActiveTool::Zoom, SVG_TOOL_ZOOM, "Zoom (Z)"),
                    (ActiveTool::Camera3D, SVG_TOOL_CAMERA, "Camera (C)"),
                    (ActiveTool::Rotation, SVG_TOOL_ROTATE, "Rotate (W)"),
                    (ActiveTool::AnchorPoint, SVG_TOOL_ANCHOR, "Anchor Point (Y)"),
                    (ActiveTool::Rectangle, SVG_TOOL_SHAPE, "Shape (Q)"),
                    (ActiveTool::Pen, SVG_TOOL_PEN, "Pen (G)"),
                    (ActiveTool::Text, SVG_TOOL_TEXT, "Text (Cmd+T)"),
                    (ActiveTool::Brush, SVG_TOOL_BRUSH, "Brush"),
                    (ActiveTool::CloneStamp, SVG_TOOL_STAMP, "Clone Stamp"),
                    (ActiveTool::Eraser, SVG_TOOL_ERASER, "Eraser"),
                    (ActiveTool::RotoBrush, SVG_TOOL_ROTO, "Roto Brush"),
                    (ActiveTool::PuppetPin, SVG_TOOL_PUPPET, "Puppet Pin"),
                ];
                use crate::ui::icons::*;

                for (tool, svg, tooltip) in tools {
                    let is_selected = app.active_tool == tool;
                    let accent = colors::ACCENT_BLUE;
                    let tint = if is_selected {
                        accent
                    } else {
                        colors::TEXT_SECONDARY
                    };

                    let (rect, resp) =
                        ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::click());
                    let fill = if is_selected || resp.hovered() {
                        colors::BG_HOVER
                    } else {
                        egui::Color32::TRANSPARENT
                    };
                    ui.painter().rect_filled(rect, 4.0, fill);
                    if is_selected {
                        ui.painter()
                            .rect_stroke(rect, 4.0, egui::Stroke::new(1.0_f32, accent));
                        // Active-tool underline (matches tab active states)
                        ui.painter().line_segment(
                            [
                                egui::pos2(rect.left() + 4.0, rect.bottom() - 1.0),
                                egui::pos2(rect.right() - 4.0, rect.bottom() - 1.0),
                            ],
                            egui::Stroke::new(2.0_f32, accent),
                        );
                    }
                    // Draw icon centered inside the button rect
                    let icon_rect = rect.shrink(4.0);
                    let icon_resp = crate::ui::icons::render_svg_at(
                        ui,
                        format!("tool_{:?}", tool),
                        svg,
                        icon_rect.size(),
                        tint,
                        icon_rect.min,
                    );
                    let _ = icon_resp;
                    let _ = rect; // rect used above
                    if resp.clicked() {
                        app.active_tool = tool;
                    }
                    resp.on_hover_text(tooltip);
                }

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(4.0);

                // AE Snapping Toggle (Vector SVG Icon)
                custom_widgets::ae_svg_toggle(
                    ui,
                    &mut app.snap_to_keyframes,
                    SVG_SNAP,
                    "tb_snap_btn",
                    egui::vec2(22.0, 22.0),
                    colors::ACCENT_CYAN,
                    "Toggle Snapping to Keyframes and Markers (Shift+S)",
                );

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);

                ui.menu_button("Align", |ui| {
                    crate::ui::align_hud::draw_alignment_hud(app, ui);
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if custom_widgets::ae_button_accent(ui, "Export").on_hover_text("Export composition (Cmd+M)").clicked() {
                        app.export.show_export_dialog = true;
                    }
                    ui.add_space(8.0);
                    let resp = ui.add_sized(
                        [120.0, 18.0],
                        egui::TextEdit::singleline(&mut app.ui_tabs.effects_search_query)
                            .hint_text("Search Effects..."),
                    );
                    if resp.changed() {
                        // Automatically switch right tab to Effects panel (tab 0) if typing search query
                        if !app.ui_tabs.effects_search_query.is_empty()
                            && app.ui_tabs.right_tab_idx != 0
                        {
                            app.ui_tabs.right_tab_idx = 0;
                        }
                    }
                });
            });
        });
}
