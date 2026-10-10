mod composition;
mod dialogs;
mod effect_anim;
pub mod effect_factory;
mod file_edit;
mod import_helpers;
mod layer;
mod navigation;

pub use effect_factory::apply_effect_by_name;
pub(crate) use navigation::ASSETS_WORKSPACE_INDEX;
use navigation::{
    activate_workspace, active_workspace_index, composition_aspect_label, draw_compact_drawers,
    load_logo_if_needed, primary_workspace_index, SVG_REDO, SVG_UNDO,
};

use eframe::egui;

pub fn draw(app: &mut crate::KagariApp, ctx: &egui::Context) {
    draw_studio_header(app, ctx);
}

fn draw_studio_header(app: &mut crate::KagariApp, ctx: &egui::Context) {
    let compact_header =
        ctx.screen_rect().width() < crate::ui::theme::layout::DOCKED_PANELS_MIN_WIDTH;
    let project_name = std::path::Path::new(&app.project_path)
        .file_stem()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .filter(|name| *name != "project")
        .unwrap_or("名称未設定")
        .to_owned();
    let composition = app.history.current().active_composition();
    let is_pristine_start = composition.id == "comp_empty_start";
    let composition_label = if is_pristine_start {
        "コンポジション未作成".to_string()
    } else {
        composition.name.clone()
    };
    let workspace_id = egui::Id::new("studio_active_workspace");
    let mut active_workspace = active_workspace_index(app, ctx);
    let project_label = crate::ui::localization::tr("Project");
    let edit_label = crate::ui::localization::tr("Edit");
    let export_label = crate::ui::localization::tr("Export");
    let primary_workspaces = [
        (
            ASSETS_WORKSPACE_INDEX,
            project_label.as_str(),
            crate::ui::home_screen::HomeNav::Projects,
        ),
        (
            1,
            edit_label.as_str(),
            crate::ui::home_screen::HomeNav::Compositing,
        ),
        (
            8,
            export_label.as_str(),
            crate::ui::home_screen::HomeNav::Render,
        ),
    ];
    let other_workspaces = [
        (4, "カラー", crate::ui::home_screen::HomeNav::Color),
        (2, "エフェクト", crate::ui::home_screen::HomeNav::Effects),
        (5, "オーディオ", crate::ui::home_screen::HomeNav::Audio),
    ];
    let secondary_modes = [
        (3, "グラフ", crate::ui::home_screen::HomeNav::Graph),
        (6, "リタイム", crate::ui::home_screen::HomeNav::Retiming),
    ];
    let top_workspace = primary_workspace_index(active_workspace);
    egui::TopBottomPanel::top("studio_header")
        .exact_height(36.0)
        .frame(
            egui::Frame::none()
                .fill(crate::ui::theme::colors::BG_DEEPEST)
                .stroke(egui::Stroke::new(
                    1.0_f32,
                    crate::ui::theme::colors::BORDER_SUBTLE,
                ))
                .inner_margin(egui::Margin::symmetric(
                    if compact_header { 8.0 } else { 11.0 },
                    0.0,
                )),
        )
        .show(ctx, |ui| {
            let header_size = ui.available_size();
            ui.allocate_ui_with_layout(
                header_size,
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    load_logo_if_needed(app, ctx);
                    if let Some(texture) = app.home_banner.as_ref() {
                        ui.add(egui::Image::new(egui::load::SizedTexture::new(
                            texture.id(),
                            egui::vec2(
                                if compact_header { 26.0 } else { 28.0 },
                                if compact_header { 26.0 } else { 28.0 },
                            ),
                        )));
                    }
                    ui.add_space(if compact_header { 6.0 } else { 4.0 });
                    let brand = ui.add(
                        egui::Label::new(
                            egui::RichText::new("Kagari")
                                .size(if compact_header { 13.0 } else { 12.0 })
                                .color(crate::ui::theme::colors::TEXT_PRIMARY),
                        )
                        .sense(egui::Sense::click()),
                    );
                    if brand.clicked() {
                        app.show_home = true;
                        crate::ui::home_screen::set_home_nav(
                            ctx,
                            crate::ui::home_screen::HomeNav::Home,
                        );
                    }
                    brand.on_hover_text(crate::ui::localization::tr("Home"));
                    if ctx.screen_rect().width() >= 1200.0 {
                        ui.add_space(16.0);
                        crate::ui::icons::render_svg_bytes(
                            ui,
                            "studio-project-folder",
                            crate::ui::icons::SVG_FOLDER,
                            egui::vec2(18.0, 18.0),
                            crate::ui::theme::colors::TEXT_SECONDARY,
                        );
                        ui.add_space(6.0);
                        let full_project_context = if is_pristine_start {
                            project_name.clone()
                        } else {
                            format!("{project_name} / {composition_label}")
                        };
                        let display_context = if full_project_context.chars().count() > 36 {
                            let truncated: String = full_project_context.chars().take(33).collect();
                            format!("{truncated}...")
                        } else {
                            full_project_context.clone()
                        };
                        ui.label(
                            egui::RichText::new(display_context)
                                .size(14.0)
                                .color(crate::ui::theme::colors::TEXT_SECONDARY),
                        )
                        .on_hover_text(&full_project_context);
                        ui.add_space(6.0);
                    } else {
                        ui.add_space(10.0);
                    }
                    if compact_header {
                        ui.menu_button(
                            egui::RichText::new("画面")
                                .size(12.0)
                                .color(crate::ui::theme::colors::TEXT_PRIMARY),
                            |ui| {
                                for (workspace_index, label, nav) in primary_workspaces {
                                    let selected = top_workspace == workspace_index;
                                    if ui.selectable_label(selected, label).clicked() {
                                        activate_workspace(
                                            app,
                                            ctx,
                                            workspace_id,
                                            &mut active_workspace,
                                            workspace_index,
                                            nav,
                                        );
                                        ui.close_menu();
                                    }
                                }
                                ui.separator();
                                for (workspace_index, label, nav) in other_workspaces {
                                    if ui
                                        .selectable_label(
                                            active_workspace == workspace_index,
                                            label,
                                        )
                                        .clicked()
                                    {
                                        activate_workspace(
                                            app,
                                            ctx,
                                            workspace_id,
                                            &mut active_workspace,
                                            workspace_index,
                                            nav,
                                        );
                                        ui.close_menu();
                                    }
                                }
                                ui.separator();
                                for (workspace_index, label, nav) in secondary_modes {
                                    if ui
                                        .selectable_label(
                                            active_workspace == workspace_index,
                                            label,
                                        )
                                        .clicked()
                                    {
                                        activate_workspace(
                                            app,
                                            ctx,
                                            workspace_id,
                                            &mut active_workspace,
                                            workspace_index,
                                            nav,
                                        );
                                        ui.close_menu();
                                    }
                                }
                            },
                        );
                    } else {
                        for (workspace_index, label, nav) in primary_workspaces {
                            let response = crate::ui::theme::draw_custom_tab(
                                ui,
                                top_workspace == workspace_index,
                                label,
                            );
                            if response.clicked() {
                                activate_workspace(
                                    app,
                                    ctx,
                                    workspace_id,
                                    &mut active_workspace,
                                    workspace_index,
                                    nav,
                                );
                            }
                        }
                        ui.menu_button("⋯", |ui| {
                            for (workspace_index, label, nav) in other_workspaces {
                                if ui
                                    .selectable_label(active_workspace == workspace_index, label)
                                    .clicked()
                                {
                                    activate_workspace(
                                        app,
                                        ctx,
                                        workspace_id,
                                        &mut active_workspace,
                                        workspace_index,
                                        nav,
                                    );
                                    ui.close_menu();
                                }
                            }
                            ui.separator();
                            for (workspace_index, label, nav) in secondary_modes {
                                if ui
                                    .selectable_label(active_workspace == workspace_index, label)
                                    .clicked()
                                {
                                    activate_workspace(
                                        app,
                                        ctx,
                                        workspace_id,
                                        &mut active_workspace,
                                        workspace_index,
                                        nav,
                                    );
                                    ui.close_menu();
                                }
                            }
                        });
                    }
                    if compact_header {
                        draw_compact_drawers(ctx, app, ui);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.menu_button("⚙", |ui| {
                            if ui.button("最小化").clicked() {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                                ui.close_menu();
                            }
                            if ui.button("終了").clicked() {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                                ui.close_menu();
                            }
                        });
                        if ui.available_width() > 300.0 {
                            egui::ComboBox::from_id_salt("studio_global_preview_quality")
                                .width(116.0)
                                .selected_text(match app.viewport_fast_preview {
                                    0 => crate::ui::localization::tr("Full Quality"),
                                    1 => crate::ui::localization::tr("Adaptive Resolution"),
                                    2 => crate::ui::localization::tr("Fast Draft"),
                                    _ => crate::ui::localization::tr("Wireframe"),
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut app.viewport_fast_preview,
                                        0,
                                        crate::ui::localization::tr("Full Quality"),
                                    );
                                    ui.selectable_value(
                                        &mut app.viewport_fast_preview,
                                        1,
                                        crate::ui::localization::tr("Adaptive Resolution"),
                                    );
                                    ui.selectable_value(
                                        &mut app.viewport_fast_preview,
                                        2,
                                        crate::ui::localization::tr("Fast Draft"),
                                    );
                                    ui.selectable_value(
                                        &mut app.viewport_fast_preview,
                                        3,
                                        crate::ui::localization::tr("Wireframe"),
                                    );
                                });
                            let composition = app.history.current().active_composition();
                            if ui
                                .button(composition_aspect_label(
                                    composition.width,
                                    composition.height,
                                ))
                                .on_hover_text(crate::ui::localization::tr("Composition Settings"))
                                .clicked()
                            {
                                app.show_comp_settings = true;
                            }
                            let redo = ui
                                .add_enabled_ui(app.history.can_redo(), |ui| {
                                    crate::ui::kui::svg_icon_button(
                                        ui,
                                        "studio-redo",
                                        SVG_REDO,
                                        "Redo",
                                        false,
                                        26.0,
                                    )
                                })
                                .inner
                                .on_hover_text(crate::ui::localization::tr("Redo"));
                            if redo.clicked() && app.history.redo().is_some() {
                                crate::core::frame_cache::bump_version();
                                app.autosave.mark_dirty();
                                app.autosave_history_generation = app.history.generation();
                            }
                            let undo = ui
                                .add_enabled_ui(app.history.can_undo(), |ui| {
                                    crate::ui::kui::svg_icon_button(
                                        ui,
                                        "studio-undo",
                                        SVG_UNDO,
                                        "Undo",
                                        false,
                                        26.0,
                                    )
                                })
                                .inner
                                .on_hover_text(crate::ui::localization::tr("Undo"));
                            if undo.clicked() && app.history.undo().is_some() {
                                crate::core::frame_cache::bump_version();
                                app.autosave.mark_dirty();
                                app.autosave_history_generation = app.history.generation();
                            }
                        }
                        let width = ui.available_width();
                        let (_, response) = ui.allocate_exact_size(
                            egui::vec2(width.max(0.0), 28.0),
                            egui::Sense::click_and_drag(),
                        );
                        if response.drag_started() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                        }
                    });
                },
            );
        });
}

