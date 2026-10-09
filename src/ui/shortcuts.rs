//! Centralized Keyboard Shortcut Manager for Kagari VFX.
//! Submodules:
//! - `defs`: Shortcut identifiers, registry definitions, display formatters, reference UI.
//! - `tools_playback`: Tool switching, zoom, and playback controls.
//! - `timeline_layers`: Timeline cursor navigation, markers, layer ordering and properties.
//! - `keyframe_ops`: Keyframe batch nudging, deletion, copy/paste.
//! - `handler`: Main event loop integration and global dialog triggers.

pub mod defs;
pub mod handler;
pub mod keyframe_ops;
pub mod timeline_layers;
pub mod tools_playback;

pub use defs::{
    cmd_name, display_shortcut, draw_shortcuts_reference, format_platform_shortcut_hint,
    format_shortcut, option_name, shortcut_pressed, shortcut_registry, ShortcutDef, ShortcutId,
    ADDITIONAL_SHORTCUTS, SHORTCUT_GESTURES, SHORTCUT_REGISTRY,
};
pub use handler::handle_global_shortcuts;
pub use keyframe_ops::{delete_kf_at, move_kf_in};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::KagariApp;
    use eframe::egui::{self, Key};

    #[test]
    fn delete_mask_vertices_preserves_layers_and_supports_undo_redo() {
        for key in [Key::Delete, Key::Backspace] {
            let mut app = KagariApp::default();
            let mut project = app.history.current().clone();
            let comp = project.active_composition_mut();
            comp.layers.clear();
            let mut layer = crate::core::timeline::Layer::new(
                "layer".into(),
                "Layer".into(),
                crate::core::timeline::LayerType::Solid { color: [1.0; 4] },
                60,
            );
            layer.masks.push(crate::core::mask::Mask::new_rect(
                "mask".into(),
                "Mask".into(),
                0.0,
                0.0,
                100.0,
                100.0,
            ));
            comp.layers.push(layer.clone());
            layer.id = "other".into();
            comp.layers.push(layer);
            app.history = crate::core::history::ProjectHistory::new(project);
            app.active_tool = crate::ui::toolbar::ActiveTool::Selection;
            app.selection.selected_layer_idx = Some(0);
            app.selection.selected_layers.insert(0);
            app.mask_selected_vertices = Some((0, 0, [1].into_iter().collect()));
            let ctx = egui::Context::default();
            let input = egui::RawInput {
                events: vec![egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
                ..Default::default()
            };
            let _ = ctx.run(input, |ctx| {
                handle_global_shortcuts(&mut app, ctx, &mut 0, 60)
            });
            let comp = app.history.current().active_composition();
            assert_eq!(comp.layers.len(), 2);
            assert_eq!(comp.layers[0].masks[0].path.vertices_at_frame(0).len(), 3);
            assert_eq!(comp.layers[1].masks[0].path.vertices_at_frame(0).len(), 4);
            assert!(app.history.undo().is_some());
            assert_eq!(
                app.history.current().active_composition().layers[0].masks[0]
                    .path
                    .vertices_at_frame(0)
                    .len(),
                4
            );
            assert!(app.history.redo().is_some());
            assert_eq!(
                app.history.current().active_composition().layers[0].masks[0]
                    .path
                    .vertices_at_frame(0)
                    .len(),
                3
            );
        }
    }

    #[test]
    fn test_format_shortcut_cross_platform() {
        let sc = format_shortcut("Z", true, true, false);
        assert!(sc.contains("Z"));
        assert!(sc.contains("Shift"));
    }

    #[test]
    fn shortcut_hints_localize_command_and_option_modifiers() {
        assert_eq!(
            defs::format_platform_shortcut_hint_for("Cmd+Alt+Shift+T", "Ctrl", "Alt"),
            "Ctrl+Alt+Shift+T"
        );
        assert_eq!(
            defs::format_platform_shortcut_hint_for("Cmd+Opt+H", "Cmd", "Option"),
            "Cmd+Option+H"
        );
        assert_eq!(
            defs::format_platform_shortcut_hint_for("F9", "Cmd", "Option"),
            "F9"
        );
    }
}
