use super::frame::write_png;
use super::*;

/// Bundled render parameters shared by all export backends.
struct RenderSpec<'a> {
    output: &'a str,
    from: u32,
    to: u32,
    w: u32,
    h: u32,
    exposure: f32,
    lut: u32,
    bindings: &'a std::collections::HashMap<String, f64>,
}

pub(super) struct RenderArgs {
    pub(super) project_path: String,
    pub(super) comp_ref: Option<String>,
    pub(super) output: String,
    pub(super) format: OutputFormat,
    pub(super) from: u32,
    pub(super) to: Option<u32>,
    pub(super) width: Option<u32>,
    pub(super) height: Option<u32>,
    pub(super) exposure: f32,
    pub(super) lut: u32,
    pub(super) bindings: Vec<(String, f64)>,
}

pub(super) fn cmd_render(args: RenderArgs) -> Result<(), Box<dyn std::error::Error>> {
    let production = kagari_vfx::core::project_migration::read_project_json_file(
        std::path::Path::new(&args.project_path),
    )
    .ok()
    .and_then(|json| {
        kagari_vfx::core::production_document::ProductionDocument::from_json(&json).ok()
    });
    let project = if let Some(document) = production.as_ref() {
        document.project.clone()
    } else {
        load_project(&args.project_path)?
    };
    let comp = find_comp(&project, args.comp_ref.as_deref())?;
    let comp_index = project
        .compositions
        .iter()
        .position(|candidate| candidate.id == comp.id)
        .unwrap_or(0);

    let binding_values = args
        .bindings
        .iter()
        .cloned()
        .collect::<std::collections::HashMap<_, _>>();
    let to = args.to.unwrap_or(comp.duration_frames.saturating_sub(1));
    if comp.duration_frames == 0 {
        return Err(format!("Composition '{}' has no frames", comp.name).into());
    }
    if args.from > to {
        return Err(format!("Invalid render range: from {} is after to {to}", args.from).into());
    }
    if to >= comp.duration_frames {
        return Err(format!(
            "Render frame {to} is outside composition '{}' (0..{})",
            comp.name,
            comp.duration_frames - 1
        )
        .into());
    }
    let width = args.width.unwrap_or(comp.width);
    let height = args.height.unwrap_or(comp.height);
    let max_dimension = kagari_vfx::core::software_renderer::MAX_RENDER_DIMENSION;
    if width == 0 || height == 0 || width > max_dimension || height > max_dimension {
        return Err(format!(
            "Render dimensions must be between 1 and {max_dimension} pixels per axis"
        )
        .into());
    }
    let render_bytes = u64::from(width) * u64::from(height) * 4;
    if render_bytes > kagari_vfx::core::software_renderer::MAX_RENDER_BYTES as u64 {
        return Err(format!(
            "Render buffer requires {render_bytes} bytes; the limit is {} bytes",
            kagari_vfx::core::software_renderer::MAX_RENDER_BYTES
        )
        .into());
    }
    if !args.exposure.is_finite() || args.lut > 2 {
        return Err("Exposure must be finite and LUT mode must be 0, 1, or 2".into());
    }
    let spec = RenderSpec {
        output: &args.output,
        from: args.from,
        to,
        w: width,
        h: height,
        exposure: args.exposure,
        lut: args.lut,
        bindings: &binding_values,
    };
    let format = args.format;

    eprintln!("Rendering composition: {}", comp.name);
    eprintln!("  Size: {}x{}", spec.w, spec.h);
    eprintln!(
        "  Frames: {}..={} ({} frames)",
        spec.from,
        spec.to,
        spec.to.saturating_sub(spec.from) + 1
    );
    eprintln!("  Format: {:?}", format);

    match format {
        OutputFormat::Mp4 => render_to_mp4(comp, &spec, production.as_ref(), comp_index)?,
        OutputFormat::Gif => render_to_gif(comp, &spec, production.as_ref(), comp_index)?,
        OutputFormat::Png => {
            if let Some(document) = production.as_ref() {
                render_to_png_sequence_with_bindings(document, comp_index, &spec)?;
            } else {
                render_to_png_sequence(comp, &spec)?;
            }
        }
    }

    Ok(())
}

