use crate::core::text_animator::{SelectorShape, TextAnimatorSettings};
use crate::core::timeline::{Layer, LayerType};
use crate::ui::inspector_property::{
    draw_easy_ease_button, draw_expression_selector, draw_property_ui,
};
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
    egui::Frame::none().show(ui, |ui| {
        if layer.is_3d {
            ui.label(
                egui::RichText::new("Transform 3D")
                    .strong()
                    .color(colors::TEXT_PRIMARY),
            );

            let pos_before = layer.transform_3d.position.clone();
            if let Some(nf) = draw_property_ui(
                current_frame,
                ui,
                "Position (XYZ)",
                &mut layer.transform_3d.position,
                |ui, val| {
                    ui.horizontal_wrapped(|ui| {
                        ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).prefix("X: "));
                        ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).prefix("Y: "));
                        ui.add(egui::DragValue::new(&mut val[2]).speed(1.0).prefix("Z: "));
                    });
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
                "Rotation (YPR)",
                &mut layer.transform_3d.rotation,
                |ui, val| {
                    ui.horizontal_wrapped(|ui| {
                        ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).suffix("° P"));
                        ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).suffix("° Y"));
                        ui.add(egui::DragValue::new(&mut val[2]).speed(1.0).suffix("° R"));
                    });
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
                "Scale (XYZ)",
                &mut layer.transform_3d.scale,
                |ui, val| {
                    ui.horizontal_wrapped(|ui| {
                        ui.add(egui::DragValue::new(&mut val[0]).speed(0.1).suffix("% X"));
                        ui.add(egui::DragValue::new(&mut val[1]).speed(0.1).suffix("% Y"));
                        ui.add(egui::DragValue::new(&mut val[2]).speed(0.1).suffix("% Z"));
                    });
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
            ui.collapsing("Material Options", |ui| {
                let mat = &mut layer.material;
                if ui
                    .checkbox(&mut mat.cast_shadows, "Casts Shadows")
                    .changed()
                {
                    *project_changed = true;
                }
                ui.horizontal(|ui| {
                    ui.label("Ambient:");
                    if ui
                        .add(egui::Slider::new(&mut mat.ambient, 0.0..=1.0))
                        .changed()
                    {
                        *project_changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Diffuse:");
                    if ui
                        .add(egui::Slider::new(&mut mat.diffuse, 0.0..=1.0))
                        .changed()
                    {
                        *project_changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Specular:");
                    if ui
                        .add(egui::Slider::new(&mut mat.specular, 0.0..=1.0))
                        .changed()
                    {
                        *project_changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Shininess:");
                    if ui
                        .add(egui::Slider::new(&mut mat.specular_exponent, 1.0..=256.0))
                        .changed()
                    {
                        *project_changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Metalness:");
                    if ui
                        .add(egui::Slider::new(&mut mat.metalness, 0.0..=1.0))
                        .changed()
                    {
                        *project_changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Emission:");
                    if ui
                        .add(egui::Slider::new(&mut mat.emission, 0.0..=1.0))
                        .changed()
                    {
                        *project_changed = true;
                    }
                });
            });
        } else {
            ui.label(
                egui::RichText::new("Transform 2D")
                    .strong()
                    .color(colors::TEXT_PRIMARY),
            );

            let val_before = layer.transform.anchor_point.clone();
            if let Some(nf) = draw_property_ui(
                current_frame,
                ui,
                "Anchor Point",
                &mut layer.transform.anchor_point,
                |ui, val| {
                    ui.horizontal_wrapped(|ui| {
                        ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).prefix("X: "));
                        ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).prefix("Y: "));
                    });
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
                "Position",
                &mut layer.transform.position,
                |ui, val| {
                    ui.horizontal_wrapped(|ui| {
                        ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).prefix("X: "));
                        ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).prefix("Y: "));
                    });
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
                "Scale",
                &mut layer.transform.scale,
                |ui, val| {
                    ui.horizontal_wrapped(|ui| {
                        ui.add(egui::DragValue::new(&mut val[0]).speed(0.1).suffix("% X"));
                        ui.add(egui::DragValue::new(&mut val[1]).speed(0.1).suffix("% Y"));
                    });
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
                "Rotation",
                &mut layer.transform.rotation,
                |ui, val| {
                    ui.add(egui::Slider::new(val, -360.0..=360.0).suffix("°"));
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
                let mode_text = match layer.auto_orient {
                    AutoOrientMode::Off => "Off",
                    AutoOrientMode::OrientAlongPath => "Orient Along Path",
                    AutoOrientMode::OrientTowardsPoint { .. } => "Orient Towards Point",
                };
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        egui::RichText::new("Auto-Orient")
                            .small()
                            .color(colors::TEXT_SECONDARY),
                    );
                    egui::ComboBox::from_id_salt("insp_auto_orient")
                        .selected_text(mode_text)
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_label(layer.auto_orient == AutoOrientMode::Off, "Off")
                                .clicked()
                            {
                                layer.auto_orient = AutoOrientMode::Off;
                            }
                            if ui
                                .selectable_label(
                                    layer.auto_orient == AutoOrientMode::OrientAlongPath,
                                    "Orient Along Path",
                                )
                                .on_hover_text("Layer rotates to follow its position motion path")
                                .clicked()
                            {
                                layer.auto_orient = AutoOrientMode::OrientAlongPath;
                            }
                            let cur_target = match layer.auto_orient {
                                AutoOrientMode::OrientTowardsPoint { target_point } => target_point,
                                _ => [960.0, 540.0],
                            };
                            if ui
                                .selectable_label(
                                    matches!(
                                        layer.auto_orient,
                                        AutoOrientMode::OrientTowardsPoint { .. }
                                    ),
                                    "Orient Towards Point",
                                )
                                .on_hover_text("Layer rotates to face a fixed point")
                                .clicked()
                            {
                                layer.auto_orient = AutoOrientMode::OrientTowardsPoint {
                                    target_point: cur_target,
                                };
                            }
                        });
                });
                if let AutoOrientMode::OrientTowardsPoint { target_point } = &mut layer.auto_orient
                {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            egui::RichText::new("Target")
                                .small()
                                .color(colors::TEXT_SECONDARY),
                        );
                        ui.add(
                            egui::DragValue::new(&mut target_point[0])
                                .prefix("X: ")
                                .speed(1.0),
                        );
                        ui.add(
                            egui::DragValue::new(&mut target_point[1])
                                .prefix("Y: ")
                                .speed(1.0),
                        );
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
                "Opacity",
                &mut layer.transform.opacity,
                |ui, val| {
                    ui.add(egui::Slider::new(val, 0.0..=100.0).suffix("%"));
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
            egui::CollapsingHeader::new(egui::RichText::new("Expressions").size(12.0).color(
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
                ui.horizontal(|ui| {
                    ui.label("Position Z (Depth):");
                    if ui
                        .add(egui::DragValue::new(&mut pos3d[2]).speed(1.0).suffix(" px"))
                        .changed()
                    {
                        layer.transform_3d.position =
                            crate::core::property::Animatable::new_constant(pos3d);
                        *project_changed = true;
                    }
                });

                let mut rot3d = layer.transform_3d.rotation.evaluate(current_frame);
                ui.horizontal(|ui| {
                    ui.label("X Rotation:");
                    if ui
                        .add(egui::DragValue::new(&mut rot3d[0]).speed(1.0).suffix("°"))
                        .changed()
                    {
                        layer.transform_3d.rotation =
                            crate::core::property::Animatable::new_constant(rot3d);
                        *project_changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Y Rotation:");
                    if ui
                        .add(egui::DragValue::new(&mut rot3d[1]).speed(1.0).suffix("°"))
                        .changed()
                    {
                        layer.transform_3d.rotation =
                            crate::core::property::Animatable::new_constant(rot3d);
                        *project_changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Z Rotation:");
                    if ui
                        .add(egui::DragValue::new(&mut rot3d[2]).speed(1.0).suffix("°"))
                        .changed()
                    {
                        layer.transform_3d.rotation =
                            crate::core::property::Animatable::new_constant(rot3d);
                        *project_changed = true;
                    }
                });

                // ── 3D Material Options ──
                ui.separator();
                ui.label(
                    egui::RichText::new("Material Options")
                        .small()
                        .strong()
                        .color(colors::ACCENT_YELLOW),
                );

                ui.horizontal(|ui| {
                    ui.label("Ambient:");
                    if ui
                        .add(
                            egui::Slider::new(&mut layer.material.ambient, 0.0..=1.0).step_by(0.01),
                        )
                        .changed()
                    {
                        *project_changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Diffuse:");
                    if ui
                        .add(
                            egui::Slider::new(&mut layer.material.diffuse, 0.0..=1.0).step_by(0.01),
                        )
                        .changed()
                    {
                        *project_changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Specular:");
                    if ui
                        .add(
                            egui::Slider::new(&mut layer.material.specular, 0.0..=1.0)
                                .step_by(0.01),
                        )
                        .changed()
                    {
                        *project_changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Specular Exp:");
                    if ui
                        .add(
                            egui::DragValue::new(&mut layer.material.specular_exponent)
                                .speed(1.0)
                                .range(1.0..=256.0),
                        )
                        .changed()
                    {
                        *project_changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Emission:");
                    if ui
                        .add(
                            egui::Slider::new(&mut layer.material.emission, 0.0..=1.0)
                                .step_by(0.01),
                        )
                        .changed()
                    {
                        *project_changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Metalness:");
                    if ui
                        .add(
                            egui::Slider::new(&mut layer.material.metalness, 0.0..=1.0)
                                .step_by(0.01),
                        )
                        .changed()
                    {
                        *project_changed = true;
                    }
                });

                ui.horizontal(|ui| {
                    if ui
                        .checkbox(&mut layer.material.cast_shadows, "Casts Shadows")
                        .changed()
                    {
                        *project_changed = true;
                    }
                });

                ui.horizontal(|ui| {
                    if ui
                        .checkbox(&mut layer.material.accepts_shadows, "Accepts Shadows")
                        .changed()
                    {
                        *project_changed = true;
                    }
                    if ui
                        .checkbox(&mut layer.material.accepts_lights, "Accepts Lights")
                        .changed()
                    {
                        *project_changed = true;
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Light Transmission:");
                    if ui
                        .add(
                            egui::Slider::new(&mut layer.material.light_transmission, 0.0..=100.0)
                                .suffix("%"),
                        )
                        .changed()
                    {
                        *project_changed = true;
                    }
                });

                // ── 3D Geometry Options (Extrusion & Bevel) ──
                ui.separator();
                ui.collapsing("Geometry Options (Extrusion & Bevel)", |ui| {
                    if let LayerType::Shape {
                        extrusion_depth,
                        bevel_depth,
                        ..
                    } = &mut layer.layer_type
                    {
                        ui.horizontal(|ui| {
                            ui.label("Extrusion Depth:");
                            if ui
                                .add(
                                    egui::DragValue::new(extrusion_depth)
                                        .speed(1.0)
                                        .range(0.0..=500.0)
                                        .suffix(" px"),
                                )
                                .changed()
                            {
                                *project_changed = true;
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Bevel Depth:");
                            if ui
                                .add(
                                    egui::DragValue::new(bevel_depth)
                                        .speed(0.5)
                                        .range(0.0..=50.0)
                                        .suffix(" px"),
                                )
                                .changed()
                            {
                                *project_changed = true;
                            }
                        });
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

            ui.horizontal(|ui| {
                ui.label("Horizontal Pin:");
                egui::ComboBox::from_id_salt(format!("pin_h_{}", layer.id))
                    .selected_text(format!("{:?}", layer.constraints.horizontal))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut layer.constraints.horizontal,
                            HorizontalPin::Left,
                            "Left",
                        );
                        ui.selectable_value(
                            &mut layer.constraints.horizontal,
                            HorizontalPin::Center,
                            "Center",
                        );
                        ui.selectable_value(
                            &mut layer.constraints.horizontal,
                            HorizontalPin::Right,
                            "Right",
                        );
                        ui.selectable_value(
                            &mut layer.constraints.horizontal,
                            HorizontalPin::Scale,
                            "Scale",
                        );
                    });
            });

            ui.horizontal(|ui| {
                ui.label("Vertical Pin:");
                egui::ComboBox::from_id_salt(format!("pin_v_{}", layer.id))
                    .selected_text(format!("{:?}", layer.constraints.vertical))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut layer.constraints.vertical,
                            VerticalPin::Top,
                            "Top",
                        );
                        ui.selectable_value(
                            &mut layer.constraints.vertical,
                            VerticalPin::Center,
                            "Center",
                        );
                        ui.selectable_value(
                            &mut layer.constraints.vertical,
                            VerticalPin::Bottom,
                            "Bottom",
                        );
                        ui.selectable_value(
                            &mut layer.constraints.vertical,
                            VerticalPin::Scale,
                            "Scale",
                        );
                    });
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
                    let btn = ui.add(
                        egui::Button::new("  ")
                            .fill(c32)
                            .min_size(egui::vec2(18.0, 18.0)),
                    );
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
                if ui.checkbox(&mut ds.enabled, "Enabled").changed() {
                    *project_changed = true;
                }
                ui.horizontal(|ui| {
                    ui.label("Distance:");
                    if ui
                        .add(egui::Slider::new(&mut ds.distance, 0.0..=200.0).suffix(" px"))
                        .changed()
                    {
                        *project_changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Blur Size:");
                    if ui
                        .add(egui::Slider::new(&mut ds.size, 0.0..=100.0).suffix(" px"))
                        .changed()
                    {
                        *project_changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Opacity:");
                    if ui
                        .add(egui::Slider::new(&mut ds.opacity, 0.0..=1.0))
                        .changed()
                    {
                        *project_changed = true;
                    }
                });
            });

            let st = &mut layer.style.stroke;
            ui.collapsing("Stroke", |ui| {
                if ui.checkbox(&mut st.enabled, "Enabled").changed() {
                    *project_changed = true;
                }
                ui.horizontal(|ui| {
                    ui.label("Size:");
                    if ui
                        .add(egui::Slider::new(&mut st.size, 1.0..=50.0).suffix(" px"))
                        .changed()
                    {
                        *project_changed = true;
                    }
                });
            });
        });
    });
}

