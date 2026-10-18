//! Left-side chapter navigation and about panel for tutorial workspace.

use crate::ui::icons;
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

use super::{selected_chapter, texture, ORANGE, PANEL};

pub fn draw_chapters(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    width: f32,
    compact: bool,
    app: &mut KagariApp,
) {
    {
        let p = ui.painter();
        p.rect_filled(
            egui::Rect::from_min_max(rect.min, egui::pos2(rect.left() + width, rect.bottom())),
            0.0,
            crate::ui::theme::colors::BG_DARKEST,
        );
        p.line_segment(
            [
                egui::pos2(rect.left() + width, rect.top()),
                egui::pos2(rect.left() + width, rect.bottom()),
            ],
            egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
        );
    }
    let chapter_icons = [
        icons::SVG_HOME,
        icons::SVG_FILE,
        icons::SVG_LAYERS,
        icons::SVG_MARKER,
        icons::SVG_EFFECTS,
        icons::SVG_FILE_PLUS,
        icons::SVG_BOOK,
    ];
    for (i, icon) in chapter_icons
        .into_iter()
        .take(crate::ui::tutorial::chapters().len())
        .enumerate()
    {
        icons::render_svg_at(
            ui,
            format!("tutorial-chapter-{i}"),
            icon,
            egui::vec2(30.0, 30.0),
            if i == selected_chapter(app.tutorial_step) {
                ORANGE
            } else {
                colors::TEXT_PRIMARY
            },
            egui::pos2(
                rect.left() + 28.0,
                rect.top() + 97.0 + i as f32 * if compact { 61.0 } else { 78.0 },
            ),
        );
    }
    let p = ui.painter().clone();
    let logo_rect = egui::Rect::from_min_size(
        egui::pos2(rect.left() + 36.0, rect.top() + 20.0),
        egui::vec2(55.0, 55.0),
    );
    if let Some(id) = texture(ui.ctx(), "assets/kagari_logo.webp", "tutorial-logo") {
        p.image(
            id,
            logo_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }
    p.text(
        egui::pos2(rect.left() + 105.0, rect.top() + 48.0),
        egui::Align2::LEFT_CENTER,
        "Kagari",
        egui::FontId::proportional(27.0),
        colors::TEXT_PRIMARY,
    );
    p.text(
        egui::pos2(rect.left() + 185.0, rect.top() + 48.0),
        egui::Align2::LEFT_CENTER,
        "VFX",
        egui::FontId::proportional(27.0),
        colors::TEXT_SECONDARY,
    );
    for (i, chapter) in crate::ui::tutorial::chapters().iter().enumerate() {
        let item_h = if compact { 57.0 } else { 72.0 };
        let y = rect.top() + 25.0 + i as f32 * if compact { 61.0 } else { 78.0 } + 72.0;
        let item = egui::Rect::from_min_size(
            egui::pos2(rect.left() + 13.0, y),
            egui::vec2(width - 25.0, item_h),
        );
        let hovered = ui.rect_contains_pointer(item);
        if i == selected_chapter(app.tutorial_step) {
            p.rect_filled(item, 7.0, egui::Color32::from_rgb(29, 32, 36));
            p.rect_filled(
                egui::Rect::from_min_size(item.min, egui::vec2(5.0, item.height())),
                3.0,
                ORANGE,
            );
        } else if hovered {
            p.rect_filled(item, 7.0, egui::Color32::from_rgb(23, 31, 38));
        }
        let label_left = item.left() + 62.0;
        let label_right = item.right() - 12.0;
        ui.put(
            egui::Rect::from_min_max(
                egui::pos2(label_left, item.top() + 3.0),
                egui::pos2(label_right, item.top() + item_h * 0.53),
            ),
            egui::Label::new(
                egui::RichText::new(chapter.title)
                    .size(if compact { 13.0 } else { 16.0 })
                    .color(colors::TEXT_PRIMARY),
            )
            .truncate(),
        );
        ui.put(
            egui::Rect::from_min_max(
                egui::pos2(label_left, item.top() + item_h * 0.49),
                egui::pos2(label_right, item.bottom() - 2.0),
            ),
            egui::Label::new(
                egui::RichText::new(chapter.description)
                    .size(if compact { 9.0 } else { 12.0 })
                    .color(colors::TEXT_SECONDARY),
            )
            .truncate(),
        );
        let response = ui.interact(
            item,
            egui::Id::new(("tutorial-chapter", i)),
            egui::Sense::click(),
        );
        if response.clicked() {
            app.tutorial_step = chapter.first_step;
        }
    }
}

pub fn draw_about(ui: &mut egui::Ui, rect: egui::Rect, left: f32, _width: f32, compact: bool) {
    let p = ui.painter();
    let panel = egui::Rect::from_min_max(
        egui::pos2(left + 20.0, rect.top() + 28.0),
        egui::pos2(rect.right() - 20.0, rect.bottom() - 18.0),
    );
    p.rect(
        panel,
        10.0,
        PANEL,
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
    let x = panel.left() + if compact { 26.0 } else { 32.0 };
    p.text(
        egui::pos2(x, panel.top() + 54.0),
        egui::Align2::LEFT_CENTER,
        "Kagari VFX とは",
        egui::FontId::proportional(if compact { 25.0 } else { 32.0 }),
        colors::TEXT_PRIMARY,
    );
    p.line_segment(
        [
            egui::pos2(x, panel.top() + 87.0),
            egui::pos2(x + 55.0, panel.top() + 87.0),
        ],
        egui::Stroke::new(2.0_f32, ORANGE),
    );
    let desc = "選んだ映像をタイムラインに読み込み、\nエフェクトを加え、再生して変化を確認します。\n説明だけで終わらず、編集画面で実際に操作します。";
    p.text(
        egui::pos2(x, panel.top() + 125.0),
        egui::Align2::LEFT_TOP,
        desc,
        egui::FontId::proportional(if compact { 12.0 } else { 15.0 }),
        colors::TEXT_PRIMARY,
    );
    p.line_segment(
        [
            egui::pos2(x, panel.top() + 225.0),
            egui::pos2(panel.right() - 32.0, panel.top() + 225.0),
        ],
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
    p.text(
        egui::pos2(x, panel.top() + 258.0),
        egui::Align2::LEFT_CENTER,
        "このチュートリアルで学べること",
        egui::FontId::proportional(if compact { 16.0 } else { 19.0 }),
        colors::TEXT_PRIMARY,
    );
    let items = [
        ("画面を知る", "Viewer・Project・Timeline の役割"),
        ("コンポジション", "解像度とフレームレートを設定"),
        ("レイヤー", "素材を重ねて合成"),
        ("アニメーション", "キーフレームを追加して再生"),
        ("動画エフェクト実習", "実映像に効果を加えて再生"),
        ("書き出し", "設定を確認して映像を出力"),
    ];
    for (i, (title, sub)) in items.into_iter().enumerate() {
        let y = panel.top() + 310.0 + i as f32 * if compact { 58.0 } else { 61.0 };
        p.circle_stroke(
            egui::pos2(x + 19.0, y),
            19.0,
            egui::Stroke::new(1.0_f32, colors::TEXT_SECONDARY),
        );
        p.text(
            egui::pos2(x + 19.0, y),
            egui::Align2::CENTER_CENTER,
            format!("{}", i + 1),
            egui::FontId::proportional(14.0),
            colors::TEXT_PRIMARY,
        );
        p.text(
            egui::pos2(x + 62.0, y - 8.0),
            egui::Align2::LEFT_CENTER,
            title,
            egui::FontId::proportional(if compact { 12.0 } else { 15.0 }),
            colors::TEXT_PRIMARY,
        );
        p.text(
            egui::pos2(x + 62.0, y + 14.0),
            egui::Align2::LEFT_CENTER,
            sub,
            egui::FontId::proportional(if compact { 10.0 } else { 12.0 }),
            colors::TEXT_SECONDARY,
        );
    }
    p.text(
        egui::pos2(panel.right() - 30.0, panel.bottom() - 28.0),
        egui::Align2::RIGHT_CENTER,
        "創造の可能性を\n広げよう。",
        egui::FontId::proportional(15.0),
        colors::TEXT_SECONDARY,
    );
}
