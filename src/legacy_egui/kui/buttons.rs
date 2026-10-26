use super::typography_layout::{caption, field_scope, section_title};
use crate::ui::theme::{colors, layout};
use eframe::egui;

#[derive(Clone, Copy)]
pub enum ButtonKind {
    Default,
    Accent,
    Quiet,
}

pub fn with_button_style(
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
