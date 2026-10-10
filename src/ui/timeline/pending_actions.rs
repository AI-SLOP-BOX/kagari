//! Post-layer-loop deferred actions: drag-drop effects, row reorder swaps,
//! duration/trim changes, pre-comp navigation, ripple edits, markers,
//! duplicate/split/precompose, and keyboard trims. All mutations are queued
//! during the borrow-conflicting layer loop and applied here afterwards.
use crate::KagariApp;
use eframe::egui;

#[derive(Default)]
pub struct TimelineActions {
    pub swap_request: Option<(usize, usize)>,
    pub pending_duration: Option<u32>,
    pub pending_trim_work_area: Option<(u32, u32)>,
    pub pending_open_comp: Option<String>,
    pub pending_ripple: Option<(usize, u32, i64)>,
    pub pending_layer_marker: Option<usize>,
    pub pending_clear_markers: Option<usize>,
    pub pending_dup_layer: Option<usize>,
    pub pending_split_layer: Option<usize>,
    pub pending_precomp_indices: Option<Vec<usize>>,
    pub pending_create_shapes: Option<usize>,
    pub pending_select_all_kfs: Option<usize>,
    pub pending_pick_whip: Option<(usize, usize)>,
    pub pending_effect_drops: Vec<(usize, String, usize)>,
    pub pending_select_label_group: Option<crate::core::timeline::LabelColor>,
}

pub fn apply_deferred(
    app: &mut KagariApp,
    ui: &egui::Ui,
    current_frame: u32,
    project_changed: &mut bool,
    actions: &mut TimelineActions,
) {
    if let Some(idx) = actions.pending_select_all_kfs.take() {
        select_all_layer_keyframes(app, idx);
    }

    if let Some(target_label) = actions.pending_select_label_group.take() {
        let matching_layers: Vec<usize> = app
            .history
            .current()
            .active_composition()
            .layers
            .iter()
            .enumerate()
            .filter_map(|(idx, layer)| (layer.label == target_label).then_some(idx))
            .collect();
        app.selection.selected_layers = matching_layers.iter().copied().collect();
        if let Some(&last) = matching_layers.last() {
            app.selection.selected_layer_idx = Some(last);
        }
        app.toasts.info(format!(
            "Selected {} layers in label group",
            app.selection.selected_layers.len()
        ));
    }

    apply_effect_drops(
        app,
        std::mem::take(&mut actions.pending_effect_drops),
        project_changed,
    );
    apply(app, ui, current_frame, project_changed, actions);

    if let Some((child_idx, parent_idx)) = actions.pending_pick_whip.take() {
        let comp_mut = app.history.current_mut().active_composition_mut();
        if child_idx < comp_mut.layers.len() && parent_idx < comp_mut.layers.len() {
            let parent_id = comp_mut.layers[parent_idx].id.clone();
            let child_name = comp_mut.layers[child_idx].name.clone();
            let parent_name = comp_mut.layers[parent_idx].name.clone();
            comp_mut.layers[child_idx].parent_id = Some(parent_id);
            app.toasts
                .info(format!("Parented '{}' → '{}'", child_name, parent_name));
            *project_changed = true;
        }
    }

    if let Some(idx) = actions.pending_create_shapes.take() {
        let comp_mut = app.history.current_mut().active_composition_mut();
        if idx < comp_mut.layers.len() {
            if let Some(shape_layer) = crate::core::text_to_shapes::convert_text_to_shapes(
                &comp_mut.layers[idx],
                comp_mut.width,
                comp_mut.height,
            ) {
                let name = comp_mut.layers[idx].name.clone();
                comp_mut.layers.insert(idx, shape_layer);
                app.toasts.info(format!("Created shapes from '{}'", name));
                *project_changed = true;
            }
        }
    }
}

