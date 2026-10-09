use crate::core::timeline::{Layer, LayerType};
use crate::ui::inspector_property::draw_property_ui;
use crate::ui::kui;
use eframe::egui;

pub(crate) fn draw_media_specs(
    ui: &mut egui::Ui,
    layer: &mut Layer,
    current_frame: u32,
    project_changed: &mut bool,
    next_frame: &mut Option<u32>,
    comp_fps: f32,
) -> bool {
    match &mut layer.layer_type {
                LayerType::Solid { color } => {
                    let val_before = *color;
                    kui::property(ui, "Color", |ui| kui::color_rgba(ui, color));
                    if val_before != *color {
                        *project_changed = true;
                    }
                }
                LayerType::Model3D { path } => {
                    let before = path.clone();
                    ui.label(egui::RichText::new("3D Model Layer").strong());
                    kui::property(ui, "OBJ", |ui| kui::text_field(ui, path));
                    if !path.is_empty()
                        && ui
                            .small("Reveal File")
                            .on_hover_text("Open the OBJ source file location")
                            .clicked()
                    {
                        crate::ui::project_io::reveal_in_file_manager(std::path::Path::new(
                            path.as_str(),
                        ));
                    }
                    if before != *path {
                        *project_changed = true;
                    }
                    ui.label(
                        egui::RichText::new(
                            "OBJ models use the composition camera and 3D transform controls.",
                        )
                        .small(),
                    );
                }
                LayerType::Image { path } => {
                    let val_before = path.clone();
                    kui::property(ui, "Source", |ui| kui::text_field(ui, path));
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
                    if val_before != *path {
                        *project_changed = true;
                    }

                    // Puppet Mesh & Pins Tool
                    ui.collapsing("Puppet Tool & Mesh Warp", |ui| {
                        if kui::button(ui, "+ Position Pin", kui::ButtonKind::Default)
                            .on_hover_text("Adds a deformation pin at the layer position")
                            .clicked()
                        {
                            let mut index = 1;
                            let id = loop {
                                let candidate = format!("pin_{index}");
                                if !layer.puppet_pins.iter().any(|pin| pin.id == candidate) {
                                    break candidate;
                                }
                                index += 1;
                            };
                            let position = layer.transform.position.evaluate(current_frame);
                            layer
                                .puppet_pins
                                .push(crate::core::timeline::PuppetPin::new(
                                    id,
                                    format!("Pin {index}"),
                                    position,
                                ));
                            *project_changed = true;
                        }

                        let mut mesh_density = ui.ctx().data(|d| {
                            d.get_temp::<u32>(egui::Id::new("puppet_mesh_density"))
                                .unwrap_or(50)
                        });
                        let mut mesh_exp = ui.ctx().data(|d| {
                            d.get_temp::<f32>(egui::Id::new("puppet_mesh_exp"))
                                .unwrap_or(5.0)
                        });
                        if kui::property(ui, "Mesh Density", |ui| {
                            kui::slider_integer(ui, &mut mesh_density, 10..=100)
                        })
                        .changed()
                        {
                            ui.ctx().data_mut(|d| {
                                d.insert_temp(egui::Id::new("puppet_mesh_density"), mesh_density)
                            });
                            *project_changed = true;
                        }
                        if kui::property(ui, "Expansion", |ui| {
                            kui::slider(ui, &mut mesh_exp, 0.0..=50.0, " px")
                        })
                        .changed()
                        {
                            ui.ctx().data_mut(|d| {
                                d.insert_temp(egui::Id::new("puppet_mesh_exp"), mesh_exp)
                            });
                            *project_changed = true;
                        }
                    });
                }
                LayerType::Video {
                    source,
                    frames_dir,
                    frame_count,
                    audio_wav,
                    speed,
                } => {
                    let before_src = source.clone();
                    let before_frames = frames_dir.clone();
                    let before_speed = *speed;
                    let before_count = *frame_count;
                    let before_frame_blending = layer.frame_blending;
                    ui.label(egui::RichText::new("Video Layer").strong());
                    kui::property(ui, "Source", |ui| kui::text_field(ui, source));
                    if !source.is_empty()
                        && ui
                            .small("Reveal File")
                            .on_hover_text("Open the source file location in file manager")
                            .clicked()
                    {
                        crate::ui::project_io::reveal_in_file_manager(std::path::Path::new(
                            source.as_str(),
                        ));
                    }
                    kui::property(ui, "Frames dir", |ui| kui::text_field(ui, frames_dir));
                    // Playback speed: 1.0 = realtime, 0.5 = half, 2.0 = double
                    kui::property(ui, "Speed", |ui| {
                        let response =
                            ui.add(kui::drag(speed).range(0.05..=10.0).speed(0.05).unit("x"));
                        if kui::button(ui, "1x", kui::ButtonKind::Quiet)
                            .on_hover_text("Reset to realtime")
                            .clicked()
                        {
                            *speed = 1.0;
                        }
                        response
                    });
                    kui::property(ui, "Frame Blending", |ui| {
                        kui::toggle(ui, &mut layer.frame_blending, "")
                    })
                    .on_hover_text(
                        "Blend adjacent source frames for fractional speed and time-remap playback",
                    );
                    kui::property(ui, "Frame count", |ui| {
                        ui.add(kui::drag(frame_count).range(1..=100_000))
                    });
                    ui.label(format!(
                        "Effective duration: {:.1}s at {:.2}x | audio: {}",
                        *frame_count as f32 / comp_fps.max(1.0) / speed.max(0.01),
                        speed,
                        if audio_wav.is_some() { "yes" } else { "no" }
                    ));

                    // Time remap toggle & parameter editing
                    {
                        let remap_enabled = layer.time_remap.is_some();
                        let mut new_enabled = remap_enabled;
                        if kui::property(ui, "Time Remap", |ui| {
                            kui::toggle(ui, &mut new_enabled, "")
                        })
                        .changed()
                        {
                            if new_enabled && !remap_enabled {
                                let in_f = layer.in_frame;
                                let out_f = layer.out_frame;
                                let linear = crate::core::keyframe::InterpolationType::Linear;
                                layer.time_remap = Some(crate::core::property::Animatable::Animated(vec![
                                    crate::core::keyframe::Keyframe::new(in_f, in_f as f32, linear),
                                    crate::core::keyframe::Keyframe::new(out_f, out_f as f32, linear),
                                ]));
                                *project_changed = true;
                            } else if !new_enabled && remap_enabled {
                                layer.time_remap = None;
                                *project_changed = true;
                            }
                        }
                    }
                    if let Some(remap) = &mut layer.time_remap {
                        let remap_before = remap.clone();
                        if let Some(nf) = draw_property_ui(
                            current_frame,
                            ui,
                            "Remapped Frame",
                            remap,
                            |ui, val| {
                                ui.add(kui::drag(val).range(0.0..=100_000.0).speed(1.0).unit("f"));
                            },
                        ) {
                            *next_frame = Some(nf);
                        }
                        crate::ui::inspector_property::draw_easy_ease_button(ui, remap, project_changed);
                        if remap_before != *remap {
                            *project_changed = true;
                        }
                    }
                    if before_src != *source
                        || before_frames != *frames_dir
                        || before_speed != *speed
                        || before_count != *frame_count
                        || before_frame_blending != layer.frame_blending
                    {
                        *project_changed = true;
                    }
                }
        _ => return false,
    }
    true
}
