//! Layer styles configuration UI (Drop Shadow, Glows, Stroke, Overlays, Bevel).
//!
//! Submodules:
//! - [`common`]: Shared UI helper widgets such as RGBA color pickers.
//! - [`shadow_glow`]: Drop shadow, inner shadow, and glow controls.
//! - [`overlays`]: Satin, bevel/emboss, stroke, gradient overlay, and color overlay.

pub mod common;
pub mod overlays;
pub mod shadow_glow;

pub use common::*;
pub use overlays::*;
pub use shadow_glow::*;

use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub fn draw_layer_styles(app: &mut KagariApp, ui: &mut egui::Ui) {
    app.ensure_panel_edit_snapshot("layer_styles", ui);
    let comp = app.history.current_mut().active_composition_mut();

    let layer_idx = match app.selection.selected_layer_idx {
        Some(idx) if idx < comp.layers.len() => idx,
        _ => {
            ui.label(
                egui::RichText::new("No layer selected.")
                    .small()
                    .color(colors::TEXT_MUTED),
            );
            return;
        }
    };

    let layer = &mut comp.layers[layer_idx];
    let style = &mut layer.style;
    let mut changed = false;

    egui::ScrollArea::vertical().show(ui, |ui| {
        draw_shadow_and_glow(ui, style, &mut changed);
        draw_overlays_and_bevel(ui, style, &mut changed);
    });

    if changed {
        let pointer_down = ui.input(|input| input.pointer.any_down());
        if pointer_down {
            if !app.drag_active() {
                if let Some(snapshot) = app.take_panel_edit_snapshot("layer_styles") {
                    app.begin_drag_with_snapshot(snapshot, "Layer Style Edit");
                }
            }
        } else if app.drag_active() {
            app.commit_drag();
        } else if let Some(snapshot) = app.take_panel_edit_snapshot("layer_styles") {
            app.begin_drag_with_snapshot(snapshot, "Layer Style Edit");
            app.commit_drag();
        }
    }
}
