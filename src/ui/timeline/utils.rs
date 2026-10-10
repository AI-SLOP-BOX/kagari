pub use super::geometry::{
    clamp_view_start, frame_to_x, visible_range, x_to_frame, TimelineViewport,
};
pub use super::keyframe_tick::{draw_keyframe_tick, KeyframeTickResult};
pub use super::ruler::{format_timecode_label, ruler_label_step, ruler_tick_frames};
pub use super::snapping::{
    collect_all_kf_frames, maybe_snap_frame, snap_to_layer_edges, snap_to_sorted_layer_edges,
};
use crate::core::property::Animatable;

pub fn get_kfs<T: Clone>(
    prop: &Animatable<T>,
) -> Vec<(u32, crate::core::keyframe::InterpolationType)> {
    prop.keyframes()
        .map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.interpolation)).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::colors;
    use eframe::egui;

    #[test]
    fn sorted_snap_search_chooses_nearest_keyframe_and_respects_threshold() {
        let frames = [10, 20, 31, 50];
        assert_eq!(maybe_snap_frame(29, true, &frames), 31);
        assert_eq!(maybe_snap_frame(25, true, &frames), 25);
        assert_eq!(maybe_snap_frame(29, false, &frames), 29);
    }

    #[test]
    fn sorted_layer_snap_skips_the_dragged_layer() {
        let edges = [(95, 2), (98, 1), (101, 0), (103, 1), (108, 3)];
        assert_eq!(snap_to_sorted_layer_edges(100, 1, &edges), 101);
        assert_eq!(snap_to_sorted_layer_edges(100, 0, &edges), 98);
        assert_eq!(
            snap_to_sorted_layer_edges(100, 1, &[(94, 1), (105, 2)]),
            105
        );
    }

    #[test]
    fn visible_range_basic() {
        let (start, span) = visible_range(1000, 1.0, 0);
        assert_eq!(start, 0);
        assert_eq!(span, 1000);
    }

    #[test]
    fn visible_range_zoomed() {
        let (_, span) = visible_range(1000, 2.0, 0);
        assert_eq!(span, 500);
    }

    #[test]
    fn visible_range_min_span() {
        let (_, span) = visible_range(0, 1.0, 0);
        assert_eq!(span, 10);
    }

    #[test]
    fn frame_to_x_start() {
        let rect = egui::Rect::from_min_size(egui::pos2(100.0, 0.0), egui::vec2(400.0, 20.0));
        let x = frame_to_x(0, 0, 100, rect);
        assert_eq!(x, 100.0);
    }

    #[test]
    fn frame_to_x_end() {
        let rect = egui::Rect::from_min_size(egui::pos2(100.0, 0.0), egui::vec2(400.0, 20.0));
        let x = frame_to_x(100, 0, 100, rect);
        assert_eq!(x, 500.0);
    }

    #[test]
    fn frame_to_x_offset_view() {
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(400.0, 20.0));
        let x = frame_to_x(50, 50, 100, rect);
        assert_eq!(x, 0.0);
    }

    #[test]
    fn x_to_frame_roundtrip() {
        let rect = egui::Rect::from_min_size(egui::pos2(100.0, 0.0), egui::vec2(400.0, 20.0));
        let f = x_to_frame(300.0, 0, 100, rect);
        assert_eq!(f, 50);
    }

    #[test]
    fn clamp_view_start_no_change() {
        let v = clamp_view_start(50, 100, 1000, 0);
        assert_eq!(v, 0);
    }

    #[test]
    fn clamp_view_start_recenter() {
        let v = clamp_view_start(150, 100, 1000, 0);
        assert_eq!(v, 100);
    }

    #[test]
    fn clamp_view_start_at_end() {
        let v = clamp_view_start(999, 100, 1000, 0);
        assert!(v + 100 >= 999);
    }

    #[test]
    fn ruler_and_layers_share_same_start_frame() {
        let total_frames = 500u32;
        let zoom = 2.0f32;
        let view_start = 100u32;
        let current_frame = 200u32;

        let (_, zoom_span_1) = visible_range(total_frames, zoom, view_start);
        let effective_start_1 =
            clamp_view_start(current_frame, zoom_span_1, total_frames, view_start);

        let (_, zoom_span_2) = visible_range(total_frames, zoom, view_start);
        let effective_start_2 =
            clamp_view_start(current_frame, zoom_span_2, total_frames, view_start);

        assert_eq!(effective_start_1, effective_start_2);
        assert_eq!(zoom_span_1, zoom_span_2);
    }

    #[test]
    fn ruler_labels_get_more_space_when_width_or_ui_text_grows() {
        let wide = ruler_label_step(1000, 1200.0, 50.0);
        let narrow = ruler_label_step(1000, 600.0, 50.0);
        let scaled_text = ruler_label_step(1000, 1200.0, 100.0);

        assert!(narrow > wide);
        assert!(scaled_text > wide);
    }

    #[test]
    fn ruler_ticks_align_to_time_intervals_across_scroll_positions() {
        assert_eq!(ruler_tick_frames(3, 27, 10), vec![10, 20, 30]);
        assert_eq!(ruler_tick_frames(40, 20, 10), vec![40, 50, 60]);
        assert_eq!(ruler_tick_frames(u32::MAX - 2, 10, 5), vec![u32::MAX]);
    }

    #[test]
    fn timecode_label_uses_composition_frame_rate() {
        assert_eq!(format_timecode_label(267, 30), "00:00:08:27");
        assert_eq!(format_timecode_label(120, 0), "00:02:00:00");
    }

    #[test]
    fn measured_timecode_labels_do_not_overlap_at_common_widths_and_scales() {
        for pixels_per_point in [1.0, 1.5, 2.0] {
            let context = egui::Context::default();
            context.set_pixels_per_point(pixels_per_point);
            let _ = context.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1600.0, 900.0),
                    )),
                    ..Default::default()
                },
                |context| {
                    crate::ui::theme::configure_ae_theme(context);
                    egui::CentralPanel::default().show(context, |ui| {
                        for (width, start_frame, span, fps) in
                            [(260.0, 37, 900, 24), (720.0, 137, 2400, 30), (1400.0, 59, 12000, 60)]
                        {
                            let font = egui::FontId::new(
                                ui.text_style_height(&egui::TextStyle::Small) * 0.78,
                                egui::FontFamily::Monospace,
                            );
                            let end_frame = start_frame + span;
                            let measured_width = [start_frame, end_frame]
                                .into_iter()
                                .map(|frame| {
                                    ui.painter()
                                        .layout_no_wrap(
                                            format_timecode_label(frame, fps),
                                            font.clone(),
                                            colors::TEXT_SECONDARY,
                                        )
                                        .size()
                                        .x
                                })
                                .fold(0.0_f32, f32::max);
                            let step = ruler_label_step(span, width, measured_width);
                            let ticks = ruler_tick_frames(start_frame, span, step);
                            let mut previous: Option<(f32, f32)> = None;
                            for frame in ticks {
                                let label_width = ui
                                    .painter()
                                    .layout_no_wrap(
                                        format_timecode_label(frame, fps),
                                        font.clone(),
                                        colors::TEXT_SECONDARY,
                                    )
                                    .size()
                                    .x;
                                let center_x =
                                    (frame - start_frame) as f32 / span as f32 * width;
                                if let Some((previous_x, previous_width)) = previous {
                                    assert!(
                                        center_x - previous_x
                                            >= (label_width + previous_width) * 0.5 + 7.9,
                                        "labels overlap at scale {pixels_per_point}, width {width}, frames {start_frame}–{end_frame}, step {step}"
                                    );
                                }
                                previous = Some((center_x, label_width));
                            }
                        }
                    });
                },
            );
        }
    }
}
