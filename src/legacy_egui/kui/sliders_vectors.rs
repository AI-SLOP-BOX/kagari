use super::inputs::{drag, number};
use super::typography_layout::field_scope;
use crate::ui::theme::{colors, layout};
use eframe::egui;
use std::ops::RangeInclusive;

pub fn vector<const N: usize>(
    ui: &mut egui::Ui,
    value: &mut [f32; N],
    range: RangeInclusive<f32>,
    speed: f64,
    unit: &str,
) -> egui::Response {
    let axes = ["X", "Y", "Z", "W"];
    vector_axes(
        ui,
        value,
        range,
        speed,
        unit,
        std::array::from_fn(|index| axes.get(index).copied().unwrap_or("")),
    )
}

pub fn vector_axes<const N: usize>(
    ui: &mut egui::Ui,
    value: &mut [f32; N],
    range: RangeInclusive<f32>,
    speed: f64,
    unit: &str,
    axes: [&str; N],
) -> egui::Response {
    vector_axes_inner(ui, value, Some(range), speed, unit, axes)
}

pub fn vector_axes_unbounded<const N: usize>(
    ui: &mut egui::Ui,
    value: &mut [f32; N],
    speed: f64,
    unit: &str,
    axes: [&str; N],
) -> egui::Response {
    vector_axes_inner(ui, value, None, speed, unit, axes)
}

fn vector_axes_inner<const N: usize>(
    ui: &mut egui::Ui,
    value: &mut [f32; N],
    range: Option<RangeInclusive<f32>>,
    speed: f64,
    unit: &str,
    axes: [&str; N],
) -> egui::Response {
    ui.set_max_width(180.0);
    let mut response: Option<egui::Response> = None;
    let mut draw_axis = |ui: &mut egui::Ui, index: usize| {
        let axis = axes[index];
        let next = ui
            .push_id(index, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    let axis_color = match axis {
                        "X" | "P" => colors::GIZMO_X,
                        "Y" => colors::GIZMO_Y,
                        "Z" | "R" => colors::GIZMO_Z,
                        _ => colors::TEXT_SECONDARY,
                    };
                    ui.label(egui::RichText::new(axis).small().strong().color(axis_color));
                    let field_width = (ui.available_width() - 4.0).clamp(36.0, 56.0);
                    let field = drag(&mut value[index])
                        .speed(speed)
                        .unit(unit)
                        .width(field_width);
                    ui.add(match &range {
                        Some(range) => field.range(range.clone()),
                        None => field,
                    })
                })
                .inner
            })
            .inner;
        response = Some(match response.take() {
            Some(previous) => previous.union(next),
            None => next,
        });
    };
    if N > 0 && ui.available_width() >= N as f32 * 90.0 {
        ui.columns(N, |columns| {
            for (index, column) in columns.iter_mut().enumerate() {
                draw_axis(column, index);
            }
        });
    } else {
        ui.vertical(|ui| {
            for index in 0..N {
                draw_axis(ui, index);
            }
        });
    }
    response.unwrap_or_else(|| ui.allocate_response(egui::Vec2::ZERO, egui::Sense::hover()))
}

pub fn parameter_slider(
    ui: &mut egui::Ui,
    value: &mut f32,
    range: RangeInclusive<f32>,
    unit: &str,
) -> egui::Response {
    parameter_slider_field(ui, value, range, unit, None)
}

pub fn parameter_slider_step(
    ui: &mut egui::Ui,
    value: &mut f32,
    range: RangeInclusive<f32>,
    unit: &str,
    step: f64,
) -> egui::Response {
    parameter_slider_field(ui, value, range, unit, Some(step))
}

fn parameter_slider_field(
    ui: &mut egui::Ui,
    value: &mut f32,
    range: RangeInclusive<f32>,
    unit: &str,
    step: Option<f64>,
) -> egui::Response {
    let width = ui.available_width();
    if width < 160.0 {
        return number(ui, value, range, step.unwrap_or(0.1), unit);
    }
    let number_width = 56.0_f32.min(width * 0.35);
    let slider_width = (width - number_width - ui.spacing().item_spacing.x).max(40.0);
    ui.horizontal(|ui| {
        let slider = ui
            .allocate_ui_with_layout(
                egui::vec2(slider_width, layout::FIELD_HEIGHT),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| parameter_slider_impl(ui, value, range.clone(), "", step, false),
            )
            .inner;
        let number = ui.add(
            drag(value)
                .range(range)
                .speed(step.unwrap_or(0.1))
                .unit(unit)
                .width(number_width),
        );
        slider.union(number)
    })
    .inner
}

