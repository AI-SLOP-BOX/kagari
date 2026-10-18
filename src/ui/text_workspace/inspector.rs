//! Text property inspector and mini track timeline for text workspace.

use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub fn selected_text(app: &KagariApp) -> Option<(usize, String, u32, [f32; 4])> {
    let idx = app.selection.selected_layer_idx?;
    let comp = app.history.current().active_composition();
    let layer = comp.layers.get(idx)?;
    if let crate::core::timeline::LayerType::Text {
        text,
        font_size,
        color,
        ..
    } = &layer.layer_type
    {
        Some((idx, text.clone(), *font_size, *color))
    } else {
        None
    }
}

pub fn draw_inspector(app: &mut KagariApp, ui: &mut egui::Ui, rect: egui::Rect, narrow: bool) {
    ui.painter().text(
        egui::pos2(rect.left() + 20.0, rect.top() + 27.0),
        egui::Align2::LEFT_CENTER,
        "テキスト       アニメーション       詳細設定",
        egui::FontId::proportional(if narrow { 10.0 } else { 14.0 }),
        colors::TEXT_PRIMARY,
    );
    ui.painter().line_segment(
        [
            egui::pos2(rect.left() + 10.0, rect.top() + 44.0),
            egui::pos2(rect.left() + 96.0, rect.top() + 44.0),
        ],
        egui::Stroke::new(2.0_f32, crate::ui::theme::colors::ACCENT_BRAND),
    );
    let Some((idx, mut text, mut font_size, mut color)) = selected_text(app) else {
        return;
    };
    let mut changed = false;
    let label_x = rect.left() + 20.0;
    let field_x = rect.left() + 138.0;
    let field_w = (rect.right() - field_x - 20.0).max(60.0);
    ui.allocate_new_ui(
        egui::UiBuilder::new().max_rect(egui::Rect::from_min_max(
            egui::pos2(field_x, rect.top() + 58.0),
            egui::pos2(rect.right() - 20.0, rect.top() + 102.0),
        )),
        |u| {
            u.add(egui::TextEdit::singleline(&mut text).desired_width(field_w));
        },
    );
    ui.painter().text(
        egui::pos2(label_x, rect.top() + 75.0),
        egui::Align2::LEFT_CENTER,
        "テキスト内容",
        egui::FontId::proportional(if narrow { 9.0 } else { 12.0 }),
        colors::TEXT_SECONDARY,
    );
    ui.allocate_new_ui(
        egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
            egui::pos2(field_x, rect.top() + 106.0),
            egui::vec2(field_w, 32.0),
        )),
        |u| {
            if u.add(
                egui::DragValue::new(&mut font_size)
                    .range(8..=512)
                    .prefix("サイズ "),
            )
            .changed()
            {
                changed = true;
            }
        },
    );
    ui.painter().text(
        egui::pos2(label_x, rect.top() + 122.0),
        egui::Align2::LEFT_CENTER,
        "フォントサイズ",
        egui::FontId::proportional(if narrow { 9.0 } else { 12.0 }),
        colors::TEXT_SECONDARY,
    );
    ui.allocate_new_ui(
        egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
            egui::pos2(field_x, rect.top() + 145.0),
            egui::vec2(field_w, 32.0),
        )),
        |u| {
            if u.color_edit_button_rgba_unmultiplied(&mut color).changed() {
                changed = true;
            }
        },
    );
    ui.painter().text(
        egui::pos2(label_x, rect.top() + 161.0),
        egui::Align2::LEFT_CENTER,
        "塗り（フィル）",
        egui::FontId::proportional(if narrow { 9.0 } else { 12.0 }),
        colors::TEXT_SECONDARY,
    );
    changed |= text != selected_text(app).map(|v| v.1).unwrap_or_default();
    if changed {
        app.modify_project(|project| {
            if let Some(layer) = project.active_composition_mut().layers.get_mut(idx) {
                if let crate::core::timeline::LayerType::Text {
                    text: dst,
                    font_size: dst_size,
                    color: dst_color,
                    ..
                } = &mut layer.layer_type
                {
                    *dst = text.clone();
                    *dst_size = font_size;
                    *dst_color = color;
                }
            }
        });
    }
}

