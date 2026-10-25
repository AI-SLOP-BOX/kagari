use super::*;

pub(in crate::ui::home_screen) fn landing_button_rect(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    label: &str,
    icon: &'static str,
    accent: bool,
) -> egui::Response {
    let scale = (ui.ctx().screen_rect().width() / 1584.0).max(1.0);
    let response = ui.interact(rect, ui.next_auto_id(), egui::Sense::click());
    let painter = ui.painter().with_clip_rect(rect);
    let fill = if accent {
        if response.hovered() {
            egui::Color32::from_rgb(255, 105, 48)
        } else {
            egui::Color32::from_rgb(255, 79, 22)
        }
    } else if response.hovered() {
        colors::BG_HOVER
    } else {
        egui::Color32::from_rgb(15, 23, 26)
    };
    painter.rect_filled(rect, 7.0 * scale, fill);
    painter.rect_stroke(
        rect,
        7.0 * scale,
        egui::Stroke::new(
            1.0 * scale,
            if accent { fill } else { colors::BORDER_MEDIUM },
        ),
    );
    let icon_size = 18.0 * scale;
    crate::ui::icons::render_svg_at(
        ui,
        format!("landing-cta-{label}"),
        icon,
        egui::vec2(icon_size, icon_size),
        egui::Color32::WHITE,
        egui::pos2(
            rect.left() + 18.0 * scale,
            rect.center().y - icon_size * 0.5,
        ),
    );
    let text_center = egui::pos2(rect.center().x + 9.0 * scale, rect.center().y);
    painter.text(
        text_center,
        egui::Align2::CENTER_CENTER,
        crate::ui::localization::tr(label),
        egui::FontId::proportional(13.0 * scale),
        if accent {
            egui::Color32::WHITE
        } else {
            colors::TEXT_PRIMARY
        },
    );
    response.on_hover_text(crate::ui::localization::tr(label))
}
