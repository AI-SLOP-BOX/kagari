use crate::{animate_with_policy, MotionPolicy, Spring};
use egui::{
    vec2, Color32, Id, InnerResponse, Layout, Painter, Rect, Response, RichText, Sense, Stroke, Ui,
};
use std::ops::RangeInclusive;

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub background: Color32,
    pub surface: Color32,
    pub surface_raised: Color32,
    pub hover: Color32,
    pub selection: Color32,
    pub accent: Color32,
    pub accent_hover: Color32,
    pub border: Color32,
    pub text: Color32,
    pub text_secondary: Color32,
    pub text_muted: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub error: Color32,
    pub radius: u8,
    pub field_radius: u8,
    pub row_height: f32,
    pub field_height: f32,
    pub motion: MotionPolicy,
}

/// Bundled options for [`Theme::icon_button`] and the free [`icon_button`],
/// keeping their argument lists small and self-documenting.
#[derive(Clone, Copy, Debug)]
pub struct IconOptions<'a> {
    /// Glyph or character drawn as the icon.
    pub glyph: &'a str,
    /// Hover tooltip.
    pub tooltip: &'a str,
    /// Whether the icon renders in its selected (accented) state.
    pub selected: bool,
    /// Whether the button responds to input.
    pub enabled: bool,
    /// Square edge length in points.
    pub size: f32,
}

impl Default for IconOptions<'_> {
    fn default() -> Self {
        Self {
            glyph: "",
            tooltip: "",
            selected: false,
            enabled: true,
            size: 24.0,
        }
    }
}

impl Theme {
    pub const fn dark() -> Self {
        Self {
            background: Color32::from_rgb(18, 20, 24),
            surface: Color32::from_rgb(27, 30, 36),
            surface_raised: Color32::from_rgb(36, 40, 48),
            hover: Color32::from_rgb(47, 53, 62),
            selection: Color32::from_rgb(49, 70, 111),
            accent: Color32::from_rgb(92, 143, 255),
            accent_hover: Color32::from_rgb(120, 164, 255),
            border: Color32::from_rgb(61, 67, 77),
            text: Color32::from_rgb(239, 241, 245),
            text_secondary: Color32::from_rgb(182, 188, 200),
            text_muted: Color32::from_rgb(130, 138, 153),
            success: Color32::from_rgb(87, 193, 139),
            warning: Color32::from_rgb(232, 174, 83),
            error: Color32::from_rgb(228, 102, 102),
            radius: 8,
            field_radius: 5,
            row_height: 30.0,
            field_height: 28.0,
            motion: MotionPolicy::Full,
        }
    }

    pub const fn light() -> Self {
        Self {
            background: Color32::from_rgb(248, 249, 251),
            surface: Color32::from_rgb(255, 255, 255),
            surface_raised: Color32::from_rgb(241, 243, 246),
            hover: Color32::from_rgb(230, 234, 240),
            selection: Color32::from_rgb(218, 230, 250),
            accent: Color32::from_rgb(50, 105, 210),
            accent_hover: Color32::from_rgb(39, 88, 190),
            border: Color32::from_rgb(211, 216, 224),
            text: Color32::from_rgb(32, 37, 45),
            text_secondary: Color32::from_rgb(83, 91, 103),
            text_muted: Color32::from_rgb(120, 128, 140),
            success: Color32::from_rgb(36, 132, 87),
            warning: Color32::from_rgb(164, 105, 20),
            error: Color32::from_rgb(190, 57, 57),
            radius: 8,
            field_radius: 5,
            row_height: 30.0,
            field_height: 28.0,
            motion: MotionPolicy::Full,
        }
    }

    pub fn reduced_motion(mut self, reduced: bool) -> Self {
        self.motion = if reduced {
            MotionPolicy::Reduced
        } else {
            MotionPolicy::Full
        };
        self
    }

    pub fn with_accent(mut self, accent: Color32, hover: Color32) -> Self {
        self.accent = accent;
        self.accent_hover = hover;
        self
    }

