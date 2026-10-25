use crate::core::mogrt_engine::{
    apply_essential_property_overrides, create_mogrt_manifest, EssentialProperty,
    EssentialPropertyType, MogrtPackage,
};
use crate::core::timeline::{LayerType, Project};
use crate::KagariApp;
use eframe::egui;

fn apply_property_edit(app: &mut KagariApp, index: usize, property_type: EssentialPropertyType) {
    if index >= app.mogrt_properties.len() {
        return;
    }
    app.mogrt_properties[index].property_type = property_type;
    let property = app.mogrt_properties[index].clone();
    app.modify_project(|project| {
        apply_essential_property_overrides(project.active_composition_mut(), &[property]);
    });
}

fn expose_selected_layer(app: &mut KagariApp) -> Option<String> {
    let selected = app.selection.selected_layer_idx?;
    let (layer_id, layer_name, property_type, target_path) = {
        let comp = app.history.current().active_composition();
        let layer = comp.layers.get(selected)?;
        let (property_type, target_path) = match &layer.layer_type {
            LayerType::Text { text, .. } => (
                EssentialPropertyType::Text { text: text.clone() },
                "text.content".to_string(),
            ),
            LayerType::Solid { color } => (
                EssentialPropertyType::Color { value: *color },
                "color".to_string(),
            ),
            _ => (
                EssentialPropertyType::Number {
                    min: 0.0,
                    max: 100.0,
                    value: layer.transform.opacity.evaluate(app.playback.current_frame),
                },
                "transform.opacity".to_string(),
            ),
        };
        (
            layer.id.clone(),
            layer.name.clone(),
            property_type,
            target_path,
        )
    };

    let id = format!("mogrt_property_{}", app.mogrt_properties.len() + 1);
    app.mogrt_properties.push(EssentialProperty {
        id,
        name: layer_name.clone(),
        comment: Some(format!("Exposed from {layer_name}")),
        target_layer_id: layer_id,
        target_property_path: target_path,
        property_type,
    });
    Some(layer_name)
}

fn draw_property_row(app: &mut KagariApp, ui: &mut egui::Ui, index: usize) -> bool {
    let Some(property) = app.mogrt_properties.get(index).cloned() else {
        return false;
    };
    let mut remove = false;
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(&property.name).strong());
            ui.weak(&property.target_property_path);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button("✕")
                    .on_hover_text("Remove exposed property")
                    .clicked()
                {
                    remove = true;
                }
            });
        });

        match property.property_type {
            EssentialPropertyType::Text { mut text } => {
                if ui.text_edit_singleline(&mut text).changed() {
                    apply_property_edit(app, index, EssentialPropertyType::Text { text });
                }
            }
            EssentialPropertyType::Number { min, max, value } => {
                let mut value = value;
                if ui
                    .add(egui::Slider::new(&mut value, min..=max).show_value(true))
                    .changed()
                {
                    apply_property_edit(
                        app,
                        index,
                        EssentialPropertyType::Number { min, max, value },
                    );
                }
            }
            EssentialPropertyType::Color { value } => {
                let mut color = egui::Color32::from_rgba_unmultiplied(
                    (value[0].clamp(0.0, 1.0) * 255.0) as u8,
                    (value[1].clamp(0.0, 1.0) * 255.0) as u8,
                    (value[2].clamp(0.0, 1.0) * 255.0) as u8,
                    (value[3].clamp(0.0, 1.0) * 255.0) as u8,
                );
                if ui.color_edit_button_srgba(&mut color).changed() {
                    let [r, g, b, a] = color.to_array();
                    apply_property_edit(
                        app,
                        index,
                        EssentialPropertyType::Color {
                            value: [
                                r as f32 / 255.0,
                                g as f32 / 255.0,
                                b as f32 / 255.0,
                                a as f32 / 255.0,
                            ],
                        },
                    );
                }
            }
            EssentialPropertyType::Checkbox { value } => {
                let mut value = value;
                if ui.checkbox(&mut value, "Enabled").changed() {
                    apply_property_edit(app, index, EssentialPropertyType::Checkbox { value });
                }
            }
            EssentialPropertyType::Point2D { value } => {
                let mut value = value;
                let mut changed = false;
                ui.horizontal(|ui| {
                    changed |= ui
                        .add(egui::DragValue::new(&mut value[0]).prefix("X "))
                        .changed();
                    changed |= ui
                        .add(egui::DragValue::new(&mut value[1]).prefix("Y "))
                        .changed();
                });
                if changed {
                    apply_property_edit(app, index, EssentialPropertyType::Point2D { value });
                }
            }
            EssentialPropertyType::Dropdown {
                options,
                selected_index,
            } => {
                let mut selected = selected_index.min(options.len().saturating_sub(1));
                egui::ComboBox::from_id_salt(("mogrt-dropdown", index))
                    .selected_text(options.get(selected).map(String::as_str).unwrap_or("-"))
                    .show_ui(ui, |ui| {
                        for (option_index, option) in options.iter().enumerate() {
                            if ui
                                .selectable_value(&mut selected, option_index, option)
                                .clicked()
                            {
                                apply_property_edit(
                                    app,
                                    index,
                                    EssentialPropertyType::Dropdown {
                                        options: options.clone(),
                                        selected_index: selected,
                                    },
                                );
                            }
                        }
                    });
            }
        }
    });
    remove
}