fn render_to_png_sequence_with_bindings(
    document: &kagari_vfx::core::production_document::ProductionDocument,
    composition_index: usize,
    spec: &RenderSpec,
) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(spec.output)?;
    let total = spec.to.saturating_sub(spec.from) + 1;
    for frame in spec.from..=spec.to {
        let comp = document
            .composition_for_frame_with_sources(composition_index, frame, spec.bindings)
            .ok_or("composition index out of range")?;
        let pixels = render_frame_to_pixels(&comp, frame, spec.w, spec.h, spec.exposure, spec.lut);
        let path = format!("{}/frame_{:05}.png", spec.output, frame);
        write_png(&path, &pixels, spec.w, spec.h)?;
        let progress = frame.saturating_sub(spec.from) + 1;
        if progress % 10 == 0 || progress == total {
            eprint!("\r  Rendered {}/{} frames", progress, total);
        }
    }
    eprintln!("\n  Done! Output: {}/", spec.output);
    Ok(())
}

fn render_to_png_sequence(
    comp: &Composition,
    spec: &RenderSpec,
) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(spec.output)?;

    let total = spec.to.saturating_sub(spec.from) + 1;
    for frame in spec.from..=spec.to {
        let pixels = render_frame_to_pixels(comp, frame, spec.w, spec.h, spec.exposure, spec.lut);
        let path = format!("{}/frame_{:05}.png", spec.output, frame);
        write_png(&path, &pixels, spec.w, spec.h)?;

        let progress = frame.saturating_sub(spec.from) + 1;
        if progress % 10 == 0 || progress == total {
            eprint!("\r  Rendered {}/{} frames", progress, total);
        }
    }
    eprintln!("\n  Done! Output: {}/", spec.output);
    Ok(())
}

/// Render a single frame as JPEG with quality setting.
#[allow(dead_code)]
fn write_jpeg(
    path: &str,
    rgba: &[u8],
    width: u32,
    height: u32,
    quality: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    let img = image::RgbaImage::from_raw(width, height, rgba.to_vec())
        .ok_or("pixel buffer size mismatch")?;
    let rgb = image::DynamicImage::ImageRgba8(img).to_rgb8();
    let file = std::fs::File::create(path)?;
    let mut writer = std::io::BufWriter::new(file);
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut writer, quality.min(100));
    rgb.write_with_encoder(encoder)?;
    Ok(())
}

fn render_to_mp4(
    comp: &Composition,
    spec: &RenderSpec,
    production: Option<&kagari_vfx::core::production_document::ProductionDocument>,
    composition_index: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    use kagari_vfx::core::ffmpeg_export::{
        is_ffmpeg_available, start_export_cancelable, ExportConfig,
    };
    use std::sync::{atomic::AtomicBool, Arc};

    if !is_ffmpeg_available() {
        return Err("ffmpeg is not installed or not in PATH".into());
    }

    let cancel_flag = Arc::new(AtomicBool::new(false));
    let cancel_clone = cancel_flag.clone();
    let (tx, rx) = std::sync::mpsc::channel();

    let audio_wav = comp.layers.iter().find_map(|l| match &l.layer_type {
        kagari_vfx::core::timeline::LayerType::Video { audio_wav, .. } => audio_wav.clone(),
        _ => None,
    });
    let config = ExportConfig {
        output_path: spec.output.to_string(),
        width: spec.w,
        height: spec.h,
        fps: comp.fps,
        total_frames: spec.to.saturating_sub(spec.from) + 1,
        audio_wav,
        codec: kagari_vfx::core::ffmpeg_export::VideoCodec::H264,
    };

    let (from, _to, w, h, exposure, lut) =
        (spec.from, spec.to, spec.w, spec.h, spec.exposure, spec.lut);
    let comp_clone = comp.clone();
    let production_clone = production.cloned();
    let binding_values = spec.bindings.clone();
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let start_export = move || {
        let result = start_export_cancelable(config, tx, cancel_clone, move |frame_idx| {
            let actual_frame = from + frame_idx;
            let frame_comp = production_clone.as_ref().and_then(|document| {
                document.composition_for_frame_with_sources(
                    composition_index,
                    actual_frame,
                    &binding_values,
                )
            });
            render_frame_to_pixels(
                frame_comp.as_ref().unwrap_or(&comp_clone),
                actual_frame,
                w,
                h,
                exposure,
                lut,
            )
        });
        let _ = done_tx.send(result);
    };

    let worker = std::thread::spawn(start_export);
    let mut event_error = None;

    while let Ok(event) = rx.recv() {
        match event {
            kagari_vfx::ExportEvent::Progress(frac, msg) => {
                let pct = (frac * 100.0) as u32;
                eprint!("\r  Encoding: {}% {}", pct, msg);
            }
            kagari_vfx::ExportEvent::Finished(msg) => {
                eprintln!("\n  {}", msg);
            }
            kagari_vfx::ExportEvent::Error(msg) => {
                eprintln!("\n  Error: {}", msg);
                event_error = Some(msg);
            }
        }
    }

    let worker_result = done_rx.recv();
    let worker_join = worker.join();
    if worker_join.is_err() {
        return Err("FFmpeg export worker panicked".into());
    }
    let worker_result = worker_result
        .map_err(|_| "FFmpeg export worker terminated without a result".to_string())?;

    if let Some(error) = event_error {
        return Err(error.into());
    }
    worker_result.map_err(|error| error.into())
}

