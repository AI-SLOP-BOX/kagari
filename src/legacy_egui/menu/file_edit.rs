use super::import_helpers;
use eframe::egui;

pub fn draw_file_menu(app: &mut crate::KagariApp, ctx: &egui::Context, ui: &mut egui::Ui) {
    if ui.button("New Project").clicked() {
        app.replace_project_history(crate::core::timeline::Project::default());
        app.selection.selected_layer_idx = None;
        app.selection.selected_layers.clear();
        crate::core::frame_cache::bump_version();
        ui.close_menu();
    }
    ui.separator();
    let save_sc = crate::ui::shortcuts::format_shortcut("S", true, false, false);
    if ui
        .add(egui::Button::new("Save Project").shortcut_text(save_sc))
        .clicked()
    {
        let path = app.project_path.clone();
        if let Err(e) =
            crate::ui::project_io::save_project_to_path(app, std::path::Path::new(&path))
        {
            app.toasts.error(format!("Save failed: {}", e));
        }
        ui.close_menu();
    }
    if ui.button("Save Project As...").clicked() {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Kagari VFX Project", &["json"])
            .set_file_name("project.json")
            .save_file()
        {
            if let Err(e) = crate::ui::project_io::save_project_to_path(app, &path) {
                app.toasts.error(e);
            }
        }
        ui.close_menu();
    }
    if ui
        .add_enabled(
            app.collect_files_rx.is_none(),
            egui::Button::new("Collect Files..."),
        )
        .on_hover_text("Copy referenced media and save a portable project bundle")
        .clicked()
    {
        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
            let mut document = app.production_document.clone().unwrap_or_else(|| {
                crate::core::production_document::ProductionDocument::new(
                    app.history.current().clone(),
                )
            });
            *document.project_mut() = app.history.current().clone();
            document.audio.correction = app.audio_correction_settings();
            document.audio.channels = app.audio_mixer_channels.clone();
            document.audio.master_gain = app.playback.master_volume;
            let source_project_dir = std::path::Path::new(&app.project_path)
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .to_path_buf();
            let (sender, receiver) = std::sync::mpsc::channel();
            app.collect_files_rx = Some(receiver);
            let handle = std::thread::spawn(move || {
                let result = crate::core::project_migration::collect_production_document(
                    &document,
                    &folder,
                    &source_project_dir,
                );
                let _ = sender.send(result);
            });
            app.track_background_thread(handle);
        }
        ui.close_menu();
    }
    if ui
        .button("Remove Unused Footage")
        .on_hover_text("Remove unused footage items from project")
        .clicked()
    {
        let mut temp_proj = app.history.current().clone();
        let mut used_names = std::collections::HashSet::new();
        for c in &temp_proj.compositions {
            for l in &c.layers {
                used_names.insert(l.name.clone());
                match &l.layer_type {
                    crate::core::timeline::LayerType::Image { path } => {
                        used_names.insert(path.clone());
                    }
                    crate::core::timeline::LayerType::Model3D { path } => {
                        used_names.insert(path.clone());
                    }
                    crate::core::timeline::LayerType::Video { source, .. } => {
                        used_names.insert(source.clone());
                    }
                    crate::core::timeline::LayerType::Audio { path, .. } => {
                        used_names.insert(path.clone());
                    }
                    _ => {}
                }
            }
        }
        let before = temp_proj.assets.len();
        temp_proj.assets.retain(|a| {
            matches!(
                a.item_type,
                crate::core::timeline::ProjectItemType::Composition { .. }
                    | crate::core::timeline::ProjectItemType::Folder { .. }
            ) || used_names.contains(&a.name)
        });
        let rem = before.saturating_sub(temp_proj.assets.len());
        app.commit_project(temp_proj);
        app.toasts
            .info(format!("Removed {} unused footage items", rem));
        ui.close_menu();
    }
    if ui
        .button("Reduce Project")
        .on_hover_text("Keep only the active composition and its dependencies")
        .clicked()
    {
        let mut temp_proj = app.history.current().clone();
        let act = temp_proj.active_composition_idx;
        if act < temp_proj.compositions.len() {
            let keep = temp_proj.compositions[act].clone();
            temp_proj.compositions = vec![keep];
            temp_proj.active_composition_idx = 0;
            app.commit_project(temp_proj);
            app.toasts.info("Project reduced to active composition");
        }
        ui.close_menu();
    }
    ui.separator();
    draw_import_subtitles(app, ui);
    ui.separator();
    draw_import_blender_camera(app, ui);
    ui.separator();
    if ui.button("Open Project...").clicked() {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Kagari VFX Project", &["json"])
            .pick_file()
        {
            if let Err(e) = crate::ui::project_io::open_project_from_path(app, &path) {
                app.toasts.error(e);
            }
        }
        ui.close_menu();
    }
    ui.separator();
    draw_export_otio(app, ui);
    import_helpers::draw_import_video_button(app, ui);
    draw_import_image(app, ui);
    draw_import_svg(app, ui);
    draw_import_obj(app, ui);
    draw_import_image_sequence(app, ui);
    draw_export_mlt(app, ui);
    draw_import_otio(app, ui);
    ui.separator();
    if ui.button("Export Video (MP4)...").clicked() {
        app.export.show_export_dialog = true;
        ui.close_menu();
    }
    draw_export_exr(app, ui);
    if ui
        .button("Quick Export Active Comp")
        .on_hover_text("Export the active composition to MP4 with current settings (no dialog)")
        .clicked()
    {
        let comp_id = app.history.current().active_composition().id.clone();
        crate::ui::export_dialog::start_comp_export(app, ctx, &comp_id);
        ui.close_menu();
    }
    ui.separator();
    if ui.button("Exit").clicked() {
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
}

