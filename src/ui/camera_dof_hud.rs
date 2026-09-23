use crate::KagariApp;
use eframe::egui;

pub fn draw_camera_dof_hud(app: &mut KagariApp, ui: &mut egui::Ui) {
    let mut project_changed = false;
    let current_f = app.playback.current_frame;
    let pre_snapshot = if !app.drag_active() {
        Some(app.history.current().clone())
    } else {
        None
    };

    {
        ui.horizontal(|ui| {
            ui.style_mut().spacing.item_spacing.x = 4.0;
            ui.small("3D DoF:");

            let selected_idx = app.selection.selected_layer_idx;

            // 1. Auto-Focus to Selected Layer Button
            if ui
                .button("Focus to Layer")
                .on_hover_text("Auto-calculate Focus Distance to selected 3D Layer")
                .clicked()
            {
                let target_info = if let Some(idx) = selected_idx {
                    let comp = app.history.current().active_composition();
                    if idx < comp.layers.len() {
                        let target_pos = if comp.layers[idx].is_3d {
                            comp.layers[idx].transform_3d.position.evaluate(current_f)
                        } else {
                            let pos = comp.layers[idx].transform.position.evaluate(current_f);
                            [pos[0], pos[1], 0.0]
                        };
                        let camera_pos =
                            comp.resolve_camera().transform.position.evaluate(current_f);
                        let distance = ((target_pos[0] - camera_pos[0]).powi(2)
                            + (target_pos[1] - camera_pos[1]).powi(2)
                            + (target_pos[2] - camera_pos[2]).powi(2))
                        .sqrt()
                        .max(10.0);
                        Some((comp.layers[idx].name.clone(), distance))
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some((layer_name, distance)) = target_info {
                    let temp_proj = app.history.current_mut();
                    let comp_mut = temp_proj.active_composition_mut();
                    let cam = comp_mut.resolve_camera_mut();
                    cam.focus_distance = distance;
                    cam.dof_enabled = true;
                    project_changed = true;
                    app.toasts.info(format!(
                        "Focused 3D Camera to '{}' ({:.0}px)",
                        layer_name, distance
                    ));
                }
            }

            // 2. DoF Enabled Checkbox
            let comp = app.history.current_mut().active_composition_mut();
            let cam = comp.resolve_camera_mut();
            if ui.checkbox(&mut cam.dof_enabled, "DoF").changed() {
                project_changed = true;
            }

            ui.label(egui::RichText::new("Aperture:").small());
            if ui
                .add(
                    egui::Slider::new(&mut cam.aperture, 0.0..=150.0)
                        .suffix(" px")
                        .show_value(true),
                )
                .changed()
            {
                project_changed = true;
            }

            ui.label(egui::RichText::new("Iris:").small());
            egui::ComboBox::from_id_salt("bokeh_shape_combo")
                .selected_text(match cam.dof_iris_sides {
                    3 => "3-Blade (Triangle)",
                    5 => "5-Blade (Pentagon)",
                    6 => "6-Blade (Hexagon)",
                    8 => "8-Blade (Octagon)",
                    _ => "Round (Circular)",
                })
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_value(&mut cam.dof_iris_sides, 0, "Round (Circular)")
                        .clicked()
                    {
                        project_changed = true;
                    }
                    if ui
                        .selectable_value(&mut cam.dof_iris_sides, 5, "5-Blade (Pentagon)")
                        .clicked()
                    {
                        project_changed = true;
                    }
                    if ui
                        .selectable_value(&mut cam.dof_iris_sides, 6, "6-Blade (Hexagon)")
                        .clicked()
                    {
                        project_changed = true;
                    }
                    if ui
                        .selectable_value(&mut cam.dof_iris_sides, 8, "8-Blade (Octagon)")
                        .clicked()
                    {
                        project_changed = true;
                    }
                });

            ui.label(egui::RichText::new("Focus:").small());
            if ui
                .add(
                    egui::DragValue::new(&mut cam.focus_distance)
                        .range(10.0..=10000.0)
                        .speed(5.0)
                        .suffix(" px"),
                )
                .changed()
            {
                project_changed = true;
            }
        });
    }

    if project_changed {
        let pointer_down = ui.input(|input| input.pointer.any_down());
        if pointer_down {
            if !app.drag_active() {
                if let Some(snapshot) = pre_snapshot {
                    app.begin_drag_with_snapshot(snapshot, "Camera Depth of Field Edit");
                }
            }
        } else if app.drag_active() {
            app.commit_drag();
        } else if let Some(snapshot) = pre_snapshot {
            app.begin_drag_with_snapshot(snapshot, "Camera Depth of Field Edit");
            app.commit_drag();
        }
    }
}
