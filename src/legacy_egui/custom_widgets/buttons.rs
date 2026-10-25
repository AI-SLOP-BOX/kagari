use crate::ui::theme::colors;
use eframe::egui;

pub(crate) fn painter_button_target(
    ui: &mut egui::Ui,
    size: egui::Vec2,
    label: &str,
    enabled: bool,
    selected: bool,
) -> (egui::Rect, egui::Response) {
    let response = ui
        .add_enabled_ui(enabled, |ui| {
            ui.scope(|ui| {
                ui.spacing_mut().button_padding = egui::Vec2::ZERO;
                let widgets = &mut ui.visuals_mut().widgets;
                for state in [
                    &mut widgets.inactive,
                    &mut widgets.hovered,
                    &mut widgets.active,
                    &mut widgets.open,
                ] {
                    state.bg_fill = egui::Color32::TRANSPARENT;
                    state.weak_bg_fill = egui::Color32::TRANSPARENT;
                    state.bg_stroke = egui::Stroke::NONE;
                    state.expansion = 0.0;
                }
                ui.add_sized(
                    size,
                    egui::Button::new(
                        egui::RichText::new(" ")
                            .size(1.0)
                            .color(egui::Color32::TRANSPARENT),
                    )
                    .frame(false)
                    .selected(selected),
                )
            })
            .inner
        })
        .inner;
    (response.rect, response.on_hover_text(label))
}

/// Professional AE-style button with subtle hover gradient
pub fn ae_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    crate::ui::kui::button(ui, label, crate::ui::kui::ButtonKind::Default)
}

/// Accent button (blue background)
pub fn ae_button_accent(ui: &mut egui::Ui, label: &str) -> egui::Response {
    crate::ui::kui::button(ui, label, crate::ui::kui::ButtonKind::Accent)
}

/// Small icon button (for toolbars)
pub fn ae_icon_button(ui: &mut egui::Ui, icon: &str, tooltip: &str) -> egui::Response {
    crate::ui::kui::button(ui, icon, crate::ui::kui::ButtonKind::Quiet).on_hover_text(tooltip)
}

/// Compact text switch for dense production panels. It stays quiet until
/// hovered, while the active state remains legible without a button box.
pub fn ae_text_toggle(
    ui: &mut egui::Ui,
    active: bool,
    label: &str,
    tooltip: &str,
) -> egui::Response {
    let (rect, response) = painter_button_target(
        ui,
        egui::vec2(27.0, crate::ui::theme::layout::HIT_TARGET_MIN),
        label,
        true,
        active,
    );
    if response.hovered() {
        ui.painter().rect_filled(rect, 3.0, colors::BG_HOVER);
    }
    let color = if active {
        colors::TEXT_PRIMARY
    } else if response.hovered() {
        colors::TEXT_SECONDARY
    } else {
        colors::TEXT_MUTED
    };
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(11.0),
        color,
    );
    if active {
        ui.painter().line_segment(
            [
                egui::pos2(rect.left() + 5.0, rect.bottom() - 2.0),
                egui::pos2(rect.right() - 5.0, rect.bottom() - 2.0),
            ],
            egui::Stroke::new(1.0_f32, colors::ACCENT_SELECTION),
        );
    }
    response.on_hover_text(tooltip)
}

/// Small transparent SVG control used for secondary timeline and Inspector actions.
pub fn ae_svg_icon_button(
    ui: &mut egui::Ui,
    id: impl Into<String>,
    svg: &'static str,
    tooltip: &str,
    tint: egui::Color32,
) -> egui::Response {
    let (rect, response) = painter_button_target(
        ui,
        egui::vec2(
            crate::ui::theme::layout::HIT_TARGET_MIN,
            crate::ui::theme::layout::HIT_TARGET_MIN,
        ),
        tooltip,
        true,
        false,
    );
    if response.hovered() {
        ui.painter().rect_filled(rect, 2.0, colors::BG_HOVER);
    }
    crate::ui::icons::render_svg_at(
        ui,
        id.into(),
        svg,
        egui::vec2(12.0, 12.0),
        tint,
        rect.center() - egui::vec2(6.0, 6.0),
    );
    response.on_hover_text(tooltip)
}

