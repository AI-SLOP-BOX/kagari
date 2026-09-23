use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;
use serde::{Deserialize, Serialize};

pub fn draw_workspace_manager(app: &mut KagariApp, ui: &mut egui::Ui) {
    crate::ui::custom_widgets::ae_section_header(ui, "Workspaces", "📐");

    // Built-in workspace presets
    let workspaces = [
        ("Standard", "Default AE 3-Column Layout", 0, 0),
        ("Small Screen", "Compact mode for laptops", 0, 1),
        ("Motion Tracking", "Tracker & viewport controls", 0, 2),
        ("Paint & Roto", "Paint brushes & vector masks", 0, 4),
        ("Text & Type", "Character & paragraph panels", 0, 7),
        ("Color & Grading", "Color management & VU meters", 1, 19),
        ("Minimal", "Maximizes viewport canvas", 1, 0),
        ("Audio", "Audio mixer & waveform", 1, 23),
        ("3D Layout", "Camera views & 3D options", 0, 25),
        ("Animation", "Keyframes, graph editor, presets", 0, 5),
        ("Effects", "Effect controls & library focus", 0, 10),
        ("Essential Graphics", "Master properties panel", 0, 12),
    ];

    let mut new_ws: Option<(usize, usize)> = None;

    egui::ScrollArea::vertical()
        .max_height(300.0)
        .show(ui, |ui| {
            for (name, desc, l_idx, r_idx) in workspaces.iter() {
                let is_active =
                    app.ui_tabs.left_tab_idx == *l_idx && app.ui_tabs.right_tab_idx == *r_idx;
                let response = ui.selectable_label(
                    is_active,
                    egui::RichText::new(*name).small().color(if is_active {
                        colors::ACCENT_CYAN
                    } else {
                        colors::TEXT_PRIMARY
                    }),
                );
                if response.clicked() {
                    new_ws = Some((*l_idx, *r_idx));
                }
                ui.label(egui::RichText::new(*desc).small().color(colors::TEXT_MUTED));
                ui.add_space(2.0);
            }
        });

    if let Some((l, r)) = new_ws {
        app.ui_tabs.left_tab_idx = l;
        app.ui_tabs.right_tab_idx = r;
    }

    ui.add_space(4.0);
    ui.separator();

    // Custom workspace actions
    ui.label(
        egui::RichText::new("Custom Workspaces")
            .small()
            .strong()
            .color(colors::TEXT_PRIMARY),
    );
    ui.add_space(2.0);

    ui.horizontal(|ui| {
        if crate::ui::custom_widgets::ae_button(ui, "Save Current").clicked() {
            let ws = SavedWorkspace::capture(
                format!("Custom {}", app.custom_workspaces.len() + 1),
                app,
                ui.ctx(),
            );
            app.custom_workspaces.push(ws);
            crate::ui::preferences_dialog::save_workspaces(&app.custom_workspaces);
            app.toasts.info("Workspace saved".to_string());
        }
        if crate::ui::custom_widgets::ae_button(ui, "Reset").clicked() {
            app.ui_tabs.left_tab_idx = 0;
            app.ui_tabs.right_tab_idx = 0;
            app.toasts.info("Workspace reset to default".to_string());
        }
    });

    // Show saved custom workspaces
    if !app.custom_workspaces.is_empty() {
        ui.add_space(4.0);
        let mut delete_idx = None;
        let mut apply_request = None;
        for (i, ws) in app.custom_workspaces.iter().enumerate() {
            let is_active = app.ui_tabs.left_tab_idx == ws.left_tab
                && app.ui_tabs.right_tab_idx == ws.right_tab;
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(
                        is_active,
                        egui::RichText::new(&ws.name).small().color(if is_active {
                            colors::ACCENT_CYAN
                        } else {
                            colors::TEXT_PRIMARY
                        }),
                    )
                    .clicked()
                {
                    apply_request = Some(ws.clone());
                }
                if ui.small_button("✕").on_hover_text("Delete").clicked() {
                    delete_idx = Some(i);
                }
            });
        }
        if let Some(idx) = delete_idx {
            app.custom_workspaces.remove(idx);
            crate::ui::preferences_dialog::save_workspaces(&app.custom_workspaces);
        }
        if let Some(ws) = apply_request {
            ws.apply(app);
        }
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
            right_tab: app.ui_tabs.right_tab_idx,
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
        app.ui_tabs.right_tab_idx = self.right_tab;
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
