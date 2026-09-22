use crate::KagariApp;
use eframe::egui;
use std::path::{Path, PathBuf};

fn render_caf_source_frame(
    comp: &crate::core::timeline::Composition,
    layer_index: usize,
    mask_index: usize,
    frame: u32,
) -> Vec<u8> {
    let mut source = comp.clone();
    source.background_color = [0.0; 4];
    for layer in &mut source.layers {
        layer.track_matte = crate::core::timeline::TrackMatteMode::None;
    }
    let Some(layer) = source.layers.get_mut(layer_index) else {
        return Vec::new();
    };
    if mask_index >= layer.masks.len() {
        return Vec::new();
    }
    layer.masks.remove(mask_index);
    crate::core::software_renderer::render_frame_to_pixels_filtered(
        &source,
        frame,
        source.width,
        source.height,
        0.0,
        0,
        Some(layer_index),
    )
}

fn caf_asset_directory(project_path: &str) -> Result<PathBuf, String> {
    let project = Path::new(project_path);
    let parent = project
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let directory = parent.join("Assets").join("KagariGenerated");
    std::fs::create_dir_all(&directory)
        .map_err(|error| format!("Could not create generated asset folder: {error}"))?;
    Ok(directory)
}

pub fn draw_content_aware_fill(app: &mut KagariApp, ui: &mut egui::Ui) {
    ui.heading("Content-Aware Fill");
    ui.separator();

    ui.label("Fill Method:");
    let method_id = egui::Id::new("ae_caf_fill_method");
    let mut method_idx = ui
        .ctx()
        .data_mut(|d| *d.get_temp_mut_or_insert_with(method_id, || 0));
    egui::ComboBox::from_id_salt("caf_method_combo")
        .selected_text(match method_idx {
            0 => "Object (Motion Objects Removal)",
            1 => "Surface (Flat Texture Fill)",
            _ => "Edge Blend (Smooth Gradient)",
        })
        .show_ui(ui, |ui| {
            if ui
                .selectable_value(&mut method_idx, 0, "Object (Motion Objects Removal)")
                .clicked()
            {
                ui.ctx().data_mut(|d| d.insert_temp(method_id, method_idx));
            }
            if ui
                .selectable_value(&mut method_idx, 1, "Surface (Flat Texture Fill)")
                .clicked()
            {
                ui.ctx().data_mut(|d| d.insert_temp(method_id, method_idx));
            }
            if ui
                .selectable_value(&mut method_idx, 2, "Edge Blend (Smooth Gradient)")
                .clicked()
            {
                ui.ctx().data_mut(|d| d.insert_temp(method_id, method_idx));
            }
        });

    ui.add_space(6.0);
    let alpha_exp_id = egui::Id::new("ae_caf_alpha_expansion");
    let mut alpha_exp: f32 = ui
        .ctx()
        .data_mut(|d| *d.get_temp_mut_or_insert_with(alpha_exp_id, || 5.0));
    ui.horizontal(|ui| {
        ui.label("Alpha Expansion:");
        if ui
            .add(egui::Slider::new(&mut alpha_exp, 0.0..=50.0).suffix(" px"))
            .changed()
        {
            ui.ctx()
                .data_mut(|d| d.insert_temp(alpha_exp_id, alpha_exp));
        }
    });

    ui.add_space(6.0);
    ui.label("Range:");
    let range_id = egui::Id::new("ae_caf_range");
    let mut range_idx = ui
        .ctx()
        .data_mut(|d| *d.get_temp_mut_or_insert_with(range_id, || 0));
    ui.horizontal(|ui| {
        if ui
            .selectable_value(&mut range_idx, 0, "Work Area")
            .clicked()
        {
            ui.ctx().data_mut(|d| d.insert_temp(range_id, range_idx));
        }
        if ui
            .selectable_value(&mut range_idx, 1, "Entire Duration")
            .clicked()
        {
            ui.ctx().data_mut(|d| d.insert_temp(range_id, range_idx));
        }
    });

    ui.add_space(10.0);
    ui.separator();

    if ui
        .button("Generate Fill Layer")
        .on_hover_text(
            "Render the current frame and synthesize a fill for the selected layer's first mask",
        )
        .clicked()
    {
        let Some(layer_idx) = app.selection.selected_layer_idx else {
            app.toasts.error("Select a layer with a mask first");
            return;
        };
        let project = app.history.current();
        let comp = project.active_composition();
        let Some(layer) = comp.layers.get(layer_idx) else {
            return;
        };
        let Some(mask_idx) = layer.masks.iter().position(|mask| mask.enabled) else {
            app.toasts
                .error("Selected layer has no mask — draw a mask around the object to remove");
            return;
        };
        let mask = &layer.masks[mask_idx];

        let (w, h) = (comp.width, comp.height);
        let frame_idx = app.playback.current_frame;
        let mut pixels = render_caf_source_frame(comp, layer_idx, mask_idx, frame_idx);
        if pixels.len() != (w as usize).saturating_mul(h as usize).saturating_mul(4) {
            app.toasts.error("Could not render the selected layer for Content-Aware Fill");
            return;
        }
        let polygon = mask.path.to_polygon(frame_idx, 12);
        let method = match method_idx {
            1 => crate::core::content_aware_engine::FillMethod::Surface,
            2 => crate::core::content_aware_engine::FillMethod::EdgeBlend,
            _ => crate::core::content_aware_engine::FillMethod::Object,
        };
        let filled = crate::core::content_aware_engine::generate_content_aware_fill_frame(
            &pixels, w, h, &polygon, alpha_exp, method,
        );
        pixels = filled;

        let expansion = mask.expansion.evaluate(frame_idx) + alpha_exp;
        let fill_region =
            crate::core::software_renderer::offset_polygon_vertices(&polygon, expansion);
        for y in 0..h {
            for x in 0..w {
                if !crate::core::mask::point_in_polygon(x as f32, y as f32, &fill_region) {
                    let alpha = ((y * w + x) * 4 + 3) as usize;
                    pixels[alpha] = 0;
                }
            }
        }

        let directory = match caf_asset_directory(&app.project_path) {
            Ok(directory) => directory,
            Err(error) => {
                app.toasts.error(error);
                return;
            }
        };
        static NEXT_CAF_ASSET: std::sync::atomic::AtomicU64 =
            std::sync::atomic::AtomicU64::new(1);
        let asset_id = NEXT_CAF_ASSET.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let out_path = directory.join(format!(
            "caf-{}-{}-{}.webp",
            std::process::id(),
            frame_idx,
            asset_id
        ));
        let temp_path = out_path.with_extension("webp.tmp");
        let save_result = image::save_buffer_with_format(
            &temp_path,
            &pixels,
            w,
            h,
            image::ColorType::Rgba8,
            image::ImageFormat::WebP,
        )
        .map_err(|error| error.to_string())
        .and_then(|()| std::fs::rename(&temp_path, &out_path).map_err(|error| error.to_string()));
        match save_result {
            Ok(_) => {
                let mut temp_proj = app.history.current().clone();
                let comp_mut = temp_proj.active_composition_mut();
                let new_layer = crate::core::timeline::Layer::new(
                    format!("caf_fill_{}_{}", frame_idx, asset_id),
                    format!("Fill Layer [CAF] (Frame {})", frame_idx + 1),
                    crate::core::timeline::LayerType::Image {
                        path: out_path.to_string_lossy().to_string(),
                    },
                    comp_mut.duration_frames,
                );
                let mut new_layer = new_layer;
                new_layer.in_frame = frame_idx;
                new_layer.out_frame = frame_idx;
                comp_mut.layers.insert(layer_idx + 1, new_layer);
                app.commit_project(temp_proj);
                app.toasts.info(format!(
                    "Generated a frame-specific fill patch for frame {}",
                    frame_idx + 1
                ));
            }
            Err(error) => app.toasts.error(format!("Failed to write fill asset: {error}")),
        }
    }

    if ui
        .button("🖼 Create Reference Frame")
        .on_hover_text("Export current frame as a reference PNG for manual painting/cleanup")
        .clicked()
    {
        let comp = app.history.current().active_composition();
        let (w, h) = (comp.width, comp.height);
        let pixels = crate::core::software_renderer::render_frame_to_pixels(
            comp,
            app.playback.current_frame,
            w,
            h,
            0.0,
            0,
        );
        let out_path =
            std::env::temp_dir().join(format!("ref_frame_{}.png", app.playback.current_frame));
        if image::save_buffer(&out_path, &pixels, w, h, image::ColorType::Rgba8).is_ok() {
            crate::ui::project_io::reveal_in_file_manager(&out_path);
            app.toasts
                .info(format!("Exported Reference Frame: {}", out_path.display()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::mask::Mask;
    use crate::core::timeline::{Composition, Layer, LayerType};

    #[test]
    fn fill_source_isolated_to_selected_layer_and_ignores_the_selection_mask() {
        let mut comp = Composition::new("comp".into(), "Comp".into(), 16, 16, 30, 30);
        let mut selected = Layer::new(
            "selected".into(),
            "Selected".into(),
            LayerType::Solid {
                color: [1.0, 0.0, 0.0, 1.0],
            },
            30,
        );
        selected.transform.position = crate::core::property::Animatable::new_constant([8.0, 8.0]);
        selected.masks.push(Mask::new_rect(
            "selection".into(),
            "Selection".into(),
            0.0,
            0.0,
            8.0,
            16.0,
        ));
        comp.layers.push(selected);
        comp.layers.push(Layer::new(
            "unrelated".into(),
            "Unrelated".into(),
            LayerType::Solid {
                color: [0.0, 0.0, 1.0, 1.0],
            },
            30,
        ));

        let pixels = render_caf_source_frame(&comp, 0, 0, 0);
        let left = ((4 * 16 + 2) * 4) as usize;
        let right = ((4 * 16 + 12) * 4) as usize;
        assert_eq!(&pixels[left..left + 4], &[255, 0, 0, 255]);
        assert_eq!(&pixels[right..right + 4], &[255, 0, 0, 255]);
    }
}
