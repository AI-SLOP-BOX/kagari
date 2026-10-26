use super::*;

pub(super) fn enter_studio_new_project(app: &mut KagariApp) {
    app.replace_project_history(crate::core::timeline::Project::empty_for_new_project());
    app.mogrt_properties.clear();
    app.selection.selected_layer_idx = None;
    app.selection.selected_layers.clear();
    crate::core::frame_cache::bump_version();
    app.ui_tabs.left_tab_idx = 0;
    app.ui_tabs.right_panel = crate::app_state::RightPanel::Properties;
    app.show_home = false;
}

pub(crate) fn new_project_from_header(app: &mut KagariApp) {
    enter_studio_new_project(app);
}

pub(super) fn enter_studio_open_dialog(app: &mut KagariApp) {
    if let Some(path) = rfd::FileDialog::new()
        .add_filter("Kagari VFX Project", &["json"])
        .pick_file()
    {
        open_project_path(app, &path);
    }
}

pub(crate) fn open_project_dialog(app: &mut KagariApp) {
    enter_studio_open_dialog(app);
}

pub(super) fn open_project_path(app: &mut KagariApp, path: &std::path::Path) {
    if let Err(e) = crate::ui::project_io::open_project_from_path(app, path) {
        app.toasts.error(e);
    }
}

pub(super) fn new_composition(app: &mut KagariApp) {
    enter_studio_new_project(app);
    app.show_new_comp_dialog = true;
}

pub(crate) fn import_footage_dialog(app: &mut KagariApp, ctx: &egui::Context) {
    if let Some(path) = rfd::FileDialog::new()
        .add_filter(
            "Footage",
            &[
                "png", "jpg", "jpeg", "webp", "bmp", "exr", "mov", "mp4", "avi", "mkv", "webm",
                "av1", "wav", "mp3",
            ],
        )
        .pick_file()
    {
        open_selected_in_studio(app, ctx, &path);
    }
}

fn open_selected_in_studio(app: &mut KagariApp, ctx: &egui::Context, path: &std::path::Path) {
    let ext = path
        .extension()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if ext == "json" {
        open_project_path(app, path);
        return;
    }
    enter_studio_new_project(app);
    let path_str = path.to_string_lossy().to_string();
    let name = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "media".to_string());
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "webp" | "bmp" => {
            app.modify_project(|p| {
                let comp = p.active_composition_mut();
                let layer = crate::core::timeline::Layer::new(
                    format!("img_{}", name),
                    name.clone(),
                    crate::core::timeline::LayerType::Image {
                        path: path_str.clone(),
                    },
                    comp.duration_frames,
                );
                comp.layers.push(layer);
            });
            app.toasts.info(format!("Imported image: {}", name));
        }
        "wav" | "mp3" => {
            app.modify_project(|p| {
                let comp = p.active_composition_mut();
                let layer = crate::core::timeline::Layer::new(
                    format!("aud_{}", name),
                    name.to_string(),
                    crate::core::timeline::LayerType::Audio {
                        path: path_str.clone(),
                        volume: crate::core::property::Animatable::new_constant(1.0),
                    },
                    comp.duration_frames,
                );
                comp.layers.push(layer);
            });
            app.toasts.info(format!("Imported audio: {}", name));
        }
        "mov" | "mp4" | "avi" | "mkv" | "webm" | "av1" => {
            crate::ui::drop_import::start_video_import(app, ctx, path.to_path_buf(), name);
        }
        _ => {
            app.toasts
                .info("Entered studio — use File > Import for unsupported media");
        }
    }
}