pub fn draw_legacy_menus(app: &mut crate::KagariApp, ctx: &egui::Context) {
    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| file_edit::draw_file_menu(app, ctx, ui));
            ui.menu_button("Edit", |ui| file_edit::draw_edit_menu(app, ui));
            ui.menu_button("Composition", |ui| {
                composition::draw_composition_menu(app, ctx, ui)
            });
            ui.menu_button("Layer", |ui| layer::draw_layer_menu(app, ui));
            ui.menu_button("Effect", |ui| effect_anim::draw_effect_menu(app, ui));
            ui.menu_button("Animation", |ui| effect_anim::draw_animation_menu(app, ui));
            ui.menu_button("View", |ui| effect_anim::draw_view_menu(app, ui));
            ui.menu_button("Help", |ui| effect_anim::draw_help_menu(app, ctx, ui));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let (btn_text, next_mode) = match app.skill_level {
                    crate::app_state::SkillLevel::Beginner => {
                        ("Mode: Beginner (Simple)", crate::app_state::SkillLevel::Pro)
                    }
                    crate::app_state::SkillLevel::Pro => (
                        "Mode: Pro Studio (Full)",
                        crate::app_state::SkillLevel::Beginner,
                    ),
                };
                if ui
                    .button(btn_text)
                    .on_hover_text("Toggle between Simple Beginner UI and Full Pro Studio Layout")
                    .clicked()
                {
                    app.skill_level = next_mode;
                    app.toasts
                        .info(format!("Switched UI Mode to {:?}", next_mode));
                }
            });
        });
    });
    dialogs::draw_dialogs(app, ctx);
}

#[cfg(test)]
mod tests {
    use super::primary_workspace_index;

    #[test]
    fn color_and_audio_are_distinct_top_level_workspaces() {
        assert_eq!(primary_workspace_index(1), 1);
        assert_eq!(primary_workspace_index(2), 1);
        assert_eq!(primary_workspace_index(3), 1);
        assert_eq!(primary_workspace_index(4), 4);
        assert_eq!(primary_workspace_index(5), 5);
        assert_eq!(primary_workspace_index(6), 1);
    }

    #[test]
    fn top_level_destinations_keep_their_own_navigation_group() {
        assert_eq!(primary_workspace_index(0), 0);
        assert_eq!(primary_workspace_index(7), 7);
        assert_eq!(primary_workspace_index(8), 8);
    }
}
