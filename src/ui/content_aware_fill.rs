use crate::KagariApp;
use eframe::egui;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

fn prepare_caf_source(
    comp: &crate::core::timeline::Composition,
    layer_index: usize,
    mask_index: usize,
) -> Option<crate::core::timeline::Composition> {
    let mut source = comp.clone();
    source.background_color = [0.0; 4];
    for layer in &mut source.layers {
        layer.track_matte = crate::core::timeline::TrackMatteMode::None;
    }
    let layer = source.layers.get_mut(layer_index)?;
    if mask_index >= layer.masks.len() {
        return None;
    }
    layer.masks.remove(mask_index);
    Some(source)
}

fn render_caf_source_frame(
    source: &crate::core::timeline::Composition,
    layer_index: usize,
    frame: u32,
) -> Vec<u8> {
    crate::core::software_renderer::render_frame_to_pixels_filtered(
        source,
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

fn caf_range(
    range_idx: usize,
    duration_frames: u32,
    work_area_in: Option<u32>,
    work_area_out: Option<u32>,
    layer_in: u32,
    layer_out: u32,
) -> Result<(u32, u32), String> {
    let last = duration_frames.saturating_sub(1);
    if duration_frames == 0 {
        return Err("The composition has no frames to fill".into());
    }
    let (mut start, mut end) = if range_idx == 0 {
        (work_area_in.unwrap_or(0), work_area_out.unwrap_or(last))
    } else {
        (0, last)
    };
    start = start.max(layer_in);
    end = end.min(layer_out).min(last);
    if start > end {
        return Err("The selected layer does not overlap the chosen fill range".into());
    }
    Ok((start, end))
}

fn caf_sequence_directory(asset_root: &Path) -> Result<PathBuf, String> {
    static NEXT_CAF_ASSET: AtomicU32 = AtomicU32::new(1);
    for _ in 0..128 {
        let id = NEXT_CAF_ASSET.fetch_add(1, Ordering::Relaxed);
        let directory = asset_root.join(format!("caf-{}-{id:08}", std::process::id()));
        match std::fs::create_dir(&directory) {
            Ok(()) => return Ok(directory),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(format!("Could not create fill sequence folder: {error}")),
        }
    }
    Err("Could not allocate a unique fill sequence folder".into())
}

#[derive(Clone)]
struct CafCompletion {
    composition_id: String,
    source_layer_id: String,
    mask_id: String,
    original_source_layer: Vec<u8>,
    start_frame: u32,
    end_frame: u32,
    method: crate::core::content_aware_engine::FillMethod,
    asset_dir: PathBuf,
    result: Result<(), String>,
}

struct CafJob {
    receiver: Mutex<std::sync::mpsc::Receiver<CafCompletion>>,
    cancelled: Arc<AtomicBool>,
    progress: Arc<AtomicU32>,
    total: u32,
}

fn bake_caf_sequence(
    source: &crate::core::timeline::Composition,
    layer_index: usize,
    mask: &crate::core::mask::Mask,
    start_frame: u32,
    end_frame: u32,
    expansion: f32,
    method: crate::core::content_aware_engine::FillMethod,
    asset_dir: &Path,
    cancelled: &AtomicBool,
    progress: &AtomicU32,
) -> Result<(), String> {
    let (width, height) = (source.width, source.height);
    if width == 0 || height == 0 {
        return Err("The composition dimensions are invalid".into());
    }
    for (sequence_index, frame) in (start_frame..=end_frame).enumerate() {
        if cancelled.load(Ordering::Relaxed) {
            return Err("Content-Aware Fill cancelled".into());
        }
        let polygon = mask.path.to_polygon(frame, 12);
        if !mask.path.is_closed || polygon.len() < 3 {
            return Err(format!(
                "Mask is not a closed polygon at frame {}",
                frame + 1
            ));
        }
        let mut pixels = render_caf_source_frame(source, layer_index, frame);
        let expected = (width as usize)
            .checked_mul(height as usize)
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or_else(|| "Composition dimensions exceed supported limits".to_owned())?;
        if pixels.len() != expected {
            return Err(format!("Could not render source at frame {}", frame + 1));
        }
        let generated = crate::core::content_aware_engine::generate_content_aware_fill_frame(
            &pixels, width, height, &polygon, expansion, method,
        );
        if generated.len() != expected {
            return Err(format!("Fill generation failed at frame {}", frame + 1));
        }
        pixels = generated;
        let mask_expansion = mask.expansion.evaluate(frame) + expansion;
        let fill_region =
            crate::core::software_renderer::offset_polygon_vertices(&polygon, mask_expansion);
        if fill_region.len() < 3 {
            return Err(format!(
                "Mask is too small to generate a patch at frame {}",
                frame + 1
            ));
        }
        for y in 0..height {
            for x in 0..width {
                if !crate::core::mask::point_in_polygon(x as f32, y as f32, &fill_region) {
                    let alpha = ((y * width + x) * 4 + 3) as usize;
                    pixels[alpha] = 0;
                }
            }
        }
        if cancelled.load(Ordering::Relaxed) {
            return Err("Content-Aware Fill cancelled".into());
        }
        let frame_path = asset_dir.join(format!("frame_{sequence_index:05}.webp"));
        image::save_buffer_with_format(
            &frame_path,
            &pixels,
            width,
            height,
            image::ColorType::Rgba8,
            image::ImageFormat::WebP,
        )
        .map_err(|error| format!("Could not write fill frame {}: {error}", frame + 1))?;
        progress.store(sequence_index as u32 + 1, Ordering::Relaxed);
    }
    Ok(())
}

fn start_caf_job(
    app: &KagariApp,
    ctx: &egui::Context,
    range_idx: usize,
    expansion: f32,
    method: crate::core::content_aware_engine::FillMethod,
) -> Result<(), String> {
    let job_id = egui::Id::new("content_aware_fill_job");
    if ctx
        .data(|data| data.get_temp::<Arc<CafJob>>(job_id))
        .is_some()
    {
        return Err("A Content-Aware Fill sequence is already running".into());
    }
    let layer_index = app
        .selection
        .selected_layer_idx
        .ok_or_else(|| "Select a layer with a mask first".to_owned())?;
    let project = app.history.current();
    let composition = project.active_composition().clone();
    let layer = composition
        .layers
        .get(layer_index)
        .ok_or_else(|| "Selected layer no longer exists".to_owned())?;
    let (mask_index, mask) = layer
        .masks
        .iter()
        .enumerate()
        .find(|(_, mask)| mask.enabled)
        .ok_or_else(|| "Selected layer has no enabled mask".to_owned())?;
    if !mask.path.is_closed || mask.path.to_polygon(app.playback.current_frame, 12).len() < 3 {
        return Err(
            "Content-Aware Fill requires a closed mask with at least three vertices".into(),
        );
    }
    let (start_frame, end_frame) = caf_range(
        range_idx,
        composition.duration_frames,
        app.playback.work_area_in,
        app.playback.work_area_out,
        layer.in_frame,
        layer.out_frame,
    )?;
    let source = prepare_caf_source(&composition, layer_index, mask_index)
        .ok_or_else(|| "Could not prepare the selected layer for fill generation".to_owned())?;
    let directory = caf_asset_directory(&app.project_path)?;
    let asset_dir = caf_sequence_directory(&directory)?;
    let composition_id = composition.id.clone();
    let source_layer_id = layer.id.clone();
    let mask_id = mask.id.clone();
    let original_source_layer = serde_json::to_vec(layer)
        .map_err(|error| format!("Could not snapshot source layer: {error}"))?;
    let mask = mask.clone();
    let cancelled = Arc::new(AtomicBool::new(false));
    let progress = Arc::new(AtomicU32::new(0));
    let total = end_frame - start_frame + 1;
    let (sender, receiver) = std::sync::mpsc::channel();
    let job = Arc::new(CafJob {
        receiver: Mutex::new(receiver),
        cancelled: cancelled.clone(),
        progress: progress.clone(),
        total,
    });
    ctx.data_mut(|data| data.insert_temp(job_id, job));
    let worker_ctx = ctx.clone();
    let worker_asset_dir = asset_dir.clone();
    let spawn_result = std::thread::Builder::new()
        .name("kagari-content-aware-fill".into())
        .spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                bake_caf_sequence(
                    &source,
                    layer_index,
                    &mask,
                    start_frame,
                    end_frame,
                    expansion,
                    method,
                    &worker_asset_dir,
                    &cancelled,
                    &progress,
                )
            }))
            .unwrap_or_else(|_| {
                Err("Content-Aware Fill worker encountered an internal error".into())
            });
            let result = if cancelled.load(Ordering::Relaxed) {
                Err("Content-Aware Fill cancelled".into())
            } else {
                result
            };
            if result.is_err() {
                let _ = std::fs::remove_dir_all(&worker_asset_dir);
            }
            let _ = sender.send(CafCompletion {
                composition_id,
                source_layer_id,
                mask_id,
                original_source_layer,
                start_frame,
                end_frame,
                method,
                asset_dir: worker_asset_dir,
                result,
            });
            worker_ctx.request_repaint();
        });
    if let Err(error) = spawn_result {
        ctx.data_mut(|data| data.remove::<Arc<CafJob>>(job_id));
        let _ = std::fs::remove_dir_all(&asset_dir);
        return Err(format!("Could not start fill generation: {error}"));
    }
    Ok(())
}