fn draw_import_subtitles(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    if ui
        .button("Import Subtitles (.srt / .vtt)...")
        .on_hover_text("Create timed, bottom-center caption text layers")
        .clicked()
    {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Subtitles", &["srt", "vtt"])
            .pick_file()
        {
            match crate::core::project_migration::read_bounded_text_file(&path, 8 * 1024 * 1024)
                .map(|s| {
                    crate::core::subtitles::parse_srt(
                        &s,
                        app.history.current().active_composition().fps,
                    )
                }) {
                Ok(cues) => {
                    if cues.is_empty() {
                        app.toasts.error("No cues found in subtitle file");
                    } else {
                        let (cw, ch) = {
                            let cc = app.history.current().active_composition();
                            (cc.width as f32, cc.height as f32)
                        };
                        let layers = crate::core::subtitles::cues_to_layers(&cues, cw, ch, 48);
                        let n = layers.len();
                        app.modify_project(|project| {
                            let comp = project.active_composition_mut();
                            for l in layers {
                                comp.add_layer(l);
                            }
                        });
                        app.toasts.info(format!("{} caption layers created", n));
                    }
                }
                Err(e) => app.toasts.error(e),
            }
        }
        ui.close_menu();
    }
}

fn draw_import_blender_camera(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    if ui
        .button("Import Blender Camera Track (.json)...")
        .on_hover_text("Bake a tracked camera solve onto this comp's active 3D camera")
        .clicked()
    {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Camera Track JSON", &["json"])
            .pick_file()
        {
            match crate::core::project_migration::read_bounded_text_file(&path, 32 * 1024 * 1024)
                .and_then(|s| crate::core::camera_track::BlenderCamTrack::parse(&s))
            {
                Ok(track) => {
                    let baked = std::cell::Cell::new(0usize);
                    app.modify_project(|p| {
                        let n = track.apply_to_comp(p.active_composition_mut(), true);
                        baked.set(n);
                    });
                    app.toasts.info(format!(
                        "Camera track baked: {} keyframes from '{}'",
                        baked.get(),
                        path.file_name().unwrap_or_default().to_string_lossy()
                    ));
                }
                Err(e) => app.toasts.error(e),
            }
        }
        ui.close_menu();
    }
}