fn select_all_layer_keyframes(app: &mut KagariApp, idx: usize) {
    let mut collected: Vec<(String, u32)> = Vec::new();
    if let Some(layer) = app.history.current().active_composition().layers.get(idx) {
        macro_rules! track {
            ($key:expr, $anim:expr) => {
                if let Some(kfs) = $anim.keyframes() {
                    for keyframe in kfs {
                        collected.push(($key.to_string(), keyframe.frame));
                    }
                }
            };
        }
        track!("position", layer.transform.position);
        track!("scale", layer.transform.scale);
        track!("rotation", layer.transform.rotation);
        track!("opacity", layer.transform.opacity);
        for effect in &layer.effects {
            for (label, param) in effect.effect_type.animatable_params_ref() {
                let key = format!("fxid:{}::{}", effect.id, label);
                match param {
                    crate::core::effect_params::ParamRefRef::Scalar(anim) => {
                        track!(key.as_str(), anim)
                    }
                    crate::core::effect_params::ParamRefRef::Vec2(anim) => {
                        track!(key.as_str(), anim)
                    }
                    crate::core::effect_params::ParamRefRef::Vec3(anim) => {
                        track!(key.as_str(), anim)
                    }
                    crate::core::effect_params::ParamRefRef::Vec4Color(anim) => {
                        track!(key.as_str(), anim)
                    }
                }
            }
        }
        for pin in &layer.puppet_pins {
            track!(format!("pin_{}", pin.id), pin.position);
        }
    }

    if collected.is_empty() {
        app.toasts.info("Layer has no keyframes");
    } else {
        app.selected_keyframes.clear();
        for (property, frame) in collected {
            app.selected_keyframes.insert((idx, property, frame));
        }
    }
}

pub fn apply(
    app: &mut KagariApp,
    ui: &egui::Ui,
    current_frame: u32,
    project_changed: &mut bool,
    actions: &mut TimelineActions,
) {
    if let Some((a, b)) = actions.swap_request.take() {
        apply_swap(app, a, b, project_changed);
    }
    if let Some(dur) = actions.pending_duration.take() {
        app.history
            .current_mut()
            .active_composition_mut()
            .duration_frames = dur;
        *project_changed = true;
    }
    if let Some((w_in, w_out)) = actions.pending_trim_work_area.take() {
        trim_layers_to_work_area(app, w_in, w_out, project_changed);
    }
    if let Some(comp_id) = actions.pending_open_comp.take() {
        open_nested_comp(app, &comp_id);
    }
    if let Some((idx, old_out, shift)) = actions.pending_ripple.take() {
        ripple_edit(app, idx, old_out, shift, project_changed);
    }
    if let Some(idx) = actions.pending_layer_marker.take() {
        add_layer_marker(app, idx, current_frame, project_changed);
    }
    if let Some(idx) = actions.pending_clear_markers.take() {
        clear_layer_markers(app, idx, project_changed);
    }
    toggle_marker_alt_m(app, ui, current_frame, project_changed);
    if let Some(idx) = actions.pending_dup_layer.take() {
        duplicate_layer_at(app, idx, project_changed, "");
    }
    if let Some(idx) = actions.pending_split_layer.take() {
        split_layer_at(app, idx, current_frame, project_changed);
    }
    if let Some(selected_indices) = actions.pending_precomp_indices.take() {
        precompose_selected(app, selected_indices, project_changed);
    }
    duplicate_shortcut_cmd_d(app, ui, current_frame, project_changed);
    trim_in_out_shortcuts(app, ui, current_frame, project_changed);
}