    pub fn card<R>(&self, ui: &mut Ui, contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        egui::Frame::none()
            .fill(self.surface)
            .rounding(egui::Rounding::same(self.radius as f32))
            .stroke(Stroke::new(1.0_f32, self.border))
            .inner_margin(egui::Margin::same(10.0))
            .show(ui, contents)
    }

    pub fn section<R>(
        &self,
        ui: &mut Ui,
        id: impl std::hash::Hash,
        title: &str,
        default_open: bool,
        contents: impl FnOnce(&mut Ui) -> R,
    ) -> egui::CollapsingResponse<R> {
        let frame = egui::Frame::none()
            .fill(self.surface)
            .rounding(egui::Rounding::same(self.radius as f32))
            .stroke(Stroke::new(1.0_f32, self.border))
            .inner_margin(egui::Margin::same(10.0));
        frame
            .show(ui, |ui| {
                egui::CollapsingHeader::new(RichText::new(title).color(self.text).strong())
                    .id_salt(id)
                    .default_open(default_open)
                    .show(ui, contents)
            })
            .inner
    }

    pub fn separator(&self, ui: &mut Ui) -> Response {
        let (rect, response) =
            ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
        ui.painter().line_segment(
            [rect.left_center(), rect.right_center()],
            Stroke::new(1.0_f32, self.border),
        );
        response
    }

    pub fn title(&self, ui: &mut Ui, text: &str) -> Response {
        ui.label(RichText::new(text).size(18.0).strong().color(self.text))
    }

    pub fn body(&self, ui: &mut Ui, text: &str) -> Response {
        ui.label(RichText::new(text).size(13.0).color(self.text))
    }

    pub fn secondary(&self, ui: &mut Ui, text: &str) -> Response {
        ui.label(RichText::new(text).size(12.0).color(self.text_secondary))
    }

    pub fn caption(&self, ui: &mut Ui, text: &str) -> Response {
        ui.label(RichText::new(text).size(10.0).color(self.text_muted))
    }

    pub fn status(&self, ui: &mut Ui, kind: StatusKind, text: &str) -> Response {
        let color = match kind {
            StatusKind::Info => self.text_secondary,
            StatusKind::Success => self.success,
            StatusKind::Warning => self.warning,
            StatusKind::Error => self.error,
        };
        ui.label(RichText::new(text).size(11.0).color(color))
    }

    pub fn truncated(&self, ui: &mut Ui, text: &str) -> Response {
        ui.add(egui::Label::new(RichText::new(text).color(self.text_secondary)).truncate())
            .on_hover_text(text)
    }