fn draw_export_otio(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    if ui.button("Export OpenTimelineIO (.otio.json)").clicked() {
        let comp = app.history.current().active_composition();
        let otio = crate::core::integration::OtioTimeline::from_composition(comp);
        match otio.to_json() {
            Ok(json) => match std::fs::write(&app.otio_path, json) {
                Ok(_) => app.toasts.info(format!("Exported OTIO: {}", app.otio_path)),
                Err(err) => app
                    .toasts
                    .error(format!("Failed to save OTIO file: {}", err)),
            },
            Err(err) => app
                .toasts
                .error(format!("Failed to serialize OTIO: {}", err)),
        }
        ui.close_menu();
    }
}

fn draw_import_image(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    if ui.button("Import Image...").clicked() {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Image Files", &["png", "jpg", "jpeg", "bmp", "tga", "webp"])
            .pick_file()
        {
            import_helpers::import_image_inline(app, path);
        }
        ui.close_menu();
    }
}

fn draw_import_svg(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    if ui
        .button("Import SVG as Mask Layer...")
        .on_hover_text("Import SVG path geometry as editable Bezier masks on a solid layer")
        .clicked()
    {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("SVG Vector", &["svg"])
            .pick_file()
        {
            match crate::core::project_migration::read_bounded_text_file(&path, 16 * 1024 * 1024) {
                Ok(svg) => {
                    let paths = crate::core::svg_ai_importer::parse_svg_document(&svg);
                    if paths.is_empty() {
                        app.toasts
                            .error("No supported SVG path geometry was found in this file");
                    } else {
                        let name = path
                            .file_stem()
                            .map(|v| v.to_string_lossy().into_owned())
                            .unwrap_or_else(|| "svg".into());
                        import_helpers::insert_imported_svg_masks(app, paths, name);
                    }
                }
                Err(error) => app.toasts.error(format!("SVG import failed: {error}")),
            }
        }
        ui.close_menu();
    }
}

fn draw_import_obj(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    if ui
        .button("Import 3D Model (OBJ / GLB / glTF)...")
        .on_hover_text("Import a Wavefront OBJ or glTF/GLB 3D mesh as a camera-rendered 3D layer")
        .clicked()
    {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("3D Models (*.obj, *.glb, *.gltf)", &["obj", "glb", "gltf"])
            .pick_file()
        {
            let res = if path.extension().and_then(|s| s.to_str()) == Some("glb") {
                std::fs::read(&path)
                    .map_err(|e| e.to_string())
                    .and_then(|b| crate::core::gltf_loader::parse_glb(&b))
            } else if path.extension().and_then(|s| s.to_str()) == Some("gltf") {
                std::fs::read(&path)
                    .map_err(|e| e.to_string())
                    .and_then(|b| crate::core::gltf_loader::parse_gltf_json_with_binary(&b, &[]))
            } else {
                crate::core::project_migration::read_bounded_text_file(&path, 128 * 1024 * 1024)
                    .and_then(|obj| crate::core::obj_loader::parse_obj_str(&obj))
            };

            match res {
                Ok(mesh) => {
                    if mesh.vertices.is_empty() || mesh.indices.is_empty() {
                        app.toasts.error("3D model contains no renderable faces");
                    } else {
                        let name = path
                            .file_stem()
                            .map(|v| v.to_string_lossy().into_owned())
                            .unwrap_or_else(|| "model".into());
                        import_helpers::insert_imported_model3d(app, path, name);
                    }
                }
                Err(error) => app.toasts.error(format!("3D Model import failed: {error}")),
            }
        }
        ui.close_menu();
    }
}

fn draw_import_image_sequence(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    if ui.button("Import Image Sequence...").clicked() {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Image Sequence", &["png", "jpg", "jpeg", "webp"])
            .pick_file()
        {
            let name = path
                .file_stem()
                .map(|v| v.to_string_lossy().to_string())
                .unwrap_or_else(|| "image_sequence".to_string());
            crate::ui::drop_import::start_image_sequence_import(app, ui.ctx(), path, name);
        }
        ui.close_menu();
    }
}

