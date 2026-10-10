//! Shadow and glow styling UI (Drop Shadow, Inner Shadow, Outer Glow, Inner Glow).

use crate::core::timeline::{BlendMode, LayerStyle};
use crate::ui::theme::colors;
use eframe::egui;

use super::common::color_row;

pub fn draw_shadow_and_glow(ui: &mut egui::Ui, style: &mut LayerStyle, changed: &mut bool) {
    // ── Drop Shadow ──
    ui.collapsing("Drop Shadow", |ui| {
        if ui
            .checkbox(&mut style.drop_shadow.enabled, "Enabled")
            .changed()
        {
            *changed = true;
        }

        let mut blend_idx = match style.drop_shadow.blend_mode {
            BlendMode::Normal => 0,
            BlendMode::Multiply => 1,
            BlendMode::Screen => 2,
            BlendMode::Overlay => 3,
            BlendMode::Add => 4,
            _ => 0,
        };
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Blend Mode")
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            egui::ComboBox::from_id_salt("ds_blend")
                .selected_text(format!("{:?}", style.drop_shadow.blend_mode))
                .show_ui(ui, |ui| {
                    if ui.selectable_value(&mut blend_idx, 0, "Normal").clicked() {
                        *changed = true;
                    }
                    if ui.selectable_value(&mut blend_idx, 1, "Multiply").clicked() {
                        *changed = true;
                    }
                    if ui.selectable_value(&mut blend_idx, 2, "Screen").clicked() {
                        *changed = true;
                    }
                    if ui.selectable_value(&mut blend_idx, 3, "Overlay").clicked() {
                        *changed = true;
                    }
                    if ui.selectable_value(&mut blend_idx, 4, "Add").clicked() {
                        *changed = true;
                    }
                });
        });
        if *changed {
            style.drop_shadow.blend_mode = match blend_idx {
                1 => BlendMode::Multiply,
                2 => BlendMode::Screen,
                3 => BlendMode::Overlay,
                4 => BlendMode::Add,
                _ => BlendMode::Normal,
            };
        }

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Opacity")
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            if ui
                .add(egui::Slider::new(&mut style.drop_shadow.opacity, 0.0..=100.0).suffix("%"))
                .changed()
            {
                *changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Angle")
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            if ui
                .add(
                    egui::Slider::new(&mut style.drop_shadow.angle, -180.0..=180.0).suffix("°"),
                )
                .changed()
            {
                *changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Distance")
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            if ui
                .add(
                    egui::Slider::new(&mut style.drop_shadow.distance, 0.0..=200.0)
                        .suffix(" px"),
                )
                .changed()
            {
                *changed = true;
            }
        });

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Size")
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            if ui
                .add(egui::Slider::new(&mut style.drop_shadow.size, 0.0..=200.0).suffix(" px"))
                .changed()
            {
                *changed = true;
            }
        });

        color_row(ui, "Color", &mut style.drop_shadow.color, changed);
    });

    // ── Inner Shadow ──
    ui.collapsing("Inner Shadow", |ui| {
        if ui
            .checkbox(&mut style.inner_shadow.enabled, "Enabled")
            .changed()
        {
            *changed = true;
        }
        let s = &mut style.inner_shadow;
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Opacity")
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            if ui
                .add(egui::Slider::new(&mut s.opacity, 0.0..=100.0).suffix("%"))
                .changed()
            {
                *changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Angle")
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            if ui
                .add(egui::Slider::new(&mut s.angle, -180.0..=180.0).suffix("°"))
                .changed()
            {
                *changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Distance")
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            if ui
                .add(egui::Slider::new(&mut s.distance, 0.0..=250.0))
                .changed()
            {
                *changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Size")
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            if ui.add(egui::Slider::new(&mut s.size, 0.0..=64.0)).changed() {
                *changed = true;
            }
        });
        color_row(ui, "Color", &mut s.color, changed);
    });

    // ── Outer Glow ──
    ui.collapsing("Outer Glow", |ui| {
        if ui
            .checkbox(&mut style.outer_glow.enabled, "Enabled")
            .changed()
        {
            *changed = true;
        }
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Opacity")
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            if ui
                .add(egui::Slider::new(&mut style.outer_glow.opacity, 0.0..=100.0).suffix("%"))
                .changed()
            {
                *changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Spread")
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            if ui
                .add(egui::Slider::new(&mut style.outer_glow.spread, 0.0..=100.0).suffix("%"))
                .changed()
            {
                *changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Size")
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            if ui
                .add(egui::Slider::new(&mut style.outer_glow.size, 0.0..=200.0).suffix(" px"))
                .changed()
            {
                *changed = true;
            }
        });
        color_row(ui, "Color", &mut style.outer_glow.color, changed);
    });

    // ── Inner Glow ──
    ui.collapsing("Inner Glow", |ui| {
        if ui
            .checkbox(&mut style.inner_glow.enabled, "Enabled")
            .changed()
        {
            *changed = true;
        }
        let ig = &mut style.inner_glow;
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Opacity")
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            if ui
                .add(egui::Slider::new(&mut ig.opacity, 0.0..=100.0).suffix("%"))
                .changed()
            {
                *changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Size")
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            if ui
                .add(egui::Slider::new(&mut ig.size, 0.0..=64.0))
                .changed()
            {
                *changed = true;
            }
        });
        color_row(ui, "Color", &mut ig.color, changed);
    });
}
