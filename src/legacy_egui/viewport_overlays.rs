#![allow(clippy::too_many_arguments)]

//! Viewport overlay renderers (grid, motion paths, guides, HUD badges, transform gizmos, split wipe).
//!
//! Submodules:
//! - [`guides_hud`]: Grid lines, safe action/title guides, performance HUD, and notice badges.
//! - [`motion_path`]: Position keyframe paths, bezier tangent handles, and puppet pin paths.
//! - [`gizmos`]: Bounding box transform handles, 3D axis gizmos, and vector mask vertex overlays.
//! - [`split_wipe`]: Snapshot A/B comparison draggable split wipe line.

pub mod gizmos;
pub mod guides_hud;
pub mod motion_path;
pub mod split_wipe;

pub use gizmos::*;
pub use guides_hud::*;
pub use motion_path::*;
pub use split_wipe::*;

use crate::KagariApp;
use eframe::egui;

pub fn draw_viewport_overlays(
    ui: &mut egui::Ui,
    app: &mut KagariApp,
    ctx: &egui::Context,
    current_frame: u32,
    viewport_rect: egui::Rect,
    origin_x: f32,
    origin_y: f32,
    draw_w: f32,
    draw_h: f32,
    comp_w: f32,
    comp_h: f32,
    rendered_gpu: bool,
) {
    let transform = crate::ui::viewport_state::ViewportTransform::new(
        egui::Rect::from_min_size(egui::pos2(origin_x, origin_y), egui::vec2(draw_w, draw_h)),
        comp_w,
        comp_h,
    );

    if app.show_grid {
        draw_grid(ui, origin_x, origin_y, draw_w, draw_h);
    }

    draw_motion_paths(ui, app, &transform);

    if app.show_guides {
        draw_safe_guides(ui, origin_x, origin_y, draw_w, draw_h);
    }

    draw_hud_badges(
        ui,
        app,
        ctx,
        origin_x,
        origin_y,
        draw_w,
        draw_h,
        current_frame,
        rendered_gpu,
    );

    draw_transform_gizmos(
        ui,
        app,
        ctx,
        origin_x,
        origin_y,
        draw_w,
        draw_h,
        comp_w,
        comp_h,
        current_frame,
    );

    draw_split_wipe(ui, ctx, origin_x, origin_y, draw_w, draw_h);

    if crate::ui::viewport_minimap::show_navigator(ctx)
        && app.viewport_mode == crate::ViewportMode::Comp2D
    {
        crate::ui::viewport_minimap::draw_minimap(
            app,
            ui,
            current_frame,
            comp_w,
            comp_h,
            viewport_rect,
            egui::Rect::from_min_size(egui::pos2(origin_x, origin_y), egui::vec2(draw_w, draw_h)),
        );
    }
}
