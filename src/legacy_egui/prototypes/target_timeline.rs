use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

#[allow(dead_code)]
#[derive(Clone, Copy, Default)]
struct LayerBodyDragState {
    frame_offset: i32,
    previous_y: f32,
    y_remainder: f32,
}

#[allow(float_literal_f32_fallback)]
#[allow(dead_code)]
fn draw_target_timeline(
    app: &mut KagariApp,
    ctx: &egui::Context,
    current_frame: &mut u32,
    total_frames: u32,
) {
    let screen_height = ctx.screen_rect().height();
    let timeline_height = (screen_height * 0.38).clamp(300.0, 394.0);
    egui::TopBottomPanel::bottom("timeline_panel")
        .resizable(false)
        .exact_height(timeline_height)
        .frame(egui::Frame::none().fill(crate::ui::theme::colors::BG_DARKEST))
        .show(ctx, |ui| {
            let rect = ui.max_rect();
            let painter = ui.painter();
            let border = crate::ui::theme::colors::BORDER_SUBTLE;
            let muted = colors::TEXT_SECONDARY;
            let left_width = 338.0;
            let compact = rect.height() < 360.0;
            let header_h = if compact { 34.0 } else { 42.0 };
            let tools_h = if compact { 30.0 } else { 42.0 };
            let ruler_h = if compact { 24.0 } else { 35.0 };
            let row_h = if compact { 28.0 } else { 37.0 };
            painter.rect_filled(rect, 0.0, crate::ui::theme::colors::BG_DARKEST);
            painter.line_segment(
                [
                    egui::pos2(rect.left(), rect.top() + header_h),
                    egui::pos2(rect.right(), rect.top() + header_h),
                ],
                egui::Stroke::new(1.0, border),
            );
            painter.line_segment(
                [
                    egui::pos2(rect.left(), rect.top() + header_h + tools_h),
                    egui::pos2(rect.right(), rect.top() + header_h + tools_h),
                ],
                egui::Stroke::new(1.0, border),
            );
            painter.line_segment(
                [
                    egui::pos2(rect.left(), rect.top() + header_h + tools_h + ruler_h),
                    egui::pos2(rect.right(), rect.top() + header_h + tools_h + ruler_h),
                ],
                egui::Stroke::new(1.0, border),
            );
            painter.line_segment(
                [
                    egui::pos2(rect.left() + left_width, rect.top()),
                    egui::pos2(rect.left() + left_width, rect.bottom()),
                ],
                egui::Stroke::new(1.0, border),
            );
            painter.text(
                egui::pos2(rect.left() + 24.0, rect.top() + header_h * 0.5),
                egui::Align2::LEFT_CENTER,
                "Timeline",
                egui::FontId::proportional(if compact { 12.0 } else { 14.0 }),
                colors::TEXT_PRIMARY,
            );
            painter.line_segment(
                [
                    egui::pos2(rect.left() + 16.0, rect.top() + header_h - 2.0),
                    egui::pos2(rect.left() + 116.0, rect.top() + header_h - 2.0),
                ],
                egui::Stroke::new(2.0, crate::ui::theme::colors::ACCENT_BRAND),
            );
            painter.text(
                egui::pos2(rect.left() + 146.0, rect.top() + header_h * 0.5),
                egui::Align2::LEFT_CENTER,
                "Graph",
                egui::FontId::proportional(if compact { 11.0 } else { 13.0 }),
                muted,
            );
            for (index, glyph) in ["↖", "♧", "⌁", "✥", "⌁", "↻", "▱", "⊙"]
                .into_iter()
                .enumerate()
            {
                painter.text(
                    egui::pos2(
                        rect.left() + 31.0 + index as f32 * 34.0,
                        rect.top() + header_h + tools_h * 0.5,
                    ),
                    egui::Align2::CENTER_CENTER,
                    glyph,
                    egui::FontId::proportional(if compact { 13.0 } else { 16.0 }),
                    if index == 0 {
                        crate::ui::theme::colors::ACCENT_BRAND
                    } else {
                        muted
                    },
                );
            }
            let ruler_y = rect.top() + header_h + tools_h;
            for second in 0..33 {
                let x = rect.left() + left_width + second as f32 * 37.0;
                painter.line_segment(
                    [
                        egui::pos2(x, ruler_y + ruler_h * 0.6),
                        egui::pos2(x, ruler_y + ruler_h),
                    ],
                    egui::Stroke::new(1.0, border),
                );
                if second % 5 == 0 {
                    painter.text(
                        egui::pos2(x + 4.0, ruler_y + ruler_h * 0.4),
                        egui::Align2::LEFT_CENTER,
                        format!("00:{second:02}"),
                        egui::FontId::proportional(if compact { 9.0 } else { 11.0 }),
                        muted,
                    );
                }
            }
            let names = [
                (6usize, "ADJ", "Adjustment Layer"),
                (1usize, "TEXT", "Main Title"),
                (5usize, "FX", "Particles"),
                (4usize, "MEDIA", "Footage"),
                (0usize, "AUDIO", "Ambient"),
                (2usize, "FX", "Light Leak"),
            ];
            let rows_top = ruler_y + ruler_h;
            let selected_layer = app.selection.selected_layer_idx;
            for (index, (layer_idx, track, name)) in names.into_iter().enumerate() {
                let y = rows_top + index as f32 * row_h;
                painter.line_segment(
                    [
                        egui::pos2(rect.left(), y + row_h),
                        egui::pos2(rect.right(), y + row_h),
                    ],
                    egui::Stroke::new(1.0, border),
                );
                if selected_layer == Some(layer_idx) {
                    painter.rect_filled(
                        egui::Rect::from_min_size(
                            egui::pos2(rect.left(), y),
                            egui::vec2(rect.width(), row_h),
                        ),
                        0.0,
                        egui::Color32::from_rgba_premultiplied(25, 85, 135, 42),
                    );
                }
                painter.text(
                    egui::pos2(rect.left() + 28.0, y + row_h * 0.5),
                    egui::Align2::CENTER_CENTER,
                    "◉",
                    egui::FontId::proportional(if compact { 11.0 } else { 14.0 }),
                    muted,
                );
                painter.text(
                    egui::pos2(rect.left() + 63.0, y + row_h * 0.5),
                    egui::Align2::CENTER_CENTER,
                    "♙",
                    egui::FontId::proportional(if compact { 11.0 } else { 14.0 }),
                    muted,
                );
                painter.rect_filled(
                    egui::Rect::from_min_size(
                        egui::pos2(rect.left() + 82.0, y + 1.0),
                        egui::vec2(50.0, row_h - 2.0),
                    ),
                    0.0,
                    egui::Color32::from_rgb(19, 30, 38),
                );
                painter.text(
                    egui::pos2(rect.left() + 99.0, y + row_h * 0.5),
                    egui::Align2::CENTER_CENTER,
                    track,
                    egui::FontId::proportional(if compact { 9.0 } else { 11.0 }),
                    muted,
                );
                painter.text(
                    egui::pos2(rect.left() + 137.0, y + row_h * 0.5),
                    egui::Align2::LEFT_CENTER,
                    "▸",
                    egui::FontId::proportional(if compact { 9.0 } else { 11.0 }),
                    muted,
                );
                painter.text(
                    egui::pos2(rect.left() + 152.0, y + row_h * 0.5),
                    egui::Align2::LEFT_CENTER,
                    name,
                    egui::FontId::proportional(if compact { 10.0 } else { 12.0 }),
                    colors::TEXT_PRIMARY,
                );
                let bar_left =
                    rect.left() + left_width + [7.0, 185.0, 188.0, 108.0, 27.0, 62.0][index];
                let bar_width: f32 = [766.0, 535.0, 324.0, 633.0, 912.0, 492.0][index];
                let bar_color = [
                    egui::Color32::from_rgb(91, 80, 169),
                    egui::Color32::from_rgb(156, 77, 150),
                    egui::Color32::from_rgb(37, 108, 177),
                    egui::Color32::from_rgb(40, 108, 182),
                    egui::Color32::from_rgb(34, 124, 91),
                    egui::Color32::from_rgb(35, 112, 82),
                ][index];
                painter.rect_filled(
                    egui::Rect::from_min_size(
                        egui::pos2(bar_left, y + 3.0),
                        egui::vec2(
                            bar_width.min(rect.right() - bar_left - 8.0),
                            (row_h - 6.0).max(12.0),
                        ),
                    ),
                    2.0,
                    bar_color,
                );
                painter.text(
                    egui::pos2(bar_left + 9.0, y + row_h * 0.5),
                    egui::Align2::LEFT_CENTER,
                    [
                        "Adjustment Layer",
                        "CREATE COMPOSITE ILLUMINATE",
                        "particles.mp4",
                        "mountain.mp4",
                        "ambient.mp3",
                        "whoosh.wav",
                    ][index],
                    egui::FontId::proportional(if compact { 10.0 } else { 12.0 }),
                    egui::Color32::from_rgb(230, 232, 237),
                );
                if layer_idx == 1 {
                    for keyframe_x in [0.34, 0.56, 0.76] {
                        painter.text(
                            egui::pos2(
                                bar_left
                                    + bar_width.min(rect.right() - bar_left - 8.0) * keyframe_x,
                                y + row_h * 0.5,
                            ),
                            egui::Align2::CENTER_CENTER,
                            "◆",
                            egui::FontId::proportional(if compact { 8.0 } else { 10.0 }),
                            egui::Color32::from_rgb(230, 220, 255),
                        );
                    }
                }
            }
            let play_x = rect.left() + left_width + 164.0;
            painter.line_segment(
                [
                    egui::pos2(play_x, ruler_y - 2.0),
                    egui::pos2(play_x, rect.bottom()),
                ],
                egui::Stroke::new(2.0, crate::ui::theme::colors::ACCENT_BRAND),
            );
            painter.rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(play_x - 6.0, ruler_y - 5.0),
                    egui::vec2(12.0, 12.0),
                ),
                2.0,
                crate::ui::theme::colors::ACCENT_BRAND,
            );
            *current_frame = (*current_frame).min(total_frames.saturating_sub(1));
            let _ = app;
        });
}
