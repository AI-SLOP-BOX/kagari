mod effect_anim;
mod effect_factory;
mod file_edit;
mod import_helpers;
mod layer;

use eframe::egui;

fn draw_header_action(ui: &mut egui::Ui, label: &str, width: f32, size: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 28.0), egui::Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 4.0, crate::ui::theme::colors::BG_HOVER);
    }
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(size),
        crate::ui::theme::colors::TEXT_SECONDARY,
    );
    response.on_hover_text("Open Render Queue")
}

pub fn draw(app: &mut crate::KagariApp, ctx: &egui::Context) {
    if !app.show_home && ctx.screen_rect().width() >= 1200.0 {
        draw_reference_studio_header(app, ctx);
        return;
    }
    draw_studio_header(app, ctx);
}

fn draw_reference_studio_header(app: &mut crate::KagariApp, ctx: &egui::Context) {
    egui::TopBottomPanel::top("studio_header")
        .exact_height(48.0)
        .resizable(false)
        .frame(egui::Frame::none().fill(crate::ui::theme::colors::BG_DEEPEST))
        .show(ctx, |ui| {
            let rect = ui.max_rect();
            ui.painter().line_segment(
                [egui::pos2(rect.left(), rect.bottom() - 1.0), egui::pos2(rect.right(), rect.bottom() - 1.0)],
                egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(43, 55, 65)),
            );
            ui.allocate_new_ui(
                egui::UiBuilder::new().max_rect(rect).layout(egui::Layout::left_to_right(egui::Align::Center)),
                |ui| {
                    ui.add_space(17.0);
                    load_logo_if_needed(app, ctx);
                    if let Some(texture) = app.home_banner.as_ref() {
                        ui.add(egui::Image::new(egui::load::SizedTexture::new(texture.id(), egui::vec2(34.0, 34.0))));
                    }
                    ui.add_space(7.0);
                    ui.label(egui::RichText::new("Kagari VFX").size(17.0).strong().color(crate::ui::theme::colors::TEXT_PRIMARY));
                    ui.add_space(22.0);
                    crate::ui::icons::render_svg_bytes(ui, "studio-breadcrumb-arrow", crate::ui::icons::SVG_CHEVRON_RIGHT, egui::vec2(18.0, 18.0), crate::ui::theme::colors::TEXT_SECONDARY);
                    ui.add_space(18.0);
                    ui.painter().line_segment(
                        [egui::pos2(ui.cursor().left(), rect.top() + 10.0), egui::pos2(ui.cursor().left(), rect.bottom() - 10.0)],
                        egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(42, 54, 64)),
                    );
                    ui.add_space(18.0);
                    crate::ui::icons::render_svg_bytes(ui, "studio-project-folder", crate::ui::icons::SVG_FOLDER, egui::vec2(18.0, 18.0), egui::Color32::from_rgb(174, 190, 207));
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new("Sample Project / main_comp").size(14.0).color(crate::ui::theme::colors::TEXT_SECONDARY));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let render_response = draw_header_action(ui, "Render", 68.0, 13.0);
                        if render_response.clicked() { app.show_home = false; app.ui_tabs.bottom_dock_tab = 1; }
                        ui.add_space(14.0);
                        ui.label(egui::RichText::new("Autosaved 10:24").size(12.0).color(crate::ui::theme::colors::TEXT_SECONDARY));
                    });
                },
            );
        });
}

