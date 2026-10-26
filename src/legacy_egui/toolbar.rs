use crate::ui::custom_widgets;
use crate::ui::icons::SVG_SNAP;
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

impl ActiveTool {
    /// Tools that directly manipulate layer and mask transforms
    pub fn is_transform(&self) -> bool {
        matches!(self, Self::Selection | Self::Rotation | Self::AnchorPoint)
    }

    /// Navigation tools (Hand / Zoom / Camera3D) that must not move layers or trigger drawing
    pub fn is_navigation(&self) -> bool {
        matches!(self, Self::Hand | Self::Zoom | Self::Camera3D)
    }

    /// Creation / content drawing tools with dedicated creation interactions
    pub fn is_creation(&self) -> bool {
        matches!(
            self,
            Self::Rectangle
                | Self::Pen
                | Self::Text
                | Self::Brush
                | Self::CloneStamp
                | Self::Eraser
                | Self::RotoBrush
                | Self::PuppetPin
        )
    }
}

#[derive(Clone, Copy)]
struct ToolDef {
    tool: ActiveTool,
    icon: &'static str,
    name: &'static str,
    shortcut: Option<crate::ui::shortcuts::ShortcutId>,
}

const TOOL_DEFS: &[ToolDef] = &[
    ToolDef {
        tool: ActiveTool::Selection,
        icon: crate::ui::icons::SVG_TOOL_SELECT,
        name: "Select",
        shortcut: Some(crate::ui::shortcuts::ShortcutId::SelectionTool),
    },
    ToolDef {
        tool: ActiveTool::Hand,
        icon: crate::ui::icons::SVG_TOOL_HAND,
        name: "Hand",
        shortcut: Some(crate::ui::shortcuts::ShortcutId::HandTool),
    },
    ToolDef {
        tool: ActiveTool::Zoom,
        icon: crate::ui::icons::SVG_TOOL_ZOOM,
        name: "Zoom",
        shortcut: Some(crate::ui::shortcuts::ShortcutId::ZoomTool),
    },
    ToolDef {
        tool: ActiveTool::Camera3D,
        icon: crate::ui::icons::SVG_TOOL_CAMERA,
        name: "Camera",
        shortcut: Some(crate::ui::shortcuts::ShortcutId::CameraTool),
    },
    ToolDef {
        tool: ActiveTool::Rotation,
        icon: crate::ui::icons::SVG_TOOL_ROTATE,
        name: "Rotate",
        shortcut: Some(crate::ui::shortcuts::ShortcutId::RotationTool),
    },
    ToolDef {
        tool: ActiveTool::AnchorPoint,
        icon: crate::ui::icons::SVG_TOOL_ANCHOR,
        name: "Anchor Point",
        shortcut: Some(crate::ui::shortcuts::ShortcutId::AnchorTool),
    },
    ToolDef {
        tool: ActiveTool::Rectangle,
        icon: crate::ui::icons::SVG_TOOL_SHAPE,
        name: "Shape",
        shortcut: Some(crate::ui::shortcuts::ShortcutId::RectangleTool),
    },
    ToolDef {
        tool: ActiveTool::Pen,
        icon: crate::ui::icons::SVG_TOOL_PEN,
        name: "Pen",
        shortcut: Some(crate::ui::shortcuts::ShortcutId::PenTool),
    },
    ToolDef {
        tool: ActiveTool::Text,
        icon: crate::ui::icons::SVG_TOOL_TEXT,
        name: "Text",
        shortcut: Some(crate::ui::shortcuts::ShortcutId::TextTool),
    },
    ToolDef {
        tool: ActiveTool::Brush,
        icon: crate::ui::icons::SVG_TOOL_BRUSH,
        name: "Brush",
        shortcut: None,
    },
    ToolDef {
        tool: ActiveTool::CloneStamp,
        icon: crate::ui::icons::SVG_TOOL_STAMP,
        name: "Clone Stamp",
        shortcut: None,
    },
    ToolDef {
        tool: ActiveTool::Eraser,
        icon: crate::ui::icons::SVG_TOOL_ERASER,
        name: "Eraser",
        shortcut: None,
    },
    ToolDef {
        tool: ActiveTool::RotoBrush,
        icon: crate::ui::icons::SVG_TOOL_ROTO,
        name: "Roto Brush",
        shortcut: None,
    },
    ToolDef {
        tool: ActiveTool::PuppetPin,
        icon: crate::ui::icons::SVG_TOOL_PUPPET,
        name: "Puppet Pin",
        shortcut: Some(crate::ui::shortcuts::ShortcutId::PuppetPinTool),
    },
];

