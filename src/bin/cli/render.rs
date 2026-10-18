use super::frame::write_png;
use super::*;

/// Bundled render parameters shared by all export backends.
struct RenderSpec<'a> {
    output: &'a str,
    frames: Vec<u32>,
    from: u32,
    to: u32,
    #[allow(dead_code)]
    step: u32,
    w: u32,
    h: u32,
    exposure: f32,
    lut: u32,
    skip_existing: bool,
    log_json: bool,
    bindings: &'a std::collections::HashMap<String, f64>,
}

pub(super) struct RenderArgs {
    pub(super) project_path: String,
    pub(super) comp_ref: Option<String>,
    pub(super) output: String,
    pub(super) format: OutputFormat,
    pub(super) frames: Option<String>,
    pub(super) from: u32,
    pub(super) to: Option<u32>,
    pub(super) width: Option<u32>,
    pub(super) height: Option<u32>,
    pub(super) exposure: f32,
    pub(super) lut: u32,
    pub(super) color_space: Option<String>,
    pub(super) step: u32,
    pub(super) threads: usize,
    pub(super) skip_existing: bool,
    pub(super) mfr: bool,
    pub(super) log_format: String,
    pub(super) bindings: Vec<(String, f64)>,
    pub(super) overrides: Vec<(String, String)>,
}

/// Parse AE / Nuke style frame spec like "1-10,15,20-50" into sorted unique u32 frames.
fn parse_frame_list(s: &str, max_frame: u32, step: u32) -> Result<Vec<u32>, String> {
    let mut result = std::collections::BTreeSet::new();
    let step = step.max(1) as usize;
    for chunk in s.split(',') {
        let chunk = chunk.trim();
        if chunk.is_empty() {
            continue;
        }
        if let Some((start_s, end_s)) = chunk.split_once('-') {
            let start: u32 = start_s.trim().parse().map_err(|_| format!("Invalid frame number: '{start_s}'"))?;
            let end: u32 = end_s.trim().parse().map_err(|_| format!("Invalid frame number: '{end_s}'"))?;
            if start > end {
                return Err(format!("Invalid range: {start}-{end}"));
            }
            if end > max_frame {
                return Err(format!("Frame {end} exceeds composition duration ({max_frame})"));
            }
            for f in (start..=end).step_by(step) {
                result.insert(f);
            }
        } else {
            let f: u32 = chunk.parse().map_err(|_| format!("Invalid frame number: '{chunk}'"))?;
            if f > max_frame {
                return Err(format!("Frame {f} exceeds composition duration ({max_frame})"));
            }
            result.insert(f);
        }
    }
    if result.is_empty() {
        return Err("No frames specified in frame list".to_string());
    }
    Ok(result.into_iter().collect())
}