pub fn parameter_slider_impl(
    ui: &mut egui::Ui,
    value: &mut f32,
    range: RangeInclusive<f32>,
    unit: &str,
    step: Option<f64>,
    show_value: bool,
) -> egui::Response {
    let minimum = *range.start();
    let maximum = *range.end();
    let width = ui.available_width();
    let (rect, mut response) = ui.allocate_exact_size(
        egui::vec2(width, layout::FIELD_HEIGHT),
        egui::Sense::click_and_drag(),
    );
    if !minimum.is_finite() || !maximum.is_finite() || maximum <= minimum {
        return response;
    }
    let value_span = maximum - minimum;
    if !value_span.is_finite() {
        return response;
    }
    let value_before = *value;
    let mut value_changed = !value.is_finite() || *value < minimum || *value > maximum;
    if value_changed {
        *value = if value.is_finite() {
            snap_slider_value(*value, minimum, maximum, step)
        } else {
            minimum
        };
    }
    let label_width = if show_value { 44.0 } else { 0.0 };
    let track_width = (rect.width() - label_width - 8.0).max(12.0);
    let track = egui::Rect::from_min_size(
        egui::pos2(rect.left() + 4.0, rect.center().y - 1.5),
        egui::vec2(track_width, 3.0),
    );
    if response.clicked() {
        response.request_focus();
    }
    if response.clicked() || response.dragged() {
        if let Some(pointer) = response.interact_pointer_pos() {
            let ratio = ((pointer.x - track.left()) / track.width()).clamp(0.0, 1.0);
            *value = snap_slider_value(minimum + ratio * value_span, minimum, maximum, step);
        }
    }
    if response.has_focus() {
        let keyboard_delta = ui.input(|input| {
            if input.key_pressed(egui::Key::ArrowRight) || input.key_pressed(egui::Key::ArrowUp) {
                1.0
            } else if input.key_pressed(egui::Key::ArrowLeft)
                || input.key_pressed(egui::Key::ArrowDown)
            {
                -1.0
            } else {
                0.0
            }
        });
        let increment = step.map_or(value_span / 100.0, |step| step as f32);
        if keyboard_delta != 0.0 {
            *value = snap_slider_value(*value + keyboard_delta * increment, minimum, maximum, step);
        }
    }
    let ratio = ((*value - minimum) / value_span).clamp(0.0, 1.0);
    let fill = egui::Rect::from_min_max(
        track.left_top(),
        egui::pos2(track.left() + ratio * track.width(), track.bottom()),
    );
    let painter = ui.painter().with_clip_rect(rect);
    painter.rect_filled(track, 1.5, colors::BORDER_SUBTLE);
    painter.rect_filled(fill, 1.5, colors::ACCENT_SELECTION);
    painter.circle_filled(
        egui::pos2(track.left() + ratio * track.width(), track.center().y),
        5.0,
        colors::ACCENT_SELECTION,
    );
    if show_value {
        let decimals = step.map_or_else(
            || {
                if value.abs() >= 1000.0 {
                    0
                } else if value.abs() >= 100.0 {
                    1
                } else {
                    2
                }
            },
            |step| (-step.log10()).ceil().clamp(0.0, 4.0) as usize,
        );
        painter.text(
            egui::pos2(rect.right() - 2.0, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            format!("{value:.decimals$}{unit}"),
            egui::FontId::proportional(11.0),
            colors::TEXT_SECONDARY,
        );
    }
    value_changed |= *value != value_before;
    if value_changed {
        response.mark_changed();
    }
    response.on_hover_text(format!("{:.4}{}", *value, unit))
}

fn snap_slider_value(value: f32, minimum: f32, maximum: f32, step: Option<f64>) -> f32 {
    let value = step.map_or(value, |step| {
        let step = step as f32;
        minimum + ((value - minimum) / step).round() * step
    });
    value.clamp(minimum, maximum)
}

pub fn slider(
    ui: &mut egui::Ui,
    value: &mut f32,
    range: RangeInclusive<f32>,
    unit: &str,
) -> egui::Response {
    parameter_slider_field(ui, value, range, unit, None)
}

pub fn slider_step(
    ui: &mut egui::Ui,
    value: &mut f32,
    range: RangeInclusive<f32>,
    unit: &str,
    step: f64,
) -> egui::Response {
    parameter_slider_field(ui, value, range, unit, Some(step))
}

pub fn slider_without_value(
    ui: &mut egui::Ui,
    value: &mut f32,
    range: RangeInclusive<f32>,
) -> egui::Response {
    field_scope(ui, |ui| {
        parameter_slider_impl(ui, value, range, "", None, false)
    })
}

pub fn slider_step_without_value(
    ui: &mut egui::Ui,
    value: &mut f32,
    range: RangeInclusive<f32>,
    step: f64,
) -> egui::Response {
    field_scope(ui, |ui| {
        parameter_slider_impl(ui, value, range, "", Some(step), false)
    })
}

pub fn slider_integer(
    ui: &mut egui::Ui,
    value: &mut u32,
    range: RangeInclusive<u32>,
) -> egui::Response {
    let minimum = *range.start();
    let maximum = *range.end();
    let width = ui.available_width();
    if maximum <= minimum || width < 160.0 {
        return ui.add(drag(value).range(range).width(width));
    }
    let number_width = 56.0_f32.min(width * 0.35);
    let slider_width = (width - number_width - ui.spacing().item_spacing.x).max(40.0);
    ui.horizontal(|ui| {
        let mut slider_value = *value as f32;
        let slider = ui
            .allocate_ui_with_layout(
                egui::vec2(slider_width, layout::FIELD_HEIGHT),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    parameter_slider_impl(
                        ui,
                        &mut slider_value,
                        minimum as f32..=maximum as f32,
                        "",
                        Some(1.0),
                        false,
                    )
                },
            )
            .inner;
        if slider.changed() {
            *value = (slider_value.round() as u32).clamp(minimum, maximum);
        }
        let number = ui.add(
            drag(value)
                .range(minimum..=maximum)
                .speed(1.0)
                .width(number_width),
        );
        slider.union(number)
    })
    .inner
}

