use super::typography_layout::field_scope;
use crate::ui::theme::colors;
use eframe::egui;
use std::ops::RangeInclusive;

pub fn color(ui: &mut egui::Ui, value: &mut egui::Color32) -> egui::Response {
    field_scope(ui, |ui| ui.color_edit_button_srgba(value))
}

pub fn color_rgb(ui: &mut egui::Ui, value: &mut [f32; 3]) -> egui::Response {
    field_scope(ui, |ui| ui.color_edit_button_rgb(value))
}

pub fn color_rgba(ui: &mut egui::Ui, value: &mut [f32; 4]) -> egui::Response {
    field_scope(ui, |ui| ui.color_edit_button_rgba_unmultiplied(value))
}

pub fn color_swatch(ui: &mut egui::Ui, color: egui::Color32, size: f32) -> egui::Response {
    let (rect, response) = crate::ui::custom_widgets::painter_button_target(
        ui,
        egui::vec2(size.max(1.0), size.max(1.0)),
        &format!("Color swatch {color:?}"),
        true,
        false,
    );
    let painter = ui.painter().with_clip_rect(rect);
    if response.hovered() || response.has_focus() {
        painter.rect_filled(rect, 3.0, colors::BG_HOVER);
    }
    painter.rect_stroke(
        rect.shrink(0.5),
        3.0,
        egui::Stroke::new(1.0_f32, colors::BORDER_MEDIUM),
    );
    painter.rect_filled(rect.shrink(2.0), 2.0, color);
    response.on_hover_text(format!(
        "#{:02X}{:02X}{:02X}",
        color.r(),
        color.g(),
        color.b()
    ))
}

pub fn level_meter(ui: &mut egui::Ui, label: &str, level: f32, detail: &str) -> egui::Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 18.0), egui::Sense::hover());
    let meter_width = (rect.width() - 82.0).max(8.0);
    let meter = egui::Rect::from_min_size(
        egui::pos2(rect.left() + 22.0, rect.center().y - 3.0),
        egui::vec2(meter_width, 6.0),
    );
    let ratio = if level.is_finite() {
        level.clamp(0.0, 1.0)
    } else {
        0.0
    };
    ui.painter().with_clip_rect(rect).text(
        egui::pos2(rect.left(), rect.center().y),
        egui::Align2::LEFT_CENTER,
        crate::ui::localization::tr(label),
        egui::FontId::proportional(11.0),
        colors::TEXT_SECONDARY,
    );
    ui.painter().rect_filled(meter, 3.0, colors::BG_MID);
    let fill_color = if ratio >= 0.95 {
        colors::ACCENT_RED
    } else if ratio >= 0.8 {
        colors::ACCENT_YELLOW
    } else {
        colors::ACCENT_GREEN
    };
    ui.painter().rect_filled(
        egui::Rect::from_min_size(meter.min, egui::vec2(meter.width() * ratio, meter.height())),
        3.0,
        fill_color,
    );
    ui.painter().with_clip_rect(rect).text(
        egui::pos2(rect.right(), rect.center().y),
        egui::Align2::RIGHT_CENTER,
        detail,
        egui::FontId::proportional(10.0),
        colors::TEXT_MUTED,
    );
    response
}

