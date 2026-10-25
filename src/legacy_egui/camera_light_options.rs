use crate::core::timeline::LightType;
use crate::ui::kui;
use crate::KagariApp;
use eframe::egui;

fn add_camera_with_layer(comp: &mut crate::core::timeline::Composition) {
    let camera = crate::core::timeline::Camera3D {
        id: comp.next_camera_id(),
        name: format!("Camera {}", comp.cameras.len() + 1),
        active: false,
        ..Default::default()
    };
    let camera_id = camera.id.clone();
    let camera_name = camera.name.clone();
    let camera_transform = camera.transform.clone();
    comp.cameras.push(camera);

    let mut layer = crate::core::timeline::Layer::new(
        comp.next_layer_id("camera"),
        camera_name,
        crate::core::timeline::LayerType::Null,
        comp.duration_frames,
    );
    layer.is_3d = true;
    layer.transform_3d = camera_transform;
    layer.scene_object = Some(crate::core::timeline::SceneObjectRef::Camera { id: camera_id });
    comp.add_layer(layer);
}

fn add_light_with_layer(comp: &mut crate::core::timeline::Composition) {
    let light = crate::core::timeline::Light3D {
        id: comp.next_light_id(),
        name: format!("Light {}", comp.lights.len() + 1),
        ..Default::default()
    };
    let light_id = light.id.clone();
    let light_name = light.name.clone();
    let light_position = light.position.clone();
    comp.lights.push(light);

    let mut layer = crate::core::timeline::Layer::new(
        comp.next_layer_id("light"),
        light_name,
        crate::core::timeline::LayerType::Null,
        comp.duration_frames,
    );
    layer.is_3d = true;
    layer.transform_3d.position = light_position;
    layer.scene_object = Some(crate::core::timeline::SceneObjectRef::Light { id: light_id });
    comp.add_layer(layer);
}

