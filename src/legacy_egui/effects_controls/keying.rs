use crate::core::timeline::EffectType;
use crate::ui::inspector_property::draw_property_ui;
use eframe::egui;

use super::common::draw_prop;

pub fn draw(
    effect_type: &mut EffectType,
    ui: &mut egui::Ui,
    current_frame: u32,
    project_changed: &mut bool,
    next_frame: &mut Option<u32>,
) {
    match effect_type {
        EffectType::ChromaKey {
            screen_color,
            screen_gain,
            clip_black,
            clip_white,
        } => {
            let c_before = screen_color.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Key Color", screen_color, |ui, val| {
                    crate::ui::kui::color_rgb(ui, val);
                })
            {
                *next_frame = Some(nf);
            }
            if c_before != *screen_color {
                *project_changed = true;
            }
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Screen Gain",
                screen_gain,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.5..=2.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Clip Black",
                clip_black,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=1.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Clip White",
                clip_white,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=1.0, "");
                },
            );
        }
        EffectType::LumaKeyRange {
            low_threshold,
            high_threshold,
            invert,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Low Threshold",
                low_threshold,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=255.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "High Threshold",
                high_threshold,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=255.0, "");
                },
            );
            if crate::ui::kui::toggle(ui, invert, "Invert").changed() {
                *project_changed = true;
            }
        }
        EffectType::SimpleChoker { choke_amount } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Choke Amount",
                choke_amount,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, -100.0..=100.0, "");
                },
            );
        }
        EffectType::MatteChokeSpread { radius, expand } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Radius",
                radius,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 1.0..=64.0, " px");
                },
            );
            if crate::ui::kui::toggle(ui, expand, "Expand (spread instead of choke)").changed() {
                *project_changed = true;
            }
        }
        EffectType::AlphaFeather { radius } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Radius",
                radius,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 1.0..=64.0, " px");
                },
            );
        }
        EffectType::AlphaFromLuminance { invert } => {
            if crate::ui::kui::toggle(ui, invert, "Invert (dark = opaque)").changed() {
                *project_changed = true;
            }
        }
        EffectType::MatteChoker {
            choke_amount,
            gray_level,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Choke Amount",
                choke_amount,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=50.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Gray Level",
                gray_level,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=1.0, "");
                },
            );
        }
        EffectType::SetMatte {
            source_layer_idx,
            source_channel,
            invert_matte,
            composite_mode,
        } => {
            let mut idx_i32 = (*source_layer_idx).min(i32::MAX as usize) as i32;
            let grid = crate::ui::kui::PropertyGrid::new(
                ui,
                &["Source Layer Index", "Use For Matte", "Composite Mode"],
            );
            if grid
                .row(ui, "Source Layer Index", |ui| {
                    ui.add(crate::ui::kui::drag(&mut idx_i32).range(0..=99))
                })
                .changed()
            {
                *source_layer_idx = idx_i32.max(0) as usize;
                *project_changed = true;
            }
            let source_channels = [
                (crate::core::set_matte::MatteSourceChannel::Alpha, "Alpha"),
                (
                    crate::core::set_matte::MatteSourceChannel::Luminance,
                    "Luminance",
                ),
                (crate::core::set_matte::MatteSourceChannel::Red, "Red"),
                (crate::core::set_matte::MatteSourceChannel::Green, "Green"),
                (crate::core::set_matte::MatteSourceChannel::Blue, "Blue"),
                (
                    crate::core::set_matte::MatteSourceChannel::Lightness,
                    "Lightness",
                ),
            ];
            *project_changed |= grid.row(ui, "Use For Matte", |ui| {
                crate::ui::kui::combo(ui, "set_matte_channel", source_channel, &source_channels)
            });
            if crate::ui::kui::toggle(ui, invert_matte, "Invert Matte").changed() {
                *project_changed = true;
            }
            let composite_modes = [
                (
                    crate::core::set_matte::MatteCompositeMode::Replace,
                    "Replace",
                ),
                (
                    crate::core::set_matte::MatteCompositeMode::Intersect,
                    "Intersect",
                ),
                (crate::core::set_matte::MatteCompositeMode::Add, "Add"),
                (
                    crate::core::set_matte::MatteCompositeMode::Subtract,
                    "Subtract",
                ),
            ];
            *project_changed |= grid.row(ui, "Composite Mode", |ui| {
                crate::ui::kui::combo(ui, "set_matte_mode", composite_mode, &composite_modes)
            });
        }
        EffectType::LinearColorKey {
            key_color,
            match_mode,
            tolerance,
            softness,
        } => {
            let kc_before = key_color.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Key Color", key_color, |ui, val| {
                    crate::ui::kui::color_rgb(ui, val);
                })
            {
                *next_frame = Some(nf);
            }
            if kc_before != *key_color {
                *project_changed = true;
            }

            let match_modes = [
                (
                    crate::core::linear_color_key::ColorMatchMode::UsingRGB,
                    "RGB distance",
                ),
                (
                    crate::core::linear_color_key::ColorMatchMode::UsingHue,
                    "Hue",
                ),
                (
                    crate::core::linear_color_key::ColorMatchMode::UsingChroma,
                    "Chroma",
                ),
            ];
            let grid = crate::ui::kui::PropertyGrid::new(ui, &["Match Colors"]);
            *project_changed |= grid.row(ui, "Match Colors", |ui| {
                crate::ui::kui::combo(ui, "key_match_mode", match_mode, &match_modes)
            });
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Tolerance",
                tolerance,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=100.0, "%");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Softness",
                softness,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=100.0, "%");
                },
            );
        }
        _ => {}
    }
}
