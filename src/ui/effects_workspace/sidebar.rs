use crate::ui::icons;
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

use super::constants::*;

pub(crate) fn draw_sidebar(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    width: f32,
    compact: bool,
    _app: &mut KagariApp,
) {
    let side = egui::Rect::from_min_max(rect.min, egui::pos2(rect.left() + width, rect.bottom()));
    {
        let p = ui.painter();
        p.rect_filled(side, 0.0, egui::Color32::from_rgb(13, 23, 30));
        p.line_segment(
            [
                egui::pos2(side.right(), side.top()),
                egui::pos2(side.right(), side.bottom()),
            ],
            egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
        );
    }
    icons::render_svg_at(
        ui,
        "effects-heading".to_string(),
        icons::SVG_EFFECTS,
        egui::vec2(30.0, 30.0),
        colors::TEXT_PRIMARY,
        egui::pos2(side.left() + 20.0, side.top() + 29.0),
    );
    for (i, icon) in [icons::SVG_LAYERS, icons::SVG_STAR, icons::SVG_CLOCK]
        .into_iter()
        .enumerate()
    {
        icons::render_svg_at(
            ui,
            format!("effects-nav-{i}"),
            icon,
            egui::vec2(22.0, 22.0),
            colors::TEXT_PRIMARY,
            egui::pos2(side.left() + 25.0, side.top() + 91.0 + i as f32 * 42.0),
        );
    }
    for i in 0..10 {
        let icon = if i == 7 {
            icons::SVG_LIGHT
        } else {
            icons::SVG_FOLDER
        };
        icons::render_svg_at(
            ui,
            format!("effects-category-{i}"),
            icon,
            egui::vec2(20.0, 20.0),
            if i == 7 { ORANGE } else { colors::TEXT_PRIMARY },
            egui::pos2(side.left() + 25.0, side.top() + 265.0 + i as f32 * 34.0),
        );
    }
    let p = ui.painter().clone();
    p.text(
        egui::pos2(side.left() + 84.0, side.top() + 44.0),
        egui::Align2::LEFT_CENTER,
        "エフェクト",
        egui::FontId::proportional(if compact { 20.0 } else { 23.0 }),
        colors::TEXT_PRIMARY,
    );
    let top = side.top() + 82.0;
    for (i, label) in ["すべて", "お気に入り", "最近使用"].into_iter().enumerate() {
        let y = top + i as f32 * 42.0;
        let active = i == 0;
        if active {
            p.rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(side.left() + 10.0, y),
                    egui::vec2(width - 20.0, 40.0),
                ),
                6.0,
                crate::ui::theme::colors::BG_PANEL,
            );
            p.rect_filled(
                egui::Rect::from_min_size(egui::pos2(side.left() + 10.0, y), egui::vec2(4.0, 40.0)),
                2.0,
                ORANGE,
            );
        }
        let label_rect = egui::Rect::from_min_max(
            egui::pos2(side.left() + 78.0, y + 2.0),
            egui::pos2(side.right() - 18.0, y + 38.0),
        );
        ui.put(
            label_rect,
            egui::Label::new(
                egui::RichText::new(label)
                    .size(14.0)
                    .color(colors::TEXT_PRIMARY),
            )
            .truncate(),
        );
    }
    p.line_segment(
        [
            egui::pos2(side.left() + 20.0, top + 145.0),
            egui::pos2(side.right() - 20.0, top + 145.0),
        ],
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
    let video_rows = [
        "ぼかし",
        "カラー補正",
        "キーイング",
        "ノイズ＆グレイン",
        "スタイライズ",
        "ディストーション",
        "生成",
        "発光",
        "時間",
        "ユーティリティ",
    ];
    let mut y = top + 177.0;
    for (i, label) in video_rows.into_iter().enumerate() {
        if i == 0 {
            p.text(
                egui::pos2(side.left() + 20.0, y),
                egui::Align2::LEFT_CENTER,
                "ビデオエフェクト",
                egui::FontId::proportional(14.0),
                colors::TEXT_PRIMARY,
            );
            p.text(
                egui::pos2(side.right() - 22.0, y),
                egui::Align2::RIGHT_CENTER,
                "⌃",
                egui::FontId::proportional(13.0),
                colors::TEXT_SECONDARY,
            );
            y += 28.0;
        }
        let active = i == 7;
        if active {
            p.rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(side.left() + 10.0, y - 18.0),
                    egui::vec2(width - 20.0, 43.0),
                ),
                5.0,
                egui::Color32::from_rgb(50, 38, 31),
            );
            p.rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(side.left() + 10.0, y - 18.0),
                    egui::vec2(4.0, 43.0),
                ),
                2.0,
                ORANGE,
            );
        }
        let label_rect = egui::Rect::from_min_max(
            egui::pos2(side.left() + 76.0, y - 16.0),
            egui::pos2(side.right() - 45.0, y + 16.0),
        );
        ui.put(
            label_rect,
            egui::Label::new(egui::RichText::new(label).size(13.0).color(if active {
                ORANGE
            } else {
                colors::TEXT_PRIMARY
            }))
            .truncate(),
        );
        p.text(
            egui::pos2(side.right() - 24.0, y),
            egui::Align2::RIGHT_CENTER,
            format!("{}", [12, 18, 10, 9, 12, 11, 16, 14, 9, 10][i]),
            egui::FontId::proportional(11.0),
            colors::TEXT_SECONDARY,
        );
        y += 34.0;
    }
    p.line_segment(
        [
            egui::pos2(side.left() + 20.0, y - 9.0),
            egui::pos2(side.right() - 20.0, y - 9.0),
        ],
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
    y += 22.0;
    p.text(
        egui::pos2(side.left() + 20.0, y),
        egui::Align2::LEFT_CENTER,
        "プリセット",
        egui::FontId::proportional(14.0),
        colors::TEXT_PRIMARY,
    );
    p.text(
        egui::pos2(side.right() - 22.0, y),
        egui::Align2::RIGHT_CENTER,
        "⌃",
        egui::FontId::proportional(13.0),
        colors::TEXT_SECONDARY,
    );
    y += 28.0;
    for (i, label) in ["ユーザープリセット", "内蔵プリセット"]
        .into_iter()
        .enumerate()
    {
        let label_rect = egui::Rect::from_min_max(
            egui::pos2(side.left() + 76.0, y - 16.0),
            egui::pos2(side.right() - 45.0, y + 16.0),
        );
        ui.put(
            label_rect,
            egui::Label::new(
                egui::RichText::new(label)
                    .size(13.0)
                    .color(colors::TEXT_PRIMARY),
            )
            .truncate(),
        );
        p.text(
            egui::pos2(side.right() - 24.0, y),
            egui::Align2::RIGHT_CENTER,
            if i == 0 { "5" } else { "24" },
            egui::FontId::proportional(11.0),
            colors::TEXT_SECONDARY,
        );
        y += 34.0;
    }
}

