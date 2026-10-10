use eframe::egui;

pub const ASSETS_WORKSPACE_INDEX: usize = 7;
pub const SVG_UNDO: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M9 14 4 9l5-5"/><path d="M4 9h10a6 6 0 1 1 0 12h-2"/></svg>"#;
pub const SVG_REDO: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="m15 4 5 5-5 5"/><path d="M20 9H10a6 6 0 1 0 0 12h2"/></svg>"#;

pub fn composition_aspect_label(width: u32, height: u32) -> String {
    let aspect = width as f32 / height.max(1) as f32;
    [
        ("16:9", 16.0 / 9.0),
        ("16:10", 16.0 / 10.0),
        ("4:3", 4.0 / 3.0),
        ("1:1", 1.0),
        ("9:16", 9.0 / 16.0),
        ("2.39:1", 2.39),
        ("2.35:1", 2.35),
    ]
    .into_iter()
    .find(|(_, preset)| (aspect - preset).abs() < 0.025)
    .map(|(label, _)| label.to_owned())
    .unwrap_or_else(|| format!("{aspect:.2}:1"))
}

pub fn load_logo_if_needed(app: &mut crate::KagariApp, ctx: &egui::Context) {
    if app.home_banner.is_none() {
        if let Ok(img) = crate::ui::embedded_assets::open_image(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/kagari_logo.webp"),
        ) {
            app.home_banner = crate::ui::home_screen::load_logo_texture(ctx, img);
        }
    }
}

pub fn activate_workspace(
    app: &mut crate::KagariApp,
    ctx: &egui::Context,
    workspace_id: egui::Id,
    active_workspace: &mut usize,
    workspace_index: usize,
    nav: crate::ui::home_screen::HomeNav,
) {
    *active_workspace = workspace_index;
    ctx.data_mut(|data| data.insert_temp(workspace_id, workspace_index));
    apply_workspace_nav(app, ctx, &nav);
}

pub fn primary_workspace_index(workspace_index: usize) -> usize {
    match workspace_index {
        1..=3 | 6 => 1,
        _ => workspace_index,
    }
}

pub(crate) fn apply_workspace_nav(
    app: &mut crate::KagariApp,
    ctx: &egui::Context,
    nav: &crate::ui::home_screen::HomeNav,
) {
    match nav {
        crate::ui::home_screen::HomeNav::Home => {
            app.show_home = true;
            crate::ui::home_screen::set_home_nav(ctx, crate::ui::home_screen::HomeNav::Home);
        }
        crate::ui::home_screen::HomeNav::Compositing => {
            app.show_home = false;
            app.show_guided_tutorial = false;
            app.show_graph_editor = false;
            app.ui_tabs.left_tab_idx = 0;
            app.ui_tabs.right_panel = crate::app_state::RightPanel::Properties;
            app.ui_tabs.bottom_dock_tab = 0;
        }
        crate::ui::home_screen::HomeNav::Effects => {
            app.show_home = false;
            app.show_guided_tutorial = false;
            app.show_graph_editor = false;
            app.ui_tabs.left_tab_idx = 1;
            app.ui_tabs.right_panel = crate::app_state::RightPanel::Effects;
            app.ui_tabs.bottom_dock_tab = 0;
        }
        crate::ui::home_screen::HomeNav::Graph => {
            app.show_home = false;
            app.show_guided_tutorial = false;
            app.show_graph_editor = true;
            app.ui_tabs.left_tab_idx = 0;
            app.ui_tabs.right_panel = crate::app_state::RightPanel::Properties;
            app.ui_tabs.bottom_dock_tab = 0;
        }
        crate::ui::home_screen::HomeNav::Color => {
            app.show_home = false;
            app.show_guided_tutorial = false;
            app.show_graph_editor = false;
            app.ui_tabs.left_tab_idx = 1;
            app.ui_tabs.right_panel = crate::app_state::RightPanel::Color;
            app.ui_tabs.bottom_dock_tab = 0;
        }
        crate::ui::home_screen::HomeNav::Audio => {
            app.show_home = false;
            app.show_guided_tutorial = false;
            app.show_graph_editor = false;
            app.ui_tabs.left_tab_idx = 0;
            app.ui_tabs.right_panel = crate::app_state::RightPanel::AudioMixer;
            app.ui_tabs.bottom_dock_tab = 0;
        }
        crate::ui::home_screen::HomeNav::Retiming => {
            app.show_home = false;
            app.show_guided_tutorial = false;
            app.show_graph_editor = false;
            app.ui_tabs.left_tab_idx = 0;
            app.ui_tabs.right_panel = crate::app_state::RightPanel::TimeRemap;
            app.ui_tabs.bottom_dock_tab = 0;
        }
        crate::ui::home_screen::HomeNav::Projects => {
            app.show_home = false;
            app.show_guided_tutorial = false;
            app.show_graph_editor = false;
            app.ui_tabs.left_tab_idx = 0;
            app.ui_tabs.right_panel = crate::app_state::RightPanel::Properties;
            app.ui_tabs.bottom_dock_tab = 0;
        }
        crate::ui::home_screen::HomeNav::Render => {
            app.show_home = false;
            app.show_guided_tutorial = false;
            app.show_graph_editor = false;
            app.ui_tabs.left_tab_idx = 0;
            app.ui_tabs.right_panel = crate::app_state::RightPanel::Properties;
            app.ui_tabs.bottom_dock_tab = 1;
        }
        _ => {}
    }
}