fn draw_studio_header(app: &mut crate::KagariApp, ctx: &egui::Context) {
    let compact_header = ctx.screen_rect().width() < 1200.0;
    let composition_label = app.history.current().active_composition().name.clone();
    let workspace_id = egui::Id::new("studio_active_workspace");
    let inferred_workspace = if app.show_home { 0_usize }
        else if app.ui_tabs.left_tab_idx == 1 && app.ui_tabs.right_tab_idx == 1 { 4 }
        else if app.ui_tabs.left_tab_idx == 1 && app.ui_tabs.right_tab_idx == 0 { 2 }
        else { 1 };
    let mut active_workspace = ctx.data_mut(|data| data.get_temp::<usize>(workspace_id)).unwrap_or(inferred_workspace);
    if app.show_home { active_workspace = 0; }
    else if active_workspace == 0 {
        active_workspace = inferred_workspace;
        ctx.data_mut(|data| data.insert_temp(workspace_id, active_workspace));
    }
    let workspaces = [
        ("Home", crate::ui::home_screen::HomeNav::Home),
        ("Compositing", crate::ui::home_screen::HomeNav::Compositing),
        ("Effects", crate::ui::home_screen::HomeNav::Effects),
        ("Assets", crate::ui::home_screen::HomeNav::Projects),
        ("Render", crate::ui::home_screen::HomeNav::Render),
    ];
    let workspace_label = workspaces.get(active_workspace).map(|(label, _)| *label).unwrap_or("Compositing");
    egui::TopBottomPanel::top("studio_header")
        .exact_height(66.0)
        .frame(
            egui::Frame::none()
                .fill(crate::ui::theme::colors::BG_DEEPEST)
                .stroke(egui::Stroke::new(1.0_f32, crate::ui::theme::colors::BORDER_SUBTLE))
                .inner_margin(egui::Margin::symmetric(if compact_header { 8.0 } else { 24.0 }, 0.0)),
        )
        .show(ctx, |ui| {
            let header_size = ui.available_size();
            ui.allocate_ui_with_layout(header_size, egui::Layout::left_to_right(egui::Align::Center), |ui| {
                load_logo_if_needed(app, ctx);
                if let Some(texture) = app.home_banner.as_ref() {
                    ui.add(egui::Image::new(egui::load::SizedTexture::new(texture.id(), egui::vec2(
                        if compact_header { 28.0 } else { 38.0 },
                        if compact_header { 28.0 } else { 38.0 },
                    ))));
                }
                ui.add_space(if compact_header { 6.0 } else { 10.0 });
                ui.label(egui::RichText::new("Kagari VFX").size(if compact_header { 15.0 } else { 20.0 }).color(crate::ui::theme::colors::TEXT_PRIMARY));
                ui.add_space(if compact_header { 12.0 } else { 22.0 });
                crate::ui::icons::render_svg_bytes(ui, "studio-project-folder", crate::ui::icons::SVG_FOLDER, egui::vec2(if compact_header { 16.0 } else { 18.0 }, if compact_header { 16.0 } else { 18.0 }), crate::ui::theme::colors::TEXT_SECONDARY);
                ui.add_space(6.0);
                ui.label(egui::RichText::new(format!("Sample Project / {composition_label}")).size(if compact_header { 12.0 } else { 14.0 }).color(crate::ui::theme::colors::TEXT_SECONDARY));
                ui.add_space(6.0);
                ui.menu_button(
                    egui::RichText::new(workspace_label).size(if compact_header { 12.0 } else { 13.0 }).color(crate::ui::theme::colors::TEXT_PRIMARY),
                    |ui| {
                        for (workspace_index, (label, nav)) in workspaces.iter().enumerate() {
                            if ui.selectable_label(active_workspace == workspace_index, *label).clicked() {
                                active_workspace = workspace_index;
                                ctx.data_mut(|data| data.insert_temp(workspace_id, active_workspace));
                                apply_workspace_nav(app, nav);
                                ui.close_menu();
                            }
                        }
                    },
                );
                if compact_header { draw_compact_drawers(ctx, app, ui); }
                let right_width = if compact_header { 220.0 } else { 390.0 };
                ui.add_space((ui.available_width() - right_width).max(0.0));
                ui.allocate_ui_with_layout(egui::vec2(right_width, header_size.y), egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let render_response = draw_header_action(ui, "Render", if compact_header { 68.0 } else { 76.0 }, if compact_header { 12.0 } else { 13.0 });
                    if render_response.clicked() { app.show_home = false; app.ui_tabs.bottom_dock_tab = 1; }
                    ui.add_space(12.0);
                    ui.label(egui::RichText::new("Autosaved").size(11.0).color(crate::ui::theme::colors::TEXT_MUTED));
                });
            });
        });
}

