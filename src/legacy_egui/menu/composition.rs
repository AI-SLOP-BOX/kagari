//! Composition menu items (New, Duplicate, Settings, Render Queue, Captions, Save Frame).

use eframe::egui;

pub fn draw_composition_menu(app: &mut crate::KagariApp, ctx: &egui::Context, ui: &mut egui::Ui) {
    if ui
        .add(egui::Button::new("New Composition...").shortcut_text(
            crate::ui::shortcuts::format_shortcut("N", true, false, false),
        ))
        .clicked()
    {
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
    if ui
        .add(egui::Button::new("Composition Settings...").shortcut_text(
            crate::ui::shortcuts::format_shortcut("E", true, true, false),
        ))
        .clicked()
    {
        app.comp_settings_draft = Some(app.history.current().clone());
        app.show_comp_settings = true;
        ui.close_menu();
    }
    if ui
        .add(egui::Button::new("Add to Render Queue").shortcut_text(
            crate::ui::shortcuts::format_shortcut("M", true, false, false),
        ))
        .clicked()
    {
        app.export.show_export_dialog = true;
        ui.close_menu();
    }
    ui.separator();
    draw_export_captions(app, ui);
    draw_save_frame(app, ui, ctx);
}

fn draw_export_captions(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    if ui
        .button("Export Captions (.srt)")
        .on_hover_text("Write all 'Caption …' text layers as an SRT file")
        .clicked()
    {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Subtitles", &["srt"])
            .set_file_name(format!(
                "{}_captions.srt",
                app.history
                    .current()
                    .active_composition()
                    .name
                    .replace(' ', "_")
            ))
            .save_file()
        {
            let proj = app.history.current();
            let comp = proj.active_composition();
            let srt = crate::core::subtitles::layers_to_srt(&comp.layers, comp.fps);
            if srt.is_empty() {
                app.toasts.error("No caption layers found");
            } else {
                match std::fs::write(&path, &srt) {
                    Ok(_) => app
                        .toasts
                        .info(format!("Captions exported: {}", path.display())),
                    Err(e) => app.toasts.error(format!("Write failed: {}", e)),
                }
            }
        }
        ui.close_menu();
    }
}

fn draw_save_frame(app: &mut crate::KagariApp, ui: &mut egui::Ui, _ctx: &egui::Context) {
    if ui
        .button("Save Frame As… (PNG)")
        .on_hover_text("Render the current frame at full resolution and save as PNG")
        .clicked()
    {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("PNG Image", &["png"])
            .set_file_name(format!(
                "{}_frame_{}.png",
                app.history.current().active_composition().name,
                app.playback.current_frame
            ))
            .save_file()
        {
            let comp = app.history.current().active_composition().clone();
            let frame = app.playback.current_frame;
            let max_dim = crate::core::software_renderer::MAX_RENDER_DIMENSION;
            let sw = (comp.width.saturating_mul(2)).min(max_dim);
            let sh = (comp.height.saturating_mul(2)).min(max_dim);
            let px = crate::core::software_renderer::render_frame_to_pixels(
                &comp, frame, sw, sh, 0.0, 0,
            );
            let (pixels, w, h) = if sw > comp.width || sh > comp.height {
                (
                    crate::core::supersample::downsample2x(&px, comp.width, comp.height),
                    comp.width,
                    comp.height,
                )
            } else {
                (px, sw, sh)
            };
            match image::save_buffer(&path, &pixels, w, h, image::ColorType::Rgba8) {
                Ok(_) => app
                    .toasts
                    .info(format!("Frame {} saved to {}", frame, path.display())),
                Err(e) => app.toasts.error(format!("Save failed: {}", e)),
            }
        }
        ui.close_menu();
    }
}
