use eframe::egui;

pub fn draw_composition_menu(app: &mut crate::KagariApp, ctx: &egui::Context, ui: &mut egui::Ui) {
    if ui.add(egui::Button::new("New Composition...").shortcut_text(
        crate::ui::shortcuts::format_shortcut("N", true, false, false),
    )).clicked() {
        app.show_new_comp_dialog = true;
        ui.close_menu();
    }
    if ui.add(egui::Button::new("Duplicate Composition")).clicked() {
        app.modify_project(|p| {
            let idx = p.active_composition_idx;
            if let Some(src) = p.compositions.get(idx).cloned() {
                let mut copy = src.clone();
                copy.name = format!("{} copy", src.name);
                copy.id = format!("{}_copy_{}", src.id, p.compositions.len());
                p.compositions.push(copy);
                p.active_composition_idx = p.compositions.len() - 1;
            }
        });
        app.toasts.info("Composition duplicated");
        ui.close_menu();
    }
    if ui.add(egui::Button::new("Composition Settings...").shortcut_text(
        crate::ui::shortcuts::format_shortcut("E", true, true, false),
    )).clicked() {
        app.comp_settings_draft = Some(app.history.current().clone());
        app.show_comp_settings = true;
        ui.close_menu();
    }
    if ui.add(egui::Button::new("Add to Render Queue").shortcut_text(
        crate::ui::shortcuts::format_shortcut("M", true, false, false),
    )).clicked() {
        app.export.show_export_dialog = true;
        ui.close_menu();
    }
    ui.separator();
    draw_export_captions(app, ui);
    draw_save_frame(app, ui, ctx);
}

fn draw_export_captions(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    if ui.button("Export Captions (.srt)").on_hover_text("Write all 'Caption …' text layers as an SRT file").clicked() {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Subtitles", &["srt"])
            .set_file_name(format!("{}_captions.srt", app.history.current().active_composition().name.replace(' ', "_")))
            .save_file()
        {
            let proj = app.history.current();
            let comp = proj.active_composition();
            let srt = crate::core::subtitles::layers_to_srt(&comp.layers, comp.fps);
            if srt.is_empty() {
                app.toasts.error("No caption layers found");
            } else {
                match std::fs::write(&path, &srt) {
                    Ok(_) => app.toasts.info(format!("Captions exported: {}", path.display())),
                    Err(e) => app.toasts.error(format!("Write failed: {}", e)),
                }
            }
        }
        ui.close_menu();
    }
}

fn draw_save_frame(app: &mut crate::KagariApp, ui: &mut egui::Ui, _ctx: &egui::Context) {
    if ui.button("Save Frame As… (PNG)").on_hover_text("Render the current frame at full resolution and save as PNG").clicked() {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("PNG Image", &["png"])
            .set_file_name(format!("{}_frame_{}.png", app.history.current().active_composition().name, app.playback.current_frame))
            .save_file()
        {
            let comp = app.history.current().active_composition().clone();
            let frame = app.playback.current_frame;
            let max_dim = crate::core::software_renderer::MAX_RENDER_DIMENSION;
            let sw = (comp.width.saturating_mul(2)).min(max_dim);
            let sh = (comp.height.saturating_mul(2)).min(max_dim);
            let px = crate::core::software_renderer::render_frame_to_pixels(&comp, frame, sw, sh, 0.0, 0);
            let (pixels, w, h) = if sw > comp.width || sh > comp.height {
                (crate::core::supersample::downsample2x(&px, comp.width, comp.height), comp.width, comp.height)
            } else {
                (px, sw, sh)
            };
            match image::save_buffer(&path, &pixels, w, h, image::ColorType::Rgba8) {
                Ok(_) => app.toasts.info(format!("Frame {} saved to {}", frame, path.display())),
                Err(e) => app.toasts.error(format!("Save failed: {}", e)),
            }
        }
        ui.close_menu();
    }
}

pub fn draw_layer_menu(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    ui.menu_button("New", |ui| draw_layer_new_submenu(app, ui));
    ui.menu_button("Transform", |ui| draw_layer_transform_submenu(app, ui));
    ui.menu_button("Time", |ui| draw_layer_time_submenu(app, ui));
    ui.menu_button("Arrange", |ui| draw_layer_arrange_submenu(app, ui));
    ui.menu_button("Create", |ui| draw_layer_create_submenu(app, ui));
    ui.separator();
    if ui.button("Un-Solo All Layers").clicked() {
        app.modify_project(|p| { for l in p.active_composition_mut().layers.iter_mut() { l.solo = false; } });
        app.toasts.info("All layers un-soloed");
        ui.close_menu();
    }
    if ui.button("Unlock All Layers").clicked() {
        app.modify_project(|p| { for l in p.active_composition_mut().layers.iter_mut() { l.locked = false; } });
        app.toasts.info("All layers unlocked");
        ui.close_menu();
    }
    ui.separator();
    draw_create_shapes_from_text_full(app, ui);
    ui.separator();
    if ui.add(egui::Button::new("Pre-Compose...").shortcut_text(
        crate::ui::shortcuts::format_shortcut("C", true, true, false),
    )).clicked() {
        ui.close_menu();
    }
}