pub(super) fn cmd_render(args: RenderArgs) -> Result<(), Box<dyn std::error::Error>> {
    let production = kagari_vfx::core::project_migration::read_project_json_file(
        std::path::Path::new(&args.project_path),
    )
    .ok()
    .and_then(|json| {
        kagari_vfx::core::production_document::ProductionDocument::from_json(&json).ok()
    });
    let mut project = if let Some(document) = production.as_ref() {
        document.project.clone()
    } else {
        load_project(&args.project_path)?
    };

    // Apply template overrides (text replacement, image/video path replacement)
    for (target, val) in &args.overrides {
        let (layer_name, prop_name) = match target.split_once('.') {
            Some((l, p)) => (l.trim(), Some(p.trim())),
            None => (target.as_str(), None),
        };
        let mut matched = false;
        for c in &mut project.compositions {
            for layer in &mut c.layers {
                if layer.name == layer_name || layer.id == layer_name {
                    matched = true;
                    match (&mut layer.layer_type, prop_name) {
                        (kagari_vfx::core::timeline::LayerType::Text { text, .. }, None | Some("text")) => {
                            *text = val.clone();
                            eprintln!("Applied override: [{}].text = \"{}\"", layer.name, val);
                        }
                        (kagari_vfx::core::timeline::LayerType::Image { path }, None | Some("src" | "path")) => {
                            *path = val.clone();
                            eprintln!("Applied override: [{}].path = \"{}\"", layer.name, val);
                        }
                        (kagari_vfx::core::timeline::LayerType::Video { source, .. }, None | Some("src" | "source")) => {
                            *source = val.clone();
                            eprintln!("Applied override: [{}].source = \"{}\"", layer.name, val);
                        }
                        (kagari_vfx::core::timeline::LayerType::Solid { color }, Some("color")) => {
                            // Parse hex or comma-separated RGBA if provided
                            let parts: Vec<f32> = val.split(',').filter_map(|s| s.trim().parse::<f32>().ok()).collect();
                            if parts.len() >= 3 {
                                color[0] = parts[0];
                                color[1] = parts[1];
                                color[2] = parts[2];
                                if parts.len() >= 4 {
                                    color[3] = parts[3];
                                }
                                eprintln!("Applied override: [{}].color = {:?}", layer.name, color);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        if !matched {
            eprintln!("Warning: Template override target '{}' not found in any composition", target);
        }
    }

    if let Some(cs_str) = &args.color_space {
        let cs = match cs_str.to_lowercase().replace(['-', '_', '.'], "").as_str() {
            "rec709" | "srgb" => kagari_vfx::core::color_science::WorkingColorSpace::Rec709,
            "displayp3" | "p3" => kagari_vfx::core::color_science::WorkingColorSpace::DisplayP3,
            "rec2020" | "bt2020" => kagari_vfx::core::color_science::WorkingColorSpace::Rec2020,
            "linear" | "linearsrgb" => kagari_vfx::core::color_science::WorkingColorSpace::LinearSRGB,
            "acescg" | "aces" | "ap1" => kagari_vfx::core::color_science::WorkingColorSpace::AcesCG,
            unknown => {
                return Err(format!("Unknown color space '{}'. Valid options: rec709, display_p3, rec2020, linear, acescg", unknown).into());
            }
        };
        for c in &mut project.compositions {
            c.working_color_space = cs;
        }
        eprintln!("Applied working color space override: {:?}", cs);
    }

    if args.threads > 0 {
        let _ = rayon::ThreadPoolBuilder::new()
            .num_threads(args.threads)
            .build_global();
        eprintln!("Configured worker thread pool to {} threads", args.threads);
    }

    kagari_vfx::core::parallel_render::set_mfr_enabled(args.mfr);
    eprintln!("Multi-Frame Rendering (MFR): {}", if args.mfr { "ENABLED" } else { "DISABLED" });

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
    let max_frame = comp.duration_frames.saturating_sub(1);
    let step = args.step.max(1);
    let frames = if let Some(ref s) = args.frames {
        parse_frame_list(s, max_frame, step)?
    } else {
        let to = args.to.unwrap_or(max_frame);
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
        (args.from..=to).step_by(step as usize).collect()
    };

    let from = frames.first().copied().unwrap_or(0);
    let to = frames.last().copied().unwrap_or(0);
    let log_json = args.log_format.eq_ignore_ascii_case("json");

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
        frames: frames.clone(),
        from,
        to,
        step,
        w: width,
        h: height,
        exposure: args.exposure,
        lut: args.lut,
        skip_existing: args.skip_existing,
        log_json,
        bindings: &binding_values,
    };
    let format = args.format;

    if !log_json {
        eprintln!("Rendering composition: {}", comp.name);
        eprintln!("  Size: {}x{}", spec.w, spec.h);
        eprintln!("  Frames: {} selected (range {}..={})", spec.frames.len(), spec.from, spec.to);
        eprintln!("  Format: {:?}", format);
    } else {
        println!(
            "{{\"event\":\"start\",\"composition\":\"{}\",\"width\":{},\"height\":{},\"total_frames\":{}}}",
            comp.name, spec.w, spec.h, spec.frames.len()
        );
    }

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
        OutputFormat::Exr => {
            render_to_exr_sequence(comp, &spec)?;
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
    let total = spec.frames.len();
    for (idx, &frame) in spec.frames.iter().enumerate() {
        let path = format!("{}/frame_{:05}.png", spec.output, frame);
        if spec.skip_existing && std::path::Path::new(&path).is_file() {
            if spec.log_json {
                println!("{{\"event\":\"skip\",\"frame\":{},\"path\":\"{}\"}}", frame, path);
            } else {
                eprintln!("  Skipping existing frame {:05} ({})", frame, path);
            }
            continue;
        }
        let t0 = std::time::Instant::now();
        let comp = document
            .composition_for_frame_with_sources(composition_index, frame, spec.bindings)
            .ok_or("composition index out of range")?;
        let pixels = render_frame_to_pixels(&comp, frame, spec.w, spec.h, spec.exposure, spec.lut);
        write_png(&path, &pixels, spec.w, spec.h)?;
        let elapsed_ms = t0.elapsed().as_secs_f64() * 1000.0;
        let progress = idx + 1;
        if spec.log_json {
            println!(
                "{{\"event\":\"frame_done\",\"frame\":{},\"index\":{},\"total\":{},\"duration_ms\":{:.1},\"path\":\"{}\"}}",
                frame, progress, total, elapsed_ms, path
            );
        } else if progress % 10 == 0 || progress == total {
            eprint!("\r  Rendered {}/{} frames", progress, total);
        }
    }
    if spec.log_json {
        println!("{{\"event\":\"completed\",\"output\":\"{}\"}}", spec.output);
    } else {
        eprintln!("\n  Done! Output: {}/", spec.output);
    }
    Ok(())
}

fn render_to_exr_sequence(
    comp: &Composition,
    spec: &RenderSpec,
) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(spec.output)?;
    let total = spec.frames.len();
    for (idx, &frame) in spec.frames.iter().enumerate() {
        let path = format!("{}/frame_{:05}.exr", spec.output, frame);
        if spec.skip_existing && std::path::Path::new(&path).is_file() {
            if spec.log_json {
                println!("{{\"event\":\"skip\",\"frame\":{},\"path\":\"{}\"}}", frame, path);
            } else {
                eprintln!("  Skipping existing frame {:05} ({})", frame, path);
            }
            continue;
        }
        let t0 = std::time::Instant::now();
        let f32_pixels = kagari_vfx::core::software_renderer::render_frame_to_pixels_f32(
            comp,
            frame,
            spec.w,
            spec.h,
            spec.exposure,
            spec.lut,
        );
        let mut flat = Vec::with_capacity(f32_pixels.len() * 4);
        for px in f32_pixels {
            flat.extend_from_slice(&px);
        }
        let image = kagari_vfx::core::exr_writer::ExrImage::rgba_f32(spec.w, spec.h, &flat);
        kagari_vfx::core::exr_writer::write_exr_file(&path, &image)
            .map_err(|e| format!("EXR write error: {e}"))?;

        let elapsed_ms = t0.elapsed().as_secs_f64() * 1000.0;
        let progress = idx + 1;
        if spec.log_json {
            println!(
                "{{\"event\":\"frame_done\",\"frame\":{},\"index\":{},\"total\":{},\"duration_ms\":{:.1},\"path\":\"{}\"}}",
                frame, progress, total, elapsed_ms, path
            );
        } else if progress % 10 == 0 || progress == total {
            eprint!("\r  Rendered {}/{} frames", progress, total);
        }
    }
    if spec.log_json {
        println!("{{\"event\":\"completed\",\"output\":\"{}\"}}", spec.output);
    } else {
        eprintln!("\n  Done! Output: {}/", spec.output);
    }
    Ok(())
}

fn render_to_png_sequence(
    comp: &Composition,
    spec: &RenderSpec,
) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(spec.output)?;
    let total = spec.frames.len();
    for (idx, &frame) in spec.frames.iter().enumerate() {
        let path = format!("{}/frame_{:05}.png", spec.output, frame);
        if spec.skip_existing && std::path::Path::new(&path).is_file() {
            if spec.log_json {
                println!("{{\"event\":\"skip\",\"frame\":{},\"path\":\"{}\"}}", frame, path);
            } else {
                eprintln!("  Skipping existing frame {:05} ({})", frame, path);
            }
            continue;
        }
        let t0 = std::time::Instant::now();
        let pixels = render_frame_to_pixels(comp, frame, spec.w, spec.h, spec.exposure, spec.lut);
        write_png(&path, &pixels, spec.w, spec.h)?;

        let elapsed_ms = t0.elapsed().as_secs_f64() * 1000.0;
        let progress = idx + 1;
        if spec.log_json {
            println!(
                "{{\"event\":\"frame_done\",\"frame\":{},\"index\":{},\"total\":{},\"duration_ms\":{:.1},\"path\":\"{}\"}}",
                frame, progress, total, elapsed_ms, path
            );
        } else if progress % 10 == 0 || progress == total {
            eprint!("\r  Rendered {}/{} frames", progress, total);
        }
    }
    if spec.log_json {
        println!("{{\"event\":\"completed\",\"output\":\"{}\"}}", spec.output);
    } else {
        eprintln!("\n  Done! Output: {}/", spec.output);
    }
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