fn poll_caf_job(app: &mut KagariApp, ctx: &egui::Context) {
    let job_id = egui::Id::new("content_aware_fill_job");
    let Some(job) = ctx.data(|data| data.get_temp::<Arc<CafJob>>(job_id)) else {
        return;
    };
    let received = match job.receiver.lock() {
        Ok(receiver) => receiver.try_recv(),
        Err(_) => Err(std::sync::mpsc::TryRecvError::Disconnected),
    };
    let completion = match received {
        Ok(completion) => completion,
        Err(std::sync::mpsc::TryRecvError::Empty) => return,
        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
            ctx.data_mut(|data| data.remove::<Arc<CafJob>>(job_id));
            app.toasts
                .error("Content-Aware Fill worker stopped unexpectedly");
            return;
        }
    };
    ctx.data_mut(|data| data.remove::<Arc<CafJob>>(job_id));
    if let Err(error) = completion.result {
        app.toasts.error(error);
        return;
    }
    let mut project = app.history.current().clone();
    let comp = project.active_composition_mut();
    if comp.id != completion.composition_id {
        let _ = std::fs::remove_dir_all(&completion.asset_dir);
        app.toasts
            .error("Composition changed; generated fill sequence was discarded");
        return;
    }
    let Some(source_index) = comp
        .layers
        .iter()
        .position(|layer| layer.id == completion.source_layer_id)
    else {
        let _ = std::fs::remove_dir_all(&completion.asset_dir);
        app.toasts
            .error("Source layer changed; generated fill sequence was discarded");
        return;
    };
    let source_layer = &comp.layers[source_index];
    let source_layer_matches = serde_json::to_vec(source_layer).ok().as_deref()
        == Some(completion.original_source_layer.as_slice());
    if !source_layer_matches
        || !source_layer
            .masks
            .iter()
            .any(|mask| mask.id == completion.mask_id)
    {
        let _ = std::fs::remove_dir_all(&completion.asset_dir);
        app.toasts
            .error("Source layer changed during generation; fill sequence was discarded");
        return;
    }
    let end_frame = completion.end_frame;
    let start_frame = completion.start_frame;
    let duration = comp.duration_frames;
    let frame_count = end_frame - start_frame + 1;
    let mut fill = crate::core::timeline::Layer::new(
        format!("caf_fill_{}_{}", start_frame, std::process::id()),
        format!("Fill Layer [CAF] ({}–{})", start_frame + 1, end_frame + 1),
        crate::core::timeline::LayerType::Video {
            source: "Kagari Content-Aware Fill sequence".into(),
            frames_dir: completion.asset_dir.to_string_lossy().into_owned(),
            frame_count,
            audio_wav: None,
            speed: 1.0,
        },
        duration,
    );
    fill.in_frame = start_frame;
    fill.out_frame = end_frame;
    fill.time_remap = Some(crate::core::property::Animatable::Animated(vec![
        crate::core::keyframe::Keyframe::new(
            start_frame,
            0.0,
            crate::core::keyframe::InterpolationType::Linear,
        ),
        crate::core::keyframe::Keyframe::new(
            end_frame.max(start_frame + 1),
            frame_count.saturating_sub(1) as f32,
            crate::core::keyframe::InterpolationType::Linear,
        ),
    ]));
    fill.frame_blending = false;
    let insert_at = if comp
        .layers
        .get(source_index + 1)
        .is_some_and(|layer| layer.track_matte != crate::core::timeline::TrackMatteMode::None)
    {
        source_index + 2
    } else {
        source_index + 1
    }
    .min(comp.layers.len());
    comp.layers.insert(insert_at, fill);
    app.commit_project(project);
    app.toasts.info(format!(
        "Generated {} WebP fill frames ({:?})",
        frame_count, completion.method
    ));
}

