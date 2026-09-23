use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

fn ensure_composition(app: &mut KagariApp) {
    if app.history.current().compositions.is_empty() {
        app.history =
            crate::core::history::ProjectHistory::new(crate::core::timeline::Project::default());
    }
}

fn open_project_dialog(app: &mut KagariApp) {
    if let Some(path) = rfd::FileDialog::new()
        .add_filter("Kagari VFX Project", &["json"])
        .pick_file()
    {
        if let Err(e) = crate::ui::project_io::open_project_from_path(app, &path) {
            app.toasts.error(e);
        } else {
            app.show_welcome = false;
        }
    }
}

fn import_media_dialog(app: &mut KagariApp) {
    let Some(path) = rfd::FileDialog::new()
        .add_filter(
            "Media",
            &[
                "png", "jpg", "jpeg", "bmp", "tga", "webp", "mp4", "mov", "mkv", "avi", "webm",
                "av1", "wav",
            ],
        )
        .pick_file()
    else {
        return;
    };
    ensure_composition(app);
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let name = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "media".to_string());
    let src = path.to_string_lossy().to_string();

    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "bmp" | "tga" | "webp" => {
            app.modify_project(|p| {
                let comp = p.active_composition_mut();
                let layer = crate::core::timeline::Layer::new(
                    format!("img_{}", name),
                    name.clone(),
                    crate::core::timeline::LayerType::Image { path: src.clone() },
                    comp.duration_frames,
                );
                comp.layers.push(layer);
            });
            app.toasts.info(format!("Imported image: {}", name));
            app.show_welcome = false;
        }
        "wav" => {
            app.modify_project(|p| {
                let comp = p.active_composition_mut();
                let layer = crate::core::timeline::Layer::new(
                    format!("aud_{}", name),
                    format!("🔊 {}", name),
                    crate::core::timeline::LayerType::Audio {
                        path: src.clone(),
                        volume: crate::core::property::Animatable::new_constant(1.0),
                    },
                    comp.duration_frames,
                );
                comp.layers.push(layer);
            });
            app.toasts.info(format!("Imported audio: {}", name));
            app.show_welcome = false;
        }
        "mp4" | "mov" | "mkv" | "avi" | "webm" | "av1" => {
            let fps = app.history.current().active_composition().fps as f32;
            let dest = std::env::temp_dir().join("kagari_media").join(&name);
            match crate::core::video_import::import_video(&src, &dest, fps) {
                Ok(asset) => {
                    let frame_count = asset.frame_count;
                    app.modify_project(|p| {
                        let comp = p.active_composition_mut();
                        let layer = crate::core::timeline::Layer::new(
                            format!("video_{}", name),
                            name.clone(),
                            crate::core::timeline::LayerType::Video {
                                source: src.clone(),
                                frames_dir: asset.frames_dir.clone(),
                                frame_count: asset.frame_count,
                                audio_wav: asset.audio_wav.clone(),
                                speed: 1.0,
                            },
                            comp.duration_frames,
                        );
                        comp.layers.push(layer);
                    });
                    app.toasts
                        .info(format!("Imported video: {} ({} frames)", name, frame_count));
                    app.show_welcome = false;
                }
                Err(err) => {
                    app.toasts.error(format!("Video import failed: {}", err));
                }
            }
        }
        _ => {
            app.toasts.error(format!("Unsupported file type: .{}", ext));
        }
    }
}

