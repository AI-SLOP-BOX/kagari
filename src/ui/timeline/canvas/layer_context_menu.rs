//! Context menu logic for timeline layer items.

use eframe::egui;
use crate::core::timeline::{LabelColor, Layer};
use crate::ui::theme::colors;
use crate::ui::timeline::pending_actions::TimelineActions;

pub struct LayerContextMenuCtx<'a> {
    pub selected_layers: &'a std::collections::HashSet<usize>,
    pub toasts: &'a mut crate::ui::notification::ToastManager,
}

pub fn draw_layer_context_menu(
    ui: &mut egui::Ui,
    ctx: &mut LayerContextMenuCtx<'_>,
    layer: &mut Layer,
    i: usize,
    layers_len: usize,
    current_frame: u32,
    comp_w_f: f32,
    actions: &mut TimelineActions,
    project_changed: &mut bool,
) {
    if i > 0 && ui.button("↑ 上へ移動").clicked() {
        actions.swap_request = Some((i, i - 1));
        ui.close_menu();
    }
    if i + 1 < layers_len && ui.button("↓ 下へ移動").clicked() {
        actions.swap_request = Some((i, i + 1));
        ui.close_menu();
    }
    ui.separator();
    if ui.button("Duplicate Layer (Cmd+D)").clicked() {
        actions.pending_dup_layer = Some(i);
        ctx.toasts.info("Duplicated selected layer");
        ui.close_menu();
    }
    if ui.button("Pre-compose… (Cmd+Shift+C)").on_hover_text("Nest this layer into a new composition").clicked() {
        let mut s = vec![i];
        if !ctx.selected_layers.is_empty() {
            s = ctx.selected_layers.iter().copied().collect();
            s.sort();
        }
        actions.pending_precomp_indices = Some(s);
        ui.close_menu();
    }
    if ui.button("Delete Layer (Del)").clicked() {
        actions.pending_delete_layer = Some(i);
        ui.close_menu();
    }
    if matches!(layer.layer_type, crate::core::timeline::LayerType::Text { .. })
        && ui.button("Create Shapes from Text").on_hover_text("Decompose this Text layer into animatable vector Bezier Shape paths").clicked()
    {
        actions.pending_create_shapes = Some(i);
        ui.close_menu();
    }
    if ui.button("Toggle 3D Layer").on_hover_text("Switch this layer between 2D and 3D").clicked() {
        layer.is_3d = !layer.is_3d;
        *project_changed = true;
        ui.close_menu();
    }
    ui.separator();

    // ── Label Color Picker ──
    ui.menu_button("Label Color", |ui| {
        for label_col in LabelColor::ALL {
            let [r, g, b] = label_col.to_rgb();
            let col = egui::Color32::from_rgb(
                (r * 255.0).round() as u8,
                (g * 255.0).round() as u8,
                (b * 255.0).round() as u8,
            );
            let is_cur = layer.label == label_col;
            let (rect, resp) = ui.allocate_exact_size(egui::vec2(120.0, 18.0), egui::Sense::click());
            if resp.hovered() {
                ui.painter().rect_filled(rect, 2.0, colors::BG_HOVER);
            }
            ui.painter().rect_filled(
                egui::Rect::from_min_size(rect.left_top() + egui::vec2(4.0, 4.0), egui::vec2(10.0, 10.0)),
                2.0,
                col,
            );
            ui.painter().text(
                rect.left_top() + egui::vec2(20.0, 2.0),
                egui::Align2::LEFT_TOP,
                label_col.name(),
                egui::FontId::proportional(11.0),
                if is_cur { colors::TEXT_PRIMARY } else { colors::TEXT_SECONDARY },
            );
            if resp.clicked() {
                layer.label = label_col;
                *project_changed = true;
                ui.close_menu();
                ui.close_menu();
            }
        }
    });

    if ui.button(if layer.solo { "Un-Solo Layer" } else { "Solo Layer" }).clicked() {
        layer.solo = !layer.solo;
        *project_changed = true;
        ui.close_menu();
    }
    if ui.button(if layer.locked { "Unlock Layer" } else { "Lock Layer" }).clicked() {
        layer.locked = !layer.locked;
        *project_changed = true;
        ui.close_menu();
    }
    if ui.button(if layer.is_guide_layer { "Remove Guide Layer" } else { "Guide Layer" }).on_hover_text("Guide layers appear in composition preview but are excluded from exports and pre-comps").clicked() {
        layer.is_guide_layer = !layer.is_guide_layer;
        *project_changed = true;
        ui.close_menu();
    }
    if ui.button("⏪ Time-Reverse Keyframes").on_hover_text("Reverse the order of all animated keyframes on this layer").clicked() {
        fn rev<T: Clone>(a: &mut crate::core::property::Animatable<T>, in_f: u32, out_f: u32) {
            if let crate::core::property::Animatable::Animated(kfs) = a {
                let span = out_f.saturating_sub(in_f);
                for k in kfs.iter_mut() {
                    k.frame = in_f + span.saturating_sub(k.frame.saturating_sub(in_f));
                }
                kfs.sort_by_key(|k| k.frame);
            }
        }
        let (inf, outf) = (layer.in_frame, layer.out_frame);
        rev(&mut layer.transform.position, inf, outf);
        rev(&mut layer.transform.scale, inf, outf);
        rev(&mut layer.transform.rotation, inf, outf);
        rev(&mut layer.transform.opacity, inf, outf);
        *project_changed = true;
        ctx.toasts.info("Keyframes time-reversed");
        ui.close_menu();
    }
    if ui.button("⏱ Enable Time Remapping (Cmd+Opt+T)").on_hover_text("Enable variable speed time remapping on this layer").clicked() {
        layer.enable_time_remapping();
        *project_changed = true;
        ctx.toasts.info("Time remapping enabled for layer");
        ui.close_menu();
    }
    if ui.button("Split Layer at Current Time (Cmd+Shift+D)").clicked() {
        actions.pending_split_layer = Some(i);
        ui.close_menu();
    }
    ui.separator();
    if ui.button("Into ← Set In-Point to Current Time (I)").clicked() {
        layer.in_frame = current_frame;
        if layer.in_frame >= layer.out_frame {
            layer.out_frame = layer.in_frame + 1;
        }
        *project_changed = true;
        ui.close_menu();
    }
    if ui.button("→ Set Out-Point to Current Time (O)").clicked() {
        layer.out_frame = current_frame;
        if layer.out_frame <= layer.in_frame {
            layer.in_frame = layer.out_frame.saturating_sub(1);
        }
        *project_changed = true;
        ui.close_menu();
    }
    ui.separator();
    if ui.button("Add Layer Marker at Playhead").clicked() {
        actions.pending_layer_marker = Some(i);
        ui.close_menu();
    }
    if !layer.markers.is_empty() && ui.button("Clear Layer Markers").clicked() {
        actions.pending_clear_markers = Some(i);
        ui.close_menu();
    }
    if ui.button("⬚ Select All Keyframes").on_hover_text("Select every keyframe on this layer (transform, effects, pins)").clicked() {
        actions.pending_select_all_kfs = Some(i);
        ui.close_menu();
    }
    if ui.button("Select Label Group").on_hover_text("Select all layers with the same label color").clicked() {
        actions.pending_select_label_group = Some(layer.label);
        ui.close_menu();
    }

    // ── Animation Presets ──
    ui.menu_button("Animation Presets", |ui| {
        let mut last_cat = "";
        for name in crate::core::presets::NAMES {
            let cat = crate::core::presets::category_of(name);
            if cat != last_cat && !cat.is_empty() {
                if !last_cat.is_empty() { ui.separator(); }
                ui.label(egui::RichText::new(cat).strong().color(colors::TEXT_SECONDARY));
                last_cat = cat;
            }
            if ui.small_button(*name).clicked() {
                let ok = crate::core::presets::apply_by_name(name, layer, current_frame, comp_w_f, 0.0);
                if ok {
                    *project_changed = true;
                    ctx.toasts.info(format!("{} applied", name));
                } else {
                    ctx.toasts.error(format!("{}: not applicable here", name));
                }
                ui.close_menu();
                ui.close_menu();
            }
        }
        ui.separator();
        ui.label(egui::RichText::new("Motion Library (keyframe data)").strong().color(colors::TEXT_SECONDARY));
        for preset in crate::core::animation_presets::all_presets() {
            if ui.small_button(preset.name.clone()).on_hover_text(preset.description.clone()).clicked() {
                let ok = crate::core::animation_presets::apply_preset_to_layer(&preset, layer, current_frame);
                if ok {
                    *project_changed = true;
                    ctx.toasts.info(format!("{} applied", preset.name));
                } else {
                    ctx.toasts.error(format!("{}: not applicable here", preset.name));
                }
                ui.close_menu();
                ui.close_menu();
            }
        }
    });

    if !layer.paint_strokes.is_empty() && ui.button("Clear All Paint Strokes").clicked() {
        layer.paint_strokes.clear();
        *project_changed = true;
        ctx.toasts.info("All paint strokes cleared");
        ui.close_menu();
    }
    if ui.button("Reset Transform").clicked() {
        layer.transform = crate::core::timeline::Transform2D::default();
        *project_changed = true;
        ui.close_menu();
    }

    // ── Audio fades (Audio layers only) ──
    if matches!(layer.layer_type, crate::core::timeline::LayerType::Audio { .. }) {
        ui.separator();
        const FADE_F: u32 = 10;
        if ui.button("Add 10-frame Audio Fade In").clicked() {
            if let crate::core::timeline::LayerType::Audio { volume, .. } = &mut layer.layer_type {
                let base = volume.evaluate(current_frame);
                let start_f = layer.in_frame;
                let end_f = (start_f + FADE_F).min(layer.out_frame);
                let mut kfs: Vec<crate::core::keyframe::Keyframe<f32>> = vec![
                    crate::core::keyframe::Keyframe::new(start_f, 0.0, crate::core::keyframe::InterpolationType::Linear),
                    crate::core::keyframe::Keyframe::new(end_f.max(start_f + 1), base, crate::core::keyframe::InterpolationType::Linear),
                ];
                if let Some(old) = volume.keyframes() {
                    for k in old { if k.frame > end_f { kfs.push(k.clone()); } }
                }
                kfs.sort_by_key(|k| k.frame);
                *volume = crate::core::property::Animatable::Animated(kfs);
            }
            *project_changed = true;
            ctx.toasts.info("Audio fade-in added");
            ui.close_menu();
        }
        if ui.button("Add 10-frame Audio Fade Out").clicked() {
            if let crate::core::timeline::LayerType::Audio { volume, .. } = &mut layer.layer_type {
                let base = volume.evaluate(current_frame);
                let end_f = layer.out_frame.saturating_sub(1);
                let start_f = end_f.saturating_sub(FADE_F).max(layer.in_frame);
                let mut kfs: Vec<crate::core::keyframe::Keyframe<f32>> = vec![
                    crate::core::keyframe::Keyframe::new(end_f, 0.0, crate::core::keyframe::InterpolationType::Linear),
                    crate::core::keyframe::Keyframe::new(start_f, base, crate::core::keyframe::InterpolationType::Linear),
                ];
                if let Some(old) = volume.keyframes() {
                    for k in old { if k.frame < start_f { kfs.push(k.clone()); } }
                }
                kfs.sort_by_key(|k| k.frame);
                *volume = crate::core::property::Animatable::Animated(kfs);
            }
            *project_changed = true;
            ctx.toasts.info("Audio fade-out added");
            ui.close_menu();
        }
    }

    ui.separator();
    let any_disabled = layer.effects.iter().any(|e| !e.enabled);
    let fx_label = if any_disabled { "Enable All Effects" } else { "Disable All Effects" };
    if ui.button(fx_label).clicked() {
        let target = any_disabled;
        for fx in layer.effects.iter_mut() {
            fx.enabled = target;
        }
        *project_changed = true;
        ui.close_menu();
    }
}