pub fn draw_timeline(
    app: &KagariApp,
    ui: &mut egui::Ui,
    top: f32,
    left: f32,
    right: f32,
    narrow: bool,
) {
    ui.painter().line_segment(
        [egui::pos2(left, top), egui::pos2(right, top)],
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
    ui.painter().text(
        egui::pos2(left + 18.0, top + 29.0),
        egui::Align2::LEFT_CENTER,
        "タイムライン",
        egui::FontId::proportional(16.0),
        colors::TEXT_PRIMARY,
    );
    for (i, g) in ["＋", "▱", "✂", "⌕", "▣"].into_iter().enumerate() {
        ui.painter().text(
            egui::pos2(left + 160.0 + i as f32 * 43.0, top + 29.0),
            egui::Align2::CENTER_CENTER,
            g,
            egui::FontId::proportional(18.0),
            colors::TEXT_SECONDARY,
        );
    }
    let ruler = top + 57.0;
    ui.painter().line_segment(
        [egui::pos2(left, ruler), egui::pos2(right, ruler)],
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
    for i in 0..8 {
        ui.painter().text(
            egui::pos2(
                left + 145.0 + i as f32 * if narrow { 45.0 } else { 80.0 },
                ruler - 15.0,
            ),
            egui::Align2::CENTER_CENTER,
            if narrow {
                format!("00:{:02}", i * 5)
            } else {
                format!("00:00:{:02}:00", i * 5)
            },
            egui::FontId::proportional(if narrow { 8.0 } else { 10.0 }),
            colors::TEXT_SECONDARY,
        );
    }
    let selected_name = selected_text(app)
        .map(|v| v.1)
        .unwrap_or_else(|| "まだ見ぬ世界へ".to_string());
    let rows: [(&str, f32, egui::Color32, String); 4] = [
        (
            "メインタイトル",
            190.0,
            egui::Color32::from_rgb(145, 76, 166),
            selected_name,
        ),
        (
            "サブタイトル",
            160.0,
            egui::Color32::from_rgb(74, 67, 175),
            "A BRIGHTER TOMORROW".to_string(),
        ),
        (
            "背景シェイプ",
            390.0,
            egui::Color32::from_rgb(80, 76, 94),
            "光のライン".to_string(),
        ),
        (
            "映像",
            500.0,
            egui::Color32::from_rgb(35, 105, 150),
            "mountain.mp4".to_string(),
        ),
    ];
    let lane_left = left + if narrow { 120.0 } else { 190.0 };
    let lane_step = if narrow { 58.0 } else { 95.0 };
    for (i, (name, span, color, value)) in rows.into_iter().enumerate() {
        let y = ruler + 14.0 + i as f32 * 43.0;
        ui.painter().line_segment(
            [egui::pos2(left, y + 39.0), egui::pos2(right, y + 39.0)],
            egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
        );
        ui.painter().text(
            egui::pos2(left + 55.0, y + 18.0),
            egui::Align2::LEFT_CENTER,
            name,
            egui::FontId::proportional(if narrow { 9.0 } else { 12.0 }),
            colors::TEXT_PRIMARY,
        );
        let bar_left = lane_left + i as f32 * lane_step;
        let bar_width = span.min((right - bar_left - 8.0).max(32.0));
        let bar = egui::Rect::from_min_size(
            egui::pos2(bar_left, y + 4.0),
            egui::vec2(bar_width, if narrow { 24.0 } else { 30.0 }),
        );
        ui.painter().rect_filled(bar, 4.0, color);
        ui.painter().text(
            egui::pos2(bar.left() + 10.0, y + 19.0),
            egui::Align2::LEFT_CENTER,
            value,
            egui::FontId::proportional(10.0),
            colors::TEXT_PRIMARY,
        );
    }
    let play_x = left + 252.0;
    ui.painter().line_segment(
        [
            egui::pos2(play_x, ruler - 5.0),
            egui::pos2(play_x, top + 300.0),
        ],
        egui::Stroke::new(2.0_f32, crate::ui::theme::colors::ACCENT_BRAND),
    );
}
