use eframe::egui;

fn draw_header_action(ui: &mut egui::Ui, label: &str, width: f32, size: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 28.0), egui::Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 4.0, crate::ui::theme::colors::BG_HOVER);
    }
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(size),
        crate::ui::theme::colors::TEXT_SECONDARY,
    );
    response.on_hover_text("Open Render Queue")
}

pub fn draw(app: &mut crate::KagariApp, ctx: &egui::Context) {
    if !app.show_home && ctx.screen_rect().width() >= 1200.0 {
        draw_reference_studio_header(app, ctx);
        return;
    }
    draw_studio_header(app, ctx);
}

fn insert_imported_sequence(
    app: &mut crate::KagariApp,
    asset: crate::core::video_import::VideoAsset,
    name: String,
) {
    let (comp_w, comp_h, comp_duration) = {
        let comp = app.history.current().active_composition();
        (comp.width as f32, comp.height as f32, comp.duration_frames)
    };
    let fit = ((comp_w / asset.width.max(1) as f32)
        .min(comp_h / asset.height.max(1) as f32))
        .min(1.0)
        * 100.0;
    app.modify_project(|p| {
        let comp = p.active_composition_mut();
        let layer_id = comp.next_layer_id("image_sequence");
        let mut layer = crate::core::timeline::Layer::new(
            layer_id,
            name.clone(),
            crate::core::timeline::LayerType::Video {
                source: asset.source_path.clone(),
                frames_dir: asset.frames_dir.clone(),
                frame_count: asset.frame_count,
                audio_wav: None,
                speed: 1.0,
            },
            comp_duration,
        );
        layer.transform.position = crate::core::property::Animatable::new_constant([
            comp_w * 0.5,
            comp_h * 0.5,
        ]);
        layer.transform.scale = crate::core::property::Animatable::new_constant([fit, fit]);
        comp.layers.push(layer);
    });
    app.toasts.info(format!(
        "Imported image sequence: {} ({} frames)",
        name, asset.frame_count
    ));
}

fn insert_imported_model3d(
    app: &mut crate::KagariApp,
    path: std::path::PathBuf,
    name: String,
) {
    let (comp_w, comp_h, comp_duration) = {
        let comp = app.history.current().active_composition();
        (comp.width as f32, comp.height as f32, comp.duration_frames)
    };
    let source = path.to_string_lossy().into_owned();
    app.modify_project(|project| {
        let asset_id = format!("model3d_asset_{}", project.assets.len() + 1);
        project.assets.push(crate::core::timeline::ProjectItem::new(
            asset_id,
            name.clone(),
            crate::core::timeline::ProjectItemType::Model3D {
                path: source.clone(),
            },
        ));
        let comp = project.active_composition_mut();
        let mut layer = crate::core::timeline::Layer::new(
            comp.next_layer_id("model3d"),
            name.clone(),
            crate::core::timeline::LayerType::Model3D {
                path: source.clone(),
            },
            comp_duration,
        );
        layer.is_3d = true;
        layer.transform.position = crate::core::property::Animatable::new_constant([
            comp_w * 0.5,
            comp_h * 0.5,
        ]);
        layer.transform_3d.position = crate::core::property::Animatable::new_constant([
            0.0,
            0.0,
            (comp_w.max(comp_h) * 0.6).max(600.0),
        ]);
        comp.layers.push(layer);
    });
    app.toasts.info(format!("Imported 3D model: {}", name));
}

fn insert_imported_svg_masks(
    app: &mut crate::KagariApp,
    paths: Vec<crate::core::svg_ai_importer::SvgVectorPath>,
    name: String,
) {
    let mask_count = paths.len();
    app.modify_project(|project| {
        let comp = project.active_composition_mut();
        for (index, vector_path) in paths.into_iter().enumerate() {
            let path_label = if vector_path.name.trim().is_empty() {
                format!("Path {}", index + 1)
            } else {
                vector_path.name.clone()
            };
            let layer_name = if mask_count == 1 {
                name.clone()
            } else {
                format!("{} - {}", name, path_label)
            };
            let fill_color = vector_path
                .fill_color
                .unwrap_or([0.0, 0.0, 0.0, 0.0]);
            let mut layer = crate::core::timeline::Layer::new(
                comp.next_layer_id("svg"),
                layer_name,
                crate::core::timeline::LayerType::Solid { color: fill_color },
                comp.duration_frames,
            );
            layer.transform.position = crate::core::property::Animatable::new_constant([
                comp.width as f32 * 0.5,
                comp.height as f32 * 0.5,
            ]);
            let mut mask = crate::core::mask::Mask::new_closed(
                format!("mask_svg_{}", index + 1),
                path_label,
                vector_path
                    .vertices
                    .iter()
                    .map(|vertex| vertex.position)
                    .collect(),
            );
            mask.path = vector_path.to_mask_path();
            if let Some((stroke_color, stroke_width)) = vector_path
                .stroke_color
                .filter(|_| vector_path.stroke_width.is_finite() && vector_path.stroke_width > 0.0)
                .map(|color| (color, vector_path.stroke_width))
            {
                layer.style.stroke.enabled = true;
                layer.style.stroke.color = stroke_color;
                layer.style.stroke.size = stroke_width;
            }
            layer.masks.push(mask);
            comp.layers.push(layer);
        }
    });
    app.toasts.info(format!(
        "Imported SVG as {} editable mask layer{}: {}",
        mask_count,
        if mask_count == 1 { "" } else { "s" },
        name
    ));
}

fn draw_reference_studio_header(app: &mut crate::KagariApp, ctx: &egui::Context) {
    egui::TopBottomPanel::top("studio_header")
        .exact_height(48.0)
        .resizable(false)
        .frame(egui::Frame::none().fill(egui::Color32::from_rgb(10, 18, 24)))
        .show(ctx, |ui| {
            let rect = ui.max_rect();
            ui.painter().line_segment(
                [egui::pos2(rect.left(), rect.bottom() - 1.0), egui::pos2(rect.right(), rect.bottom() - 1.0)],
                egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(43, 55, 65)),
            );
            let content_rect = rect;
            ui.allocate_new_ui(
                egui::UiBuilder::new().max_rect(content_rect).layout(egui::Layout::left_to_right(egui::Align::Center)),
                |ui| {
                    ui.add_space(17.0);
                    if app.home_banner.is_none() {
                        if let Ok(img) = image::open(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/kagari_logo.webp")) {
                            app.home_banner = crate::ui::home_screen::load_logo_texture(ctx, img);
                        }
                    }
                    if let Some(texture) = app.home_banner.as_ref() {
                        ui.add(egui::Image::new(egui::load::SizedTexture::new(texture.id(), egui::vec2(34.0, 34.0))));
                    }
                    ui.add_space(7.0);
                    ui.label(egui::RichText::new("Kagari VFX").size(17.0).strong().color(crate::ui::theme::colors::TEXT_PRIMARY));
                    ui.add_space(22.0);
                    crate::ui::icons::render_svg_bytes(ui, "studio-breadcrumb-arrow", crate::ui::icons::SVG_CHEVRON_RIGHT, egui::vec2(18.0, 18.0), crate::ui::theme::colors::TEXT_SECONDARY);
                    ui.add_space(18.0);
                    ui.painter().line_segment([egui::pos2(ui.cursor().left(), rect.top() + 10.0), egui::pos2(ui.cursor().left(), rect.bottom() - 10.0)], egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(42, 54, 64)));
                    ui.add_space(18.0);
                    crate::ui::icons::render_svg_bytes(ui, "studio-project-folder", crate::ui::icons::SVG_FOLDER, egui::vec2(18.0, 18.0), egui::Color32::from_rgb(174, 190, 207));
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new("Sample Project / main_comp").size(14.0).color(crate::ui::theme::colors::TEXT_SECONDARY));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let render_response = draw_header_action(ui, "Render", 68.0, 13.0);
                        if render_response.clicked() {
                            app.show_home = false;
                            app.ui_tabs.bottom_dock_tab = 1;
                        }
                        ui.add_space(14.0);
                        ui.label(egui::RichText::new("Autosaved 10:24").size(12.0).color(crate::ui::theme::colors::TEXT_SECONDARY));
                    });
                },
            );
        });
}

