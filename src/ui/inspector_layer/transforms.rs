use crate::core::timeline::{Layer, LayerType};
use crate::ui::inspector_property::{
    draw_easy_ease_button, draw_expression_selector, draw_property_ui,
};
use crate::ui::kui;
use crate::ui::theme::colors;
use eframe::egui;

pub fn draw_layer_transforms(
    ui: &mut egui::Ui,
    layer: &mut Layer,
    current_frame: u32,
    fps: u32,
    project_changed: &mut bool,
    next_frame: &mut Option<u32>,
) {
    let tr = crate::ui::localization::tr;
    egui::Frame::none().show(ui, |ui| {
        if layer.is_3d {
            ui.label(
                egui::RichText::new(tr("Transform 3D"))
                    .strong()
                    .color(colors::TEXT_PRIMARY),
            );

            let pos_before = layer.transform_3d.position.clone();
            if let Some(nf) = draw_property_ui(
                current_frame,
                ui,
                &tr("Position (XYZ)"),
                &mut layer.transform_3d.position,
                |ui, val| {
                    kui::vector_axes_unbounded(ui, val, 1.0, "", ["X", "Y", "Z"]);
                },
            ) {
                *next_frame = Some(nf);
            }
            draw_easy_ease_button(ui, &mut layer.transform_3d.position, project_changed);
            if pos_before != layer.transform_3d.position {
                *project_changed = true;
            }

            ui.add_space(3.0);
            let rot_before = layer.transform_3d.rotation.clone();
            if let Some(nf) = draw_property_ui(
                current_frame,
                ui,
                &tr("Rotation (YPR)"),
                &mut layer.transform_3d.rotation,
                |ui, val| {
                    kui::vector_axes_unbounded(ui, val, 1.0, "°", ["P", "Y", "R"]);
                },
            ) {
                *next_frame = Some(nf);
            }
            draw_easy_ease_button(ui, &mut layer.transform_3d.rotation, project_changed);
            if rot_before != layer.transform_3d.rotation {
                *project_changed = true;
            }

            ui.add_space(3.0);
            let scale_before = layer.transform_3d.scale.clone();
            if let Some(nf) = draw_property_ui(
                current_frame,
                ui,
                &tr("Scale (XYZ)"),
                &mut layer.transform_3d.scale,
                |ui, val| {
                    kui::vector_axes_unbounded(ui, val, 0.1, "%", ["X", "Y", "Z"]);
                },
            ) {
                *next_frame = Some(nf);
            }
            draw_easy_ease_button(ui, &mut layer.transform_3d.scale, project_changed);
            if scale_before != layer.transform_3d.scale {
                *project_changed = true;
            }

            // ── 3D Material Options ──
            ui.add_space(4.0);
            ui.collapsing(tr("Material Options"), |ui| {
                let mat = &mut layer.material;
                if kui::property(ui, "Casts Shadows", |ui| {
                    kui::toggle(ui, &mut mat.cast_shadows, "")
                })
                .changed()
                {
                    *project_changed = true;
                }
                *project_changed |= kui::property(ui, "Ambient", |ui| {
                    kui::slider(ui, &mut mat.ambient, 0.0..=1.0, "")
                })
                .changed();
                *project_changed |= kui::property(ui, "Diffuse", |ui| {
                    kui::slider(ui, &mut mat.diffuse, 0.0..=1.0, "")
                })
                .changed();
                *project_changed |= kui::property(ui, "Specular", |ui| {
                    kui::slider(ui, &mut mat.specular, 0.0..=1.0, "")
                })
                .changed();
                *project_changed |= kui::property(ui, "Shininess", |ui| {
                    kui::slider(ui, &mut mat.specular_exponent, 1.0..=256.0, "")
                })
                .changed();
                *project_changed |= kui::property(ui, "Metalness", |ui| {
                    kui::slider(ui, &mut mat.metalness, 0.0..=1.0, "")
                })
                .changed();
                *project_changed |= kui::property(ui, "Emission", |ui| {
                    kui::slider(ui, &mut mat.emission, 0.0..=1.0, "")
                })
                .changed();
            });
        } else {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(tr("Transform 2D"))
                        .strong()
                        .color(colors::TEXT_PRIMARY),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if kui::button(ui, "Reset", kui::ButtonKind::Quiet)
                        .on_hover_text("Reset position, scale, rotation, and anchor point to default values")
                        .clicked()
                    {
                        layer.transform = crate::core::timeline::Transform2D::default();
                        *project_changed = true;
                    }
                });
            });

            let val_before = layer.transform.anchor_point.clone();
            if let Some(nf) = draw_property_ui(
                current_frame,
                ui,
                &tr("Anchor Point"),
                &mut layer.transform.anchor_point,
                |ui, val| {
                    kui::vector_axes_unbounded(ui, val, 1.0, "", ["X", "Y"]);
                },
            ) {
                *next_frame = Some(nf);
            }

            if val_before != layer.transform.anchor_point {
                *project_changed = true;
            }
            ui.add_space(3.0);
            let pos_before = layer.transform.position.clone();
            if let Some(nf) = draw_property_ui(
                current_frame,
                ui,
                &tr("Position"),
                &mut layer.transform.position,
                |ui, val| {
                    kui::vector_axes_unbounded(ui, val, 1.0, "", ["X", "Y"]);
                },
            ) {
                *next_frame = Some(nf);
            }
            draw_easy_ease_button(ui, &mut layer.transform.position, project_changed);
            if pos_before != layer.transform.position {
                *project_changed = true;
            }

            ui.add_space(3.0);
            let scale_before = layer.transform.scale.clone();
            if let Some(nf) = draw_property_ui(
                current_frame,
                ui,
                &tr("Scale"),
                &mut layer.transform.scale,
                |ui, val| {
                    kui::vector_axes_unbounded(ui, val, 0.1, "%", ["X", "Y"]);
                },
            ) {
                *next_frame = Some(nf);
            }
            draw_easy_ease_button(ui, &mut layer.transform.scale, project_changed);
            if scale_before != layer.transform.scale {
                *project_changed = true;
            }

            ui.add_space(3.0);
            let rot_before = layer.transform.rotation.clone();
            if let Some(nf) = draw_property_ui(
                current_frame,
                ui,
                &tr("Rotation"),
                &mut layer.transform.rotation,
                |ui, val| {
                    ui.horizontal(|ui| {
                        crate::ui::custom_widgets::ae_scrubber(ui, val, 1.0, "°", None);
                        kui::slider(ui, val, -360.0..=360.0, "°");
                    });
                },
            ) {
                *next_frame = Some(nf);
            }
            draw_easy_ease_button(ui, &mut layer.transform.rotation, project_changed);
            if rot_before != layer.transform.rotation {
                *project_changed = true;
            }

            // ── Auto-Orient (AE parity): rotation follows motion path ──
            {
                use crate::core::auto_orient::AutoOrientMode;
                let ao_before = layer.auto_orient;
                let mut mode_idx = match layer.auto_orient {
                    AutoOrientMode::Off => 0,
                    AutoOrientMode::OrientAlongPath => 1,
                    AutoOrientMode::OrientTowardsPoint { .. } => 2,
                };
                let orient_options = [
                    (0, "Off"),
                    (1, "Orient Along Path"),
                    (2, "Orient Towards Point"),
                ];
                let mode_changed = kui::property(ui, &tr("Auto-Orient"), |ui| {
                    let translated = orient_options.map(|(value, label)| (value, tr(label)));
                    let options = translated
                        .iter()
                        .map(|(value, label)| (*value, label.as_str()))
                        .collect::<Vec<_>>();
                    kui::combo(ui, "insp_auto_orient", &mut mode_idx, &options)
                });
                if mode_changed {
                    let target_point = match layer.auto_orient {
                        AutoOrientMode::OrientTowardsPoint { target_point } => target_point,
                        _ => [960.0, 540.0],
                    };
                    layer.auto_orient = match mode_idx {
                        1 => AutoOrientMode::OrientAlongPath,
                        2 => AutoOrientMode::OrientTowardsPoint { target_point },
                        _ => AutoOrientMode::Off,
                    };
                }
                if let AutoOrientMode::OrientTowardsPoint { target_point } = &mut layer.auto_orient
                {
                    kui::property(ui, &tr("Target"), |ui| {
                        kui::vector_axes_unbounded(ui, target_point, 1.0, "", ["X", "Y"])
                    });
                }
                if ao_before != layer.auto_orient {
                    *project_changed = true;
                }
            }

            ui.add_space(3.0);
            let _op_before = layer.transform.opacity.clone();
            if let Some(nf) = draw_property_ui(
                current_frame,
                ui,
                &tr("Opacity"),
                &mut layer.transform.opacity,
                |ui, val| {
                    ui.horizontal(|ui| {
                        crate::ui::custom_widgets::ae_scrubber(ui, val, 1.0, "%", Some(0.0..=100.0));
                        kui::slider(ui, val, 0.0..=100.0, "%");
                    });
                },
            ) {
                *next_frame = Some(nf);
            }
            draw_easy_ease_button(ui, &mut layer.transform.opacity, project_changed);
            let has_transform_expression = layer.transform.anchor_point_expression.is_some()
                || layer.transform.position_expression.is_some()
                || layer.transform.scale_expression.is_some()
                || layer.transform.rotation_expression.is_some()
                || layer.transform.opacity_expression.is_some();
            egui::CollapsingHeader::new(egui::RichText::new(tr("Expressions")).size(12.0).color(
                if has_transform_expression {
                    colors::TEXT_PRIMARY
                } else {
                    colors::TEXT_SECONDARY
                },
            ))
            .id_salt("transform_expressions")
            .default_open(has_transform_expression)
            .show(ui, |ui| {
                draw_expression_selector(
                    ui,
                    "anchor",
                    &mut layer.transform.anchor_point_expression,
                    project_changed,
                    Some(current_frame),
                    Some(fps),
                );
                draw_expression_selector(
                    ui,
                    "position",
                    &mut layer.transform.position_expression,
                    project_changed,
                    Some(current_frame),
                    Some(fps),
                );
                draw_expression_selector(
                    ui,
                    "scale",
                    &mut layer.transform.scale_expression,
                    project_changed,
                    Some(current_frame),
                    Some(fps),
                );
                draw_expression_selector(
                    ui,
                    "rotation",
                    &mut layer.transform.rotation_expression,
                    project_changed,
                    Some(current_frame),
                    Some(fps),
                );
                draw_expression_selector(
                    ui,
                    "opacity",
                    &mut layer.transform.opacity_expression,
                    project_changed,
                    Some(current_frame),
                    Some(fps),
                );
            });
            if layer.is_3d {
                ui.separator();
                ui.label(
                    egui::RichText::new("3D Spatial Transform")
                        .small()
                        .strong()
                        .color(colors::ACCENT_CYAN),
                );

                let mut pos3d = layer.transform_3d.position.evaluate(current_frame);
                if kui::property(ui, "Position Z (Depth)", |ui| {
                    kui::number(ui, &mut pos3d[2], f32::MIN..=f32::MAX, 1.0, " px")
                })
                .changed()
                {
                    layer.transform_3d.position =
                        crate::core::property::Animatable::new_constant(pos3d);
                    *project_changed = true;
                }

                let mut rot3d = layer.transform_3d.rotation.evaluate(current_frame);
                if kui::property(ui, "Rotation XYZ", |ui| {
                    kui::vector_axes_unbounded(ui, &mut rot3d, 1.0, "°", ["X", "Y", "Z"])
                })
                .changed()
                {
                    layer.transform_3d.rotation =
                        crate::core::property::Animatable::new_constant(rot3d);
                    *project_changed = true;
                }

                // ── 3D Material Options ──
                ui.separator();
                ui.label(
                    egui::RichText::new("Material Options")
                        .small()
                        .strong()
                        .color(colors::ACCENT_YELLOW),
                );

                *project_changed |= kui::property(ui, "Ambient", |ui| {
                    kui::slider_step(ui, &mut layer.material.ambient, 0.0..=1.0, "", 0.01)
                })
                .changed();
                *project_changed |= kui::property(ui, "Diffuse", |ui| {
                    kui::slider_step(ui, &mut layer.material.diffuse, 0.0..=1.0, "", 0.01)
                })
                .changed();
                *project_changed |= kui::property(ui, "Specular", |ui| {
                    kui::slider_step(ui, &mut layer.material.specular, 0.0..=1.0, "", 0.01)
                })
                .changed();
                *project_changed |= kui::property(ui, "Specular Exp", |ui| {
                    kui::number(
                        ui,
                        &mut layer.material.specular_exponent,
                        1.0..=256.0,
                        1.0,
                        "",
                    )
                })
                .changed();
                *project_changed |= kui::property(ui, "Emission", |ui| {
                    kui::slider_step(ui, &mut layer.material.emission, 0.0..=1.0, "", 0.01)
                })
                .changed();
                *project_changed |= kui::property(ui, "Metalness", |ui| {
                    kui::slider_step(ui, &mut layer.material.metalness, 0.0..=1.0, "", 0.01)
                })
                .changed();
                *project_changed |= kui::property(ui, "Casts Shadows", |ui| {
                    kui::toggle(ui, &mut layer.material.cast_shadows, "")
                })
                .changed();
                *project_changed |= kui::property(ui, "Accepts Shadows", |ui| {
                    kui::toggle(ui, &mut layer.material.accepts_shadows, "")
                })
                .changed();
                *project_changed |= kui::property(ui, "Accepts Lights", |ui| {
                    kui::toggle(ui, &mut layer.material.accepts_lights, "")
                })
                .changed();
                *project_changed |= kui::property(ui, "Light Transmission", |ui| {
                    kui::slider(ui, &mut layer.material.light_transmission, 0.0..=100.0, "%")
                })
                .changed();

                // ── 3D Geometry Options (Extrusion & Bevel) ──
                ui.separator();
                ui.collapsing("Geometry Options (Extrusion & Bevel)", |ui| {
                    if let LayerType::Shape {
                        extrusion_depth,
                        bevel_depth,
                        ..
                    } = &mut layer.layer_type
                    {
                        *project_changed |= kui::property(ui, "Extrusion Depth", |ui| {
                            kui::number(ui, extrusion_depth, 0.0..=500.0, 1.0, " px")
                        })
                        .changed();
                        *project_changed |= kui::property(ui, "Bevel Depth", |ui| {
                            kui::number(ui, bevel_depth, 0.0..=50.0, 0.5, " px")
                        })
                        .changed();
                    } else {
                        ui.label("Extrusion is only available for Shape layers.");
                    }
                });
            }
        }

        ui.separator();
        ui.collapsing("Responsive Constraints (Pinning)", |ui| {
            use crate::core::layer_constraints::{HorizontalPin, VerticalPin};
            let constraints_before = layer.constraints;

            let horizontal_options = [
                (HorizontalPin::Left, "Left"),
                (HorizontalPin::Center, "Center"),
                (HorizontalPin::Right, "Right"),
                (HorizontalPin::Scale, "Scale"),
            ];
            *project_changed |= kui::property(ui, "Horizontal Pin", |ui| {
                kui::combo(
                    ui,
                    format!("pin_h_{}", layer.id),
                    &mut layer.constraints.horizontal,
                    &horizontal_options,
                )
            });

            let vertical_options = [
                (VerticalPin::Top, "Top"),
                (VerticalPin::Center, "Center"),
                (VerticalPin::Bottom, "Bottom"),
                (VerticalPin::Scale, "Scale"),
            ];
            *project_changed |= kui::property(ui, "Vertical Pin", |ui| {
                kui::combo(
                    ui,
                    format!("pin_v_{}", layer.id),
                    &mut layer.constraints.vertical,
                    &vertical_options,
                )
            });

            if constraints_before != layer.constraints {
                *project_changed = true;
            }
        });

        // Label Color picker (AE standard track color)
        ui.collapsing("Label Color", |ui| {
            ui.horizontal(|ui| {
                use crate::core::timeline::LabelColor;
                for color in [
                    LabelColor::None,
                    LabelColor::Red,
                    LabelColor::Yellow,
                    LabelColor::Aqua,
                    LabelColor::Pink,
                    LabelColor::Lavender,
                    LabelColor::Peach,
                    LabelColor::Sea,
                    LabelColor::Blue,
                    LabelColor::Purple,
                ] {
                    let rgb = color.to_rgb();
                    let c32 = egui::Color32::from_rgb(
                        (rgb[0] * 255.0) as u8,
                        (rgb[1] * 255.0) as u8,
                        (rgb[2] * 255.0) as u8,
                    );
                    let btn = crate::ui::custom_widgets::ae_color_swatch(ui, c32, 18.0);
                    if btn.clicked() && layer.label != color {
                        layer.label = color;
                        *project_changed = true;
                    }
                    if layer.label == color {
                        ui.painter().rect_stroke(
                            btn.rect,
                            2.0,
                            egui::Stroke::new(2.0_f32, egui::Color32::WHITE),
                        );
                    }
                }
            });
        });

        // Layer Styles (Drop Shadow & Stroke) UI Controls
        ui.collapsing("Layer Styles", |ui| {
            let ds = &mut layer.style.drop_shadow;
            ui.collapsing("Drop Shadow", |ui| {
                *project_changed |=
                    kui::property(ui, "Enabled", |ui| kui::toggle(ui, &mut ds.enabled, ""))
                        .changed();
                *project_changed |= kui::property(ui, "Distance", |ui| {
                    kui::slider(ui, &mut ds.distance, 0.0..=200.0, " px")
                })
                .changed();
                *project_changed |= kui::property(ui, "Blur Size", |ui| {
                    kui::slider(ui, &mut ds.size, 0.0..=100.0, " px")
                })
                .changed();
                *project_changed |= kui::property(ui, "Opacity", |ui| {
                    kui::slider(ui, &mut ds.opacity, 0.0..=1.0, "")
                })
                .changed();
            });

            let st = &mut layer.style.stroke;
            ui.collapsing("Stroke", |ui| {
                *project_changed |=
                    kui::property(ui, "Enabled", |ui| kui::toggle(ui, &mut st.enabled, ""))
                        .changed();
                *project_changed |= kui::property(ui, "Size", |ui| {
                    kui::slider(ui, &mut st.size, 1.0..=50.0, " px")
                })
                .changed();
            });
        });
    });
}
