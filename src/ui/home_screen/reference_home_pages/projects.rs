use super::*;

pub(in crate::ui::home_screen) fn draw_reference_project_row(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    compact: bool,
) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Recent Projects").size(18.0).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if reference_view_all_button(ui, "View All") {
                set_home_nav(ctx, HomeNav::Projects);
            }
        });
    });
    ui.add_space(if compact { 7.0 } else { 10.0 });
    let projects = recent_project_entries(ctx);
    if projects.is_empty() {
        let empty_height = if compact { 154.0 } else { 228.0 };
        let empty = fixed_card_with_inset(
            ui,
            egui::vec2(ui.available_width(), empty_height),
            egui::Color32::TRANSPARENT,
            crate::ui::theme::colors::BORDER_MEDIUM,
            7.0,
            if compact { 12.0 } else { 22.0 },
            |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(if compact { 20.0 } else { 34.0 });
                    crate::ui::icons::render_svg_bytes(
                        ui,
                        "empty-project-folder",
                        crate::ui::icons::SVG_FOLDER,
                        egui::vec2(
                            if compact { 30.0 } else { 42.0 },
                            if compact { 30.0 } else { 42.0 },
                        ),
                        colors::TEXT_SECONDARY,
                    );
                    ui.add_space(if compact { 8.0 } else { 12.0 });
                    ui.label(
                        egui::RichText::new("まだプロジェクトはありません")
                            .size(if compact { 14.0 } else { 19.0 })
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("新規プロジェクトを作成して、はじめましょう。")
                            .size(if compact { 10.0 } else { 14.0 })
                            .color(colors::TEXT_SECONDARY),
                    );
                    ui.add_space(if compact { 10.0 } else { 16.0 });
                    if reference_cta_button(ui, "新規プロジェクト", compact).clicked() {
                        enter_studio_new_project(app);
                        app.show_new_comp_dialog = true;
                    }
                });
            },
        );
        let _ = empty;
        return;
    }
    let available_width = ui.available_width();
    let columns = if available_width < 680.0 {
        1.0
    } else if available_width < 1040.0 {
        2.0
    } else {
        4.0
    };
    let card_gap = if columns == 1.0 {
        0.0
    } else if compact {
        12.0
    } else {
        18.0
    };
    let card_width = ((available_width - card_gap * (columns - 1.0)) / columns).max(140.0);
    let card_height = if compact { 150.0 } else { 215.0 };
    let image_height = if compact { 74.0 } else { 124.0 };
    let project_inset = if compact { 10.0 } else { 20.0 };
    let thumbnails = [
        "recent_project_eclipse.webp",
        "recent_project_citadel.webp",
        "recent_project_rift.webp",
        "recent_project_atlas.webp",
    ];
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for (card_index, project) in projects.iter().take(4).enumerate() {
            let asset = thumbnails[card_index % thumbnails.len()];
            let title = project.name.as_str();
            let meta = project
                .summary
                .as_ref()
                .and_then(|summary| summary.comp_names.get(summary.active_idx))
                .map(String::as_str)
                .unwrap_or("Project");
            let age = project
                .modified
                .map(fmt_age)
                .unwrap_or_else(|| "Not found".to_string());
            let response = fixed_card_with_inset(
                ui,
                egui::vec2(card_width, card_height),
                crate::ui::theme::colors::BG_PANEL_RAISED,
                crate::ui::theme::colors::BG_ELEVATED,
                5.0,
                0.0,
                |ui| {
                    if let Some(id) = reference_texture(app, ctx, asset) {
                        reference_cover_image(
                            ui,
                            id,
                            egui::vec2(card_width, image_height),
                            316.0 / 121.0,
                        );
                    }
                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        ui.add_space(project_inset);
                        ui.label(egui::RichText::new(title).size(16.0).strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(project_inset);
                            crate::ui::icons::render_svg_bytes(
                                ui,
                                &format!("{title}-more"),
                                crate::ui::icons::SVG_MORE,
                                egui::vec2(15.0, 15.0),
                                colors::TEXT_SECONDARY,
                            );
                        });
                    });
                    ui.horizontal(|ui| {
                        ui.add_space(project_inset);
                        ui.label(
                            egui::RichText::new(meta)
                                .size(12.0)
                                .color(colors::TEXT_SECONDARY),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.add_space(project_inset);
                        ui.label(
                            egui::RichText::new(&age)
                                .size(12.0)
                                .color(colors::TEXT_MUTED),
                        );
                    });
                },
            );
            if response.clicked() {
                open_project_path(app, &project.path);
            }
            if card_index < 3 {
                ui.add_space(card_gap);
            }
        }
    });
}
