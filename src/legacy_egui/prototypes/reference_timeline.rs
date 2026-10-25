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

#[allow(dead_code)]
fn draw_reference_timeline(
    app: &mut KagariApp,
    ctx: &egui::Context,
    current_frame: &mut u32,
    total_frames: u32,
) {
    egui::TopBottomPanel::bottom("timeline_panel")
        .resizable(true)
        .min_height(350.0)
        .default_height(359.0)
        .show(ctx, |ui| {
            let rect = ui.max_rect();
            let left_width = 312.0_f32.min(rect.width() * 0.24);
            let main_left = rect.left() + left_width;
            let list_width = (rect.width() * 0.33).clamp(420.0, 506.0);
            let bars_left = (main_left + list_width).min(rect.right() - 220.0);
            let fill = egui::Color32::from_rgb(14, 24, 34);
            let border = egui::Color32::from_rgb(39, 55, 71);
            let secondary = colors::TEXT_SECONDARY;
            ui.painter().rect_filled(rect, 0.0, fill);
            ui.painter().rect_filled(
                egui::Rect::from_min_max(rect.min, egui::pos2(main_left, rect.bottom())),
                0.0,
                egui::Color32::from_rgb(12, 22, 32),
            );
            ui.painter().line_segment(
                [
                    egui::pos2(main_left, rect.top()),
                    egui::pos2(main_left, rect.bottom()),
                ],
                egui::Stroke::new(1.0_f32, border),
            );

            let assets = [
                ("city_01.mp4", "3840 × 2160   00:12"),
                ("mountain_bg.exr", "3840 × 2160"),
                ("smoke_01.mov", "3840 × 2160"),
                ("light_leak.mov", "3840 × 2160"),
                ("glass_texture.png", "4096 × 4096"),
                ("logo.png", "1024 × 1024"),
            ];
            for (index, (name, meta)) in assets.into_iter().enumerate() {
                let y = rect.top() + 8.0 + index as f32 * 55.0;
                let selected = index == 0;
                let row = egui::Rect::from_min_size(
                    egui::pos2(rect.left() + 10.0, y),
                    egui::vec2(left_width - 20.0, 48.0),
                );
                if selected {
                    ui.painter()
                        .rect_filled(row, 4.0, crate::ui::theme::colors::BG_ACTIVE);
                }
                ui.painter()
                    .rect_stroke(row, 4.0, egui::Stroke::new(1.0_f32, border));
                let thumb = egui::Rect::from_min_size(
                    egui::pos2(row.left() + 8.0, row.top() + 5.0),
                    egui::vec2(62.0, 38.0),
                );
                if let Some(texture) = timeline_reference_texture(ui.ctx(), index) {
                    ui.painter().image(
                        texture.id(),
                        thumb,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                } else {
                    ui.painter()
                        .rect_filled(thumb, 3.0, egui::Color32::from_rgb(31, 48, 64));
                }
                ui.painter().text(
                    egui::pos2(thumb.right() + 9.0, row.top() + 16.0),
                    egui::Align2::LEFT_CENTER,
                    name,
                    egui::FontId::proportional(11.0),
                    colors::TEXT_PRIMARY,
                );
                ui.painter().text(
                    egui::pos2(thumb.right() + 9.0, row.top() + 33.0),
                    egui::Align2::LEFT_CENTER,
                    meta,
                    egui::FontId::proportional(9.0),
                    secondary,
                );
            }
            ui.painter().text(
                egui::pos2(rect.left() + 12.0, rect.bottom() - 12.0),
                egui::Align2::LEFT_CENTER,
                "Frame Render Time   12ms",
                egui::FontId::proportional(10.0),
                secondary,
            );

            let tabs = [
                ("main_comp", true),
                ("text_comp", false),
                ("render_comp", false),
            ];
            for (index, (label, active)) in tabs.into_iter().enumerate() {
                let x = main_left + 12.0 + index as f32 * 116.0;
                ui.painter().text(
                    egui::pos2(x + 14.0, rect.top() + 22.0),
                    egui::Align2::LEFT_CENTER,
                    label,
                    egui::FontId::proportional(12.0),
                    if active {
                        colors::TEXT_PRIMARY
                    } else {
                        secondary
                    },
                );
                if active {
                    ui.painter().line_segment(
                        [
                            egui::pos2(x, rect.top() + 37.0),
                            egui::pos2(x + 96.0, rect.top() + 37.0),
                        ],
                        egui::Stroke::new(2.0_f32, colors::ACCENT_BLUE),
                    );
                }
            }
            ui.painter().line_segment(
                [
                    egui::pos2(main_left, rect.top() + 42.0),
                    egui::pos2(rect.right(), rect.top() + 42.0),
                ],
                egui::Stroke::new(1.0_f32, border),
            );
            ui.painter().text(
                egui::pos2(main_left + 14.0, rect.top() + 64.0),
                egui::Align2::LEFT_CENTER,
                "00:00:03:12",
                egui::FontId::proportional(16.0),
                colors::ACCENT_BLUE,
            );
            ui.painter().text(
                egui::pos2(main_left + 14.0, rect.top() + 80.0),
                egui::Align2::LEFT_CENTER,
                "00102   (24.00 fps)",
                egui::FontId::proportional(9.0),
                secondary,
            );
            for (index, glyph) in ["↶", "♧", "♧", "↷", "▧"].into_iter().enumerate() {
                ui.painter().text(
                    egui::pos2(main_left + 165.0 + index as f32 * 28.0, rect.top() + 66.0),
                    egui::Align2::CENTER_CENTER,
                    glyph,
                    egui::FontId::proportional(15.0),
                    secondary,
                );
            }
            ui.painter().line_segment(
                [
                    egui::pos2(main_left, rect.top() + 91.0),
                    egui::pos2(rect.right(), rect.top() + 91.0),
                ],
                egui::Stroke::new(1.0_f32, border),
            );

            let ruler_top = rect.top() + 94.0;
            ui.painter().text(
                egui::pos2(bars_left + 8.0, ruler_top + 16.0),
                egui::Align2::LEFT_CENTER,
                "00s          02s          04s          06s          08s          10s          12s",
                egui::FontId::proportional(10.0),
                secondary,
            );
            ui.painter().line_segment(
                [
                    egui::pos2(bars_left, ruler_top + 30.0),
                    egui::pos2(rect.right() - 10.0, ruler_top + 30.0),
                ],
                egui::Stroke::new(1.0_f32, border),
            );
            let names = [
                "[Adjustment layer]",
                "[logo.png]",
                "[Light Leak]",
                "[Character]",
                "[city_01.mp4]",
                "[Clouds]",
                "[BG]",
            ];
            let spans = [
                (8, 142),
                (14, 132),
                (4, 126),
                (34, 149),
                (26, 149),
                (20, 146),
                (12, 146),
            ];
            let row_top = ruler_top + 38.0;
            let row_height =
                ((rect.bottom() - row_top - 20.0) / names.len() as f32).clamp(25.0, 31.0);
            for (index, name) in names.into_iter().enumerate() {
                let y = row_top + index as f32 * row_height;
                ui.painter().line_segment(
                    [
                        egui::pos2(main_left, y + row_height),
                        egui::pos2(rect.right(), y + row_height),
                    ],
                    egui::Stroke::new(1.0_f32, border),
                );
                ui.painter().text(
                    egui::pos2(main_left + 12.0, y + row_height * 0.5),
                    egui::Align2::LEFT_CENTER,
                    format!("{}  {}", index + 1, name),
                    egui::FontId::proportional(11.0),
                    colors::TEXT_PRIMARY,
                );
                let start = bars_left
                    + (spans[index].0 as f32 / total_frames.max(1) as f32)
                        * (rect.right() - bars_left - 10.0);
                let end = bars_left
                    + (spans[index].1 as f32 / total_frames.max(1) as f32)
                        * (rect.right() - bars_left - 10.0);
                let bar = egui::Rect::from_min_max(
                    egui::pos2(start, y + 4.0),
                    egui::pos2(end.max(start + 8.0), y + row_height - 5.0),
                );
                let colors_by_row = [
                    egui::Color32::from_rgb(111, 105, 157),
                    egui::Color32::from_rgb(118, 108, 161),
                    egui::Color32::from_rgb(172, 105, 142),
                    egui::Color32::from_rgb(42, 67, 82),
                    egui::Color32::from_rgb(73, 145, 157),
                    egui::Color32::from_rgb(70, 128, 119),
                    egui::Color32::from_rgb(56, 135, 116),
                ];
                ui.painter().rect_filled(bar, 2.0, colors_by_row[index]);
                if index == 1 || index == 3 {
                    for frame in if index == 1 {
                        [28, 68, 110].as_slice()
                    } else {
                        [56, 82, 122].as_slice()
                    } {
                        let x = bars_left
                            + (*frame as f32 / total_frames.max(1) as f32)
                                * (rect.right() - bars_left - 10.0);
                        let diamond = [
                            egui::pos2(x, y + row_height * 0.5 - 5.0),
                            egui::pos2(x + 5.0, y + row_height * 0.5),
                            egui::pos2(x, y + row_height * 0.5 + 5.0),
                            egui::pos2(x - 5.0, y + row_height * 0.5),
                        ];
                        ui.painter().add(egui::Shape::convex_polygon(
                            diamond.to_vec(),
                            colors::TEXT_PRIMARY,
                            egui::Stroke::NONE,
                        ));
                    }
                }
            }
            let play_x = bars_left
                + (102.0 / total_frames.max(1) as f32) * (rect.right() - bars_left - 10.0);
            ui.painter().line_segment(
                [
                    egui::pos2(play_x, ruler_top + 2.0),
                    egui::pos2(play_x, rect.bottom() - 14.0),
                ],
                egui::Stroke::new(2.0_f32, colors::ACCENT_CYAN),
            );
            *current_frame = (*current_frame).min(total_frames.saturating_sub(1));
            let _ = app;
        });
}

#[allow(dead_code)]
fn timeline_reference_texture(ctx: &egui::Context, index: usize) -> Option<egui::TextureHandle> {
    let id = egui::Id::new(("reference-timeline-thumb", index));
    if let Some(texture) = ctx.data_mut(|data| data.get_temp::<egui::TextureHandle>(id)) {
        return Some(texture);
    }
    let file = match index {
        0 => "assets/studio/assets_city.webp",
        1 => "assets/studio/assets_mountain.webp",
        2 => "assets/studio/assets_clouds.webp",
        3 => "assets/studio/assets_light_leak.webp",
        4 => "assets/studio/assets_glass_texture.webp",
        _ => "assets/kagari_logo.webp",
    };
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(file);
    let image = crate::ui::embedded_assets::open_image(path)
        .ok()?
        .to_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    let texture = ctx.load_texture(
        format!("reference-timeline-thumb-{index}"),
        egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw()),
        egui::TextureOptions::LINEAR,
    );
    ctx.data_mut(|data| data.insert_temp(id, texture.clone()));
    Some(texture)
}
