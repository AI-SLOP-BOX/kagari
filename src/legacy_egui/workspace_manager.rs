use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq)]
enum WorkspaceLeftPanel {
    Project,
    Effects,
}

impl WorkspaceLeftPanel {
    fn index(self) -> usize {
        match self {
            Self::Project => 0,
            Self::Effects => 1,
        }
    }
}

struct BuiltInWorkspace {
    name: &'static str,
    description: &'static str,
    left_panel: WorkspaceLeftPanel,
    right_panel: crate::app_state::RightPanel,
}

pub fn draw_workspace_manager(app: &mut KagariApp, ui: &mut egui::Ui) {
    crate::ui::custom_widgets::ae_section_header(ui, "Workspaces", "📐");

    // Built-in workspace presets
    use crate::app_state::RightPanel;
    let workspaces = [
        BuiltInWorkspace {
            name: "Standard",
            description: "Default AE 3-Column Layout",
            left_panel: WorkspaceLeftPanel::Project,
            right_panel: RightPanel::Effects,
        },
        BuiltInWorkspace {
            name: "Small Screen",
            description: "Compact mode for laptops",
            left_panel: WorkspaceLeftPanel::Project,
            right_panel: RightPanel::Align,
        },
        BuiltInWorkspace {
            name: "Motion Tracking",
            description: "Tracker & viewport controls",
            left_panel: WorkspaceLeftPanel::Project,
            right_panel: RightPanel::Tracker,
        },
        BuiltInWorkspace {
            name: "Paint & Roto",
            description: "Paint brushes & vector masks",
            left_panel: WorkspaceLeftPanel::Project,
            right_panel: RightPanel::Paint,
        },
        BuiltInWorkspace {
            name: "Text & Type",
            description: "Character & paragraph panels",
            left_panel: WorkspaceLeftPanel::Project,
            right_panel: RightPanel::Character,
        },
        BuiltInWorkspace {
            name: "Color & Grading",
            description: "Color management & VU meters",
            left_panel: WorkspaceLeftPanel::Effects,
            right_panel: RightPanel::Color,
        },
        BuiltInWorkspace {
            name: "Minimal",
            description: "Maximizes viewport canvas",
            left_panel: WorkspaceLeftPanel::Effects,
            right_panel: RightPanel::Effects,
        },
        BuiltInWorkspace {
            name: "Audio",
            description: "Audio mixer & waveform",
            left_panel: WorkspaceLeftPanel::Effects,
            right_panel: RightPanel::AudioMixer,
        },
        BuiltInWorkspace {
            name: "3D Layout",
            description: "Camera views & 3D options",
            left_panel: WorkspaceLeftPanel::Project,
            right_panel: RightPanel::CameraLight,
        },
        BuiltInWorkspace {
            name: "Animation",
            description: "Keyframes, graph editor, presets",
            left_panel: WorkspaceLeftPanel::Project,
            right_panel: RightPanel::SpeedGraph,
        },
        BuiltInWorkspace {
            name: "Effects",
            description: "Effect controls & library focus",
            left_panel: WorkspaceLeftPanel::Project,
            right_panel: RightPanel::Effects,
        },
        BuiltInWorkspace {
            name: "Essential Graphics",
            description: "Master properties panel",
            left_panel: WorkspaceLeftPanel::Project,
            right_panel: RightPanel::EssentialGraphics,
        },
    ];

    let mut new_ws: Option<(usize, crate::app_state::RightPanel)> = None;

    egui::ScrollArea::vertical()
        .max_height(300.0)
        .show(ui, |ui| {
            for workspace in &workspaces {
                let is_active = app.ui_tabs.left_tab_idx == workspace.left_panel.index()
                    && app.ui_tabs.right_panel == workspace.right_panel;
                let (rect, response) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), 34.0),
                    egui::Sense::click(),
                );
                let fill = if is_active {
                    colors::BG_SELECTION
                } else if response.hovered() {
                    colors::BG_HOVER
                } else {
                    egui::Color32::TRANSPARENT
                };
                ui.painter().rect_filled(rect, 5.0, fill);
                if is_active {
                    ui.painter().rect_stroke(
                        rect,
                        5.0,
                        egui::Stroke::new(1.0_f32, colors::ACCENT_BRAND),
                    );
                }
                let icon = workspace_icon(workspace.name);
                let icon_rect = egui::Rect::from_min_size(
                    rect.min + egui::vec2(10.0, 7.0),
                    egui::vec2(20.0, 20.0),
                );
                let image = egui::Image::from_bytes(
                    format!(
                        "bytes://workspace-icon-{}.svg",
                        egui::Id::new(workspace.name).value()
                    ),
                    icon.as_bytes(),
                )
                .fit_to_exact_size(icon_rect.size());
                ui.put(icon_rect, image);
                ui.painter().text(
                    egui::pos2(icon_rect.right() + 10.0, rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    crate::ui::localization::tr(workspace.name),
                    egui::FontId::proportional(16.0),
                    if is_active {
                        colors::ACCENT_BRAND
                    } else {
                        colors::TEXT_PRIMARY
                    },
                );
                if response.clicked() {
                    new_ws = Some((workspace.left_panel.index(), workspace.right_panel));
                }
                ui.add_space(2.0);
                ui.label(
                    egui::RichText::new(crate::ui::localization::tr(workspace.description))
                        .size(14.0)
                        .color(colors::TEXT_MUTED),
                );
                ui.add_space(4.0);
            }
        });

    if let Some((l, r)) = new_ws {
        app.ui_tabs.left_tab_idx = l;
        app.ui_tabs.right_panel = r;
    }

    ui.add_space(4.0);
    ui.separator();

    // Custom workspace actions
    ui.label(
        egui::RichText::new(crate::ui::localization::tr("Custom Workspaces"))
            .small()
            .strong()
            .color(colors::TEXT_PRIMARY),
    );
    ui.add_space(2.0);

    ui.horizontal(|ui| {
        if crate::ui::custom_widgets::ae_button(ui, "Save Current").clicked() {
            let ws = SavedWorkspace::capture(
                format!(
                    "{} {}",
                    crate::ui::localization::tr("Custom"),
                    app.custom_workspaces.len() + 1
                ),
                app,
                ui.ctx(),
            );
            app.custom_workspaces.push(ws);
            match crate::ui::preferences_dialog::save_workspaces(&app.custom_workspaces) {
                Ok(()) => app
                    .toasts
                    .info(crate::ui::localization::tr("Workspace saved")),
                Err(error) => app.toasts.error(error),
            }
        }
        if crate::ui::custom_widgets::ae_button(ui, "Reset").clicked() {
            app.ui_tabs.left_tab_idx = 0;
            app.ui_tabs.right_panel = crate::app_state::RightPanel::Effects;
            app.toasts
                .info(crate::ui::localization::tr("Workspace reset to default"));
        }
    });

    // Show saved custom workspaces
    if !app.custom_workspaces.is_empty() {
        ui.add_space(4.0);
        let mut delete_idx = None;
        let mut apply_request = None;
        for (i, ws) in app.custom_workspaces.iter().enumerate() {
            let is_active = app.ui_tabs.left_tab_idx == ws.left_tab
                && app.ui_tabs.right_panel.index() == ws.right_tab;
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(
                        is_active,
                        egui::RichText::new(&ws.name).small().color(if is_active {
                            colors::ACCENT_WORKSPACE
                        } else {
                            colors::TEXT_PRIMARY
                        }),
                    )
                    .clicked()
                {
                    apply_request = Some(ws.clone());
                }
                if crate::ui::kui::button(ui, "✕", crate::ui::kui::ButtonKind::Quiet)
                    .on_hover_text(crate::ui::localization::tr("Delete"))
                    .clicked()
                {
                    delete_idx = Some(i);
                }
            });
        }
        if let Some(idx) = delete_idx {
            app.custom_workspaces.remove(idx);
            if let Err(error) =
                crate::ui::preferences_dialog::save_workspaces(&app.custom_workspaces)
            {
                app.toasts.error(error);
            }
        }
        if let Some(ws) = apply_request {
            ws.apply(app);
        }
    }
}

