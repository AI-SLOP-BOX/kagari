use super::*;

fn recent_project_path_label(path: &std::path::Path) -> String {
    let full = path.to_string_lossy();
    if full.chars().count() <= 54 {
        return full.into_owned();
    }

    let file_name = path.file_name().map(|name| name.to_string_lossy());
    let parent_name = path
        .parent()
        .and_then(std::path::Path::file_name)
        .map(|name| name.to_string_lossy());
    match (parent_name, file_name) {
        (Some(parent), Some(file)) => format!("…/{parent}/{file}"),
        (_, Some(file)) => format!("…/{file}"),
        _ => {
            let suffix = full
                .chars()
                .rev()
                .take(53)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<String>();
            format!("…{suffix}")
        }
    }
}

pub(in crate::ui::home_screen) fn draw_landing_recent_work(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    left: f32,
    right: f32,
    top: f32,
    projects: &[RecentProject],
    density: f32,
    row_limit: usize,
) {
    let painter = ui.painter().clone();
    let entries = projects.iter().take(row_limit);
    let mut open_path = None;
    let row_height = 36.0 * density;
    let (project_column_x, date_column_x) = activity_column_anchors(left, right, density);
    let mut count = 0;
    for (index, project) in entries.enumerate() {
        count += 1;
        let row = egui::Rect::from_min_size(
            egui::pos2(left, top + index as f32 * row_height),
            egui::vec2(right - left, row_height),
        );
        let response = ui.interact(
            row,
            egui::Id::new(("landing-recent-work", index)),
            egui::Sense::click(),
        );
        if response.hovered() {
            painter.rect_filled(row, 2.0 * density, colors::BG_HOVER);
        }
        painter.line_segment(
            [row.left_bottom(), row.right_bottom()],
            egui::Stroke::new(1.0 * density, colors::BORDER_SUBTLE),
        );
        crate::ui::icons::render_svg_at(
            ui,
            format!("landing-work-icon-{index}"),
            crate::ui::icons::SVG_FILE,
            egui::vec2(15.0 * density, 15.0 * density),
            colors::TEXT_SECONDARY_BRIGHT,
            egui::pos2(left + 14.0 * density, row.center().y - 7.5 * density),
        );
        let item_rect = egui::Rect::from_min_max(
            egui::pos2(left + 48.0 * density, row.top()),
            egui::pos2(project_column_x - 12.0 * density, row.bottom()),
        );
        painter.with_clip_rect(item_rect).text(
            item_rect.left_center(),
            egui::Align2::LEFT_CENTER,
            format!("{} を更新しました", project.name),
            egui::FontId::proportional(11.5 * density),
            colors::TEXT_PRIMARY,
        );
        let project_rect = egui::Rect::from_min_max(
            egui::pos2(project_column_x, row.top()),
            egui::pos2(date_column_x - 12.0 * density, row.bottom()),
        );
        painter.with_clip_rect(project_rect).text(
            project_rect.left_center(),
            egui::Align2::LEFT_CENTER,
            &project.name,
            egui::FontId::proportional(11.5 * density),
            colors::TEXT_SECONDARY,
        );
        painter.text(
            egui::pos2(date_column_x, row.center().y),
            egui::Align2::LEFT_CENTER,
            project
                .modified
                .map(fmt_timestamp)
                .unwrap_or_else(|| "—".into()),
            egui::FontId::proportional(10.5 * density),
            colors::TEXT_SECONDARY,
        );
        if response.clicked() {
            open_path = Some(project.path.clone());
        }
    }
    if count == 0 {
        let empty =
            egui::Rect::from_min_size(egui::pos2(left, top), egui::vec2(right - left, row_height));
        painter.line_segment(
            [empty.left_top(), empty.right_top()],
            egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
        );
        painter.text(
            empty.left_center() + egui::vec2(14.0 * density, 0.0),
            egui::Align2::LEFT_CENTER,
            "最近のアクティビティはここに表示されます",
            egui::FontId::proportional(11.5 * density),
            colors::TEXT_MUTED,
        );
    }
    if let Some(path) = open_path {
        open_project_path(app, &path);
    }
}

