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
        if crate::ui::custom_widgets::ae_button(ui, "💾 Save Current").clicked() {
            let ws = SavedWorkspace::capture(
                format!("Custom {}", app.custom_workspaces.len() + 1),
                app,
            );
            app.custom_workspaces.push(ws);
            crate::ui::preferences_dialog::save_workspaces(&app.custom_workspaces);
            app.toasts.info("Workspace saved".to_string());
        }
        if crate::ui::custom_widgets::ae_button(ui, "🔄 Reset").clicked() {
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
        }
    }
}

impl SavedWorkspace {
    pub fn capture(name: String, app: &KagariApp) -> Self {
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
        }
    }

    pub fn apply(&self, app: &mut KagariApp) {
        app.ui_tabs.left_tab_idx = self.left_tab;
        app.ui_tabs.right_tab_idx = self.right_tab;
        app.ui_tabs.bottom_dock_tab = self.bottom_dock_tab;
        app.ui_tabs.viewport_mag_ratio = self.viewport_mag_ratio;
        app.ui_tabs.show_switches_pane = self.show_switches_pane;
        app.ui_tabs.global_shy_active = self.global_shy_active;
        app.ui_tabs.layer_filter_text.clone_from(&self.layer_filter_text);
        app.ui_tabs.effects_search_query.clone_from(&self.effects_search_query);
        app.viewer_maximized = self.viewer_maximized;
        app.show_graph_editor = self.show_graph_editor;
        app.timeline_zoom = self.timeline_zoom;
    }
}