fn draw_layer_new_submenu(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    if ui.add(egui::Button::new("Solid...").shortcut_text(
        crate::ui::shortcuts::format_shortcut("Y", true, false, false),
    )).clicked() {
        crate::ui::shortcuts::create_new_layer(app, crate::ui::shortcuts::NewLayerKind::Solid);
        ui.close_menu();
    }
    if ui.add(egui::Button::new("Text").shortcut_text(
        crate::ui::shortcuts::format_shortcut("T", true, true, true),
    )).clicked() {
        crate::ui::shortcuts::create_new_layer(app, crate::ui::shortcuts::NewLayerKind::Text);
        ui.close_menu();
    }
    if ui.add(egui::Button::new("Null Object").shortcut_text(
        crate::ui::shortcuts::format_shortcut("Y", true, true, true),
    )).clicked() {
        crate::ui::shortcuts::create_new_layer(app, crate::ui::shortcuts::NewLayerKind::Null);
        ui.close_menu();
    }
    if ui.add(egui::Button::new("Adjustment Layer").shortcut_text(
        crate::ui::shortcuts::format_shortcut("Y", true, false, true),
    )).clicked() {
        crate::ui::shortcuts::create_new_layer(app, crate::ui::shortcuts::NewLayerKind::Adjustment);
        ui.close_menu();
    }
    if ui.button("Light").on_hover_text("Adds a 3D light to the composition").clicked() {
        let comp = app.history.current().active_composition();
        let mut light = crate::core::timeline::Light3D::default();
        light.id = comp.next_light_id();
        light.name = format!("Light {}", comp.lights.len() + 1);
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
            layer.scene_object = Some(crate::core::timeline::SceneObjectRef::Light { id: light_id.clone() });
            comp.add_layer(layer);
        });
        app.toasts.info(format!("Added {}", name));
        ui.close_menu();
    }
}

