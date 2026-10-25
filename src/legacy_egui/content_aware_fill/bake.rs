//! Content-aware fill frame sequence baking and asset folder resolution.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

pub fn prepare_caf_source(
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

pub fn render_caf_source_frame(
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

pub fn caf_asset_directory(project_path: &str) -> Result<PathBuf, String> {
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

pub fn caf_range(
    range_idx: usize,
    duration_frames: u32,
    work_area_in: Option<u32>,
    work_area_out: Option<u32>,
    layer_in: u32,
    layer_out: u32,
) -> Result<(u32, u32), String> {
    let (mut start, mut end) = match range_idx {
        0 => (
            work_area_in.unwrap_or(0).max(layer_in),
            work_area_out
                .unwrap_or(duration_frames.saturating_sub(1))
                .min(layer_out),
        ),
        _ => (layer_in, layer_out),
    };
    start = start.min(duration_frames.saturating_sub(1));
    end = end.min(duration_frames.saturating_sub(1));
    if start > end {
        return Err("The selected range is empty or falls outside the layer duration".into());
    }
    Ok((start, end))
}

pub fn caf_sequence_directory(asset_root: &Path) -> Result<PathBuf, String> {
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
pub struct CafCompletion {
    pub composition_id: String,
    pub source_layer_id: String,
    pub mask_id: String,
    pub original_source_layer: Vec<u8>,
    pub start_frame: u32,
    pub end_frame: u32,
    pub method: crate::core::content_aware_engine::FillMethod,
    pub asset_dir: PathBuf,
    pub result: Result<(), String>,
}

pub struct CafJob {
    pub receiver: Mutex<std::sync::mpsc::Receiver<CafCompletion>>,
    pub cancelled: Arc<AtomicBool>,
    pub progress: Arc<AtomicU32>,
    pub total: u32,
}

pub struct CafBakeRequest<'a> {
    pub source: &'a crate::core::timeline::Composition,
    pub layer_index: usize,
    pub mask: &'a crate::core::mask::Mask,
    pub start_frame: u32,
    pub end_frame: u32,
    pub expansion: f32,
    pub method: crate::core::content_aware_engine::FillMethod,
    pub asset_dir: &'a Path,
    pub cancelled: &'a AtomicBool,
    pub progress: &'a AtomicU32,
}

pub fn bake_caf_sequence(request: CafBakeRequest<'_>) -> Result<(), String> {
    let CafBakeRequest {
        source,
        layer_index,
        mask,
        start_frame,
        end_frame,
        expansion,
        method,
        asset_dir,
        cancelled,
        progress,
    } = request;
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
