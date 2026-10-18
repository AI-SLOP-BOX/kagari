use super::*;

pub(in crate::ui::home_screen) fn draw_landing_home(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
) {
    let scale = ctx
        .native_pixels_per_point()
        .unwrap_or(1.0)
        .recip()
        .clamp(0.5, 1.0);
    let viewport = ui.available_rect_before_wrap();
    let width = viewport.width();
    let height = viewport.height();
    let responsive_scale = (width / (1298.0 * scale)).max(0.72);
    let margin = (width * 0.032).clamp(
        18.0 * scale * responsive_scale,
        41.0 * scale * responsive_scale,
    );
    let max_content_width = 1217.0 * scale * responsive_scale;
    let inner_width = (width - margin * 2.0).clamp(1.0, max_content_width);
    let content_inset = ((width - inner_width) * 0.5).max(margin);
    let density = scale * responsive_scale * (inner_width / max_content_width).clamp(0.68, 1.0);
    let vertical = scale * (height / (993.0 * scale)).clamp(0.68, 1.3);
    let projects = recent_project_entries(app, ctx);
    let visible_project_count = projects.len().min(4);
    let card_gap = LANDING_CARD_GAP * density;
    let minimum_card_width = 274.0 * density;
    let mut columns = ((inner_width + card_gap) / (minimum_card_width + card_gap))
        .floor()
        .clamp(1.0, 4.0) as usize;
    if visible_project_count == 4 && columns == 3 {
        columns = 2;
    }
    let card_height = match columns {
        4 => 216.0 * density,
        3 => 200.0 * density,
        2 => 184.0 * density,
        _ => 164.0 * density,
    };
    let project_rows = visible_project_count.div_ceil(columns);
    let card_grid_height = if project_rows == 0 {
        card_height
    } else {
        project_rows as f32 * card_height + project_rows.saturating_sub(1) as f32 * 16.0 * density
    };
    let hero_height = (height * 0.30).clamp(150.0 * vertical, 300.0 * vertical);
    let header_top = hero_height - 3.0 * vertical;
    let stacked_header = inner_width < 850.0 * density;
    let stacked_buttons = inner_width < 470.0 * density;
    let button_height = if stacked_header {
        48.0 * density
    } else {
        56.0 * density
    };
    let cards_top = if stacked_header {
        header_top
            + 44.0 * vertical
            + button_height * if stacked_buttons { 2.0 } else { 1.0 }
            + if stacked_buttons {
                8.0 * vertical
            } else {
                18.0 * vertical
            }
    } else {
        header_top + button_height + 19.0 * vertical
    };
    let work_heading_y = cards_top + card_grid_height + 65.0 * vertical;
    let rows_top = work_heading_y + 44.0 * vertical;
    let work_rows = projects
        .iter()
        .filter(|project| project.path.is_file())
        .take(5)
        .count()
        .max(1);
    let content_height =
        (rows_top + work_rows as f32 * 44.0 * density + 64.0 * vertical).max(height);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let (content, _) =
                ui.allocate_exact_size(egui::vec2(width, content_height), egui::Sense::hover());
            let painter = ui.painter().with_clip_rect(content);
            painter.rect_filled(content, 0.0, egui::Color32::from_rgb(8, 15, 17));
            let left = content.left() + content_inset;
            let right = left + inner_width;
            let hero = egui::Rect::from_min_max(
                egui::pos2(content.left() + margin, content.top()),
                egui::pos2(content.right() - margin, content.top() + hero_height),
            );
            draw_landing_hero(ui, hero);
            let title_size = (inner_width * 0.044).clamp(30.0 * density, 53.0 * density) * 0.86;
            painter.text(
                egui::pos2(left, content.top() + hero_height * 0.633),
                egui::Align2::LEFT_CENTER,
                "Kagari VFX",
                egui::FontId::proportional(title_size),
                colors::TEXT_PRIMARY,
            );

            if stacked_header {
                painter.text(
                    egui::pos2(left, content.top() + header_top + 14.0 * vertical),
                    egui::Align2::LEFT_CENTER,
                    "最近のプロジェクト",
                    egui::FontId::proportional(21.0 * density),
                    colors::TEXT_PRIMARY,
                );
            } else {
                painter.text(
                    egui::pos2(left, content.top() + header_top + button_height * 0.5),
                    egui::Align2::LEFT_CENTER,
                    "最近のプロジェクト",
                    egui::FontId::proportional(21.0 * density),
                    colors::TEXT_PRIMARY,
                );
            }

            let button_gap = 18.0 * density;
            let button_width = if stacked_header {
                if stacked_buttons {
                    inner_width
                } else {
                    ((inner_width - 10.0 * density) * 0.5).min(208.0 * density)
                }
            } else {
                208.0 * density
            };
            let button_y = if stacked_header {
                content.top() + header_top + 44.0 * vertical
            } else {
                content.top() + header_top
            };
            let new_button = if stacked_header {
                egui::Rect::from_min_size(
                    egui::pos2(left, button_y),
                    egui::vec2(button_width, button_height),
                )
            } else {
                egui::Rect::from_min_size(
                    egui::pos2(right - button_width * 2.0 - button_gap, button_y),
                    egui::vec2(button_width, button_height),
                )
            };
            let open_button = if stacked_header {
                let open_y = button_y
                    + if stacked_buttons {
                        button_height + 8.0 * vertical
                    } else {
                        0.0
                    };
                egui::Rect::from_min_size(
                    egui::pos2(
                        if stacked_buttons {
                            left
                        } else {
                            left + button_width + 10.0 * density
                        },
                        open_y,
                    ),
                    egui::vec2(button_width, button_height),
                )
            } else {
                egui::Rect::from_min_size(
                    egui::pos2(right - button_width, button_y),
                    egui::vec2(button_width, button_height),
                )
            };
            if landing_button_rect(
                ui,
                new_button,
                "新規プロジェクト",
                crate::ui::icons::SVG_FILE_PLUS,
                true,
            )
            .clicked()
            {
                set_home_nav(ctx, HomeNav::NewProject);
            }
            if landing_button_rect(
                ui,
                open_button,
                "プロジェクトを開く",
                crate::ui::icons::SVG_OPEN_FOLDER,
                false,
            )
            .clicked()
            {
                enter_studio_open_dialog(app);
            }

            let cards_rect = egui::Rect::from_min_size(
                egui::pos2(left, content.top() + cards_top),
                egui::vec2(inner_width, card_grid_height),
            );
            draw_landing_recent_projects(app, ui, cards_rect, &projects);

            painter.text(
                egui::pos2(left, content.top() + work_heading_y),
                egui::Align2::LEFT_CENTER,
                "最近の作業",
                egui::FontId::proportional(21.0 * density),
                colors::TEXT_PRIMARY,
            );
            let view_all_rect = egui::Rect::from_min_size(
                egui::pos2(
                    right - 126.0 * density,
                    content.top() + work_heading_y - 16.0 * density,
                ),
                egui::vec2(126.0 * density, 32.0 * density),
            );
            let view_all = ui.interact(
                view_all_rect,
                egui::Id::new("landing-view-all-projects"),
                egui::Sense::click(),
            );
            painter.text(
                view_all_rect.center() - egui::vec2(8.0 * density, 0.0),
                egui::Align2::CENTER_CENTER,
                "すべて表示",
                egui::FontId::proportional(13.0 * density),
                if view_all.hovered() {
                    colors::TEXT_PRIMARY
                } else {
                    colors::TEXT_SECONDARY
                },
            );
            crate::ui::icons::render_svg_at(
                ui,
                "landing-view-all-arrow".to_owned(),
                crate::ui::icons::SVG_ARROW_RIGHT,
                egui::vec2(14.0 * density, 14.0 * density),
                if view_all.hovered() {
                    colors::TEXT_PRIMARY
                } else {
                    colors::TEXT_SECONDARY
                },
                egui::pos2(
                    view_all_rect.center().x + 39.0 * density,
                    view_all_rect.center().y - 7.0 * density,
                ),
            );
            if view_all.clicked() {
                set_home_nav(ctx, HomeNav::Projects);
            }
            draw_landing_recent_work(app, ui, left, right, content.top() + rows_top, &projects);
        });
}

#[path = "recent.rs"]
mod recent;
use recent::*;
