use super::commands::move_kf;
use crate::core::timeline::Layer;

/// Resolve every keyframe frame of a property (for label dbl-click select-all).
pub fn prop_all_frames(layer: &Layer, pk: &str) -> Vec<u32> {
    let mut out = Vec::new();
    let mut push_anim_f32 = |a: &crate::core::property::Animatable<f32>| {
        if let Some(kfs) = a.keyframes() {
            for k in kfs {
                out.push(k.frame);
            }
        }
    };
    match pk {
        "position" | "scale" => {
            let a = if pk == "position" {
                &layer.transform.position
            } else {
                &layer.transform.scale
            };
            if let Some(kfs) = a.keyframes() {
                for k in kfs {
                    out.push(k.frame);
                }
            }
        }
        "rotation" => push_anim_f32(&layer.transform.rotation),
        "opacity" => push_anim_f32(&layer.transform.opacity),
        "repeater_copies" => {
            if let Some(anim) = layer
                .shape_repeater
                .as_ref()
                .and_then(|repeater| repeater.copies_animation.as_ref())
            {
                push_anim_f32(anim);
            }
        }
        "repeater_rotation" => {
            if let Some(anim) = layer
                .shape_repeater
                .as_ref()
                .and_then(|repeater| repeater.rotation_offset_animation.as_ref())
            {
                push_anim_f32(anim);
            }
        }
        "repeater_position" | "repeater_scale" | "repeater_opacity" => {
            let keyframes = layer.shape_repeater.as_ref().and_then(|repeater| match pk {
                "repeater_position" => repeater
                    .position_offset_animation
                    .as_ref()
                    .and_then(|a| a.keyframes()),
                "repeater_scale" => repeater
                    .scale_offset_animation
                    .as_ref()
                    .and_then(|a| a.keyframes()),
                _ => repeater
                    .opacity_animation
                    .as_ref()
                    .and_then(|a| a.keyframes()),
            });
            if let Some(keyframes) = keyframes {
                out.extend(keyframes.iter().map(|keyframe| keyframe.frame));
            }
        }
        _ if crate::ui::graph_editor::is_effect_property(pk) => {
            use crate::core::effect_params::ParamRefRef;
            let Some((effect_id, label, _)) =
                crate::ui::graph_editor::parse_effect_property(pk)
            else {
                return out;
            };
            for eff in &layer.effects {
                if eff.id != effect_id {
                    continue;
                }
                for (plabel, pref) in eff.effect_type.animatable_params_ref() {
                    if plabel != label {
                        continue;
                    }
                    let frames: Vec<u32> = match pref {
                        ParamRefRef::Scalar(a) => a
                            .keyframes()
                            .map(|ks| ks.iter().map(|k| k.frame).collect())
                            .unwrap_or_default(),
                        ParamRefRef::Vec2(a) => a
                            .keyframes()
                            .map(|ks| ks.iter().map(|k| k.frame).collect())
                            .unwrap_or_default(),
                        ParamRefRef::Vec3(a) => a
                            .keyframes()
                            .map(|ks| ks.iter().map(|k| k.frame).collect())
                            .unwrap_or_default(),
                        ParamRefRef::Vec4Color(a) => a
                            .keyframes()
                            .map(|ks| ks.iter().map(|k| k.frame).collect())
                            .unwrap_or_default(),
                    };
                    out.extend(frames);
                    break;
                }
            }
        }
        _ if pk.starts_with("pin_") => {
            let pid = pk.strip_prefix("pin_").unwrap_or("");
            if let Some(pin) = layer.puppet_pins.iter().find(|p| p.id == pid) {
                if let Some(kfs) = pin.position.keyframes() {
                    for k in kfs {
                        out.push(k.frame);
                    }
                }
            }
        }
        _ => {}
    }
    out.sort_unstable();
    out.dedup();
    out
}