fn export_mogrt(app: &mut KagariApp) {
    let project = app.history.current();
    let comp = project.active_composition();
    let manifest = create_mogrt_manifest(
        comp,
        comp.name.clone(),
        "Kagari VFX",
        app.mogrt_properties.clone(),
    );
    let project_json = match serde_json::to_string(project) {
        Ok(json) => json,
        Err(error) => {
            app.toasts
                .error(format!("MOGRT project serialization failed: {error}"));
            return;
        }
    };
    let package = MogrtPackage::new(manifest, project_json);
    let Some(path) = rfd::FileDialog::new()
        .add_filter("Kagari Motion Graphics Template", &["mogrt"])
        .set_file_name(format!("{}.mogrt", comp.name.replace(['/', '\\'], "_")))
        .save_file()
    else {
        return;
    };
    match package
        .to_json()
        .map_err(|error| error.to_string())
        .and_then(|json| std::fs::write(&path, json).map_err(|error| error.to_string()))
    {
        Ok(()) => {
            crate::ui::project_io::reveal_in_file_manager(&path);
            app.toasts
                .info(format!("MOGRT exported: {}", path.display()));
        }
        Err(error) => app.toasts.error(format!("MOGRT export failed: {error}")),
    }
}

fn import_mogrt(app: &mut KagariApp, path: &std::path::Path) {
    let content =
        match crate::core::project_migration::read_bounded_text_file(path, 64 * 1024 * 1024) {
            Ok(content) => content,
            Err(error) => {
                app.toasts.error(error);
                return;
            }
        };
    let package = match MogrtPackage::from_json(&content) {
        Ok(package) => package,
        Err(error) => {
            app.toasts.error(format!("Invalid MOGRT package: {error}"));
            return;
        }
    };
    let imported = match serde_json::from_str::<Project>(&package.project_json) {
        Ok(project) => project,
        Err(error) => {
            app.toasts.error(format!("Invalid MOGRT project: {error}"));
            return;
        }
    };
    let mut merged = app.history.current().clone();
    let first_imported_comp = merged.compositions.len();
    let imported_comp_id = package.manifest.comp_id.clone();
    let Some(imported_comp_offset) = imported
        .compositions
        .iter()
        .position(|comp| comp.id == imported_comp_id)
    else {
        app.toasts.error(format!(
            "Invalid MOGRT package: composition '{}' is missing",
            imported_comp_id
        ));
        return;
    };
    merged.compositions.extend(imported.compositions);
    merged.assets.extend(imported.assets);
    let imported_index = first_imported_comp.saturating_add(imported_comp_offset);
    if let Some(comp) = merged.compositions.get_mut(imported_index) {
        apply_essential_property_overrides(comp, &package.manifest.essential_properties);
    }
    app.mogrt_properties = package.manifest.essential_properties;
    merged.active_composition_idx = imported_index;
    app.commit_project(merged);
    app.selection.selected_layer_idx = None;
    app.selection.selected_layers.clear();
    app.toasts
        .info(format!("MOGRT imported: {}", package.manifest.name));
}

pub fn draw_essential_graphics(app: &mut KagariApp, ui: &mut egui::Ui) {
    ui.heading("Essential Graphics (MOGRT Creator)");
    ui.separator();

    let comp_name = app.history.current().active_composition().name.clone();
    ui.label(format!("Master Composition: {comp_name}"));

    ui.horizontal(|ui| {
        if ui
            .button("Export Motion Graphics Template (.mogrt)")
            .on_hover_text("Export the current project and exposed properties as a MOGRT package")
            .clicked()
        {
            export_mogrt(app);
        }
        if ui.button("Import MOGRT...").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("MOGRT Template", &["mogrt", "json"])
                .pick_file()
            {
                import_mogrt(app, &path);
            }
        }
    });

    ui.add_space(8.0);
    ui.separator();
    ui.label(egui::RichText::new("Exposed Controllers & Parameters:").strong());

    let mut remove_idx = None;
    egui::ScrollArea::vertical()
        .max_height(300.0)
        .show(ui, |ui| {
            for index in 0..app.mogrt_properties.len() {
                if draw_property_row(app, ui, index) {
                    remove_idx = Some(index);
                }
            }
        });

    if let Some(index) = remove_idx {
        app.mogrt_properties.remove(index);
    }

    if app.mogrt_properties.is_empty() {
        ui.weak(
            "Expose a text, solid, or layer opacity property to make it editable in the template.",
        );
    }
    if ui
        .button("➕ Expose Active Layer Property to MOGRT")
        .clicked()
    {
        match expose_selected_layer(app) {
            Some(name) => app
                .toasts
                .info(format!("Exposed {name} to Essential Graphics")),
            None => app.toasts.error("Select a layer first"),
        }
    }
}