const VIEWER_TOOLS: &[ActiveTool] = &[
    ActiveTool::Selection,
    ActiveTool::Hand,
    ActiveTool::Zoom,
    ActiveTool::Rotation,
    ActiveTool::AnchorPoint,
    ActiveTool::Pen,
    ActiveTool::Text,
    ActiveTool::Camera3D,
];

fn tool_definition(tool: ActiveTool) -> &'static ToolDef {
    TOOL_DEFS
        .iter()
        .find(|definition| definition.tool == tool)
        .expect("every toolbar tool must have metadata")
}

fn tool_tooltip(definition: &ToolDef) -> String {
    match definition.shortcut {
        Some(shortcut) => format!(
            "{} ({})",
            crate::ui::localization::tr(definition.name),
            crate::ui::shortcuts::display_shortcut(shortcut)
        ),
        None => crate::ui::localization::tr(definition.name).to_owned(),
    }
}

/// Compact tool strip for the composition Viewer. The full editing tool set
/// remains available through the overflow menu on narrow workspaces.
pub fn draw_viewer_tool_strip(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    use crate::ui::theme::colors;

    let tools = VIEWER_TOOLS
        .iter()
        .copied()
        .map(tool_definition)
        .collect::<Vec<_>>();
    let compact = ui.available_width() < 680.0;
    let visible_count = if compact { 3 } else { tools.len() };

    for (index, definition) in tools.iter().enumerate() {
        if (index == 3 || index == 7) && !compact {
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);
        }
        if index >= visible_count {
            break;
        }
        let selected = app.active_tool == definition.tool;
        let (rect, response) = ui.allocate_exact_size(egui::vec2(25.0, 25.0), egui::Sense::click());
        let fill = if selected || response.hovered() {
            colors::BG_HOVER
        } else {
            egui::Color32::TRANSPARENT
        };
        ui.painter().rect_filled(rect, 3.0, fill);
        if selected {
            ui.painter().line_segment(
                [
                    egui::pos2(rect.left() + 4.0, rect.bottom() - 1.0),
                    egui::pos2(rect.right() - 4.0, rect.bottom() - 1.0),
                ],
                egui::Stroke::new(2.0_f32, colors::ACCENT_SELECTION),
            );
        }
        let _ = crate::ui::icons::render_svg_at(
            ui,
            format!("viewer_tool_{:?}", definition.tool),
            definition.icon,
            rect.shrink(4.0).size(),
            if selected {
                colors::ACCENT_SELECTION
            } else {
                colors::TEXT_SECONDARY
            },
            rect.shrink(4.0).min,
        );
        if response.clicked() {
            app.active_tool = definition.tool;
        }
        response.on_hover_text(tool_tooltip(definition));
    }

    if compact {
        ui.menu_button("...", |ui| {
            for definition in tools.iter().skip(3) {
                let tooltip = tool_tooltip(definition);
                if ui
                    .selectable_label(app.active_tool == definition.tool, tooltip)
                    .clicked()
                {
                    app.active_tool = definition.tool;
                    ui.close_menu();
                }
            }
        });
    }
}

