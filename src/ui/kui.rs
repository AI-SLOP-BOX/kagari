use crate::ui::theme::{colors, layout};
use eframe::egui;
use std::ops::RangeInclusive;

pub mod space {
    pub const XS: f32 = 4.0;
    pub const SM: f32 = 8.0;
    pub const MD: f32 = 12.0;
    pub const LG: f32 = 16.0;
    pub const XL: f32 = 24.0;
}

pub fn title(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.label(
        egui::RichText::new(crate::ui::localization::tr(text))
            .size(layout::FONT_SIZE_TITLE)
            .strong()
            .color(colors::TEXT_PRIMARY),
    )
}

pub fn section_title(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.label(
        egui::RichText::new(crate::ui::localization::tr(text))
            .size(13.0)
            .strong()
            .color(colors::TEXT_PRIMARY),
    )
}

pub fn body(ui: &mut egui::Ui, text: impl Into<String>) -> egui::Response {
    ui.label(
        egui::RichText::new(crate::ui::localization::tr(&text.into()))
            .size(13.0)
            .color(colors::TEXT_PRIMARY),
    )
}

pub fn secondary(ui: &mut egui::Ui, text: impl Into<String>) -> egui::Response {
    ui.label(
        egui::RichText::new(crate::ui::localization::tr(&text.into()))
            .size(12.0)
            .color(colors::TEXT_SECONDARY),
    )
}

pub fn caption(ui: &mut egui::Ui, text: impl Into<String>) -> egui::Response {
    ui.label(
        egui::RichText::new(crate::ui::localization::tr(&text.into()))
            .size(10.0)
            .color(colors::TEXT_MUTED),
    )
}

pub fn readout(ui: &mut egui::Ui, text: impl Into<String>, color: egui::Color32) -> egui::Response {
    ui.label(
        egui::RichText::new(text.into())
            .monospace()
            .size(11.0)
            .color(color),
    )
}

pub fn icon_glyph(
    ui: &mut egui::Ui,
    glyph: &str,
    size: f32,
    color: egui::Color32,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(size.max(0.0), size.max(0.0)),
        egui::Sense::hover(),
    );
    ui.painter().with_clip_rect(rect).text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        glyph,
        egui::FontId::proportional(size.max(0.0)),
        color,
    );
    response
}

pub fn error_text(ui: &mut egui::Ui, text: impl Into<String>) -> egui::Response {
    ui.label(
        egui::RichText::new(text.into())
            .size(11.0)
            .color(colors::ACCENT_RED),
    )
}

#[derive(Clone, Copy)]
pub enum StatusKind {
    Info,
    Success,
    Warning,
    Error,
}

pub fn hint(ui: &mut egui::Ui, text: impl Into<String>) -> egui::Response {
    secondary(ui, text)
}

pub fn status(ui: &mut egui::Ui, kind: StatusKind, text: impl Into<String>) -> egui::Response {
    let color = match kind {
        StatusKind::Info => colors::TEXT_SECONDARY,
        StatusKind::Success => colors::ACCENT_GREEN,
        StatusKind::Warning => colors::ACCENT_YELLOW,
        StatusKind::Error => colors::ACCENT_RED,
    };
    ui.label(
        egui::RichText::new(crate::ui::localization::tr(&text.into()))
            .size(11.0)
            .color(color),
    )
}

pub fn truncated_secondary(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Label::new(
            egui::RichText::new(text)
                .size(12.0)
                .color(colors::TEXT_SECONDARY),
        )
        .truncate(),
    )
    .on_hover_text(text)
}

pub fn truncated_primary(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Label::new(
            egui::RichText::new(text)
                .size(13.0)
                .color(colors::TEXT_PRIMARY),
        )
        .truncate(),
    )
    .on_hover_text(text)
}

pub fn item_title(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Label::new(
            egui::RichText::new(text)
                .size(13.0)
                .strong()
                .color(colors::TEXT_PRIMARY),
        )
        .truncate(),
    )
    .on_hover_text(text)
}