    pub fn field<R>(&self, ui: &mut Ui, contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        ui.scope(|ui| {
            ui.spacing_mut().interact_size.y = self.field_height;
            ui.spacing_mut().button_padding = vec2(7.0, 4.0);
            ui.spacing_mut().item_spacing.x = 6.0;
            let visuals = ui.visuals_mut();
            visuals.extreme_bg_color = self.background;
            for widget in [
                &mut visuals.widgets.inactive,
                &mut visuals.widgets.hovered,
                &mut visuals.widgets.active,
                &mut visuals.widgets.open,
            ] {
                widget.rounding = egui::Rounding::same(self.field_radius as f32);
                widget.expansion = 0.0;
            }
            visuals.widgets.inactive.bg_fill = self.surface_raised;
            visuals.widgets.inactive.weak_bg_fill = self.surface_raised;
            visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, self.border);
            visuals.widgets.hovered.bg_fill = self.hover;
            visuals.widgets.hovered.weak_bg_fill = self.hover;
            visuals.widgets.active.bg_fill = self.selection;
            contents(ui)
        })
    }

    pub fn button(&self, ui: &mut Ui, text: &str, kind: ButtonKind) -> Response {
        self.button_enabled(ui, text, kind, true)
    }

    pub fn button_enabled(
        &self,
        ui: &mut Ui,
        text: &str,
        kind: ButtonKind,
        enabled: bool,
    ) -> Response {
        self.field(ui, |ui| {
            let widgets = &mut ui.visuals_mut().widgets;
            let (fill, hover, stroke, text_color) = match kind {
                ButtonKind::Default => (self.surface_raised, self.hover, self.border, self.text),
                ButtonKind::Accent => (
                    self.accent,
                    self.accent_hover,
                    self.accent,
                    Color32::from_rgb(10, 24, 26),
                ),
                ButtonKind::Quiet => (
                    Color32::TRANSPARENT,
                    self.hover,
                    Color32::TRANSPARENT,
                    self.text_secondary,
                ),
            };
            widgets.inactive.bg_fill = fill;
            widgets.inactive.weak_bg_fill = fill;
            widgets.inactive.bg_stroke = Stroke::new(1.0_f32, stroke);
            widgets.hovered.bg_fill = hover;
            widgets.hovered.weak_bg_fill = hover;
            widgets.hovered.bg_stroke = Stroke::new(1.0_f32, stroke);
            widgets.active.bg_fill = hover;
            widgets.active.weak_bg_fill = hover;
            widgets.active.bg_stroke = Stroke::new(1.0_f32, stroke);
            let button_text = if enabled { text_color } else { self.text_muted };
            let button = egui::Button::new(RichText::new(text).color(button_text))
                .rounding(egui::Rounding::same(self.radius as f32))
                .min_size(vec2(0.0, self.row_height));
            ui.add_enabled(enabled, button)
        })
        .inner
    }

    pub fn icon_button(
        &self,
        ui: &mut Ui,
        id: impl std::hash::Hash + std::fmt::Debug,
        opts: IconOptions<'_>,
    ) -> Response {
        let IconOptions {
            glyph,
            tooltip,
            selected,
            enabled,
            size,
        } = opts;
        self.icon_button_painted(
            ui,
            id,
            IconOptions {
                glyph: "",
                tooltip,
                selected,
                enabled,
                size,
            },
            |painter, rect, color| {
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    glyph,
                    egui::FontId::proportional(size * 0.56),
                    color,
                );
            },
        )
    }

    pub fn icon_button_painted(
        &self,
        ui: &mut Ui,
        id: impl std::hash::Hash + std::fmt::Debug,
        opts: IconOptions<'_>,
        paint_icon: impl FnOnce(&Painter, Rect, Color32),
    ) -> Response {
        let IconOptions {
            tooltip,
            selected,
            enabled,
            size,
            ..
        } = opts;
        let response = self
            .field(ui, |ui| {
                ui.add_enabled(
                    enabled,
                    egui::Button::new("")
                        .fill(Color32::TRANSPARENT)
                        .stroke(Stroke::NONE)
                        .min_size(vec2(size, size)),
                )
            })
            .inner;
        let hover = animate_with_policy(
            ui.ctx(),
            response.id.with(id),
            f32::from(response.hovered() && enabled && !selected),
            Spring::snappy(),
            self.motion,
        );
        let fill = if selected {
            self.selection
        } else {
            self.hover.gamma_multiply(hover * 0.75)
        };
        ui.painter()
            .rect_filled(response.rect, self.radius as f32, fill);
        if selected {
            ui.painter().rect_stroke(
                response.rect,
                self.radius as f32,
                Stroke::new(1.0_f32, self.accent),
            );
        }
        paint_icon(
            ui.painter(),
            response.rect.shrink(size * 0.22),
            if enabled { self.text } else { self.text_muted },
        );
        response.on_hover_text(tooltip)
    }

    pub fn tab(&self, ui: &mut Ui, selected: bool, label: &str) -> Response {
        self.field(ui, |ui| {
            let (fill, color) = if selected {
                (self.selection, self.text)
            } else {
                (Color32::TRANSPARENT, self.text_secondary)
            };
            ui.add(
                egui::Button::new(RichText::new(label).color(color))
                    .fill(fill)
                    .stroke(Stroke::NONE)
                    .rounding(egui::Rounding::same(self.radius as f32))
                    .min_size(vec2(0.0, self.row_height)),
            )
        })
        .inner
    }

    pub fn slider(
        &self,
        ui: &mut Ui,
        value: &mut f32,
        range: RangeInclusive<f32>,
        unit: &str,
    ) -> Response {
        self.field(ui, |ui| {
            ui.add_sized(
                [ui.available_width(), self.field_height],
                egui::Slider::new(value, range).suffix(unit),
            )
        })
        .inner
    }

    pub fn number(
        &self,
        ui: &mut Ui,
        value: &mut f32,
        range: RangeInclusive<f32>,
        speed: f64,
        unit: &str,
    ) -> Response {
        self.field(ui, |ui| {
            ui.add_sized(
                [ui.available_width().min(112.0), self.field_height],
                egui::DragValue::new(value)
                    .range(range)
                    .speed(speed)
                    .suffix(unit),
            )
        })
        .inner
    }

    pub fn combo<T: PartialEq + Clone>(
        &self,
        ui: &mut Ui,
        id: impl std::hash::Hash,
        value: &mut T,
        selected_label: &str,
        options: &[(T, &str)],
    ) -> Response {
        let mut changed = false;
        let response = self
            .field(ui, |ui| {
                egui::ComboBox::from_id_salt(id)
                    .width(ui.available_width().min(240.0))
                    .selected_text(selected_label)
                    .truncate()
                    .show_ui(ui, |ui| {
                        for (option, label) in options {
                            changed |= ui.selectable_value(value, option.clone(), *label).changed();
                        }
                    })
            })
            .inner
            .response;
        let mut response = response;
        if changed {
            response.mark_changed();
        }
        response
    }

    pub fn color(&self, ui: &mut Ui, value: &mut Color32) -> Response {
        self.field(ui, |ui| ui.color_edit_button_srgba(value)).inner
    }

    pub fn progress_bar(&self, ui: &mut Ui, fraction: f32, label: Option<&str>) -> Response {
        let (rect, response) =
            ui.allocate_exact_size(vec2(ui.available_width(), 16.0), Sense::hover());
        let painter = ui.painter().with_clip_rect(rect);
        painter.rect_filled(rect, 4.0, self.surface_raised);
        let width = rect.width() * fraction.clamp(0.0, 1.0);
        if width > 0.0 {
            painter.rect_filled(
                egui::Rect::from_min_size(rect.min, vec2(width, rect.height())),
                4.0,
                self.accent,
            );
        }
        if let Some(label) = label {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(10.0),
                self.text,
            );
        }
        response
    }

    pub fn level_meter(&self, ui: &mut Ui, level: f32, detail: &str) -> Response {
        let (rect, response) =
            ui.allocate_exact_size(vec2(ui.available_width(), 10.0), Sense::hover());
        let painter = ui.painter().with_clip_rect(rect);
        painter.rect_filled(rect, 3.0, self.surface_raised);
        let width = rect.width() * level.clamp(0.0, 1.0);
        if width > 0.0 {
            let color = if level >= 0.9 {
                self.error
            } else if level >= 0.72 {
                self.warning
            } else {
                self.accent
            };
            painter.rect_filled(
                egui::Rect::from_min_size(rect.min, vec2(width, rect.height())),
                3.0,
                color,
            );
        }
        response.on_hover_text(detail)
    }

    pub fn toggle(&self, ui: &mut Ui, value: &mut bool, label: &str) -> Response {
        self.field(ui, |ui| ui.add(egui::Checkbox::new(value, label)))
            .inner
    }

    pub fn selectable_row<R>(
        &self,
        ui: &mut Ui,
        id: impl std::hash::Hash + std::fmt::Debug,
        selected: bool,
        contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        // Allocate the row's space and click sense directly instead of dropping a
        // transparent Button widget. This keeps the selection/hover background from
        // being painted on top of sibling drawing (order is fully under our control),
        // and lets interactive child widgets added afterwards win input on their own
        // area while clicks on empty row space still select the row.
        let size = vec2(ui.available_width(), self.row_height);
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        let hover = animate_with_policy(
            ui.ctx(),
            response.id.with(id),
            f32::from(response.hovered()),
            Spring::new(10.0, 1.0),
            self.motion,
        );
        // Paint the background first so any contents drawn by the child Ui below are
        // always composited on top of it.
        if selected {
            ui.painter()
                .rect_filled(rect, self.field_radius as f32, self.selection);
            if hover > 0.001 {
                ui.painter().rect_filled(
                    rect,
                    self.field_radius as f32,
                    self.hover.gamma_multiply(hover * 0.35),
                );
            }
        } else if hover > 0.001 {
            ui.painter().rect_filled(
                rect,
                self.field_radius as f32,
                self.hover.gamma_multiply(hover),
            );
        }
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect.shrink2(vec2(8.0, 1.0)))
                .layout(Layout::left_to_right(egui::Align::Center)),
        );
        child.set_clip_rect(rect.shrink(1.0));
        let inner = contents(&mut child);
        InnerResponse { inner, response }
    }

    pub fn property_grid(&self, ui: &Ui, labels: &[&str]) -> PropertyGrid {
        PropertyGrid::new(ui, labels, self.text_secondary, self.field_height)
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonKind {
    Default,
    Accent,
    Quiet,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusKind {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Clone, Copy, Debug)]
pub struct PropertyGrid {
    label_width: f32,
    text_color: Color32,
    row_height: f32,
    min_control_width: f32,
}

impl PropertyGrid {
    pub fn new(ui: &Ui, labels: &[&str], text_color: Color32, row_height: f32) -> Self {
        let font = egui::TextStyle::Body.resolve(ui.style());
        let measured = labels
            .iter()
            .map(|label| {
                ui.painter()
                    .layout_no_wrap((*label).to_owned(), font.clone(), text_color)
                    .size()
                    .x
            })
            .fold(0.0, f32::max);
        Self {
            label_width: measured.min(ui.available_width() * 0.42),
            text_color,
            row_height,
            min_control_width: 120.0,
        }
    }

    pub fn from_measured_width(
        label_width: f32,
        text_color: Color32,
        row_height: f32,
        min_control_width: f32,
    ) -> Self {
        Self {
            label_width: label_width.max(0.0),
            text_color,
            row_height,
            min_control_width: min_control_width.max(1.0),
        }
    }

    pub fn row<R>(&self, ui: &mut Ui, label: &str, control: impl FnOnce(&mut Ui) -> R) -> R {
        let gap = ui.spacing().item_spacing.x;
        let available = ui.available_width();
        let min_control_width = self.min_control_width;
        if available < self.label_width + gap + min_control_width {
            ui.vertical(|ui| {
                ui.add_sized(
                    [ui.available_width(), self.row_height],
                    egui::Label::new(RichText::new(label).color(self.text_color)).truncate(),
                )
                .on_hover_text(label);
                control(ui)
            })
            .inner
        } else {
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(
                    vec2(self.label_width, self.row_height),
                    Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.set_min_width(self.label_width);
                        ui.add(
                            egui::Label::new(RichText::new(label).color(self.text_color))
                                .truncate(),
                        )
                        .on_hover_text(label);
                    },
                );
                ui.allocate_ui_with_layout(
                    vec2(
                        (available - self.label_width - gap).max(min_control_width),
                        self.row_height,
                    ),
                    Layout::left_to_right(egui::Align::Center),
                    control,
                )
                .inner
            })
            .inner
        }
    }
}