pub fn apply_group_moves(
    layer: &mut Layer,
    layer_idx: usize,
    selected_keyframes: &std::collections::HashSet<(usize, String, u32)>,
    group_moves: Vec<(String, u32, i32)>,
) {
    let t = &mut layer.transform;
    for (pk, dragged_f, delta) in group_moves {
        let followers: Vec<u32> = selected_keyframes
            .iter()
            .filter(|(li, p, fr)| *li == layer_idx && p == &pk && *fr != dragged_f)
            .map(|(_, _, fr)| *fr)
            .collect();
        if followers.is_empty() {
            continue;
        }
        let new_frames: Vec<(u32, u32)> = followers
            .iter()
            .map(|&f| (f, (f as i64 + delta as i64).max(0) as u32))
            .collect();
        macro_rules! shift_track {
            ($anim:expr) => {{
                for (old_f, new_f) in &new_frames {
                    move_kf($anim, *old_f, *new_f);
                }
                if let Some(kfs) = $anim.keyframes_mut() {
                    kfs.sort_by_key(|k| k.frame);
                }
            }};
        }
        match pk.as_str() {
            "position" => shift_track!(&mut t.position),
            "scale" => shift_track!(&mut t.scale),
            "rotation" => shift_track!(&mut t.rotation),
            "opacity" => shift_track!(&mut t.opacity),
            "repeater_copies" => {
                if let Some(a) = layer
                    .shape_repeater
                    .as_mut()
                    .and_then(|r| r.copies_animation.as_mut())
                {
                    shift_track!(a);
                }
            }
            "repeater_position" => {
                if let Some(a) = layer
                    .shape_repeater
                    .as_mut()
                    .and_then(|r| r.position_offset_animation.as_mut())
                {
                    shift_track!(a);
                }
            }
            "repeater_scale" => {
                if let Some(a) = layer
                    .shape_repeater
                    .as_mut()
                    .and_then(|r| r.scale_offset_animation.as_mut())
                {
                    shift_track!(a);
                }
            }
            "repeater_rotation" => {
                if let Some(a) = layer
                    .shape_repeater
                    .as_mut()
                    .and_then(|r| r.rotation_offset_animation.as_mut())
                {
                    shift_track!(a);
                }
            }
            "repeater_opacity" => {
                if let Some(a) = layer
                    .shape_repeater
                    .as_mut()
                    .and_then(|r| r.opacity_animation.as_mut())
                {
                    shift_track!(a);
                }
            }
            _ if pk.starts_with("pin_") => {
                let pid = pk.strip_prefix("pin_").unwrap_or("");
                if let Some(pin) = layer.puppet_pins.iter_mut().find(|p| p.id == pid) {
                    shift_track!(&mut pin.position);
                }
            }
            _ => {}
        }
    }
}

pub fn apply_selection_requests(
    layer: &Layer,
    layer_idx: usize,
    selected_keyframes: &mut std::collections::HashSet<(usize, String, u32)>,
    mut select_all_reqs: Vec<String>,
    mut select_requests: Vec<(String, u32, bool, bool)>,
    project_changed: &mut bool,
) {
    for pk in select_all_reqs.drain(..) {
        select_requests.push((pk, u32::MAX, false, false));
    }

    for (pk, f, shift, cmd) in select_requests {
        if f == u32::MAX {
            let all = prop_all_frames(layer, &pk);
            if shift || cmd {
                for fr in all {
                    selected_keyframes.insert((layer_idx, pk.to_string(), fr));
                }
            } else {
                selected_keyframes.clear();
                for fr in all {
                    selected_keyframes.insert((layer_idx, pk.to_string(), fr));
                }
            }
            *project_changed = false;
            continue;
        }
        let entry = (layer_idx, pk.to_string(), f);
        if shift || cmd {
            if !selected_keyframes.remove(&entry) {
                selected_keyframes.insert(entry);
            }
        } else {
            selected_keyframes.clear();
            selected_keyframes.insert(entry);
        }
        *project_changed = false;
    }
}
