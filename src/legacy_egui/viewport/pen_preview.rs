use super::*;

pub(super) fn draw(
    app: &KagariApp,
    ui: &mut egui::Ui,
    viewport_response: &egui::Response,
    geometry: ViewportGeometry,
) {
    let ViewportGeometry {
        origin_x,
        origin_y,
        draw_w,
        draw_h,
        comp_w,
        comp_h,
    } = geometry;
    if app.active_tool == crate::ui::toolbar::ActiveTool::Pen && !app.pen_points.is_empty() {
        let painter = ui.painter();
        let pen_stroke = egui::Stroke::new(1.5_f32, crate::ui::theme::colors::ACCENT_CYAN);
        let to_screen = |v: [f32; 2]| {
            egui::pos2(
                origin_x + v[0] / comp_w * draw_w,
                origin_y + v[1] / comp_h * draw_h,
            )
        };
        let has_any_tangents = app
            .pen_tangents
            .iter()
            .any(|(o, i)| o[0] != 0.0 || o[1] != 0.0 || i[0] != 0.0 || i[1] != 0.0);

        if has_any_tangents {
            // Draw cubic Bezier curves between points
            for seg in app.pen_points.windows(2) {
                let i0 = app
                    .pen_points
                    .iter()
                    .position(|p| p[0] == seg[0][0] && p[1] == seg[0][1])
                    .unwrap_or(0);
                let i1 = app
                    .pen_points
                    .iter()
                    .position(|p| p[0] == seg[1][0] && p[1] == seg[1][1])
                    .unwrap_or(0);
                let p0 = to_screen(seg[0]);
                let p1 = to_screen(seg[1]);
                let t_out = app
                    .pen_tangents
                    .get(i0)
                    .map(|(o, _)| *o)
                    .unwrap_or([0.0, 0.0]);
                let t_in = app
                    .pen_tangents
                    .get(i1)
                    .map(|(_, i)| *i)
                    .unwrap_or([0.0, 0.0]);
                let c0 = egui::pos2(
                    p0.x + t_out[0] / comp_w * draw_w,
                    p0.y + t_out[1] / comp_h * draw_h,
                );
                let c1 = egui::pos2(
                    p1.x + t_in[0] / comp_w * draw_w,
                    p1.y + t_in[1] / comp_h * draw_h,
                );
                // Approximate cubic Bezier with line segments
                let steps = 12;
                let mut prev = p0;
                for s in 1..=steps {
                    let t = s as f32 / steps as f32;
                    let t2 = t * t;
                    let t3 = t2 * t;
                    let mt = 1.0 - t;
                    let mt2 = mt * mt;
                    let mt3 = mt2 * mt;
                    let x = mt3 * p0.x + 3.0 * mt2 * t * c0.x + 3.0 * mt * t2 * c1.x + t3 * p1.x;
                    let y = mt3 * p0.y + 3.0 * mt2 * t * c0.y + 3.0 * mt * t2 * c1.y + t3 * p1.y;
                    let cur = egui::pos2(x, y);
                    painter.line_segment([prev, cur], pen_stroke);
                    prev = cur;
                }
            }
            // Closing segment
            if app.pen_points.len() >= 3 {
                let first = app.pen_points[0];
                let last = app.pen_points[app.pen_points.len() - 1];
                let p0 = to_screen(last);
                let p1 = to_screen(first);
                let t_out = app
                    .pen_tangents
                    .last()
                    .map(|(o, _)| *o)
                    .unwrap_or([0.0, 0.0]);
                let t_in = app
                    .pen_tangents
                    .first()
                    .map(|(_, i)| *i)
                    .unwrap_or([0.0, 0.0]);
                let c0 = egui::pos2(
                    p0.x + t_out[0] / comp_w * draw_w,
                    p0.y + t_out[1] / comp_h * draw_h,
                );
                let c1 = egui::pos2(
                    p1.x + t_in[0] / comp_w * draw_w,
                    p1.y + t_in[1] / comp_h * draw_h,
                );
                let steps = 12;
                let mut prev = p0;
                for s in 1..=steps {
                    let t = s as f32 / steps as f32;
                    let t2 = t * t;
                    let t3 = t2 * t;
                    let mt = 1.0 - t;
                    let mt2 = mt * mt;
                    let mt3 = mt2 * mt;
                    let x = mt3 * p0.x + 3.0 * mt2 * t * c0.x + 3.0 * mt * t2 * c1.x + t3 * p1.x;
                    let y = mt3 * p0.y + 3.0 * mt2 * t * c0.y + 3.0 * mt * t2 * c1.y + t3 * p1.y;
                    let cur = egui::pos2(x, y);
                    painter.line_segment(
                        [prev, cur],
                        egui::Stroke::new(
                            1.0_f32,
                            crate::ui::theme::colors::ACCENT_CYAN.linear_multiply(0.4),
                        ),
                    );
                    prev = cur;
                }
            }
        } else {
            // No tangents: draw straight lines (original behavior)
            for pair in app.pen_points.windows(2) {
                painter.line_segment([to_screen(pair[0]), to_screen(pair[1])], pen_stroke);
            }
            if app.pen_points.len() >= 3 {
                let first = app.pen_points[0];
                let last = app.pen_points[app.pen_points.len() - 1];
                painter.line_segment(
                    [to_screen(last), to_screen(first)],
                    egui::Stroke::new(
                        1.0_f32,
                        crate::ui::theme::colors::ACCENT_CYAN.linear_multiply(0.4),
                    ),
                );
            }
        }

        // Draw vertex dots
        for (i, p) in app.pen_points.iter().enumerate() {
            let sp = to_screen(*p);
            painter.circle_filled(sp, 3.5, crate::ui::theme::colors::ACCENT_CYAN);
            painter.circle_stroke(sp, 3.5, egui::Stroke::new(1.0_f32, egui::Color32::BLACK));
            // Draw tangent handles if they exist
            if let Some((t_out, t_in)) = app.pen_tangents.get(i) {
                if t_out[0] != 0.0 || t_out[1] != 0.0 {
                    let h_out = egui::pos2(
                        sp.x + t_out[0] / comp_w * draw_w,
                        sp.y + t_out[1] / comp_h * draw_h,
                    );
                    painter.line_segment(
                        [sp, h_out],
                        egui::Stroke::new(1.0_f32, crate::ui::theme::colors::ACCENT_CYAN),
                    );
                    painter.circle_filled(h_out, 3.0, crate::ui::theme::colors::ACCENT_CYAN);
                }
                if t_in[0] != 0.0 || t_in[1] != 0.0 {
                    let h_in = egui::pos2(
                        sp.x + t_in[0] / comp_w * draw_w,
                        sp.y + t_in[1] / comp_h * draw_h,
                    );
                    painter.line_segment(
                        [sp, h_in],
                        egui::Stroke::new(1.0_f32, crate::ui::theme::colors::ACCENT_CYAN),
                    );
                    painter.circle_filled(h_in, 3.0, crate::ui::theme::colors::ACCENT_CYAN);
                }
            }
        }
        if viewport_response.hovered() {
            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Crosshair);
        }
    }
}