pub(in crate::ui::home_screen) fn draw_landing_recent_projects(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    rect: egui::Rect,
    entries: &[RecentProject],
    columns: usize,
) {
    let scale = (ui.ctx().screen_rect().width() / 1584.0).max(1.0);
    let density = scale;
    let painter = ui.painter().with_clip_rect(rect);
    if entries.is_empty() {
        painter.rect_filled(rect, 6.0, egui::Color32::from_rgb(13, 22, 25));
        painter.rect_stroke(rect, 6.0, egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE));
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "プロジェクトを作成するか、既存のプロジェクトを開いてください",
            egui::FontId::proportional(14.0 * density),
            colors::TEXT_MUTED,
        );
        return;
    }

    let gap = 12.0 * density;
    let columns = columns.max(1).min(3);
    let card_width =
        ((rect.width() - gap * (columns - 1) as f32) / columns as f32).max(150.0 * density);
    let card_height = 222.0 * density;
    let cover_height = 136.0 * density;

    let mut open_path = None;
    let mut remove_path = None;
    for (index, project) in entries.iter().take(1).enumerate() {
        let column = index % columns;
        let x = rect.left() + column as f32 * (card_width + gap);
        let y = rect.top();
        let card = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(card_width, card_height));
        let response = ui.interact(
            card,
            egui::Id::new(("landing-recent-project-card", index)),
            egui::Sense::click(),
        );
        painter.rect_filled(card, 6.0, egui::Color32::from_rgb(16, 25, 28));
        painter.rect_stroke(
            card,
            6.0,
            egui::Stroke::new(
                if response.hovered() {
                    1.5_f32 * density
                } else {
                    1.0_f32 * density
                },
                if response.hovered() {
                    colors::ACCENT_BRAND
                } else {
                    colors::BORDER_SUBTLE
                },
            ),
        );

        let cover = egui::Rect::from_min_size(card.min, egui::vec2(card_width, cover_height));
        let preview = project
            .summary
            .as_ref()
            .and_then(|summary| summary.preview_path.as_deref())
            .map(std::path::PathBuf::from)
            .map(|path| {
                if path.is_absolute() {
                    path
                } else {
                    project
                        .path
                        .parent()
                        .unwrap_or_else(|| std::path::Path::new("."))
                        .join(path)
                }
            })
            .filter(|path| path.is_file())
            .and_then(|path| thumb_for(app, ui.ctx(), &path));
        let fallback_name = [
            "recent_project_eclipse.webp",
            "recent_project_citadel.webp",
            "recent_project_rift.webp",
        ][index % 3];
        let fallback = if preview.is_none() {
            let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("assets/home")
                .join(fallback_name);
            thumb_for(app, ui.ctx(), &path)
        } else {
            None
        };
        if let Some(texture) = preview.or(fallback) {
            painter.image(
                texture,
                cover,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        } else {
            draw_project_cover(&painter, cover, index, &project.name);
        }

        let name_rect = egui::Rect::from_min_max(
            egui::pos2(card.left() + 14.0 * density, cover.bottom() + 4.0 * density),
            egui::pos2(
                card.right() - 42.0 * density,
                cover.bottom() + 29.0 * density,
            ),
        );
        painter.with_clip_rect(name_rect).text(
            name_rect.left_center(),
            egui::Align2::LEFT_CENTER,
            &project.name,
            egui::FontId::proportional(13.0 * density),
            colors::TEXT_PRIMARY,
        );
        let path_rect = egui::Rect::from_min_max(
            egui::pos2(
                card.left() + 14.0 * density,
                cover.bottom() + 30.0 * density,
            ),
            egui::pos2(
                card.right() - 12.0 * density,
                cover.bottom() + 49.0 * density,
            ),
        );
        painter.with_clip_rect(path_rect).text(
            path_rect.left_center(),
            egui::Align2::LEFT_CENTER,
            recent_project_path_label(&project.path),
            egui::FontId::proportional(9.5 * density),
            colors::TEXT_SECONDARY,
        );
        crate::ui::icons::render_svg_at(
            ui,
            format!("landing-card-folder-{index}"),
            crate::ui::icons::SVG_FOLDER,
            egui::vec2(12.0 * density, 12.0 * density),
            colors::TEXT_SECONDARY_BRIGHT,
            egui::pos2(
                card.left() + 13.0 * density,
                cover.bottom() + 61.0 * density,
            ),
        );
        let metadata = project
            .summary
            .as_ref()
            .map(|summary| {
                format!(
                    "{} × {}  |  {}レイヤー",
                    summary.width, summary.height, summary.layers
                )
            })
            .unwrap_or_else(|| "Kagari VFX プロジェクト".to_string());
        let metadata_rect = egui::Rect::from_min_max(
            egui::pos2(
                card.left() + 31.0 * density,
                cover.bottom() + 54.0 * density,
            ),
            egui::pos2(
                card.right() - 82.0 * density,
                cover.bottom() + 78.0 * density,
            ),
        );
        painter.with_clip_rect(metadata_rect).text(
            metadata_rect.left_center(),
            egui::Align2::LEFT_CENTER,
            metadata,
            egui::FontId::proportional(9.5 * density),
            colors::TEXT_SECONDARY,
        );
        painter.text(
            egui::pos2(
                card.right() - 12.0 * density,
                cover.bottom() + 67.0 * density,
            ),
            egui::Align2::RIGHT_CENTER,
            project
                .modified
                .map(fmt_timestamp)
                .unwrap_or_else(|| "—".into()),
            egui::FontId::proportional(9.0 * density),
            colors::TEXT_SECONDARY,
        );
        let menu_rect = egui::Rect::from_min_size(
            egui::pos2(
                card.right() - 33.0 * density,
                cover.bottom() + 3.0 * density,
            ),
            egui::vec2(24.0 * density, 22.0 * density),
        );
        let menu_response = ui.interact(
            menu_rect,
            egui::Id::new(("landing-project-menu-button", index)),
            egui::Sense::click(),
        );
        painter.text(
            menu_rect.center(),
            egui::Align2::CENTER_CENTER,
            "···",
            egui::FontId::proportional(13.0 * density),
            if menu_response.hovered() {
                colors::TEXT_PRIMARY
            } else {
                colors::TEXT_SECONDARY
            },
        );

        let menu_state_id = egui::Id::new("landing-project-menu-path");
        if menu_response.clicked() {
            let is_open = ui.ctx().data(|data| {
                data.get_temp::<std::path::PathBuf>(menu_state_id)
                    .is_some_and(|path| path == project.path)
            });
            ui.ctx().data_mut(|data| {
                if is_open {
                    data.remove::<std::path::PathBuf>(menu_state_id);
                } else {
                    data.insert_temp(menu_state_id, project.path.clone());
                }
            });
        }
        let menu_is_open = ui.ctx().data(|data| {
            data.get_temp::<std::path::PathBuf>(menu_state_id)
                .is_some_and(|path| path == project.path)
        });
        let mut menu_action = None;
        if menu_is_open {
            egui::Area::new(egui::Id::new(("landing-project-menu-popup", index)))
                .order(egui::Order::Foreground)
                .fixed_pos(menu_rect.left_bottom() + egui::vec2(0.0, 4.0 * density))
                .show(ui.ctx(), |popup_ui| {
                    egui::Frame::popup(popup_ui.style()).show(popup_ui, |popup_ui| {
                        popup_ui.set_min_width(190.0 * density);
                        if popup_ui.button("開く").clicked() {
                            menu_action = Some(false);
                        }
                        if popup_ui.button("最近の一覧から削除").clicked() {
                            menu_action = Some(true);
                        }
                    });
                });
        }
        if let Some(remove) = menu_action {
            ui.ctx()
                .data_mut(|data| data.remove::<std::path::PathBuf>(menu_state_id));
            if remove {
                remove_path = Some(project.path.clone());
            } else {
                open_path = Some(project.path.clone());
            }
        }
        let menu_clicked = menu_response.clicked() || menu_action.is_some();
        if response.clicked() && !menu_clicked {
            app.home_selected = Some(project.path.clone());
            open_path = Some(project.path.clone());
        }
    }
    if let Some(path) = remove_path {
        match crate::ui::project_io::remove_recent(&path) {
            Ok(()) => app
                .home_recent_projects
                .retain(|project| project.path != path),
            Err(error) => app.toasts.error(error),
        }
    }
    if let Some(path) = open_path {
        app.home_selected = Some(path.clone());
        open_project_path(app, &path);
    }
}

