#![allow(clippy::too_many_arguments)]
use super::*;

pub(crate) fn draw_timeline(
    scene: &mut Scene,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    frame: u32,
    work_area: Option<(u32, u32)>,
    duration_frames: u32,
    timeline_start: f64,
    visible_frames: f64,
    vertical_scroll: f64,
    layers: &[kagari_vfx::core::timeline::Layer],
    selected_layer_id: Option<&str>,
    selected_layer_marker_frame: Option<u32>,
    waveforms: &std::collections::HashMap<String, std::sync::Arc<CachedAudioWaveform>>,
    fps: u32,
) {
    let bounds = Rect::new(x, y, x + width, y + height);
    scene.fill(
        Fill::NonZero,
        Affine::IDENTITY,
        Color::from_rgb8(13, 15, 17),
        None,
        &bounds,
    );
    let visible_frames = visible_frames.max(1.0);
    let tracks_top = y + 22.0;
    let track_height = 28.0;
    let grid = Stroke::new(1.0);

    for tick in 0..=5 {
        let tick_x = x + width * tick as f64 / 5.0;
        scene.stroke(
            &grid,
            Affine::IDENTITY,
            Color::from_rgb8(38, 42, 47),
            None,
            &Line::new((tick_x, y), (tick_x, y + height)),
        );
    }
    let first_visible_row = (vertical_scroll / track_height).floor() as usize;
    let last_visible_row =
        ((vertical_scroll + (height - 22.0).max(0.0)) / track_height).ceil() as usize;
    for (index, layer) in layers
        .iter()
        .enumerate()
        .skip(first_visible_row)
        .take(last_visible_row.saturating_sub(first_visible_row))
    {
        let row_y = tracks_top + index as f64 * track_height - vertical_scroll;
        scene.stroke(
            &grid,
            Affine::IDENTITY,
            Color::from_rgb8(32, 36, 40),
            None,
            &Line::new((x, row_y), (x + width, row_y)),
        );
        let start =
            (f64::from(layer.in_frame.min(duration_frames)) - timeline_start) / visible_frames;
        let end = (f64::from(
            layer
                .out_frame
                .max(layer.in_frame.saturating_add(1))
                .min(duration_frames),
        ) - timeline_start)
            / visible_frames;
        if end <= 0.0 || start >= 1.0 {
            continue;
        }
        let clip_start_x = (x + width * start).max(x);
        let clip_end_x = (x + width * end).min(x + width).max(clip_start_x + 4.0);
        let clip = RoundedRect::new(
            clip_start_x,
            row_y + 3.0,
            clip_end_x,
            (row_y + track_height - 3.0).max(row_y + 8.0),
            3.0,
        );
        let selected = selected_layer_id == Some(layer.id.as_str());
        let color = if selected {
            Color::from_rgba8(220, 105, 48, 210)
        } else {
            if layer.label == kagari_vfx::core::timeline::LabelColor::None {
                Color::from_rgba8(83, 94, 102, if layer.visible { 175 } else { 75 })
            } else {
                let [red, green, blue] = layer.label.to_rgb();
                Color::from_rgba8(
                    (red.clamp(0.0, 1.0) * 255.0) as u8,
                    (green.clamp(0.0, 1.0) * 255.0) as u8,
                    (blue.clamp(0.0, 1.0) * 255.0) as u8,
                    if layer.visible { 175 } else { 75 },
                )
            }
        };
        scene.fill(Fill::NonZero, Affine::IDENTITY, color, None, &clip);
        scene.stroke(
            &Stroke::new(1.0),
            Affine::IDENTITY,
            Color::from_rgba8(175, 185, 190, if layer.visible { 110 } else { 55 }),
            None,
            &clip,
        );
        draw_timeline_waveform(
            scene,
            layer,
            x,
            width,
            row_y + 3.0,
            row_y + track_height - 3.0,
            clip_start_x,
            clip_end_x,
            timeline_start,
            visible_frames,
            fps,
            waveforms,
        );
        if selected {
            for edge_x in [clip_start_x + 2.0, clip_end_x - 2.0] {
                scene.stroke(
                    &Stroke::new(2.0),
                    Affine::IDENTITY,
                    Color::from_rgb8(255, 222, 192),
                    None,
                    &Line::new((edge_x, row_y + 7.0), (edge_x, row_y + track_height - 7.0)),
                );
            }
        }

        // Show transform keyframes directly on the layer strip. Multiple
        // animated channels at the same frame share one marker so the strip
        // remains legible at dense timeline scales.
        for keyframe_frame in layer_keyframe_frames(layer) {
            if keyframe_frame < layer.in_frame || keyframe_frame > layer.out_frame {
                continue;
            }
            let marker_x = x + width
                * (f64::from(keyframe_frame.min(duration_frames)) - timeline_start)
                / visible_frames;
            let center_y = row_y + track_height * 0.5;
            let mut diamond = BezPath::new();
            diamond.move_to((marker_x, center_y - 4.0));
            diamond.line_to((marker_x + 4.0, center_y));
            diamond.line_to((marker_x, center_y + 4.0));
            diamond.line_to((marker_x - 4.0, center_y));
            diamond.close_path();
            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                Color::from_rgb8(255, 218, 174),
                None,
                &diamond,
            );
        }

        for marker in &layer.markers {
            if marker.frame < layer.in_frame || marker.frame > layer.out_frame {
                continue;
            }
            let marker_x = x + width
                * (f64::from(marker.frame.min(duration_frames)) - timeline_start)
                / visible_frames;
            if marker_x < x || marker_x > x + width {
                continue;
            }
            let marker_y = row_y + 7.0;
            let mut pin = BezPath::new();
            pin.move_to((marker_x, marker_y - 4.5));
            pin.line_to((marker_x + 4.0, marker_y + 3.5));
            pin.line_to((marker_x - 4.0, marker_y + 3.5));
            pin.close_path();
            let color = if selected_layer_id == Some(layer.id.as_str())
                && selected_layer_marker_frame == Some(marker.frame)
            {
                Color::from_rgb8(228, 255, 246)
            } else {
                Color::from_rgb8(87, 224, 190)
            };
            scene.fill(Fill::NonZero, Affine::IDENTITY, color, None, &pin);
        }
    }

    if let Some((start_frame, end_frame)) = work_area {
        let start_x = x + width * (f64::from(start_frame) - timeline_start) / visible_frames;
        let end_x =
            x + width * (f64::from(end_frame.saturating_add(1)) - timeline_start) / visible_frames;
        let visible_left = start_x.max(x);
        let visible_right = end_x.min(x + width);
        if visible_right > visible_left {
            let work_area_rect = Rect::new(visible_left, y, visible_right, y + height);
            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                Color::from_rgba8(245, 140, 75, 20),
                None,
                &work_area_rect,
            );
        }
        for edge_x in [start_x, end_x] {
            if (x..=x + width).contains(&edge_x) {
                scene.stroke(
                    &Stroke::new(1.5),
                    Affine::IDENTITY,
                    Color::from_rgba8(245, 140, 75, 210),
                    None,
                    &Line::new((edge_x, y), (edge_x, y + height)),
                );
                let handle = Rect::new(edge_x - 4.0, y + 1.0, edge_x + 4.0, y + 10.0);
                scene.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    Color::from_rgb8(245, 140, 75),
                    None,
                    &handle,
                );
            }
        }
    }

    let playhead_x = x + width * (f64::from(frame.min(duration_frames.max(1))) - timeline_start)
        / visible_frames;
    if playhead_x >= x && playhead_x <= x + width {
        scene.stroke(
            &Stroke::new(1.8),
            Affine::IDENTITY,
            Color::from_rgb8(255, 116, 28),
            None,
            &Line::new((playhead_x, y), (playhead_x, y + height)),
        );
        let mut marker = BezPath::new();
        marker.move_to((playhead_x - 5.0, y));
        marker.line_to((playhead_x + 5.0, y));
        marker.line_to((playhead_x, y + 8.0));
        marker.close_path();
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            Color::from_rgb8(255, 116, 28),
            None,
            &marker,
        );
    }
}
