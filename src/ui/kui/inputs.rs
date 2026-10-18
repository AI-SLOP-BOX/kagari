use super::buttons::{button, ButtonKind};
use super::typography_layout::field_scope;
use crate::ui::theme::{colors, layout};
use eframe::egui;
use std::ops::RangeInclusive;

pub struct NumberField<'a> {
    widget: egui::DragValue<'a>,
    width: Option<f32>,
    base_speed: Option<f64>,
}

pub fn drag<N: egui::emath::Numeric>(value: &mut N) -> NumberField<'_> {
    NumberField {
        widget: egui::DragValue::new(value),
        width: None,
        base_speed: None,
    }
}

impl NumberField<'_> {
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width.max(0.0));
        self
    }

    pub fn unit(mut self, unit: &str) -> Self {
        self.widget = self.widget.suffix(unit);
        self
    }

    pub fn prefix(mut self, prefix: &str) -> Self {
        self.widget = self.widget.prefix(prefix);
        self
    }

    pub fn range<N: egui::emath::Numeric>(mut self, range: RangeInclusive<N>) -> Self {
        self.widget = self.widget.range(range);
        self
    }

    pub fn speed(mut self, speed: f64) -> Self {
        self.base_speed = Some(speed);
        self
    }
}

impl egui::Widget for NumberField<'_> {
    fn ui(mut self, ui: &mut egui::Ui) -> egui::Response {
        // AE modifier keys: Shift = 10x fast scrub, Alt/Option = 0.1x precision scrub
        let speed_mult = if ui.input(|i| i.modifiers.shift) {
            10.0
        } else if ui.input(|i| i.modifiers.alt) {
            0.1
        } else {
            1.0
        };

        let effective_speed = self.base_speed.unwrap_or(1.0) * speed_mult;
        self.widget = self.widget.speed(effective_speed);

        let resp = field_scope(ui, |ui| {
            ui.add_sized(
                [
                    self.width
                        .unwrap_or_else(|| ui.available_width().min(180.0)),
                    layout::FIELD_HEIGHT,
                ],
                self.widget,
            )
        });

        // Interactive cursor feedback
        if resp.hovered() || resp.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }

        resp
    }
}

pub fn number(
    ui: &mut egui::Ui,
    value: &mut f32,
    range: RangeInclusive<f32>,
    speed: f64,
    unit: &str,
) -> egui::Response {
    ui.add(drag(value).range(range).speed(speed).unit(unit).width(96.0))
}

pub fn integer(ui: &mut egui::Ui, value: &mut u32, range: RangeInclusive<u32>) -> egui::Response {
    ui.add(drag(value).range(range).width(96.0))
}

pub fn text_field(ui: &mut egui::Ui, value: &mut String) -> egui::Response {
    text_field_with_hint(ui, value, "")
}

pub fn text_field_with_hint(ui: &mut egui::Ui, value: &mut String, hint: &str) -> egui::Response {
    field_scope(ui, |ui| {
        ui.add(
            egui::TextEdit::singleline(value)
                .hint_text(crate::ui::localization::tr(hint))
                .desired_width(ui.available_width())
                .margin(egui::Margin::symmetric(6.0, 4.0)),
        )
    })
}

pub fn file_field(
    ui: &mut egui::Ui,
    path: &mut String,
    hint: &str,
    browse_tooltip: &str,
) -> egui::Response {
    ui.horizontal(|ui| {
        let browse_width = 76.0;
        let field_width =
            (ui.available_width() - browse_width - ui.spacing().item_spacing.x).max(80.0);
        let field = field_scope(ui, |ui| {
            ui.add(
                egui::TextEdit::singleline(path)
                    .hint_text(crate::ui::localization::tr(hint))
                    .desired_width(field_width)
                    .margin(egui::Margin::symmetric(6.0, 4.0)),
            )
        });
        let browse = button(ui, "Browse…", ButtonKind::Quiet)
            .on_hover_text(crate::ui::localization::tr(browse_tooltip));
        field.union(browse)
    })
    .inner
}

