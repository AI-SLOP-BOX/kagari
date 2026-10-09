use crate::core::timeline::Layer;
use crate::ui::theme::colors;
use eframe::egui;

/// Timeline graph workspace: animation curves only, never a dependency/node graph.
/// Each property has its own value range so Position, Rotation and Opacity stay readable.
pub fn draw_animation_graph_editor(
    selected_property: &mut Option<String>,
    ui: &mut egui::Ui,
    duration_frames: u32,
    layer: &Layer,
    current_frame: &mut u32,
) {
    struct Track {
        key: String,
        label: &'static str,
        color: egui::Color32,
        values: Vec<(u32, f32)>,
        animated: bool,
    }
    let total = duration_frames.max(1);
    let mut tracks = Vec::with_capacity(5);
    let mut add = |key: &str,
                   label: &'static str,
                   color: egui::Color32,
                   mut values: Vec<(u32, f32)>,
                   animated: bool| {
        if values.is_empty() {
            values.push((0, 0.0));
            values.push((total, 0.0));
        }
        tracks.push(Track {
            key: key.into(),
            label,
            color,
            values,
            animated,
        });
    };
    let v2 = |a: &crate::core::property::Animatable<[f32; 2]>, axis: usize| -> Vec<(u32, f32)> {
        a.keyframes()
            .map(|k| {
                k.iter()
                    .map(|x| (x.frame.min(total), x.value[axis]))
                    .collect()
            })
            .unwrap_or_default()
    };
    let s = |a: &crate::core::property::Animatable<f32>| -> Vec<(u32, f32)> {
        a.keyframes()
            .map(|k| k.iter().map(|x| (x.frame.min(total), x.value)).collect())
            .unwrap_or_default()
    };
    let px = v2(&layer.transform.position, 0);
    let py = v2(&layer.transform.position, 1);
    let sx = v2(&layer.transform.scale, 0);
    let rot = s(&layer.transform.rotation);
    let op = s(&layer.transform.opacity);
    add(
        "Position X",
        "Position X",
        colors::ACCENT_BLUE,
        if px.is_empty() {
            vec![
                (0, layer.transform.position.evaluate(0)[0]),
                (total, layer.transform.position.evaluate(total)[0]),
            ]
        } else {
            px
        },
        !v2(&layer.transform.position, 0).is_empty(),
    );
    add(
        "Position Y",
        "Position Y",
        egui::Color32::from_rgb(92, 200, 180),
        if py.is_empty() {
            vec![
                (0, layer.transform.position.evaluate(0)[1]),
                (total, layer.transform.position.evaluate(total)[1]),
            ]
        } else {
            py
        },
        !v2(&layer.transform.position, 1).is_empty(),
    );
    add(
        "Scale X",
        "Scale",
        egui::Color32::from_rgb(172, 132, 255),
        if sx.is_empty() {
            vec![
                (0, layer.transform.scale.evaluate(0)[0]),
                (total, layer.transform.scale.evaluate(total)[0]),
            ]
        } else {
            sx
        },
        !v2(&layer.transform.scale, 0).is_empty(),
    );
    add(
        "Rotation",
        "Rotation",
        egui::Color32::from_rgb(255, 173, 92),
        if rot.is_empty() {
            vec![
                (0, layer.transform.rotation.evaluate(0)),
                (total, layer.transform.rotation.evaluate(total)),
            ]
        } else {
            rot
        },
        !s(&layer.transform.rotation).is_empty(),
    );
    add(
        "Opacity",
        "Opacity",
        egui::Color32::from_rgb(242, 112, 148),
        if op.is_empty() {
            vec![
                (0, layer.transform.opacity.evaluate(0)),
                (total, layer.transform.opacity.evaluate(total)),
            ]
        } else {
            op
        },
        !s(&layer.transform.opacity).is_empty(),
    );

    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Graph Editor").size(13.0).strong());
            ui.label(
                egui::RichText::new("Animation Curves")
                    .small()
                    .color(colors::TEXT_MUTED),
            );
            ui.separator();
            ui.label(
                egui::RichText::new("X: time   Y: value")
                    .small()
                    .color(colors::TEXT_MUTED),
            );
        });
        ui.add_space(3.0);
        let (toolbar, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 25.0), egui::Sense::hover());
        ui.painter().line_segment(
            [toolbar.left_bottom(), toolbar.right_bottom()],
            egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
        );
        ui.painter().text(
            toolbar.left_center() + egui::vec2(8.0, 0.0),
            egui::Align2::LEFT_CENTER,
            "Value Graph   ·   Bezier",
            egui::FontId::proportional(11.0),
            colors::TEXT_SECONDARY,
        );
        ui.painter().text(
            toolbar.right_center() - egui::vec2(8.0, 0.0),
            egui::Align2::RIGHT_CENTER,
            format!(
                "{} animated tracks",
                tracks.iter().filter(|x| x.animated).count()
            ),
            egui::FontId::proportional(10.0),
            colors::TEXT_MUTED,
        );

        let row_h = ((ui.available_height() - 8.0) / tracks.len().max(1) as f32).clamp(48.0, 76.0);
        let graph = egui::Rect::from_min_size(
            ui.cursor().min,
            egui::vec2(ui.available_width(), row_h * tracks.len() as f32),
        );
        ui.allocate_rect(graph, egui::Sense::hover());
        let left = graph.left() + 116.0;
        let width = (graph.width() - 124.0).max(40.0);
        let x_of = |frame: u32| left + frame.min(total) as f32 / total as f32 * width;
        let playhead = x_of(*current_frame);
        ui.painter().line_segment(
            [
                egui::pos2(playhead, graph.top()),
                egui::pos2(playhead, graph.bottom()),
            ],
            egui::Stroke::new(1.0_f32, colors::ACCENT_ORANGE),
        );
        for (index, track) in tracks.iter().enumerate() {
            let row = egui::Rect::from_min_size(
                egui::pos2(graph.left(), graph.top() + index as f32 * row_h),
                egui::vec2(graph.width(), row_h),
            );
            let min = track
                .values
                .iter()
                .map(|(_, v)| *v)
                .fold(f32::INFINITY, f32::min);
            let max = track
                .values
                .iter()
                .map(|(_, v)| *v)
                .fold(f32::NEG_INFINITY, f32::max);
            let pad = (max - min).abs().max(1.0) * 0.12;
            let lo = min - pad;
            let hi = max + pad;
            let y_of = |v: f32| {
                row.bottom()
                    - 10.0
                    - ((v - lo) / (hi - lo).max(0.01)).clamp(0.0, 1.0) * (row.height() - 20.0)
            };
            if index > 0 {
                ui.painter().line_segment(
                    [row.left_top(), row.right_top()],
                    egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
                );
            }
            ui.painter().text(
                egui::pos2(row.left() + 8.0, row.center().y - 5.0),
                egui::Align2::LEFT_CENTER,
                track.label,
                egui::FontId::proportional(11.0),
                if selected_property.as_deref() == Some(track.key.as_str()) {
                    colors::TEXT_PRIMARY
                } else {
                    colors::TEXT_SECONDARY
                },
            );
            ui.painter().text(
                egui::pos2(row.left() + 8.0, row.center().y + 11.0),
                egui::Align2::LEFT_CENTER,
                if track.animated {
                    "animated"
                } else {
                    "constant"
                },
                egui::FontId::proportional(9.0),
                if track.animated {
                    track.color
                } else {
                    colors::TEXT_MUTED
                },
            );
            for division in 0..=4 {
                let x = left + division as f32 / 4.0 * width;
                ui.painter().line_segment(
                    [egui::pos2(x, row.top()), egui::pos2(x, row.bottom())],
                    egui::Stroke::new(0.5_f32, colors::GRID_LINE),
                );
                if index == tracks.len() - 1 {
                    ui.painter().text(
                        egui::pos2(x + 2.0, row.bottom() - 2.0),
                        egui::Align2::LEFT_BOTTOM,
                        format!("{}f", total * division / 4),
                        egui::FontId::proportional(9.0),
                        colors::TEXT_MUTED,
                    );
                }
            }
            for division in 0..=2 {
                let y = row.bottom() - 10.0 - division as f32 / 2.0 * (row.height() - 20.0);
                ui.painter().line_segment(
                    [egui::pos2(left, y), egui::pos2(graph.right(), y)],
                    egui::Stroke::new(0.5_f32, colors::GRID_LINE),
                );
            }
            let points: Vec<_> = track
                .values
                .iter()
                .map(|(f, v)| egui::pos2(x_of(*f), y_of(*v)))
                .collect();
            for pair in points.windows(2) {
                let p0 = pair[0];
                let p3 = pair[1];
                let span = (p3.x - p0.x).max(8.0);
                let p1 = egui::pos2(p0.x + span * 0.36, p0.y);
                let p2 = egui::pos2(p3.x - span * 0.36, p3.y);
                let mut previous = p0;
                for step in 1..=18 {
                    let t = step as f32 / 18.0;
                    let q = 1.0 - t;
                    let next = egui::pos2(
                        q.powi(3) * p0.x
                            + 3.0 * q.powi(2) * t * p1.x
                            + 3.0 * q * t.powi(2) * p2.x
                            + t.powi(3) * p3.x,
                        q.powi(3) * p0.y
                            + 3.0 * q.powi(2) * t * p1.y
                            + 3.0 * q * t.powi(2) * p2.y
                            + t.powi(3) * p3.y,
                    );
                    ui.painter().line_segment(
                        [previous, next],
                        egui::Stroke::new(
                            if track.animated { 1.8_f32 } else { 1.0_f32 },
                            track.color,
                        ),
                    );
                    previous = next;
                }
            }
            for (i, point) in points.iter().enumerate() {
                if track.animated {
                    let value = track.values[i].1;
                    let prev = track
                        .values
                        .get(i.saturating_sub(1))
                        .map(|x| x.1)
                        .unwrap_or(value);
                    let next = track
                        .values
                        .get((i + 1).min(track.values.len() - 1))
                        .map(|x| x.1)
                        .unwrap_or(value);
                    let span = 14.0_f32.min(width * 0.08);
                    let hout = egui::pos2(point.x + span, y_of((value + next) * 0.5));
                    let hin = egui::pos2(point.x - span, y_of((value + prev) * 0.5));
                    ui.painter().line_segment(
                        [*point, hout],
                        egui::Stroke::new(0.8_f32, track.color.linear_multiply(0.65)),
                    );
                    ui.painter().line_segment(
                        [*point, hin],
                        egui::Stroke::new(0.8_f32, track.color.linear_multiply(0.65)),
                    );
                    ui.painter()
                        .circle_filled(hout, 2.5, track.color.linear_multiply(0.75));
                    ui.painter()
                        .circle_filled(hin, 2.5, track.color.linear_multiply(0.75));
                    ui.painter().circle_filled(
                        *point,
                        4.0,
                        if selected_property.as_deref() == Some(track.key.as_str()) {
                            colors::ACCENT_ORANGE
                        } else {
                            track.color
                        },
                    );
                }
            }
        }
    });
}