fn draw_studio_header(app: &mut crate::KagariApp, ctx: &egui::Context) {
    let compact_header = ctx.screen_rect().width() < 1200.0;
    let composition_label = app.history.current().active_composition().name.clone();
    let workspace_id = egui::Id::new("studio_active_workspace");
    let inferred_workspace = if app.show_home {
        0_usize
    } else if app.ui_tabs.left_tab_idx == 1 && app.ui_tabs.right_tab_idx == 1 {
        4
    } else if app.ui_tabs.left_tab_idx == 1 && app.ui_tabs.right_tab_idx == 0 {
        2
    } else {
        1
    };
    let mut active_workspace = ctx
        .data_mut(|data| data.get_temp::<usize>(workspace_id))
        .unwrap_or(inferred_workspace);
    if app.show_home {
        active_workspace = 0;
    } else if active_workspace == 0 {
        active_workspace = inferred_workspace;
        ctx.data_mut(|data| data.insert_temp(workspace_id, active_workspace));
    }
    let workspaces = [
        ("Home", crate::ui::home_screen::HomeNav::Home),
        ("Compositing", crate::ui::home_screen::HomeNav::Compositing),
        ("Effects", crate::ui::home_screen::HomeNav::Effects),
        ("Assets", crate::ui::home_screen::HomeNav::Projects),
        ("Render", crate::ui::home_screen::HomeNav::Render),
    ];
    let workspace_label = workspaces
        .get(active_workspace)
        .map(|(label, _)| *label)
        .unwrap_or("Compositing");
    egui::TopBottomPanel::top("studio_header")
        .exact_height(66.0)
        .frame(
            egui::Frame::none()
                .fill(crate::ui::theme::colors::BG_DEEPEST)
                .stroke(egui::Stroke::new(1.0_f32, crate::ui::theme::colors::BORDER_SUBTLE))
                .inner_margin(egui::Margin::symmetric(if compact_header { 8.0 } else { 24.0 }, 0.0)),
        )
        .show(ctx, |ui| {
            let header_size = ui.available_size();
            ui.allocate_ui_with_layout(
                header_size,
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                if app.home_banner.is_none() {
                    if let Ok(img) = image::open(
                        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                            .join("assets/kagari_logo.webp"),
                    ) {
                        app.home_banner = crate::ui::home_screen::load_logo_texture(ctx, img);
                    }
                }
                if let Some(texture) = app.home_banner.as_ref() {
                    ui.add(egui::Image::new(egui::load::SizedTexture::new(
                        texture.id(),
                        egui::vec2(if compact_header { 28.0 } else { 38.0 }, if compact_header { 28.0 } else { 38.0 }),
                    )));
                }
                ui.add_space(if compact_header { 6.0 } else { 10.0 });
                ui.label(
                    egui::RichText::new("Kagari VFX")
                        .size(if compact_header { 15.0 } else { 20.0 })
                        .color(crate::ui::theme::colors::TEXT_PRIMARY),
                );
                ui.add_space(if compact_header { 12.0 } else { 22.0 });
                crate::ui::icons::render_svg_bytes(
                    ui,
                    "studio-project-folder",
                    crate::ui::icons::SVG_FOLDER,
                    egui::vec2(if compact_header { 16.0 } else { 18.0 }, if compact_header { 16.0 } else { 18.0 }),
                    crate::ui::theme::colors::TEXT_SECONDARY,
                );
                ui.add_space(6.0);
                ui.label(egui::RichText::new(format!("Sample Project / {composition_label}"))
                    .size(if compact_header { 12.0 } else { 14.0 })
                    .color(crate::ui::theme::colors::TEXT_SECONDARY));
                ui.add_space(6.0);
                ui.menu_button(egui::RichText::new(workspace_label).size(if compact_header { 12.0 } else { 13.0 }).color(crate::ui::theme::colors::TEXT_PRIMARY), |ui| {
                    for (workspace_index, (label, nav)) in workspaces.iter().enumerate() {
                        if ui.selectable_label(active_workspace == workspace_index, *label).clicked() {
                            active_workspace = workspace_index;
                            ctx.data_mut(|data| data.insert_temp(workspace_id, active_workspace));
                            match *nav {
                                crate::ui::home_screen::HomeNav::Home => app.show_home = true,
                                crate::ui::home_screen::HomeNav::Compositing => {
                                    app.show_home = false;
                                    app.ui_tabs.left_tab_idx = 0;
                                    app.ui_tabs.right_tab_idx = 30;
                                }
                                crate::ui::home_screen::HomeNav::Effects => {
                                    app.show_home = false;
                                    app.ui_tabs.left_tab_idx = 1;
                                    app.ui_tabs.right_tab_idx = 0;
                                }
                                crate::ui::home_screen::HomeNav::Projects => {
                                    app.show_home = false;
                                    app.ui_tabs.left_tab_idx = 0;
                                    app.ui_tabs.right_tab_idx = 30;
                                }
                                crate::ui::home_screen::HomeNav::Render => {
                                    app.show_home = false;
                                    app.ui_tabs.left_tab_idx = 1;
                                    app.ui_tabs.right_tab_idx = 1;
                                }
                                _ => {}
                            }
                            ui.close_menu();
                        }
                    }
                });
                if compact_header {
                    let drawer_id = egui::Id::new("compact_project_drawer");
                    let drawer_open = ctx.data(|data| data.get_temp::<bool>(drawer_id)).unwrap_or(false);
                    let drawer_label = if drawer_open {
                        "Close Panel"
                    } else if app.ui_tabs.left_tab_idx == 1 {
                        "Effects"
                    } else {
                        "Project"
                    };
                    if ui.small_button(drawer_label).clicked() {
                        let next_open = !drawer_open;
                        ctx.data_mut(|data| {
                            data.insert_temp(drawer_id, next_open);
                            if next_open {
                                data.insert_temp(egui::Id::new("compact_inspector_drawer"), false);
                            }
                        });
                    }
                    if ctx.screen_rect().width() < 950.0 {
                        let inspector_id = egui::Id::new("compact_inspector_drawer");
                        let inspector_open = ctx
                            .data(|data| data.get_temp::<bool>(inspector_id))
                            .unwrap_or(false);
                        let inspector_label = if inspector_open {
                            "Hide Inspector"
                        } else {
                            "Inspector"
                        };
                        if ui.small_button(inspector_label).clicked() {
                            let next_open = !inspector_open;
                            ctx.data_mut(|data| {
                                data.insert_temp(inspector_id, next_open);
                                if next_open {
                                    data.insert_temp(egui::Id::new("compact_project_drawer"), false);
                                }
                            });
                        }
                    }
                }
                let right_header_width = if compact_header { 220.0 } else { 390.0 };
                ui.add_space((ui.available_width() - right_header_width).max(0.0));
                ui.allocate_ui_with_layout(
                    egui::vec2(right_header_width, header_size.y),
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        let render_response = draw_header_action(
                            ui,
                            "Render",
                            if compact_header { 68.0 } else { 76.0 },
                            if compact_header { 12.0 } else { 13.0 },
                        );
                        if render_response.clicked() {
                            app.show_home = false;
                            app.ui_tabs.bottom_dock_tab = 1;
                        }
                        ui.add_space(12.0);
                        ui.label(egui::RichText::new("Autosaved").size(11.0).color(crate::ui::theme::colors::TEXT_MUTED));
                    },
                );
                },
            );
        });
}

