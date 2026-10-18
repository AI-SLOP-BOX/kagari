use super::*;

pub(super) fn draw_landing_recent_work(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    left: f32,
    right: f32,
    top: f32,
    projects: &[RecentProject],
) {
    let scale = ui
        .ctx()
        .native_pixels_per_point()
        .unwrap_or(1.0)
        .recip()
        .clamp(0.5, 1.0);
    let painter = ui.painter().clone();
    let entries = projects
        .iter()
        .filter(|project| project.path.is_file())
        .take(5);
    let mut open_path = None;
    let row_height = 44.0 * scale;
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
            painter.rect_filled(row, 3.0, colors::BG_HOVER);
        }
        painter.line_segment(
            [row.left_bottom(), row.right_bottom()],
            egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
        );
        crate::ui::icons::render_svg_at(
            ui,
            format!("landing-work-icon-{index}"),
            crate::ui::icons::SVG_COMPOSITION,
            egui::vec2(20.0 * scale, 20.0 * scale),
            colors::TEXT_SECONDARY_BRIGHT,
            egui::pos2(left + 10.0 * scale, row.center().y - 10.0 * scale),
        );
        let composition = project
            .summary
            .as_ref()
            .and_then(|summary| {
                summary.comp_names.get(
                    summary
                        .active_idx
                        .min(summary.comp_names.len().saturating_sub(1)),
                )
            })
            .map(String::as_str)
            .unwrap_or("プロジェクト");
        let compact = right - left < 680.0 * scale;
        if compact {
            let name_rect = egui::Rect::from_min_max(
                egui::pos2(left + 42.0 * scale, row.top()),
                egui::pos2(right - 94.0 * scale, row.bottom()),
            );
            let label = format!("{composition} · {}", project.name);
            ui.put(
                name_rect,
                egui::Label::new(
                    egui::RichText::new(label)
                        .size(13.0 * scale)
                        .color(colors::TEXT_PRIMARY),
                )
                .truncate()
                .sense(egui::Sense {
                    click: false,
                    drag: false,
                    focusable: false,
                }),
            );
        } else {
            let project_rect = egui::Rect::from_min_max(
                egui::pos2(left + 68.0 * scale, row.top()),
                egui::pos2(left + (right - left) * 0.43, row.bottom()),
            );
            let comp_rect = egui::Rect::from_min_max(
                egui::pos2(left + (right - left) * 0.43, row.top()),
                egui::pos2(right - 132.0 * scale, row.bottom()),
            );
            ui.put(
                project_rect,
                egui::Label::new(
                    egui::RichText::new(composition)
                        .size(14.0 * scale)
                        .color(colors::TEXT_PRIMARY),
                )
                .truncate()
                .sense(egui::Sense {
                    click: false,
                    drag: false,
                    focusable: false,
                }),
            );
            ui.put(
                comp_rect,
                egui::Label::new(
                    egui::RichText::new(&project.name)
                        .size(14.0 * scale)
                        .color(colors::TEXT_SECONDARY),
                )
                .truncate()
                .sense(egui::Sense {
                    click: false,
                    drag: false,
                    focusable: false,
                }),
            );
        }
        painter.text(
            egui::pos2(right - 66.0 * scale, row.center().y),
            egui::Align2::CENTER_CENTER,
            project.modified.map(fmt_age).unwrap_or_else(|| "—".into()),
            egui::FontId::proportional(12.0 * scale),
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
            empty.left_center() + egui::vec2(14.0 * scale, 0.0),
            egui::Align2::LEFT_CENTER,
            "最近開いたプロジェクトはここに表示されます",
            egui::FontId::proportional(13.0 * scale),
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
) {
    let scale = ui
        .ctx()
        .native_pixels_per_point()
        .unwrap_or(1.0)
        .recip()
        .clamp(0.5, 1.0);
    let density = scale * (rect.width() / (1217.0 * scale)).max(0.68);
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

    let gap = LANDING_CARD_GAP * density;
    let minimum_card_width = 274.0 * density;
    let mut columns = ((rect.width() + gap) / (minimum_card_width + gap))
        .floor()
        .clamp(1.0, 4.0) as usize;
    if entries.len().min(4) == 4 && columns == 3 {
        columns = 2;
    }
    let card_width =
        ((rect.width() - gap * (columns - 1) as f32) / columns as f32).max(150.0 * density);
    let card_height = match columns {
        4 => 216.0 * density,
        3 => 200.0 * density,
        2 => 184.0 * density,
        _ => 164.0 * density,
    };
    let row_gap = 16.0 * density;
    let cover_height = (card_height * 0.54).clamp(96.0 * density, 120.0 * density);

    for (index, project) in entries.iter().take(4).enumerate() {
        let column = index % columns;
        let row = index / columns;
        let x = rect.left() + column as f32 * (card_width + gap);
        let y = rect.top() + row as f32 * (card_height + row_gap);
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
        if let Some(texture) = preview {
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
            egui::pos2(card.left() + 14.0 * density, cover.bottom() + 7.0 * density),
            egui::pos2(
                card.right() - 18.0 * density,
                cover.bottom() + 31.0 * density,
            ),
        );
        painter.with_clip_rect(name_rect).text(
            name_rect.left_center(),
            egui::Align2::LEFT_CENTER,
            &project.name,
            egui::FontId::proportional(14.0 * density),
            colors::TEXT_PRIMARY,
        );
        let summary = project.summary.as_ref();
        let metadata = summary
            .map(|summary| {
                format!(
                    "{}コンポジション・{}レイヤー",
                    summary.comp_names.len(),
                    summary.layers
                )
            })
            .unwrap_or_else(|| "Kagari プロジェクト".to_string());
        painter.text(
            egui::pos2(
                card.left() + 14.0 * density,
                cover.bottom() + 48.0 * density,
            ),
            egui::Align2::LEFT_CENTER,
            metadata,
            egui::FontId::proportional(11.0 * density),
            colors::TEXT_SECONDARY,
        );
        painter.text(
            egui::pos2(
                card.left() + 14.0 * density,
                cover.bottom() + 72.0 * density,
            ),
            egui::Align2::LEFT_CENTER,
            project.modified.map(fmt_age).unwrap_or_else(|| "—".into()),
            egui::FontId::proportional(11.0 * density),
            colors::TEXT_MUTED,
        );
        if response.clicked() {
            app.home_selected = Some(project.path.clone());
            open_project_path(app, &project.path);
        }
    }
}

fn draw_project_cover(painter: &egui::Painter, rect: egui::Rect, index: usize, name: &str) {
    let painter = painter.with_clip_rect(rect);
    let seed = name.bytes().fold(0_u32, |hash, byte| {
        hash.wrapping_mul(33).wrapping_add(u32::from(byte))
    });
    let scale = painter
        .ctx()
        .native_pixels_per_point()
        .unwrap_or(1.0)
        .recip()
        .clamp(0.5, 1.0);
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