pub fn search_field(ui: &mut egui::Ui, value: &mut String, hint: &str) -> egui::Response {
    let width = ui.available_width();
    search_field_sized(ui, value, hint, width)
}

pub fn search_field_sized(
    ui: &mut egui::Ui,
    value: &mut String,
    hint: &str,
    width: f32,
) -> egui::Response {
    field_scope(ui, |ui| {
        ui.add(
            egui::TextEdit::singleline(value)
                .hint_text(crate::ui::localization::tr(hint))
                .desired_width(width.max(24.0).min(ui.available_width()))
                .margin(egui::Margin::symmetric(8.0, 5.0)),
        )
    })
}

pub fn empty_state(ui: &mut egui::Ui, message: &str, height: f32) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height.max(layout::FIELD_HEIGHT)),
        egui::Sense::hover(),
    );
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(
        egui::Layout::centered_and_justified(egui::Direction::TopDown),
    ));
    secondary(&mut child, message);
}

pub fn selectable_item_row<R>(
    ui: &mut egui::Ui,
    selected: bool,
    height: f32,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height.max(layout::FIELD_HEIGHT)),
        egui::Sense::click(),
    );
    let hover = choegui::animate_from(
        ui.ctx(),
        response.id.with("kui-hover"),
        0.0,
        f32::from(response.hovered()),
        choegui::Spring::new(9.0, 1.0),
    );
    let bg_color = match (selected, hover > 0.001) {
        (true, true) => colors::BG_SELECTION_HOVER,
        (true, false) => colors::BG_SELECTION,
        (false, true) => colors::BG_HOVER.gamma_multiply(hover),
        (false, false) => egui::Color32::TRANSPARENT,
    };
    if bg_color != egui::Color32::TRANSPARENT {
        ui.painter().with_clip_rect(rect).rect_filled(rect, layout::BUTTON_RADIUS, bg_color);
    }
    if selected {
        let indicator_rect = egui::Rect::from_min_size(
            egui::pos2(rect.min.x + 2.0, rect.min.y + 4.0),
            egui::vec2(3.0, (rect.height() - 8.0).max(4.0)),
        );
        ui.painter().rect_filled(indicator_rect, 1.5, colors::ACCENT_SELECTION);
    }
    if response.clicked() {
        response.request_focus();
    }
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(egui::vec2(8.0, 2.0)))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    child.set_clip_rect(rect.shrink(1.0));
    let inner = contents(&mut child);
    egui::InnerResponse { inner, response }
}

pub fn shortcut_hint(ui: &mut egui::Ui, hint: &str) -> egui::Response {
    let hint = crate::ui::localization::tr(hint);
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width().min(92.0), 20.0),
        egui::Sense::hover(),
    );
    ui.painter().rect_filled(rect, 4.0, colors::BG_MID);
    ui.painter()
        .rect_stroke(rect, 4.0, egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE));
    ui.painter().with_clip_rect(rect).text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        hint.clone(),
        egui::FontId::proportional(10.0),
        colors::TEXT_SECONDARY,
    );
    response.on_hover_text(hint)
}

pub fn separator(ui: &mut egui::Ui) {
    ui.add_space(space::XS);
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter().line_segment(
        [rect.left_center(), rect.right_center()],
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
    ui.add_space(space::XS);
}

pub fn divider(ui: &mut egui::Ui) {
    separator(ui);
}

pub fn group<R>(
    ui: &mut egui::Ui,
    heading: &str,
    body: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    egui::Frame::none()
        .fill(colors::BG_DARK)
        .rounding(egui::Rounding::same(layout::SURFACE_RADIUS))
        .stroke(egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE))
        .inner_margin(egui::Margin::same(space::SM))
        .show(ui, |ui| {
            section_title(ui, heading);
            ui.add_space(space::SM);
            body(ui)
        })
}

pub fn toolbar<R>(
    ui: &mut egui::Ui,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    egui::Frame::none()
        .fill(egui::Color32::TRANSPARENT)
        .rounding(egui::Rounding::same(layout::BUTTON_RADIUS))
        .stroke(egui::Stroke::NONE)
        .inner_margin(egui::Margin::symmetric(space::SM, space::XS))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = space::XS;
            ui.horizontal_wrapped(contents).inner
        })
}