fn draw_layer_transform_submenu(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    if ui.button("Center in Comp").on_hover_text("Move position to the composition center").clicked() {
        if let Some(idx) = app.selection.selected_layer_idx {
            let (cw, ch) = { let c = app.history.current().active_composition(); (c.width as f32, c.height as f32) };
            app.modify_project(move |p| {
                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                    l.transform.position = crate::core::property::Animatable::new_constant([cw / 2.0, ch / 2.0]);
                }
            });
            app.toasts.info("Centered in Comp");
            ui.close_menu();
        }
    }
    if ui.button("Fit to Comp").on_hover_text("Scale so the layer covers the full comp frame").clicked() {
        if let Some(idx) = app.selection.selected_layer_idx {
            let (cw, ch) = { let c = app.history.current().active_composition(); (c.width as f32, c.height as f32) };
            app.modify_project(move |p| {
                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                    let bs = l.bounding_size();
                    if bs[0] > 1.0 && bs[1] > 1.0 {
                        let s = (cw / bs[0] * 100.0).max(ch / bs[1] * 100.0);
                        l.transform.scale = crate::core::property::Animatable::new_constant([s, s]);
                        l.transform.position = crate::core::property::Animatable::new_constant([cw / 2.0, ch / 2.0]);
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
                    l.transform.scale = crate::core::property::Animatable::new_constant([-s[0], s[1]]);
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
                    l.transform.scale = crate::core::property::Animatable::new_constant([s[0], -s[1]]);
                }
            });
            app.toasts.info("Flipped Vertical");
            ui.close_menu();
        }
    }
    ui.separator();
    if ui.button("Reset All Transforms").clicked() {
        if let Some(idx) = app.selection.selected_layer_idx {
            let dims = { let c = app.history.current().active_composition(); (c.width as f32, c.height as f32) };
            app.modify_project(move |p| {
                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                    l.transform.position = crate::core::property::Animatable::new_constant([dims.0 / 2.0, dims.1 / 2.0]);
                    l.transform.scale = crate::core::property::Animatable::new_constant([100.0, 100.0]);
                    l.transform.rotation = crate::core::property::Animatable::new_constant(0.0);
                    l.transform.opacity = crate::core::property::Animatable::new_constant(100.0);
                }
            });
            app.toasts.info("All transforms reset");
            ui.close_menu();
        }
    }
    ui.separator();
    if ui.add(egui::Button::new("Auto-Orient...").shortcut_text(
        crate::ui::shortcuts::format_shortcut("O", true, false, true),
    )).on_hover_text("Rotate layer automatically along its motion path direction").clicked() {
        if let Some(idx) = app.selection.selected_layer_idx {
            app.modify_project(move |p| {
                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                    l.auto_orient = match l.auto_orient {
                        crate::core::auto_orient::AutoOrientMode::Off => crate::core::auto_orient::AutoOrientMode::OrientAlongPath,
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
    if ui.add(egui::Button::new("Enable Time Remapping").shortcut_text(
        crate::ui::shortcuts::format_shortcut("T", true, false, true),
    )).clicked() {
        if let Some(idx) = app.selection.selected_layer_idx {
            app.modify_project(move |p| {
                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) { l.enable_time_remapping(); }
            });
            app.toasts.info("Time remapping enabled — edit keyframes in the Graph Editor");
        } else {
            app.toasts.info("Select a layer first");
        }
        ui.close_menu();
    }
    if ui.add(egui::Button::new("Time-Reverse Layer")).clicked() {
        if let Some(idx) = app.selection.selected_layer_idx {
            app.modify_project(move |p| {
                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) { l.time_reverse(); }
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
                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) { l.freeze_at(frame); }
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
                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) { l.clear_time_remap(); }
            });
            app.toasts.info("Time remapping removed");
        } else {
            app.toasts.info("Select a layer first");
        }
        ui.close_menu();
    }
    ui.separator();
    if ui.add(egui::Button::new("Time Stretch ×2 (Slow)")).clicked() {
        if let Some(idx) = app.selection.selected_layer_idx {
            app.modify_project(move |p| {
                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) { l.time_stretch(2.0); }
            });
            app.toasts.info("Layer stretched to ×2 duration");
        } else { app.toasts.info("Select a layer first"); }
        ui.close_menu();
    }
    if ui.add(egui::Button::new("Time Stretch ×0.5 (Fast)")).clicked() {
        if let Some(idx) = app.selection.selected_layer_idx {
            app.modify_project(move |p| {
                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) { l.time_stretch(0.5); }
            });
            app.toasts.info("Layer compressed to ×0.5 duration");
        } else { app.toasts.info("Select a layer first"); }
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
        app.selection.selected_layer_idx = if i < len { Some(len - 1) } else { app.selection.selected_layer_idx };
        ui.close_menu();
    }
    if ui.add(egui::Button::new("Bring Forward").shortcut_text(
        crate::ui::shortcuts::format_shortcut("]", true, false, false),
    )).clicked() {
        if i + 1 < len {
            app.modify_project(move |p| {
                let comp = p.active_composition_mut();
                if i + 1 < comp.layers.len() { comp.layers.swap(i, i + 1); }
            });
            app.selection.selected_layer_idx = Some(i + 1);
        }
        ui.close_menu();
    }
    if ui.add(egui::Button::new("Send Backward").shortcut_text(
        crate::ui::shortcuts::format_shortcut("[", true, false, false),
    )).clicked() {
        if i > 0 {
            app.modify_project(move |p| {
                let comp = p.active_composition_mut();
                if i < comp.layers.len() { comp.layers.swap(i - 1, i); }
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
        app.selection.selected_layer_idx = if i < len { Some(0) } else { app.selection.selected_layer_idx };
        ui.close_menu();
    }
}

fn draw_layer_create_submenu(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    if ui.button("Create Shapes from Text").on_hover_text("Convert text characters into editable vector shape layer").clicked() {
        if let Some(idx) = app.selection.selected_layer_idx {
            let mut temp_proj = app.history.current().clone();
            let comp = temp_proj.active_composition_mut();
            if let Some(layer) = comp.layers.get(idx) {
                if let crate::core::timeline::LayerType::Text { text, font_size, color, .. } = &layer.layer_type {
                    let total_frames = comp.duration_frames;
                    let text_str = text.clone();
                    let font_sz = *font_size;
                    let col = *color;
                    let mut shape_layer = crate::core::timeline::Layer::new(
                        format!("shape_from_text_{}", comp.layers.len()),
                        format!("{} Outlines", layer.name),
                        crate::core::timeline::LayerType::Shape {
                            shape_type: crate::core::timeline::ShapeType::Rectangle {
                                width: crate::core::property::Animatable::new_constant(font_sz as f32 * text_str.len() as f32 * 0.6),
                                height: crate::core::property::Animatable::new_constant(font_sz as f32),
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
    if ui.button("Create Shapes from Text").on_hover_text("Decompose selected Text layer into animatable vector Bezier Shape paths").clicked() {
        let mut created = false;
        let selected_idx = app.selection.selected_layer_idx;
        app.modify_project(|p| {
            let comp = p.active_composition_mut();
            if let Some(idx) = selected_idx {
                if idx < comp.layers.len() {
                    if let Some(shape_layer) = crate::core::text_to_shapes::convert_text_to_shapes(&comp.layers[idx], comp.width, comp.height) {
                        comp.layers.insert(idx, shape_layer);
                        created = true;
                    }
                }
            }
        });
        if created { app.toasts.info("Created Shapes from Text layer"); }
        else { app.toasts.warning("Please select a Text layer first"); }
        ui.close_menu();
    }
}