fn render_to_gif(
    comp: &Composition,
    spec: &RenderSpec,
    production: Option<&kagari_vfx::core::production_document::ProductionDocument>,
    composition_index: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    use kagari_vfx::core::ffmpeg_export::{is_ffmpeg_available, start_gif_export, ExportConfig};
    use std::sync::{atomic::AtomicBool, Arc};

    if !is_ffmpeg_available() {
        return Err("ffmpeg is not installed or not in PATH".into());
    }

    let cancel_flag = Arc::new(AtomicBool::new(false));
    let cancel_clone = cancel_flag.clone();
    let (tx, rx) = std::sync::mpsc::channel();

    let audio_wav = comp.layers.iter().find_map(|l| match &l.layer_type {
        kagari_vfx::core::timeline::LayerType::Video { audio_wav, .. } => audio_wav.clone(),
        _ => None,
    });
    let config = ExportConfig {
        output_path: spec.output.to_string(),
        width: spec.w,
        height: spec.h,
        fps: comp.fps,
        total_frames: spec.to.saturating_sub(spec.from) + 1,
        audio_wav,
        codec: kagari_vfx::core::ffmpeg_export::VideoCodec::H264,
    };

    let (from, w, h, exposure, lut) = (spec.from, spec.w, spec.h, spec.exposure, spec.lut);
    let comp_clone = comp.clone();
    let production_clone = production.cloned();
    let binding_values = spec.bindings.clone();
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let start_export = move || {
        let result = start_gif_export(config, tx, cancel_clone, move |frame_idx| {
            let actual_frame = from + frame_idx;
            let frame_comp = production_clone.as_ref().and_then(|document| {
                document.composition_for_frame_with_sources(
                    composition_index,
                    actual_frame,
                    &binding_values,
                )
            });
            render_frame_to_pixels(
                frame_comp.as_ref().unwrap_or(&comp_clone),
                actual_frame,
                w,
                h,
                exposure,
                lut,
            )
        });
        let _ = done_tx.send(result);
    };

    let worker = std::thread::spawn(start_export);
    let mut event_error = None;

    while let Ok(event) = rx.recv() {
        match event {
            kagari_vfx::ExportEvent::Progress(frac, msg) => {
                let pct = (frac * 100.0) as u32;
                eprint!("\r  GIF Encoding: {}% {}", pct, msg);
            }
            kagari_vfx::ExportEvent::Finished(msg) => {
                eprintln!("\n  {}", msg);
            }
            kagari_vfx::ExportEvent::Error(msg) => {
                eprintln!("\n  Error: {}", msg);
                event_error = Some(msg);
            }
        }
    }

    let worker_result = done_rx.recv();
    let worker_join = worker.join();
    if worker_join.is_err() {
        return Err("GIF export worker panicked".into());
    }
    let worker_result =
        worker_result.map_err(|_| "GIF export worker terminated without a result".to_string())?;

    if let Some(error) = event_error {
        return Err(error.into());
    }
    worker_result.map_err(|error| error.into())
}