#[allow(dead_code)]
fn draw_legacy_menus(app: &mut crate::KagariApp, ctx: &egui::Context) {
    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New Project").clicked() {
                    app.history = crate::core::history::ProjectHistory::new(
                        crate::core::timeline::Project::default(),
                    );
                    app.selection.selected_layer_idx = None;
                    app.selection.selected_layers.clear();
                    crate::core::frame_cache::bump_version();
                    ui.close_menu();
                }
                ui.separator();
                let save_sc = crate::ui::shortcuts::format_shortcut("S", true, false, false);
                if ui.add(egui::Button::new("Save Project").shortcut_text(save_sc)).clicked() {
                    let path = app.project_path.clone();
                    let project = app.history.current();
                    match crate::core::project_migration::save_project_atomic(project, &path) {
                        Ok(_) => {
                            let _ = app.autosave.save_now(project);
                            app.toasts.info(format!("Saved: {}", path));
                        }
                        Err(e) => {
                            app.toasts.error(format!("Save failed: {}", e));
                        }
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
                if ui.button("📦 Collect Files...").on_hover_text("Collect all source footage, audio, and asset dependencies into an archive folder").clicked() {
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        let project = app.history.current();
                        let target_json = folder.join("collected_project.json");
                        match crate::core::project_migration::save_project_atomic(project, target_json.to_str().unwrap_or("")) {
                            Ok(_) => {
                                crate::ui::project_io::reveal_in_file_manager(&folder);
                                app.toasts.info(format!("📦 Collected project & assets to {}", folder.display()));
                            },
                            Err(e) => app.toasts.error(format!("Collect failed: {}", e)),
                        }
                    }
                    ui.close_menu();
                }
                if ui.button("🧹 Remove Unused Footage").on_hover_text("Remove unused footage items from project").clicked() {
                    let mut temp_proj = app.history.current().clone();
                    let mut used_names = std::collections::HashSet::new();
                    for c in &temp_proj.compositions {
                        for l in &c.layers {
                            used_names.insert(l.name.clone());
                            match &l.layer_type {
                                crate::core::timeline::LayerType::Image { path } => { used_names.insert(path.clone()); }
                                crate::core::timeline::LayerType::Model3D { path } => { used_names.insert(path.clone()); }
                                crate::core::timeline::LayerType::Video { source, .. } => { used_names.insert(source.clone()); }
                                crate::core::timeline::LayerType::Audio { path, .. } => { used_names.insert(path.clone()); }
                                _ => {}
                            }
                        }
                    }
                    let before = temp_proj.assets.len();
                    temp_proj.assets.retain(|a| {
                        matches!(a.item_type, crate::core::timeline::ProjectItemType::Composition { .. } | crate::core::timeline::ProjectItemType::Folder { .. })
                            || used_names.contains(&a.name)
                    });
                    let rem = before.saturating_sub(temp_proj.assets.len());
                    app.commit_project(temp_proj);
                    app.toasts.info(format!("Removed {} unused footage items", rem));
                    ui.close_menu();
                }
                if ui.button("🗜 Reduce Project").on_hover_text("Keep only the active composition and its dependencies").clicked() {
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
                if ui.button("Import Subtitles (.srt / .vtt)...").on_hover_text("Create timed, bottom-center caption text layers — pair with Kdenlive/Shotcut Whisper output").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Subtitles", &["srt", "vtt"])
                        .pick_file()
                    {
                        match crate::core::project_migration::read_bounded_text_file(
                            &path,
                            8 * 1024 * 1024,
                        )
                            .map(|s| crate::core::subtitles::parse_srt(&s, app.history.current().active_composition().fps))
                        {
                            Ok(cues) => {
                                if cues.is_empty() {
                                    app.toasts.error("No cues found in subtitle file");
                                } else {
                                    let (cw, ch) = { let cc = app.history.current().active_composition(); (cc.width as f32, cc.height as f32) };
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
                ui.separator();
                if ui.button("Import Blender Camera Track (.json)...").on_hover_text("Bake a tracked camera solve onto this comp's active 3D camera — run tools/blender_camera_export.py inside Blender first").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Camera Track JSON", &["json"])
                        .pick_file()
                    {
                        match crate::core::project_migration::read_bounded_text_file(
                            &path,
                            32 * 1024 * 1024,
                        )
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
                if ui.button("Export OpenTimelineIO (.otio.json)").clicked() {
                    let comp = app.history.current().active_composition();
                    let otio = crate::core::integration::OtioTimeline::from_composition(comp);
                    match serde_json::to_string_pretty(&otio) {
                        Ok(json) => match std::fs::write(&app.otio_path, json) {
                            Ok(_) => {
                                app.toasts.info(format!("Exported OTIO: {}", app.otio_path));
                            }
                            Err(err) => {
                                app.toasts.error(format!("Failed to save OTIO file: {}", err));
                            }
                        },
                        Err(err) => {
                            app.toasts.error(format!("Failed to serialize OTIO: {}", err));
                        }
                    }
                    ui.close_menu();
                }
                if ui.button("Import Video (FFmpeg)...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Video Files", &["mp4", "mov", "avi", "mkv", "webm", "av1"])
                        .pick_file()
                    {
                        // Extract at the active composition's fps so 1 seq frame == 1 comp frame
                        let comp = app.history.current().active_composition();
                        let fps = comp.fps as f32;
                        let name = path.file_stem().map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| "video".to_string());
                        let dest = std::env::temp_dir().join("kagari_media").join(&name);
                        let src = path.to_string_lossy().to_string();
                        app.toasts.info(format!("Extracting video frames for '{}' via FFmpeg...", name));
                        ui.ctx().request_repaint();

                        match crate::core::video_import::import_video(
                            &src, &dest, fps,
                        ) {
                            Ok(asset) => {
                                let (comp_w, comp_h, comp_duration) = {
                                    let comp = app.history.current().active_composition();
                                    (comp.width as f32, comp.height as f32, comp.duration_frames)
                                };
                                let fit = ((comp_w / asset.width.max(1) as f32)
                                    .min(comp_h / asset.height.max(1) as f32))
                                    .min(1.0)
                                    * 100.0;
                                app.modify_project(|p| {
                                    let comp = p.active_composition_mut();
                                    let layer_id = comp.next_layer_id("video");
                                    let mut layer = crate::core::timeline::Layer::new(
                                        layer_id,
                                        name.clone(),
                                        crate::core::timeline::LayerType::Video {
                                            source: src.clone(),
                                            frames_dir: asset.frames_dir.clone(),
                                            frame_count: asset.frame_count,
                                            audio_wav: asset.audio_wav.clone(),
                                            speed: 1.0,
                                        },
                                        comp_duration,
                                    );
                                    layer.transform.position =
                                        crate::core::property::Animatable::new_constant([
                                            comp_w * 0.5,
                                            comp_h * 0.5,
                                        ]);
                                    layer.transform.scale =
                                        crate::core::property::Animatable::new_constant([fit, fit]);
                                    comp.layers.push(layer);
                                });
                                app.toasts.info(format!(
                                    "Imported video: {} ({} frames)",
                                    name, asset.frame_count
                                ));
                            }
                            Err(err) => {
                                app.toasts.error(format!("Video import failed: {}", err));
                            }
                        }
                    }
                    ui.close_menu();
                }
                if ui.button("Import Image...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Image Files", &["png", "jpg", "jpeg", "bmp", "tga", "webp"])
                        .pick_file()
                    {
                        let src = path.to_string_lossy().to_string();
                        let name = path.file_stem().map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| "image".to_string());
                        let dimensions = image::image_dimensions(&path).ok();
                        let (comp_w, comp_h, comp_duration) = {
                            let comp = app.history.current().active_composition();
                            (comp.width as f32, comp.height as f32, comp.duration_frames)
                        };
                        let fit = dimensions
                            .map(|(width, height)| {
                                ((comp_w / width.max(1) as f32)
                                    .min(comp_h / height.max(1) as f32))
                                    .min(1.0)
                                    * 100.0
                            })
                            .unwrap_or(100.0);
                        app.modify_project(|p| {
                            let comp = p.active_composition_mut();
                            let layer_id = comp.next_layer_id("image");
                            let mut layer = crate::core::timeline::Layer::new(
                                layer_id,
                                name.clone(),
                                crate::core::timeline::LayerType::Image { path: src.clone() },
                                comp_duration,
                            );
                            layer.transform.position =
                                crate::core::property::Animatable::new_constant([
                                    comp_w * 0.5,
                                    comp_h * 0.5,
                                ]);
                            layer.transform.scale =
                                crate::core::property::Animatable::new_constant([fit, fit]);
                            comp.layers.push(layer);
                        });
                        app.toasts.info(format!("Imported image: {}", name));
                    }
                    ui.close_menu();
                }
                if ui
                    .button("Import SVG as Mask Layer...")
                    .on_hover_text(
                        "Import SVG path geometry as editable Bezier masks on a solid layer",
                    )
                    .clicked()
                {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("SVG Vector", &["svg"])
                        .pick_file()
                    {
                        match crate::core::project_migration::read_bounded_text_file(
                            &path,
                            16 * 1024 * 1024,
                        ) {
                            Ok(svg) => {
                                let paths = crate::core::svg_ai_importer::parse_svg_document(&svg);
                                if paths.is_empty() {
                                    app.toasts.error(
                                        "No supported SVG path geometry was found in this file",
                                    );
                                } else {
                                    let name = path
                                        .file_stem()
                                        .map(|value| value.to_string_lossy().into_owned())
                                        .unwrap_or_else(|| "svg".into());
                                    insert_imported_svg_masks(app, paths, name);
                                }
                            }
                            Err(error) => {
                                app.toasts.error(format!("SVG import failed: {error}"));
                            }
                        }
                    }
                    ui.close_menu();
                }
                if ui
                    .button("Import OBJ 3D Model...")
                    .on_hover_text("Import a Wavefront OBJ mesh as a camera-rendered 3D layer")
                    .clicked()
                {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Wavefront OBJ", &["obj"])
                        .pick_file()
                    {
                        match crate::core::project_migration::read_bounded_text_file(
                            &path,
                            128 * 1024 * 1024,
                        )
                        .and_then(|obj| {
                            let mesh = crate::core::obj_loader::parse_obj_str(&obj)?;
                            if mesh.vertices.is_empty() || mesh.indices.is_empty() {
                                return Err("OBJ contains no renderable faces".to_string());
                            }
                            Ok(())
                        }) {
                            Ok(()) => {
                                let name = path
                                    .file_stem()
                                    .map(|value| value.to_string_lossy().into_owned())
                                    .unwrap_or_else(|| "model".into());
                                insert_imported_model3d(app, path, name);
                            }
                            Err(error) => {
                                app.toasts.error(format!("OBJ import failed: {error}"));
                            }
                        }
                    }
                    ui.close_menu();
                }
                if ui.button("Import Image Sequence...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Image Sequence", &["png", "jpg", "jpeg", "bmp", "tga", "webp"])
                        .pick_file()
                    {
                        let fps = app.history.current().active_composition().fps.max(1) as f32;
                        let name = path
                            .file_stem()
                            .map(|value| value.to_string_lossy().to_string())
                            .unwrap_or_else(|| "image_sequence".to_string());
                        let destination = std::env::temp_dir()
                            .join("kagari_media")
                            .join("image_sequences")
                            .join(&name);
                        app.toasts.info(format!(
                            "Importing image sequence '{}' as WebP frames...",
                            name
                        ));
                        match crate::core::video_import::import_image_sequence(
                            &path,
                            &destination,
                            fps,
                        ) {
                            Ok(asset) => insert_imported_sequence(app, asset, name),
                            Err(error) => app.toasts.error(format!(
                                "Image sequence import failed: {error}"
                            )),
                        }
                    }
                    ui.close_menu();
                }
                if ui.button("Export MLT XML (Shotcut / Kdenlive)...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("MLT XML", &["xml", "mlt"])
                        .set_file_name(format!("{}.mlt.xml",
                            app.history.current().active_composition().name))
                        .save_file()
                    {
                        let comp = app.history.current().active_composition();
                        let xml = crate::core::mlt_export::MltExporter::export_to_xml(comp);
                        match std::fs::write(&path, xml) {
                            Ok(_) => {
                                app.toasts.info(format!(
                                    "Exported MLT XML: {}",
                                    path.to_string_lossy()
                                ));
                            }
                            Err(err) => {
                                app.toasts.error(format!("Failed to save MLT file: {}", err));
                            }
                        }
                    }
                    ui.close_menu();
                }
                if ui.button("Import OpenTimelineIO (.otio.json)").clicked() {
                    match crate::core::project_migration::read_bounded_text_file(
                        std::path::Path::new(&app.otio_path),
                        crate::core::project_migration::MAX_PROJECT_JSON_BYTES,
                    ) {
                        Ok(json) => match serde_json::from_str::<crate::core::integration::OtioTimeline>(&json) {
                            Ok(otio) => {
                                let comp = otio.to_composition();
                                app.modify_project(|p| p.compositions[0] = comp);
                                app.toasts.info(format!("Imported OTIO: {}", app.otio_path));
                            }
                            Err(err) => {
                                app.toasts.error(format!("Invalid OTIO format: {}", err));
                            }
                        },
                        Err(err) => {
                            app.toasts.error(format!("Could not read OTIO file: {}", err));
                        }
                    }
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Export Video (MP4)...").clicked() {
                    app.export.show_export_dialog = true;
                    ui.close_menu();
                }
                if ui.button("⚡ Quick Export Active Comp").on_hover_text("Export the active composition to MP4 with current settings (no dialog)").clicked() {
                    let comp_name = app.history.current().active_composition().name.clone();
                    crate::ui::export_dialog::start_comp_export(app, ctx, &comp_name);
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Exit").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.menu_button("Edit", |ui| {
                let undo_sc = crate::ui::shortcuts::format_shortcut("Z", true, false, false);
                let undo_btn = egui::Button::new("Undo (元に戻す)").shortcut_text(undo_sc);
                if ui.add_enabled(app.history.can_undo(), undo_btn).clicked() {
                    app.history.undo();
                    ui.close_menu();
                }
                let redo_sc = crate::ui::shortcuts::format_shortcut("Z", true, true, false);
                let redo_btn = egui::Button::new("Redo (やり直す)").shortcut_text(redo_sc);
                if ui.add_enabled(app.history.can_redo(), redo_btn).clicked() {
                    app.history.redo();
                    ui.close_menu();
                }
                if ui.button("🕘 Undo History…").on_hover_text("Open the named-step history panel and jump to any step").clicked() {
                    app.show_history_panel = !app.show_history_panel;
                    ui.close_menu();
                }
                ui.separator();
                if ui.add(egui::Button::new("Duplicate").shortcut_text("Cmd+D")).clicked() {
                    if let Some(sel_idx) = app.selection.selected_layer_idx {
                        let mut duplicated = false;
                        app.modify_project(|project| {
                            let comp = project.active_composition_mut();
                            if sel_idx < comp.layers.len() {
                                let mut cloned = comp.layers[sel_idx].clone();
                                let n = comp.layers.len();
                                cloned.id = format!("{}_copy_{}", cloned.id, n);
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
                if ui.add(egui::Button::new("Auto-Trace Layer")).on_hover_text("Trace selected layer's alpha into a new Shape layer (FreeformBezier)").clicked() {
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
                                contour = vec![
                                    [cx, cy],
                                    [cx + w, cy],
                                    [cx + w, cy + h],
                                    [cx, cy + h],
                                ];
                            }
                            if contour.len() >= 3 {
                                let len = contour.len();
                                let shape = crate::core::timeline::ShapeType::FreeformBezier {
                                    points: contour,
                                    tangents: (0..len).map(|_| ([0.0f32, 0.0], [0.0f32, 0.0])).collect(),
                                    closed: true,
                                };
                                let opacity = layer.transform.opacity.evaluate(cur_frame).clamp(0.0, 100.0);
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
                                app.toasts.info(format!("Auto-traced '{name}' into shape layer"));
                            } else {
                                app.toasts.error("Layer has no traceable mask or alpha data");
                            }
                        }
                    }
                    ui.close_menu();
                }
            });
            ui.menu_button("Composition", |ui| {
                if ui.add(egui::Button::new("New Composition...").shortcut_text("Cmd+N")).clicked() {
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
                let comp_sc = crate::ui::shortcuts::format_shortcut("E", true, true, false);
                let btn = egui::Button::new("Composition Settings...").shortcut_text(comp_sc);
                if ui.add(btn).clicked() {
                    app.comp_settings_draft = Some(app.history.current().clone());
                    app.show_comp_settings = true;
                    ui.close_menu();
                }
                let rq_sc = crate::ui::shortcuts::format_shortcut("M", true, false, false);
                if ui.add(egui::Button::new("Add to Render Queue").shortcut_text(rq_sc)).clicked() {
                    app.export.show_export_dialog = true;
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Export Captions (.srt)").on_hover_text("Write all 'Caption …' text layers as an SRT file").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Subtitles", &["srt"])
                        .set_file_name(format!("{}_captions.srt",
                            app.history.current().active_composition().name.replace(' ', "_")))
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
                if ui.button("Save Frame As… (PNG)").on_hover_text("Render the current frame at full resolution and save as PNG").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("PNG Image", &["png"])
                        .set_file_name(format!("{}_frame_{}.png",
                            app.history.current().active_composition().name,
                            app.playback.current_frame))
                        .save_file()
                    {
                        let comp = app.history.current().active_composition().clone();
                        let frame = app.playback.current_frame;
                        // 2× supersample render then alpha-weighted downsample
                        // for clean anti-aliased edges (clamped by raster max).
                        let max_dim = crate::core::software_renderer::MAX_RENDER_DIMENSION;
                        let sw = (comp.width.saturating_mul(2)).min(max_dim);
                        let sh = (comp.height.saturating_mul(2)).min(max_dim);
                        let px = crate::core::software_renderer::render_frame_to_pixels(
                            &comp, frame, sw, sh, 0.0, 0,
                        );
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
            });
            ui.menu_button("Layer", |ui| {
                ui.menu_button("New", |ui| {
                    let solid_sc = crate::ui::shortcuts::format_shortcut("Y", true, false, false);
                    if ui.add(egui::Button::new("Solid...").shortcut_text(solid_sc)).clicked() {
                        crate::ui::shortcuts::create_new_layer(app, crate::ui::shortcuts::NewLayerKind::Solid);
                        ui.close_menu();
                    }
                    let text_sc = crate::ui::shortcuts::format_shortcut("T", true, true, true);
                    if ui.add(egui::Button::new("Text").shortcut_text(text_sc)).clicked() {
                        crate::ui::shortcuts::create_new_layer(app, crate::ui::shortcuts::NewLayerKind::Text);
                        ui.close_menu();
                    }
                    let null_sc = crate::ui::shortcuts::format_shortcut("Y", true, true, true);
                    if ui.add(egui::Button::new("Null Object").shortcut_text(null_sc)).clicked() {
                        crate::ui::shortcuts::create_new_layer(app, crate::ui::shortcuts::NewLayerKind::Null);
                        ui.close_menu();
                    }
                    let adjustment_sc = crate::ui::shortcuts::format_shortcut("Y", true, false, true);
                    if ui.add(egui::Button::new("Adjustment Layer").shortcut_text(adjustment_sc)).clicked() {
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
                            layer.scene_object = Some(
                                crate::core::timeline::SceneObjectRef::Light { id: light_id.clone() },
                            );
                            comp.add_layer(layer);
                        });
                        app.toasts.info(format!("Added {}", name));
                        ui.close_menu();
                    }
                });
                ui.menu_button("Transform", |ui| {
                    // Center / Fit / Flip / Reset commands (AE Layer > Transform parity)
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
                                        let sx = cw / bs[0] * 100.0;
                                        let sy = ch / bs[1] * 100.0;
                                        let s = sx.max(sy);
                                        l.transform.scale = crate::core::property::Animatable::new_constant([s, s]);
                                        l.transform.position = crate::core::property::Animatable::new_constant([cw / 2.0, ch / 2.0]);
                                    }
                                }
                            });
                            app.toasts.info("Fit to Comp");
                            ui.close_menu();
                        }
                    }
                    if ui.button("Fit to Comp Width").clicked() {
                        if let Some(idx) = app.selection.selected_layer_idx {
                            let cw = app.history.current().active_composition().width as f32;
                            app.modify_project(move |p| {
                                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                                    let bs = l.bounding_size();
                                    if bs[0] > 1.0 {
                                        let s = cw / bs[0] * 100.0;
                                        l.transform.scale = crate::core::property::Animatable::new_constant([s, s]);
                                    }
                                }
                            });
                            app.toasts.info("Fit to Comp Width");
                            ui.close_menu();
                        }
                    }
                    if ui.button("Fit to Comp Height").clicked() {
                        if let Some(idx) = app.selection.selected_layer_idx {
                            let ch = app.history.current().active_composition().height as f32;
                            app.modify_project(move |p| {
                                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                                    let bs = l.bounding_size();
                                    if bs[1] > 1.0 {
                                        let s = ch / bs[1] * 100.0;
                                        l.transform.scale = crate::core::property::Animatable::new_constant([s, s]);
                                    }
                                }
                            });
                            app.toasts.info("Fit to Comp Height");
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
                    if ui.button("Reset Position").clicked() {
                        if let Some(idx) = app.selection.selected_layer_idx {
                            let dims = { let c = app.history.current().active_composition(); (c.width as f32, c.height as f32) };
                            app.modify_project(move |p| {
                                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                                    l.transform.position = crate::core::property::Animatable::new_constant([dims.0 / 2.0, dims.1 / 2.0]);
                                }
                            });
                            app.toasts.info("Position reset");
                            ui.close_menu();
                        }
                    }
                    if ui.button("Reset Scale").clicked() {
                        if let Some(idx) = app.selection.selected_layer_idx {
                            app.modify_project(move |p| {
                                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                                    l.transform.scale = crate::core::property::Animatable::new_constant([100.0, 100.0]);
                                }
                            });
                            app.toasts.info("Scale reset");
                            ui.close_menu();
                        }
                    }
                    if ui.button("Reset Rotation").clicked() {
                        if let Some(idx) = app.selection.selected_layer_idx {
                            app.modify_project(move |p| {
                                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                                    l.transform.rotation = crate::core::property::Animatable::new_constant(0.0);
                                }
                            });
                            app.toasts.info("Rotation reset");
                            ui.close_menu();
                        }
                    }
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
                    if ui.add(egui::Button::new("Auto-Orient...").shortcut_text("Cmd+Alt+O")).on_hover_text("Rotate layer automatically along its motion path direction").clicked() {
                        if let Some(idx) = app.selection.selected_layer_idx {
                            app.modify_project(move |p| {
                                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                                    l.auto_orient = match l.auto_orient {
                                        crate::core::auto_orient::AutoOrientMode::Off => crate::core::auto_orient::AutoOrientMode::OrientAlongPath,
                                        crate::core::auto_orient::AutoOrientMode::OrientAlongPath => crate::core::auto_orient::AutoOrientMode::Off,
                                        crate::core::auto_orient::AutoOrientMode::OrientTowardsPoint { .. } => crate::core::auto_orient::AutoOrientMode::Off,
                                    };
                                }
                            });
                            app.toasts.info("Toggled Auto-Orient along Motion Path");
                        }
                        ui.close_menu();
                    }
                });
                ui.menu_button("Time", |ui| {
                    if ui.add(egui::Button::new("Enable Time Remapping").shortcut_text("Cmd+Alt+T")).clicked() {
                        if let Some(idx) = app.selection.selected_layer_idx {
                            app.modify_project(move |p| {
                                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                                    l.enable_time_remapping();
                                }
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
                    if ui.add(egui::Button::new("🎯 Stabilize Motion (from Track)")).on_hover_text("Bake counter-movement position keyframes from the layer's first tracker").clicked() {
                        if let Some(idx) = app.selection.selected_layer_idx {
                            let mut baked_count = 0usize;
                            app.modify_project(|p| {
                                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                                    baked_count = crate::core::stabilizer::stabilize_layer_smoothed(l, 2);
                                }
                            });
                            if baked_count > 0 {
                                app.toasts.info(format!("Stabilized: {} position keyframes baked", baked_count));
                            } else {
                                app.toasts.error("Layer has no tracked data — run the Tracker first");
                            }
                        } else {
                            app.toasts.info("Select a layer first");
                        }
                        ui.close_menu();
                    }
                    if ui.add(egui::Button::new("Freeze Frame at Playhead")).clicked() {
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
                    if ui.button("Posterize Time 12fps (Stop Motion)").on_hover_text("Quantizes layer time to 12fps — toggles off if already enabled").clicked() {
                        if let Some(idx) = app.selection.selected_layer_idx {
                            let already = matches!(
                                app.history.current().active_composition().layers.get(idx).and_then(|l| l.posterize_time.as_ref()),
                                Some(pt) if pt.enabled
                            );
                            app.modify_project(move |p| {
                                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                                    l.posterize_time = if already {
                                        None
                                    } else {
                                        Some(crate::core::posterize_time::PosterizeTimeSettings::default())
                                    };
                                }
                            });
                            app.toasts.info(if already { "Posterize Time removed" } else { "Posterize Time: 12fps stop-motion" });
                        } else {
                            app.toasts.info("Select a layer first");
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.add(egui::Button::new("Time Stretch ×2 (Slow)")).clicked() {
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
                    if ui.add(egui::Button::new("Time Stretch ×0.5 (Fast)")).clicked() {
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
                });
                ui.menu_button("Arrange", |ui| {
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
                    if ui.add(egui::Button::new("Bring Forward").shortcut_text("Cmd+]")).clicked() {
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
                    if ui.add(egui::Button::new("Send Backward").shortcut_text("Cmd+[")).clicked() {
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
                        app.selection.selected_layer_idx = if i < len { Some(0) } else { app.selection.selected_layer_idx };
                        ui.close_menu();
                    }
                });
                ui.menu_button("Create", |ui| {
                    if ui.button("🔤 Create Shapes from Text").on_hover_text("Convert text characters into editable vector shape layer").clicked() {
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
                                                width:         crate::core::property::Animatable::new_constant(font_sz as f32 * text_str.len() as f32 * 0.6),
                                                height:        crate::core::property::Animatable::new_constant(font_sz as f32),
                                                corner_radius: crate::core::property::Animatable::new_constant(0.0),
                                            },
                                            color:        col,
                                            stroke_color: [0.0, 0.0, 0.0, 0.0],
                                            stroke_width: 0.0,
                                            fill_type:    crate::core::timeline::ShapeFillType::Solid,
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
                    if ui.button("🎭 Create Masks from Text").on_hover_text("Convert text outline into vector mask").clicked() {
                        if let Some(idx) = app.selection.selected_layer_idx {
                            let mut temp_proj = app.history.current().clone();
                            let comp = temp_proj.active_composition_mut();
                            if let Some(layer) = comp.layers.get_mut(idx) {
                                if let crate::core::timeline::LayerType::Text { font_size, text, .. } = &layer.layer_type {
                                    let w = *font_size as f32 * text.len() as f32 * 0.55;
                                    let h = *font_size as f32;
                                    let pos = layer.transform.position.evaluate(0);
                                    let mask = crate::core::mask::Mask::new_rect(
                                        format!("mask_text_{}", layer.masks.len() + 1),
                                        format!("Text Mask {}", layer.masks.len() + 1),
                                        pos[0] - w * 0.5, pos[1] - h * 0.5, w, h,
                                    );
                                    layer.masks.push(mask);
                                    app.commit_project(temp_proj);
                                    app.toasts.info("Converted Text into Vector Mask");
                                } else {
                                    app.toasts.error("Selected layer is not a Text layer");
                                }
                            }
                        }
                        ui.close_menu();
                    }
                });
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
                if ui.button("🔤 Create Shapes from Text").on_hover_text("Decompose selected Text layer into animatable vector Bezier Shape paths").clicked() {
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
                    if created {
                        app.toasts.info("🔤 Created Shapes from Text layer");
                    } else {
                        app.toasts.warning("Please select a Text layer first");
                    }
                    ui.close_menu();
                }
                ui.separator();
                if ui.add(egui::Button::new("Pre-Compose...").shortcut_text("Cmd+Shift+C")).clicked() {
                    ui.close_menu();
                }
            });
            ui.menu_button("Effect", |ui| {
                ui.menu_button("Expression Controls", |ui| {
                    if ui.button("Slider Control").on_hover_text("Keyframeable scalar — drive other layers via effect_param()").clicked() {
                        apply_effect_by_name(app, "Slider Control");
                        ui.close_menu();
                    }
                    if ui.button("Angle Control").clicked() {
                        apply_effect_by_name(app, "Angle Control");
                        ui.close_menu();
                    }
                    if ui.button("Point Control").clicked() {
                        apply_effect_by_name(app, "Point Control");
                        ui.close_menu();
                    }
                    if ui.button("Color Control").clicked() {
                        apply_effect_by_name(app, "Color Control");
                        ui.close_menu();
                    }
                });
                ui.menu_button("Blur & Sharpen", |ui| {
                    if ui.button("Gaussian Blur").clicked() {
                        apply_effect_by_name(app, "Gaussian Blur");
                        ui.close_menu();
                    }
                    if ui.button("Directional Blur").clicked() {
                        apply_effect_by_name(app, "Directional Blur");
                        ui.close_menu();
                    }
                    if ui.button("Radial Blur").clicked() {
                        apply_effect_by_name(app, "Radial Blur");
                        ui.close_menu();
                    }
                    if ui.button("Sharpen").clicked() {
                        apply_effect_by_name(app, "Sharpen");
                        ui.close_menu();
                    }
                    if ui.button("Camera Lens Blur").on_hover_text("Optical camera aperture defocus with iris polygonal blade shapes and highlight gain").clicked() {
                        apply_effect_by_name(app, "Camera Lens Blur");
                        ui.close_menu();
                    }
                });
                ui.menu_button("Color Correction", |ui| {
                    if ui.button("Color Tint").clicked() {
                        apply_effect_by_name(app, "Color Tint");
                        ui.close_menu();
                    }
                    if ui.button("Levels").clicked() {
                        apply_effect_by_name(app, "Levels");
                        ui.close_menu();
                    }
                    if ui.button("Hue/Saturation").clicked() {
                        apply_effect_by_name(app, "Hue/Saturation");
                        ui.close_menu();
                    }
                    if ui.button("Vibrance").clicked() {
                        apply_effect_by_name(app, "Vibrance");
                        ui.close_menu();
                    }
                });
                ui.menu_button("Channel", |ui| {
                    if ui.button("Set Matte").on_hover_text("Replace or composite alpha channel with another layer").clicked() {
                        apply_effect_by_name(app, "Set Matte");
                        ui.close_menu();
                    }
                    if ui.button("Shift Channels").clicked() {
                        apply_effect_by_name(app, "Shift Channels");
                        ui.close_menu();
                    }
                    if ui.button("Channel Combiner").on_hover_text("Extract and remap color and alpha channels (Luma, Hue, Sat, RGB)").clicked() {
                        apply_effect_by_name(app, "Channel Combiner");
                        ui.close_menu();
                    }
                });
                ui.menu_button("Keying", |ui| {
                    if ui.button("Linear Color Key").on_hover_text("Key out specific color ranges in RGB, Hue, or Chroma space").clicked() {
                        apply_effect_by_name(app, "Linear Color Key");
                        ui.close_menu();
                    }
                });
                ui.menu_button("Time", |ui| {
                    if ui.button("Echo").on_hover_text("Temporal visual trail blending across multiple frames").clicked() {
                        apply_effect_by_name(app, "Echo");
                        ui.close_menu();
                    }
                });
                if crate::ui::mode::menu_visible(app.ui_mode, "OpenFX Plugins") { ui.menu_button("OpenFX Plugins", |ui| {
                    if ui.button("Scan Standard Paths...").on_hover_text("Search /Library/OFX/Plugins and $OFX_PLUGIN_PATH for plugin bundles, then probe their ABI exports").clicked() {
                        let found = crate::core::openfx_bridge::discover_all_ofx_plugins();
                        if found.is_empty() {
                            app.toasts.info("No OpenFX plugins found in standard paths");
                        } else {
                            let mut loadable = 0usize;
                            let mut names: Vec<String> = Vec::new();
                            for p in &found {
                                if let crate::core::openfx_bridge::OfxProbeResult::Loaded { plugin_version, .. } =
                                    crate::core::openfx_bridge::probe_ofx_plugin(&p.binary_path)
                                {
                                    loadable += 1;
                                    if names.len() < 4 {
                                        names.push(format!("{} v{}.{}", p.name, plugin_version.0, plugin_version.1));
                                    }
                                }
                            }
                            if loadable > 0 {
                                app.toasts.info(format!(
                                    "{loadable}/{} OFX effect(s) loadable: {}{}",
                                    found.len(),
                                    names.join(", "),
                                    if loadable > 4 { "…" } else { "" }
                                ));
                            } else {
                                app.toasts.info(format!("Found {} bundle(s), none expose OfxImageEffectAPI", found.len()));
                            }
                        }
                        ui.close_menu();
                    }
                });
                }
                ui.menu_button("Stylize", |ui| {
                    if ui.button("Glow").clicked() {
                        apply_effect_by_name(app, "Glow");
                        ui.close_menu();
                    }
                    if ui.button("Lens Flare (GPU)").clicked() {
                        apply_effect_by_name(app, "Lens Flare");
                        ui.close_menu();
                    }
                    if ui.button("Optical Flares (Cinematic)").on_hover_text("Multi-element physical lens flare with anamorphic streaks and iris ghosts").clicked() {
                        apply_effect_by_name(app, "Optical Flares");
                        ui.close_menu();
                    }
                    if ui.button("Vignette").clicked() {
                        apply_effect_by_name(app, "Vignette");
                        ui.close_menu();
                    }
                    if ui.button("Film Grain").clicked() {
                        apply_effect_by_name(app, "Film Grain");
                        ui.close_menu();
                    }
                    if ui.button("Drop Shadow").clicked() {
                        apply_effect_by_name(app, "Drop Shadow");
                        ui.close_menu();
                    }
                    if ui.button("Find Edges").on_hover_text("Emphasize color transitions and borders").clicked() {
                        apply_effect_by_name(app, "Find Edges");
                        ui.close_menu();
                    }
                });
                ui.menu_button("Distort", |ui| {
                    if ui.button("Twirl").clicked() {
                        apply_effect_by_name(app, "Twirl");
                        ui.close_menu();
                    }
                    if ui.button("Bulge").clicked() {
                        apply_effect_by_name(app, "Bulge");
                        ui.close_menu();
                    }
                    if ui.button("Mesh Warp").clicked() {
                        apply_effect_by_name(app, "Mesh Warp");
                        ui.close_menu();
                    }
                    if ui.button("Chromatic Aberration").clicked() {
                        apply_effect_by_name(app, "Chromatic Aberration");
                        ui.close_menu();
                    }
                    if ui.button("Motion Tile").on_hover_text("Seamlessly replicate layer content with mirror edges and phase").clicked() {
                        apply_effect_by_name(app, "Motion Tile");
                        ui.close_menu();
                    }
                    if ui.button("CC Page Turn").on_hover_text("3D cylindrical page peel & curl deformation").clicked() {
                        apply_effect_by_name(app, "CC Page Turn");
                        ui.close_menu();
                    }
                    if ui.button("Transform").on_hover_text("2D Affine transform (Anchor, Position, Scale, Skew, Rotation, Opacity) in effect chain").clicked() {
                        apply_effect_by_name(app, "Transform");
                        ui.close_menu();
                    }
                });
                ui.menu_button("Generate", |ui| {
                    if ui.button("⚡ Lightning").on_hover_text("Procedural electric lightning arcs with glow").clicked() {
                        apply_effect_by_name(app, "Lightning");
                        ui.close_menu();
                    }
                    if ui.button("🔴 Laser Beam").on_hover_text("High-energy projectile laser beam with customizable core and glow").clicked() {
                        apply_effect_by_name(app, "Laser Beam");
                        ui.close_menu();
                    }
                });
            });
            ui.menu_button("Animation", |ui| {
                ui.menu_button("Keyframe Assistant", |ui| {
                    if ui.button("🎵 Convert Audio to Keyframes").on_hover_text("Extract RMS amplitude from audio layer into Slider Controls").clicked() {
                        let mut audio_source: Option<String> = None;
                        let comp = app.history.current().active_composition();
                        if let Some(idx) = app.selection.selected_layer_idx {
                            if let Some(l) = comp.layers.get(idx) {
                                if let crate::core::timeline::LayerType::Audio { path, .. } = &l.layer_type {
                                    audio_source = Some(path.clone());
                                } else if let crate::core::timeline::LayerType::Video { audio_wav: Some(w), .. } = &l.layer_type {
                                    audio_source = Some(w.clone());
                                }
                            }
                        }
                        if let Some(src) = audio_source {
                            let mut temp_proj = app.history.current().clone();
                            match crate::core::audio_to_keyframes::convert_audio_to_keyframes(temp_proj.active_composition_mut(), &src) {
                                Ok(name) => {
                                    app.commit_project(temp_proj);
                                    app.toasts.info(format!("Created '{}' with Left/Right/Both channels", name));
                                }
                                Err(e) => app.toasts.error(e),
                            }
                        } else {
                            app.toasts.error("Select an Audio or Video-with-Audio layer first");
                        }
                        ui.close_menu();
                    }
                    if ui.button("🎧 Convert Multi-Band Audio (Bass/Mid/Treble)").on_hover_text("Extract frequency-separated amplitude (Master, Bass, Mid, Treble) into Sliders").clicked() {
                        let mut audio_source: Option<String> = None;
                        let comp = app.history.current().active_composition();
                        if let Some(idx) = app.selection.selected_layer_idx {
                            if let Some(l) = comp.layers.get(idx) {
                                if let crate::core::timeline::LayerType::Audio { path, .. } = &l.layer_type {
                                    audio_source = Some(path.clone());
                                } else if let crate::core::timeline::LayerType::Video { audio_wav: Some(w), .. } = &l.layer_type {
                                    audio_source = Some(w.clone());
                                }
                            }
                        }
                        if let Some(src) = audio_source {
                            let mut temp_proj = app.history.current().clone();
                            match crate::core::audio_to_keyframes::convert_multiband_audio_to_keyframes(temp_proj.active_composition_mut(), &src, None) {
                                Ok(name) => {
                                    app.commit_project(temp_proj);
                                    app.toasts.info(format!("Created '{}' with Master/Bass/Mid/Treble", name));
                                }
                                Err(e) => app.toasts.error(e),
                            }
                        } else {
                            app.toasts.error("Select an Audio or Video-with-Audio layer first");
                        }
                        ui.close_menu();
                    }
                    if ui.button("📈 Exponential Scale").on_hover_text("Convert linear scale keyframes into exponential logarithmic zoom").clicked() {
                        if let Some(idx) = app.selection.selected_layer_idx {
                            let mut temp_proj = app.history.current().clone();
                            let comp = temp_proj.active_composition_mut();
                            if let Some(layer) = comp.layers.get_mut(idx) {
                                if let crate::core::property::Animatable::Animated(ref mut kfs) = layer.transform.scale {
                                    if kfs.len() >= 2 {
                                        let (Some(first), Some(last)) = (kfs.first(), kfs.last()) else {
                                            return;
                                        };
                                        let first = first.clone();
                                        let last = last.clone();
                                        let (f0, f1) = (first.frame as f32, last.frame as f32);
                                        let (s0, s1) = (first.value[0].max(0.01), last.value[0].max(0.01));
                                        let mut exp_kfs = Vec::new();
                                        let count = (last.frame - first.frame).max(1);
                                        for step in 0..=count {
                                            let f = first.frame + step;
                                            let t = (f as f32 - f0) / (f1 - f0).max(1.0);
                                            let log_val = (s0.ln() + t * (s1.ln() - s0.ln())).exp();
                                            exp_kfs.push(crate::core::keyframe::Keyframe::new(f, [log_val, log_val], crate::core::keyframe::InterpolationType::Linear));
                                        }
                                        *kfs = exp_kfs;
                                        app.commit_project(temp_proj);
                                        app.toasts.info("Converted scale to Exponential Zoom curve");
                                    }
                                }
                            }
                        }
                        ui.close_menu();
                    }
                    if ui.button("⏪ Time-Reverse Keyframes").clicked() {
                        if let Some(idx) = app.selection.selected_layer_idx {
                            let mut temp_proj = app.history.current().clone();
                            let comp = temp_proj.active_composition_mut();
                            if let Some(layer) = comp.layers.get_mut(idx) {
                                fn rev_vec2(a: &mut crate::core::property::Animatable<[f32; 2]>, in_f: u32, out_f: u32) {
                                    if let crate::core::property::Animatable::Animated(kfs) = a {
                                        let span = out_f.saturating_sub(in_f);
                                        for k in kfs.iter_mut() {
                                            k.frame = in_f + span.saturating_sub(k.frame.saturating_sub(in_f));
                                        }
                                        kfs.sort_by_key(|k| k.frame);
                                    }
                                }
                                let (inf, outf) = (layer.in_frame, layer.out_frame);
                                rev_vec2(&mut layer.transform.position, inf, outf);
                                rev_vec2(&mut layer.transform.scale, inf, outf);
                                app.commit_project(temp_proj);
                                app.toasts.info("Keyframes time-reversed");
                            }
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.add(egui::Button::new("Easy Ease").shortcut_text("F9")).clicked() {
                        if let Some(idx) = app.selection.selected_layer_idx {
                            app.modify_project(move |p| {
                                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                                    l.easy_ease_transform();
                                }
                            });
                            app.toasts.info("Easy Ease applied to transform keyframes");
                        }
                        ui.close_menu();
                    }
                    if ui.button("Sequence Layers...").clicked() {
                        app.show_sequence_layers = true;
                        ui.close_menu();
                    }
                });
                if ui.button("🌊 The Smoother...").on_hover_text("Reduce keyframe density with RDP curve simplification").clicked() {
                    app.show_the_smoother = true;
                    ui.close_menu();
                }
                if ui.button("🎲 The Wiggler...").on_hover_text("Bake procedural noise keyframes into layer properties").clicked() {
                    app.show_the_wiggler = true;
                    ui.close_menu();
                }
                if ui.button("✏️ Motion Sketch...").on_hover_text("Record real-time mouse dragging in viewport to position keyframes").clicked() {
                    app.show_motion_sketch = true;
                    ui.close_menu();
                }
                if ui.button("⚛ Physics & Dynamics...").on_hover_text("2D Rigid Body Collision Simulation & Keyframe Baker").clicked() {
                    app.show_physics = true;
                    ui.close_menu();
                }
            });
            ui.menu_button("View", |ui| {
                // ── UI Mode (skill level) ──
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("UI Mode").small().color(egui::Color32::from_rgb(140,140,148)));
                    let mode = app.ui_mode;
                    if ui.selectable_label(mode.is_beginner(), "初心者").on_hover_text("Hide advanced panels and menus").clicked() {
                        app.ui_mode = crate::ui::mode::UiMode::Beginner;
                        crate::ui::mode::save_mode(app.ui_mode);
                        app.toasts.info("初心者モード: 高度なパネルを非表示にしました");
                    }
                    if ui.selectable_label(mode.is_advanced(), "上級者").on_hover_text("Show every panel, menu and tool").clicked() {
                        app.ui_mode = crate::ui::mode::UiMode::Advanced;
                        crate::ui::mode::save_mode(app.ui_mode);
                        app.toasts.info("上級者モード: 全機能を表示中");
                    }
                });
                ui.separator();
                if ui.button("🎓 Restart Tutorial").clicked() {
                    crate::ui::tutorial::restart(app);
                    ui.close_menu();
                }
                ui.separator();
                ui.checkbox(&mut app.show_grid, "Show Grid");
                ui.checkbox(&mut app.show_guides, "Show Safe Zones");
                ui.checkbox(&mut app.show_handles, "Show Handles");
                if ui.button("📊 Analyze / Quality Check…").clicked() {
                    app.show_quality_check_panel = true;
                    ui.close_menu();
                }
                if ui.button("🎚 Automation Bindings…").clicked() {
                    app.show_automation_panel = true;
                    ui.close_menu();
                }
                ui.separator();
                // ── GPU Compute (experimental) ──
                let mut gpu_fx = crate::core::compute_pipeline::gpu_effects_enabled();
                if ui.checkbox(&mut gpu_fx, "GPU Compute Effects (beta)").changed() {
                    crate::core::compute_pipeline::set_gpu_effects_enabled(gpu_fx);
                    app.modify_project(|project| project.use_gpu_compute = gpu_fx);
                    if gpu_fx {
                        match crate::core::compute_pipeline::global() {
                            Some(ctx) => app.toasts.info(format!("GPU compute: {}", ctx.backend_label())),
                            None => {
                                app.toasts.error("No GPU adapter — staying on CPU");
                                crate::core::compute_pipeline::set_gpu_effects_enabled(false);
                                app.modify_project(|project| project.use_gpu_compute = false);
                            }
                        }
                    } else {
                        app.toasts.info("GPU compute off — deterministic CPU rendering");
                    }
                }
                if crate::core::compute_pipeline::gpu_effects_enabled() {
                    ui.label(
                        egui::RichText::new(crate::core::compute_pipeline::timing_hud_line())
                            .small()
                            .color(egui::Color32::from_rgb(110, 110, 110)),
                    );
                }
                if ui.button("Reset Timeline Zoom (100%)").clicked() {
                    app.timeline_zoom = 1.0;
                    ui.close_menu();
                }
                if ui.button("Purge All RAM Cache").clicked() {
                    app.frame_cache.invalidate_all();
                    crate::core::frame_cache::bump_version();
                    ui.close_menu();
                }
                ui.menu_button("Workspaces", |ui| {
                    if ui.button("Standard").clicked() {
                        app.ui_tabs.right_tab_idx = 0;
                        app.show_graph_editor = false;
                        ui.close_menu();
                    }
                    if ui.button("Motion Graphics").clicked() {
                        app.ui_tabs.right_tab_idx = 0;
                        app.show_graph_editor = true;
                        ui.close_menu();
                    }
                    if ui.button("VFX & Color").clicked() {
                        app.ui_tabs.right_tab_idx = 30; // Properties
                        app.show_graph_editor = false;
                        ui.close_menu();
                    }
                    if ui.button("Audio Editing").clicked() {
                        app.ui_tabs.right_tab_idx = 7; // Audio Panel
                        app.show_graph_editor = false;
                        ui.close_menu();
                    }
                });
            });
            ui.menu_button("Help", |ui| {
                if ui.add(egui::Button::new("⚙ Preferences…").shortcut_text("Cmd+,")).clicked() {
                    app.show_preferences = true;
                    ui.close_menu();
                }
                if ui.button("✨ Show Welcome Screen").clicked() {
                    app.show_welcome = true;
                    ui.close_menu();
                }
                if ui.button("Keyboard Shortcuts Reference...").clicked() {
                    let help_id = egui::Id::new("show_shortcuts_modal");
                    ctx.data_mut(|d| d.insert_temp(help_id, true));
                    ui.close_menu();
                }
                if ui.button("🎓 Start Guided Tutorial...").clicked() {
                    app.show_guided_tutorial = true;
                    app.tutorial_step = 0;
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("ℹ️ About Kagari Studio...").clicked() {
                    let about_id = egui::Id::new("show_about_modal");
                    ctx.data_mut(|d| d.insert_temp(about_id, true));
                    ui.close_menu();
                }
            });

            // Right-aligned UI Mode Switcher (Beginner vs Pro)
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let (btn_text, next_mode) = match app.skill_level {
                    crate::app_state::SkillLevel::Beginner => ("🔰 Mode: Beginner (Simple)", crate::app_state::SkillLevel::Pro),
                    crate::app_state::SkillLevel::Pro => ("⚡ Mode: Pro Studio (Full)", crate::app_state::SkillLevel::Beginner),
                };
                if ui.button(btn_text).on_hover_text("Click to toggle between Simple Beginner UI and Full Pro Studio Layout").clicked() {
                    app.skill_level = next_mode;
                    app.toasts.info(format!("Switched UI Mode to {:?}", next_mode));
                }
            });
        });
    });

    let mut show_tutorial = app.show_guided_tutorial;
    if show_tutorial {
        let mut finish_tutorial = false;
        egui::Window::new("🎓 Kagari VFX — Quickstart Guided Tour")
            .open(&mut show_tutorial)
            .resizable(false)
            .show(ctx, |ui| {
                ui.heading("Welcome to Kagari VFX Motion Graphics!");
                ui.add_space(4.0);
                match app.tutorial_step {
                    0 => {
                        ui.label("Step 1: Composition & Layers");
                        ui.label("• Left panel contains your Project assets and composition hierarchy.");
                        ui.label("• Press 'T' on a selected layer to reveal Opacity, or 'P' for Position.");
                    }
                    1 => {
                        ui.label("Step 2: Keyframing & Animation");
                        ui.label("• Click the stopwatch icon next to any property in the Inspector to add a keyframe.");
                        ui.label("• Press Spacebar to start smooth real-time GPU/CPU RAM preview playback.");
                    }
                    2 => {
                        ui.label("Step 3: Effects & 3D Extrusion");
                        ui.label("• Drag effects from the Effects Library into your layers.");
                        ui.label("• Switch layers to 3D and enable Cinema 4D-style extrusion and ray-traced soft shadows.");
                    }
                    _ => {
                        ui.label("Step 4: Export & Sharing");
                        ui.label("• Press Cmd+M (or File -> Export) to render high-quality MP4, ProRes, or Lottie JSON.");
                        ui.label("• You are ready to create mind-blowing motion graphics!");
                    }
                }
                ui.add_space(8.0);
                ui.separator();
                ui.horizontal(|ui| {
                    if app.tutorial_step > 0 && ui.button("◀ Previous").clicked() {
                        app.tutorial_step -= 1;
                    }
                    if app.tutorial_step < 3 {
                        if ui.button("Next Step ▶").clicked() {
                            app.tutorial_step += 1;
                        }
                    } else if ui.button("Finish Tour 🎉").clicked() {
                        finish_tutorial = true;
                    }
                });
            });
        if finish_tutorial {
            show_tutorial = false;
            app.toasts.info("Tutorial completed! Have fun animating!");
        }
        app.show_guided_tutorial = show_tutorial;
    }

    let about_id = egui::Id::new("show_about_modal");
    let mut show_about = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(about_id, || false));
    if show_about {
        egui::Window::new("About Kagari Studio")
            .open(&mut show_about)
            .resizable(false)
            .show(ctx, |ui| {
                ui.heading("Kagari Studio (篝)");
                ui.label(format!("Version: {} (Open Source)", env!("CARGO_PKG_VERSION")));
                ui.label("Licensed MIT OR Apache-2.0 — see LICENSE-MIT, LICENSE-APACHE,");
                ui.label("and THIRD-PARTY-NOTICES in the repository root.");
                ui.label("A high-performance Motion Graphics & 32bpc HDR Visual Effects engine written in Rust.");
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(4.0);
                ui.small("Kagari Studio is an independent open-source project unrelated to any commercial compositing software.");
                ui.add_space(8.0);
            });
        ctx.data_mut(|d| d.insert_temp(about_id, show_about));
    }

    let help_id = egui::Id::new("show_shortcuts_modal");
    let mut show_help = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(help_id, || false));
    if show_help {
        egui::Window::new("Keyboard Shortcuts Reference")
            .open(&mut show_help)
            .resizable(false)
            .show(ctx, |ui| {
                ui.heading("Kagari VFX — Shortcuts Reference");
                ui.separator();
                egui::Grid::new("shortcuts_grid")
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("Spacebar");
                        ui.label("Play / Pause RAM Preview");
                        ui.end_row();
                        ui.label("V");
                        ui.label("Selection Tool");
                        ui.end_row();
                        ui.label("H");
                        ui.label("Hand Tool (Pan)");
                        ui.end_row();
                        ui.label("Z");
                        ui.label("Zoom Tool");
                        ui.end_row();
                        ui.label("W");
                        ui.label("Rotation Tool");
                        ui.end_row();
                        ui.label("Y");
                        ui.label("Anchor Point Tool");
                        ui.end_row();
                        ui.label("Cmd + Z");
                        ui.label("Undo");
                        ui.end_row();
                        ui.label("Cmd + Shift + Z");
                        ui.label("Redo");
                        ui.end_row();
                        ui.label("J / K / L");
                        ui.label("Prev Keyframe / Stop / Next Keyframe");
                        ui.end_row();
                        ui.label("Arrow Keys");
                        ui.label("Nudge Selected Layer 1px (Shift = 10px)");
                        ui.end_row();
                        ui.label("PageUp / PageDown");
                        ui.label("Step Frame Backward / Forward");
                        ui.end_row();
                        ui.label("Home / End");
                        ui.label("First / Last Frame");
                        ui.end_row();
                        ui.label("B / N");
                        ui.label("Set Work Area Start / End");
                        ui.end_row();
                        ui.label("= / -");
                        ui.label("Timeline Zoom In / Out (Cmd+Scroll on ruler)");
                        ui.end_row();
                        ui.label("F9");
                        ui.label("Apply Easy Ease to Keyframes");
                        ui.end_row();
                        ui.label("Click / Shift+Click");
                        ui.label("Select / Add to Selection (keyframes)");
                        ui.end_row();
                        ui.label(", / .");
                        ui.label("Nudge Selected Keyframes (Shift = x10)");
                        ui.end_row();
                        ui.label("Delete");
                        ui.label("Delete Selected Keyframes");
                        ui.end_row();
                        ui.label("Cmd + C / V");
                        ui.label("Copy / Paste Selected Keyframes");
                        ui.end_row();
                        ui.label("Cmd + A");
                        ui.label("Select All Keyframes of Layer");
                        ui.end_row();
                        ui.label("Esc");
                        ui.label("Deselect (Keyframes, then Layers)");
                        ui.end_row();
                        ui.label("M");
                        ui.label("Add / Remove Timeline Marker");
                        ui.end_row();
                        ui.label("[ / ]");
                        ui.label("Jump to Prev / Next Marker");
                        ui.end_row();
                    });
            });
        ctx.data_mut(|d| d.insert_temp(help_id, show_help));
    }

    // 📦 Pre-Compose Dialog (Cmd+Shift+C)
    crate::ui::precompose_dialog::draw_precompose_dialog(app, ctx);
    crate::ui::recovery_dialog::draw_recovery_dialog(app, ctx);
    crate::ui::sequence_layers_dialog::draw_sequence_layers_dialog(app, ctx);

    if app.show_the_smoother {
        let mut open = app.show_the_smoother;
        egui::Window::new("🌊 The Smoother")
            .open(&mut open)
            .resizable(false)
            .default_width(280.0)
            .show(ctx, |ui| {
                crate::ui::the_smoother_panel::draw_the_smoother_panel(app, ui);
            });
        app.show_the_smoother = open;
    }

    if app.show_the_wiggler {
        let mut open = app.show_the_wiggler;
        egui::Window::new("🎲 The Wiggler")
            .open(&mut open)
            .resizable(false)
            .default_width(280.0)
            .show(ctx, |ui| {
                crate::ui::the_wiggler_panel::draw_the_wiggler_panel(app, ui);
            });
        app.show_the_wiggler = open;
    }

    if app.show_motion_sketch {
        let mut open = app.show_motion_sketch;
        egui::Window::new("✏️ Motion Sketch")
            .open(&mut open)
            .resizable(false)
            .default_width(280.0)
            .show(ctx, |ui| {
                crate::ui::motion_sketch_panel::draw_motion_sketch_panel(app, ui);
            });
        app.show_motion_sketch = open;
    }

    if app.show_physics {
        let mut open = app.show_physics;
        egui::Window::new("⚛ Physics & Dynamics")
            .open(&mut open)
            .resizable(false)
            .default_width(300.0)
            .show(ctx, |ui| {
                crate::ui::physics_panel::draw_physics_panel(app, ui);
            });
        app.show_physics = open;
    }
}

