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
    let (layer_idx, mut style) = {
        let comp = app.history.current().active_composition();
        match app.selection.selected_layer_idx {
            Some(idx) if idx < comp.layers.len() => (idx, comp.layers[idx].style.clone()),
            _ => {
                ui.label(
                    egui::RichText::new("No layer selected.")
                        .small()
                        .color(colors::TEXT_MUTED),
                );
                return;
            }
        }
    };

    let mut changed = false;

    egui::ScrollArea::vertical().show(ui, |ui| {
        draw_shadow_and_glow(ui, &mut style, &mut changed);
        draw_overlays_and_bevel(ui, &mut style, &mut changed);
    });

    if changed {
        app.dispatch(crate::app_state::AppCommand::SetLayerStyle {
            layer_idx,
            style: Box::new(style),
        });
    }
}