pub fn draw_automation_curve(
    ui: &mut egui::Ui,
    curve: &mut crate::core::automation_binding::AutomationCurve,
    changed: &mut bool,
) {
    if curve.points.is_empty() {
        return;
    }
    ui.separator();
    ui.label(egui::RichText::new("Automation Channel").strong());
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 100.0),
        egui::Sense::hover(),
    );
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, crate::ui::theme::colors::BG_DEEPEST);
    let min_time = curve
        .points
        .first()
        .map(|point| point.time.numerator as f32 / point.time.denominator as f32)
        .unwrap_or(0.0);
    let max_time = curve
        .points
        .last()
        .map(|point| point.time.numerator as f32 / point.time.denominator as f32)
        .unwrap_or(min_time + 1.0)
        .max(min_time + f32::EPSILON);
    let min_value = curve
        .points
        .iter()
        .map(|point| point.value as f32)
        .fold(f32::INFINITY, f32::min);
    let max_value = curve
        .points
        .iter()
        .map(|point| point.value as f32)
        .fold(f32::NEG_INFINITY, f32::max)
        .max(min_value + f32::EPSILON);
    let to_screen = |time: f32, value: f32| {
        egui::pos2(
            rect.left()
                + ((time - min_time) / (max_time - min_time)).clamp(0.0, 1.0) * rect.width(),
            rect.bottom()
                - ((value - min_value) / (max_value - min_value)).clamp(0.0, 1.0) * rect.height(),
        )
    };
    let points: Vec<_> = (0..=64)
        .filter_map(|index| {
            let ratio = index as f32 / 64.0;
            let time = min_time + ratio * (max_time - min_time);
            let sample_time = crate::core::unified_time::Time::new(
                (time * 1_000_000.0).round() as i64,
                1_000_000,
            );
            curve
                .sample(sample_time)
                .map(|value| to_screen(time, value as f32))
        })
        .collect();
    for segment in points.windows(2) {
        painter.line_segment(
            [segment[0], segment[1]],
            egui::Stroke::new(2.0_f32, crate::ui::theme::colors::ACCENT_CYAN),
        );
    }
    for point in &curve.points {
        let time = point.time.numerator as f32 / point.time.denominator as f32;
        painter.circle_filled(
            to_screen(time, point.value as f32),
            3.0,
            crate::ui::theme::colors::ACCENT_ORANGE,
        );
    }
    if response.double_clicked() {
        if let Some(pointer) = response.interact_pointer_pos() {
            let normalized_time =
                ((pointer.x - rect.left()) / rect.width().max(1.0)).clamp(0.0, 1.0);
            let normalized_value =
                ((rect.bottom() - pointer.y) / rect.height().max(1.0)).clamp(0.0, 1.0);
            let time = min_time + normalized_time * (max_time - min_time);
            let value = min_value + normalized_value * (max_value - min_value);
            if curve
                .upsert_point(
                    crate::core::unified_time::Time::new(
                        (time * 1_000_000.0).round() as i64,
                        1_000_000,
                    ),
                    value as f64,
                )
                .is_ok()
            {
                *changed = true;
            }
        }
    }
}