pub(crate) fn active_workspace_index(app: &crate::KagariApp, ctx: &egui::Context) -> usize {
    if app.show_home {
        0
    } else if app.ui_tabs.bottom_dock_tab == 1 {
        8
    } else if app.ui_tabs.left_tab_idx == 1
        && app.ui_tabs.right_panel == crate::app_state::RightPanel::Effects
    {
        2
    } else if app.ui_tabs.left_tab_idx == 1
        && app.ui_tabs.right_panel == crate::app_state::RightPanel::Color
    {
        4
    } else if app.ui_tabs.left_tab_idx == 0
        && app.ui_tabs.right_panel == crate::app_state::RightPanel::AudioMixer
    {
        5
    } else if app.ui_tabs.left_tab_idx == 0
        && app.ui_tabs.right_panel == crate::app_state::RightPanel::TimeRemap
    {
        6
    } else if app.show_graph_editor {
        3
    } else if app.ui_tabs.left_tab_idx == 0
        && app.ui_tabs.right_panel == crate::app_state::RightPanel::Properties
        && ctx.data(|data| {
            data.get_temp::<usize>(egui::Id::new("studio_active_workspace"))
                == Some(ASSETS_WORKSPACE_INDEX)
        })
    {
        7
    } else {
        1
    }
}

pub fn draw_compact_drawers(ctx: &egui::Context, app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    let drawer_id = egui::Id::new("compact_project_drawer");
    let drawer_open = ctx
        .data(|data| data.get_temp::<bool>(drawer_id))
        .unwrap_or(false);
    let drawer_label = if drawer_open {
        "パネルを閉じる"
    } else if app.ui_tabs.left_tab_idx == 1 {
        "エフェクト"
    } else {
        "プロジェクト"
    };
    if ui.small_button(drawer_label).clicked() {
        let next_open = !drawer_open;
        ctx.data_mut(|data| {
            data.insert_temp(drawer_id, next_open);
            if next_open {
                data.insert_temp(egui::Id::new("compact_inspector_drawer"), false);
            }
        });
    }
    if ctx.screen_rect().width() < crate::ui::theme::layout::DOCKED_PANELS_MIN_WIDTH {
        let inspector_id = egui::Id::new("compact_inspector_drawer");
        let inspector_open = ctx
            .data(|data| data.get_temp::<bool>(inspector_id))
            .unwrap_or(false);
        let inspector_label = if inspector_open {
            "インスペクターを閉じる"
        } else {
            "インスペクター"
        };
        if ui.small_button(inspector_label).clicked() {
            let next_open = !inspector_open;
            ctx.data_mut(|data| {
                data.insert_temp(inspector_id, next_open);
                if next_open {
                    data.insert_temp(egui::Id::new("compact_project_drawer"), false);
                }
            });
        }
    }
}