fn draw_export_mlt(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    if ui
        .button("Export MLT XML (Shotcut / Kdenlive)...")
        .clicked()
    {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("MLT XML", &["xml", "mlt"])
            .set_file_name(format!(
                "{}.mlt.xml",
                app.history.current().active_composition().name
            ))
            .save_file()
        {
            let comp = app.history.current().active_composition();
            match crate::core::mlt_export::MltExporter::export_to_xml(comp) {
                Ok(xml) => match std::fs::write(&path, xml) {
                    Ok(_) => app
                        .toasts
                        .info(format!("Exported MLT XML: {}", path.to_string_lossy())),
                    Err(err) => app
                        .toasts
                        .error(format!("Failed to save MLT file: {}", err)),
                },
                Err(err) => app.toasts.error(err),
            }
        }
        ui.close_menu();
    }
}

fn draw_export_exr(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    if ui
        .button("Export Frame as OpenEXR (.exr)...")
        .on_hover_text("Export the current frame as a 32-bit linear floating point OpenEXR image")
        .clicked()
    {
        let comp = app.history.current().active_composition();
        let frame = app
            .playback
            .current_frame
            .clamp(0, comp.duration_frames.saturating_sub(1));
        let default_name = format!("{}_frame_{:05}.exr", comp.name, frame);
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("OpenEXR High Dynamic Range Image", &["exr"])
            .set_file_name(default_name)
            .save_file()
        {
            let width = comp.width;
            let height = comp.height;
            let f32_pixels = crate::core::software_renderer::render_frame_to_pixels_f32(
                comp,
                frame,
                width,
                height,
                0.0,
                app.color_lut_mode,
            );
            if f32_pixels.is_empty() {
                app.toasts.error("Frame render failed: empty buffer");
            } else {
                let mut flat_rgba = Vec::with_capacity(f32_pixels.len() * 4);
                for px in f32_pixels {
                    flat_rgba.extend_from_slice(&px);
                }
                let image = crate::core::exr_writer::ExrImage::rgba_f32(width, height, &flat_rgba);
                match crate::core::exr_writer::write_exr_file(&path, &image) {
                    Ok(()) => {
                        app.toasts.info(format!(
                            "Exported OpenEXR frame {}: {}",
                            frame,
                            path.file_name().unwrap_or_default().to_string_lossy()
                        ));
                    }
                    Err(err) => {
                        app.toasts
                            .error(format!("Failed to write EXR file: {}", err));
                    }
                }
            }
        }
        ui.close_menu();
    }
}

fn draw_import_otio(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    if ui.button("Import OpenTimelineIO (.otio.json)").clicked() {
        match crate::core::project_migration::read_bounded_text_file(
            std::path::Path::new(&app.otio_path),
            crate::core::project_migration::MAX_PROJECT_JSON_BYTES,
        ) {
            Ok(json) => match crate::core::integration::OtioTimeline::from_json(&json) {
                Ok(otio) => match otio.to_composition() {
                    Ok(comp) => {
                        app.modify_project(|p| {
                            let idx = p
                                .active_composition_idx
                                .min(p.compositions.len().saturating_sub(1));
                            if let Some(target) = p.compositions.get_mut(idx) {
                                *target = comp;
                            } else {
                                p.compositions.push(comp);
                            }
                        });
                        app.toasts.info(format!("Imported OTIO: {}", app.otio_path));
                    }
                    Err(err) => app.toasts.error(format!("Invalid OTIO timeline: {err}")),
                },
                Err(err) => app.toasts.error(format!("Invalid OTIO format: {}", err)),
            },
            Err(err) => app
                .toasts
                .error(format!("Could not read OTIO file: {}", err)),
        }
        ui.close_menu();
    }
}

