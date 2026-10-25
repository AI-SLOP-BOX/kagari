use crate::ui::icons;
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

use super::constants::*;
use super::helpers::*;

pub(crate) fn draw_center(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    rect: egui::Rect,
    compact: bool,
    mobile: bool,
    app: &mut KagariApp,
) {
    let pad = if mobile {
        16.0
    } else if compact {
        18.0
    } else {
        30.0
    };
    let p = ui.painter().clone();
    p.text(
        egui::pos2(rect.left() + pad, rect.top() + 43.0),
        egui::Align2::LEFT_CENTER,
        "エフェクト",
        egui::FontId::proportional(if mobile { 28.0 } else { 36.0 }),
        colors::TEXT_PRIMARY,
    );
    p.text(
        egui::pos2(rect.left() + pad, rect.top() + 76.0),
        egui::Align2::LEFT_CENTER,
        "映像表現を広げる、豊富なエフェクトライブラリ",
        egui::FontId::proportional(15.0),
        colors::TEXT_SECONDARY,
    );

    let list_view = ctx.data_mut(|d| {
        d.get_temp::<bool>(egui::Id::new("effects-list-view"))
            .unwrap_or(false)
    });
    let toolbar_top = rect.top() + 105.0;
    let toolbar_h = 42.0;
    let toggle_w = if mobile { 0.0 } else { 92.0 };
    let sort_w = if mobile { 0.0 } else { 125.0 };
    let gaps = if mobile { 0.0 } else { 28.0 };
    let search_w = (rect.width() - pad * 2.0 - sort_w - toggle_w - gaps).max(130.0);
    let search = egui::Rect::from_min_size(
        egui::pos2(rect.left() + pad, toolbar_top),
        egui::vec2(search_w, toolbar_h),
    );
    p.rect(
        search,
        6.0,
        crate::ui::theme::colors::BG_PANEL_ALT,
        egui::Stroke::new(1.0_f32, colors::BORDER_MEDIUM),
    );
    p.text(
        egui::pos2(search.left() + 42.0, search.center().y),
        egui::Align2::LEFT_CENTER,
        "エフェクトを検索...",
        egui::FontId::proportional(14.0),
        colors::TEXT_SECONDARY,
    );
    icons::render_svg_at(
        ui,
        "effects-search".to_string(),
        icons::SVG_SEARCH,
        egui::vec2(20.0, 20.0),
        colors::TEXT_SECONDARY,
        egui::pos2(search.left() + 12.0, search.top() + 11.0),
    );
    if !mobile {
        let sort_rect = egui::Rect::from_min_size(
            egui::pos2(search.right() + 14.0, toolbar_top),
            egui::vec2(sort_w, toolbar_h),
        );
        p.rect(
            sort_rect,
            6.0,
            crate::ui::theme::colors::BG_PANEL_ALT,
            egui::Stroke::new(1.0_f32, colors::BORDER_MEDIUM),
        );
        p.text(
            sort_rect.center(),
            egui::Align2::CENTER_CENTER,
            "人気順",
            egui::FontId::proportional(13.0),
            colors::TEXT_PRIMARY,
        );
        icons::render_svg_at(
            ui,
            "effects-sort-chevron".to_string(),
            icons::SVG_CHEVRON_DOWN,
            egui::vec2(16.0, 16.0),
            colors::TEXT_PRIMARY,
            egui::pos2(sort_rect.right() - 24.0, sort_rect.top() + 13.0),
        );
        let view_rect = egui::Rect::from_min_size(
            egui::pos2(sort_rect.right() + 14.0, toolbar_top),
            egui::vec2(toggle_w, toolbar_h),
        );
        p.rect(
            view_rect,
            6.0,
            crate::ui::theme::colors::BG_PANEL_ALT,
            egui::Stroke::new(1.0_f32, colors::BORDER_MEDIUM),
        );
        let grid_rect = egui::Rect::from_min_size(view_rect.min, egui::vec2(46.0, toolbar_h));
        let list_rect = egui::Rect::from_min_size(
            egui::pos2(view_rect.left() + 46.0, view_rect.top()),
            egui::vec2(46.0, toolbar_h),
        );
        p.rect_filled(
            if list_view { list_rect } else { grid_rect },
            5.0,
            egui::Color32::from_rgb(31, 47, 59),
        );
        if ui
            .interact(
                grid_rect,
                egui::Id::new("effects-grid-view"),
                egui::Sense::click(),
            )
            .clicked()
        {
            ctx.data_mut(|d| d.insert_temp(egui::Id::new("effects-list-view"), false));
        }
        if ui
            .interact(
                list_rect,
                egui::Id::new("effects-list-view"),
                egui::Sense::click(),
            )
            .clicked()
        {
            ctx.data_mut(|d| d.insert_temp(egui::Id::new("effects-list-view"), true));
        }
        icons::render_svg_at(
            ui,
            "effects-grid-icon".to_string(),
            icons::SVG_GRID,
            egui::vec2(18.0, 18.0),
            colors::TEXT_PRIMARY,
            egui::pos2(grid_rect.left() + 14.0, grid_rect.top() + 12.0),
        );
        icons::render_svg_at(
            ui,
            "effects-list-icon".to_string(),
            icons::SVG_SORT,
            egui::vec2(18.0, 18.0),
            colors::TEXT_PRIMARY,
            egui::pos2(list_rect.left() + 14.0, list_rect.top() + 12.0),
        );
    }

    let effects = [
        ("アナログフィルム", "質感", ""),
        ("ソフトブルーム", "発光", ""),
        ("プリズムディストーション", "レンズ", ""),
        ("Gaussian Blur", "Blur", "assets/studio/assets_smoke.webp"),
        ("Glow", "Light", "assets/studio/assets_light_leak.webp"),
        (
            "Film Grain",
            "Texture",
            "assets/studio/assets_glass_texture.webp",
        ),
        (
            "Color Balance",
            "Color",
            "assets/studio/assets_particles.webp",
        ),
        ("Vignette", "Blur", "assets/studio/assets_mountain.webp"),
        (
            "Lens Flare",
            "Light",
            "assets/studio/assets_light_leak.webp",
        ),
        (
            "Chromatic Aberration",
            "Distort",
            "assets/studio/assets_city.webp",
        ),
        (
            "Light Leak",
            "Light",
            "assets/studio/assets_light_leak.webp",
        ),
        ("Halftone", "Stylize", "assets/studio/assets_floor_ref.webp"),
    ];
    let selected_index = ctx.data_mut(|d| {
        d.get_temp::<usize>(egui::Id::new("effect-selected"))
            .unwrap_or(1)
    });
    if list_view || mobile {
        let cols = if mobile { 1 } else { 2 };
        let gap = if compact { 10.0 } else { 16.0 };
        let row_h = if mobile { 54.0 } else { 58.0 };
        let row_w =
            ((rect.width() - pad * 2.0 - gap * (cols as f32 - 1.0)) / cols as f32).max(150.0);
        let list_top = rect.top() + if mobile { 154.0 } else { 170.0 };
        for (i, (name, category, asset)) in effects.into_iter().enumerate() {
            let x = rect.left() + pad + (i % cols) as f32 * (row_w + gap);
            let y = list_top + (i / cols) as f32 * (row_h + 4.0);
            let row = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(row_w, row_h));
            let response = ui.interact(row, egui::Id::new(("effect-row", i)), egui::Sense::click());
            let selected = i == selected_index;
            if selected || response.hovered() {
                p.rect_filled(
                    row,
                    4.0,
                    egui::Color32::from_rgba_premultiplied(34, 50, 62, 210),
                );
            }
            if selected {
                p.rect_filled(
                    egui::Rect::from_min_size(row.min, egui::vec2(3.0, row.height())),
                    1.5,
                    ORANGE,
                );
            }
            let preview = egui::Rect::from_min_size(
                egui::pos2(x + 12.0, y + 8.0),
                egui::vec2(68.0, row_h - 16.0),
            );
            if !asset.is_empty() {
                if let Some(id) = texture(ctx, asset, &format!("effect-row-{i}")) {
                    p.image(
                        id,
                        preview,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
            } else {
                draw_effect_preview(&p, preview, name);
            }
            p.text(
                egui::pos2(x + 92.0, y + 21.0),
                egui::Align2::LEFT_CENTER,
                name,
                egui::FontId::proportional(if compact { 13.0 } else { 14.0 }),
                colors::TEXT_PRIMARY,
            );
            p.text(
                egui::pos2(x + 92.0, y + 41.0),
                egui::Align2::LEFT_CENTER,
                category,
                egui::FontId::proportional(11.0),
                colors::TEXT_MUTED,
            );
            p.line_segment(
                [
                    egui::pos2(x + 92.0, row.bottom()),
                    egui::pos2(row.right(), row.bottom()),
                ],
                egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
            );
            if response.clicked() {
                ctx.data_mut(|d| d.insert_temp(egui::Id::new("effect-selected"), i));
                apply_effect(app, name);
            }
        }
    } else {
        let cols = 3;
        let gap = 18.0;
        let card_w =
            ((rect.width() - pad * 2.0 - gap * (cols as f32 - 1.0)) / cols as f32).max(150.0);
        let card_h = if compact { 120.0 } else { 185.0 };
        let thumb_h = if compact { 90.0 } else { 155.0 };
        let row_step = if compact { 132.0 } else { 205.0 };
        let grid_top = rect.top() + if compact { 170.0 } else { 194.0 };
        for (i, (name, _category, asset)) in effects.into_iter().enumerate() {
            let x = rect.left() + pad + (i % cols) as f32 * (card_w + gap);
            let y = grid_top + (i / cols) as f32 * row_step;
            let card = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(card_w, card_h));
            let response = ui.interact(
                card,
                egui::Id::new(("effect-card", i)),
                egui::Sense::click(),
            );
            let selected = i == selected_index;
            if response.hovered() && !selected {
                p.rect_filled(
                    card,
                    6.0,
                    egui::Color32::from_rgba_premultiplied(24, 38, 48, 150),
                );
            }
            let thumb = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(card_w, thumb_h));
            if !asset.is_empty() {
                if let Some(id) = texture(ctx, asset, &format!("effect-card-{i}")) {
                    p.image(
                        id,
                        thumb,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
            } else {
                draw_effect_preview(&p, thumb, name);
            }
            p.rect_stroke(
                thumb,
                5.0,
                egui::Stroke::new(
                    if selected { 2.0_f32 } else { 1.0_f32 },
                    if selected {
                        ORANGE
                    } else {
                        colors::BORDER_SUBTLE
                    },
                ),
            );
            p.text(
                egui::pos2(x, y + if compact { 109.0 } else { 174.0 }),
                egui::Align2::LEFT_CENTER,
                name,
                egui::FontId::proportional(if compact { 13.0 } else { 16.0 }),
                colors::TEXT_PRIMARY,
            );
            if response.clicked() {
                ctx.data_mut(|d| d.insert_temp(egui::Id::new("effect-selected"), i));
                apply_effect(app, name);
            }
        }
    }
}

fn draw_effect_preview(painter: &egui::Painter, rect: egui::Rect, name: &str) {
    painter.rect_filled(rect, 5.0, egui::Color32::from_rgb(18, 24, 30));
    match name {
        "アナログフィルム" => {
            for i in 0..72 {
                let x = ((i * 37 + 13) % 101) as f32 / 100.0;
                let y = ((i * 61 + 29) % 97) as f32 / 96.0;
                let point = egui::pos2(
                    rect.left() + 3.0 + x * (rect.width() - 6.0),
                    rect.top() + 3.0 + y * (rect.height() - 6.0),
                );
                let alpha = 45 + ((i * 19) % 100) as u8;
                painter.circle_filled(
                    point,
                    if i % 9 == 0 { 1.2 } else { 0.65 },
                    egui::Color32::from_white_alpha(alpha),
                );
            }
            painter.rect_stroke(
                rect.shrink(5.0),
                6.0,
                egui::Stroke::new(1.0_f32, egui::Color32::from_white_alpha(35)),
            );
        }
        "ソフトブルーム" => {
            let center = rect.center();
            let radius = rect.height().min(rect.width()) * 0.34;
            for (scale, alpha) in [(2.0, 10), (1.5, 18), (1.0, 35)] {
                painter.circle_filled(
                    center,
                    radius * scale,
                    egui::Color32::from_white_alpha(alpha),
                );
            }
            painter.circle_filled(
                center,
                radius * 0.28,
                egui::Color32::from_rgb(245, 246, 238),
            );
        }
        "プリズムディストーション" => {
            for row in 0..7 {
                let base_y = rect.top() + 5.0 + row as f32 * (rect.height() - 10.0) / 6.0;
                let mut points = Vec::new();
                for step in 0..=16 {
                    let t = step as f32 / 16.0;
                    let x = rect.left() + 4.0 + t * (rect.width() - 8.0);
                    let wave = ((t * 8.0 + row as f32 * 0.7).sin()) * (2.0 + row as f32 * 0.12);
                    points.push(egui::pos2(x, base_y + wave));
                }
                let color = match row % 3 {
                    0 => egui::Color32::from_rgb(240, 83, 94),
                    1 => egui::Color32::from_rgb(230, 236, 242),
                    _ => egui::Color32::from_rgb(65, 180, 220),
                };
                painter.add(egui::Shape::line(points, egui::Stroke::new(1.0_f32, color)));
            }
        }
        _ => {}
    }
}
