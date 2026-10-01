use eframe::egui;

pub(super) fn insert_imported_sequence(
    app: &mut crate::KagariApp,
    asset: crate::core::video_import::VideoAsset,
    name: String,
) {
    let (comp_w, comp_h, comp_duration) = {
        let comp = app.history.current().active_composition();
        (comp.width as f32, comp.height as f32, comp.duration_frames)
    };
    let fit = ((comp_w / asset.width.max(1) as f32).min(comp_h / asset.height.max(1) as f32))
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
        layer.transform.position =
            crate::core::property::Animatable::new_constant([comp_w * 0.5, comp_h * 0.5]);
        layer.transform.scale = crate::core::property::Animatable::new_constant([fit, fit]);
        comp.layers.push(layer);
    });
    app.toasts.info(format!(
        "Imported image sequence: {} ({} frames)",
        name, asset.frame_count
    ));
}

pub(super) fn insert_imported_model3d(
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

pub(super) fn insert_imported_svg_masks(
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
            let fill_color = vector_path.fill_color.unwrap_or([0.0, 0.0, 0.0, 0.0]);
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
                vector_path.vertices.iter().map(|v| v.position).collect(),
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

pub(super) fn import_image_inline(app: &mut crate::KagariApp, path: std::path::PathBuf) {
    let src = path.to_string_lossy().to_string();
    let name = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "image".to_string());
    let dimensions = image::image_dimensions(&path).ok();
    let (comp_w, comp_h, comp_duration) = {
        let comp = app.history.current().active_composition();
        (comp.width as f32, comp.height as f32, comp.duration_frames)
    };
    let fit = dimensions
        .map(|(w, h)| {
            ((comp_w / w.max(1) as f32).min(comp_h / h.max(1) as f32)).min(1.0) * 100.0
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
            crate::core::property::Animatable::new_constant([comp_w * 0.5, comp_h * 0.5]);
        layer.transform.scale = crate::core::property::Animatable::new_constant([fit, fit]);
        comp.layers.push(layer);
    });
    app.toasts.info(format!("Imported image: {}", name));
}

pub fn draw_import_video_button(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    if ui.button("Import Video (FFmpeg)...").clicked() {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Video Files", &["mp4", "mov", "avi", "mkv", "webm", "av1"])
            .pick_file()
        {
            let name = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "video".to_string());
            crate::ui::drop_import::start_video_import(app, ui.ctx(), path, name);
        }
        ui.close_menu();
    }
}