fn apply_swap(app: &mut KagariApp, a: usize, b: usize, project_changed: &mut bool) {
    let temp_project = app.history.current_mut();
    if a < temp_project.active_composition().layers.len()
        && b < temp_project.active_composition().layers.len()
    {
        temp_project.active_composition_mut().layers.swap(a, b);
        // Selection must follow the swapped rows, otherwise a drag leaves
        // the selection pointing at the wrong layer.
        let remap = |idx: usize| -> usize {
            if idx == a {
                b
            } else if idx == b {
                a
            } else {
                idx
            }
        };
        app.selection.selected_layers = app
            .selection
            .selected_layers
            .iter()
            .map(|i| remap(*i))
            .collect();
        if let Some(sel) = app.selection.selected_layer_idx {
            app.selection.selected_layer_idx = Some(remap(sel));
        }
        app.selection.expanded_layers = app
            .selection
            .expanded_layers
            .iter()
            .map(|i| remap(*i))
            .collect();
        *project_changed = true;
    }
}

fn trim_layers_to_work_area(
    app: &mut KagariApp,
    w_in: u32,
    w_out: u32,
    project_changed: &mut bool,
) {
    let temp_project = app.history.current_mut();
    for layer in temp_project.active_composition_mut().layers.iter_mut() {
        layer.in_frame = layer.in_frame.max(w_in);
        layer.out_frame = layer.out_frame.min(w_out);
        if layer.in_frame >= layer.out_frame {
            layer.out_frame = layer.in_frame + 1;
        }
    }
    *project_changed = true;
}

fn open_nested_comp(app: &mut KagariApp, comp_id: &str) {
    let temp_project = app.history.current_mut();
    // First search top-level compositions
    if let Some(c_idx) = temp_project
        .compositions
        .iter()
        .position(|c| c.id == comp_id)
    {
        if c_idx != temp_project.active_composition_idx {
            app.comp_nav_stack.push(temp_project.active_composition_idx);
        }
        temp_project.active_composition_idx = c_idx;
        crate::core::frame_cache::bump_version();
        let name = temp_project.compositions[c_idx].name.clone();
        app.toasts
            .info(format!("Opened nested composition: {}", name));
    } else if let Some(sub) = temp_project.active_composition().find_sub_comp(comp_id) {
        // Found in sub_compositions — navigate by adding to top-level if needed
        let name = sub.name.clone();
        app.toasts
            .info(format!("Opened nested composition: {}", name));
    } else {
        app.toasts.error("Nested composition not found");
    }
}

fn ripple_edit(
    app: &mut KagariApp,
    idx: usize,
    old_out: u32,
    shift: i64,
    project_changed: &mut bool,
) {
    let temp_project = app.history.current_mut();
    let mut changed = false;
    for l2 in temp_project
        .active_composition_mut()
        .layers
        .iter_mut()
        .skip(idx + 1)
    {
        if l2.in_frame >= old_out {
            l2.in_frame = (l2.in_frame as i64 + shift).max(0) as u32;
            l2.out_frame = (l2.out_frame as i64 + shift).max(l2.in_frame as i64 + 1) as u32;
            changed = true;
        }
    }
    if changed {
        *project_changed = true;
        app.toasts.info("Ripple edit applied");
    }
}

fn add_layer_marker(
    app: &mut KagariApp,
    idx: usize,
    current_frame: u32,
    project_changed: &mut bool,
) {
    let temp_project = app.history.current_mut();
    if let Some(layer) = temp_project.active_composition_mut().layers.get_mut(idx) {
        layer.markers.push(crate::core::timeline::TimelineMarker {
            frame: current_frame,
            label: format!("Marker {}", layer.markers.len() + 1),
            color: [0.95, 0.85, 0.10],
        });
        *project_changed = true;
        app.toasts
            .info(format!("Layer marker at frame {}", current_frame));
    }
}

fn clear_layer_markers(app: &mut KagariApp, idx: usize, project_changed: &mut bool) {
    let temp_project = app.history.current_mut();
    if let Some(layer) = temp_project.active_composition_mut().layers.get_mut(idx) {
        layer.markers.clear();
        *project_changed = true;
    }
}