pub fn draw_viewer_tool_strip_vertical(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    use crate::ui::theme::colors;

    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
        ui.add_space(6.0);
        for (index, tool) in VIEWER_TOOLS.iter().copied().enumerate() {
            if index == 3 || index == 7 {
                ui.add_space(4.0);
                let (sep_r, _) =
                    ui.allocate_exact_size(egui::vec2(22.0, 1.0), egui::Sense::hover());
                ui.painter().rect_filled(sep_r, 0.5, colors::BORDER_SUBTLE);
                ui.add_space(4.0);
            }
            let definition = tool_definition(tool);
            let selected = app.active_tool == tool;
            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::click());
            if selected {
                ui.painter().rect_filled(rect, 6.0, colors::BG_ACTIVE);
                ui.painter().rect_stroke(
                    rect,
                    6.0,
                    egui::Stroke::new(1.0_f32, colors::ACCENT_BRAND.linear_multiply(0.8)),
                );
            } else if response.hovered() {
                ui.painter().rect_filled(rect, 6.0, colors::BG_HOVER);
                ui.painter().rect_stroke(
                    rect,
                    6.0,
                    egui::Stroke::new(1.0_f32, colors::BORDER_MEDIUM),
                );
            }
            crate::ui::icons::render_svg_at(
                ui,
                format!("viewer_tool_vertical_{tool:?}"),
                definition.icon,
                egui::vec2(16.0, 16.0),
                if selected {
                    colors::ACCENT_BRAND
                } else if response.hovered() {
                    colors::TEXT_PRIMARY
                } else {
                    colors::TEXT_SECONDARY
                },
                rect.center() - egui::vec2(8.0, 8.0),
            );
            if response.clicked() {
                app.active_tool = tool;
            }
            response.on_hover_text(tool_tooltip(definition));
            ui.add_space(3.0);
        }
    });
}

pub fn draw_tool_browser(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    crate::ui::kui::secondary(ui, "Select a tool for the Viewer.");
    ui.add_space(crate::ui::kui::space::SM);
    let list_height = ui.available_height().max(0.0);
    egui::ScrollArea::vertical()
        .id_salt("viewer_tool_browser")
        .max_height(list_height)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for (index, definition) in TOOL_DEFS.iter().enumerate() {
                let selected = app.active_tool == definition.tool;
                let row = crate::ui::kui::selectable_item_row(ui, selected, 32.0, |ui| {
                    let icon_id = format!("tool-browser-icon-{index}");
                    crate::ui::icons::render_svg_bytes(
                        ui,
                        &icon_id,
                        definition.icon,
                        egui::vec2(16.0, 16.0),
                        if selected {
                            crate::ui::theme::colors::ACCENT_SELECTION
                        } else {
                            crate::ui::theme::colors::TEXT_SECONDARY
                        },
                    );
                    ui.add_space(8.0);
                    ui.label(crate::ui::localization::tr(definition.name));
                });
                if row.response.clicked() {
                    app.active_tool = definition.tool;
                }
                row.response.on_hover_text(tool_tooltip(definition));
            }
        });
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

                for definition in TOOL_DEFS {
                    let tool = definition.tool;
                    let is_selected = app.active_tool == tool;
                    let accent = colors::ACCENT_SELECTION;
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
                        definition.icon,
                        icon_rect.size(),
                        tint,
                        icon_rect.min,
                    );
                    let _ = icon_resp;
                    let _ = rect; // rect used above
                    if resp.clicked() {
                        app.active_tool = tool;
                    }
                    resp.on_hover_text(tool_tooltip(definition));
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
                    colors::ACCENT_TOGGLE,
                    "Toggle Snapping to Keyframes and Markers (Shift+S)",
                );

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);

                ui.menu_button("Align", |ui| {
                    crate::ui::align_hud::draw_alignment_hud(app, ui);
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if custom_widgets::ae_button_accent(ui, "Export")
                        .on_hover_text("Export composition (Cmd+M)")
                        .clicked()
                    {
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
                            && app.ui_tabs.right_panel != crate::app_state::RightPanel::Effects
                        {
                            app.ui_tabs.right_panel = crate::app_state::RightPanel::Effects;
                        }
                    }
                });
            });
        });
}
