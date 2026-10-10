mod commands;
mod selection;

use super::keyframe_menu::{self, KeyframeMenuAction};
use super::layers::{draw_prop_row, draw_prop_row_ext, PropertyRowAction};
use super::utils::get_kfs;
use crate::core::timeline::Layer;
use crate::ui::theme::colors;
use eframe::egui;

pub struct ExpandedRowsState<'a> {
    pub layer: &'a mut Layer,
    pub layer_index: usize,
    pub selected_keyframes: &'a mut std::collections::HashSet<(usize, String, u32)>,
    pub selected_property: &'a mut Option<String>,
    pub current_frame: &'a mut u32,
    pub start_frame: u32,
    pub zoom_span: u32,
    pub left_pane_w: f32,
    pub all_kf_frames: &'a [u32],
    pub precomp_children: &'a [(usize, Vec<super::precomp_children::PreCompChild>, String)],
    pub open_comp_request: &'a mut Option<String>,
    pub project_changed: &'a mut bool,
}

pub fn draw_expanded_rows(ui: &mut egui::Ui, state: ExpandedRowsState<'_>) {
    let ExpandedRowsState {
        layer,
        layer_index: i,
        selected_keyframes,
        selected_property,
        current_frame,
        start_frame,
        zoom_span,
        left_pane_w,
        all_kf_frames,
        precomp_children,
        open_comp_request,
        project_changed,
    } = state;

    let pos_kfs = get_kfs(&layer.transform.position);
    let scale_kfs = get_kfs(&layer.transform.scale);
    let rot_kfs = get_kfs(&layer.transform.rotation);
    let op_kfs = get_kfs(&layer.transform.opacity);
    let position_value = layer.transform.position.evaluate(*current_frame);
    let scale_value = layer.transform.scale.evaluate(*current_frame);
    let rotation_value = layer.transform.rotation.evaluate(*current_frame);
    let opacity_value = layer.transform.opacity.evaluate(*current_frame);

    // Keyframes selected for this layer: (prop_key, frame)
    let prop_sel: std::collections::HashSet<(String, u32)> = selected_keyframes
        .iter()
        .filter(|(li, _, _)| *li == i)
        .map(|(_, pk, f)| (pk.clone(), *f))
        .collect();

    // Collect selection toggles first; apply to app after the
    // row borrows end (app and layer cannot borrow together).
    let mut select_requests: Vec<(String, u32, bool, bool)> = Vec::new();
    let mut select_all_reqs: Vec<String> = Vec::new();
    let mut kf_menu_cmds: Vec<(String, u32, KeyframeMenuAction)> = Vec::new();

    // Marquee box-select results: (prop_key, boxed frames)
    let mut box_selects: Vec<(String, Vec<u32>)> = Vec::new();
    // Group keyframe moves: (prop_key, dragged_frame, delta)
    let mut group_moves: Vec<(String, u32, i32)> = Vec::new();

    macro_rules! draw_property_row {
        ($row_ui:expr, $label:expr, $anim:expr, $key:expr $(,)?) => {{
            let label = $label;
            let anim = $anim;
            let kfs = get_kfs(anim);
            let response = draw_prop_row_ext(
                $row_ui,
                &label,
                &kfs,
                current_frame,
                start_frame,
                zoom_span,
                left_pane_w,
                &prop_sel,
                $key,
                all_kf_frames,
            );
            for action in response.actions {
                match action {
                    PropertyRowAction::MoveKeyframe {
                        old_frame,
                        new_frame,
                    } => {
                        commands::move_kf(anim, old_frame, new_frame);
                        *project_changed = true;
                    }
                    PropertyRowAction::SelectKeyframe {
                        property_key,
                        frame,
                        shift,
                        command,
                    } => select_requests.push((property_key, frame, shift, command)),
                    PropertyRowAction::OpenMenu {
                        property_key,
                        frame,
                        response,
                    } => keyframe_menu::attach(&response, &property_key, frame, &mut kf_menu_cmds),
                    PropertyRowAction::BoxSelect {
                        property_key,
                        frames,
                        additive: _,
                    } => box_selects.push((property_key, frames)),
                    PropertyRowAction::MoveSelection {
                        property_key,
                        dragged_frame,
                        delta,
                    } => group_moves.push((property_key, dragged_frame, delta)),
                    PropertyRowAction::SelectAll { property_key } => {
                        select_all_reqs.push(property_key)
                    }
                }
            }
        }};
    }

    {
        let t = &mut layer.transform;
        // ── U / UU / A reveal modes (AE parity) ──
        let reveal_mode = selected_property.clone().unwrap_or_default();
        let show_transform_rows = reveal_mode != "Anchor Point";
        let is_position_solo = reveal_mode.starts_with("Position");
        let is_scale_solo = reveal_mode.starts_with("Scale");
        let is_rotation_solo = reveal_mode == "Rotation";
        let is_opacity_solo = reveal_mode == "Opacity";
        let is_specific_solo =
            is_position_solo || is_scale_solo || is_rotation_solo || is_opacity_solo;
        let kf_only = reveal_mode == "Keyframed";

        if show_transform_rows
            && (!is_specific_solo || is_position_solo)
            && (!kf_only || !pos_kfs.is_empty())
        {
            draw_property_row!(
                ui,
                format!(
                    "  Position    {:.0}, {:.0}",
                    position_value[0], position_value[1]
                ),
                &mut t.position,
                "position",
            );
        }
        if show_transform_rows
            && (!is_specific_solo || is_scale_solo)
            && (!kf_only || !scale_kfs.is_empty())
        {
            draw_property_row!(
                ui,
                format!(
                    "  Scale       {:.0}%, {:.0}%",
                    scale_value[0], scale_value[1]
                ),
                &mut t.scale,
                "scale",
            );
        }
        if show_transform_rows
            && (!is_specific_solo || is_rotation_solo)
            && (!kf_only || !rot_kfs.is_empty())
        {
            draw_property_row!(
                ui,
                format!("  Rotation    {:.1}°", rotation_value),
                &mut t.rotation,
                "rotation",
            );
        }
        if show_transform_rows
            && (!is_specific_solo || is_opacity_solo)
            && (!kf_only || !op_kfs.is_empty())
        {
            draw_property_row!(
                ui,
                format!("  Opacity     {:.0}%", opacity_value),
                &mut t.opacity,
                "opacity",
            );
        }
        if reveal_mode == "Anchor Point" {
            let ap_kfs = get_kfs(&t.anchor_point);
            let moved: &mut bool = project_changed;
            draw_prop_row(
                ui,
                "  Anchor Point",
                &ap_kfs,
                current_frame,
                start_frame,
                zoom_span,
                left_pane_w,
                Some(&mut |old_f, new_f| {
                    commands::move_kf(&mut t.anchor_point, old_f, new_f);
                    *moved = true;
                }),
            );
        }
    }

    if let Some(remap) = layer.time_remap.as_mut() {
        let current_val = remap.evaluate(*current_frame);
        draw_property_row!(
            ui,
            format!("  Time Remap  {:.2}s", current_val),
            remap,
            "time_remap",
        );
    }

    // Shape Repeater animation tracks
    if let Some(repeater) = layer.shape_repeater.as_mut() {
        macro_rules! repeater_row {
            ($anim:expr, $label:expr, $key:expr) => {{
                draw_property_row!(ui, $label, $anim, $key);
            }};
        }
        if let Some(copies) = repeater.copies_animation.as_mut() {
            repeater_row!(copies, "  Repeater Copies", "repeater_copies");
        }
        if let Some(position) = repeater.position_offset_animation.as_mut() {
            repeater_row!(position, "  Repeater Position", "repeater_position");
        }
        if let Some(scale) = repeater.scale_offset_animation.as_mut() {
            repeater_row!(scale, "  Repeater Scale", "repeater_scale");
        }
        if let Some(rotation) = repeater.rotation_offset_animation.as_mut() {
            repeater_row!(rotation, "  Repeater Rotation", "repeater_rotation");
        }
        if let Some(opacity) = repeater.opacity_animation.as_mut() {
            repeater_row!(opacity, "  Repeater Opacity", "repeater_opacity");
        }
    }

    // Effect keyframe rows
    for effect in layer.effects.iter_mut() {
        let fx_name = effect.name.clone();
        use crate::core::effect_params::ParamRef;
        let fx_enabled = effect.enabled;
        let hdr = egui::CollapsingHeader::new(
            egui::RichText::new(format!(
                "{} {}",
                if fx_enabled {
                    egui_phosphor::regular::POWER
                } else {
                    egui_phosphor::regular::PROHIBIT
                },
                fx_name
            ))
            .small()
            .color(if fx_enabled {
                colors::TEXT_PRIMARY
            } else {
                colors::TEXT_MUTED
            }),
        )
        .id_salt(ui.make_persistent_id(format!("fxgrp_{}_{}", i, fx_name)))
        .default_open(false);
        hdr.show(ui, |ui| {
            for (label, param) in effect.effect_type.animatable_params() {
                let prop_key = format!("fxid:{}::{}", effect.id, label);
                let row_label = format!("  [{}] {}", fx_name, label);
                match param {
                    ParamRef::Scalar(anim) => draw_property_row!(ui, &row_label, anim, &prop_key),
                    ParamRef::Vec2(anim) => draw_property_row!(ui, &row_label, anim, &prop_key),
                    ParamRef::Vec3(anim) => draw_property_row!(ui, &row_label, anim, &prop_key),
                    ParamRef::Vec4Color(anim) => {
                        draw_property_row!(ui, &row_label, anim, &prop_key)
                    }
                }
            }
        });
    }

    // Puppet Pin rows
    for pin in layer.puppet_pins.iter_mut() {
        let pin_key = format!("pin_{}", pin.id);
        draw_property_row!(ui, format!("  {}", pin.name), &mut pin.position, &pin_key);
    }

    // Apply keyframe context-menu commands
    commands::apply_kf_menu_commands(layer, i, selected_keyframes, kf_menu_cmds, project_changed);

    // Apply marquee box-selects
    for (pk, frames) in box_selects {
        for f in frames {
            selected_keyframes.insert((i, pk.to_string(), f));
        }
        *project_changed = false;
    }

    // Group move
    selection::apply_group_moves(layer, i, selected_keyframes, group_moves);

    // Apply selection requests
    selection::apply_selection_requests(
        layer,
        i,
        selected_keyframes,
        select_all_reqs,
        select_requests,
        project_changed,
    );

    // Time Remap row (when enabled)
    if let Some(remap) = layer.time_remap.as_mut() {
        let moved_r: &mut bool = project_changed;
        let tr_kfs = get_kfs(remap);
        draw_prop_row(
            ui,
            "  Time Remap",
            &tr_kfs,
            current_frame,
            start_frame,
            zoom_span,
            left_pane_w,
            Some(&mut |o, n| {
                commands::move_kf(remap, o, n);
                *moved_r = true;
            }),
        );
    }

    // Mask property rows
    for (m_idx, mask) in layer.masks.iter_mut().enumerate() {
        let moved_m: &mut bool = project_changed;
        ui.label(
            egui::RichText::new(format!("  {}", mask.name))
                .small()
                .strong()
                .color(colors::TEXT_SECONDARY),
        );
        let f_kfs = get_kfs(&mask.feather);
        draw_prop_row(
            ui,
            "    Feather",
            &f_kfs,
            current_frame,
            start_frame,
            zoom_span,
            left_pane_w,
            Some(&mut |o, n| {
                commands::move_kf(&mut mask.feather, o, n);
                *moved_m = true;
            }),
        );
        let op_kfs_m = get_kfs(&mask.opacity);
        draw_prop_row(
            ui,
            "    Opacity",
            &op_kfs_m,
            current_frame,
            start_frame,
            zoom_span,
            left_pane_w,
            Some(&mut |o, n| {
                commands::move_kf(&mut mask.opacity, o, n);
                *moved_m = true;
            }),
        );
        let ex_kfs = get_kfs(&mask.expansion);
        draw_prop_row(
            ui,
            "    Expansion",
            &ex_kfs,
            current_frame,
            start_frame,
            zoom_span,
            left_pane_w,
            Some(&mut |o, n| {
                commands::move_kf(&mut mask.expansion, o, n);
                *moved_m = true;
            }),
        );
        let p_kfs = get_kfs(&mask.path.vertices);
        draw_prop_row(
            ui,
            &format!("    Path ({} pts)", p_kfs.len()),
            &p_kfs,
            current_frame,
            start_frame,
            zoom_span,
            left_pane_w,
            Some(&mut |o, n| {
                commands::move_kf(&mut mask.path.vertices, o, n);
                *moved_m = true;
            }),
        );
        let _ = m_idx;
    }

    // Pre-comp nested child layers
    if let Some((_parent_idx, children, sub_id)) =
        precomp_children.iter().find(|(pi, _, _)| *pi == i)
    {
        if crate::ui::timeline::precomp_children::draw_children_rows(
            ui,
            sub_id,
            children,
            left_pane_w,
            start_frame,
            zoom_span,
        ) {
            *open_comp_request = Some(sub_id.clone());
        }
    }
}