fn fmt_timestamp(time: std::time::SystemTime) -> String {
    #[cfg(unix)]
    {
        let Ok(duration) = time.duration_since(std::time::UNIX_EPOCH) else {
            return fmt_age(time);
        };
        let timestamp = duration.as_secs() as libc::time_t;
        let mut local: libc::tm = unsafe { std::mem::zeroed() };
        if unsafe { libc::localtime_r(&timestamp, &mut local) }.is_null() {
            return fmt_age(time);
        }
        return format!(
            "{:04}/{:02}/{:02} {:02}:{:02}",
            local.tm_year + 1900,
            local.tm_mon + 1,
            local.tm_mday,
            local.tm_hour,
            local.tm_min,
        );
    }
    #[cfg(not(unix))]
    fmt_age(time)
}

fn draw_project_cover(painter: &egui::Painter, rect: egui::Rect, index: usize, name: &str) {
    let painter = painter.with_clip_rect(rect);
    let seed = name.bytes().fold(0_u32, |hash, byte| {
        hash.wrapping_mul(33).wrapping_add(u32::from(byte))
    });
    let scale = (painter.ctx().screen_rect().width() / 1584.0).max(1.0);
    let palettes = [
        (
            egui::Color32::from_rgb(13, 22, 25),
            egui::Color32::from_rgb(255, 79, 22),
        ),
        (
            egui::Color32::from_rgb(18, 23, 25),
            egui::Color32::from_rgb(159, 86, 60),
        ),
        (
            egui::Color32::from_rgb(24, 23, 22),
            egui::Color32::from_rgb(166, 125, 96),
        ),
        (
            egui::Color32::from_rgb(16, 23, 27),
            egui::Color32::from_rgb(100, 119, 132),
        ),
    ];
    let (base, accent) = palettes[index % palettes.len()];
    painter.rect_filled(rect, 6.0, base);
    for wave in 0..2 {
        let mut points = Vec::new();
        for step in 0..=32 {
            let t = step as f32 / 32.0;
            let phase = ((seed >> (wave * 5)) & 31) as f32 * 0.025;
            let x = rect.left() + t * rect.width();
            let y = rect.top() + rect.height() * (0.64 - wave as f32 * 0.09)
                - (t * 3.6 + phase).sin().abs() * rect.height() * (0.3 + wave as f32 * 0.08);
            points.push(egui::pos2(x, y));
        }
        painter.add(egui::Shape::line(
            points,
            egui::Stroke::new(
                if wave == 0 { 2.0_f32 } else { 1.0_f32 } * scale,
                accent.linear_multiply(if wave == 0 { 0.35 } else { 0.16 }),
            ),
        ));
    }
}
