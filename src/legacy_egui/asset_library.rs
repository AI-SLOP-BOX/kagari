#![allow(clippy::possible_missing_else)]

//! Asset library browser and manager.
//!
//! Submodules:
//! - [`stock`]: Stock assets pack dialog and file copier.
//! - [`nav`]: Top header bar, breadcrumbs, and folder hierarchy.
//! - [`grid`]: Center grid card display with asset type filters.
//! - [`detail`]: Right-side inspector pane for asset metadata.

pub mod detail;
pub mod grid;
pub mod nav;
pub mod stock;

#[cfg(test)]
mod tests;

pub use detail::*;
pub use grid::*;
pub use nav::*;
pub use stock::*;

use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub mod metrics {
    pub const HEADER_HEIGHT: f32 = 176.0;
    pub const FOOTER_INSET: f32 = 18.0;
    pub const PANEL_GUTTER: f32 = 14.0;
    pub const PANEL_HEADER_HEIGHT: f32 = 52.0;
    pub const FOLDER_ROW_COMPACT: f32 = 28.0;
    pub const FOLDER_ROW: f32 = 34.0;
    pub const CARD_GAP: f32 = 12.0;
    pub const CARD_HEIGHT_COMPACT: f32 = 104.0;
    pub const CARD_HEIGHT: f32 = 126.0;
}

pub fn draw(app: &mut KagariApp, ctx: &egui::Context) {
    let width = ctx.screen_rect().width();
    let compact = width < 780.0;
    egui::CentralPanel::default()
        .frame(egui::Frame::none().fill(crate::ui::theme::colors::BG_DEEPEST))
        .show(ctx, |ui| {
            let r = ui.max_rect();
            let folder_w = (r.width() * 0.19).clamp(if compact { 72.0 } else { 164.0 }, 244.0);
            let detail_w = if compact {
                0.0
            } else {
                (r.width() * 0.24).clamp(248.0, 384.0)
            };
            let folder = egui::Rect::from_min_max(
                egui::pos2(r.left(), r.top() + metrics::HEADER_HEIGHT),
                egui::pos2(r.left() + folder_w, r.bottom() - metrics::FOOTER_INSET),
            );
            let grid = egui::Rect::from_min_max(
                folder.right_top(),
                egui::pos2(r.right() - detail_w, r.bottom() - metrics::FOOTER_INSET),
            );
            let detail = egui::Rect::from_min_max(
                egui::pos2(grid.right(), r.top() + metrics::HEADER_HEIGHT),
                egui::pos2(r.right(), r.bottom() - metrics::FOOTER_INSET),
            );
            let border = egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE);
            ui.painter()
                .rect_filled(folder, 0.0, crate::ui::theme::colors::BG_PANEL_BASE);
            ui.painter().line_segment(
                [
                    egui::pos2(folder.right(), 0.0),
                    egui::pos2(folder.right(), r.bottom()),
                ],
                border,
            );
            if detail_w > 0.0 {
                ui.painter().line_segment(
                    [
                        egui::pos2(grid.right(), 0.0),
                        egui::pos2(grid.right(), r.bottom()),
                    ],
                    border,
                );
            }
            draw_header(app, ui, r, folder.left(), compact);
            draw_folders(app, ui, ctx, folder);
            draw_grid(app, ui, ctx, grid, compact);
            if detail_w > 0.0 {
                draw_detail(app, ui, ctx, detail, compact);
            }
        });
    draw_stock_assets_dialog(app, ctx);
}
