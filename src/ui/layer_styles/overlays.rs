//! Overlays and bevel styling UI (Satin, Bevel/Emboss, Stroke, Gradient Overlay, Color Overlay).

use crate::core::timeline::LayerStyle;
use crate::ui::theme::colors;
use eframe::egui;

use super::common::color_row;

pub fn draw_overlays_and_bevel(ui: &mut egui::Ui, style: &mut LayerStyle, changed: &mut bool) {
    // ── Satin ──
    ui.collapsing("Satin", |ui| {
        if ui.checkbox(&mut style.satin.enabled, "Enabled").changed() {
            *changed = true;
        }
        let s = &mut style.satin;
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
                .add(egui::Slider::new(&mut s.distance, 0.0..=150.0))
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
            if ui.add(egui::Slider::new(&mut s.size, 1.0..=64.0)).changed() {
                *changed = true;
            }
        });
        color_row(ui, "Color", &mut s.color, changed);
    });

    // ── Bevel / Emboss ──
    ui.collapsing("Bevel / Emboss", |ui| {
        if ui
            .checkbox(&mut style.bevel_emboss.enabled, "Enabled")
            .changed()
        {
            *changed = true;
        }
        let bv = &mut style.bevel_emboss;
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Angle")
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            if ui
                .add(egui::Slider::new(&mut bv.angle, -180.0..=180.0).suffix("°"))
                .changed()
            {
                *changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Depth")
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            if ui
                .add(egui::Slider::new(&mut bv.depth, 1.0..=20.0))
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
                .add(egui::Slider::new(&mut bv.size, 0.0..=64.0))
                .changed()
            {
                *changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Highlight")
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            if ui
                .add(egui::Slider::new(&mut bv.highlight, 0.0..=100.0).suffix("%"))
                .changed()
            {
                *changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Shadow")
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            if ui
                .add(egui::Slider::new(&mut bv.shadow, 0.0..=100.0).suffix("%"))
                .changed()
            {
                *changed = true;
            }
        });
        color_row(ui, "Light", &mut bv.color_light, changed);
        color_row(ui, "Dark", &mut bv.color_dark, changed);
    });

    // ── Stroke ──
    ui.collapsing("Stroke", |ui| {
        if ui.checkbox(&mut style.stroke.enabled, "Enabled").changed() {
            *changed = true;
        }
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Size")
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            if ui
                .add(egui::Slider::new(&mut style.stroke.size, 1.0..=250.0).suffix(" px"))
                .changed()
            {
                *changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Position")
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            egui::ComboBox::from_id_salt("stroke_pos")
                .selected_text(match style.stroke.position {
                    0 => "Outside",
                    1 => "Inside",
                    2 => "Center",
                    _ => "Outside",
                })
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_value(&mut style.stroke.position, 0, "Outside")
                        .clicked()
                    {
                        *changed = true;
                    }
                    if ui
                        .selectable_value(&mut style.stroke.position, 1, "Inside")
                        .clicked()
                    {
                        *changed = true;
                    }
                    if ui
                        .selectable_value(&mut style.stroke.position, 2, "Center")
                        .clicked()
                    {
                        *changed = true;
                    }
                });
        });
        color_row(ui, "Color", &mut style.stroke.color, changed);
    });

    // ── Gradient Overlay ──
    ui.collapsing("Gradient Overlay", |ui| {
        if ui
            .checkbox(&mut style.gradient_overlay.enabled, "Enabled")
            .changed()
        {
            *changed = true;
        }
        let go = &mut style.gradient_overlay;
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Opacity")
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            if ui
                .add(egui::Slider::new(&mut go.opacity, 0.0..=100.0).suffix("%"))
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
                .add(egui::Slider::new(&mut go.angle, -180.0..=180.0).suffix("°"))
                .changed()
            {
                *changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Scale")
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            if ui
                .add(egui::Slider::new(&mut go.scale, 10.0..=400.0).suffix("%"))
                .changed()
            {
                *changed = true;
            }
        });
        color_row(ui, "Start", &mut go.color_start, changed);
        color_row(ui, "End", &mut go.color_end, changed);
    });

    // ── Color Overlay ──
    ui.collapsing("Color Overlay", |ui| {
        if ui
            .checkbox(&mut style.color_overlay.enabled, "Enabled")
            .changed()
        {
            *changed = true;
        }
        let co = &mut style.color_overlay;
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Opacity")
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            if ui
                .add(egui::Slider::new(&mut co.opacity, 0.0..=100.0).suffix("%"))
                .changed()
            {
                *changed = true;
            }
        });
        color_row(ui, "Color", &mut co.color, changed);
    });
}