fn apply_effect_by_name(app: &mut crate::KagariApp, effect_name: &str) {
    if let Some(idx) = app.selection.selected_layer_idx {
        let effect_name = effect_name.to_string();
        let mut added = false;
        app.modify_project(|project| {
            let comp = project.active_composition_mut();
            if idx < comp.layers.len() {
                let layer = &mut comp.layers[idx];
                let len = layer.effects.len();
                let effect = match effect_name.as_str() {
                "Slider Control" => crate::core::timeline::Effect {
                    id: format!("slider_{}", len),
                    name: "Slider Control".to_string(),
                    effect_type: crate::core::timeline::EffectType::SliderControl {
                        value: crate::core::property::Animatable::new_constant(50.0),
                    },
                    enabled: true,
                },
                "Angle Control" => crate::core::timeline::Effect {
                    id: format!("angle_{}", len),
                    name: "Angle Control".to_string(),
                    effect_type: crate::core::timeline::EffectType::AngleControl {
                        angle_degrees: crate::core::property::Animatable::new_constant(0.0),
                    },
                    enabled: true,
                },
                "Point Control" => crate::core::timeline::Effect {
                    id: format!("point_{}", len),
                    name: "Point Control".to_string(),
                    effect_type: crate::core::timeline::EffectType::PointControl {
                        point: crate::core::property::Animatable::new_constant([960.0, 540.0]),
                    },
                    enabled: true,
                },
                "Color Control" => crate::core::timeline::Effect {
                    id: format!("color_{}", len),
                    name: "Color Control".to_string(),
                    effect_type: crate::core::timeline::EffectType::ColorControl {
                        color: crate::core::property::Animatable::new_constant([
                            1.0, 1.0, 1.0, 1.0,
                        ]),
                    },
                    enabled: true,
                },
                "Checkbox Control" => crate::core::timeline::Effect {
                    id: format!("checkbox_{}", len),
                    name: "Checkbox Control".to_string(),
                    effect_type: crate::core::timeline::EffectType::CheckboxControl {
                        checked: false,
                    },
                    enabled: true,
                },
                "Dropdown Control" => crate::core::timeline::Effect {
                    id: format!("dropdown_{}", len),
                    name: "Dropdown Control".to_string(),
                    effect_type: crate::core::timeline::EffectType::DropdownControl {
                        value: 0,
                        options: vec![
                            "Option 1".to_string(),
                            "Option 2".to_string(),
                            "Option 3".to_string(),
                        ],
                    },
                    enabled: true,
                },
                "3D Point Control" => crate::core::timeline::Effect {
                    id: format!("point3d_{}", len),
                    name: "3D Point Control".to_string(),
                    effect_type: crate::core::timeline::EffectType::Point3DControl {
                        point: crate::core::property::Animatable::new_constant([0.0, 0.0, 0.0]),
                    },
                    enabled: true,
                },
                "Lens Flare" => crate::core::timeline::Effect {
                    id: format!("flare_{}", len),
                    name: "Lens Flare".to_string(),
                    effect_type: crate::core::timeline::EffectType::LensFlare {
                        enabled: crate::core::property::Animatable::new_constant(1.0),
                        position_x: crate::core::property::Animatable::new_constant(0.5),
                        position_y: crate::core::property::Animatable::new_constant(0.35),
                        intensity: crate::core::property::Animatable::new_constant(1.0),
                        threshold: crate::core::property::Animatable::new_constant(0.8),
                        color: crate::core::property::Animatable::new_constant([
                            1.0, 0.95, 0.9, 1.0,
                        ]),
                        link_to_light: None,
                    },
                    enabled: true,
                },
                "Gaussian Blur" => crate::core::timeline::Effect {
                    id: format!("blur_{}", len),
                    name: "Gaussian Blur".to_string(),
                    effect_type: crate::core::timeline::EffectType::GaussianBlur {
                        blur_radius: crate::core::property::Animatable::new_constant(5.0),
                    },
                    enabled: true,
                },
                "Directional Blur" => crate::core::timeline::Effect {
                    id: format!("dirblur_{}", len),
                    name: "Directional Blur".to_string(),
                    effect_type: crate::core::timeline::EffectType::DirectionalBlur {
                        angle: crate::core::property::Animatable::new_constant(0.0),
                        length: crate::core::property::Animatable::new_constant(10.0),
                    },
                    enabled: true,
                },
                "Radial Blur" => crate::core::timeline::Effect {
                    id: format!("radblur_{}", len),
                    name: "Radial Blur".to_string(),
                    effect_type: crate::core::timeline::EffectType::RadialBlur {
                        amount: crate::core::property::Animatable::new_constant(10.0),
                    },
                    enabled: true,
                },
                "Sharpen" => crate::core::timeline::Effect {
                    id: format!("sharp_{}", len),
                    name: "Sharpen".to_string(),
                    effect_type: crate::core::timeline::EffectType::Sharpen {
                        amount: crate::core::property::Animatable::new_constant(50.0),
                    },
                    enabled: true,
                },
                "Color Tint" => crate::core::timeline::Effect {
                    id: format!("tint_{}", len),
                    name: "Color Tint".to_string(),
                    effect_type: crate::core::timeline::EffectType::ColorTint {
                        color: crate::core::property::Animatable::new_constant([
                            1.0, 0.2, 0.4, 1.0,
                        ]),
                        intensity: crate::core::property::Animatable::new_constant(1.0),
                    },
                    enabled: true,
                },
                "Levels" => crate::core::timeline::Effect {
                    id: format!("levels_{}", len),
                    name: "Levels".to_string(),
                    effect_type: crate::core::timeline::EffectType::Levels {
                        input_black: crate::core::property::Animatable::new_constant(0.0),
                        input_white: crate::core::property::Animatable::new_constant(255.0),
                        gamma: crate::core::property::Animatable::new_constant(1.0),
                        output_black: crate::core::property::Animatable::new_constant(0.0),
                        output_white: crate::core::property::Animatable::new_constant(255.0),
                    },
                    enabled: true,
                },
                "Hue/Saturation" => crate::core::timeline::Effect {
                    id: format!("hs_{}", len),
                    name: "Hue/Saturation".to_string(),
                    effect_type: crate::core::timeline::EffectType::HueSaturation {
                        hue_shift: crate::core::property::Animatable::new_constant(0.0),
                        saturation: crate::core::property::Animatable::new_constant(0.0),
                        lightness: crate::core::property::Animatable::new_constant(0.0),
                    },
                    enabled: true,
                },
                "Vibrance" => crate::core::timeline::Effect {
                    id: format!("vib_{}", len),
                    name: "Vibrance".to_string(),
                    effect_type: crate::core::timeline::EffectType::Vibrance {
                        amount: crate::core::property::Animatable::new_constant(50.0),
                    },
                    enabled: true,
                },
                "Glow" => crate::core::timeline::Effect {
                    id: format!("glow_{}", len),
                    name: "Glow".to_string(),
                    effect_type: crate::core::timeline::EffectType::Glow {
                        threshold: crate::core::property::Animatable::new_constant(60.0),
                        radius: crate::core::property::Animatable::new_constant(10.0),
                        intensity: crate::core::property::Animatable::new_constant(1.0),
                        color: crate::core::property::Animatable::new_constant([
                            1.0, 1.0, 1.0, 1.0,
                        ]),
                    },
                    enabled: true,
                },
                "Vignette" => crate::core::timeline::Effect {
                    id: format!("vig_{}", len),
                    name: "Vignette".to_string(),
                    effect_type: crate::core::timeline::EffectType::Vignette {
                        intensity: crate::core::property::Animatable::new_constant(0.5),
                        roundness: crate::core::property::Animatable::new_constant(0.5),
                        feather: crate::core::property::Animatable::new_constant(0.5),
                        color: crate::core::property::Animatable::new_constant([
                            0.0, 0.0, 0.0, 1.0,
                        ]),
                    },
                    enabled: true,
                },
                "Film Grain" => crate::core::timeline::Effect {
                    id: format!("grain_{}", len),
                    name: "Film Grain".to_string(),
                    effect_type: crate::core::timeline::EffectType::FilmGrain {
                        intensity: crate::core::property::Animatable::new_constant(0.1),
                        grain_size: 2.0,
                        color_film: false,
                    },
                    enabled: true,
                },
                "Drop Shadow" => crate::core::timeline::Effect {
                    id: format!("ds_{}", len),
                    name: "Drop Shadow".to_string(),
                    effect_type: crate::core::timeline::EffectType::DropShadow {
                        color: crate::core::property::Animatable::new_constant([
                            0.0, 0.0, 0.0, 1.0,
                        ]),
                        opacity: crate::core::property::Animatable::new_constant(75.0),
                        direction: crate::core::property::Animatable::new_constant(120.0),
                        distance: crate::core::property::Animatable::new_constant(5.0),
                        softness: crate::core::property::Animatable::new_constant(5.0),
                    },
                    enabled: true,
                },
                "Twirl" => crate::core::timeline::Effect {
                    id: format!("twirl_{}", len),
                    name: "Twirl".to_string(),
                    effect_type: crate::core::timeline::EffectType::Twirl {
                        angle: crate::core::property::Animatable::new_constant(50.0),
                        radius: crate::core::property::Animatable::new_constant(100.0),
                    },
                    enabled: true,
                },
                "Bulge" => crate::core::timeline::Effect {
                    id: format!("bulge_{}", len),
                    name: "Bulge".to_string(),
                    effect_type: crate::core::timeline::EffectType::Bulge {
                        amount: crate::core::property::Animatable::new_constant(50.0),
                        radius: crate::core::property::Animatable::new_constant(100.0),
                    },
                    enabled: true,
                },
                "Mesh Warp" => crate::core::timeline::Effect {
                    id: format!("meshwarp_{}", len),
                    name: "Mesh Warp".to_string(),
                    effect_type: crate::core::timeline::EffectType::MeshWarp {
                        top_left: crate::core::property::Animatable::new_constant([0.0, 0.0]),
                        top_right: crate::core::property::Animatable::new_constant([1.0, 0.0]),
                        bottom_left: crate::core::property::Animatable::new_constant([0.0, 1.0]),
                        bottom_right: crate::core::property::Animatable::new_constant([1.0, 1.0]),
                    },
                    enabled: true,
                },
                "Corner Pin" => {
                    let (cw, ch) = (comp.width as f32, comp.height as f32);
                    crate::core::timeline::Effect {
                        id: format!("cornerpin_{}", len),
                        name: "Corner Pin".to_string(),
                        effect_type: crate::core::timeline::EffectType::CornerPin {
                            top_left: crate::core::property::Animatable::new_constant([0.0, 0.0]),
                            top_right: crate::core::property::Animatable::new_constant([cw, 0.0]),
                            bottom_right: crate::core::property::Animatable::new_constant([cw, ch]),
                            bottom_left: crate::core::property::Animatable::new_constant([0.0, ch]),
                        },
                        enabled: true,
                    }
                }
                "Chromatic Aberration" => crate::core::timeline::Effect {
                    id: format!("ca_{}", len),
                    name: "Chromatic Aberration".to_string(),
                    effect_type: crate::core::timeline::EffectType::ChromaticAberration {
                        shift_r: crate::core::property::Animatable::new_constant(5.0),
                        shift_b: crate::core::property::Animatable::new_constant(-5.0),
                        edge_falloff: crate::core::property::Animatable::new_constant(0.5),
                        iris_linked: true,
                    },
                    enabled: true,
                },
                "Bass & Treble" => crate::core::timeline::Effect {
                    id: format!("bass_treble_{}", len),
                    name: "Bass & Treble".to_string(),
                    effect_type: crate::core::timeline::EffectType::BassTreble {
                        bass_gain: crate::core::property::Animatable::new_constant(0.0),
                        treble_gain: crate::core::property::Animatable::new_constant(0.0),
                        crossover_freq: crate::core::property::Animatable::new_constant(300.0),
                    },
                    enabled: true,
                },
                "Flanger" => crate::core::timeline::Effect {
                    id: format!("flanger_{}", len),
                    name: "Flanger".to_string(),
                    effect_type: crate::core::timeline::EffectType::Flanger {
                        max_delay_ms: crate::core::property::Animatable::new_constant(5.0),
                        lfo_rate: crate::core::property::Animatable::new_constant(0.5),
                        feedback: crate::core::property::Animatable::new_constant(0.5),
                        wet_dry: crate::core::property::Animatable::new_constant(0.5),
                    },
                    enabled: true,
                },
                "Chorus" => crate::core::timeline::Effect {
                    id: format!("chorus_{}", len),
                    name: "Chorus".to_string(),
                    effect_type: crate::core::timeline::EffectType::Chorus {
                        delay_ms: crate::core::property::Animatable::new_constant(15.0),
                        depth_ms: crate::core::property::Animatable::new_constant(5.0),
                        rate_hz: crate::core::property::Animatable::new_constant(1.0),
                        voices: crate::core::property::Animatable::new_constant(3.0),
                        feedback: crate::core::property::Animatable::new_constant(0.3),
                    },
                    enabled: true,
                },
                "Parametric EQ" => crate::core::timeline::Effect {
                    id: format!("peq_{}", len),
                    name: "Parametric EQ".to_string(),
                    effect_type: crate::core::timeline::EffectType::ParametricEQ {
                        freq_hz: crate::core::property::Animatable::new_constant(1000.0),
                        gain_db: crate::core::property::Animatable::new_constant(0.0),
                        q_factor: crate::core::property::Animatable::new_constant(1.0),
                    },
                    enabled: true,
                },
                "Optical Flares" => {
                    let (cw, ch) = (comp.width as f32, comp.height as f32);
                    crate::core::timeline::Effect {
                        id: format!("optflare_{}", len),
                        name: "Optical Flares".to_string(),
                        effect_type: crate::core::timeline::EffectType::OpticalFlares {
                            position: crate::core::property::Animatable::new_constant([
                                cw * 0.5,
                                ch * 0.5,
                            ]),
                            brightness: crate::core::property::Animatable::new_constant(1.0),
                            scale: crate::core::property::Animatable::new_constant(1.0),
                        },
                        enabled: true,
                    }
                }
                "Motion Tile" => {
                    let (cw, ch) = (comp.width as f32, comp.height as f32);
                    crate::core::timeline::Effect {
                        id: format!("motiontile_{}", len),
                        name: "Motion Tile".to_string(),
                        effect_type: crate::core::timeline::EffectType::MotionTile {
                            tile_center: crate::core::property::Animatable::new_constant([
                                cw * 0.5,
                                ch * 0.5,
                            ]),
                            tile_width: crate::core::property::Animatable::new_constant(100.0),
                            tile_height: crate::core::property::Animatable::new_constant(100.0),
                            output_width: crate::core::property::Animatable::new_constant(100.0),
                            output_height: crate::core::property::Animatable::new_constant(100.0),
                            mirror_edges: true,
                            phase: crate::core::property::Animatable::new_constant(0.0),
                        },
                        enabled: true,
                    }
                }
                "CC Page Turn" => {
                    let (cw, ch) = (comp.width as f32, comp.height as f32);
                    crate::core::timeline::Effect {
                        id: format!("pageturn_{}", len),
                        name: "CC Page Turn".to_string(),
                        effect_type: crate::core::timeline::EffectType::PageTurn {
                            fold_position: crate::core::property::Animatable::new_constant([
                                cw, ch,
                            ]),
                            fold_radius: crate::core::property::Animatable::new_constant(120.0),
                            fold_direction_deg: crate::core::property::Animatable::new_constant(
                                -45.0,
                            ),
                            light_direction_deg: crate::core::property::Animatable::new_constant(
                                -45.0,
                            ),
                            back_opacity: crate::core::property::Animatable::new_constant(100.0),
                            back_color: crate::core::property::Animatable::new_constant([
                                0.92, 0.92, 0.94, 1.0,
                            ]),
                        },
                        enabled: true,
                    }
                }
                "Set Matte" => crate::core::timeline::Effect {
                    id: format!("setmatte_{}", len),
                    name: "Set Matte".to_string(),
                    effect_type: crate::core::timeline::EffectType::SetMatte {
                        source_layer_idx: 0,
                        source_channel: crate::core::set_matte::MatteSourceChannel::Alpha,
                        invert_matte: false,
                        composite_mode: crate::core::set_matte::MatteCompositeMode::Replace,
                    },
                    enabled: true,
                },
                "Echo" => crate::core::timeline::Effect {
                    id: format!("echo_{}", len),
                    name: "Echo".to_string(),
                    effect_type: crate::core::timeline::EffectType::Echo {
                        echo_time_seconds: crate::core::property::Animatable::new_constant(-0.033),
                        num_echoes: 3,
                        starting_intensity: crate::core::property::Animatable::new_constant(1.0),
                        decay: crate::core::property::Animatable::new_constant(0.5),
                        operator: crate::core::echo_effect::EchoOperator::Add,
                    },
                    enabled: true,
                },
                "Find Edges" => crate::core::timeline::Effect {
                    id: format!("findedges_{}", len),
                    name: "Find Edges".to_string(),
                    effect_type: crate::core::timeline::EffectType::FindEdges { invert: false },
                    enabled: true,
                },
                "Transform" => crate::core::timeline::Effect {
                    id: format!("transform_{}", len),
                    name: "Transform".to_string(),
                    effect_type: crate::core::timeline::EffectType::Transform {
                        anchor_point: crate::core::property::Animatable::new_constant([
                            layer.transform.anchor_point.evaluate(0)[0],
                            layer.transform.anchor_point.evaluate(0)[1],
                        ]),
                        position: crate::core::property::Animatable::new_constant([
                            layer.transform.position.evaluate(0)[0],
                            layer.transform.position.evaluate(0)[1],
                        ]),
                        scale_width: crate::core::property::Animatable::new_constant(100.0),
                        scale_height: crate::core::property::Animatable::new_constant(100.0),
                        uniform_scale: true,
                        skew_deg: crate::core::property::Animatable::new_constant(0.0),
                        skew_axis_deg: crate::core::property::Animatable::new_constant(0.0),
                        rotation_deg: crate::core::property::Animatable::new_constant(0.0),
                        opacity: crate::core::property::Animatable::new_constant(100.0),
                    },
                    enabled: true,
                },
                "Camera Lens Blur" => crate::core::timeline::Effect {
                    id: format!("cameralensblur_{}", len),
                    name: "Camera Lens Blur".to_string(),
                    effect_type: crate::core::timeline::EffectType::CameraLensBlur {
                        blur_radius: crate::core::property::Animatable::new_constant(15.0),
                        iris_blades: 6,
                        iris_rotation_deg: crate::core::property::Animatable::new_constant(0.0),
                        iris_roundness: crate::core::property::Animatable::new_constant(0.0),
                        highlight_gain: crate::core::property::Animatable::new_constant(1.5),
                        highlight_threshold: crate::core::property::Animatable::new_constant(0.8),
                    },
                    enabled: true,
                },
                "Linear Color Key" => crate::core::timeline::Effect {
                    id: format!("linearcolorkey_{}", len),
                    name: "Linear Color Key".to_string(),
                    effect_type: crate::core::timeline::EffectType::LinearColorKey {
                        key_color: crate::core::property::Animatable::new_constant([0.0, 1.0, 0.0]),
                        match_mode: crate::core::linear_color_key::ColorMatchMode::UsingRGB,
                        tolerance: crate::core::property::Animatable::new_constant(15.0),
                        softness: crate::core::property::Animatable::new_constant(10.0),
                    },
                    enabled: true,
                },
                "Channel Combiner" => crate::core::timeline::Effect {
                    id: format!("channelcombiner_{}", len),
                    name: "Channel Combiner".to_string(),
                    effect_type: crate::core::timeline::EffectType::ChannelCombiner {
                        from_channel: crate::core::channel_combiner::ChannelCombinerFrom::Luminance,
                        to_target: crate::core::channel_combiner::ChannelCombinerTo::Alpha,
                        invert: false,
                    },
                    enabled: true,
                },
                "Lightning" => crate::core::timeline::Effect {
                    id: format!("lightning_{}", len),
                    name: "Lightning".to_string(),
                    effect_type: crate::core::timeline::EffectType::LightningArc {
                        start_x: crate::core::property::Animatable::new_constant(0.2),
                        start_y: crate::core::property::Animatable::new_constant(0.2),
                        end_x: crate::core::property::Animatable::new_constant(0.8),
                        end_y: crate::core::property::Animatable::new_constant(0.8),
                        seed: crate::core::property::Animatable::new_constant(12345.0),
                        glow: crate::core::property::Animatable::new_constant(1.0),
                    },
                    enabled: true,
                },
                "Laser Beam" => crate::core::timeline::Effect {
                    id: format!("laser_{}", len),
                    name: "Laser Beam".to_string(),
                    effect_type: crate::core::timeline::EffectType::LaserBeam {
                        start_x: crate::core::property::Animatable::new_constant(0.1),
                        start_y: crate::core::property::Animatable::new_constant(0.5),
                        end_x: crate::core::property::Animatable::new_constant(0.9),
                        end_y: crate::core::property::Animatable::new_constant(0.5),
                        progress: crate::core::property::Animatable::new_constant(0.5),
                        length: crate::core::property::Animatable::new_constant(40.0),
                        starting_thickness: crate::core::property::Animatable::new_constant(12.0),
                        ending_thickness: crate::core::property::Animatable::new_constant(4.0),
                        core_color: crate::core::property::Animatable::new_constant([
                            1.0, 1.0, 1.0, 1.0,
                        ]),
                        glow_color: crate::core::property::Animatable::new_constant([
                            1.0, 0.2, 0.1, 0.8,
                        ]),
                    },
                    enabled: true,
                },
                    _ => return,
                };
                layer.effects.push(effect);
                added = true;
            }
        });
        if added {
            crate::core::frame_cache::bump_version();
            app.toasts.info(format!("Added '{}' effect", effect_name));
        }
    }
}
