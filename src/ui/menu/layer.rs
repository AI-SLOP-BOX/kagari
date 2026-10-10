//! Layer menu and actions (New, Transform, Time, Arrange, Create Shapes).

use eframe::egui;

pub fn draw_layer_menu(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    ui.menu_button("New", |ui| draw_layer_new_submenu(app, ui));
    ui.menu_button("Transform", |ui| draw_layer_transform_submenu(app, ui));
    ui.menu_button("Time", |ui| draw_layer_time_submenu(app, ui));
    ui.menu_button("Arrange", |ui| draw_layer_arrange_submenu(app, ui));
    ui.menu_button("Create", |ui| draw_layer_create_submenu(app, ui));
    ui.separator();
    if ui.button("Un-Solo All Layers").clicked() {
        app.modify_project(|p| {
            for l in p.active_composition_mut().layers.iter_mut() {
                l.solo = false;
            }
        });
        app.toasts.info("All layers un-soloed");
        ui.close_menu();
    }
    if ui.button("Unlock All Layers").clicked() {
        app.modify_project(|p| {
            for l in p.active_composition_mut().layers.iter_mut() {
                l.locked = false;
            }
        });
        app.toasts.info("All layers unlocked");
        ui.close_menu();
    }
    ui.separator();
    draw_create_shapes_from_text_full(app, ui);
    ui.separator();
    if ui
        .add(egui::Button::new("Pre-Compose...").shortcut_text(
            crate::ui::shortcuts::format_shortcut("C", true, true, false),
        ))
        .clicked()
    {
        ui.close_menu();
    }
}

fn draw_layer_new_submenu(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    if ui
        .add(
            egui::Button::new("Solid...").shortcut_text(crate::ui::shortcuts::format_shortcut(
                "Y", true, false, false,
            )),
        )
        .clicked()
    {
        crate::ui::commands::layer::create_new_layer(
            app,
            crate::ui::commands::layer::NewLayerKind::Solid,
        );
        ui.close_menu();
    }
    if ui
        .add(
            egui::Button::new("Text")
                .shortcut_text(crate::ui::shortcuts::format_shortcut("T", true, true, true)),
        )
        .clicked()
    {
        crate::ui::commands::layer::create_new_layer(
            app,
            crate::ui::commands::layer::NewLayerKind::Text,
        );
        ui.close_menu();
    }
    if ui
        .add(
            egui::Button::new("Null Object")
                .shortcut_text(crate::ui::shortcuts::format_shortcut("Y", true, true, true)),
        )
        .clicked()
    {
        crate::ui::commands::layer::create_new_layer(
            app,
            crate::ui::commands::layer::NewLayerKind::Null,
        );
        ui.close_menu();
    }
    if ui
        .add(egui::Button::new("Adjustment Layer").shortcut_text(
            crate::ui::shortcuts::format_shortcut("Y", true, false, true),
        ))
        .clicked()
    {
        crate::ui::commands::layer::create_new_layer(
            app,
            crate::ui::commands::layer::NewLayerKind::Adjustment,
        );
        ui.close_menu();
    }
    if ui
        .button("Light")
        .on_hover_text("Adds a 3D light to the composition")
        .clicked()
    {
        let comp = app.history.current().active_composition();
        let light = crate::core::timeline::Light3D {
            id: comp.next_light_id(),
            name: format!("Light {}", comp.lights.len() + 1),
            ..Default::default()
        };
        let name = light.name.clone();
        let light_id = light.id.clone();
        let light_position = light.position.clone();
        app.modify_project(|project| {
            let comp = project.active_composition_mut();
            comp.lights.push(light);
            let mut layer = crate::core::timeline::Layer::new(
                comp.next_layer_id("light"),
                name.clone(),
                crate::core::timeline::LayerType::Null,
                comp.duration_frames,
            );
            layer.is_3d = true;
            layer.transform_3d.position = light_position.clone();
            layer.scene_object = Some(crate::core::timeline::SceneObjectRef::Light {
                id: light_id.clone(),
            });
            comp.add_layer(layer);
        });
        app.toasts.info(format!("Added {}", name));
        ui.close_menu();
    }
}

