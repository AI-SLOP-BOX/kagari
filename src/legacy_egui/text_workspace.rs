//! Text and typography workspace layout, title presets, inspector, and preview.
//!
//! Submodules:
//! - [`nav`]: Sidebar logo branding, icon navigation, and asset loading.
//! - [`presets_preview`]: Preset cards, search, and live text preview with transport controls.
//! - [`inspector`]: Property inspector controls and mini timeline representation.

pub mod inspector;
pub mod nav;
pub mod presets_preview;

pub use inspector::*;
pub use nav::*;
pub use presets_preview::*;

use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub fn draw(app: &mut KagariApp, ctx: &egui::Context) {
    let width = ctx.screen_rect().width();
    let narrow = width < 1100.0;
    let left_w = if narrow { 72.0 } else { 231.0 };
    let preset_w = if narrow { 230.0 } else { 364.0 };
    egui::TopBottomPanel::top("text_workspace_window_bar")
        .exact_height(43.0)
        .resizable(false)
        .frame(egui::Frame::none().fill(crate::ui::theme::colors::BG_PANEL_DARK))
        .show(ctx, |ui| {
            for (x, c) in [
                (23.0, crate::ui::theme::colors::METER_BAD),
                (46.0, crate::ui::theme::colors::METER_WARNING),
                (69.0, crate::ui::theme::colors::METER_GOOD),
            ] {
                ui.painter().circle_filled(egui::pos2(x, 22.0), 7.0, c);
            }
            ui.painter().text(
                egui::pos2(95.0, 22.0),
                egui::Align2::LEFT_CENTER,
                "Kagari VFX",
                egui::FontId::proportional(14.0),
                colors::TEXT_PRIMARY,
            );
            ui.painter().text(
                egui::pos2(width - 28.0, 22.0),
                egui::Align2::RIGHT_CENTER,
                "Create. Composite. Illuminate.",
                egui::FontId::proportional(13.0),
                colors::TEXT_MUTED,
            );
        });
    egui::CentralPanel::default()
        .frame(egui::Frame::none().fill(crate::ui::theme::colors::BG_DEEPEST))
        .show(ctx, |ui| {
            let r = ui.max_rect();
            let sidebar = egui::Rect::from_min_max(r.min, egui::pos2(left_w, r.bottom()));
            let preset = egui::Rect::from_min_max(
                egui::pos2(left_w, r.top() + 62.0),
                egui::pos2(left_w + preset_w, r.bottom()),
            );
            let right_w = if narrow { 250.0 } else { 384.0 };
            let center = egui::Rect::from_min_max(
                egui::pos2(preset.right(), r.top() + 62.0),
                egui::pos2(r.right() - right_w, r.bottom()),
            );
            let inspector = egui::Rect::from_min_max(center.right_top(), r.right_bottom());
            let timeline_top = r.bottom() - if narrow { 260.0 } else { 316.0 };
            let border = egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE);
            ui.painter()
                .rect_filled(sidebar, 0.0, crate::ui::theme::colors::BG_PANEL_BASE);
            ui.painter().line_segment(
                [egui::pos2(left_w, 0.0), egui::pos2(left_w, r.bottom())],
                border,
            );
            ui.painter().line_segment(
                [
                    egui::pos2(preset.right(), 0.0),
                    egui::pos2(preset.right(), r.bottom()),
                ],
                border,
            );
            ui.painter().line_segment(
                [
                    egui::pos2(center.right(), 0.0),
                    egui::pos2(center.right(), r.bottom()),
                ],
                border,
            );
            draw_logo(app, ctx, ui, sidebar, narrow);
            draw_nav(ui, sidebar, r, narrow);
            draw_presets(ui, ctx, preset);
            draw_preview(app, ui, ctx, center, timeline_top, narrow);
            draw_inspector(app, ui, inspector, narrow);
            draw_timeline(app, ui, timeline_top, center.left(), r.right(), narrow);
        });
}
