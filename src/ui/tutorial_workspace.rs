//! Tutorial workspace interface and interactive walkthrough launcher.
//!
//! Submodules:
//! - [`bars`]: Top bar (actions/window controls) and bottom footer bar.
//! - [`panels`]: Left chapter navigation and right "About Kagari" info card.
//! - [`practice`]: Main hero area, embedded lesson outlines, and video practice launcher.

pub mod bars;
pub mod panels;
pub mod practice;

pub use bars::*;
pub use panels::*;
pub use practice::*;

use crate::KagariApp;
use eframe::egui;

pub const ORANGE: egui::Color32 = crate::ui::theme::colors::ACCENT_BRAND;
pub const PANEL: egui::Color32 = egui::Color32::from_rgb(15, 24, 31);

pub fn selected_chapter(step: usize) -> usize {
    crate::ui::tutorial::chapters()
        .iter()
        .enumerate()
        .rev()
        .find(|(_, chapter)| chapter.first_step <= step)
        .map(|(index, _)| index)
        .unwrap_or(0)
}

pub fn draw(app: &mut KagariApp, ctx: &egui::Context) {
    let width = ctx.screen_rect().width();
    let compact = width < 1100.0;
    let mobile = width < 760.0;
    egui::TopBottomPanel::top("tutorial_window_bar")
        .exact_height(if mobile { 52.0 } else { 60.0 })
        .frame(egui::Frame::none().fill(egui::Color32::from_rgb(15, 23, 30)))
        .show(ctx, |ui| draw_topbar(ui, width, mobile, app, ctx));
    egui::TopBottomPanel::bottom("tutorial_footer")
        .exact_height(if mobile { 38.0 } else { 68.0 })
        .frame(egui::Frame::none().fill(crate::ui::theme::colors::BG_DEEPEST))
        .show(ctx, |ui| draw_footer(ui, width, mobile));
    egui::CentralPanel::default()
        .frame(egui::Frame::none().fill(egui::Color32::from_rgb(8, 15, 21)))
        .show(ctx, |ui| {
            let rect = ui.max_rect();
            let screen_clip = ui.clip_rect();
            let nav_w = if mobile {
                0.0
            } else if compact {
                230.0
            } else {
                340.0
            };
            let right_w = if mobile || compact { 0.0 } else { 498.0 };
            if nav_w > 0.0 {
                ui.set_clip_rect(
                    egui::Rect::from_min_max(
                        rect.min,
                        egui::pos2(rect.left() + nav_w, rect.bottom()),
                    )
                    .intersect(screen_clip),
                );
                draw_chapters(ui, rect, nav_w, compact, app);
            }
            let main = egui::Rect::from_min_max(
                egui::pos2(rect.left() + nav_w, rect.top()),
                egui::pos2(rect.right() - right_w, rect.bottom()),
            );
            ui.set_clip_rect(main.intersect(screen_clip));
            draw_main(ui, ctx, main, compact, mobile, app);
            if right_w > 0.0 {
                let about = egui::Rect::from_min_max(
                    egui::pos2(rect.right() - right_w, rect.top()),
                    rect.max,
                );
                ui.set_clip_rect(about.intersect(screen_clip));
                draw_about(ui, about, rect.right() - right_w, right_w, compact);
            }
            ui.set_clip_rect(screen_clip);
        });
}

pub fn texture(ctx: &egui::Context, path: &str, key: &str) -> Option<egui::TextureId> {
    let id = egui::Id::new(("tutorial-texture", key));
    if let Some(handle) = ctx.data_mut(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return Some(handle.id());
    }
    let image = crate::ui::embedded_assets::open_image(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path),
    )
    .ok()?
    .to_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    let handle = ctx.load_texture(
        key,
        egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw()),
        egui::TextureOptions::LINEAR,
    );
    let out = handle.id();
    ctx.data_mut(|d| d.insert_temp(id, handle));
    Some(out)
}