fn draw_layer_transform_submenu(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    if ui
        .button("Center in Comp")
        .on_hover_text("Move position to the composition center")
        .clicked()
    {
        if let Some(idx) = app.selection.selected_layer_idx {
            let (cw, ch) = {
                let c = app.history.current().active_composition();
                (c.width as f32, c.height as f32)
            };
            app.modify_project(move |p| {
                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                    l.transform.position =
                        crate::core::property::Animatable::new_constant([cw / 2.0, ch / 2.0]);
                }
            });
            app.toasts.info("Centered in Comp");
            ui.close_menu();
        }
    }
    if ui
        .button("Fit to Comp")
        .on_hover_text("Scale so the layer covers the full comp frame")
        .clicked()
    {
        if let Some(idx) = app.selection.selected_layer_idx {
            let (cw, ch) = {
                let c = app.history.current().active_composition();
                (c.width as f32, c.height as f32)
            };
            app.modify_project(move |p| {
                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                    let bs = l.bounding_size();
                    if bs[0] > 1.0 && bs[1] > 1.0 {
                        let s = (cw / bs[0] * 100.0).max(ch / bs[1] * 100.0);
                        l.transform.scale = crate::core::property::Animatable::new_constant([s, s]);
                        l.transform.position =
                            crate::core::property::Animatable::new_constant([cw / 2.0, ch / 2.0]);
                    }
                }
            });
            app.toasts.info("Fit to Comp");
            ui.close_menu();
        }
    }
    if ui.button("Flip Horizontal").clicked() {
        if let Some(idx) = app.selection.selected_layer_idx {
            let cf = app.playback.current_frame;
            app.modify_project(move |p| {
                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                    let s = l.transform.scale.evaluate(cf);
                    l.transform.scale =
                        crate::core::property::Animatable::new_constant([-s[0], s[1]]);
                }
            });
            app.toasts.info("Flipped Horizontal");
            ui.close_menu();
        }
    }
    if ui.button("Flip Vertical").clicked() {
        if let Some(idx) = app.selection.selected_layer_idx {
            let cf = app.playback.current_frame;
            app.modify_project(move |p| {
                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                    let s = l.transform.scale.evaluate(cf);
                    l.transform.scale =
                        crate::core::property::Animatable::new_constant([s[0], -s[1]]);
                }
            });
            app.toasts.info("Flipped Vertical");
            ui.close_menu();
        }
    }
    ui.separator();
    if ui.button("Reset All Transforms").clicked() {
        if let Some(idx) = app.selection.selected_layer_idx {
            let dims = {
                let c = app.history.current().active_composition();
                (c.width as f32, c.height as f32)
            };
            app.modify_project(move |p| {
                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                    l.transform.position = crate::core::property::Animatable::new_constant([
                        dims.0 / 2.0,
                        dims.1 / 2.0,
                    ]);
                    l.transform.scale =
                        crate::core::property::Animatable::new_constant([100.0, 100.0]);
                    l.transform.rotation = crate::core::property::Animatable::new_constant(0.0);
                    l.transform.opacity = crate::core::property::Animatable::new_constant(100.0);
                }
            });
            app.toasts.info("All transforms reset");
            ui.close_menu();
        }
    }
    ui.separator();
    if ui
        .add(egui::Button::new("Auto-Orient...").shortcut_text(
            crate::ui::shortcuts::format_shortcut("O", true, false, true),
        ))
        .on_hover_text("Rotate layer automatically along its motion path direction")
        .clicked()
    {
        if let Some(idx) = app.selection.selected_layer_idx {
            app.modify_project(move |p| {
                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                    l.auto_orient = match l.auto_orient {
                        crate::core::auto_orient::AutoOrientMode::Off => {
                            crate::core::auto_orient::AutoOrientMode::OrientAlongPath
                        }
                        _ => crate::core::auto_orient::AutoOrientMode::Off,
                    };
                }
            });
            app.toasts.info("Toggled Auto-Orient along Motion Path");
        }
        ui.close_menu();
    }
}