/// A numeric field you scrub by dragging and type into by double-clicking.
///
/// Drag horizontally to change the value, hold `Shift` for fine control, and
/// double-click to type an exact number. Hover and drag feedback share the
/// kit's springs, so the feel matches the buttons and rows.
#[derive(Clone, Copy, Debug)]
pub struct ScrubField {
    theme: Theme,
    min: f32,
    max: f32,
    speed: f64,
    precision: usize,
    suffix: &'static str,
    id: Option<Id>,
    width: f32,
}

impl ScrubField {
    pub fn new(theme: Theme) -> Self {
        Self {
            theme,
            min: 0.0,
            max: 1.0,
            speed: 0.01,
            precision: 2,
            suffix: "",
            id: None,
            width: 112.0,
        }
    }

    pub fn range(mut self, range: RangeInclusive<f32>) -> Self {
        self.min = *range.start();
        self.max = *range.end();
        self
    }

    /// Value change per pixel of horizontal drag. `Shift` divides this by ten.
    pub fn speed(mut self, speed: f64) -> Self {
        self.speed = speed.max(0.0);
        self
    }

    pub fn precision(mut self, precision: usize) -> Self {
        self.precision = precision.min(9);
        self
    }

    pub fn suffix(mut self, suffix: &'static str) -> Self {
        self.suffix = suffix;
        self
    }