pub fn scalar_slider<N: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    value: &mut N,
    range: RangeInclusive<N>,
    unit: &str,
    step: Option<f64>,
    logarithmic: bool,
) -> egui::Response {
    let minimum = range.start().to_f64();
    let maximum = range.end().to_f64();
    let width = ui.available_width();
    if !minimum.is_finite()
        || !maximum.is_finite()
        || maximum <= minimum
        || (logarithmic && minimum <= 0.0)
        || width < 160.0
    {
        return ui.add(
            drag(value)
                .range(range)
                .speed(step.unwrap_or(0.1))
                .unit(unit),
        );
    }

    let number_width = 56.0_f32.min(width * 0.35);
    let slider_width = (width - number_width - ui.spacing().item_spacing.x).max(40.0);
    ui.horizontal(|ui| {
        let mut slider_value = value.to_f64() as f32;
        if logarithmic {
            slider_value = slider_value.ln();
        }
        let slider_min = if logarithmic {
            minimum.ln() as f32
        } else {
            minimum as f32
        };
        let slider_max = if logarithmic {
            maximum.ln() as f32
        } else {
            maximum as f32
        };
        let slider = ui
            .allocate_ui_with_layout(
                egui::vec2(slider_width, layout::FIELD_HEIGHT),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    parameter_slider_impl(
                        ui,
                        &mut slider_value,
                        slider_min..=slider_max,
                        "",
                        if logarithmic { None } else { step },
                        false,
                    )
                },
            )
            .inner;
        if slider.changed() {
            let value_f64 = if logarithmic {
                slider_value.exp() as f64
            } else {
                slider_value as f64
            };
            *value = N::from_f64(value_f64.clamp(minimum, maximum));
        }
        let number = ui.add(
            drag(value)
                .range(range)
                .speed(step.unwrap_or(if N::INTEGRAL { 1.0 } else { 0.1 }))
                .unit(unit)
                .width(number_width),
        );
        slider.union(number)
    })
    .inner
}