pub fn toolbar_group<R>(
    ui: &mut egui::Ui,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = space::XS;
        contents(ui)
    })
}

pub fn toolbar_separator(ui: &mut egui::Ui) {
    let (line, _) = ui.allocate_exact_size(egui::vec2(1.0, 14.0), egui::Sense::hover());
    ui.painter().line_segment(
        [line.center_top(), line.center_bottom()],
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
}

pub fn icon_button(
    ui: &mut egui::Ui,
    glyph: &str,
    tooltip: &str,
    selected: bool,
    size: f32,
) -> egui::Response {
    let (rect, mut response) = ui.allocate_exact_size(
        egui::vec2(size.max(24.0), size.max(24.0)),
        egui::Sense::click(),
    );
    if response.clicked() {
        response.request_focus();
    }
    if response.has_focus()
        && ui.input(|input| {
            input.key_pressed(egui::Key::Space) || input.key_pressed(egui::Key::Enter)
        })
    {
        response.mark_changed();
    }
    let fill = if selected {
        colors::BG_SELECTION
    } else if response.hovered() || response.has_focus() {
        colors::BG_HOVER
    } else {
        egui::Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, layout::BUTTON_RADIUS, fill);
    if selected || response.has_focus() {
        ui.painter().rect_stroke(
            rect,
            layout::BUTTON_RADIUS,
            egui::Stroke::new(1.0_f32, colors::BORDER_ACTIVE),
        );
    }
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        glyph,
        egui::FontId::proportional((size * 0.58).clamp(13.0, 18.0)),
        if selected {
            colors::ACCENT_SELECTION
        } else {
            colors::TEXT_SECONDARY
        },
    );
    response.on_hover_text(crate::ui::localization::tr(tooltip))
}

pub fn svg_icon_button(
    ui: &mut egui::Ui,
    id: impl Into<String>,
    svg: &'static str,
    tooltip: &str,
    selected: bool,
    size: f32,
) -> egui::Response {
    let size = size.max(24.0);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::click());
    let fill = if selected {
        colors::BG_SELECTION
    } else if response.hovered() || response.has_focus() {
        colors::BG_HOVER
    } else {
        egui::Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, layout::BUTTON_RADIUS, fill);
    if selected || response.has_focus() {
        ui.painter().rect_stroke(
            rect,
            layout::BUTTON_RADIUS,
            egui::Stroke::new(1.0_f32, colors::BORDER_ACTIVE),
        );
    }
    let icon_size = (size * 0.58).clamp(13.0, 18.0);
    crate::ui::icons::render_svg_at(
        ui,
        id.into(),
        svg,
        egui::vec2(icon_size, icon_size),
        if selected {
            colors::ACCENT_SELECTION
        } else {
            colors::TEXT_SECONDARY
        },
        rect.center() - egui::vec2(icon_size * 0.5, icon_size * 0.5),
    );
    response.on_hover_text(crate::ui::localization::tr(tooltip))
}

pub fn segmented_button(
    ui: &mut egui::Ui,
    selected: bool,
    label: &str,
    tooltip: &str,
) -> egui::Response {
    tool_button(ui, selected, label).on_hover_text(crate::ui::localization::tr(tooltip))
}

pub fn badge(ui: &mut egui::Ui, label: &str, accent: egui::Color32) -> egui::Response {
    let label = crate::ui::localization::tr(label);
    let font = egui::FontId::proportional(10.0);
    let galley = ui
        .painter()
        .layout_no_wrap(label.clone(), font.clone(), colors::TEXT_ON_ACCENT);
    let size = galley.size() + egui::vec2(12.0, 6.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());
    ui.painter().rect_filled(rect, 4.0, accent);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        font,
        colors::TEXT_ON_ACCENT,
    );
    response
}

