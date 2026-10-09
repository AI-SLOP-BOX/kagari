use crate::core::timeline::{Layer, LayerType};
use crate::ui::inspector_property::draw_property_ui;
use crate::ui::kui;
use crate::ui::theme::colors;
use eframe::egui;

pub(crate) fn draw_other_specs(
    ui: &mut egui::Ui,
    layer: &mut Layer,
    current_frame: u32,
    project_changed: &mut bool,
    next_frame: &mut Option<u32>,
) {
    match &mut layer.layer_type {
                LayerType::Null => {
                    ui.label("Null Object (Controller)");
                }
                LayerType::PreComp { comp_id } => {
                    ui.label(format!("Pre-Composition ({})", comp_id));
                }
                LayerType::Audio { path, volume } => {
                    ui.label(format!("Audio File ({})", path));
                    if !path.is_empty()
                        && ui
                            .small("Reveal File")
                            .on_hover_text("Open the source file location in file manager")
                            .clicked()
                    {
                        crate::ui::project_io::reveal_in_file_manager(std::path::Path::new(
                            path.as_str(),
                        ));
                    }
                    let v_before = volume.clone();
                    if let Some(nf) =
                        draw_property_ui(current_frame, ui, "  Volume", volume, |ui, val| {
                            kui::slider(ui, val, -48.0..=12.0, " dB");
                        })
                    {
                        *next_frame = Some(nf);
                    }
                    if v_before != *volume {
                        *project_changed = true;
                    }
                }
                LayerType::AdjustmentLayer => {
                    ui.label("⚙ Adjustment Layer (Applies effects to lower composite)");
                }
                LayerType::Particle { emitter } => {
                    ui.label("✦ Particle Emitter");
                    let mut changed = false;
                    changed |= kui::property(ui, "Rate", |ui| {
                        ui.add(kui::drag(&mut emitter.rate).unit(" /s"))
                    })
                    .changed();
                    changed |= kui::property(ui, "Lifetime", |ui| {
                        ui.add(kui::drag(&mut emitter.lifetime).unit(" s"))
                    })
                    .changed();
                    changed |= kui::property(ui, "Speed", |ui| {
                        kui::slider(ui, &mut emitter.speed, 0.0..=2000.0, "")
                    })
                    .changed();
                    changed |= kui::property(ui, "Spread", |ui| {
                        kui::slider(ui, &mut emitter.spread_degrees, 0.0..=360.0, "°")
                    })
                    .changed();
                    changed |= kui::property(ui, "Gravity", |ui| {
                        ui.add(kui::drag(&mut emitter.gravity[1]))
                    })
                    .changed();
                    changed |= kui::property(ui, "Turbulence", |ui| {
                        kui::slider(ui, &mut emitter.turbulence, 0.0..=1000.0, "")
                    })
                    .changed();
                    ui.collapsing("Forces", |ui| {
                        changed |= kui::property(ui, "Vortex (+CW)", |ui| {
                            kui::slider(ui, &mut emitter.vortex_strength, -2000.0..=2000.0, "")
                        })
                        .changed();
                        changed |= kui::property(ui, "Vortex Center", |ui| {
                            kui::vector_axes_unbounded(
                                ui,
                                &mut emitter.vortex_center,
                                1.0,
                                "",
                                ["X", "Y"],
                            )
                        })
                        .changed();
                        changed |= kui::property(ui, "Attract (+) / Repel (−)", |ui| {
                            kui::slider(ui, &mut emitter.attract_strength, -2000.0..=2000.0, "")
                        })
                        .changed();
                        changed |= kui::property(ui, "Force Point", |ui| {
                            kui::vector_axes_unbounded(
                                ui,
                                &mut emitter.attract_center,
                                1.0,
                                "",
                                ["X", "Y"],
                            )
                        })
                        .changed();
                        ui.separator();
                        let mut ds = emitter.death_spawn_count as i32;
                        if kui::property(ui, "Death Spawn", |ui| {
                            ui.add(kui::drag(&mut ds).range(0..=32).prefix("×"))
                        })
                        .changed()
                        {
                            emitter.death_spawn_count = ds.max(0) as u32;
                            changed = true;
                        }
                        if emitter.death_spawn_count > 0 {
                            changed |= kui::property(ui, "Child Speed ×", |ui| {
                                kui::slider(
                                    ui,
                                    &mut emitter.death_spawn_speed_scale,
                                    0.05..=2.0,
                                    "",
                                )
                            })
                            .changed();
                            changed |= kui::property(ui, "Child Life ×", |ui| {
                                kui::slider(ui, &mut emitter.death_spawn_life_scale, 0.05..=1.0, "")
                            })
                            .changed();
                        }
                    });
                    ui.collapsing("Trail", |ui| {
                        let mut tl = emitter.trail_length as u32;
                        if kui::property(ui, "Length", |ui| kui::slider_integer(ui, &mut tl, 0..=8))
                            .changed()
                        {
                            emitter.trail_length = tl as u8;
                            changed = true;
                        }
                        changed |= kui::property(ui, "Taper", |ui| {
                            kui::slider(ui, &mut emitter.trail_taper, 0.1..=1.0, "")
                        })
                        .changed();
                    });
                    ui.collapsing("3D Depth", |ui| {
                        changed |=
                            kui::toggle(ui, &mut emitter.depth_enabled, "Project through camera")
                                .changed();
                        if emitter.depth_enabled {
                            changed |= kui::property(ui, "Z Range", |ui| {
                                kui::vector_axes_unbounded(
                                    ui,
                                    &mut emitter.depth_range,
                                    1.0,
                                    "",
                                    ["Min", "Max"],
                                )
                            })
                            .changed();
                            ui.label(
                                egui::RichText::new(
                                    "Particles scale with distance from the active camera",
                                )
                                .small()
                                .color(colors::TEXT_MUTED),
                            );
                        }
                    });
                    changed |= kui::property(ui, "Start Color", |ui| {
                        kui::color_rgba(ui, &mut emitter.color_start)
                    })
                    .changed();
                    changed |= kui::property(ui, "End Color", |ui| {
                        kui::color_rgba(ui, &mut emitter.color_end)
                    })
                    .changed();
                    if changed {
                        *project_changed = true;
                    }
                }
        _ => {}
    }
}