fn draw_layer_time_submenu(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    if ui
        .add(egui::Button::new("Enable Time Remapping").shortcut_text(
            crate::ui::shortcuts::format_shortcut("T", true, false, true),
        ))
        .clicked()
    {
        if let Some(idx) = app.selection.selected_layer_idx {
            app.modify_project(move |p| {
                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                    l.enable_time_remapping();
                }
            });
            app.toasts
                .info("Time remapping enabled — edit keyframes in the Graph Editor");
        } else {
            app.toasts.info("Select a layer first");
        }
        ui.close_menu();
    }
    if ui.add(egui::Button::new("Time-Reverse Layer")).clicked() {
        if let Some(idx) = app.selection.selected_layer_idx {
            app.modify_project(move |p| {
                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                    l.time_reverse();
                }
            });
            app.toasts.info("Layer time-reversed");
        } else {
            app.toasts.info("Select a layer first");
        }
        ui.close_menu();
    }
    if ui.button("Freeze Frame at Playhead").clicked() {
        let frame = app.playback.current_frame;
        if let Some(idx) = app.selection.selected_layer_idx {
            app.modify_project(move |p| {
                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                    l.freeze_at(frame);
                }
            });
            app.toasts.info(format!("Frozen at source frame {}", frame));
        } else {
            app.toasts.info("Select a layer first");
        }
        ui.close_menu();
    }
    if ui.button("Remove Time Remapping").clicked() {
        if let Some(idx) = app.selection.selected_layer_idx {
            app.modify_project(move |p| {
                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                    l.clear_time_remap();
                }
            });
            app.toasts.info("Time remapping removed");
        } else {
            app.toasts.info("Select a layer first");
        }
        ui.close_menu();
    }
    ui.separator();
    if ui
        .add(egui::Button::new("Time Stretch ×2 (Slow)"))
        .clicked()
    {
        if let Some(idx) = app.selection.selected_layer_idx {
            app.modify_project(move |p| {
                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                    l.time_stretch(2.0);
                }
            });
            app.toasts.info("Layer stretched to ×2 duration");
        } else {
            app.toasts.info("Select a layer first");
        }
        ui.close_menu();
    }
    if ui
        .add(egui::Button::new("Time Stretch ×0.5 (Fast)"))
        .clicked()
    {
        if let Some(idx) = app.selection.selected_layer_idx {
            app.modify_project(move |p| {
                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                    l.time_stretch(0.5);
                }
            });
            app.toasts.info("Layer compressed to ×0.5 duration");
        } else {
            app.toasts.info("Select a layer first");
        }
        ui.close_menu();
    }
}

