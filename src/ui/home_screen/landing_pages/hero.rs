use super::*;

pub(in crate::ui::home_screen) fn draw_landing_hero(ui: &mut egui::Ui, rect: egui::Rect) {
    let scale = ui
        .ctx()
        .native_pixels_per_point()
        .unwrap_or(1.0)
        .recip()
        .clamp(0.5, 1.0);
    let width_scale = (rect.width() / (1217.0 * scale)).max(0.45);
    let height_scale = (rect.height() / (300.0 * scale)).max(0.5);
    let painter = ui.painter().with_clip_rect(rect);
    let curve = |start: egui::Pos2, c1: egui::Pos2, c2: egui::Pos2, end: egui::Pos2| {
        (0..=48)
            .map(move |index| {
                let t = index as f32 / 48.0;
                let u = 1.0 - t;
                egui::pos2(
                    u * u * u * start.x
                        + 3.0 * u * u * t * c1.x
                        + 3.0 * u * t * t * c2.x
                        + t * t * t * end.x,
                    u * u * u * start.y
                        + 3.0 * u * u * t * c1.y
                        + 3.0 * u * t * t * c2.y
                        + t * t * t * end.y,
                )
            })
            .collect::<Vec<_>>()
    };
    let start = egui::pos2(
        rect.left() + rect.width() * 0.405,
        rect.bottom() + 15.0 * scale * height_scale,
    );
    let end = egui::pos2(
        rect.right() + 20.0 * scale,
        rect.top() + rect.height() * 0.267,
    );
    for (delta, width, color) in [
        (0.0_f32, 3.0_f32, egui::Color32::from_rgb(123, 43, 25)),
        (
            -95.0_f32 * scale * width_scale,
            12.0_f32,
            egui::Color32::from_rgb(181, 53, 22),
        ),
        (
            -210.0_f32 * scale * width_scale,
            40.0_f32,
            egui::Color32::from_rgb(255, 79, 22),
        ),
    ] {
        let points = curve(
            egui::pos2(start.x - delta, start.y),
            egui::pos2(
                rect.left() + rect.width() * 0.64 - delta,
                rect.top() + rect.height() * 0.44,
            ),
            egui::pos2(
                rect.left() + rect.width() * 0.78 - delta,
                rect.top() + rect.height() * 0.143,
            ),
            egui::pos2(end.x - delta * 0.16, end.y),
        );
        painter.add(egui::Shape::line(
            points,
            egui::Stroke::new(
                width * scale * width_scale * height_scale,
                color.linear_multiply(0.72),
            ),
        ));
    }
}