fn toggle_marker_alt_m(
    app: &mut KagariApp,
    ui: &egui::Ui,
    current_frame: u32,
    project_changed: &mut bool,
) {
    if !ui.input(|inp| inp.modifiers.alt && inp.key_pressed(egui::Key::M)) {
        return;
    }
    let temp_project = app.history.current_mut();
    if let Some(sel_idx) = app.selection.selected_layer_idx {
        if let Some(layer) = temp_project
            .active_composition_mut()
            .layers
            .get_mut(sel_idx)
        {
            if let Some(pos) = layer.markers.iter().position(|m| m.frame == current_frame) {
                layer.markers.remove(pos);
                app.toasts.info("Layer marker removed");
            } else {
                layer.markers.push(crate::core::timeline::TimelineMarker {
                    frame: current_frame,
                    label: format!("Marker {}", layer.markers.len() + 1),
                    color: [0.95, 0.85, 0.10],
                });
                app.toasts
                    .info(format!("Layer marker at frame {}", current_frame));
            }
            *project_changed = true;
        }
    }
}

/// Insert a clone of `idx`'s layer directly below it. `suffix_extra`
/// disambiguates ids when invoked from both menu ("_copy_N") and shortcut ("_copy").
fn duplicate_layer_at(
    app: &mut KagariApp,
    idx: usize,
    project_changed: &mut bool,
    id_suffix: &str,
) {
    let temp_project = app.history.current_mut();
    let layers_len = temp_project.active_composition().layers.len();
    if idx < layers_len {
        let mut cloned = temp_project.active_composition().layers[idx].clone();
        cloned.id = format!("{}_copy{}{}", cloned.id, id_suffix, layers_len);
        cloned.name = format!("{} copy", cloned.name);
        temp_project
            .active_composition_mut()
            .layers
            .insert(idx + 1, cloned);
        app.selection.selected_layer_idx = Some(idx + 1);
        app.selection.selected_layers.clear();
        app.selection.selected_layers.insert(idx + 1);
        *project_changed = true;
    }
}

fn split_layer_at(app: &mut KagariApp, idx: usize, current_frame: u32, project_changed: &mut bool) {
    let temp_project = app.history.current_mut();
    let comp = temp_project.active_composition_mut();
    let layers_len = comp.layers.len();
    if idx >= layers_len {
        return;
    }
    let orig_out = comp.layers[idx].out_frame;
    if current_frame > comp.layers[idx].in_frame && current_frame < orig_out {
        let split_id = comp.next_layer_id(&format!("{}_split", comp.layers[idx].id));
        // 1) Head keeps [in .. cur)
        comp.layers[idx].out_frame = current_frame;
        // 2) Tail is a fresh layer covering [cur .. out)
        let mut tail = comp.layers[idx].clone();
        tail.id = split_id;
        tail.name = format!("{} split", tail.name);
        tail.in_frame = current_frame;
        tail.out_frame = orig_out;
        comp.layers.insert(idx + 1, tail);
        app.selection.selected_layer_idx = Some(idx + 1);
        app.selection.selected_layers.clear();
        app.selection.selected_layers.insert(idx + 1);
        *project_changed = true;
        app.toasts
            .info(format!("Split layer at frame {}", current_frame));
    } else {
        app.toasts
            .error("Split point must be inside the layer's duration");
    }
}

