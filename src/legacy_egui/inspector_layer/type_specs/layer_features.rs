use crate::core::timeline::Layer;
use crate::ui::inspector_property::draw_property_ui;
use crate::ui::kui;
use crate::ui::theme::colors;
use eframe::egui;

pub(crate) fn draw_layer_features(
    ui: &mut egui::Ui,
    layer: &mut Layer,
    current_frame: u32,
    project_changed: &mut bool,
    next_frame: &mut Option<u32>,
) {
    // ── Shape Repeater (AE Contents > Repeater parity) ──
    ui.collapsing("⧉ Shape Repeater", |ui| {
        if layer.shape_repeater.is_none() {
            if kui::button(ui, "+ Add Repeater", kui::ButtonKind::Default)
                .on_hover_text("Duplicate the shape N times with cumulative offsets")
                .clicked()
            {
                layer.shape_repeater =
                    Some(crate::core::shape_repeater::ShapeRepeaterOptions::default());
                *project_changed = true;
            }
        } else if let Some(rep) = &mut layer.shape_repeater {
            ui.horizontal(|ui| {
                ui.label("Copies:");
                if rep.copies_animation.is_none() {
                    if kui::button(ui, "Animate", kui::ButtonKind::Quiet).clicked() {
                        rep.copies_animation = Some(
                            crate::core::property::Animatable::new_constant(rep.copies as f32),
                        );
                        *project_changed = true;
                    }
                    let mut copies_i = rep.copies as i32;
                    if ui
                        .add(kui::drag(&mut copies_i).range(0..=500).speed(1.0))
                        .changed()
                    {
                        rep.copies = copies_i.max(0) as u32;
                        *project_changed = true;
                    }
                } else if let Some(copies) = &mut rep.copies_animation {
                    if let Some(nf) =
                        draw_property_ui(current_frame, ui, "", copies, |ui, value| {
                            kui::number(ui, value, 0.0..=500.0, 1.0, "");
                        })
                    {
                        *next_frame = Some(nf);
                    }
                    if kui::button(ui, "× Static", kui::ButtonKind::Quiet).clicked() {
                        rep.copies = copies.evaluate(current_frame).max(0.0).floor() as u32;
                        rep.copies_animation = None;
                        *project_changed = true;
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label("Offset:");
                if ui
                    .add(kui::drag(&mut rep.offset).range(-100.0..=100.0).speed(0.1))
                    .changed()
                {
                    *project_changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Position Offset:");
                if rep.position_offset_animation.is_none() {
                    if kui::button(ui, "Animate", kui::ButtonKind::Quiet).clicked() {
                        rep.position_offset_animation = Some(
                            crate::core::property::Animatable::new_constant(rep.position_offset),
                        );
                        *project_changed = true;
                    }
                    if ui
                        .add(
                            kui::drag(&mut rep.position_offset[0])
                                .speed(1.0)
                                .prefix("X "),
                        )
                        .changed()
                        || ui
                            .add(
                                kui::drag(&mut rep.position_offset[1])
                                    .speed(1.0)
                                    .prefix("Y "),
                            )
                            .changed()
                    {
                        *project_changed = true;
                    }
                } else if let Some(position) = &mut rep.position_offset_animation {
                    if let Some(nf) =
                        draw_property_ui(current_frame, ui, "", position, |ui, value| {
                            kui::vector_axes_unbounded(ui, value, 1.0, "", ["X", "Y"]);
                        })
                    {
                        *next_frame = Some(nf);
                    }
                    if kui::button(ui, "× Static", kui::ButtonKind::Quiet).clicked() {
                        rep.position_offset = position.evaluate(current_frame);
                        rep.position_offset_animation = None;
                        *project_changed = true;
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label("Scale /copy %:");
                if rep.scale_offset_animation.is_none() {
                    if kui::button(ui, "Animate", kui::ButtonKind::Quiet).clicked() {
                        rep.scale_offset_animation = Some(
                            crate::core::property::Animatable::new_constant(rep.scale_offset),
                        );
                        *project_changed = true;
                    }
                    if ui
                        .add(
                            kui::drag(&mut rep.scale_offset[0])
                                .range(0.05..=5.0)
                                .speed(0.01)
                                .unit("×"),
                        )
                        .changed()
                        || ui
                            .add(
                                kui::drag(&mut rep.scale_offset[1])
                                    .range(0.05..=5.0)
                                    .speed(0.01)
                                    .unit("×"),
                            )
                            .changed()
                    {
                        *project_changed = true;
                    }
                } else if let Some(scale) = &mut rep.scale_offset_animation {
                    if let Some(nf) = draw_property_ui(current_frame, ui, "", scale, |ui, value| {
                        ui.add(
                            kui::drag(&mut value[0])
                                .range(0.05..=5.0)
                                .speed(0.01)
                                .unit("×"),
                        );
                        ui.add(
                            kui::drag(&mut value[1])
                                .range(0.05..=5.0)
                                .speed(0.01)
                                .unit("×"),
                        );
                    }) {
                        *next_frame = Some(nf);
                    }
                    if kui::button(ui, "× Static", kui::ButtonKind::Quiet).clicked() {
                        rep.scale_offset = scale.evaluate(current_frame);
                        rep.scale_offset_animation = None;
                        *project_changed = true;
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label("Rotation /copy:");
                if rep.rotation_offset_animation.is_none() {
                    if kui::button(ui, "Animate", kui::ButtonKind::Quiet).clicked() {
                        rep.rotation_offset_animation =
                            Some(crate::core::property::Animatable::new_constant(
                                rep.rotation_offset_deg,
                            ));
                        *project_changed = true;
                    }
                    if ui
                        .add(kui::drag(&mut rep.rotation_offset_deg).speed(1.0).unit("°"))
                        .changed()
                    {
                        *project_changed = true;
                    }
                } else if let Some(rotation) = &mut rep.rotation_offset_animation {
                    if let Some(nf) =
                        draw_property_ui(current_frame, ui, "", rotation, |ui, value| {
                            ui.add(kui::drag(value).speed(1.0).unit("°"));
                        })
                    {
                        *next_frame = Some(nf);
                    }
                    if kui::button(ui, "× Static", kui::ButtonKind::Quiet).clicked() {
                        rep.rotation_offset_deg = rotation.evaluate(current_frame);
                        rep.rotation_offset_animation = None;
                        *project_changed = true;
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label("Opacity fade:");
                if rep.opacity_animation.is_none() {
                    if kui::button(ui, "Animate", kui::ButtonKind::Quiet).clicked() {
                        rep.opacity_animation =
                            Some(crate::core::property::Animatable::new_constant([
                                rep.start_opacity,
                                rep.end_opacity,
                            ]));
                        *project_changed = true;
                    }
                    if kui::slider(ui, &mut rep.start_opacity, 0.0..=1.0, "%").changed()
                        || kui::slider(ui, &mut rep.end_opacity, 0.0..=1.0, "%").changed()
                    {
                        *project_changed = true;
                    }
                } else if let Some(opacity) = &mut rep.opacity_animation {
                    if let Some(nf) =
                        draw_property_ui(current_frame, ui, "", opacity, |ui, value| {
                            kui::property(ui, "Start", |ui| {
                                kui::slider(ui, &mut value[0], 0.0..=1.0, "")
                            });
                            kui::property(ui, "End", |ui| {
                                kui::slider(ui, &mut value[1], 0.0..=1.0, "")
                            });
                        })
                    {
                        *next_frame = Some(nf);
                    }
                    if kui::button(ui, "× Static", kui::ButtonKind::Quiet).clicked() {
                        let values = opacity.evaluate(current_frame);
                        rep.start_opacity = values[0];
                        rep.end_opacity = values[1];
                        rep.opacity_animation = None;
                        *project_changed = true;
                    }
                }
            });
            if kui::toggle(ui, &mut rep.composite_below, "Composite below").changed() {
                *project_changed = true;
            }
            if kui::button(ui, "Remove Repeater", kui::ButtonKind::Quiet).clicked() {
                layer.shape_repeater = None;
                *project_changed = true;
            }
        }
    });

    ui.separator();
    // ── Puppet Pins (deformation mesh handles, AE Puppet Tool parity) ──
    ui.collapsing("Puppet Pins & Mesh", |ui| {
        ui.horizontal(|ui| {
            if ui
                .button("+ Position Pin")
                .on_hover_text("Deformation position pin")
                .clicked()
            {
                let n = layer.puppet_pins.len() + 1;
                let center = layer.transform.position.evaluate(current_frame);
                layer
                    .puppet_pins
                    .push(crate::core::timeline::PuppetPin::new(
                        format!("pin_{}", n),
                        format!("Pin {}", n),
                        center,
                    ));
                *project_changed = true;
            }
            if ui
                .button("+ Starch Pin")
                .on_hover_text("Pin to add stiffness and prevent stretching")
                .clicked()
            {
                let n = layer.puppet_pins.len() + 1;
                let center = layer.transform.position.evaluate(current_frame);
                layer
                    .puppet_pins
                    .push(crate::core::timeline::PuppetPin::new(
                        format!("starch_{}", n),
                        format!("Starch {}", n),
                        center,
                    ));
                *project_changed = true;
            }
        });

        let mut mesh_density = ui.ctx().data(|d| {
            d.get_temp::<f32>(egui::Id::new("puppet_mesh_density"))
                .unwrap_or(50.0)
        });
        ui.horizontal(|ui| {
            if kui::property(ui, "Mesh Density", |ui| {
                kui::slider(ui, &mut mesh_density, 10.0..=100.0, " tri")
            })
            .changed()
            {
                ui.ctx().data_mut(|d| {
                    d.insert_temp(egui::Id::new("puppet_mesh_density"), mesh_density)
                });
                *project_changed = true;
            }
        });
        let mut remove_idx: Option<usize> = None;
        for (pi, pin) in layer.puppet_pins.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(pin.name.to_string()).small());
                if let Some(nf) =
                    draw_property_ui(current_frame, ui, "", &mut pin.position, |ui, val| {
                        kui::vector_axes_unbounded(ui, val, 1.0, "", ["X", "Y"]);
                    })
                {
                    *next_frame = Some(nf);
                }
                if kui::button(ui, "×", kui::ButtonKind::Quiet)
                    .on_hover_text("Remove pin")
                    .clicked()
                {
                    remove_idx = Some(pi);
                }
            });
        }
        if let Some(ri) = remove_idx {
            layer.puppet_pins.remove(ri);
            *project_changed = true;
        }
    });

    ui.separator();
    // ── Paint Strokes ──
    ui.collapsing("Paint Strokes", |ui| {
        let n = layer.paint_strokes.len();
        if n == 0 {
            ui.label(
                egui::RichText::new("No strokes — use the Brush tool in the viewport")
                    .small()
                    .color(colors::TEXT_MUTED),
            );
        } else {
            ui.label(
                egui::RichText::new(format!("{} stroke{}", n, if n == 1 { "" } else { "s" }))
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
        }
        let mut remove_idx: Option<usize> = None;
        for (si, s) in layer.paint_strokes.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                let cr = (s.color[0] * 255.0) as u8;
                let cg = (s.color[1] * 255.0) as u8;
                let cb = (s.color[2] * 255.0) as u8;
                let (rct, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                ui.painter()
                    .rect_filled(rct, 2.0, egui::Color32::from_rgb(cr, cg, cb));
                ui.label(
                    egui::RichText::new(format!("Stroke {} · {} pts", si + 1, s.points.len()))
                        .small(),
                );
                if kui::button(ui, "×", kui::ButtonKind::Quiet)
                    .on_hover_text("Delete stroke")
                    .clicked()
                {
                    remove_idx = Some(si);
                }
            });
            if kui::property(ui, "In", |ui| {
                ui.add(kui::drag(&mut s.start_frame).range(0..=999_999))
            })
            .changed()
            {
                *project_changed = true;
            }
            let mut ef = s.end_frame;
            let suffix = if ef == 0 { " (auto)" } else { "" };
            if kui::property(ui, "Out", |ui| {
                ui.add(kui::drag(&mut ef).range(0..=999_999).unit(suffix))
            })
            .changed()
            {
                s.end_frame = ef;
                *project_changed = true;
            }
        }
        if let Some(ri) = remove_idx {
            layer.paint_strokes.remove(ri);
            *project_changed = true;
        }
    });

    ui.separator();
    ui.collapsing("Trim Paths Animator", |ui| {
        if layer.trim_paths.is_none() {
            if kui::button(ui, "+ Add Trim Paths", kui::ButtonKind::Default).clicked() {
                layer.trim_paths = Some(crate::core::timeline::TrimPaths::default());
                *project_changed = true;
            }
        } else if let Some(ref mut trim) = layer.trim_paths {
            ui.horizontal(|ui| {
                ui.label("Start:");
                if let Some(nf) =
                    draw_property_ui(current_frame, ui, "Start", &mut trim.start, |ui, val| {
                        kui::slider(ui, val, 0.0..=100.0, "%");
                    })
                {
                    *next_frame = Some(nf);
                }
            });
            ui.horizontal(|ui| {
                ui.label("End:");
                if let Some(nf) =
                    draw_property_ui(current_frame, ui, "End", &mut trim.end, |ui, val| {
                        kui::slider(ui, val, 0.0..=100.0, "%");
                    })
                {
                    *next_frame = Some(nf);
                }
            });
            ui.horizontal(|ui| {
                ui.label("Offset:");
                if let Some(nf) =
                    draw_property_ui(current_frame, ui, "Offset", &mut trim.offset, |ui, val| {
                        ui.add(kui::drag(val).speed(1.0).unit("°"));
                    })
                {
                    *next_frame = Some(nf);
                }
            });
        }
    });

    ui.separator();
    // ── Layer Styles Inspector Section ──
    ui.collapsing("Layer Styles", |ui| {
        let style = &mut layer.style;

        // Stroke
        ui.collapsing("Stroke", |ui| {
            if kui::toggle(ui, &mut style.stroke.enabled, "Enabled").changed() {
                *project_changed = true;
            }
            *project_changed |= kui::property(ui, "Size", |ui| {
                kui::slider(ui, &mut style.stroke.size, 1.0..=100.0, " px")
            })
            .changed();
            *project_changed |= kui::property(ui, "Color", |ui| {
                kui::color_rgba(ui, &mut style.stroke.color)
            })
            .changed();
        });

        // Drop Shadow
        ui.collapsing("Drop Shadow", |ui| {
            if kui::toggle(ui, &mut style.drop_shadow.enabled, "Enabled").changed() {
                *project_changed = true;
            }
            *project_changed |= kui::property(ui, "Opacity", |ui| {
                kui::slider(ui, &mut style.drop_shadow.opacity, 0.0..=100.0, "%")
            })
            .changed();
            *project_changed |= kui::property(ui, "Distance", |ui| {
                ui.add(kui::drag(&mut style.drop_shadow.distance).prefix("Dist: "))
            })
            .changed();
            *project_changed |= kui::property(ui, "Size", |ui| {
                ui.add(kui::drag(&mut style.drop_shadow.size))
            })
            .changed();
            *project_changed |= kui::property(ui, "Angle", |ui| {
                kui::slider(ui, &mut style.drop_shadow.angle, -180.0..=180.0, "°")
            })
            .changed();
        });

        // Color Overlay
        ui.collapsing("Color Overlay", |ui| {
            if kui::toggle(ui, &mut style.color_overlay.enabled, "Enabled").changed() {
                *project_changed = true;
            }
            *project_changed |= kui::property(ui, "Opacity", |ui| {
                kui::slider(ui, &mut style.color_overlay.opacity, 0.0..=100.0, "%")
            })
            .changed();
            *project_changed |= kui::property(ui, "Color", |ui| {
                kui::color_rgba(ui, &mut style.color_overlay.color)
            })
            .changed();
        });

        // Gradient Overlay
        ui.collapsing("Gradient Overlay", |ui| {
            if kui::toggle(ui, &mut style.gradient_overlay.enabled, "Enabled").changed() {
                *project_changed = true;
            }
            let go = &mut style.gradient_overlay;
            *project_changed |= kui::property(ui, "Opacity", |ui| {
                kui::number(ui, &mut go.opacity, 0.0..=100.0, 0.1, "%")
            })
            .changed();
            *project_changed |= kui::property(ui, "Angle", |ui| {
                kui::number(ui, &mut go.angle, -360.0..=360.0, 1.0, "°")
            })
            .changed();
        });

        // Bevel & Emboss
        ui.collapsing("Bevel / Emboss", |ui| {
            if kui::toggle(ui, &mut style.bevel_emboss.enabled, "Enabled").changed() {
                *project_changed = true;
            }
            let bv = &mut style.bevel_emboss;
            *project_changed |=
                kui::property(ui, "Depth", |ui| ui.add(kui::drag(&mut bv.depth))).changed();
            *project_changed |=
                kui::property(ui, "Size", |ui| ui.add(kui::drag(&mut bv.size))).changed();
            *project_changed |= kui::property(ui, "Highlight", |ui| {
                kui::number(ui, &mut bv.highlight, 0.0..=100.0, 0.1, "%")
            })
            .changed();
            *project_changed |= kui::property(ui, "Shadow", |ui| {
                kui::number(ui, &mut bv.shadow, 0.0..=100.0, 0.1, "%")
            })
            .changed();
        });
    });
}