pub fn draw(app: &mut KagariApp, ctx: &egui::Context) {
    let project_empty = app.history.current().compositions.is_empty();
    if !project_empty && !app.show_welcome {
        return;
    }

    let mut open = app.show_welcome || project_empty;
    let mut action: Option<&str> = None;
    let mut recent_to_open: Option<std::path::PathBuf> = None;

    crate::ui::modal::window("Welcome to Kagari VFX")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(520.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(16.0);
                ui.heading(
                    egui::RichText::new("Kagari VFX")
                        .size(28.0)
                        .strong()
                        .color(colors::TEXT_PRIMARY),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("GPU-accelerated motion graphics in Rust")
                        .size(13.0)
                        .color(colors::TEXT_SECONDARY),
                );
            });
            ui.add_space(20.0);

            ui.separator();
            ui.add_space(8.0);

            let starred = app.home_starred_projects.clone();
            if !starred.is_empty() {
                ui.heading(
                    egui::RichText::new("Starred Projects")
                        .size(13.0)
                        .strong()
                        .color(colors::TEXT_PRIMARY),
                );
                ui.add_space(8.0);
                egui::ScrollArea::vertical()
                    .max_height(120.0)
                    .show(ui, |ui| {
                        for path_str in &starred {
                            let path = std::path::PathBuf::from(path_str);
                            let display = path
                                .file_name()
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_else(|| path_str.clone());
                            ui.horizontal(|ui| {
                                ui.add_space(8.0);
                                let resp = ui
                                    .selectable_label(false, format!("★ {}", display))
                                    .on_hover_text(path_str.clone());
                                if resp.clicked() {
                                    recent_to_open = Some(path.clone());
                                }
                                if resp.hovered() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                }
                            });
                            ui.add_space(2.0);
                        }
                    });
                ui.add_space(16.0);
            }

            let recent = crate::ui::project_io::recent_projects();
            if !recent.is_empty() {
                ui.heading(
                    egui::RichText::new("Recent Projects")
                        .size(13.0)
                        .strong()
                        .color(colors::TEXT_PRIMARY),
                );
                ui.add_space(8.0);

                egui::ScrollArea::vertical()
                    .max_height(160.0)
                    .show(ui, |ui| {
                        for path_str in &recent {
                            let path = std::path::PathBuf::from(path_str);
                            let display = path
                                .file_name()
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_else(|| path_str.clone());
                            ui.horizontal(|ui| {
                                ui.add_space(8.0);
                                let resp = ui
                                    .selectable_label(false, &display)
                                    .on_hover_text(path_str.clone());
                                if resp.clicked() {
                                    recent_to_open = Some(path.clone());
                                }
                                if resp.hovered() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                }
                            });
                            ui.add_space(2.0);
                        }
                    });
                ui.add_space(16.0);
            }

            ui.heading(
                egui::RichText::new("Quick Start")
                    .size(13.0)
                    .strong()
                    .color(colors::TEXT_PRIMARY),
            );
            ui.add_space(8.0);

            let actions = [
                ("New Composition", "Create a new composition"),
                ("Open Project", "Open existing project"),
                ("Import Media", "Import footage, images, audio"),
                ("Load Demo Scene", "Load demo scene to explore"),
            ];

            for (label, desc) in actions {
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    let btn = ui.add(
                        egui::Button::new(
                            egui::RichText::new(label)
                                .size(13.0)
                                .color(colors::TEXT_PRIMARY),
                        )
                        .min_size(egui::vec2(180.0, 36.0)),
                    );
                    if btn.clicked() {
                        action = Some(label);
                    }
                    ui.add_space(12.0);
                    ui.label(
                        egui::RichText::new(desc)
                            .size(11.0)
                            .color(colors::TEXT_SECONDARY),
                    );
                });
                ui.add_space(4.0);
            }

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Drop files anywhere to import")
                        .small()
                        .color(colors::TEXT_MUTED),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(egui::Button::new(
                            egui::RichText::new("Don't show again")
                                .small()
                                .color(colors::TEXT_MUTED),
                        ))
                        .clicked()
                    {
                        app.show_welcome = false;
                        crate::ui::project_io::set_welcome_on_startup(false);
                    }
                });
            });
        });

    if let Some(path) = recent_to_open {
        if let Err(e) = crate::ui::project_io::open_project_from_path(app, &path) {
            app.toasts.error(e);
        } else {
            app.show_welcome = false;
        }
    }

    if let Some(label) = action {
        match label {
            "New Composition" => {
                app.show_welcome = false;
                app.show_new_comp_dialog = true;
            }
            "Open Project" => open_project_dialog(app),
            "Import Media" => import_media_dialog(app),
            "Load Demo Scene" => {
                crate::ui::demo_scene::build(app);
                app.show_welcome = false;
            }
            _ => {}
        }
    }

    let still_empty = app.history.current().compositions.is_empty();
    if still_empty {
        app.show_welcome = open;
    } else {
        app.show_welcome = open && app.show_welcome;
    }
}
