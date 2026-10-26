use super::*;

pub(in crate::ui::home_screen) fn draw_landing_home(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
) {
    // Reference measurements map to egui points. Do not shrink these on
    // Retina displays: egui already lays out in logical points.
    let scale = 1.0;
    let viewport = ui.available_rect_before_wrap();
    let width = viewport.width();
    let height = viewport.height();
    // Geometry values are expressed in reference-image pixels. On Retina
    // displays one egui point covers two pixels, so use the inverse scale.
    let margin = if width < 500.0 { 16.0 } else { 32.0 };
    let max_content_width = 1684.0;
    let inner_width = (width - margin * 2.0).min(max_content_width).max(1.0);
    let narrow = inner_width < 500.0;
    let content_inset = margin;
    let density = scale;
    let projects = recent_project_entries(app, ctx);
    // Home is a quick-launch surface: show the latest project in the first
    // slot of the three-column project grid. Keeping the reference tile width
    // prevents its cover artwork from being stretched across the whole page.
    let columns = 3;
    let card_height = 222.0 * density;
    let card_grid_height = card_height;
    let cards_top = if narrow { 270.0 } else { 241.0 } * density;
    let activity_heading_y = cards_top + card_grid_height + 56.0 * density;
    let table_header_y = activity_heading_y + if narrow { 54.0 } else { 34.0 } * density;
    let rows_top = table_header_y + 17.0 * density;
    let row_height = 36.0 * density;
    let activity_id = egui::Id::new("landing_activity_expanded");
    let activity_expanded =
        ctx.data_mut(|data| data.get_temp::<bool>(activity_id).unwrap_or(false));
    let visible_rows = projects
        .len()
        .min(if activity_expanded { 8 } else { 3 })
        .max(1);
    let content_height = height.max(rows_top + visible_rows as f32 * row_height + 18.0 * density);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let (content, _) =
                ui.allocate_exact_size(egui::vec2(width, content_height), egui::Sense::hover());
            let painter = ui.painter().with_clip_rect(content);
            painter.rect_filled(content, 0.0, egui::Color32::from_rgb(9, 14, 17));
            let left = content.left() + content_inset;
            let right = left + inner_width;

            painter.text(
                egui::pos2(
                    left,
                    content.top() + if narrow { 48.0 } else { 55.0 } * density,
                ),
                egui::Align2::LEFT_CENTER,
                "おかえりなさい",
                egui::FontId::proportional(if narrow { 28.0 } else { 32.0 } * density),
                colors::TEXT_PRIMARY,
            );
            painter.text(
                egui::pos2(
                    left,
                    content.top() + if narrow { 90.0 } else { 94.0 } * density,
                ),
                egui::Align2::LEFT_CENTER,
                if narrow {
                    "創造の続きから、\nまたは新しいプロジェクトを始めましょう。"
                } else {
                    "創造の続きから、または新しいプロジェクトを始めましょう。"
                },
                egui::FontId::proportional(if narrow { 12.0 } else { 14.0 } * density),
                colors::TEXT_SECONDARY,
            );

            let button_height = 38.0 * density;
            let button_gap = if narrow { 12.0 } else { 16.0 } * density;
            let new_button_width = 171.0_f32.min((inner_width - button_gap) * 0.5) * density;
            let open_button_width = 176.0_f32
                .min(inner_width / density - new_button_width / density - button_gap / density)
                * density;
            let button_top = content.top() + if narrow { 126.0 } else { 123.0 } * density;
            let new_button = egui::Rect::from_min_size(
                egui::pos2(left, button_top),
                egui::vec2(new_button_width, button_height),
            );
            let open_button = egui::Rect::from_min_size(
                egui::pos2(left + new_button_width + button_gap, button_top),
                egui::vec2(open_button_width, button_height),
            );
            if landing_button_rect(
                ui,
                new_button,
                "新規プロジェクト",
                crate::ui::icons::SVG_PLUS,
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

            draw_home_divider(
                &painter,
                left,
                right,
                content.top() + 184.0 * density,
                density,
            );

            let projects_heading_y = content.top() + 216.0 * density;
            painter.text(
                egui::pos2(left, projects_heading_y),
                egui::Align2::LEFT_CENTER,
                "最近のプロジェクト",
                egui::FontId::proportional(20.0 * density),
                colors::TEXT_PRIMARY,
            );
            let project_link = egui::Rect::from_min_size(
                egui::pos2(
                    if narrow {
                        left
                    } else {
                        right - 190.0 * density
                    },
                    projects_heading_y + if narrow { 10.0 } else { -16.0 } * density,
                ),
                egui::vec2(190.0 * density, 32.0 * density),
            );
            let project_link_response = ui.interact(
                project_link,
                egui::Id::new("landing-view-all-projects"),
                egui::Sense::click(),
            );
            painter.text(
                egui::pos2(
                    if narrow { left } else { right - 10.0 * density },
                    if narrow {
                        projects_heading_y + 26.0 * density
                    } else {
                        projects_heading_y
                    },
                ),
                if narrow {
                    egui::Align2::LEFT_CENTER
                } else {
                    egui::Align2::RIGHT_CENTER
                },
                "すべてのプロジェクトを見る  →",
                egui::FontId::proportional(11.0 * density),
                if project_link_response.hovered() {
                    colors::TEXT_PRIMARY
                } else {
                    colors::TEXT_SECONDARY
                },
            );
            if project_link_response.clicked() {
                set_home_nav(ctx, HomeNav::Projects);
            }

            let cards_rect = egui::Rect::from_min_size(
                egui::pos2(left, content.top() + cards_top),
                egui::vec2(inner_width, card_grid_height),
            );
            draw_landing_recent_projects(app, ui, cards_rect, &projects, columns);

            let activity_heading_y = content.top() + activity_heading_y;
            draw_home_divider(
                &painter,
                left,
                right,
                activity_heading_y - 25.0 * density,
                density,
            );
            painter.text(
                egui::pos2(left, activity_heading_y),
                egui::Align2::LEFT_CENTER,
                "最近のアクティビティ",
                egui::FontId::proportional(20.0 * density),
                colors::TEXT_PRIMARY,
            );
            let activity_link = egui::Rect::from_min_size(
                egui::pos2(
                    if narrow {
                        left
                    } else {
                        right - 210.0 * density
                    },
                    activity_heading_y + if narrow { 10.0 } else { -16.0 } * density,
                ),
                egui::vec2(210.0 * density, 32.0 * density),
            );
            let activity_link_response = ui.interact(
                activity_link,
                egui::Id::new("landing-view-all-activity"),
                egui::Sense::click(),
            );
            painter.text(
                egui::pos2(
                    if narrow { left } else { right - 10.0 * density },
                    if narrow {
                        activity_heading_y + 26.0 * density
                    } else {
                        activity_heading_y
                    },
                ),
                if narrow {
                    egui::Align2::LEFT_CENTER
                } else {
                    egui::Align2::RIGHT_CENTER
                },
                if activity_expanded {
                    "ホームに戻る  →"
                } else {
                    "すべてのアクティビティを見る  →"
                },
                egui::FontId::proportional(11.0 * density),
                if activity_link_response.hovered() {
                    colors::TEXT_PRIMARY
                } else {
                    colors::TEXT_SECONDARY
                },
            );
            if activity_link_response.clicked() {
                ctx.data_mut(|data| data.insert_temp(activity_id, !activity_expanded));
            }

            let header_y = content.top() + table_header_y;
            let (project_column_x, date_column_x) = activity_column_anchors(left, right, density);
            painter.text(
                egui::pos2(left + 6.0 * density, header_y),
                egui::Align2::LEFT_CENTER,
                "項目",
                egui::FontId::proportional(10.5 * density),
                colors::TEXT_SECONDARY,
            );
            painter.text(
                egui::pos2(project_column_x, header_y),
                egui::Align2::LEFT_CENTER,
                "プロジェクト",
                egui::FontId::proportional(10.5 * density),
                colors::TEXT_SECONDARY,
            );
            painter.text(
                egui::pos2(date_column_x, header_y),
                egui::Align2::LEFT_CENTER,
                "日付",
                egui::FontId::proportional(10.5 * density),
                colors::TEXT_SECONDARY,
            );
            draw_home_divider(&painter, left, right, content.top() + rows_top, density);
            draw_landing_recent_work(
                app,
                ui,
                left,
                right,
                content.top() + rows_top,
                &projects,
                density,
                visible_rows,
            );
        });
}

fn draw_home_divider(painter: &egui::Painter, left: f32, right: f32, y: f32, density: f32) {
    painter.line_segment(
        [egui::pos2(left, y), egui::pos2(right, y)],
        egui::Stroke::new(1.0 * density, colors::BORDER_SUBTLE),
    );
}
