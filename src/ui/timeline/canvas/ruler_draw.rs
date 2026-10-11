use crate::ui::theme::colors;
use eframe::egui;

pub(super) fn draw_time_ruler(
    ui: &mut egui::Ui,
    ruler_rect: egui::Rect,
    start_frame: u32,
    zoom_span: u32,
    fps: u32,
    current_frame: u32,
) {
    let fps = fps.max(1);
    let timecode_font = egui::FontId::new(
        ui.text_style_height(&egui::TextStyle::Small) * 0.72,
        egui::FontFamily::Monospace,
    );
    let end_frame = start_frame.saturating_add(zoom_span);
    let measured_label_width = [start_frame, end_frame]
        .into_iter()
        .map(|frame| {
            let label = crate::ui::timeline::ruler::format_timecode_label(frame, fps);
            ui.painter()
                .layout_no_wrap(label, timecode_font.clone(), colors::TEXT_MUTED)
                .size()
                .x
        })
        .fold(0.0_f32, f32::max);
    let step = crate::ui::timeline::ruler::ruler_label_step(
        zoom_span,
        ruler_rect.width(),
        measured_label_width,
    );
    let mut last_label_right = f32::NEG_INFINITY;
    for frame in crate::ui::timeline::ruler::ruler_tick_frames(start_frame, zoom_span, step) {
        let norm = (frame - start_frame) as f32 / zoom_span as f32;
        let tick_x = ruler_rect.left() + norm * ruler_rect.width();
        ui.painter().line_segment(
            [
                egui::pos2(tick_x, ruler_rect.bottom() - 6.0),
                egui::pos2(tick_x, ruler_rect.bottom()),
            ],
            egui::Stroke::new(1.0_f32, colors::BORDER_STRONG),
        );
        let label = crate::ui::timeline::ruler::format_timecode_label(frame, fps);
        let galley = ui
            .painter()
            .layout_no_wrap(label, timecode_font.clone(), colors::TEXT_MUTED);
        let label_rect =
            egui::Rect::from_center_size(egui::pos2(tick_x, ruler_rect.top() + 7.0), galley.size());
        if label_rect.left() >= ruler_rect.left()
            && label_rect.right() <= ruler_rect.right()
            && label_rect.left() >= last_label_right + 6.0
        {
            ui.painter().with_clip_rect(ruler_rect).galley(
                label_rect.min,
                galley,
                colors::TEXT_MUTED,
            );
            last_label_right = label_rect.right();
        }
    }

    let playhead_norm = (current_frame as f32 - start_frame as f32) / zoom_span as f32;
    let playhead_x = ruler_rect.left() + playhead_norm * ruler_rect.width();
    if (0.0..=1.0).contains(&playhead_norm) {
        ui.painter().line_segment(
            [
                egui::pos2(playhead_x, ruler_rect.top() + 4.0),
                egui::pos2(playhead_x, ruler_rect.bottom()),
            ],
            egui::Stroke::new(1.5_f32, colors::TIMELINE_PLAYHEAD),
        );
        let triangle = vec![
            egui::pos2(playhead_x - 4.0, ruler_rect.top()),
            egui::pos2(playhead_x + 4.0, ruler_rect.top()),
            egui::pos2(playhead_x, ruler_rect.top() + 6.0),
        ];
        ui.painter().add(egui::Shape::convex_polygon(
            triangle,
            colors::TIMELINE_PLAYHEAD,
            egui::Stroke::NONE,
        ));
    }
}
