use crate::ui::icons;
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

use super::constants::*;
use super::helpers::*;

pub(crate) fn draw_detail(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    rect: egui::Rect,
    compact: bool,
    _app: &mut KagariApp,
) {
    let pad = if compact { 16.0 } else { 22.0 };
    let card = egui::Rect::from_min_max(
        egui::pos2(rect.left() + pad, rect.top() + 18.0),
        egui::pos2(rect.right() - pad, rect.bottom() * 0.53),
    );
    let p = ui.painter().clone();
    p.rect(
        card,
        8.0,
        PANEL,
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
    icons::render_svg_at(
        ui,
        "effect-detail-star".to_string(),
        icons::SVG_STAR,
        egui::vec2(25.0, 25.0),
        ORANGE,
        egui::pos2(card.right() - 37.0, card.top() + 23.0),
    );
    p.text(
        egui::pos2(card.left() + 20.0, card.top() + 35.0),
        egui::Align2::LEFT_CENTER,
        "Glow",
        egui::FontId::proportional(if compact { 24.0 } else { 30.0 }),
        colors::TEXT_PRIMARY,
    );
    if compact {
        p.text(
            egui::pos2(card.left() + 20.0, card.top() + 65.0),
            egui::Align2::LEFT_CENTER,
            "明るい部分に",
            egui::FontId::proportional(11.0),
            colors::TEXT_PRIMARY,
        );
        p.text(
            egui::pos2(card.left() + 20.0, card.top() + 81.0),
            egui::Align2::LEFT_CENTER,
            "にじむような発光を適用。",
            egui::FontId::proportional(11.0),
            colors::TEXT_PRIMARY,
        );
    } else {
        p.text(
            egui::pos2(card.left() + 20.0, card.top() + 70.0),
            egui::Align2::LEFT_CENTER,
            "明るい部分にじむような発光を適用します。",
            egui::FontId::proportional(13.0),
            colors::TEXT_PRIMARY,
        );
    }
    let tags_top = card.top() + if compact { 104.0 } else { 96.0 };
    let tag_gap = 6.0;
    let tag_width = ((card.width() - 40.0 - tag_gap * 2.0) / 3.0).max(52.0);
    for (i, tag) in ["光・発光", "スタイライズ", "よく使う"]
        .into_iter()
        .enumerate()
    {
        let x = card.left() + 20.0 + i as f32 * (tag_width + tag_gap);
        let tag_rect =
            egui::Rect::from_min_size(egui::pos2(x, tags_top), egui::vec2(tag_width, 31.0));
        p.rect(
            tag_rect,
            15.0,
            crate::ui::theme::colors::BG_PANEL,
            egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
        );
        p.text(
            tag_rect.center(),
            egui::Align2::CENTER_CENTER,
            tag,
            egui::FontId::proportional(if compact { 9.0 } else { 10.0 }),
            colors::TEXT_PRIMARY,
        );
    }
    let rows = [("しきい値", "0.60"), ("強さ", "5.00"), ("拡散", "1.00")];
    let slider_left = card.left() + if compact { 116.0 } else { 160.0 };
    let slider_right = card.right() - if compact { 96.0 } else { 84.0 };
    for (i, (label, value)) in rows.into_iter().enumerate() {
        let y = card.top() + 171.0 + i as f32 * 44.0;
        p.text(
            egui::pos2(card.left() + 20.0, y),
            egui::Align2::LEFT_CENTER,
            if i == 0 { "◉" } else { "◷" },
            egui::FontId::proportional(16.0),
            colors::TEXT_PRIMARY,
        );
        p.text(
            egui::pos2(card.left() + if compact { 42.0 } else { 50.0 }, y),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(if compact { 11.0 } else { 14.0 }),
            colors::TEXT_PRIMARY,
        );
        p.line_segment(
            [egui::pos2(slider_left, y), egui::pos2(slider_right, y)],
            egui::Stroke::new(4.0_f32, colors::BORDER_MEDIUM),
        );
        let knob_x = slider_left + (slider_right - slider_left) * (0.55 + i as f32 * 0.12);
        p.line_segment(
            [egui::pos2(slider_left, y), egui::pos2(knob_x, y)],
            egui::Stroke::new(4.0_f32, ORANGE),
        );
        p.circle_filled(egui::pos2(knob_x, y), 6.0, ORANGE);
        p.rect(
            egui::Rect::from_min_size(
                egui::pos2(card.right() - if compact { 76.0 } else { 84.0 }, y - 17.0),
                egui::vec2(if compact { 58.0 } else { 63.0 }, 34.0),
            ),
            5.0,
            egui::Color32::from_rgb(22, 35, 45),
            egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
        );
        p.text(
            egui::pos2(card.right() - if compact { 47.0 } else { 52.0 }, y),
            egui::Align2::CENTER_CENTER,
            value,
            egui::FontId::proportional(if compact { 11.0 } else { 13.0 }),
            colors::TEXT_PRIMARY,
        );
    }
    if !compact {
        p.text(
            egui::pos2(card.left() + 20.0, card.bottom() - 38.0),
            egui::Align2::LEFT_CENTER,
            "カラー     ▣  白",
            egui::FontId::proportional(14.0),
            colors::TEXT_PRIMARY,
        );
        p.text(
            egui::pos2(card.right() - 20.0, card.bottom() - 38.0),
            egui::Align2::RIGHT_CENTER,
            "↻ リセット",
            egui::FontId::proportional(12.0),
            colors::TEXT_SECONDARY,
        );
    }
    let preview = egui::Rect::from_min_max(
        egui::pos2(rect.left() + pad, rect.bottom() * 0.56),
        egui::pos2(rect.right() - pad, rect.bottom() - 18.0),
    );
    p.rect(
        preview,
        8.0,
        PANEL,
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
    p.text(
        egui::pos2(preview.left() + 18.0, preview.top() + 28.0),
        egui::Align2::LEFT_CENTER,
        "プレビュー",
        egui::FontId::proportional(18.0),
        colors::TEXT_PRIMARY,
    );
    if let Some(id) = texture(
        ctx,
        "assets/home/continue_working_preview.webp",
        "effect-preview",
    ) {
        p.image(
            id,
            egui::Rect::from_min_max(
                egui::pos2(preview.left() + 12.0, preview.top() + 48.0),
                egui::pos2(preview.right() - 12.0, preview.bottom() - 66.0),
            ),
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }
    crate::ui::icons::render_svg_at(
        ui,
        "effects-preview-play".to_string(),
        crate::ui::icons::SVG_PLAY,
        egui::vec2(14.0, 14.0),
        colors::TEXT_PRIMARY,
        egui::pos2(preview.left() + 12.0, preview.bottom() - 38.0),
    );
    p.text(
        egui::pos2(preview.left() + 34.0, preview.bottom() - 31.0),
        egui::Align2::LEFT_CENTER,
        "00:00 / 00:10",
        egui::FontId::proportional(12.0),
        colors::TEXT_SECONDARY,
    );
}

pub(crate) fn draw_footer(ui: &mut egui::Ui, width: f32, mobile: bool) {
    let p = ui.painter();
    p.text(
        egui::pos2(28.0, if mobile { 17.0 } else { 25.0 }),
        egui::Align2::LEFT_CENTER,
        "Kagari VFX  |  映像で、まだ見ぬ世界を。",
        egui::FontId::proportional(11.0),
        colors::TEXT_SECONDARY,
    );
    if !mobile {
        p.text(
            egui::pos2(width * 0.5, 25.0),
            egui::Align2::CENTER_CENTER,
            "エフェクトライブラリ",
            egui::FontId::proportional(19.0),
            colors::TEXT_PRIMARY,
        );
        p.text(
            egui::pos2(width - 28.0, 25.0),
            egui::Align2::RIGHT_CENTER,
            "VFX  /  COMPOSITING  /  MORE POSSIBILITIES",
            egui::FontId::proportional(9.0),
            colors::TEXT_MUTED,
        );
    }
}
