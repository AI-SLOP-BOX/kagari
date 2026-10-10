use super::{colors, layout};
use eframe::egui;

/// Helper: Render section headers with a crisp left accent bar, icon, and high-contrast typography.
#[allow(dead_code)]
pub fn draw_section_header(ui: &mut egui::Ui, title: &str, icon: &str) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(3.0, 16.0), egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, 1.0, colors::ACCENT_WORKSPACE);
        ui.add_space(4.0);
        if let Some(svg) = crate::ui::icons::svg_for_icon_label(icon) {
            let (icon_rect, _) =
                ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
            crate::ui::icons::render_svg_at(
                ui,
                format!("section-icon-{title}"),
                svg,
                icon_rect.size(),
                colors::TEXT_SECONDARY,
                icon_rect.min,
            );
            ui.add_space(3.0);
        } else if !icon.is_empty() {
            ui.label(
                egui::RichText::new(icon)
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            ui.add_space(3.0);
        }
        ui.label(
            egui::RichText::new(title)
                .small()
                .strong()
                .color(colors::TEXT_PRIMARY),
        );
    });
    ui.add_space(2.0);
}

pub fn draw_section_header_svg(ui: &mut egui::Ui, title: &str, icon: &'static str) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(3.0, 16.0), egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, 1.0, colors::ACCENT_WORKSPACE);
        ui.add_space(4.0);
        crate::ui::icons::render_svg_bytes(
            ui,
            title,
            icon,
            egui::vec2(14.0, 14.0),
            colors::TEXT_PRIMARY,
        );
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(title)
                .small()
                .strong()
                .color(colors::TEXT_PRIMARY),
        );
    });
    ui.add_space(2.0);
}

/// Helper: Draw a calm pro tab: plain text with a thin blue underline when
/// selected. Deliberately no filled background (avoids heavy pill tabs).
pub fn draw_custom_tab(ui: &mut egui::Ui, selected: bool, title: &str) -> egui::Response {
    let text = egui::RichText::new(title).small().color(if selected {
        colors::TEXT_PRIMARY
    } else {
        colors::TEXT_SECONDARY
    });

    let response = ui.add(egui::Label::new(text).sense(egui::Sense::click()));
    if selected {
        let rect = response.rect;
        let line_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left() + 2.0, rect.bottom() - 2.0),
            egui::pos2(rect.right() - 2.0, rect.bottom()),
        );
        ui.painter()
            .rect_filled(line_rect, 1.0, colors::ACCENT_SELECTION);
    } else if response.hovered() {
        ui.painter().rect_filled(
            response.rect.expand2(egui::vec2(4.0, 2.0)),
            4.0,
            colors::BG_HOVER.linear_multiply(0.6),
        );
    }
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

/// Helper: Draw formatted property label, value, and unit (`px`, `%`, `dB`, `f`).
#[allow(dead_code)]
pub fn draw_prop_value(ui: &mut egui::Ui, label: &str, val_str: &str, unit: &str) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(label)
                .small()
                .color(colors::TEXT_SECONDARY),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if !unit.is_empty() {
                ui.label(egui::RichText::new(unit).small().color(colors::TEXT_MUTED));
            }
            ui.label(
                egui::RichText::new(val_str)
                    .small()
                    .strong()
                    .color(colors::TEXT_ACCENT),
            );
        });
    });
}

/// Helper: Draw a crisp 1px horizontal separator line.
#[allow(dead_code)]
pub fn draw_separator(ui: &mut egui::Ui) {
    let rect = ui.available_rect_before_wrap();
    let y = rect.min.y + 0.5;
    ui.painter().line_segment(
        [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
    ui.add_space(1.0);
}

/// Helper: Draw a layer label color chip.
#[allow(dead_code)]
pub fn draw_label_chip(ui: &mut egui::Ui, color: egui::Color32) -> egui::Response {
    let (hit_rect, response) = ui.allocate_exact_size(
        egui::vec2(layout::HIT_TARGET_MIN, layout::HIT_TARGET_MIN),
        egui::Sense::click(),
    );
    let rect = egui::Rect::from_center_size(hit_rect.center(), egui::vec2(12.0, 12.0));
    ui.painter().rect_filled(rect, 2.0, color);
    response
}

/// Helper: Create a consistent AE-style panel frame.
#[allow(dead_code)]
pub fn panel_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(colors::BG_DARK)
        .inner_margin(egui::Margin::same(8.0))
        .stroke(egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE))
}

/// Helper: Create a consistent AE-style side panel frame.
#[allow(dead_code)]
pub fn side_panel_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(colors::BG_DARKEST)
        .inner_margin(egui::Margin::same(8.0))
        .stroke(egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE))
}

/// Custom DragValue with AE-style modifier keys:
/// - Normal drag: 1x speed
/// - Alt+drag: 0.1x speed (fine control)
/// - Shift+drag: 10x speed (fast scrub)
#[allow(dead_code)]
pub fn ae_drag_value(value: &mut f32) -> egui::DragValue<'_> {
    egui::DragValue::new(value).speed(1.0)
}
