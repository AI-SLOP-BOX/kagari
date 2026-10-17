use crate::ui::icons;
use crate::ui::theme::colors;
use eframe::egui;

/// Minimal hotspot interaction response
#[derive(Debug, Clone, Copy)]
pub struct HotspotResponse {
    pub clicked: bool,
    pub secondary_clicked: bool,
    pub hovered: bool,
    pub drag_delta: egui::Vec2,
    pub is_dragging: bool,
    pub rect: egui::Rect,
}

impl HotspotResponse {
    pub fn clicked(&self) -> bool {
        self.clicked
    }
    pub fn secondary_clicked(&self) -> bool {
        self.secondary_clicked
    }
    pub fn hovered(&self) -> bool {
        self.hovered
    }
    pub fn drag_delta(&self) -> egui::Vec2 {
        self.drag_delta
    }
    pub fn is_dragging(&self) -> bool {
        self.is_dragging
    }
}

/// Creates a minimal interaction hotspot with arbitrary size.
/// No heavy layout frames, no bloated containers.
pub fn hotspot(ui: &mut egui::Ui, id_salt: impl std::hash::Hash, size: egui::Vec2) -> (egui::Rect, egui::Response) {
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
    let id = ui.make_persistent_id(id_salt);
    let resp = ui.interact(rect, id, egui::Sense::click_and_drag()).union(resp);
    (rect, resp)
}

/// One-liner SVG Action Button:
/// Places an SVG in a minimal hotspot with hover feedback and returns click/drag events.
pub fn svg_hotspot(
    ui: &mut egui::Ui,
    id_name: &str,
    svg_str: &'static str,
    size: egui::Vec2,
    active: bool,
    tooltip: &str,
) -> HotspotResponse {
    let (rect, resp) = hotspot(ui, id_name, size);
    let hovered = resp.hovered();
    let clicked = resp.clicked();
    let secondary_clicked = resp.secondary_clicked();
    let is_dragging = resp.dragged();
    let drag_delta = if is_dragging { resp.drag_delta() } else { egui::Vec2::ZERO };

    // Responsive tint
    let tint = if active {
        colors::ACCENT_ORANGE
    } else if hovered {
        egui::Color32::WHITE
    } else {
        colors::TEXT_SECONDARY
    };

    // Draw background hover glow if hovered or active
    if active {
        ui.painter().rect_filled(
            rect.expand(2.0),
            4.0,
            egui::Color32::from_rgba_unmultiplied(255, 120, 20, 45),
        );
    } else if hovered {
        ui.painter().rect_filled(
            rect.expand(2.0),
            4.0,
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 20),
        );
    }

    // Paint SVG
    icons::render_svg_at(
        ui,
        id_name.to_string(),
        svg_str,
        size,
        tint,
        rect.min,
    );

    if !tooltip.is_empty() {
        resp.on_hover_text(tooltip);
    }

    HotspotResponse {
        clicked,
        secondary_clicked,
        hovered,
        drag_delta,
        is_dragging,
        rect,
    }
}

/// One-liner SVG Drag-Value:
/// An SVG icon that acts as a scrubber/drag controller (e.g. scrub font size, scrub time, scrub tracking).
pub fn svg_drag_value<T>(
    ui: &mut egui::Ui,
    id_name: &str,
    svg_str: &'static str,
    val: &mut T,
    speed: f32,
    range: std::ops::RangeInclusive<T>,
    tooltip: &str,
) -> bool
where
    T: Copy + PartialOrd + std::ops::AddAssign + std::ops::SubAssign + From<f32> + Into<f32>,
{
    let is_active = hs_is_active(ui, id_name);
    let hs = svg_hotspot(ui, id_name, svg_str, egui::vec2(18.0, 18.0), is_active, tooltip);
    if hs.hovered() || hs.is_dragging {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
    }
    set_hs_active(ui, id_name, hs.is_dragging);
    if hs.is_dragging && hs.drag_delta.x.abs() > 0.001 {
        let speed_mult = if ui.input(|i| i.modifiers.shift) {
            10.0
        } else if ui.input(|i| i.modifiers.alt) {
            0.1
        } else {
            1.0
        };
        let current_f: f32 = (*val).into();
        let delta = hs.drag_delta.x * speed * speed_mult;
        let min_f: f32 = (*range.start()).into();
        let max_f: f32 = (*range.end()).into();
        let next_f = (current_f + delta).clamp(min_f, max_f);
        *val = T::from(next_f);
        return true;
    }
    false
}

fn hs_is_active(ui: &egui::Ui, id_name: &str) -> bool {
    let key = egui::Id::new("hs_active_tag").with(id_name);
    ui.ctx().data(|d| d.get_temp::<bool>(key).unwrap_or(false))
}

fn set_hs_active(ui: &egui::Ui, id_name: &str, active: bool) {
    let key = egui::Id::new("hs_active_tag").with(id_name);
    ui.ctx().data_mut(|d| {
        if active {
            d.insert_temp(key, true);
        } else {
            d.remove_temp::<bool>(key);
        }
    });
}

/// Micro Action Tag:
/// Text badge that is clickable and acts as a direct action trigger without needing a full button frame.
pub fn micro_tag(
    ui: &mut egui::Ui,
    label: &str,
    active: bool,
    tooltip: &str,
) -> bool {
    let font_id = egui::FontId::proportional(11.0);
    let galley = ui.painter().layout_no_wrap(label.to_string(), font_id, egui::Color32::WHITE);
    let pad = egui::vec2(6.0, 3.0);
    let size = galley.size() + pad * 2.0;
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let clicked = resp.clicked();
    let (bg, stroke, text_color) = if active {
        (
            egui::Color32::from_rgba_unmultiplied(255, 120, 20, 60),
            egui::Stroke::new(1.0_f32, colors::ACCENT_ORANGE),
            egui::Color32::WHITE,
        )
    } else if resp.hovered() {
        (
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 25),
            egui::Stroke::new(1.0_f32, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 80)),
            egui::Color32::WHITE,
        )
    } else {
        (
            egui::Color32::from_rgba_unmultiplied(35, 42, 52, 180),
            egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
            colors::TEXT_SECONDARY,
        )
    };

    ui.painter().rect(rect, 4.0, bg, stroke);
    ui.painter().galley(rect.min + pad, galley, text_color);

    if !tooltip.is_empty() {
        resp.on_hover_text(tooltip);
    }

    clicked
}
