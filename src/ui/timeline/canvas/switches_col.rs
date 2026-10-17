//! Layer switches (Eye, Lock, Shy, 3D, Collapse, MB) and Modes/Parenting column.

use eframe::egui;
use crate::core::timeline::{BlendMode, Layer, TrackMatteMode};
use crate::ui::theme::colors;
use crate::ui::timeline::state::{would_create_parent_cycle, TimelineDerivedData};

pub struct SwitchesColParams<'a> {
    pub layer: &'a mut Layer,
    pub i: usize,
    pub show_switches_pane: bool,
    pub derived: &'a TimelineDerivedData,
    pub pick_whip_mode: &'a mut bool,
    pub pick_whip_target: &'a mut Option<usize>,
    pub toasts: &'a mut crate::ui::notification::ToastManager,
}

pub fn draw_switches(ui: &mut egui::Ui, layer: &mut Layer, i: usize, project_changed: &mut bool) {
    use crate::ui::custom_widgets::ae_svg_toggle;
    use crate::ui::icons::*;

    let visibility_icon = if layer.visible { SVG_EYE_OPEN } else { SVG_EYE_CLOSED };
    if ae_svg_toggle(ui, &mut layer.visible, visibility_icon, &format!("row_vis_{i}"), egui::vec2(15.0, 15.0), colors::TEXT_PRIMARY, "表示／非表示 (V)").clicked() {
        *project_changed = true;
    }
    let lock_icon = if layer.locked { SVG_LOCK } else { SVG_UNLOCK };
    if ae_svg_toggle(ui, &mut layer.locked, lock_icon, &format!("row_lock_{i}"), egui::vec2(15.0, 15.0), colors::ACCENT_TOGGLE, "ロック (L)").clicked() {
        *project_changed = true;
    }
    if ae_svg_toggle(ui, &mut layer.is_shy, SVG_SHY, &format!("row_shy_{i}"), egui::vec2(15.0, 15.0), colors::ACCENT_CYAN, "シャイレイヤー (Shy)").clicked() {
        *project_changed = true;
    }
    if ae_svg_toggle(ui, &mut layer.is_collapsed, SVG_COLLAPSE_TRANSFORM, &format!("row_col_{i}"), egui::vec2(15.0, 15.0), colors::ACCENT_ORANGE, "トランスフォームの折りたたみ／コラプス").clicked() {
        *project_changed = true;
    }
    if ae_svg_toggle(ui, &mut layer.is_3d, SVG_3D_CUBE, &format!("row_3d_{i}"), egui::vec2(15.0, 15.0), colors::ACCENT_TOGGLE, "3Dレイヤー").clicked() {
        *project_changed = true;
    }
    if ae_svg_toggle(ui, &mut layer.motion_blur, SVG_MOTION_BLUR, &format!("row_mb_{i}"), egui::vec2(15.0, 15.0), colors::ACCENT_TOGGLE, "モーションブラー").clicked() {
        *project_changed = true;
    }
}

