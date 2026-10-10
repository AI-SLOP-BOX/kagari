use crate::ui::theme::colors;
use eframe::egui;

/// AE-style section header with accent bar
pub fn ae_section_header(ui: &mut egui::Ui, title: &str, icon: &str) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        // Accent bar
        let (bar_rect, _) = ui.allocate_exact_size(egui::vec2(3.0, 14.0), egui::Sense::hover());
        ui.painter().rect_filled(bar_rect, 1.0, colors::ACCENT_CYAN);

        ui.add_space(4.0);

        if let Some(svg) = crate::ui::icons::svg_for_icon_label(icon) {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
            crate::ui::icons::render_svg_at(
                ui,
                format!("section-icon-{title}"),
                svg,
                rect.size(),
                colors::TEXT_SECONDARY,
                rect.min,
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
            egui::RichText::new(crate::ui::localization::tr(title))
                .small()
                .strong()
                .color(colors::TEXT_PRIMARY),
        );
    });
    ui.add_space(2.0);
}

/// AE-style property row with label and value
pub fn ae_property_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(crate::ui::localization::tr(label))
                .small()
                .color(colors::TEXT_SECONDARY),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(value)
                    .small()
                    .strong()
                    .color(colors::TEXT_ACCENT),
            );
        });
    });
}

/// AE-style horizontal separator
pub fn ae_separator(ui: &mut egui::Ui) {
    let rect = ui.available_rect_before_wrap();
    let y = rect.min.y + 0.5;
    ui.painter().line_segment(
        [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
    ui.add_space(1.0);
}

/// AE-style panel header with subtle gradient
pub fn ae_panel_header(ui: &mut egui::Ui, title: &str) {
    let rect = ui.available_rect_before_wrap();
    let header_rect = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), 28.0));

    // Subtle gradient background
    ui.painter().rect_filled(header_rect, 0.0, colors::BG_DARK);

    // Bottom border
    ui.painter().line_segment(
        [
            egui::pos2(header_rect.left(), header_rect.bottom()),
            egui::pos2(header_rect.right(), header_rect.bottom()),
        ],
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );

    // Title
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(title)
                .small()
                .strong()
                .color(colors::TEXT_PRIMARY),
        );
    });
    ui.add_space(6.0);
}

/// AE-style color swatch
pub fn ae_color_swatch(ui: &mut egui::Ui, color: egui::Color32, size: f32) -> egui::Response {
    crate::ui::kui::color_swatch(ui, color, size)
}

/// AE-style progress bar
pub fn ae_progress_bar(ui: &mut egui::Ui, progress: f32, height: f32) {
    let rect = ui
        .allocate_ui_with_layout(
            egui::vec2(ui.available_width(), height),
            egui::Layout::left_to_right(egui::Align::Center),
            |_ui| {},
        )
        .response
        .rect;

    // Background
    ui.painter().rect_filled(rect, 2.0, colors::BG_SURFACE);

    // Fill
    let fill_width = rect.width() * progress.clamp(0.0, 1.0);
    let fill_rect = egui::Rect::from_min_size(rect.min, egui::vec2(fill_width, rect.height()));
    ui.painter()
        .rect_filled(fill_rect, 2.0, colors::ACCENT_PROGRESS);

    // Border
    ui.painter()
        .rect_stroke(rect, 2.0, egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE));
}

/// AE-style text input field
pub fn ae_text_field(ui: &mut egui::Ui, text: &mut String, placeholder: &str) -> egui::Response {
    crate::ui::kui::text_field_with_hint(ui, text, placeholder)
}

/// AE-style tactile scrubber: blue/accent underlined text that can be scrubbed
/// left/right with the mouse (Shift for 10x, Alt for 0.1x), or clicked to enter exact value.
pub fn ae_scrubber(
    ui: &mut egui::Ui,
    value: &mut f32,
    speed: f32,
    unit: &str,
    range: Option<std::ops::RangeInclusive<f32>>,
) -> egui::Response {
    let speed = if ui.input(|i| i.modifiers.shift) {
        speed * 10.0
    } else if ui.input(|i| i.modifiers.alt) {
        speed * 0.1
    } else {
        speed
    };

    let text_val = if unit.is_empty() {
        format!("{:.1}", value)
    } else {
        format!("{:.1}{}", value, unit)
    };

    let (rect, mut resp) = ui.allocate_exact_size(
        egui::vec2(
            (text_val.len() as f32 * 8.0).clamp(42.0, 96.0),
            crate::ui::theme::layout::FIELD_HEIGHT,
        ),
        egui::Sense::click_and_drag(),
    );

    if resp.dragged() {
        let delta = resp.drag_delta().x * speed;
        *value += delta;
        if let Some(r) = &range {
            *value = value.clamp(*r.start(), *r.end());
        }
        resp.mark_changed();
    }

    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        ui.painter().rect_filled(
            rect.shrink2(egui::vec2(2.0, 2.0)),
            4.0,
            colors::BG_HOVER.linear_multiply(0.5),
        );
    }

    let text_color = if resp.dragged() {
        colors::ACCENT_BRAND_HOVER
    } else if resp.hovered() {
        colors::ACCENT_BRAND
    } else {
        colors::ACCENT_BLUE_LIGHT
    };

    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text_val,
        egui::FontId::monospace(11.5),
        text_color,
    );

    // Subtle dashed or dotted style underline for AE look
    let line_y = rect.bottom() - 3.0;
    ui.painter().line_segment(
        [egui::pos2(rect.left() + 4.0, line_y), egui::pos2(rect.right() - 4.0, line_y)],
        egui::Stroke::new(1.0_f32, text_color.linear_multiply(0.7)),
    );

    resp
}
