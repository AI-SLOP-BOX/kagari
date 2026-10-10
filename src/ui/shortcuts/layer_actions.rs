//! Layer manipulation shortcuts (split, pre-comp, reordering, duplicate, delete, nudging).

use crate::KagariApp;
use eframe::egui::{self, Key};
use super::defs::{shortcut_pressed, ShortcutId};

pub fn handle_layer_manipulation(
    app: &mut KagariApp,
    i: &egui::InputState,
    current_frame: &mut u32,
    total_frames: u32,
    allow_single_key: bool,
    cmd: bool,
    shift: bool,
) {
    // ── I / O → Jump to selected layer's in-point / out-point (AE parity) ──
    if allow_single_key && shortcut_pressed(i, ShortcutId::LayerInPoint) {
        if let Some(idx) = app.selection.selected_layer_idx {
            let comp = app.history.current().active_composition();
            if let Some(layer) = comp.layers.get(idx) {
                *current_frame = layer.in_frame;
            }
        }
    }
    if allow_single_key && shortcut_pressed(i, ShortcutId::LayerOutPoint) {
        if let Some(idx) = app.selection.selected_layer_idx {
            let comp = app.history.current().active_composition();
            if let Some(layer) = comp.layers.get(idx) {
                *current_frame = layer.out_frame;
            }
        }
    }

    // Easy Ease (F9 / Shift+F9 / Cmd+F9)
    if allow_single_key && shortcut_pressed(i, ShortcutId::EasyEase) {
        if let Some(idx) = app.selection.selected_layer_idx {
            let ease = if shift {
                // Easy Ease In (Shift+F9)
                crate::core::keyframe::InterpolationType::Bezier {
                    outgoing: crate::core::keyframe::BezierControlPoint {
                        influence: 0.0,
                        speed: 0.0,
                    },
                    incoming: crate::core::keyframe::BezierControlPoint {
                        influence: 0.85,
                        speed: 0.0,
                    },
                    custom_bezier: Some([0.85, 0.0, 1.0, 1.0]),
                }
            } else if cmd {
                // Easy Ease Out (Cmd+Shift+F9 / Cmd+F9)
                crate::core::keyframe::InterpolationType::Bezier {
                    outgoing: crate::core::keyframe::BezierControlPoint {
                        influence: 0.85,
                        speed: 0.0,
                    },
                    incoming: crate::core::keyframe::BezierControlPoint {
                        influence: 0.0,
                        speed: 0.0,
                    },
                    custom_bezier: Some([0.0, 0.0, 0.15, 1.0]),
                }
            } else {
                // Easy Ease (F9)
                crate::core::keyframe::InterpolationType::Bezier {
                    outgoing: crate::core::keyframe::BezierControlPoint {
                        influence: 0.33,
                        speed: 0.0,
                    },
                    incoming: crate::core::keyframe::BezierControlPoint {
                        influence: 0.33,
                        speed: 0.0,
                    },
                    custom_bezier: Some([0.33, 0.0, 0.67, 1.0]),
                }
            };
            app.modify_project(|project| {
                let comp = project.active_composition_mut();
                if let Some(layer) = comp.layers.get_mut(idx) {
                    if let crate::core::property::Animatable::Animated(kfs) =
                        &mut layer.transform.position
                    {
                        for kf in kfs {
                            kf.interpolation = ease;
                        }
                    }
                    if let crate::core::property::Animatable::Animated(kfs) =
                        &mut layer.transform.scale
                    {
                        for kf in kfs {
                            kf.interpolation = ease;
                        }
                    }
                    if let crate::core::property::Animatable::Animated(kfs) =
                        &mut layer.transform.rotation
                    {
                        for kf in kfs {
                            kf.interpolation = ease;
                        }
                    }
                    if let crate::core::property::Animatable::Animated(kfs) =
                        &mut layer.transform.opacity
                    {
                        for kf in kfs {
                            kf.interpolation = ease;
                        }
                    }
                }
            });
        }
    }

    // Arrow keys → nudge selected layer position by 1 px (Shift = 10 px)
    let step = if i.modifiers.shift { 10.0 } else { 1.0 };
    let cur_frame = *current_frame;
    let mut arrow_nudge = |dx: f32, dy: f32| -> bool {
        let Some(idx) = app.selection.selected_layer_idx else {
            return false;
        };
        let mut nudged = false;
        app.modify_project(|project| {
            if let Some(layer) = project.active_composition_mut().layers.get_mut(idx) {
                let cur = layer.transform.position.evaluate(cur_frame);
                layer.transform.position =
                    crate::core::property::Animatable::new_constant([cur[0] + dx, cur[1] + dy]);
                nudged = true;
            }
        });
        nudged
    };
    if allow_single_key && i.key_pressed(Key::ArrowLeft) {
        let moved = arrow_nudge(-step, 0.0);
        if !moved {
            *current_frame = cur_frame.saturating_sub(1);
        }
    }
    if allow_single_key && i.key_pressed(Key::ArrowRight) {
        let moved = arrow_nudge(step, 0.0);
        if !moved {
            *current_frame = (cur_frame + 1).min(total_frames.saturating_sub(1));
        }
    }
    if allow_single_key && i.key_pressed(Key::ArrowUp) {
        arrow_nudge(0.0, -step);
    }
    if allow_single_key && i.key_pressed(Key::ArrowDown) {
        arrow_nudge(0.0, step);
    }

    // Cmd+A: select all layers
    if cmd && !shift && i.key_pressed(Key::A) {
        let count = app.history.current().active_composition().layers.len();
        app.selection.selected_layers.clear();
        app.selection.selected_layer_idx = Some(count.saturating_sub(1));
        for idx in 0..count {
            app.selection.selected_layers.insert(idx);
        }
    }

    // Escape: clear keyframe selection, then layer selection
    if allow_single_key && i.key_pressed(Key::Escape) {
        if !app.selected_keyframes.is_empty() {
            app.selected_keyframes.clear();
        } else {
            app.selection.selected_layers.clear();
            app.selection.selected_layer_idx = None;
        }
    }

    // F2 → Rename selected layer
    if allow_single_key && i.key_pressed(Key::F2) {
        if let Some(idx) = app.selection.selected_layer_idx {
            app.renaming_layer = Some(idx);
        }
    }

    // ── Cmd+Shift+D → Split Selected Layer at Playhead ──
    if cmd && shift && i.key_pressed(Key::D) {
        if let Some(sel_idx) = app.selection.selected_layer_idx {
            let cur = *current_frame;
            let mut split_done = false;
            app.modify_project(|p| {
                let comp = p.active_composition_mut();
                if let Some(layer) = comp.layers.get(sel_idx) {
                    let orig_out = layer.out_frame;
                    if cur > layer.in_frame && cur < orig_out {
                        let split_id = comp.next_layer_id(&format!("{}_split", layer.id));
                        comp.layers[sel_idx].out_frame = cur;
                        let mut tail = comp.layers[sel_idx].clone();
                        tail.id = split_id;
                        tail.name = format!("{} split", tail.name);
                        tail.in_frame = cur;
                        tail.out_frame = orig_out;
                        comp.layers.insert(sel_idx + 1, tail);
                        split_done = true;
                    }
                }
            });
            if split_done {
                app.toasts.info(format!("Split layer at frame {}", cur));
            } else {
                app.toasts
                    .error("Split point must be inside the layer's duration");
            }
        } else {
            app.toasts.info("Select a layer first");
        }
    }

    // ── Cmd+Alt+T → Toggle Time Remapping ──
    if cmd && i.modifiers.alt && !shift && i.key_pressed(Key::T) {
        if let Some(sel_idx) = app.selection.selected_layer_idx {
            let mut enabled = false;
            app.modify_project(|p| {
                let comp = p.active_composition_mut();
                if let Some(l) = comp.layers.get_mut(sel_idx) {
                    if l.time_remap.is_some() {
                        l.clear_time_remap();
                    } else {
                        l.enable_time_remapping();
                        enabled = true;
                    }
                }
            });
            if enabled {
                app.toasts.info("Time Remapping enabled");
            } else {
                app.toasts.info("Time Remapping disabled");
            }
        } else {
            app.toasts.info("Select a layer first");
        }
    }

    // Cmd+Shift+C → Pre-Compose Selected Layers
    if cmd && shift && i.key_pressed(Key::C) {
        let mut temp_project = app.history.current().clone();
        let selected_indices: Vec<usize> = if !app.selection.selected_layers.is_empty() {
            let mut s: Vec<usize> = app.selection.selected_layers.iter().copied().collect();
            s.sort();
            s
        } else if let Some(idx) = app.selection.selected_layer_idx {
            vec![idx]
        } else {
            vec![]
        };

        if !selected_indices.is_empty() {
            let precomp_id = temp_project.next_composition_id("precomp");
            let (width, height, fps, duration_frames) = {
                let comp = temp_project.active_composition();
                (comp.width, comp.height, comp.fps, comp.duration_frames)
            };

            let precomp_name = format!("Pre-comp {}", temp_project.compositions.len() + 1);
            let mut new_comp = crate::core::timeline::Composition::new(
                precomp_id.clone(),
                precomp_name.clone(),
                width,
                height,
                fps,
                duration_frames,
            );

            let comp_mut = temp_project.active_composition_mut();
            let mut extracted_layers = Vec::new();
            for &idx in selected_indices.iter().rev() {
                if idx < comp_mut.layers.len() {
                    extracted_layers.push(comp_mut.layers.remove(idx));
                }
            }
            extracted_layers.reverse();
            new_comp.layers = extracted_layers;

            let moved_layers = new_comp.layers.clone();
            comp_mut.move_scene_objects_to(&mut new_comp, &moved_layers);

            let precomp_layer = crate::core::timeline::Layer::new(
                comp_mut.next_layer_id("precomp"),
                precomp_name,
                crate::core::timeline::LayerType::PreComp {
                    comp_id: precomp_id,
                },
                duration_frames,
            );
            let insert_pos = selected_indices
                .first()
                .copied()
                .unwrap_or(0)
                .min(comp_mut.layers.len());
            comp_mut.layers.insert(insert_pos, precomp_layer);
            temp_project.compositions.push(new_comp);

            app.selection.selected_layers.clear();
            app.selection.selected_layers.insert(insert_pos);
            app.selection.selected_layer_idx = Some(insert_pos);
            app.commit_project(temp_project);
        }
    }

    // Cmd+D → Duplicate selected layer
    if shortcut_pressed(i, ShortcutId::DuplicateLayer) {
        if let Some(idx) = app.selection.selected_layer_idx {
            let mut proj = app.history.current().clone();
            let comp = proj.active_composition_mut();
            if idx < comp.layers.len() {
                let mut dup = comp.layers[idx].clone();
                dup.id = comp.next_layer_id(&format!("{}_dup", dup.id));
                dup.name = format!("{} Copy", dup.name);
                comp.layers.insert(idx + 1, dup);
                app.selection.selected_layer_idx = Some(idx + 1);
                app.commit_project(proj);
            }
        }
    }

    // Cmd+[ → Send Backward, Cmd+] → Bring Forward (layer stacking order)
    if cmd && (i.key_pressed(Key::OpenBracket) || i.key_pressed(Key::CloseBracket)) {
        if let Some(idx) = app.selection.selected_layer_idx {
            let forward = i.key_pressed(Key::CloseBracket);
            let len = app.history.current().active_composition().layers.len();
            let target = if forward {
                (idx + 1 < len).then_some(idx + 1)
            } else {
                (idx > 0).then_some(idx - 1)
            };
            if let Some(new_idx) = target {
                let mut proj = app.history.current().clone();
                proj.active_composition_mut().layers.swap(idx, new_idx);
                app.selection.selected_layer_idx = Some(new_idx);
                app.commit_project(proj);
            }
        }
    }

    // Delete / Backspace → Delete all selected layers
    if allow_single_key
        && (i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace))
        && !shift
    {
        let mut indices: Vec<usize> = app.selection.selected_layers.iter().copied().collect();
        if indices.is_empty() {
            if let Some(s) = app.selection.selected_layer_idx {
                indices.push(s);
            }
        }
        if !indices.is_empty() {
            indices.sort_unstable_by(|a, b| b.cmp(a));
            app.modify_project(|project| {
                let comp = project.active_composition_mut();
                for idx in indices {
                    if idx < comp.layers.len() {
                        let _ = comp.remove_layer_at(idx);
                    }
                }
            });
            app.selection.selected_layers.clear();
            app.selection.selected_layer_idx = None;
        }
    }

    // Property Selection Shortcuts (P, S, T, R, A)
    if allow_single_key && shortcut_pressed(i, ShortcutId::RevealPosition) {
        if let Some(idx) = app.selection.selected_layer_idx {
            app.selection.expanded_layers.insert(idx);
        }
        app.selection.selected_property = Some("Position X".to_string());
    }
    if allow_single_key && shortcut_pressed(i, ShortcutId::RevealScale) {
        if let Some(idx) = app.selection.selected_layer_idx {
            app.selection.expanded_layers.insert(idx);
        }
        app.selection.selected_property = Some("Scale X".to_string());
    }
    if allow_single_key && shortcut_pressed(i, ShortcutId::RevealOpacity) {
        if let Some(idx) = app.selection.selected_layer_idx {
            app.selection.expanded_layers.insert(idx);
        }
        app.selection.selected_property = Some("Opacity".to_string());
    }
    if allow_single_key && shortcut_pressed(i, ShortcutId::RevealRotation) {
        if let Some(idx) = app.selection.selected_layer_idx {
            app.selection.expanded_layers.insert(idx);
        }
        app.selection.selected_property = Some("Rotation".to_string());
    }
    if allow_single_key && shortcut_pressed(i, ShortcutId::RevealAnchorPoint) {
        if let Some(idx) = app.selection.selected_layer_idx {
            app.selection.expanded_layers.insert(idx);
        }
        app.selection.selected_property = Some("Anchor Point".to_string());
        app.toasts.info("A: Reveal Anchor Point");
    }

    // ── I / O: Jump to Layer In / Out Point ──
    if allow_single_key && !cmd && i.key_pressed(Key::I) {
        if let Some(idx) = app.selection.selected_layer_idx {
            let comp = app.history.current().active_composition();
            if idx < comp.layers.len() {
                *current_frame = comp.layers[idx].in_frame;
                app.toasts
                    .info(format!("Jumped to Layer In Point: frame {}", current_frame));
            }
        }
    }
    if allow_single_key && !cmd && i.key_pressed(Key::O) {
        if let Some(idx) = app.selection.selected_layer_idx {
            let comp = app.history.current().active_composition();
            if idx < comp.layers.len() {
                *current_frame = comp.layers[idx].out_frame.saturating_sub(1);
                app.toasts.info(format!(
                    "Jumped to Layer Out Point: frame {}",
                    current_frame
                ));
            }
        }
    }

    // ── U / UU: Reveal Keyframed / All Modified Properties ──
    if allow_single_key && !cmd && i.key_pressed(Key::U) {
        let now = i.time;
        let is_double = app
            .u_key_last_press
            .map(|last| (now - last) < 0.4)
            .unwrap_or(false);
        if is_double {
            if let Some(idx) = app.selection.selected_layer_idx {
                app.selection.expanded_layers.insert(idx);
            }
            app.selection.selected_property = Some("All Modified".to_string());
            app.toasts.info("UU: Reveal All Modified Properties");
            app.u_key_last_press = None;
        } else {
            if let Some(idx) = app.selection.selected_layer_idx {
                app.selection.expanded_layers.insert(idx);
            }
            app.selection.selected_property = Some("Keyframed".to_string());
            app.toasts.info("U: Reveal Keyframed Properties");
            app.u_key_last_press = Some(now);
        }
    }

    // Tab / Shift+Tab → Cycle selected layer down / up
    if allow_single_key && i.key_pressed(Key::Tab) {
        let count = app.history.current().active_composition().layers.len();
        if count > 0 {
            let next = if shift {
                app.selection
                    .selected_layer_idx
                    .map_or(count - 1, |idx| idx.saturating_sub(1))
            } else {
                app.selection
                    .selected_layer_idx
                    .map_or(0, |idx| (idx + 1).min(count - 1))
            };
            app.selection.selected_layer_idx = Some(next);
            app.selection.selected_layers.clear();
            app.selection.selected_layers.insert(next);
        }
    }

    // Cmd+Alt+F → Fit to Comp
    if cmd && i.modifiers.alt && i.key_pressed(Key::F) {
        if let Some(idx) = app.selection.selected_layer_idx {
            let dims = {
                let cc = app.history.current().active_composition();
                (cc.width as f32, cc.height as f32)
            };
            app.modify_project(move |p| {
                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                    let bs = l.bounding_size();
                    if bs[0] > 1.0 && bs[1] > 1.0 {
                        let s = ((dims.0 / bs[0]).max(dims.1 / bs[1])) * 100.0;
                        l.transform.scale =
                            crate::core::property::Animatable::new_constant([s, s]);
                        l.transform.position =
                            crate::core::property::Animatable::new_constant([
                                dims.0 / 2.0,
                                dims.1 / 2.0,
                            ]);
                    }
                }
            });
            app.toasts.info("Fit to Comp");
        } else {
            app.toasts.info("Select a layer first");
        }
    }
}