    pub fn id(mut self, id: impl std::hash::Hash + std::fmt::Debug) -> Self {
        self.id = Some(Id::new(id));
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width.max(24.0);
        self
    }

    fn bounds(&self) -> (f64, f64) {
        if self.min <= self.max {
            (self.min as f64, self.max as f64)
        } else {
            (self.max as f64, self.min as f64)
        }
    }

    fn clamp(&self, value: f64) -> f32 {
        let (lo, hi) = self.bounds();
        value.clamp(lo, hi) as f32
    }

    fn display(&self, value: f32) -> String {
        format!("{:.*}{}", self.precision, value, self.suffix)
    }

    fn paint_field(&self, ui: &Ui, rect: Rect, hover: f32, dragging: bool) {
        let painter = ui.painter();
        let radius = self.theme.field_radius as f32;
        painter.rect_filled(rect, radius, self.theme.surface_raised);
        if hover > 0.001 {
            painter.rect_filled(rect, radius, self.theme.hover.gamma_multiply(hover * 0.5));
        }
        let stroke = if dragging {
            Stroke::new(2.0_f32, self.theme.accent)
        } else {
            Stroke::new(1.0_f32, self.theme.border)
        };
        painter.rect_stroke(rect, radius, stroke);
    }

    fn paint_value(&self, ui: &Ui, rect: Rect, value: f32) {
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            self.display(value),
            egui::FontId::proportional(12.0),
            self.theme.text,
        );
    }

    fn close_editor(&self, ui: &Ui, base: Id) {
        ui.data_mut(|data| {
            data.insert_temp::<bool>(base.with("editing"), false);
            data.remove_temp::<bool>(base.with("focused"));
        });
    }

    /// Draw the field. Returns the response and whether `value` changed this frame.
    pub fn show(&self, ui: &mut Ui, value: &mut f32) -> (Response, bool) {
        let id = self.id.unwrap_or_else(|| Id::new("choegui_scrub_field"));
        let editing = ui
            .data(|data| data.get_temp::<bool>(id.with("editing")))
            .unwrap_or(false);

        let size = vec2(self.width, self.theme.field_height);
        let sense = if editing {
            Sense::click()
        } else {
            Sense::click_and_drag()
        };
        let (rect, response) = ui.allocate_exact_size(size, sense);

        let hover = animate_with_policy(
            ui.ctx(),
            response.id.with("hover"),
            f32::from(response.hovered()),
            Spring::new(10.0, 1.0),
            self.theme.motion,
        );
        let dragging = response.dragged();

        self.paint_field(ui, rect, hover, dragging);
        let mut changed = false;

        if editing {
            let mut buffer = ui
                .data(|data| data.get_temp::<String>(id.with("buffer")))
                .unwrap_or_else(|| self.display(*value));
            let editor = egui::TextEdit::singleline(&mut buffer)
                .id(id.with("editor"))
                .font(egui::FontId::proportional(12.0))
                .desired_width(self.width - 12.0)
                .horizontal_align(egui::Align::Center);
            let inner = ui
                .scope_builder(
                    egui::UiBuilder::new().max_rect(rect.shrink2(vec2(6.0, 1.0))),
                    |ui| ui.add(editor),
                )
                .inner;
            ui.data_mut(|data| data.insert_temp(id.with("buffer"), buffer.clone()));
            // Commit when the editor loses focus (Enter or click-away).
            if inner.lost_focus() {
                let trimmed = buffer.trim();
                let numeric: String = trimmed
                    .chars()
                    .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-' || *c == '+')
                    .collect();
                if let Ok(parsed) = numeric.parse::<f32>() {
                    *value = self.clamp(parsed as f64);
                    changed = true;
                }
                self.close_editor(ui, id);
            }
        } else {
            self.paint_value(ui, rect, *value);
            if response.double_clicked() {
                ui.data_mut(|data| {
                    data.insert_temp(id.with("editing"), true);
                    data.insert_temp(id.with("buffer"), self.display(*value));
                    data.insert_temp(id.with("focused"), true);
                });
            }
            if dragging {
                let delta = response.drag_delta().x as f64;
                let fine = ui.input(|i| i.modifiers.shift);
                let step = self.speed * if fine { 0.1 } else { 1.0 };
                let next = *value as f64 + delta * step;
                let clamped = self.clamp(next);
                if (clamped - *value).abs() > f32::EPSILON {
                    *value = clamped;
                    changed = true;
                }
            }
        }

        (response, changed)
    }
}

