use crate::KagariApp;
use eframe::egui::{self, Key};

/// Moves a keyframe within an animatable track (used by batch keyframe nudging).
pub fn move_kf_in<T: Clone + crate::core::property::Interpolate>(
    anim: &mut crate::core::property::Animatable<T>,
    old_frame: u32,
    new_frame: u32,
) {
    if let Some(kfs) = anim.keyframes_mut() {
        if let Some(kf) = kfs.iter_mut().find(|k| k.frame == old_frame) {
            kf.frame = new_frame;
            kfs.sort_by_key(|k| k.frame);
        }
    }
}

/// Deletes the keyframe at `frame` from a track (keeps at least nothing —
/// emptying a track turns it back into a constant, which Animatable handles).
pub fn delete_kf_at<T: Clone>(anim: &mut crate::core::property::Animatable<T>, frame: u32) {
    if let Some(kfs) = anim.keyframes_mut() {
        kfs.retain(|k| k.frame != frame);
    }
}

/// Handles batch nudging, deletion, and copy/paste of keyframes.
pub fn handle_keyframe_editing(
    app: &mut KagariApp,
    i: &egui::InputState,
    current_frame: &mut u32,
    allow_single_key: bool,
    cmd: bool,
    shift: bool,
) {
    // ── Batch-move selected keyframes with , / . (comma/period) ──
    // Comma shifts all selected keyframes 1 frame left, period right (Shift = 10).
    if allow_single_key && (i.key_pressed(Key::Comma) || i.key_pressed(Key::Period)) {
        let delta: i32 = if i.key_pressed(Key::Comma) { -1 } else { 1 };
        let delta = if i.modifiers.shift { delta * 10 } else { delta };
        if !app.selected_keyframes.is_empty() {
            let selected_keyframes = app.selected_keyframes.clone();
            use std::collections::HashMap;
            let mut by_layer: HashMap<usize, Vec<(String, u32)>> = HashMap::new();
            for (li, pk, f) in &selected_keyframes {
                by_layer.entry(*li).or_default().push((pk.clone(), *f));
            }
            app.modify_project(|project| {
                for (li, kfs) in by_layer {
                    let Some(comp) = project.active_composition_mut().layers.get_mut(li) else {
                        continue;
                    };
                    let t = &mut comp.transform;
                    for (pk, old_f) in kfs {
                        let new_f = ((old_f as i32) + delta).max(0) as u32;
                        match pk.as_str() {
                            "position" => move_kf_in(&mut t.position, old_f, new_f),
                            "scale" => move_kf_in(&mut t.scale, old_f, new_f),
                            "rotation" => move_kf_in(&mut t.rotation, old_f, new_f),
                            "opacity" => move_kf_in(&mut t.opacity, old_f, new_f),
                            _ => {}
                        }
                    }
                }
            });
            // Remap selection to the moved frames
            app.selected_keyframes = app
                .selected_keyframes
                .iter()
                .map(|(li, pk, f)| (*li, pk.clone(), ((*f as i32) + delta).max(0) as u32))
                .collect();
        }
    }

    // ── Delete / Backspace removes all selected keyframes ──
    if allow_single_key
        && (i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace))
        && !app.selected_keyframes.is_empty()
    {
        use std::collections::HashMap;
        let mut by_layer: HashMap<usize, Vec<(String, u32)>> = HashMap::new();
        let selected_keyframes = app.selected_keyframes.clone();
        for (li, pk, f) in &selected_keyframes {
            by_layer.entry(*li).or_default().push((pk.clone(), *f));
        }
        app.modify_project(|project| {
            for (li, kfs) in by_layer {
                let Some(comp) = project.active_composition_mut().layers.get_mut(li) else {
                    continue;
                };
                let t = &mut comp.transform;
                for (pk, frame) in kfs {
                    match pk.as_str() {
                        "position" => delete_kf_at(&mut t.position, frame),
                        "scale" => delete_kf_at(&mut t.scale, frame),
                        "rotation" => delete_kf_at(&mut t.rotation, frame),
                        "opacity" => delete_kf_at(&mut t.opacity, frame),
                        _ => {}
                    }
                }
            }
        });
        app.selected_keyframes.clear();
    }

    // ── Cmd+C: copy selected keyframes ──
    if cmd && !shift && i.key_pressed(Key::C) && !app.selected_keyframes.is_empty() {
        let anchor = *current_frame;
        let mut clip: Vec<(String, i32, serde_json::Value)> = Vec::new();
        let mut selection: Vec<(usize, String, u32)> =
            app.selected_keyframes.iter().cloned().collect();
        selection.sort_by_key(|(_, _, f)| *f);

        let project = app.history.current();
        for (li, pk, frame) in selection {
            let Some(comp) = project.active_composition().layers.get(li) else {
                continue;
            };
            let t = &comp.transform;
            let kf_json = match pk.as_str() {
                "position" => t
                    .position
                    .keyframes()
                    .and_then(|k| k.iter().find(|k| k.frame == frame))
                    .and_then(|k| serde_json::to_value(k).ok()),
                "scale" => t
                    .scale
                    .keyframes()
                    .and_then(|k| k.iter().find(|k| k.frame == frame))
                    .and_then(|k| serde_json::to_value(k).ok()),
                "rotation" => t
                    .rotation
                    .keyframes()
                    .and_then(|k| k.iter().find(|k| k.frame == frame))
                    .and_then(|k| serde_json::to_value(k).ok()),
                "opacity" => t
                    .opacity
                    .keyframes()
                    .and_then(|k| k.iter().find(|k| k.frame == frame))
                    .and_then(|k| serde_json::to_value(k).ok()),
                _ if pk.starts_with("pin_") => {
                    let pin_id = pk.strip_prefix("pin_").unwrap_or("");
                    if let Some(pin) = comp.puppet_pins.iter().find(|p| p.id == pin_id) {
                        if let Some(kfs) = pin.position.keyframes() {
                            kfs.iter()
                                .find(|k| k.frame == frame)
                                .and_then(|k| serde_json::to_value(k).ok())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                _ if crate::ui::graph_editor::is_effect_property(&pk) => {
                    if let Some((effect_id, param_label, _)) =
                        crate::ui::graph_editor::parse_effect_property(&pk)
                    {
                        if let Some(effect) = comp.effects.iter().find(|e| e.id == effect_id) {
                            use crate::core::effect_params::ParamRefRef;
                            let mut found_json = None;
                            for (label, param) in effect.effect_type.animatable_params_ref() {
                                if label == param_label {
                                    match param {
                                        ParamRefRef::Scalar(anim) => {
                                            if let Some(kfs) = anim.keyframes() {
                                                found_json = kfs
                                                    .iter()
                                                    .find(|k| k.frame == frame)
                                                    .and_then(|k| serde_json::to_value(k).ok());
                                            }
                                        }
                                        ParamRefRef::Vec2(anim) => {
                                            if let Some(kfs) = anim.keyframes() {
                                                found_json = kfs
                                                    .iter()
                                                    .find(|k| k.frame == frame)
                                                    .and_then(|k| serde_json::to_value(k).ok());
                                            }
                                        }
                                        ParamRefRef::Vec3(anim) => {
                                            if let Some(kfs) = anim.keyframes() {
                                                found_json = kfs
                                                    .iter()
                                                    .find(|k| k.frame == frame)
                                                    .and_then(|k| serde_json::to_value(k).ok());
                                            }
                                        }
                                        ParamRefRef::Vec4Color(anim) => {
                                            if let Some(kfs) = anim.keyframes() {
                                                found_json = kfs
                                                    .iter()
                                                    .find(|k| k.frame == frame)
                                                    .and_then(|k| serde_json::to_value(k).ok());
                                            }
                                        }
                                    }
                                    break;
                                }
                            }
                            found_json
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                _ => None,
            };
            if let Some(v) = kf_json {
                let offset = frame as i32 - anchor as i32;
                clip.push((pk.clone(), offset, v));
            }
        }
        if !clip.is_empty() {
            app.kf_clipboard = clip;
            app.kf_clipboard_anchor = anchor;
        }
    }

    // ── Cmd+V: paste keyframes ──
    if cmd && !shift && i.key_pressed(Key::V) && !app.kf_clipboard.is_empty() {
        let paste_origin = *current_frame;
        let target_layer_idx = app.selection.selected_layer_idx.unwrap_or(0);
        let clipboard = app.kf_clipboard.clone();
        let clipboard_anchor = app.kf_clipboard_anchor;

        app.modify_project(|project| {
            let Some(layer) = project
                .active_composition_mut()
                .layers
                .get_mut(target_layer_idx)
            else {
                return;
            };
            let t = &mut layer.transform;

            macro_rules! paste_into {
                ($anim:expr, $ty:ty, $value_json:expr) => {{
                    if let Ok(mut kf) =
                        serde_json::from_value::<crate::core::keyframe::Keyframe<$ty>>(
                            $value_json.clone(),
                        )
                    {
                        kf.frame = ((kf.frame as i64 + paste_origin as i64
                            - clipboard_anchor as i64)
                            .max(0)) as u32;
                        $anim.add_keyframe(kf);
                    }
                }};
            }

            for (pk, _offset, value_json) in &clipboard {
                match pk.as_str() {
                    "position" => paste_into!(t.position, [f32; 2], value_json),
                    "scale" => paste_into!(t.scale, [f32; 2], value_json),
                    "rotation" => paste_into!(t.rotation, f32, value_json),
                    "opacity" => paste_into!(t.opacity, f32, value_json),
                    _ if pk.starts_with("pin_") => {
                        let pid = pk.strip_prefix("pin_").unwrap_or("");
                        if let Some(pin) = layer.puppet_pins.iter_mut().find(|p| p.id == pid) {
                            paste_into!(pin.position, [f32; 2], value_json);
                        }
                    }
                    _ if crate::ui::graph_editor::is_effect_property(pk) => {
                        if let Some((effect_id, param_label, _)) =
                            crate::ui::graph_editor::parse_effect_property(pk)
                        {
                            if let Some(effect) =
                                layer.effects.iter_mut().find(|e| e.id == effect_id)
                            {
                                use crate::core::effect_params::ParamRef;
                                for (label, param) in effect.effect_type.animatable_params() {
                                    if label == param_label {
                                        match param {
                                            ParamRef::Scalar(anim) => {
                                                paste_into!(anim, f32, value_json);
                                            }
                                            ParamRef::Vec2(anim) => {
                                                paste_into!(anim, [f32; 2], value_json);
                                            }
                                            ParamRef::Vec3(anim) => {
                                                paste_into!(anim, [f32; 3], value_json);
                                            }
                                            ParamRef::Vec4Color(anim) => {
                                                paste_into!(anim, [f32; 4], value_json);
                                            }
                                        }
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        });
    }
}