pub fn badge_button(
    ui: &mut egui::Ui,
    label: &str,
    accent: egui::Color32,
    tooltip: &str,
) -> egui::Response {
    let label = crate::ui::localization::tr(label);
    let font = egui::FontId::proportional(10.0);
    let galley = ui
        .painter()
        .layout_no_wrap(label.clone(), font.clone(), colors::TEXT_ON_ACCENT);
    let (rect, response) =
        ui.allocate_exact_size(galley.size() + egui::vec2(12.0, 6.0), egui::Sense::click());
    ui.painter().rect_filled(rect, 4.0, accent);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        font,
        colors::TEXT_ON_ACCENT,
    );
    response.on_hover_text(crate::ui::localization::tr(tooltip))
}

pub fn breadcrumb_link(ui: &mut egui::Ui, label: &str, tooltip: &str) -> egui::Response {
    button(ui, label, ButtonKind::Quiet).on_hover_text(crate::ui::localization::tr(tooltip))
}

pub fn breadcrumb_separator(ui: &mut egui::Ui) -> egui::Response {
    caption(ui, "›")
}

pub fn breadcrumb_current(ui: &mut egui::Ui, label: &str) -> egui::Response {
    section_title(ui, label)
}

#[derive(Clone, Copy)]
pub enum ButtonKind {
    Default,
    Accent,
    Quiet,
}

fn field_scope<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    ui.scope(|ui| {
        ui.spacing_mut().interact_size.y = layout::FIELD_HEIGHT;
        ui.spacing_mut().button_padding = egui::vec2(6.0, 4.0);
        let visuals = ui.visuals_mut();
        visuals.extreme_bg_color = colors::BG_SURFACE;
        for state in [
            &mut visuals.widgets.inactive,
            &mut visuals.widgets.hovered,
            &mut visuals.widgets.active,
            &mut visuals.widgets.open,
        ] {
            state.rounding = egui::Rounding::same(layout::FIELD_RADIUS);
            state.expansion = 0.0;
        }
        visuals.widgets.inactive.bg_fill = colors::BG_SURFACE;
        visuals.widgets.inactive.weak_bg_fill = colors::BG_SURFACE;
        visuals.widgets.inactive.bg_stroke.color = colors::BORDER_MEDIUM;
        visuals.widgets.hovered.bg_fill = colors::BG_HOVER;
        visuals.widgets.hovered.weak_bg_fill = colors::BG_HOVER;
        add(ui)
    })
    .inner
}

pub struct NumberField<'a> {
    widget: egui::DragValue<'a>,
    width: Option<f32>,
}

pub fn drag<N: egui::emath::Numeric>(value: &mut N) -> NumberField<'_> {
    NumberField {
        widget: egui::DragValue::new(value),
        width: None,
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
        self.widget = self.widget.speed(speed);
        self
    }
}

impl egui::Widget for NumberField<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        field_scope(ui, |ui| {
            ui.add_sized(
                [
                    self.width
                        .unwrap_or_else(|| ui.available_width().min(180.0)),
                    layout::FIELD_HEIGHT,
                ],
                self.widget,
            )
        })
    }
}

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
                    ui.label(
                        egui::RichText::new(axis)
                            .small()
                            .strong()
                            .color(axis_color),
                    );
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

fn with_button_style(
    ui: &mut egui::Ui,
    kind: ButtonKind,
    add: impl FnOnce(&mut egui::Ui) -> egui::Response,
) -> egui::Response {
    ui.scope(|ui| {
        let widgets = &mut ui.visuals_mut().widgets;
        for state in [
            &mut widgets.inactive,
            &mut widgets.hovered,
            &mut widgets.active,
        ] {
            state.rounding = egui::Rounding::same(layout::BUTTON_RADIUS);
            state.expansion = 0.0;
        }
        match kind {
            ButtonKind::Default => {
                widgets.inactive.weak_bg_fill = colors::BG_MID;
                widgets.hovered.weak_bg_fill = colors::BG_HOVER;
            }
            ButtonKind::Accent => {
                widgets.inactive.weak_bg_fill = colors::ACCENT_ACTION;
                widgets.hovered.weak_bg_fill = colors::ACCENT_ACTION_HOVER;
                widgets.active.weak_bg_fill = colors::ACCENT_ACTION;
                for state in [
                    &mut widgets.inactive,
                    &mut widgets.hovered,
                    &mut widgets.active,
                ] {
                    state.fg_stroke.color = colors::TEXT_ON_ACCENT;
                    state.bg_stroke.color = colors::ACCENT_ACTION_STROKE;
                }
            }
            ButtonKind::Quiet => {
                widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
                widgets.inactive.bg_stroke = egui::Stroke::NONE;
                widgets.hovered.weak_bg_fill = colors::BG_HOVER;
                widgets.hovered.bg_stroke = egui::Stroke::NONE;
                widgets.active.weak_bg_fill = colors::BG_SELECTION;
                widgets.active.bg_stroke = egui::Stroke::NONE;
            }
        }
        ui.spacing_mut().button_padding = egui::vec2(10.0, 5.0);
        add(ui)
    })
    .inner
}

