mod center;
mod constants;
mod detail;
mod helpers;
mod sidebar;
mod topbar;

use crate::KagariApp;
use constants::BG;
use eframe::egui;

pub fn draw(app: &mut KagariApp, ctx: &egui::Context) {
    let width = ctx.screen_rect().width();
    let compact = width < 1350.0;
    let mobile = width < 760.0;
    egui::TopBottomPanel::top("effects_workspace_bar")
        .exact_height(if mobile { 52.0 } else { 60.0 })
        .frame(egui::Frame::none().fill(egui::Color32::from_rgb(14, 23, 30)))
        .show(ctx, |ui| topbar::draw_topbar(ui, width, mobile, app, ctx));
    egui::TopBottomPanel::bottom("effects_workspace_footer")
        .exact_height(if mobile { 34.0 } else { 50.0 })
        .frame(egui::Frame::none().fill(crate::ui::theme::colors::BG_DEEPEST))
        .show(ctx, |ui| detail::draw_footer(ui, width, mobile));
    egui::CentralPanel::default()
        .frame(egui::Frame::none().fill(BG))
        .show(ctx, |ui| {
            let rect = ui.max_rect();
            // At compact widths the category rail collapses so the effect grid keeps a usable width.
            let left_w = if mobile || compact { 0.0 } else { 293.0 };
            let right_w = if mobile || compact { 0.0 } else { 430.0 };
            if left_w > 0.0 {
                sidebar::draw_sidebar(ui, rect, left_w, compact, app);
            }
            let center = egui::Rect::from_min_max(
                egui::pos2(rect.left() + left_w, rect.top()),
                egui::pos2(rect.right() - right_w, rect.bottom()),
            );
            center::draw_center(ui, ctx, center, compact, mobile, app);
            if right_w > 0.0 {
                detail::draw_detail(
                    ui,
                    ctx,
                    egui::Rect::from_min_max(egui::pos2(center.right(), rect.top()), rect.max),
                    compact,
                    app,
                );
            }
        });
}
