use crate::core::timeline::{Effect, EffectType};
use crate::KagariApp;
use eframe::egui;
use super::effects_io::*;
use super::widgets::*;

pub fn draw_workspace_controls(app: &mut KagariApp, ui: &mut egui::Ui) {
    let Some(layer_idx) = app.selection.selected_layer_idx else {
        crate::ui::kui::empty_state(ui, "Select a layer to color grade.", 120.0);
        return;
    };
    let comp = app.history.current().active_composition();
    if comp.layers.get(layer_idx).is_none() {
        crate::ui::kui::empty_state(ui, "The selected layer is unavailable.", 96.0);
        return;
    }

    crate::ui::kui::collapsing_section(
        ui,
        "color-basic-correction",
        "Basic Correction",
        true,
        |ui| {
            let mut wb = read_wb(app).unwrap_or((0.0, 0.0));
            let mut wb_changed = false;
            let grid = crate::ui::kui::PropertyGrid::new(ui, &["Temperature", "Tint"]);
            wb_changed |= grid
                .row(ui, "Temperature", |ui| {
                    crate::ui::kui::parameter_slider(ui, &mut wb.0, -100.0..=100.0, "")
                })
                .changed();
            wb_changed |= grid
                .row(ui, "Tint", |ui| {
                    crate::ui::kui::parameter_slider(ui, &mut wb.1, -100.0..=100.0, "")
                })
                .changed();
            if wb_changed {
                write_wb(app, wb.0, wb.1);
            }

            let mut hsl = read_hsl(app).unwrap_or((0.0, 0.0, 0.0));
            let mut hsl_changed = false;
            let grid = crate::ui::kui::PropertyGrid::new(ui, &["Hue", "Saturation", "Lightness"]);
            hsl_changed |= grid
                .row(ui, "Hue", |ui| {
                    crate::ui::kui::parameter_slider(ui, &mut hsl.0, -180.0..=180.0, "°")
                })
                .changed();
            hsl_changed |= grid
                .row(ui, "Saturation", |ui| {
                    crate::ui::kui::parameter_slider(ui, &mut hsl.1, -100.0..=100.0, "%")
                })
                .changed();
            hsl_changed |= grid
                .row(ui, "Lightness", |ui| {
                    crate::ui::kui::parameter_slider(ui, &mut hsl.2, -100.0..=100.0, "%")
                })
                .changed();
            if hsl_changed {
                write_hsl(app, hsl.0, hsl.1, hsl.2);
            }

            let mut vibrance = read_single_f32(app, VIB_EFFECT, |kind, frame| match kind {
                EffectType::Vibrance { amount } => Some(amount.evaluate(frame)),
                _ => None,
            })
            .unwrap_or(0.0);
            if crate::ui::kui::PropertyGrid::new(ui, &["Vibrance"])
                .row(ui, "Vibrance", |ui| {
                    crate::ui::kui::parameter_slider(ui, &mut vibrance, -100.0..=100.0, "%")
                })
                .changed()
            {
                write_single_f32(app, VIB_EFFECT, "lumetri_vib", vibrance, |amount| {
                    EffectType::Vibrance { amount }
                });
            }
        },
    );

    crate::ui::kui::collapsing_section(ui, "color-curves", "Curves", false, |ui| {
        let value = app.history.current().active_composition().layers[layer_idx]
            .effects
            .iter()
            .find(|effect| effect.name == "Lumetri Master Curve")
            .and_then(|effect| match &effect.effect_type {
                EffectType::Curves { channel } => {
                    Some(channel.evaluate(app.playback.current_frame))
                }
                _ => None,
            })
            .unwrap_or(0.0);
        let mut value = value;
        if crate::ui::kui::PropertyGrid::new(ui, &["Master"])
            .row(ui, "Master", |ui| {
                crate::ui::kui::parameter_slider(ui, &mut value, -100.0..=100.0, "")
            })
            .changed()
        {
            app.modify_project(move |project| {
                let comp = project.active_composition_mut();
                let Some(layer) = comp.layers.get_mut(layer_idx) else {
                    return;
                };
                let value = crate::core::property::Animatable::new_constant(value);
                if let Some(effect) = layer
                    .effects
                    .iter_mut()
                    .find(|effect| effect.name == "Lumetri Master Curve")
                {
                    if let EffectType::Curves { channel } = &mut effect.effect_type {
                        *channel = value;
                        effect.enabled = true;
                    }
                } else {
                    layer.effects.push(Effect {
                        id: format!("lumetri_curve_{}", layer.effects.len()),
                        name: "Lumetri Master Curve".into(),
                        effect_type: EffectType::Curves { channel: value },
                        enabled: true,
                    });
                }
            });
        }
    });

    crate::ui::kui::collapsing_section(ui, "color-wheels", "Color Wheels", false, |ui| {
        let mut balance = read_cb(app).unwrap_or(([0.0; 3], [0.0; 3], [0.0; 3], true));
        let mut changed = false;
        ui.horizontal_wrapped(|ui| {
            changed |= wheel_widget(ui, "Shadows", &mut balance.0);
            changed |= wheel_widget(ui, "Midtones", &mut balance.1);
            changed |= wheel_widget(ui, "Highlights", &mut balance.2);
        });
        if changed {
            write_cb(app, balance.0, balance.1, balance.2, balance.3);
        }
    });

    crate::ui::kui::collapsing_section(ui, "color-looks", "Looks", false, |ui| {
        draw_look_presets(app, ui);
    });
}

