//! Title presets browser and live composition text preview with transport controls.

use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

use super::inspector::selected_text;
use super::nav::texture;

pub fn draw_presets(ui: &mut egui::Ui, ctx: &egui::Context, rect: egui::Rect) {
    let compact = rect.width() < 250.0;
    ui.painter().text(
        egui::pos2(rect.left() + 17.0, rect.top() + 28.0),
        egui::Align2::LEFT_CENTER,
        "タイトルプリセット",
        egui::FontId::proportional(if compact { 16.0 } else { 18.0 }),
        colors::TEXT_PRIMARY,
    );
    let search = egui::Rect::from_min_size(
        egui::pos2(
            rect.left() + 17.0,
            rect.top() + if compact { 48.0 } else { 51.0 },
        ),
        egui::vec2(rect.width() - 30.0, if compact { 32.0 } else { 36.0 }),
    );
    ui.painter().rect(
        search,
        5.0,
        crate::ui::theme::colors::BG_PANEL_ALT,
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
    ui.painter().text(
        egui::pos2(search.left() + 12.0, search.center().y),
        egui::Align2::LEFT_CENTER,
        "⌕  プリセットを検索...",
        egui::FontId::proportional(if compact { 11.0 } else { 12.0 }),
        colors::TEXT_SECONDARY,
    );
    for (i, label) in [
        "すべて",
        "シネマティック",
        "モダン",
        "ミニマル",
        "テクノロジー",
        "和風",
        "VFX / グロー",
        "字幕（テロップ）",
        "ソーシャルメディア",
        "カスタム",
    ]
    .into_iter()
    .enumerate()
    {
        let y = rect.top()
            + if compact { 82.0 } else { 92.0 }
            + i as f32 * if compact { 21.0 } else { 28.0 };
        if i == 0 {
            ui.painter().rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(rect.left() + 10.0, y - if compact { 11.0 } else { 14.0 }),
                    egui::vec2(rect.width() - 20.0, if compact { 28.0 } else { 34.0 }),
                ),
                4.0,
                crate::ui::theme::colors::BG_PANEL_DARK,
            );
            ui.painter().rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(rect.left() + 10.0, y - if compact { 11.0 } else { 14.0 }),
                    egui::vec2(4.0, if compact { 28.0 } else { 34.0 }),
                ),
                1.0,
                crate::ui::theme::colors::ACCENT_BRAND,
            );
        }
        ui.painter().text(
            egui::pos2(rect.left() + if compact { 30.0 } else { 38.0 }, y),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(if compact { 10.0 } else { 12.0 }),
            if i == 0 {
                colors::TEXT_PRIMARY
            } else {
                colors::TEXT_SECONDARY
            },
        );
    }
    let cards = [
        ("recent_project_eclipse.webp", "シネマティック 01"),
        ("recent_project_atlas.webp", "モダン 01"),
        ("recent_project_rift.webp", "和風 01"),
        ("recent_project_citadel.webp", "テクノロジー 01"),
        ("recent_project_atlas.webp", "ミニマル 01"),
        ("recent_project_eclipse.webp", "グロー 01"),
    ];
    let cw = (rect.width() - 47.0) * 0.5;
    for (i, (asset, label)) in cards.into_iter().enumerate() {
        let x = rect.left() + 17.0 + (i % 2) as f32 * (cw + 14.0);
        let y = rect.top()
            + if compact { 305.0 } else { 378.0 }
            + (i / 2) as f32 * if compact { 77.0 } else { 106.0 };
        let card_h = if compact { 64.0 } else { 83.0 };
        let card = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(cw, card_h));
        ui.painter().rect(
            card,
            6.0,
            crate::ui::theme::colors::BG_PANEL_BASE,
            egui::Stroke::new(
                1.0_f32,
                if i == 0 {
                    crate::ui::theme::colors::ACCENT_BRAND
                } else {
                    colors::BORDER_SUBTLE
                },
            ),
        );
        if let Some(id) = texture(ctx, asset) {
            ui.painter().image(
                id,
                egui::Rect::from_min_size(
                    egui::pos2(x + 2.0, y + 2.0),
                    egui::vec2(cw - 4.0, if compact { 43.0 } else { 57.0 }),
                ),
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }
        ui.painter().text(
            egui::pos2(x + 4.0, y + if compact { 59.0 } else { 76.0 }),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(if compact { 9.0 } else { 10.0 }),
            colors::TEXT_SECONDARY,
        );
    }
}

pub fn draw_preview(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    rect: egui::Rect,
    timeline_top: f32,
    narrow: bool,
) {
    ui.painter().text(
        egui::pos2(rect.left() + 15.0, rect.top() + 26.0),
        egui::Align2::LEFT_CENTER,
        "プレビュー",
        egui::FontId::proportional(18.0),
        colors::TEXT_PRIMARY,
    );
    let image = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 15.0, rect.top() + 50.0),
        egui::pos2(rect.right() - 14.0, timeline_top - 98.0),
    );
    if let Some(id) = texture(ctx, "continue_working_preview.webp") {
        ui.painter().image(
            id,
            image,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }
    ui.painter().rect_stroke(
        image,
        0.0,
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
    let box_rect = egui::Rect::from_min_size(
        egui::pos2(
            image.left() + image.width() * 0.16,
            image.top() + image.height() * 0.32,
        ),
        egui::vec2(image.width() * 0.68, image.height() * 0.35),
    );
    ui.painter().rect_stroke(
        box_rect,
        0.0,
        egui::Stroke::new(2.0_f32, crate::ui::theme::colors::ACCENT_BRAND),
    );
    let preview_text = selected_text(app)
        .map(|v| v.1)
        .unwrap_or_else(|| "まだ見ぬ世界へ".to_string());
    ui.painter().text(
        box_rect.center(),
        egui::Align2::CENTER_CENTER,
        preview_text,
        egui::FontId::proportional(if narrow { 24.0 } else { 36.0 }),
        egui::Color32::WHITE,
    );
    ui.painter().text(
        egui::pos2(box_rect.center().x, box_rect.center().y + 35.0),
        egui::Align2::CENTER_CENTER,
        "A BRIGHTER TOMORROW",
        egui::FontId::proportional(11.0),
        egui::Color32::WHITE,
    );
    ui.painter().text(
        egui::pos2(rect.left() + 15.0, timeline_top - 62.0),
        egui::Align2::LEFT_CENTER,
        "00:00:04:12",
        egui::FontId::proportional(16.0),
        crate::ui::theme::colors::ACCENT_BRAND,
    );
    ui.painter().line_segment(
        [
            egui::pos2(rect.left() + 143.0, timeline_top - 62.0),
            egui::pos2(rect.right() - 85.0, timeline_top - 62.0),
        ],
        egui::Stroke::new(3.0_f32, crate::ui::theme::colors::ACCENT_BRAND),
    );
    for (i, icon) in [
        crate::ui::icons::SVG_JUMP_BACK,
        crate::ui::icons::SVG_STEP_BACK,
        crate::ui::icons::SVG_PLAY,
        crate::ui::icons::SVG_STEP_FORWARD,
    ]
    .into_iter()
    .enumerate()
    {
        crate::ui::icons::render_svg_at(
            ui,
            format!("text-preview-transport-{i}"),
            icon,
            egui::vec2(16.0, 16.0),
            colors::TEXT_PRIMARY,
            egui::pos2(
                rect.center().x - 84.0 + i as f32 * 42.0,
                timeline_top - 34.0,
            ),
        );
    }
}