pub fn draw_modes_and_parenting(
    ui: &mut egui::Ui,
    params: SwitchesColParams<'_>,
    project_changed: &mut bool,
) {
    let SwitchesColParams {
        layer,
        i,
        show_switches_pane,
        derived,
        pick_whip_mode,
        pick_whip_target,
        toasts,
    } = params;

    if show_switches_pane {
        return;
    }

    let parent_choices_ref = &derived.parent_choices;

    ui.scope(|ui| {
        let widgets = &mut ui.style_mut().visuals.widgets;
        widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
        widgets.inactive.bg_stroke = egui::Stroke::NONE;
        widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
        widgets.hovered.bg_fill = colors::BG_HOVER;
        widgets.hovered.bg_stroke = egui::Stroke::NONE;
        widgets.hovered.weak_bg_fill = colors::BG_HOVER;
        ui.menu_button(egui::RichText::new("モード").small(), |ui| {
            // ── Blend Mode Dropdown ──
            let bm_text = format!("{:?}", layer.blend_mode);
            egui::ComboBox::from_id_salt(format!("tl_blend_{}", i))
                .selected_text(format!("Blend: {}", bm_text))
                .show_ui(ui, |ui| {
                    for bm in [
                        BlendMode::Normal,
                        BlendMode::Multiply,
                        BlendMode::Screen,
                        BlendMode::Overlay,
                        BlendMode::Add,
                        BlendMode::Darken,
                        BlendMode::Lighten,
                        BlendMode::SoftLight,
                        BlendMode::HardLight,
                        BlendMode::Difference,
                        BlendMode::Exclusion,
                        BlendMode::Divide,
                        BlendMode::Subtract,
                    ] {
                        if ui.selectable_label(layer.blend_mode == bm, format!("{:?}", bm)).clicked() {
                            layer.blend_mode = bm;
                            *project_changed = true;
                        }
                    }
                });

            // ── Track Matte Dropdown ──
            let tm_text = match layer.track_matte {
                TrackMatteMode::None => "None",
                TrackMatteMode::AlphaMatte => "Alpha",
                TrackMatteMode::AlphaMatteInverted => "Alpha Inv",
                TrackMatteMode::LumaMatte => "Luma",
                TrackMatteMode::LumaMatteInverted => "Luma Inv",
            };
            egui::ComboBox::from_id_salt(format!("tl_matte_{}", i))
                .selected_text(format!("Matte: {}", tm_text))
                .show_ui(ui, |ui| {
                    for (mode, label) in [
                        (TrackMatteMode::None, "None"),
                        (TrackMatteMode::AlphaMatte, "Alpha Matte"),
                        (TrackMatteMode::AlphaMatteInverted, "Alpha Matte Inverted"),
                        (TrackMatteMode::LumaMatte, "Luma Matte"),
                        (TrackMatteMode::LumaMatteInverted, "Luma Matte Inverted"),
                    ] {
                        if ui.selectable_label(layer.track_matte == mode, label).clicked() {
                            layer.track_matte = mode;
                            *project_changed = true;
                        }
                    }
                });

            // ── Parenting Pick Whip @ & Dropdown ──
            let pw_active = *pick_whip_mode;
            let pw_btn = ui.selectable_label(pw_active, "@").on_hover_text("Parenting Pick Whip: Click to enter pick mode, then click target layer");
            if pw_btn.clicked() {
                *pick_whip_mode = !*pick_whip_mode;
                *pick_whip_target = None;
                if *pick_whip_mode {
                    toasts.info("Pick Whip active: click a layer to set as parent");
                }
            }
            let parent_name_resolved = layer.parent_id.as_deref()
                .and_then(|pid| derived.parent_indices.get(pid))
                .and_then(|index| parent_choices_ref.get(*index))
                .map(|(_, name)| name.as_str())
                .unwrap_or("None");
            let parent_prefix = if layer.is_3d { "Parent (3D)" } else { "Parent" };
            egui::ComboBox::from_id_salt(format!("tl_parent_{}", i))
                .selected_text(format!("{}: {}", parent_prefix, parent_name_resolved))
                .show_ui(ui, |ui| {
                    if ui.selectable_label(layer.parent_id.is_none(), "None").clicked() {
                        layer.parent_id = None;
                        *project_changed = true;
                    }
                    for (p_idx, (p_id, p_name)) in parent_choices_ref.iter().enumerate() {
                        if p_idx != i {
                            let is_p = layer.parent_id.as_deref() == Some(p_id);
                            if ui.selectable_label(is_p, p_name).clicked() {
                                let is_cycle = would_create_parent_cycle(
                                    &layer.id,
                                    p_id,
                                    &derived.parents,
                                );

                                if is_cycle {
                                    toasts.warning(format!("Cycle prevented! Cannot parent '{}' to '{}'", layer.name, p_name));
                                } else {
                                    layer.parent_id = Some(p_id.clone());
                                    *project_changed = true;
                                    toasts.info(format!("Parented '{}' ➔ '{}'", layer.name, p_name));
                                }
                            }
                        }
                    }
                });
        });
    });
}
