use crate::ui::theme::{colors, layout};
use eframe::egui;

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

pub fn panel<R>(
    ui: &mut egui::Ui,
    body: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    crate::ui::theme::panel_frame().show(ui, body)
}

pub fn section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(space::MD);
    section_title(ui, title);
    ui.add_space(space::SM);
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

pub fn field_scope<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
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