pub fn draw_content_aware_fill(app: &mut KagariApp, ui: &mut egui::Ui) {
    poll_caf_job(app, ui.ctx());
    ui.heading("Content-Aware Fill");
    ui.separator();

    let method_id = egui::Id::new("ae_caf_fill_method");
    let mut method_idx = ui
        .ctx()
        .data_mut(|data| *data.get_temp_mut_or_insert_with(method_id, || 0));
    egui::ComboBox::from_id_salt("caf_method_combo")
        .selected_text(match method_idx {
            0 => "Object (Motion Objects Removal)",
            1 => "Surface (Flat Texture Fill)",
            _ => "Edge Blend (Smooth Gradient)",
        })
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut method_idx, 0, "Object (Motion Objects Removal)");
            ui.selectable_value(&mut method_idx, 1, "Surface (Flat Texture Fill)");
            ui.selectable_value(&mut method_idx, 2, "Edge Blend (Smooth Gradient)");
        });
    ui.ctx()
        .data_mut(|data| data.insert_temp(method_id, method_idx));

    let alpha_exp_id = egui::Id::new("ae_caf_alpha_expansion");
    let mut alpha_exp: f32 = ui
        .ctx()
        .data_mut(|data| *data.get_temp_mut_or_insert_with(alpha_exp_id, || 5.0));
    ui.horizontal(|ui| {
        ui.label("Alpha Expansion:");
        ui.add(egui::Slider::new(&mut alpha_exp, 0.0..=50.0).suffix(" px"));
    });
    ui.ctx()
        .data_mut(|data| data.insert_temp(alpha_exp_id, alpha_exp));

    let range_id = egui::Id::new("ae_caf_range");
    let mut range_idx = ui
        .ctx()
        .data_mut(|data| *data.get_temp_mut_or_insert_with(range_id, || 0));
    ui.horizontal(|ui| {
        ui.selectable_value(&mut range_idx, 0, "Work Area");
        ui.selectable_value(&mut range_idx, 1, "Entire Duration");
    });
    ui.ctx()
        .data_mut(|data| data.insert_temp(range_id, range_idx));
    ui.add_space(8.0);
    ui.separator();

    let job_id = egui::Id::new("content_aware_fill_job");
    let job = ui.ctx().data(|data| data.get_temp::<Arc<CafJob>>(job_id));
    if let Some(job) = job {
        let done = job.progress.load(Ordering::Relaxed).min(job.total);
        ui.label(format!("Generating WebP frames: {done} / {}", job.total));
        ui.add(egui::ProgressBar::new(done as f32 / job.total.max(1) as f32).show_percentage());
        if ui.button("Cancel").clicked() {
            job.cancelled.store(true, Ordering::Relaxed);
        }
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(120));
    } else if ui
        .button("Generate Fill Layer")
        .on_hover_text("Generate a WebP fill sequence for the selected layer and range")
        .clicked()
    {
        let method = match method_idx {
            1 => crate::core::content_aware_engine::FillMethod::Surface,
            2 => crate::core::content_aware_engine::FillMethod::EdgeBlend,
            _ => crate::core::content_aware_engine::FillMethod::Object,
        };
        if let Err(error) = start_caf_job(app, ui.ctx(), range_idx, alpha_exp, method) {
            app.toasts.error(error);
        }
    }

    if ui
        .button("🖼 Create Reference Frame")
        .on_hover_text("Export current frame as a reference PNG for manual painting/cleanup")
        .clicked()
    {
        let comp = app.history.current().active_composition();
        let (width, height) = (comp.width, comp.height);
        let pixels = crate::core::software_renderer::render_frame_to_pixels(
            comp,
            app.playback.current_frame,
            width,
            height,
            0.0,
            0,
        );
        let out_path =
            std::env::temp_dir().join(format!("ref_frame_{}.png", app.playback.current_frame));
        if image::save_buffer(&out_path, &pixels, width, height, image::ColorType::Rgba8).is_ok() {
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
    use image::GenericImageView;

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

        let source = prepare_caf_source(&comp, 0, 0).unwrap();
        let pixels = render_caf_source_frame(&source, 0, 0);
        let left = ((4 * 16 + 2) * 4) as usize;
        let right = ((4 * 16 + 12) * 4) as usize;
        assert_eq!(&pixels[left..left + 4], &[255, 0, 0, 255]);
        assert_eq!(&pixels[right..right + 4], &[255, 0, 0, 255]);
    }

    #[test]
    fn selected_fill_ranges_are_clamped_to_the_layer_and_composition() {
        assert_eq!(caf_range(0, 100, Some(10), Some(80), 20, 90), Ok((20, 80)));
        assert_eq!(caf_range(1, 100, None, None, 20, 90), Ok((20, 90)));
        assert!(caf_range(0, 100, Some(0), Some(10), 20, 90).is_err());
        assert_eq!(caf_range(1, 10, None, None, 0, 99), Ok((0, 9)));
    }

    #[test]
    fn range_bake_writes_decodable_webp_sequence_and_reports_progress() {
        let mut comp = Composition::new("comp".into(), "Comp".into(), 16, 16, 30, 2);
        let mut layer = Layer::new(
            "layer".into(),
            "Layer".into(),
            LayerType::Solid {
                color: [0.8, 0.2, 0.1, 1.0],
            },
            2,
        );
        layer.masks.push(Mask::new_rect(
            "mask".into(),
            "Mask".into(),
            4.0,
            4.0,
            12.0,
            12.0,
        ));
        comp.layers.push(layer);
        let source = prepare_caf_source(&comp, 0, 0).unwrap();
        let mask = &comp.layers[0].masks[0];
        let root = std::env::temp_dir().join(format!(
            "kagari-caf-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let output = caf_sequence_directory(&root).unwrap();
        let cancelled = AtomicBool::new(false);
        let progress = AtomicU32::new(0);
        bake_caf_sequence(
            &source,
            0,
            mask,
            0,
            1,
            2.0,
            crate::core::content_aware_engine::FillMethod::Object,
            &output,
            &cancelled,
            &progress,
        )
        .unwrap();

        for index in 0..2 {
            let path = output.join(format!("frame_{index:05}.webp"));
            let image = image::open(path).unwrap();
            assert_eq!(image.dimensions(), (16, 16));
        }
        assert_eq!(progress.load(Ordering::Relaxed), 2);
        std::fs::remove_dir_all(root).unwrap();
    }
}