/// Professional SVG vector icon toggle button for timeline switches and toolbars
pub fn ae_svg_toggle(
    ui: &mut egui::Ui,
    active: &mut bool,
    svg_str: &'static str,
    id_name: &str,
    size: egui::Vec2,
    active_color: egui::Color32,
    tooltip: &str,
) -> egui::Response {
    let (rect, resp) = painter_button_target(ui, size, tooltip, true, *active);
    let is_active = *active;
    let bg_color = if is_active {
        colors::BG_HOVER
    } else if resp.hovered() {
        colors::BG_MID
    } else {
        egui::Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 3.0, bg_color);
    if is_active {
        ui.painter()
            .rect_stroke(rect, 3.0, egui::Stroke::new(1.0_f32, active_color));
    }
    let tint = if is_active {
        active_color
    } else if resp.hovered() {
        colors::TEXT_PRIMARY
    } else {
        colors::TEXT_MUTED
    };
    let icon_rect = rect.shrink(2.0);
    let _ = crate::ui::icons::render_svg_at(
        ui,
        id_name.to_string(),
        svg_str,
        icon_rect.size(),
        tint,
        icon_rect.min,
    );
    if resp.clicked() {
        *active = !*active;
    }
    resp.on_hover_text(tooltip)
}

/// Professional SVG vector action button
pub fn ae_svg_button(
    ui: &mut egui::Ui,
    svg_str: &'static str,
    id_name: &str,
    size: egui::Vec2,
    default_color: egui::Color32,
    tooltip: &str,
) -> egui::Response {
    let (rect, resp) = painter_button_target(ui, size, tooltip, true, false);
    let bg_color = if resp.is_pointer_button_down_on() {
        colors::BG_ACTIVE
    } else if resp.hovered() {
        colors::BG_HOVER
    } else {
        egui::Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 3.0, bg_color);
    let tint = if resp.hovered() {
        colors::TEXT_PRIMARY
    } else {
        default_color
    };
    let icon_rect = rect.shrink(2.0);
    let _ = crate::ui::icons::render_svg_at(
        ui,
        id_name.to_string(),
        svg_str,
        icon_rect.size(),
        tint,
        icon_rect.min,
    );
    resp.on_hover_text(tooltip)
}

/// AE-style toggle switch
pub fn ae_toggle(ui: &mut egui::Ui, value: &mut bool, label: &str) -> egui::Response {
    ui.horizontal(|ui| {
        let (hit_rect, mut response) = painter_button_target(
            ui,
            egui::vec2(32.0, crate::ui::theme::layout::HIT_TARGET_MIN),
            label,
            true,
            *value,
        );
        let rect = egui::Rect::from_center_size(hit_rect.center(), egui::vec2(32.0, 16.0));

        // Track
        let track_color = if *value {
            colors::ACCENT_TOGGLE
        } else {
            colors::BG_SURFACE
        };
        ui.painter().rect_filled(rect, 8.0, track_color);
        ui.painter()
            .rect_stroke(rect, 8.0, egui::Stroke::new(1.0_f32, colors::BORDER_MEDIUM));

        // Thumb
        let thumb_x = if *value {
            rect.right() - 10.0
        } else {
            rect.left() + 2.0
        };
        let thumb_rect = egui::Rect::from_center_size(
            egui::pos2(thumb_x, rect.center().y),
            egui::vec2(12.0, 12.0),
        );
        ui.painter()
            .rect_filled(thumb_rect, 6.0, egui::Color32::WHITE);

        let keyboard_toggle = response.has_focus()
            && ui.input(|input| {
                input.key_pressed(egui::Key::Space) || input.key_pressed(egui::Key::Enter)
            });
        if response.clicked() || keyboard_toggle {
            *value = !*value;
            response.mark_changed();
        }

        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(label)
                .small()
                .color(colors::TEXT_SECONDARY),
        );

        response
    })
    .inner
}
