use crate::core::editor::EditorSession;
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;
use super::color_controls::{color_control_value, set_color_control};

pub(crate) fn draw_effect_controls(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    current_frame: u32,
    next_frame: &mut Option<u32>,
    slider_changed: &mut bool,
) {
    let Some(idx) = app.selection.selected_layer_idx else {
        ui.weak(crate::ui::localization::tr("No layer selected"));
        return;
    };

    // Immutable read for layer name and effect count
    let layer_info = app
        .history
        .current()
        .active_composition()
        .layers
        .get(idx)
        .map(|l| (l.name.clone(), l.effects.len(), l.id.clone()));

    let Some((layer_name, _effect_count, _layer_id)) = layer_info else {
        ui.weak(crate::ui::localization::tr("Select a layer."));
        return;
    };

    let drag_info = app.dragging_effect.clone();
    ui.horizontal(|ui| {
        crate::ui::icons::render_svg_bytes(
            ui,
            "properties-layer-file",
            crate::ui::icons::SVG_FILE,
            egui::vec2(16.0, 16.0),
            colors::TEXT_SECONDARY,
        );
        ui.label(egui::RichText::new(&layer_name).color(colors::TEXT_PRIMARY));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let _ =
                ui.add(egui::Button::new(crate::ui::localization::tr("Open in Viewer")).small());
        });
    });
    ui.add_space(8.0);

    // ── Layer Transform (same controls as the inspector; separate egui ID
    // scope so both panels can stay open on the same layer) ──
    ui.push_id("right_props_transform", |ui| {
        let fps = app.history.current().active_composition().fps;
        let mut changed = false;
        let mut nf = None;
        {
            let temp_project = app.history.current_mut();
            let comp = temp_project.active_composition_mut();
            if let Some(layer) = comp.layers.get_mut(idx) {
                crate::ui::inspector_layer::draw_layer_transforms(
                    ui,
                    layer,
                    current_frame,
                    fps,
                    &mut changed,
                    &mut nf,
                );
            }
        }
        if changed {
            *slider_changed = true;
        }
        if nf.is_some() {
            *next_frame = nf;
        }
    });
    ui.add_space(4.0);
    egui::CollapsingHeader::new("Color")
        .default_open(false)
        .show(ui, |ui| {
            for (key, label, range) in [
                ("exposure", "Exposure", -2.0_f32..=2.0),
                ("contrast", "Contrast", -1.0_f32..=1.0),
                ("highlights", "Highlights", -1.0_f32..=1.0),
                ("shadows", "Shadows", -1.0_f32..=1.0),
                ("saturation", "Saturation", 0.0_f32..=2.0),
            ] {
                let Some(mut value) = app
                    .history
                    .current()
                    .active_composition()
                    .layers
                    .get(idx)
                    .map(|layer| color_control_value(layer, key, current_frame))
                else {
                    continue;
                };
                let mut changed = false;
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(label).color(colors::TEXT_SECONDARY));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!("{value:.2}")).color(colors::ACCENT_BLUE),
                        );
                        changed = ui
                            .add_sized(
                                [110.0, 16.0],
                                egui::Slider::new(&mut value, range).show_value(false),
                            )
                            .changed();
                    });
                });
                if changed {
                    if let Some(layer) = app
                        .history
                        .current_mut()
                        .active_composition_mut()
                        .layers
                        .get_mut(idx)
                    {
                        set_color_control(layer, key, current_frame, value);
                    }
                    *slider_changed = true;
                }
            }
        });
    ui.add_space(4.0);
    ui.heading("Effects");
    ui.separator();

    // ── Drop zone for effects ──
    if let Some((ref effect_name, _)) = drag_info {
        let drop_rect = ui
            .allocate_ui_with_layout(
                egui::vec2(ui.available_width(), 40.0),
                egui::Layout::top_down(egui::Align::Center),
                |ui| {
                    ui.label(
                        egui::RichText::new(format!("Drop '{}' here", effect_name))
                            .small()
                            .color(colors::TEXT_ACCENT),
                    );
                },
            )
            .response;
        ui.painter()
            .rect_filled(drop_rect.rect, 4.0, colors::TIMELINE_SELECTION);
        ui.painter().rect_stroke(
            drop_rect.rect,
            4.0,
            egui::Stroke::new(1.5_f32, colors::TEXT_ACCENT),
        );
        let is_hovered = ui.rect_contains_pointer(drop_rect.rect);
        if is_hovered && ui.input(|i| i.pointer.any_released()) {
            let presets = crate::ui::effects_controls::get_all_effect_presets();
            if let Some(preset) = presets.iter().find(|p| p.name == effect_name) {
                let mut session = EditorSession::new(&mut app.history, "Drop Effect");
                let comp = session.current_mut().active_composition_mut();
                if idx < comp.layers.len() {
                    let effect = (preset.create_fn)(comp.layers[idx].effects.len());
                    comp.layers[idx].effects.push(effect);
                    session.commit();
                    app.toasts
                        .info(format!("Applied '{}' to '{}'", effect_name, layer_name));
                }
            }
            app.dragging_effect = None;
        }
    }

    // ── Effect list with discrete actions via EditorSession ──
    let effect_count = app
        .history
        .current()
        .active_composition()
        .layers
        .get(idx)
        .map_or(0, |l| l.effects.len());

    if effect_count == 0 && drag_info.is_none() {
        ui.weak("No effects applied. Drag an effect from 'Effects & Presets' tab.");
    } else {
        // Collect discrete actions during rendering, apply after.
        let mut fx_move_up: Option<usize> = None;
        let mut fx_move_down: Option<usize> = None;
        let mut fx_dup: Option<usize> = None;
        let mut fx_del: Option<usize> = None;
        let mut fx_reset: Option<usize> = None;
        let mut fx_toggle: Option<(usize, bool)> = None;

        // Render effect list in a block scope so the mutable borrow on
        // app.history is released before we create EditorSessions below.
        {
            let temp_project = app.history.current_mut();
            let comp = temp_project.active_composition_mut();

            if idx < comp.layers.len() {
                let layer = &mut comp.layers[idx];
                let total_effects = layer.effects.len();

                for (e_idx, fx) in layer.effects.iter_mut().enumerate() {
                    let effect_row =
                        ui.collapsing(format!("fx {} - {}", e_idx + 1, fx.name), |ui| {
                            ui.horizontal(|ui| {
                                let was_enabled = fx.enabled;
                                let mut next_enabled = was_enabled;
                                if ui.checkbox(&mut next_enabled, "Enabled (fx)").changed()
                                    && next_enabled != was_enabled
                                {
                                    fx_toggle = Some((e_idx, next_enabled));
                                }
                                if e_idx > 0
                                    && ui
                                        .small_button("▲")
                                        .on_hover_text("Move effect up in render order")
                                        .clicked()
                                {
                                    fx_move_up = Some(e_idx);
                                }
                                if e_idx + 1 < total_effects
                                    && ui
                                        .small_button("▼")
                                        .on_hover_text("Move effect down in render order")
                                        .clicked()
                                {
                                    fx_move_down = Some(e_idx);
                                }
                                if ui
                                    .small_button("Dup")
                                    .on_hover_text("Duplicate effect")
                                    .clicked()
                                {
                                    fx_dup = Some(e_idx);
                                }
                                if ui
                                    .small_button("Del")
                                    .on_hover_text("Remove effect")
                                    .clicked()
                                {
                                    fx_del = Some(e_idx);
                                }
                                if ui
                                    .small_button("Reset")
                                    .on_hover_text("Reset effect to factory defaults")
                                    .clicked()
                                {
                                    fx_reset = Some(e_idx);
                                }
                            });

                            if ui.small_button("Save as Preset").clicked() {
                                let preset = crate::core::effect_presets::EffectPreset::from_effect(
                                    fx,
                                    fx.name.clone(),
                                );
                                let preset_dir = crate::core::effect_presets::default_preset_dir();
                                let _ = std::fs::create_dir_all(&preset_dir);
                                let filename = format!(
                                    "{}.kagari-preset.json",
                                    fx.name.replace(' ', "_").to_lowercase()
                                );
                                let path = preset_dir.join(&filename);
                                match preset.save_to_file(&path) {
                                    Ok(()) => {
                                        app.toasts.info(format!("Preset saved: {}", filename));
                                    }
                                    Err(e) => {
                                        app.toasts.error(format!("Failed to save preset: {}", e));
                                    }
                                }
                            }
                        });
                    effect_row.header_response.context_menu(|ui| {
                        if ui
                            .button(if fx.enabled {
                                "Disable Effect"
                            } else {
                                "Enable Effect"
                            })
                            .clicked()
                        {
                            fx_toggle = Some((e_idx, !fx.enabled));
                            ui.close_menu();
                        }
                        if ui.button("Duplicate Effect").clicked() {
                            fx_dup = Some(e_idx);
                            ui.close_menu();
                        }
                        if e_idx > 0 && ui.button("Move Up").clicked() {
                            fx_move_up = Some(e_idx);
                            ui.close_menu();
                        }
                        if e_idx + 1 < total_effects && ui.button("Move Down").clicked() {
                            fx_move_down = Some(e_idx);
                            ui.close_menu();
                        }
                        if ui.button("Reset to Defaults").clicked() {
                            fx_reset = Some(e_idx);
                            ui.close_menu();
                        }
                        if ui.button("Delete Effect").clicked() {
                            fx_del = Some(e_idx);
                            ui.close_menu();
                        }
                    });

                    // Continuous interaction: parameter edits via draw_effect_type_ui.
                    ui.collapsing("Parameters", |ui| {
                        if let crate::core::timeline::EffectType::LensFlare {
                            link_to_light, ..
                        } = &mut fx.effect_type
                        {
                            let light_names: Vec<String> =
                                comp.lights.iter().map(|l| l.name.clone()).collect();
                            let current = link_to_light
                                .clone()
                                .unwrap_or_else(|| "(manual position)".into());
                            egui::ComboBox::from_id_salt(("flare_link", fx.id.as_str()))
                                .selected_text(current)
                                .show_ui(ui, |ui| {
                                    if ui
                                        .selectable_label(
                                            link_to_light.is_none(),
                                            "(manual position)",
                                        )
                                        .clicked()
                                    {
                                        *link_to_light = None;
                                        *slider_changed = true;
                                    }
                                    for ln in &light_names {
                                        if ui
                                            .selectable_label(
                                                link_to_light.as_deref() == Some(ln.as_str()),
                                                ln,
                                            )
                                            .clicked()
                                        {
                                            *link_to_light = Some(ln.clone());
                                            *slider_changed = true;
                                        }
                                    }
                                });
                            if link_to_light.is_some() {
                                ui.label(
                                    egui::RichText::new("tracking selected light")
                                        .small()
                                        .color(colors::TEXT_SECONDARY),
                                );
                            }
                        }
                        crate::ui::effects_controls::draw_effect_type_ui(
                            &mut fx.effect_type,
                            &fx.id,
                            ui,
                            current_frame,
                            slider_changed,
                            next_frame,
                        );
                    });
                }
            }
        } // <-- mutable borrow on app.history released here

        // Apply collected discrete actions via EditorSession.
        if let Some(i) = fx_move_up {
            let mut session = EditorSession::new(&mut app.history, "Move Effect Up");
            let comp = session.current_mut().active_composition_mut();
            if idx < comp.layers.len() {
                comp.layers[idx].effects.swap(i, i - 1);
            }
            session.commit();
        }
        if let Some(i) = fx_move_down {
            let mut session = EditorSession::new(&mut app.history, "Move Effect Down");
            let comp = session.current_mut().active_composition_mut();
            if idx < comp.layers.len() {
                comp.layers[idx].effects.swap(i, i + 1);
            }
            session.commit();
        }
        if let Some(i) = fx_dup {
            let mut session = EditorSession::new(&mut app.history, "Duplicate Effect");
            let comp = session.current_mut().active_composition_mut();
            if idx < comp.layers.len() {
                let mut cloned = comp.layers[idx].effects[i].clone();
                cloned.id = format!("{}_copy", cloned.id);
                comp.layers[idx].effects.insert(i + 1, cloned);
            }
            session.commit();
        }
        if let Some(i) = fx_del {
            let mut session = EditorSession::new(&mut app.history, "Delete Effect");
            let comp = session.current_mut().active_composition_mut();
            if idx < comp.layers.len() {
                comp.layers[idx].effects.remove(i);
            }
            session.commit();
        }
        if let Some(i) = fx_reset {
            let mut session = EditorSession::new(&mut app.history, "Reset Effect");
            let comp = session.current_mut().active_composition_mut();
            let mut reset_name: Option<String> = None;
            let mut missing_name: Option<String> = None;
            if idx < comp.layers.len() && i < comp.layers[idx].effects.len() {
                let fx = &mut comp.layers[idx].effects[i];
                match crate::ui::effects_controls::presets::fresh_default_for(&fx.effect_type) {
                    Some(fresh) => {
                        fx.effect_type = fresh;
                        reset_name = Some(fx.name.clone());
                    }
                    None => {
                        missing_name = Some(fx.name.clone());
                    }
                }
            }
            session.commit();
            if let Some(name) = reset_name {
                app.toasts.info(format!("Reset '{}' to defaults", name));
            } else if let Some(name) = missing_name {
                app.toasts
                    .error(format!("No factory defaults for '{}'", name));
            }
        }
        if let Some((e_idx, new_enabled)) = fx_toggle {
            let mut session = EditorSession::new(&mut app.history, "Toggle Effect");
            let comp = session.current_mut().active_composition_mut();
            if idx < comp.layers.len() && e_idx < comp.layers[idx].effects.len() {
                comp.layers[idx].effects[e_idx].enabled = new_enabled;
            }
            session.commit();
        }

        // ── Preset loading (discrete actions) ──
        ui.add_space(4.0);
        ui.separator();
        if ui.button("Load Preset from File...").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Effect Preset", &["json", "kagari-preset"])
                .pick_file()
            {
                match crate::core::effect_presets::EffectPreset::load_from_file(&path) {
                    Ok(preset) => {
                        let mut session = EditorSession::new(&mut app.history, "Load Preset");
                        let comp = session.current_mut().active_composition_mut();
                        if idx < comp.layers.len() {
                            preset.apply_to_layer(&mut comp.layers[idx]);
                            session.commit();
                        }
                        app.toasts.info(format!("Loaded preset: {}", preset.name));
                    }
                    Err(e) => {
                        app.toasts.error(format!("Failed to load preset: {}", e));
                    }
                }
            }
        }

        // Show saved presets from default directory
        let preset_dir = crate::core::effect_presets::default_preset_dir();
        if preset_dir.is_dir() {
            let presets = crate::core::effect_presets::discover_presets_in_dir(&preset_dir);
            if !presets.is_empty() {
                ui.collapsing(format!("Saved Presets ({})", presets.len()), |ui| {
                    for (name, path) in &presets {
                        if ui.selectable_label(false, name.to_string()).clicked() {
                            match crate::core::effect_presets::EffectPreset::load_from_file(path) {
                                Ok(preset) => {
                                    let mut session =
                                        EditorSession::new(&mut app.history, "Load Preset");
                                    let comp = session.current_mut().active_composition_mut();
                                    if idx < comp.layers.len() {
                                        preset.apply_to_layer(&mut comp.layers[idx]);
                                        session.commit();
                                    }
                                    app.toasts.info(format!("Loaded: {}", name));
                                }
                                Err(e) => {
                                    app.toasts.error(format!("Failed to load: {}", e));
                                }
                            }
                        }
                    }
                });
            }
        }
    }
}