pub fn draw_camera_light_options(app: &mut KagariApp, ui: &mut egui::Ui) {
    let current_frame = app.playback.current_frame;
    app.ensure_panel_edit_snapshot("camera_light_options", ui);
    let comp = app.history.current_mut().active_composition_mut();
    let mut changed = false;

    egui::ScrollArea::vertical().show(ui, |ui| {
        // ── 3D Camera Options ──
        kui::section(ui, "3D Camera");
        let grid = kui::PropertyGrid::new(ui, &["FOV", "Focus Distance", "Aperture", "Enable DOF", "Max Blur", "Iris Sides"]);

        // Multi-camera switching: list scene cameras; active one wins.
        if !comp.cameras.is_empty() {
            ui.horizontal_wrapped(|ui| {
                kui::section_title(ui, "Cameras");
                if kui::button(ui, "+ Add", kui::ButtonKind::Default)
                    .on_hover_text("Add another camera to the scene")
                    .clicked()
                {
                    add_camera_with_layer(comp);
                    changed = true;
                }
            });
            let mut activate_idx: Option<usize> = None;
            let mut remove_idx: Option<usize> = None;
            for (i, c) in comp.cameras.iter().enumerate() {
                ui.horizontal_wrapped(|ui| {
                    if kui::tool_button(ui, c.active, &c.name).clicked() && !c.active {
                        activate_idx = Some(i);
                    }
                    if kui::button(ui, "×", kui::ButtonKind::Quiet)
                        .on_hover_text("Remove this camera")
                        .clicked()
                    {
                        remove_idx = Some(i);
                    }
                });
            }
            if let Some(i) = activate_idx {
                comp.set_active_camera(Some(i));
                changed = true;
            }
            if let Some(i) = remove_idx {
                let camera_id = comp.cameras.get(i).map(|camera| camera.id.clone());
                if let Some(camera_id) = camera_id {
                    if let Some(layer_idx) = comp.layers.iter().position(|layer| {
                        matches!(
                            layer.scene_object.as_ref(),
                            Some(crate::core::timeline::SceneObjectRef::Camera { id }) if id == &camera_id
                        )
                    }) {
                        let _ = comp.remove_layer_at(layer_idx);
                    } else {
                        comp.cameras.remove(i);
                    }
                }
                changed = true;
            }
            kui::separator(ui);
        }
        let cam = comp.resolve_camera_mut();

        grid.row(ui, "FOV", |ui| {
                let mut value = cam.fov_at(current_frame);
                if kui::number(ui, &mut value, 1.0..=179.0, 1.0, "°")
                    .changed()
                {
                    cam.set_fov_at(current_frame, value);
                    changed = true;
                }
        });

        grid.row(ui, "Focus Distance", |ui| {
                let mut value = cam.focus_distance_at(current_frame);
                if kui::number(ui, &mut value, 0.0..=100000.0, 1.0, " px")
                    .changed()
                {
                    cam.set_focus_distance_at(current_frame, value);
                    changed = true;
                }
        });

        grid.row(ui, "Aperture", |ui| {
                let mut value = cam.aperture_at(current_frame);
                if kui::number(ui, &mut value, 0.0..=500.0, 0.1, " px")
                    .changed()
                {
                    cam.set_aperture_at(current_frame, value);
                    changed = true;
                }
        });

        ui.add_space(kui::space::XS);
        kui::section(ui, "Depth of Field");

        let mut dof_enabled = cam.dof_enabled_at(current_frame);
        if grid.row(ui, "Enable DOF", |ui| kui::toggle(ui, &mut dof_enabled, "")).changed() {
            cam.set_dof_enabled_at(current_frame, dof_enabled);
            changed = true;
        }

        if cam.dof_enabled_at(current_frame) {
            grid.row(ui, "Max Blur", |ui| {
                ui.vertical(|ui| {
                let mut value = cam.dof_max_blur_at(current_frame);
                let changed_value = kui::number(ui, &mut value, 1.0..=64.0, 0.5, " px").changed();
                let (insert_key, remove_key) = ui.horizontal_wrapped(|ui| {
                    let insert = kui::button(ui, "Add key", kui::ButtonKind::Default).on_hover_text("Insert a keyframe at the current frame").clicked();
                    let remove = kui::button(ui, "Remove key", kui::ButtonKind::Default).on_hover_text("Remove the keyframe at the current frame").clicked();
                    (insert, remove)
                }).inner;
                if remove_key {
                    if let Some(crate::core::property::Animatable::Animated(keys)) = &mut cam.dof_max_blur_animation {
                        keys.retain(|key| key.frame != current_frame);
                        if keys.is_empty() { cam.dof_max_blur_animation = None; }
                    }
                    changed = true;
                } else if changed_value || insert_key {
                    cam.set_dof_max_blur_at(current_frame, value);
                    changed = true;
                }
                });
            });

            changed |= grid.row(ui, "Iris Sides", |ui| {
                kui::combo(ui, "iris_sides", &mut cam.dof_iris_sides, &[
                    (0, "Circle"), (3, "Triangle"), (5, "Pentagon"),
                    (6, "Hexagon"), (8, "Octagon"),
                ])
            });
        }

        // ── 3D Light Options ──
        ui.add_space(kui::space::SM);
        kui::section(ui, "3D Lights");

        let light_count = comp.lights.len();
        if light_count == 0 {
            kui::secondary(ui, "No lights in composition.");
            if kui::button(ui, "+ Add Light", kui::ButtonKind::Accent).clicked() {
                add_light_with_layer(comp);
                changed = true;
            }
        } else {
            for (li, light) in comp.lights.iter_mut().enumerate() {
                kui::collapsing_section(ui, ("camera_light", li), &format!("{} {}", light.name, li + 1), false, |ui| {
                    let grid = kui::PropertyGrid::new(ui, &["Name", "Type", "Cone Angle", "Cone Feather", "Intensity", "Color", "Casts Shadows"]);
                    changed |= grid.row(ui, "Name", |ui| kui::text_field(ui, &mut light.name)).changed();
                    grid.row(ui, "Type", |ui| {
                        let mut type_idx = match light.light_type {
                            LightType::Ambient => 0, LightType::Point => 1,
                            LightType::Spot { .. } => 2, LightType::Parallel => 3,
                        };
                        if kui::combo(ui, ("light_type", li), &mut type_idx, &[(0, "Ambient"), (1, "Point"), (2, "Spot"), (3, "Parallel")]) {
                            light.light_type = match type_idx {
                                0 => LightType::Ambient, 1 => LightType::Point,
                                2 => LightType::Spot { cone_angle_deg: 90.0, cone_feather_pct: 50.0 },
                                _ => LightType::Parallel,
                            };
                            changed = true;
                        }
                    });
                    if let LightType::Spot { ref mut cone_angle_deg, ref mut cone_feather_pct } = light.light_type {
                        changed |= grid.row(ui, "Cone Angle", |ui| kui::slider(ui, cone_angle_deg, 0.0..=180.0, "°")).changed();
                        changed |= grid.row(ui, "Cone Feather", |ui| kui::slider(ui, cone_feather_pct, 0.0..=100.0, "%")).changed();
                    }
                    changed |= grid.row(ui, "Intensity", |ui| kui::slider(ui, &mut light.intensity, 0.0..=500.0, "%")).changed();
                    grid.row(ui, "Color", |ui| {
                        let c = &mut light.color;
                        let mut col = egui::Color32::from_rgba_premultiplied(
                            (c[0] * 255.0) as u8, (c[1] * 255.0) as u8,
                            (c[2] * 255.0) as u8, (c[3] * 255.0) as u8,
                        );
                        if kui::color(ui, &mut col).changed() {
                            let [r, g, b, a] = col.to_array();
                            *c = [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0];
                            changed = true;
                        }
                    });
                    changed |= grid.row(ui, "Casts Shadows", |ui| kui::toggle(ui, &mut light.casts_shadows, "")).changed();
                });
            }

            ui.add_space(kui::space::XS);
            if kui::button(ui, "+ Add Light", kui::ButtonKind::Default).clicked() {
                add_light_with_layer(comp);
                changed = true;
            }
        }
    });

    if changed {
        let pointer_down = ui.input(|input| input.pointer.any_down());
        if pointer_down {
            if !app.drag_active() {
                if let Some(snapshot) = app.take_panel_edit_snapshot("camera_light_options") {
                    app.begin_drag_with_snapshot(snapshot, "Camera and Light Edit");
                }
            }
        } else if app.drag_active() {
            app.commit_drag();
        } else if let Some(snapshot) = app.take_panel_edit_snapshot("camera_light_options") {
            app.begin_drag_with_snapshot(snapshot, "Camera and Light Edit");
            app.commit_drag();
        }
    }
}