pub fn button(ui: &mut egui::Ui, label: &str, kind: ButtonKind) -> egui::Response {
    button_min_size(ui, label, kind, egui::Vec2::ZERO)
}

pub fn button_min_size(
    ui: &mut egui::Ui,
    label: &str,
    kind: ButtonKind,
    min_size: egui::Vec2,
) -> egui::Response {
    with_button_style(ui, kind, |ui| {
        ui.add(
            egui::Button::new(egui::RichText::new(crate::ui::localization::tr(label)).small())
                .wrap()
                .min_size(min_size),
        )
    })
}

pub fn button_enabled(
    ui: &mut egui::Ui,
    label: &str,
    kind: ButtonKind,
    enabled: bool,
) -> egui::Response {
    ui.add_enabled_ui(enabled, |ui| button(ui, label, kind))
        .inner
}

pub fn button_enabled_min_size(
    ui: &mut egui::Ui,
    label: &str,
    kind: ButtonKind,
    enabled: bool,
    min_size: egui::Vec2,
) -> egui::Response {
    ui.add_enabled_ui(enabled, |ui| button_min_size(ui, label, kind, min_size))
        .inner
}

pub fn menu_button<R>(
    ui: &mut egui::Ui,
    label: &str,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<Option<R>> {
    ui.menu_button(crate::ui::localization::tr(label), contents)
}

pub fn menu_item(ui: &mut egui::Ui, label: &str) -> egui::Response {
    with_button_style(ui, ButtonKind::Quiet, |ui| {
        ui.add_sized(
            [ui.available_width(), layout::FIELD_HEIGHT],
            egui::Button::new(crate::ui::localization::tr(label)),
        )
    })
}

pub fn combo_popup<R>(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash,
    selected: &str,
    width: Option<f32>,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<Option<R>> {
    field_scope(ui, |ui| {
        let available = ui.available_width();
        egui::ComboBox::from_id_salt(id)
            .width(width.unwrap_or(available).min(available))
            .selected_text(crate::ui::localization::tr(selected))
            .truncate()
            .show_ui(ui, contents)
    })
}

pub fn tool_button(ui: &mut egui::Ui, active: bool, label: &str) -> egui::Response {
    with_button_style(ui, ButtonKind::Quiet, |ui| {
        ui.add(
            egui::Button::new(crate::ui::localization::tr(label))
                .selected(active)
                .truncate(),
        )
        .on_hover_text(crate::ui::localization::tr(label))
    })
}

pub fn selectable_row(ui: &mut egui::Ui, active: bool, label: &str, height: f32) -> egui::Response {
    let label = crate::ui::localization::tr(label);
    with_button_style(ui, ButtonKind::Quiet, |ui| {
        ui.add_sized(
            [ui.available_width(), height],
            egui::Button::new(label.clone()).selected(active).truncate(),
        )
        .on_hover_text(label)
    })
}

pub fn icon_label_button_at(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    label: &str,
    svg: &'static str,
    kind: ButtonKind,
) -> egui::Response {
    with_button_style(ui, kind, |ui| {
        let image = egui::Image::from_bytes(
            format!("bytes://kui-icon-{}.svg", egui::Id::new(svg).value()),
            svg.as_bytes(),
        )
        .fit_to_exact_size(egui::vec2(24.0, 24.0));
        ui.put(
            rect,
            egui::Button::image_and_text(image, egui::RichText::new(label).size(16.0))
                .wrap()
                .min_size(rect.size()),
        )
        .on_hover_text(label)
    })
}

pub fn section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(space::MD);
    section_title(ui, title);
    ui.add_space(space::SM);
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

fn parameter_slider_impl(
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
    let (rect, mut response) =
        ui.allocate_exact_size(egui::vec2(width, 24.0), egui::Sense::click_and_drag());
    if !minimum.is_finite() || !maximum.is_finite() || maximum <= minimum {
        return response;
    }
    let value_span = maximum - minimum;
    if !value_span.is_finite() {
        return response;
    }
    let step = step.filter(|step| step.is_finite() && *step > 0.0);

    let mut value_changed = !value.is_finite() || *value < minimum || *value > maximum;
    if value_changed {
        *value = if value.is_finite() {
            value.clamp(minimum, maximum)
        } else {
            minimum
        };
    }
    let value_before = *value;
    let value_width = if show_value { 68.0 } else { 5.0 };
    let track = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 5.0, rect.center().y - 1.5),
        egui::pos2(
            (rect.right() - value_width).max(rect.left() + 6.0),
            rect.center().y + 1.5,
        ),
    );
    if response.clicked() {
        response.request_focus();
    }
    if response.dragged() || response.clicked() {
        if let Some(pointer) = response.interact_pointer_pos() {
            let ratio = ((pointer.x - track.left()) / track.width().max(1.0)).clamp(0.0, 1.0);
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
            caption(ui, label.clone());
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

pub fn collapsing_section<R>(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash,
    title: &str,
    default_open: bool,
    body: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::CollapsingResponse<R> {
    egui::CollapsingHeader::new(crate::ui::localization::tr(title))
        .id_salt(id)
        .default_open(default_open)
        .show_background(true)
        .icon(|ui, openness, response| {
            let color = ui.style().interact(response).fg_stroke.color;
            let center = response.rect.center();
            let radius = 3.5;
            let angle = egui::remap_clamp(openness, 0.0..=1.0, 0.0..=std::f32::consts::FRAC_PI_2);
            let base = [
                egui::vec2(-radius * 0.65, -radius),
                egui::vec2(radius * 0.85, 0.0),
                egui::vec2(-radius * 0.65, radius),
            ];
            let points = base.map(|point| {
                let rotated = egui::emath::Rot2::from_angle(angle) * point;
                center + rotated
            });
            ui.painter().add(egui::Shape::convex_polygon(
                points.to_vec(),
                color,
                egui::Stroke::NONE,
            ));
        })
        .show(ui, body)
}

pub fn toggle(ui: &mut egui::Ui, value: &mut bool, label: &str) -> egui::Response {
    switch(ui, value, label)
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

pub fn panel<R>(
    ui: &mut egui::Ui,
    body: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    crate::ui::theme::panel_frame().show(ui, body)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_and_vector_fields_keep_values_and_fit_narrow_panels() {
        for width in [180.0, 360.0, 600.0] {
            let ctx = egui::Context::default();
            let mut integer = 1920_u32;
            let mut point = [-1234.0, 0.0, 1000.0];
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width, 500.0),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let bounds = ui.max_rect();
                        let integer_response =
                            ui.add(drag(&mut integer).range(16..=16384).unit(" px"));
                        assert!(bounds.contains_rect(integer_response.rect));
                        let vector_response = vector_axes(
                            ui,
                            &mut point,
                            -10000.0..=10000.0,
                            1.0,
                            "°",
                            ["P", "Y", "R"],
                        );
                        assert!(bounds.contains_rect(vector_response.rect));
                        let mut gain = 0.0;
                        let fader_response = vertical_fader(ui, &mut gain, -60.0..=12.0, 108.0);
                        assert!(bounds.contains_rect(fader_response.rect));
                        assert!((fader_response.rect.height() - 108.0).abs() < 0.1);
                        assert!(
                            !integer_response.changed()
                                && !vector_response.changed()
                                && !fader_response.changed()
                        );
                    });
                },
            );
            assert_eq!(integer, 1920);
            assert_eq!(point, [-1234.0, 0.0, 1000.0]);
        }
    }

    #[test]
    fn property_controls_align_and_stay_inside_panel() {
        for width in [180.0, 320.0, 600.0] {
            for scale in [1.0, 2.0] {
                let ctx = egui::Context::default();
                ctx.set_pixels_per_point(scale);
                let mut rects = Vec::new();
                let _ = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(width, 600.0),
                        )),
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            let bounds = ui.max_rect();
                            let grid = PropertyGrid::new(ui, &["Focus Distance", "焦点距離"]);
                            let mut value = 500.0;
                            for label in ["Focus Distance", "焦点距離"] {
                                let response = grid.row(ui, label, |ui| {
                                    number(ui, &mut value, 0.0..=10000.0, 1.0, " px")
                                });
                                assert!(
                                    bounds.contains_rect(response.rect),
                                    "{width} / {scale}: {:?}",
                                    response.rect
                                );
                                rects.push(response.rect);
                            }
                        });
                    },
                );
                assert!(
                    (rects[0].left() - rects[1].left()).abs() < 0.1,
                    "{width} / {scale}: {rects:?}"
                );
                assert!((rects[0].width() - rects[1].width()).abs() < 0.1);
                assert!(rects[0].bottom() <= rects[1].top());
            }
        }
    }

    #[test]
    fn hovered_button_background_precedes_text_and_keeps_rect() {
        let ctx = egui::Context::default();
        let mut rect = egui::Rect::NOTHING;
        let expected_label = crate::ui::localization::tr("Create");
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                rect = button(ui, "Create", ButtonKind::Accent).rect;
            });
        });
        let output = ctx.run(
            egui::RawInput {
                events: vec![egui::Event::PointerMoved(rect.center())],
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let response = button(ui, "Create", ButtonKind::Accent);
                    assert!(response.hovered());
                    assert_eq!(response.rect, rect);
                });
            },
        );
        let background = output
            .shapes
            .iter()
            .position(|shape| {
                matches!(&shape.shape,
            egui::Shape::Rect(shape) if shape.fill == colors::ACCENT_ACTION_HOVER)
            })
            .expect("hover fill");
        let text = output
            .shapes
            .iter()
            .position(|shape| {
                matches!(&shape.shape,
            egui::Shape::Text(shape) if shape.galley.job.text == expected_label)
            })
            .expect("button label");
        assert!(background < text);
    }

    #[test]
    fn disabled_button_has_no_interactive_hover_state() {
        let ctx = egui::Context::default();
        let mut rect = egui::Rect::NOTHING;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                rect = button_enabled(ui, "Undo", ButtonKind::Quiet, false).rect;
            });
        });
        let _ = ctx.run(
            egui::RawInput {
                events: vec![egui::Event::PointerMoved(rect.center())],
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let response = button_enabled(ui, "Undo", ButtonKind::Quiet, false);
                    assert!(!response.hovered());
                    assert!(!response.clicked());
                    assert_eq!(response.rect, rect);
                });
            },
        );
    }

    #[test]
    fn japanese_home_action_fits_reserved_rect() {
        let ctx = egui::Context::default();
        crate::ui::theme::configure_ae_theme(&ctx);
        crate::ui::icons::init_image_loaders(&ctx);
        let rect = egui::Rect::from_min_size(egui::pos2(20.0, 20.0), egui::vec2(220.0, 62.0));
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let response = icon_label_button_at(
                    ui,
                    rect,
                    "プロジェクトを開く",
                    crate::ui::icons::SVG_OPEN_FOLDER,
                    ButtonKind::Default,
                );
                assert_eq!(response.rect, rect);
            });
        });
        let text = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text == "プロジェクトを開く" => {
                    Some(text)
                }
                _ => None,
            })
            .expect("Japanese action label");
        assert!(rect.contains_rect(egui::Rect::from_min_size(text.pos, text.galley.size())));
    }
}
