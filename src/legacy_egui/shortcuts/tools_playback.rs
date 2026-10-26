use super::defs::{shortcut_pressed, ShortcutId};
use crate::KagariApp;
use eframe::egui::{self, Key};

/// Handles tools switching, playback controls, viewport zooming, and layer creation shortcuts.
pub fn handle_tools_and_playback(
    app: &mut KagariApp,
    i: &egui::InputState,
    current_frame: &mut u32,
    allow_single_key: bool,
    cmd: bool,
    shift: bool,
) {
    // Space → Play / Pause RAM Preview (single-key: suppressed while typing)
    if allow_single_key && shortcut_pressed(i, ShortcutId::TogglePlayback) {
        if app.playback.is_playing {
            app.playback.stop_playing();
            if let Some(playback) = &mut app.audio_playback {
                playback.pause();
            }
        } else {
            app.playback.start_playing();
            if let Some(playback) = &mut app.audio_playback {
                playback.resume();
            }
        }
        if !app.playback.is_playing && app.motion_sketch_active {
            app.motion_sketch_active = false;
            app.toasts.info("Motion Sketch OFF");
        }
    }

    // ── Tool Switching Shortcuts (AE standard) ──
    // Only when no modifier keys are pressed and text is not focused
    if allow_single_key && !cmd && !shift && !i.modifiers.alt {
        if shortcut_pressed(i, ShortcutId::SelectionTool) {
            app.active_tool = crate::ui::toolbar::ActiveTool::Selection;
        }
        if shortcut_pressed(i, ShortcutId::HandTool) {
            app.active_tool = crate::ui::toolbar::ActiveTool::Hand;
        }
        if shortcut_pressed(i, ShortcutId::ZoomTool) {
            app.active_tool = crate::ui::toolbar::ActiveTool::Zoom;
        }
        if shortcut_pressed(i, ShortcutId::RotationTool) {
            app.active_tool = crate::ui::toolbar::ActiveTool::Rotation;
        }
        if shortcut_pressed(i, ShortcutId::AnchorTool) {
            app.active_tool = crate::ui::toolbar::ActiveTool::AnchorPoint;
        }
        if shortcut_pressed(i, ShortcutId::RectangleTool) {
            app.active_tool = crate::ui::toolbar::ActiveTool::Rectangle;
        }
        if shortcut_pressed(i, ShortcutId::PenTool) {
            app.active_tool = crate::ui::toolbar::ActiveTool::Pen;
        }
        if shortcut_pressed(i, ShortcutId::CameraTool) {
            app.active_tool = crate::ui::toolbar::ActiveTool::Camera3D;
        }
    }
    if allow_single_key && shortcut_pressed(i, ShortcutId::PuppetPinTool) {
        app.active_tool = crate::ui::toolbar::ActiveTool::PuppetPin;
    }

    // Layer creation shortcuts advertised in the Layer > New menu.
    if cmd && !shift && !i.modifiers.alt && i.key_pressed(Key::Y) {
        crate::ui::commands::layer::create_new_layer(
            app,
            crate::ui::commands::layer::NewLayerKind::Solid,
        );
    }
    if cmd && shift && i.modifiers.alt && i.key_pressed(Key::T) {
        crate::ui::commands::layer::create_new_layer(
            app,
            crate::ui::commands::layer::NewLayerKind::Text,
        );
    }
    if cmd && shift && i.modifiers.alt && i.key_pressed(Key::Y) {
        crate::ui::commands::layer::create_new_layer(
            app,
            crate::ui::commands::layer::NewLayerKind::Null,
        );
    }
    if cmd && !shift && i.modifiers.alt && i.key_pressed(Key::Y) {
        crate::ui::commands::layer::create_new_layer(
            app,
            crate::ui::commands::layer::NewLayerKind::Adjustment,
        );
    }
    // Cmd+T → Text tool (bare T reveals Opacity, AE parity)
    if shortcut_pressed(i, ShortcutId::TextTool) {
        app.active_tool = crate::ui::toolbar::ActiveTool::Text;
    }

    // Shift+Z → Viewport zoom to fit / to selected layers' bbox (AE parity)
    if shift && !cmd && i.key_pressed(Key::Z) {
        let mut bbox_request: Option<([f32; 2], [f32; 2])> = None;
        if !app.selection.selected_layers.is_empty() || app.selection.selected_layer_idx.is_some() {
            let comp = app.history.current().active_composition();
            let cf = *current_frame;
            let idxs: Vec<usize> = if app.selection.selected_layers.is_empty() {
                app.selection.selected_layer_idx.into_iter().collect()
            } else {
                let mut v: Vec<usize> = app.selection.selected_layers.iter().copied().collect();
                v.sort();
                v
            };
            let mut min = [f32::MAX; 2];
            let mut max = [f32::MIN; 2];
            for &li in &idxs {
                let Some(l) = comp.layers.get(li) else {
                    continue;
                };
                let (pos, _, _, _) = comp.resolve_world_transform(l, cf);
                let bs = l.bounding_size();
                let sc = l.transform.scale.evaluate(cf);
                let hx = bs[0] * sc[0].abs() * 0.005;
                let hy = bs[1] * sc[1].abs() * 0.005;
                min[0] = min[0].min(pos[0] - hx);
                min[1] = min[1].min(pos[1] - hy);
                max[0] = max[0].max(pos[0] + hx);
                max[1] = max[1].max(pos[1] + hy);
            }
            if max[0] > min[0] && max[1] > min[1] {
                bbox_request = Some((min, max));
            }
        }
        match bbox_request {
            Some(b) => app.viewport_focus_bbox = Some(b),
            None => {
                app.ui_tabs.viewport_mag_ratio = 0.0;
                app.playback.viewport_pan = egui::Vec2::ZERO;
            }
        }
    }
    // Cmd+0 → Viewport zoom to fit (AE standard)
    if cmd && i.key_pressed(Key::Num0) {
        app.ui_tabs.viewport_mag_ratio = 0.0;
        app.playback.viewport_pan = egui::Vec2::ZERO;
    }

    // ── L: Shuttle Forward (press again to increase speed up to 3x) ──
    // J/K are reserved for keyframe navigation (AE standard).
    if allow_single_key && shortcut_pressed(i, ShortcutId::PlayForward) {
        if app.playback.is_playing && app.playback_speed > 0 {
            // Already playing forward: increase speed (max 3x)
            app.playback_speed = (app.playback_speed + 1).min(3);
        } else {
            app.playback.start_playing();
            app.playback_speed = 1;
        }
        app.toasts
            .info(format!("▶ Forward {}x", app.playback_speed));
    }

    // Numpad 0 → RAM Preview (force work-area pre-render + play)
    if allow_single_key && i.key_pressed(Key::Num0) && !app.playback.is_playing {
        app.playback.start_playing();
    }
}
