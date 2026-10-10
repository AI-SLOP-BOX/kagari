use crate::core::timeline::EffectType;
use crate::ui::inspector_property::draw_property_ui;
use eframe::egui;

use super::super::common::draw_prop;

pub fn draw_advanced_color(
    effect_type: &mut EffectType,
    ui: &mut egui::Ui,
    current_frame: u32,
    project_changed: &mut bool,
    next_frame: &mut Option<u32>,
) -> bool {
    match effect_type {
        EffectType::Colorama {
            preset_index,
            cycle_phase,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Preset",
                preset_index,
                |ui, v| {
                    let options = [
                        (0.0_f32, "Rainbow"),
                        (1.0, "Heatmap"),
                        (2.0, "Sepia"),
                        (3.0, "Solarize"),
                    ];
                    let mut selected = v.round().rem_euclid(4.0);
                    if crate::ui::kui::combo(ui, "colorama_preset", &mut selected, &options) {
                        *v = selected;
                    }
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Cycle Phase",
                cycle_phase,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=360.0, "°");
                },
            );
            true
        }
        EffectType::Curves { channel } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Channel",
                channel,
                |ui, v| {
                    let options = [
                        (0.0_f32, "Master"),
                        (1.0, "Red"),
                        (2.0, "Green"),
                        (3.0, "Blue"),
                    ];
                    let mut selected = v.round().clamp(0.0, 3.0);
                    if crate::ui::kui::combo(ui, "curves_channel", &mut selected, &options) {
                        *v = selected;
                    }
                },
            );
            crate::ui::kui::hint(ui, "Use S-curve preset (5-point Catmull–Rom)");
            true
        }
        EffectType::ShiftChannels {
            take_red,
            take_green,
            take_blue,
            take_alpha,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Take Red (0=R,1=G,2=B,3=A,4=Off,5=On)",
                take_red,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=5.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Take Green",
                take_green,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=5.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Take Blue",
                take_blue,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=5.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Take Alpha",
                take_alpha,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=5.0, "");
                },
            );
            true
        }
        EffectType::ColorBalance {
            shadows,
            midtones,
            highlights,
            preserve_luminosity,
        } => {
            let grid = crate::ui::kui::PropertyGrid::new(
                ui,
                &["Shadows", "Midtones", "Highlights", "Preserve Luminosity"],
            );
            for (label, band) in [
                ("Shadows", shadows),
                ("Midtones", midtones),
                ("Highlights", highlights),
            ] {
                *project_changed |= grid
                    .row(ui, label, |ui| {
                        crate::ui::kui::vector_axes(
                            ui,
                            band,
                            -100.0..=100.0,
                            1.0,
                            "",
                            ["R", "G", "B"],
                        )
                    })
                    .changed();
            }
            *project_changed |= grid
                .row(ui, "Preserve Luminosity", |ui| {
                    crate::ui::kui::switch(ui, preserve_luminosity, "")
                })
                .changed();
            true
        }
        EffectType::ChannelMixer { matrix, monochrome } => {
            crate::ui::kui::hint(ui, "Output channel values from input RGB (%)");
            let grid = crate::ui::kui::PropertyGrid::new(
                ui,
                &["Red Output", "Green Output", "Blue Output"],
            );
            for (label, row) in ["Red Output", "Green Output", "Blue Output"]
                .into_iter()
                .zip(matrix.iter_mut())
            {
                *project_changed |= grid
                    .row(ui, label, |ui| {
                        crate::ui::kui::vector_axes(
                            ui,
                            row,
                            -200.0..=200.0,
                            1.0,
                            "%",
                            ["R", "G", "B"],
                        )
                    })
                    .changed();
            }
            if crate::ui::kui::toggle(ui, monochrome, "Monochrome").changed() {
                *project_changed = true;
            }
            true
        }
        EffectType::Tritone {
            shadow_color,
            mid_color,
            highlight_color,
        } => {
            let sc_before = shadow_color.clone();
            if let Some(nf) = draw_property_ui(
                current_frame,
                ui,
                "Shadow Color",
                shadow_color,
                |ui, val| {
                    crate::ui::kui::color_rgb(ui, val);
                },
            ) {
                *next_frame = Some(nf);
            }
            if sc_before != *shadow_color {
                *project_changed = true;
            }

            let mc_before = mid_color.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Mid Color", mid_color, |ui, val| {
                    crate::ui::kui::color_rgb(ui, val);
                })
            {
                *next_frame = Some(nf);
            }
            if mc_before != *mid_color {
                *project_changed = true;
            }

            let hc_before = highlight_color.clone();
            if let Some(nf) = draw_property_ui(
                current_frame,
                ui,
                "Highlight Color",
                highlight_color,
                |ui, val| {
                    crate::ui::kui::color_rgb(ui, val);
                },
            ) {
                *next_frame = Some(nf);
            }
            if hc_before != *highlight_color {
                *project_changed = true;
            }
            true
        }
        EffectType::Vibrance { amount } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Amount",
                amount,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, -100.0..=100.0, "");
                },
            );
            true
        }
        EffectType::WhiteBalance { temperature, tint } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Temperature",
                temperature,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, -100.0..=100.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Tint",
                tint,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, -100.0..=100.0, "");
                },
            );
            true
        }
        EffectType::HslAdjust {
            hue_deg,
            saturation,
            lightness,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Hue",
                hue_deg,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, -180.0..=180.0, "°");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Saturation",
                saturation,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, -100.0..=100.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Lightness",
                lightness,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, -100.0..=100.0, "");
                },
            );
            true
        }
        EffectType::FilmEmulation {
            lift,
            gamma,
            gain,
            hue_shift_deg,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Lift",
                lift,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, -0.5..=0.5, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Gamma",
                gamma,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.1..=3.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Gain",
                gain,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=3.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Hue Shift",
                hue_shift_deg,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, -180.0..=180.0, "°");
                },
            );
            true
        }
        EffectType::GradientMap {
            low_color,
            mid_color,
            high_color,
        } => {
            let lc_before = low_color.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Shadow Color", low_color, |ui, val| {
                    crate::ui::kui::color_rgb(ui, val);
                })
            {
                *next_frame = Some(nf);
            }
            if lc_before != *low_color {
                *project_changed = true;
            }

            let mc_before = mid_color.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Mid Color", mid_color, |ui, val| {
                    crate::ui::kui::color_rgb(ui, val);
                })
            {
                *next_frame = Some(nf);
            }
            if mc_before != *mid_color {
                *project_changed = true;
            }

            let hc_before = high_color.clone();
            if let Some(nf) = draw_property_ui(
                current_frame,
                ui,
                "Highlight Color",
                high_color,
                |ui, val| {
                    crate::ui::kui::color_rgb(ui, val);
                },
            ) {
                *next_frame = Some(nf);
            }
            if hc_before != *high_color {
                *project_changed = true;
            }
            true
        }
        _ => false,
    }
}