pub fn field<R>(
    ui: &mut Ui,
    theme: &Theme,
    contents: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<R> {
    theme.field(ui, contents)
}

pub fn property<R>(
    ui: &mut Ui,
    theme: &Theme,
    label: &str,
    control: impl FnOnce(&mut Ui) -> R,
) -> R {
    theme.property_grid(ui, &[label]).row(ui, label, control)
}

pub fn icon_button(
    ui: &mut Ui,
    theme: &Theme,
    id: impl std::hash::Hash + std::fmt::Debug,
    opts: IconOptions<'_>,
) -> Response {
    theme.icon_button(ui, id, opts)
}

pub fn selectable_row<R>(
    ui: &mut Ui,
    theme: &Theme,
    id: impl std::hash::Hash + std::fmt::Debug,
    selected: bool,
    contents: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<R> {
    theme.selectable_row(ui, id, selected, contents)
}

pub fn stable_id(id: impl std::hash::Hash + std::fmt::Debug) -> Id {
    Id::new(id)
}

#[cfg(test)]
mod scrub_field_tests {
    use super::*;

    fn field() -> ScrubField {
        ScrubField::new(Theme::default())
    }

    #[test]
    fn test_default_range_clamps_to_unit_interval() {
        let f = field();
        assert_eq!(f.clamp(-5.0), 0.0);
        assert_eq!(f.clamp(5.0), 1.0);
        assert_eq!(f.clamp(0.5), 0.5);
    }

    #[test]
    fn test_reversed_range_is_normalized() {
        let f = field().range(10.0..=0.0);
        assert_eq!(f.bounds(), (0.0, 10.0));
        assert_eq!(f.clamp(-1.0), 0.0);
        assert_eq!(f.clamp(20.0), 10.0);
        assert_eq!(f.clamp(3.0), 3.0);
    }

    #[test]
    fn test_equal_bounds_collapse_to_single_value() {
        let f = field().range(4.0..=4.0);
        assert_eq!(f.bounds(), (4.0, 4.0));
        assert_eq!(f.clamp(99.0), 4.0);
    }

    #[test]
    fn test_negative_range_is_supported() {
        let f = field().range(-100.0..=100.0);
        assert_eq!(f.bounds(), (-100.0, 100.0));
        assert_eq!(f.clamp(-500.0), -100.0);
        assert_eq!(f.clamp(500.0), 100.0);
        assert_eq!(f.clamp(-42.5), -42.5);
    }

    #[test]
    fn test_display_honours_precision_and_suffix() {
        let f = field().precision(3).suffix(" px");
        assert_eq!(f.display(1.5), "1.500 px");
    }

    #[test]
    fn test_precision_is_capped() {
        let f = field().precision(50);
        assert_eq!(f.display(1.0), "1.000000000");
    }

    #[test]
    fn test_speed_and_width_reject_negative() {
        let f = field().speed(-1.0).width(-40.0);
        assert_eq!(f.speed, 0.0);
        assert_eq!(f.width, 24.0);
    }
}