fn draw_layer_arrange_submenu(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    let len = app.history.current().active_composition().layers.len();
    let Some(i) = app.selection.selected_layer_idx else {
        ui.label("Select a layer first");
        return;
    };
    if ui.button("Bring to Front").clicked() {
        app.modify_project(move |p| {
            let comp = p.active_composition_mut();
            if i < comp.layers.len() && comp.layers.len() > 1 {
                let l = comp.layers.remove(i);
                comp.layers.push(l);
            }
        });
        app.selection.selected_layer_idx = if i < len {
            Some(len - 1)
        } else {
            app.selection.selected_layer_idx
        };
        ui.close_menu();
    }
    if ui
        .add(egui::Button::new("Bring Forward").shortcut_text(
            crate::ui::shortcuts::format_shortcut("]", true, false, false),
        ))
        .clicked()
    {
        if i + 1 < len {
            app.modify_project(move |p| {
                let comp = p.active_composition_mut();
                if i + 1 < comp.layers.len() {
                    comp.layers.swap(i, i + 1);
                }
            });
            app.selection.selected_layer_idx = Some(i + 1);
        }
        ui.close_menu();
    }
    if ui
        .add(egui::Button::new("Send Backward").shortcut_text(
            crate::ui::shortcuts::format_shortcut("[", true, false, false),
        ))
        .clicked()
    {
        if i > 0 {
            app.modify_project(move |p| {
                let comp = p.active_composition_mut();
                if i < comp.layers.len() {
                    comp.layers.swap(i - 1, i);
                }
            });
            app.selection.selected_layer_idx = Some(i - 1);
        }
        ui.close_menu();
    }
    if ui.button("Send to Back").clicked() {
        app.modify_project(move |p| {
            let comp = p.active_composition_mut();
            if i < comp.layers.len() && comp.layers.len() > 1 {
                let l = comp.layers.remove(i);
                comp.layers.insert(0, l);
            }
        });
        app.selection.selected_layer_idx = if i < len {
            Some(0)
        } else {
            app.selection.selected_layer_idx
        };
        ui.close_menu();
    }
}

fn draw_layer_create_submenu(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    if ui
        .button("Create Shapes from Text")
        .on_hover_text("Convert text characters into editable vector shape layer")
        .clicked()
    {
        if let Some(idx) = app.selection.selected_layer_idx {
            let mut temp_proj = app.history.current().clone();
            let comp = temp_proj.active_composition_mut();
            if let Some(layer) = comp.layers.get(idx) {
                if let crate::core::timeline::LayerType::Text {
                    text,
                    font_size,
                    color,
                    ..
                } = &layer.layer_type
                {
                    let total_frames = comp.duration_frames;
                    let text_str = text.clone();
                    let font_sz = *font_size;
                    let col = *color;
                    let mut shape_layer = crate::core::timeline::Layer::new(
                        comp.next_layer_id("shape_from_text"),
                        format!("{} Outlines", layer.name),
                        crate::core::timeline::LayerType::Shape {
                            shape_type: crate::core::timeline::ShapeType::Rectangle {
                                width: crate::core::property::Animatable::new_constant(
                                    font_sz as f32 * text_str.len() as f32 * 0.6,
                                ),
                                height: crate::core::property::Animatable::new_constant(
                                    font_sz as f32,
                                ),
                                corner_radius: crate::core::property::Animatable::new_constant(0.0),
                            },
                            color: col,
                            stroke_color: [0.0, 0.0, 0.0, 0.0],
                            stroke_width: 0.0,
                            fill_type: crate::core::timeline::ShapeFillType::Solid,
                            extrusion_depth: 0.0,
                            bevel_depth: 0.0,
                        },
                        total_frames,
                    );
                    shape_layer.transform = layer.transform.clone();
                    comp.layers.insert(idx + 1, shape_layer);
                    app.commit_project(temp_proj);
                    app.toasts.info("Converted Text to Vector Shape Layer");
                } else {
                    app.toasts.error("Selected layer is not a Text layer");
                }
            }
        }
        ui.close_menu();
    }
}

fn draw_create_shapes_from_text_full(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    if ui
        .button("Create Shapes from Text")
        .on_hover_text("Decompose selected Text layer into animatable vector Bezier Shape paths")
        .clicked()
    {
        let mut created = false;
        let selected_idx = app.selection.selected_layer_idx;
        app.modify_project(|p| {
            let comp = p.active_composition_mut();
            if let Some(idx) = selected_idx {
                if idx < comp.layers.len() {
                    if let Some(shape_layer) = crate::core::text_to_shapes::convert_text_to_shapes(
                        &comp.layers[idx],
                        comp.width,
                        comp.height,
                    ) {
                        comp.layers.insert(idx, shape_layer);
                        created = true;
                    }
                }
            }
        });
        if created {
            app.toasts.info("Created Shapes from Text layer");
        } else {
            app.toasts.warning("Please select a Text layer first");
        }
        ui.close_menu();
    }
}