fn workspace_icon(name: &str) -> &'static str {
    use crate::ui::icons;
    match name {
        "Motion Tracking" => icons::SVG_TRACKING,
        "Paint & Roto" => icons::SVG_TOOL_BRUSH,
        "Text & Type" => icons::SVG_TOOL_TEXT,
        "Color & Grading" => icons::SVG_PALETTE,
        "Audio" => icons::SVG_AUDIO,
        "3D Layout" => icons::SVG_3D_CUBE,
        "Animation" => icons::SVG_GRAPH_EDITOR,
        "Effects" => icons::SVG_EFFECTS,
        "Essential Graphics" => icons::SVG_COMPOSITION,
        _ => icons::SVG_COMPOSITION,
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SavedWorkspace {
    pub name: String,
    pub left_tab: usize,
    pub right_tab: usize,
    pub bottom_dock_tab: usize,
    pub viewport_mag_ratio: f32,
    pub show_switches_pane: bool,
    pub global_shy_active: bool,
    pub layer_filter_text: String,
    pub effects_search_query: String,
    pub viewer_maximized: bool,
    pub show_graph_editor: bool,
    pub timeline_zoom: f32,
    #[serde(default)]
    pub left_panel_width: f32,
    #[serde(default)]
    pub right_panel_width: f32,
    #[serde(default)]
    pub timeline_height: f32,
    #[serde(default)]
    pub compact_project_drawer: bool,
    #[serde(default)]
    pub compact_inspector_drawer: bool,
}

impl Default for SavedWorkspace {
    fn default() -> Self {
        Self {
            name: String::new(),
            left_tab: 0,
            right_tab: 30,
            bottom_dock_tab: 0,
            viewport_mag_ratio: 0.0,
            show_switches_pane: true,
            global_shy_active: false,
            layer_filter_text: String::new(),
            effects_search_query: String::new(),
            viewer_maximized: false,
            show_graph_editor: false,
            timeline_zoom: 1.0,
            left_panel_width: 0.0,
            right_panel_width: 0.0,
            timeline_height: 0.0,
            compact_project_drawer: false,
            compact_inspector_drawer: false,
        }
    }
}

impl SavedWorkspace {
    pub fn capture(name: String, app: &KagariApp, ctx: &egui::Context) -> Self {
        Self {
            name,
            left_tab: app.ui_tabs.left_tab_idx,
            right_tab: app.ui_tabs.right_panel.index(),
            bottom_dock_tab: app.ui_tabs.bottom_dock_tab,
            viewport_mag_ratio: app.ui_tabs.viewport_mag_ratio,
            show_switches_pane: app.ui_tabs.show_switches_pane,
            global_shy_active: app.ui_tabs.global_shy_active,
            layer_filter_text: app.ui_tabs.layer_filter_text.clone(),
            effects_search_query: app.ui_tabs.effects_search_query.clone(),
            viewer_maximized: app.viewer_maximized,
            show_graph_editor: app.show_graph_editor,
            timeline_zoom: app.timeline_zoom,
            left_panel_width: panel_width(ctx, "left_panel", true),
            right_panel_width: panel_width(ctx, "right_panel", true),
            timeline_height: panel_width(ctx, "timeline_panel", false),
            compact_project_drawer: ctx
                .data(|data| data.get_temp::<bool>(egui::Id::new("compact_project_drawer")))
                .unwrap_or(false),
            compact_inspector_drawer: ctx
                .data(|data| data.get_temp::<bool>(egui::Id::new("compact_inspector_drawer")))
                .unwrap_or(false),
        }
    }

    pub fn apply(&self, app: &mut KagariApp) {
        app.ui_tabs.left_tab_idx = self.left_tab;
        app.ui_tabs.right_panel = crate::app_state::RightPanel::from_index(self.right_tab);
        app.ui_tabs.bottom_dock_tab = self.bottom_dock_tab;
        app.ui_tabs.viewport_mag_ratio = self.viewport_mag_ratio;
        app.ui_tabs.show_switches_pane = self.show_switches_pane;
        app.ui_tabs.global_shy_active = self.global_shy_active;
        app.ui_tabs
            .layer_filter_text
            .clone_from(&self.layer_filter_text);
        app.ui_tabs
            .effects_search_query
            .clone_from(&self.effects_search_query);
        app.viewer_maximized = self.viewer_maximized;
        app.show_graph_editor = self.show_graph_editor;
        app.timeline_zoom = self.timeline_zoom;
        if let Some(ctx) = app.ui_ctx.as_ref() {
            ctx.data_mut(|data| {
                data.insert_temp(
                    egui::Id::new("compact_project_drawer"),
                    self.compact_project_drawer,
                );
                data.insert_temp(
                    egui::Id::new("compact_inspector_drawer"),
                    self.compact_inspector_drawer,
                );
                data.insert_temp(workspace_restore_id(), self.clone());
            });
            ctx.request_repaint();
        }
    }

    fn restore_panel_sizes(&self, ctx: &egui::Context) {
        restore_panel_size(ctx, "left_panel", self.left_panel_width, true);
        restore_panel_size(ctx, "right_panel", self.right_panel_width, true);
        restore_panel_size(ctx, "timeline_panel", self.timeline_height, false);
    }
}

fn workspace_restore_id() -> egui::Id {
    egui::Id::new("pending_workspace_restore")
}

/// Applies a workspace's persisted egui panel geometry before the panels are
/// laid out. Applying it from inside a panel closure is too late: egui stores
/// that panel's measured size again when the closure returns.
pub fn restore_pending_workspace(ctx: &egui::Context) {
    let pending = ctx.data_mut(|data| data.remove_temp::<SavedWorkspace>(workspace_restore_id()));
    if let Some(workspace) = pending {
        workspace.restore_panel_sizes(ctx);
    }
}

fn panel_width(ctx: &egui::Context, id: &str, horizontal: bool) -> f32 {
    egui::containers::panel::PanelState::load(ctx, egui::Id::new(id))
        .map(|state| {
            if horizontal {
                state.size().x
            } else {
                state.size().y
            }
        })
        .unwrap_or(0.0)
}

fn restore_panel_size(ctx: &egui::Context, id: &str, size: f32, horizontal: bool) {
    if !size.is_finite() || size <= 0.0 {
        return;
    }
    let dimensions = if horizontal {
        egui::vec2(size, 1.0)
    } else {
        egui::vec2(1.0, size)
    };
    ctx.data_mut(|data| {
        data.insert_persisted(
            egui::Id::new(id),
            egui::containers::panel::PanelState {
                rect: egui::Rect::from_min_size(egui::Pos2::ZERO, dimensions),
            },
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_workspace_json_defaults_new_drawer_state() {
        let workspace: SavedWorkspace = serde_json::from_str(r#"{"name":"Legacy"}"#)
            .expect("legacy workspace should remain readable");
        assert!(!workspace.compact_project_drawer);
        assert!(!workspace.compact_inspector_drawer);
        assert_eq!(workspace.timeline_height, 0.0);
    }

    #[test]
    fn pending_workspace_restore_applies_geometry_once() {
        let ctx = egui::Context::default();
        ctx.data_mut(|data| {
            data.insert_temp(
                workspace_restore_id(),
                SavedWorkspace {
                    left_panel_width: 321.0,
                    right_panel_width: 345.0,
                    timeline_height: 278.0,
                    ..Default::default()
                },
            );
        });

        restore_pending_workspace(&ctx);

        assert_eq!(
            panel_width(&ctx, "left_panel", true),
            321.0,
            "left panel width should be restored before layout"
        );
        assert_eq!(panel_width(&ctx, "right_panel", true), 345.0);
        assert_eq!(panel_width(&ctx, "timeline_panel", false), 278.0);
        assert!(ctx
            .data_mut(|data| data.remove_temp::<SavedWorkspace>(workspace_restore_id()))
            .is_none());
    }
}