pub fn switch(ui: &mut egui::Ui, value: &mut bool, label: &str) -> egui::Response {
    let label = crate::ui::localization::tr(label);
    let (switch_rect, mut response) =
        ui.allocate_exact_size(egui::vec2(34.0, 20.0), egui::Sense::click());
    let label_response =
        (!label.is_empty()).then(|| ui.label(egui::RichText::new(label.clone()).size(12.0)));
    let mut changed =
        response.clicked() || label_response.as_ref().is_some_and(|label| label.clicked());
    if changed {
        *value = !*value;
        response.request_focus();
    }
    if response.has_focus()
        && ui.input(|input| {
            input.key_pressed(egui::Key::Space) || input.key_pressed(egui::Key::Enter)
        })
    {
        *value = !*value;
        changed = true;
    }
    let track = if *value {
        colors::ACCENT_WORKSPACE
    } else {
        colors::BG_MID
    };
    ui.painter().rect_filled(switch_rect, 10.0, track);
    ui.painter().rect_stroke(
        switch_rect,
        10.0,
        egui::Stroke::new(1.0_f32, colors::BORDER_MEDIUM),
    );
    let knob_x = if *value {
        switch_rect.right() - 10.0
    } else {
        switch_rect.left() + 10.0
    };
    ui.painter().circle_filled(
        egui::pos2(knob_x, switch_rect.center().y),
        7.0,
        colors::TEXT_PRIMARY,
    );
    if changed {
        response.mark_changed();
    }
    response = response.on_hover_text(label);
    if let Some(label_response) = label_response {
        response.union(label_response)
    } else {
        response
    }
}

pub fn toggle(ui: &mut egui::Ui, value: &mut bool, label: &str) -> egui::Response {
    switch(ui, value, label)
}

pub fn combo<T: PartialEq + Clone>(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash,
    value: &mut T,
    options: &[(T, &str)],
) -> bool {
    let selected = options
        .iter()
        .find(|(key, _)| key == value)
        .map(|(_, text)| crate::ui::localization::tr(text))
        .unwrap_or_else(|| "—".to_owned());
    combo_with_selected(ui, id, value, &selected, options)
}

pub fn combo_with_selected<T: PartialEq + Clone>(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash,
    value: &mut T,
    selected: &str,
    options: &[(T, &str)],
) -> bool {
    let selected = crate::ui::localization::tr(selected);
    field_scope(ui, |ui| {
        let mut changed = false;
        egui::ComboBox::from_id_salt(id)
            .width(ui.available_width().min(240.0))
            .selected_text(selected.clone())
            .truncate()
            .show_ui(ui, |ui| {
                for (key, text) in options {
                    changed |= ui
                        .selectable_value(value, key.clone(), crate::ui::localization::tr(text))
                        .changed();
                }
            })
            .response
            .on_hover_text(selected);
        changed
    })
}

pub struct PropertyGrid {
    inner: choegui::design::PropertyGrid,
}

impl PropertyGrid {
    pub fn new(ui: &egui::Ui, labels: &[&str]) -> Self {
        let font = egui::TextStyle::Body.resolve(ui.style());
        let label_width = labels
            .iter()
            .map(|label| {
                let english_width = ui
                    .painter()
                    .layout_no_wrap((*label).to_owned(), font.clone(), colors::TEXT_SECONDARY)
                    .size()
                    .x;
                let localized_width = ui
                    .painter()
                    .layout_no_wrap(
                        crate::ui::localization::tr(label),
                        font.clone(),
                        colors::TEXT_SECONDARY,
                    )
                    .size()
                    .x;
                english_width.max(localized_width)
            })
            .fold(layout::PROPERTY_LABEL_WIDTH, f32::max);
        Self {
            inner: choegui::design::PropertyGrid::from_measured_width(
                label_width,
                colors::TEXT_SECONDARY,
                layout::FIELD_HEIGHT,
                layout::PROPERTY_CONTROL_MIN_WIDTH,
            ),
        }
    }

    pub fn row<R>(
        &self,
        ui: &mut egui::Ui,
        label: &str,
        control: impl FnOnce(&mut egui::Ui) -> R,
    ) -> R {
        let label = crate::ui::localization::tr(label);
        self.inner.row(ui, &label, control)
    }
}

pub fn property<R>(ui: &mut egui::Ui, label: &str, control: impl FnOnce(&mut egui::Ui) -> R) -> R {
    PropertyGrid {
        inner: choegui::design::PropertyGrid::from_measured_width(
            layout::PROPERTY_LABEL_WIDTH,
            colors::TEXT_SECONDARY,
            layout::FIELD_HEIGHT,
            layout::PROPERTY_CONTROL_MIN_WIDTH,
        ),
    }
    .row(ui, label, control)
}
