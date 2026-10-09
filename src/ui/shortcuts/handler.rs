use crate::KagariApp;
use eframe::egui::{self, Key};
use super::defs::{shortcut_pressed, ShortcutId};
use super::keyframe_ops::handle_keyframe_editing;
use super::timeline_layers::handle_timeline_and_layers;
use super::tools_playback::handle_tools_and_playback;

/// Centralized Keyboard Shortcut Manager for Kagari VFX.
/// Encapsulates global keybindings: Spacebar playback, frame stepping, keyframe navigation (J/K),
/// Easy Ease (F9), Undo/Redo, Pre-compose (Cmd+Shift+C), Duplicate/Split (Cmd+D / Cmd+Shift+D),
/// Layer Deletion, property selection (P, S, T, R), and dialog triggers.
pub fn handle_global_shortcuts(
    app: &mut KagariApp,
    ctx: &egui::Context,
    current_frame: &mut u32,
    total_frames: u32,
) {
    if app.has_blocking_dialog() {
        return;
    }

    // ── Two-Tier Focus Guard ────────────────────────────────────────────────
    // Tier 1: is a text-edit widget actively focused?
    let text_focused = crate::ui::focus::is_text_input_focused(ctx);
    // Tier 2: does egui report that it wants all keyboard input right now?
    let wants_all_keys = ctx.wants_keyboard_input();

    // When `wants_all_keys` is true, block ALL shortcuts unconditionally.
    if wants_all_keys {
        return;
    }

    // When a text field is focused, only allow modifier-based shortcuts
    // (Cmd+Z, Cmd+Shift+Z, Cmd+M, etc.) to pass through.
    let allow_single_key = !text_focused;

    if allow_single_key
        && app.active_tool == crate::ui::toolbar::ActiveTool::Selection
        && ctx.input(|i| i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace))
        && app
            .mask_selected_vertices
            .as_ref()
            .is_some_and(|(_, _, vertices)| !vertices.is_empty())
    {
        if let Some((layer_idx, mask_idx, vertices)) = app.mask_selected_vertices.take() {
            if app.selection.selected_layer_idx == Some(layer_idx) {
                app.commit_drag();
                app.modify_project(|project| {
                    if let Some(mask) = project
                        .active_composition_mut()
                        .layers
                        .get_mut(layer_idx)
                        .filter(|layer| !layer.locked)
                        .and_then(|layer| layer.masks.get_mut(mask_idx))
                    {
                        let mut vertices: Vec<_> = vertices.into_iter().collect();
                        vertices.sort_unstable_by(|a, b| b.cmp(a));
                        for vertex in vertices {
                            mask.path.remove_vertex_at_frame(vertex);
                        }
                    }
                });
            }
        }
        app.viewport_mask_drag_state = None;
        app.viewport_mask_drag_vertices.clear();
        return;
    }

    ctx.input(|i| {
        let cmd = i.modifiers.command;
        let shift = i.modifiers.shift;

        // 1. Tools, Playback, and Layer Creation
        handle_tools_and_playback(app, i, current_frame, allow_single_key, cmd, shift);

        // 2. Timeline Navigation, Markers, Layer Ordering, and Property Selectors
        handle_timeline_and_layers(app, i, current_frame, total_frames, allow_single_key, cmd, shift);

        // 3. Keyframe Editing (Batch nudging, Delete, Copy, Paste)
        handle_keyframe_editing(app, i, current_frame, allow_single_key, cmd, shift);

        // 4. Global Undo / Redo
        if shortcut_pressed(i, ShortcutId::Undo)
            && app.history.can_undo()
            && app.history.undo().is_some()
        {
            crate::core::frame_cache::bump_version();
            app.autosave.mark_dirty();
            app.autosave_history_generation = app.history.generation();
            app.toasts.info("Undo");
        }
        if shortcut_pressed(i, ShortcutId::Redo)
            && app.history.can_redo()
            && app.history.redo().is_some()
        {
            crate::core::frame_cache::bump_version();
            app.autosave.mark_dirty();
            app.autosave_history_generation = app.history.generation();
            app.toasts.info("Redo");
        }

        // 5. Global Dialog Triggers
        // Cmd+N → New Composition
        if shortcut_pressed(i, ShortcutId::NewComposition) {
            app.show_new_comp_dialog = true;
        }

        // Cmd+S → Save Project (overwrite)
        if shortcut_pressed(i, ShortcutId::Save) {
            let path = app.project_path.clone();
            if let Err(e) =
                crate::ui::project_io::save_project_to_path(app, std::path::Path::new(&path))
            {
                app.toasts.error(format!("Save failed: {}", e));
            }
        }

        // Cmd+Shift+E → Composition Settings Dialog
        if cmd && shift && i.key_pressed(Key::E) {
            app.comp_settings_draft = Some(app.history.current().clone());
            app.show_comp_settings = true;
        }

        // Cmd+M → Render Queue / Export Dialog
        if shortcut_pressed(i, ShortcutId::OpenRenderQueue) {
            app.export.show_export_dialog = true;
        }

        // Cmd+1~4: Composition Tab Switcher
        if cmd {
            for (key, idx) in [
                (Key::Num1, 0usize),
                (Key::Num2, 1),
                (Key::Num3, 2),
                (Key::Num4, 3),
            ] {
                if i.key_pressed(key) {
                    let comp_count = app.history.current().compositions.len();
                    if idx < comp_count {
                        let mut project = app.history.current().clone();
                        project.active_composition_idx = idx;
                        app.commit_project(project);
                        app.selection.selected_layer_idx = None;
                        app.selection.selected_layers.clear();
                        let name = app.history.current().compositions[idx].name.clone();
                        app.toasts
                            .info(format!("Switched to Composition: {}", name));
                    }
                }
            }
        }

        // Cmd+, → Preferences
        if cmd && i.key_pressed(egui::Key::Comma) {
            app.show_preferences = true;
        }

        // Cmd+Alt+← → Breadcrumb Back (previous composition)
        if cmd && i.modifiers.alt && i.key_pressed(Key::ArrowLeft) && !app.comp_nav_stack.is_empty()
        {
            if let Some(prev) = app.comp_nav_stack.pop() {
                let mut project = app.history.current().clone();
                project.active_composition_idx = prev;
                app.commit_project(project);
                app.selection.selected_layer_idx = None;
                app.selection.selected_layers.clear();
            }
        }

        // Ctrl+Shift+K → Toggle Motion Sketch
        if cmd && shift && i.key_pressed(Key::K) {
            app.motion_sketch_active = !app.motion_sketch_active;
            if app.motion_sketch_active {
                app.playback.start_playing();
                app.toasts
                    .info("Motion Sketch ON — drag layer to record position");
            } else {
                app.toasts.info("Motion Sketch OFF");
            }
        }

        // Shift+F3 → Toggle Graph Editor / Tracks Mode
        if i.key_pressed(Key::F3) && shift {
            app.show_graph_editor = !app.show_graph_editor;
        }

        // Shift+F4 → Toggle Vectorscope / RGB Parade window
        if i.key_pressed(Key::F4) && shift {
            app.show_vectorscope = !app.show_vectorscope;
            app.toasts.info(if app.show_vectorscope {
                "Vectorscope ON (Shift+F4)"
            } else {
                "Vectorscope OFF"
            });
        }
    });
}
