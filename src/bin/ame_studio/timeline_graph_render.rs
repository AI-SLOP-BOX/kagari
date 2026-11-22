#![allow(clippy::too_many_arguments)]
use super::*;

pub(crate) fn draw_value_graph(
    scene: &mut Scene,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    frame: u32,
    duration_frames: u32,
    timeline_start: f64,
    visible_frames: f64,
    channel: GraphChannel,
    layer: Option<&kagari_vfx::core::timeline::Layer>,
    camera: Option<&kagari_vfx::core::timeline::Camera3D>,
    light: Option<&kagari_vfx::core::timeline::Light3D>,
    mask_id: Option<&str>,
    effect_id: Option<&str>,
    effect_parameter_id: Option<&str>,
    keys: &[(u32, f32)],
) {
    let bounds = Rect::new(x, y, x + width, y + height);
    scene.fill(
        Fill::NonZero,
        Affine::IDENTITY,
        Color::from_rgb8(13, 15, 17),
        None,
        &bounds,
    );
    let left = x + 14.0;
    let right = (x + width - 14.0).max(left + 1.0);
    let top = y + 12.0;
    let bottom = (y + height - 12.0).max(top + 1.0);
    let (min_value, max_value) = graph_value_range(keys);
    let value_span = f64::from((max_value - min_value).max(f32::EPSILON));
    let plot_height = bottom - top;
    let plot_width = right - left;
    let grid = Stroke::new(1.0);
    for division in 0..=4 {
        let fraction = division as f64 / 4.0;
        let grid_y = top + plot_height * fraction;
        let grid_x = left + plot_width * fraction;
        scene.stroke(
            &grid,
            Affine::IDENTITY,
            Color::from_rgb8(39, 44, 49),
            None,
            &Line::new((left, grid_y), (right, grid_y)),
        );
        scene.stroke(
            &grid,
            Affine::IDENTITY,
            Color::from_rgb8(31, 36, 41),
            None,
            &Line::new((grid_x, top), (grid_x, bottom)),
        );
    }
    if let (Some(layer), Some(first), Some(last)) = (layer, keys.first(), keys.last()) {
        if last.0 > first.0 {
            let sample_count = (last.0 - first.0).clamp(2, 120);
            let mut curve = BezPath::new();
            let mut area = BezPath::new();

            for sample in 0..=sample_count {
                let sample_frame = first.0 + (last.0 - first.0) * sample / sample_count;
                let value = graph_channel_value(
                    layer,
                    camera,
                    light,
                    mask_id,
                    effect_id,
                    effect_parameter_id,
                    channel,
                    sample_frame,
                );
                let px = left
                    + (f64::from(sample_frame) - timeline_start) / visible_frames.max(1.0)
                        * plot_width;
                let py =
                    bottom - (f64::from(value) - f64::from(min_value)) / value_span * plot_height;
                if sample == 0 {
                    curve.move_to((px, py));
                    area.move_to((px, bottom));
                    area.line_to((px, py));
                } else {
                    curve.line_to((px, py));
                    area.line_to((px, py));
                    if sample == sample_count {
                        area.line_to((px, bottom));
                        area.close_path();
                    }
                }
            }
            // 1. Soft glowing area fill under curve for instant visual comprehension of velocity/amplitude
            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                Color::from_rgba8(255, 135, 55, 28),
                None,
                &area,
            );
            // 2. Outer curve glow
            scene.stroke(
                &Stroke::new(4.0),
                Affine::IDENTITY,
                Color::from_rgba8(255, 135, 55, 45),
                None,
                &curve,
            );
            // 3. Crisp vibrant main curve
            scene.stroke(
                &Stroke::new(2.2),
                Affine::IDENTITY,
                Color::from_rgb8(255, 148, 68),
                None,
                &curve,
            );
        }
    }
    // Keyframe diamonds with high-contrast inner core and accent border
    for (key_frame, value) in keys {
        let px =
            left + (f64::from(*key_frame) - timeline_start) / visible_frames.max(1.0) * plot_width;
        if px < left - 6.0 || px > right + 6.0 {
            continue;
        }
        let py = bottom - (f64::from(*value) - f64::from(min_value)) / value_span * plot_height;
        // Dynamic tangent handles if easing/bezier is active on this keyframe
        let is_bezier = layer
            .and_then(|l| {
                graph_interpolation_label(
                    l,
                    camera,
                    light,
                    mask_id,
                    effect_id,
                    effect_parameter_id,
                    channel,
                    *key_frame,
                )
            })
            .is_some_and(|label| label == "イージング");

        if is_bezier {
            let handle_len = 22.0;
            // In/out tangent line with subtle glow
            scene.stroke(
                &Stroke::new(1.2),
                Affine::IDENTITY,
                Color::from_rgba8(255, 180, 70, 160),
                None,
                &Line::new((px - handle_len, py), (px + handle_len, py)),
            );
            // In handle circle (left)
            let in_handle = Circle::new((px - handle_len, py), 2.5);
            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                Color::from_rgb8(255, 200, 100),
                None,
                &in_handle,
            );
            scene.stroke(
                &Stroke::new(0.8),
                Affine::IDENTITY,
                Color::from_rgb8(13, 15, 17),
                None,
                &in_handle,
            );

            // Out handle circle (right)
            let out_handle = Circle::new((px + handle_len, py), 2.5);
            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                Color::from_rgb8(255, 200, 100),
                None,
                &out_handle,
            );
            scene.stroke(
                &Stroke::new(0.8),
                Affine::IDENTITY,
                Color::from_rgb8(13, 15, 17),
                None,
                &out_handle,
            );
        } else {
            // Stub line for linear/hold
            scene.stroke(
                &Stroke::new(1.0),
                Affine::IDENTITY,
                Color::from_rgba8(160, 170, 185, 70),
                None,
                &Line::new((px - 6.0, py), (px + 6.0, py)),
            );
        }

        let mut diamond = BezPath::new();
        diamond.move_to((px, py - 5.5));
        diamond.line_to((px + 5.5, py));
        diamond.line_to((px, py + 5.5));
        diamond.line_to((px - 5.5, py));
        diamond.close_path();
        // Drop shadow for keyframe diamond
        scene.stroke(
            &Stroke::new(2.0),
            Affine::IDENTITY,
            Color::from_rgba8(10, 11, 13, 220),
            None,
            &diamond,
        );
        let diamond_color = if is_bezier {
            Color::from_rgb8(255, 215, 140)
        } else {
            Color::from_rgb8(210, 225, 240)
        };
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            diamond_color,
            None,
            &diamond,
        );
    }
    let playhead_x = left
        + (f64::from(frame.min(duration_frames)) - timeline_start) / visible_frames.max(1.0)
            * plot_width;
    if playhead_x >= left && playhead_x <= right {
        scene.stroke(
            &Stroke::new(1.5),
            Affine::IDENTITY,
            Color::from_rgb8(255, 116, 28),
            None,
            &Line::new((playhead_x, top), (playhead_x, bottom)),
        );
    }
}