fn load_logo_if_needed(app: &mut crate::KagariApp, ctx: &egui::Context) {
    if app.home_banner.is_none() {
        if let Ok(img) = crate::ui::embedded_assets::open_image(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/kagari_logo.webp"),
        ) {
            app.home_banner = crate::ui::home_screen::load_logo_texture(ctx, img);
        }
    }
}

fn apply_workspace_nav(app: &mut crate::KagariApp, nav: &crate::ui::home_screen::HomeNav) {
    match nav {
        crate::ui::home_screen::HomeNav::Home => app.show_home = true,
        crate::ui::home_screen::HomeNav::Compositing => { app.show_home = false; app.ui_tabs.left_tab_idx = 0; app.ui_tabs.right_tab_idx = 30; }
        crate::ui::home_screen::HomeNav::Effects => { app.show_home = false; app.ui_tabs.left_tab_idx = 1; app.ui_tabs.right_tab_idx = 0; }
        crate::ui::home_screen::HomeNav::Projects => { app.show_home = false; app.ui_tabs.left_tab_idx = 0; app.ui_tabs.right_tab_idx = 30; }
        crate::ui::home_screen::HomeNav::Render => { app.show_home = false; app.ui_tabs.left_tab_idx = 1; app.ui_tabs.right_tab_idx = 1; }
        _ => {}
    }
}

fn draw_compact_drawers(ctx: &egui::Context, app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    let drawer_id = egui::Id::new("compact_project_drawer");
    let drawer_open = ctx.data(|data| data.get_temp::<bool>(drawer_id)).unwrap_or(false);
    let drawer_label = if drawer_open { "Close Panel" } else if app.ui_tabs.left_tab_idx == 1 { "Effects" } else { "Project" };
    if ui.small_button(drawer_label).clicked() {
        let next_open = !drawer_open;
        ctx.data_mut(|data| {
            data.insert_temp(drawer_id, next_open);
            if next_open { data.insert_temp(egui::Id::new("compact_inspector_drawer"), false); }
        });
    }
    if ctx.screen_rect().width() < 950.0 {
        let inspector_id = egui::Id::new("compact_inspector_drawer");
        let inspector_open = ctx.data(|data| data.get_temp::<bool>(inspector_id)).unwrap_or(false);
        let inspector_label = if inspector_open { "Hide Inspector" } else { "Inspector" };
        if ui.small_button(inspector_label).clicked() {
            let next_open = !inspector_open;
            ctx.data_mut(|data| {
                data.insert_temp(inspector_id, next_open);
                if next_open { data.insert_temp(egui::Id::new("compact_project_drawer"), false); }
            });
        }
    }
}

pub fn draw_legacy_menus(app: &mut crate::KagariApp, ctx: &egui::Context) {
    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| file_edit::draw_file_menu(app, ctx, ui));
            ui.menu_button("Edit", |ui| file_edit::draw_edit_menu(app, ui));
            ui.menu_button("Composition", |ui| layer::draw_composition_menu(app, ctx, ui));
            ui.menu_button("Layer", |ui| layer::draw_layer_menu(app, ui));
            ui.menu_button("Effect", |ui| effect_anim::draw_effect_menu(app, ui));
            ui.menu_button("Animation", |ui| effect_anim::draw_animation_menu(app, ui));
            ui.menu_button("View", |ui| effect_anim::draw_view_menu(app, ui));
            ui.menu_button("Help", |ui| effect_anim::draw_help_menu(app, ctx, ui));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let (btn_text, next_mode) = match app.skill_level {
                    crate::app_state::SkillLevel::Beginner => ("Mode: Beginner (Simple)", crate::app_state::SkillLevel::Pro),
                    crate::app_state::SkillLevel::Pro => ("Mode: Pro Studio (Full)", crate::app_state::SkillLevel::Beginner),
                };
                if ui.button(btn_text).on_hover_text("Toggle between Simple Beginner UI and Full Pro Studio Layout").clicked() {
                    app.skill_level = next_mode;
                    app.toasts.info(format!("Switched UI Mode to {:?}", next_mode));
                }
            });
        });
    });
    effect_anim::draw_dialogs(app, ctx);
}