fn precompose_selected(
    app: &mut KagariApp,
    selected_indices: Vec<usize>,
    project_changed: &mut bool,
) {
    let temp_project = app.history.current_mut();
    let precomp_id = temp_project.next_composition_id("precomp");
    let comp_len = temp_project.compositions.len();
    let (c_w, c_h, c_fps, c_dur) = {
        let active = temp_project.active_composition();
        (
            active.width,
            active.height,
            active.fps,
            active.duration_frames,
        )
    };
    let precomp_name = format!("Pre-comp {}", comp_len + 1);
    let mut new_comp = crate::core::timeline::Composition::new(
        precomp_id.clone(),
        precomp_name.clone(),
        c_w,
        c_h,
        c_fps,
        c_dur,
    );

    let active_comp = temp_project.active_composition_mut();
    let mut extracted_layers = Vec::new();
    for &idx in selected_indices.iter().rev() {
        if idx < active_comp.layers.len() {
            extracted_layers.push(active_comp.layers.remove(idx));
        }
    }
    extracted_layers.reverse();
    new_comp.layers = extracted_layers;

    let moved_layers = new_comp.layers.clone();
    active_comp.move_scene_objects_to(&mut new_comp, &moved_layers);

    let precomp_layer = crate::core::timeline::Layer::new(
        active_comp.next_layer_id("precomp"),
        precomp_name,
        crate::core::timeline::LayerType::PreComp {
            comp_id: precomp_id,
        },
        c_dur,
    );
    let insert_pos = selected_indices
        .first()
        .copied()
        .unwrap_or(0)
        .min(active_comp.layers.len());
    active_comp.layers.insert(insert_pos, precomp_layer);
    temp_project.compositions.push(new_comp);

    app.selection.selected_layers.clear();
    app.selection.selected_layers.insert(insert_pos);
    app.selection.selected_layer_idx = Some(insert_pos);
    *project_changed = true;
}

fn duplicate_shortcut_cmd_d(
    app: &mut KagariApp,
    ui: &egui::Ui,
    current_frame: u32,
    project_changed: &mut bool,
) {
    let is_split =
        ui.input(|i| i.modifiers.command && i.modifiers.shift && i.key_pressed(egui::Key::D));
    let is_dup =
        ui.input(|i| i.modifiers.command && !i.modifiers.shift && i.key_pressed(egui::Key::D));

    if is_split {
        if let Some(sel_idx) = app.selection.selected_layer_idx {
            split_layer_at(app, sel_idx, current_frame, project_changed);
        }
        return;
    }

    if is_dup {
        if let Some(sel_idx) = app.selection.selected_layer_idx {
            duplicate_layer_at(app, sel_idx, project_changed, " (Cmd+D)");
        }
    }
}

fn trim_in_out_shortcuts(
    app: &mut KagariApp,
    ui: &egui::Ui,
    current_frame: u32,
    project_changed: &mut bool,
) {
    let trim_in = ui.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::OpenBracket));
    let trim_out = ui.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::CloseBracket));
    if !trim_in && !trim_out {
        return;
    }
    let temp_project = app.history.current_mut();
    if let Some(sel_idx) = app.selection.selected_layer_idx {
        if let Some(layer) = temp_project
            .active_composition_mut()
            .layers
            .get_mut(sel_idx)
        {
            if trim_in {
                layer.in_frame = current_frame.min(layer.out_frame.saturating_sub(1));
                *project_changed = true;
                app.toasts
                    .info(format!("Trimmed In Point to frame {}", current_frame));
            } else {
                layer.out_frame = current_frame.max(layer.in_frame + 1);
                *project_changed = true;
                app.toasts
                    .info(format!("Trimmed Out Point to frame {}", current_frame));
            }
        }
    }
}

/// Apply queued effect drag-drops from the effects library onto layer rows.
pub fn apply_effect_drops(
    app: &mut KagariApp,
    drops: Vec<(usize, String, usize)>,
    project_changed: &mut bool,
) {
    if drops.is_empty() {
        return;
    }
    let temp_project = app.history.current_mut();
    let presets = crate::ui::effects_controls::get_all_effect_presets();
    for (layer_idx, effect_name, preset_idx) in drops {
        if let Some(preset) = presets.get(preset_idx) {
            let comp = temp_project.active_composition_mut();
            if layer_idx < comp.layers.len() {
                let effect = (preset.create_fn)(comp.layers[layer_idx].effects.len());
                comp.layers[layer_idx].effects.push(effect);
                *project_changed = true;
                app.toasts.info(format!(
                    "Applied '{}' to '{}'",
                    effect_name, comp.layers[layer_idx].name
                ));
            }
        }
    }
    app.dragging_effect = None;
}