pub fn draw_edit_menu(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    let undo_sc = crate::ui::shortcuts::format_shortcut("Z", true, false, false);
    if ui
        .add_enabled(
            app.history.can_undo(),
            egui::Button::new("Undo (元に戻す)").shortcut_text(undo_sc),
        )
        .clicked()
    {
        app.history.undo();
        ui.close_menu();
    }
    let redo_sc = crate::ui::shortcuts::format_shortcut("Z", true, true, false);
    if ui
        .add_enabled(
            app.history.can_redo(),
            egui::Button::new("Redo (やり直す)").shortcut_text(redo_sc),
        )
        .clicked()
    {
        app.history.redo();
        ui.close_menu();
    }
    if ui
        .button("Undo History…")
        .on_hover_text("Open the named-step history panel and jump to any step")
        .clicked()
    {
        app.show_history_panel = !app.show_history_panel;
        ui.close_menu();
    }
    ui.separator();
    if ui
        .add(
            egui::Button::new("Duplicate").shortcut_text(crate::ui::shortcuts::format_shortcut(
                "D", true, false, false,
            )),
        )
        .clicked()
    {
        if let Some(sel_idx) = app.selection.selected_layer_idx {
            let mut duplicated = false;
            app.modify_project(|project| {
                let comp = project.active_composition_mut();
                if sel_idx < comp.layers.len() {
                    let mut cloned = comp.layers[sel_idx].clone();
                    cloned.id = comp.next_layer_id(&format!("{}_copy", cloned.id));
                    cloned.name = format!("{} copy", cloned.name);
                    comp.layers.insert(sel_idx + 1, cloned);
                    duplicated = true;
                }
            });
            if duplicated {
                app.toasts.info("Layer duplicated");
            }
        }
        ui.close_menu();
    }
    ui.separator();
    draw_auto_trace(app, ui);
}

fn draw_auto_trace(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    if ui
        .add(egui::Button::new("Auto-Trace Layer"))
        .on_hover_text("Trace selected layer's alpha into a new Shape layer (FreeformBezier)")
        .clicked()
    {
        if let Some(sel_idx) = app.selection.selected_layer_idx {
            let cur_frame = app.playback.current_frame;
            let comp = app.history.current().active_composition();
            if sel_idx < comp.layers.len() {
                let layer = &comp.layers[sel_idx];
                let name = layer.name.clone();
                let dur = comp.duration_frames;
                let mut contour: Vec<[f32; 2]> = Vec::new();
                for mask in &layer.masks {
                    if mask.enabled {
                        let verts = mask.path.vertices_at_frame(cur_frame);
                        contour.extend_from_slice(&verts);
                    }
                }
                if contour.is_empty() {
                    let pos = layer.transform.position.evaluate(cur_frame);
                    let scale = layer.transform.scale.evaluate(cur_frame);
                    let anchor = layer.transform.anchor_point.evaluate(cur_frame);
                    let w = 100.0 * scale[0] / 100.0;
                    let h = 100.0 * scale[1] / 100.0;
                    let cx = pos[0] - anchor[0];
                    let cy = pos[1] - anchor[1];
                    contour = vec![[cx, cy], [cx + w, cy], [cx + w, cy + h], [cx, cy + h]];
                }
                if contour.len() >= 3 {
                    let len = contour.len();
                    let shape = crate::core::timeline::ShapeType::FreeformBezier {
                        points: contour,
                        tangents: (0..len).map(|_| ([0.0f32, 0.0], [0.0f32, 0.0])).collect(),
                        closed: true,
                    };
                    let opacity = layer
                        .transform
                        .opacity
                        .evaluate(cur_frame)
                        .clamp(0.0, 100.0);
                    let alpha = opacity / 100.0;
                    let new_layer = crate::core::timeline::Layer::new(
                        format!("Auto-Trace {name}"),
                        format!("Auto-Trace {name}"),
                        crate::core::timeline::LayerType::Shape {
                            shape_type: shape,
                            color: [0.5 * alpha, 0.5 * alpha, 0.5 * alpha, alpha],
                            stroke_color: [1.0, 1.0, 1.0, 1.0],
                            stroke_width: 2.0,
                            fill_type: Default::default(),
                            extrusion_depth: 0.0,
                            bevel_depth: 0.0,
                        },
                        dur,
                    );
                    app.modify_project(|p| {
                        p.active_composition_mut().layers.push(new_layer);
                    });
                    app.toasts
                        .info(format!("Auto-traced '{name}' into shape layer"));
                } else {
                    app.toasts
                        .error("Layer has no traceable mask or alpha data");
                }
            }
        }
        ui.close_menu();
    }
}