pub fn vertical_fader(
    ui: &mut egui::Ui,
    value: &mut f32,
    range: RangeInclusive<f32>,
    height: f32,
) -> egui::Response {
    let minimum = *range.start();
    let maximum = *range.end();
    let (rect, mut response) = ui.allocate_exact_size(
        egui::vec2(28.0, height.max(32.0)),
        egui::Sense::click_and_drag(),
    );
    if !minimum.is_finite() || !maximum.is_finite() || maximum <= minimum {
        return response;
    }
    let span = maximum - minimum;
    if !span.is_finite() {
        return response;
    }
    let value_before = *value;
    let mut changed = !value.is_finite() || *value < minimum || *value > maximum;
    if changed {
        *value = if value.is_finite() {
            value.clamp(minimum, maximum)
        } else {
            minimum
        };
    }
    if response.clicked() {
        response.request_focus();
    }
    if response.clicked() || response.dragged() {
        if let Some(pointer) = response.interact_pointer_pos() {
            let ratio = (1.0 - (pointer.y - rect.top()) / rect.height()).clamp(0.0, 1.0);
            *value = minimum + ratio * span;
        }
    }
    if response.has_focus() {
        let keyboard_delta = ui.input(|input| {
            if input.key_pressed(egui::Key::ArrowUp) || input.key_pressed(egui::Key::ArrowRight) {
                1.0
            } else if input.key_pressed(egui::Key::ArrowDown)
                || input.key_pressed(egui::Key::ArrowLeft)
            {
                -1.0
            } else {
                0.0
            }
        });
        *value = (*value + keyboard_delta * span / 100.0).clamp(minimum, maximum);
    }
    changed |= (*value - value_before).abs() > f32::EPSILON;
    if changed {
        response.mark_changed();
    }
    let ratio = ((*value - minimum) / span).clamp(0.0, 1.0);
    let track = egui::Rect::from_min_max(
        egui::pos2(rect.center().x - 3.0, rect.top() + 5.0),
        egui::pos2(rect.center().x + 3.0, rect.bottom() - 5.0),
    );
    let filled = egui::Rect::from_min_max(
        egui::pos2(track.left(), track.bottom() - track.height() * ratio),
        track.right_bottom(),
    );
    let painter = ui.painter().with_clip_rect(rect);
    painter.rect_filled(track, 3.0, colors::BG_MID);
    painter.rect_filled(filled, 3.0, colors::ACCENT_WORKSPACE);
    let thumb_y = track.bottom() - ratio * track.height();
    painter.rect_filled(
        egui::Rect::from_center_size(egui::pos2(rect.center().x, thumb_y), egui::vec2(18.0, 8.0)),
        3.0,
        if response.hovered() || response.dragged() {
            colors::TEXT_PRIMARY
        } else {
            colors::TEXT_SECONDARY
        },
    );
    response.on_hover_text(format!("{:.2}", *value))
}

pub fn vertical_meter(
    ui: &mut egui::Ui,
    label: &str,
    peak: f32,
    width: f32,
    height: f32,
) -> egui::Response {
    let label = crate::ui::localization::tr(label);
    ui.vertical_centered(|ui| {
        if !label.is_empty() {
            super::typography_layout::caption(ui, label.clone());
        }
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
        let peak = if peak.is_finite() {
            peak.clamp(0.0, 1.0)
        } else {
            0.0
        };
        ui.painter()
            .rect_stroke(rect, 2.0, egui::Stroke::new(1.0_f32, colors::BORDER_MEDIUM));
        let clip_indicator = egui::Rect::from_min_size(
            egui::pos2(rect.left() + 2.0, rect.top() + 2.0),
            egui::vec2((width - 4.0).max(0.0), 4.0),
        );
        ui.painter().rect_filled(
            clip_indicator,
            1.0,
            if peak >= 0.92 {
                colors::ACCENT_RED
            } else {
                colors::BG_DEEPEST
            },
        );
        let segments = 24;
        let gap = 1.5;
        let segment_height = (height - gap * (segments - 1) as f32 - 10.0) / segments as f32;
        let active = (peak * segments as f32).round() as usize;
        for index in 0..segments {
            let ratio = index as f32 / segments as f32;
            let color = if ratio >= 0.88 {
                colors::ACCENT_RED
            } else if ratio >= 0.70 {
                colors::ACCENT_YELLOW
            } else {
                colors::ACCENT_GREEN
            };
            let bottom = rect.bottom() - 2.0 - index as f32 * (segment_height + gap);
            let segment = egui::Rect::from_min_size(
                egui::pos2(rect.left() + 2.0, bottom - segment_height),
                egui::vec2((width - 4.0).max(0.0), segment_height.max(0.0)),
            );
            if index < active {
                ui.painter().rect_filled(segment, 1.0, color);
            } else {
                ui.painter()
                    .rect_filled(segment, 1.0, color.linear_multiply(0.2));
            }
        }
        response
    })
    .inner
}

pub fn progress_bar(ui: &mut egui::Ui, fraction: f32, label: Option<&str>) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 16.0), egui::Sense::hover());
    let fraction = if fraction.is_finite() {
        fraction.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let painter = ui.painter().with_clip_rect(rect);
    painter.rect_filled(rect, 4.0, colors::BG_MID);
    painter.rect_filled(
        egui::Rect::from_min_max(
            rect.min,
            egui::pos2(rect.left() + rect.width() * fraction, rect.bottom()),
        ),
        4.0,
        colors::ACCENT_WORKSPACE,
    );
    let label = label
        .map(str::to_owned)
        .unwrap_or_else(|| format!("{:.0}%", fraction * 100.0));
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(10.0),
        colors::TEXT_PRIMARY,
    );
    response
}
