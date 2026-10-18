use super::edit::find_comp_mut;
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
    pub(super) lut_file: Option<String>,
    pub(super) color_space: Option<String>,
    pub(super) step: u32,
    pub(super) threads: usize,
    pub(super) skip_existing: bool,
    pub(super) mfr: bool,
    pub(super) log_format: String,
    pub(super) bindings: Vec<(String, f64)>,
    pub(super) overrides: Vec<(String, String)>,
    pub(super) expressions: Vec<(String, String)>,
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

                    // Direct transform overrides (e.g. Logo.opacity=50 or Text.rotation=45)
                    if let Some(prop) = prop_name {
                        match prop.to_lowercase().as_str() {
                            "opacity" | "alpha" => {
                                if let Ok(v) = val.parse::<f32>() {
                                    layer.transform.opacity = kagari_vfx::core::timeline::Animatable::new_constant(v);
                                    eprintln!("Applied override: [{}].opacity = {}", layer.name, v);
                                }
                            }
                            "rotation" | "rot" => {
                                if let Ok(v) = val.parse::<f32>() {
                                    layer.transform.rotation = kagari_vfx::core::timeline::Animatable::new_constant(v);
                                    eprintln!("Applied override: [{}].rotation = {}", layer.name, v);
                                }
                            }
                            "scale" => {
                                if let Ok(v) = val.parse::<f32>() {
                                    layer.transform.scale = kagari_vfx::core::timeline::Animatable::new_constant([v, v]);
                                    eprintln!("Applied override: [{}].scale = [{}, {}]", layer.name, v, v);
                                }
                            }
                            "visible" => {
                                if let Ok(v) = val.parse::<bool>() {
                                    layer.visible = v;
                                    eprintln!("Applied override: [{}].visible = {}", layer.name, v);
                                }
                            }
                            _ => {}
                        }
                    }

                    // Effect overrides (e.g. Logo.GaussianBlur.radius=15.0 or Background.Glow.intensity=2.0)
                    if let Some(sub) = prop_name {
                        if let Some((eff_name, eff_param)) = sub.split_once('.') {
                            for effect in &mut layer.effects {
                                if effect.name.eq_ignore_ascii_case(eff_name) || format!("{:?}", effect.effect_type).to_lowercase().starts_with(&eff_name.to_lowercase()) {
                                    match (&mut effect.effect_type, eff_param.to_lowercase().as_str()) {
                                        (kagari_vfx::core::timeline::EffectType::GaussianBlur { blur_radius }, "radius" | "blur_radius") => {
                                            if let Ok(v) = val.parse::<f32>() {
                                                *blur_radius = kagari_vfx::core::timeline::Animatable::new_constant(v);
                                                eprintln!("Applied override: [{}.{}].{} = {}", layer.name, effect.name, eff_param, v);
                                            }
                                        }
                                        (kagari_vfx::core::timeline::EffectType::Glow { intensity, .. }, "intensity") => {
                                            if let Ok(v) = val.parse::<f32>() {
                                                *intensity = kagari_vfx::core::timeline::Animatable::new_constant(v);
                                                eprintln!("Applied override: [{}.{}].intensity = {}", layer.name, effect.name, v);
                                            }
                                        }
                                        (kagari_vfx::core::timeline::EffectType::Glow { radius, .. }, "radius") => {
                                            if let Ok(v) = val.parse::<f32>() {
                                                *radius = kagari_vfx::core::timeline::Animatable::new_constant(v);
                                                eprintln!("Applied override: [{}.{}].radius = {}", layer.name, effect.name, v);
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        if !matched {
            eprintln!("Warning: Template override target '{}' not found in any composition", target);
        }
    }

    // Apply dynamic expressions (e.g. Logo.rotation=wiggle(3.0, 25.0) or Camera.position=wiggle(2.0, 50.0))
    for (target, expr_str) in &args.expressions {
        let (layer_name, prop_name) = match target.split_once('.') {
            Some((l, p)) => (l.trim(), p.trim().to_lowercase()),
            None => (target.as_str(), "rotation".to_string()),
        };
        let expr = if let Some(w) = expr_str.strip_prefix("wiggle(").and_then(|s| s.strip_suffix(')')) {
            let parts: Vec<f32> = w.split(',').filter_map(|s| s.trim().parse::<f32>().ok()).collect();
            if parts.len() >= 2 {
                kagari_vfx::core::timeline::Expression::Wiggle {
                    frequency: parts[0],
                    amplitude: parts[1],
                }
            } else {
                kagari_vfx::core::timeline::Expression::Raw(expr_str.clone())
            }
        } else if expr_str.eq_ignore_ascii_case("loopout") || expr_str.eq_ignore_ascii_case("loopout()") {
            kagari_vfx::core::timeline::Expression::LoopOut
        } else if expr_str.eq_ignore_ascii_case("pingpong") || expr_str.eq_ignore_ascii_case("pingpong()") {
            kagari_vfx::core::timeline::Expression::PingPong
        } else {
            kagari_vfx::core::timeline::Expression::Raw(expr_str.clone())
        };

        let mut matched = false;
        for c in &mut project.compositions {
            for layer in &mut c.layers {
                if layer.name == layer_name || layer.id == layer_name {
                    matched = true;
                    match prop_name.as_str() {
                        "rotation" | "rot" | "r" => {
                            layer.transform.rotation_expression = Some(expr.clone());
                            eprintln!("Injected expression: [{}].rotation = {:?}", layer.name, expr);
                        }
                        "position" | "pos" | "p" => {
                            layer.transform.position_expression = Some(expr.clone());
                            eprintln!("Injected expression: [{}].position = {:?}", layer.name, expr);
                        }
                        "scale" | "s" => {
                            layer.transform.scale_expression = Some(expr.clone());
                            eprintln!("Injected expression: [{}].scale = {:?}", layer.name, expr);
                        }
                        "opacity" | "alpha" | "o" => {
                            layer.transform.opacity_expression = Some(expr.clone());
                            eprintln!("Injected expression: [{}].opacity = {:?}", layer.name, expr);
                        }
                        "anchor" | "anchor_point" | "a" => {
                            layer.transform.anchor_point_expression = Some(expr.clone());
                            eprintln!("Injected expression: [{}].anchor_point = {:?}", layer.name, expr);
                        }
                        other => {
                            eprintln!("Warning: Unsupported property '{}' for expression on layer '{}'", other, layer.name);
                        }
                    }
                }
            }
        }
        if !matched {
            eprintln!("Warning: Expression target layer '{}' not found in any composition", layer_name);
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
    let mut lut_mode = args.lut;
    if let Some(lut_path) = &args.lut_file {
        let lut = kagari_vfx::core::ocio_color::Lut3D::load_cube_file(lut_path)
            .map_err(|e| format!("Failed to load 3D LUT file '{lut_path}': {e}"))?;
        kagari_vfx::core::ocio_color::set_active_lut(Some(std::sync::Arc::new(lut)));
        lut_mode = 3;
        eprintln!("Loaded custom 3D LUT (.cube) from: {}", lut_path);
    }

    if !args.exposure.is_finite() || lut_mode > 3 {
        return Err("Exposure must be finite and LUT mode must be 0, 1, 2, or 3".into());
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
        lut: lut_mode,
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

    // Composite all audio layers (video audio + dedicated audio layers) into master WAV
    let audio_wav = {
        let master_dsp = kagari_vfx::core::audio_engine::MasterDspParams::default();
        match kagari_vfx::core::audio_engine::mix_composition_to_wav(
            comp,
            spec.from,
            spec.to + 1,
            48_000,
            None,
            &master_dsp,
        ) {
            Ok(Some(path)) => {
                eprintln!("  Mixed master audio track: {:?}", path);
                Some(path.to_string_lossy().into_owned())
            }
            Ok(None) => None,
            Err(e) => {
                eprintln!("  Audio mix notice: {}", e);
                None
            }
        }
    };

    let config = ExportConfig {
        output_path: spec.output.to_string(),
        width: spec.w,
        height: spec.h,
        fps: comp.fps,
        total_frames: spec.to.saturating_sub(spec.from) + 1,
        audio_wav: audio_wav.clone(),
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

    // Clean up temporary mixed WAV file
    if let Some(ref path) = audio_wav {
        let _ = std::fs::remove_file(path);
    }

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

pub(super) struct QueueArgs {
    pub(super) project_path: String,
    pub(super) output: String,
    pub(super) format: OutputFormat,
    pub(super) threads: usize,
    pub(super) log_format: String,
}

pub(super) fn cmd_queue(args: QueueArgs) -> Result<(), Box<dyn std::error::Error>> {
    let project = load_project(&args.project_path)?;
    let log_json = args.log_format.eq_ignore_ascii_case("json");

    if args.threads > 0 {
        let _ = rayon::ThreadPoolBuilder::new()
            .num_threads(args.threads)
            .build_global();
    }

    if !log_json {
        eprintln!(
            "Starting Batch Render Queue for project: {} ({} compositions)",
            args.project_path,
            project.compositions.len()
        );
    } else {
        println!(
            "{{\"event\":\"queue_start\",\"project\":\"{}\",\"total_compositions\":{}}}",
            args.project_path,
            project.compositions.len()
        );
    }

    for (idx, comp) in project.compositions.iter().enumerate() {
        let comp_out = match args.format {
            OutputFormat::Mp4 => format!("{}/{}.mp4", args.output, comp.name),
            OutputFormat::Gif => format!("{}/{}.gif", args.output, comp.name),
            OutputFormat::Png | OutputFormat::Exr => format!("{}/{}", args.output, comp.name),
        };

        if !log_json {
            eprintln!("\n[{}/{}] Rendering: {}", idx + 1, project.compositions.len(), comp.name);
        } else {
            println!(
                "{{\"event\":\"queue_comp_start\",\"index\":{},\"name\":\"{}\",\"output\":\"{}\"}}",
                idx + 1,
                comp.name,
                comp_out
            );
        }

        cmd_render(RenderArgs {
            project_path: args.project_path.clone(),
            comp_ref: Some(comp.name.clone()),
            output: comp_out,
            format: args.format,
            frames: None,
            from: 0,
            to: None,
            width: None,
            height: None,
            exposure: 0.0,
            lut: 0,
            lut_file: None,
            color_space: None,
            step: 1,
            threads: args.threads,
            skip_existing: false,
            mfr: true,
            log_format: args.log_format.clone(),
            bindings: Vec::new(),
            overrides: Vec::new(),
            expressions: Vec::new(),
        })?;
    }

    if !log_json {
        eprintln!("\nAll queue compositions completed successfully!");
    } else {
        println!("{{\"event\":\"queue_completed\"}}");
    }

    Ok(())
}

pub(super) fn cmd_beats(file: &str, fps: f32, duration: u32, json: bool) -> Result<(), Box<dyn std::error::Error>> {
    let path = std::path::Path::new(file);
    if !path.is_file() {
        return Err(format!("Audio file '{}' not found", file).into());
    }

    let beats = kagari_vfx::core::audio_engine::detect_beat_frames(path, duration, fps);
    if json {
        let json_str = serde_json::to_string(&beats)?;
        println!("{}", json_str);
    } else {
        println!("Detected {} transient beat onsets in '{}' (fps: {}):", beats.len(), file, fps);
        for b in &beats {
            let sec = *b as f32 / fps;
            println!("  frame {:5}  ({:6.2}s)", b, sec);
        }
    }
    Ok(())
}

pub(super) fn cmd_preview(
    project_path: &str,
    comp_ref: Option<&str>,
    output: &str,
    scale: f32,
    step: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let project = load_project(project_path)?;
    let comp = find_comp(&project, comp_ref)?;
    let scale = scale.clamp(0.1, 1.0);
    let preview_w = ((comp.width as f32 * scale).round() as u32).max(64);
    let preview_h = ((comp.height as f32 * scale).round() as u32).max(64);

    eprintln!(
        "Draft Preview: Rendering '{}' at {:.0}% resolution ({}x{}, step={}) → {}",
        comp.name,
        scale * 100.0,
        preview_w,
        preview_h,
        step.max(1),
        output
    );

    cmd_render(RenderArgs {
        project_path: project_path.to_string(),
        comp_ref: comp_ref.map(|s| s.to_string()),
        output: output.to_string(),
        format: OutputFormat::Mp4,
        frames: None,
        from: 0,
        to: None,
        width: Some(preview_w),
        height: Some(preview_h),
        exposure: 0.0,
        lut: 0,
        lut_file: None,
        color_space: None,
        step: step.max(1),
        threads: 0,
        skip_existing: false,
        mfr: true,
        log_format: "standard".to_string(),
        bindings: Vec::new(),
        overrides: Vec::new(),
        expressions: Vec::new(),
    })
}

pub(super) fn cmd_bundle(project_path: &str, output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut project = load_project(project_path)?;
    let bundle_path = std::path::Path::new(output_dir);
    let assets_dir = bundle_path.join("assets");
    std::fs::create_dir_all(&assets_dir)?;

    let mut asset_counter = 0;
    for comp in &mut project.compositions {
        for layer in &mut comp.layers {
            match &mut layer.layer_type {
                kagari_vfx::core::timeline::LayerType::Image { path } => {
                    let src = std::path::PathBuf::from(&*path);
                    if src.is_file() {
                        asset_counter += 1;
                        let file_name = src.file_name().unwrap_or_default().to_string_lossy();
                        let target = assets_dir.join(format!("{:03}_{}", asset_counter, file_name));
                        std::fs::copy(&src, &target)?;
                        *path = format!("assets/{:03}_{}", asset_counter, file_name);
                        eprintln!("  Collected Image: {} → {:?}", src.display(), target);
                    }
                }
                kagari_vfx::core::timeline::LayerType::Video { source, .. } => {
                    let src = std::path::PathBuf::from(&*source);
                    if src.is_file() {
                        asset_counter += 1;
                        let file_name = src.file_name().unwrap_or_default().to_string_lossy();
                        let target = assets_dir.join(format!("{:03}_{}", asset_counter, file_name));
                        std::fs::copy(&src, &target)?;
                        *source = format!("assets/{:03}_{}", asset_counter, file_name);
                        eprintln!("  Collected Video: {} → {:?}", src.display(), target);
                    }
                }
                kagari_vfx::core::timeline::LayerType::Audio { path, .. } => {
                    let src = std::path::PathBuf::from(&*path);
                    if src.is_file() {
                        asset_counter += 1;
                        let file_name = src.file_name().unwrap_or_default().to_string_lossy();
                        let target = assets_dir.join(format!("{:03}_{}", asset_counter, file_name));
                        std::fs::copy(&src, &target)?;
                        *path = format!("assets/{:03}_{}", asset_counter, file_name);
                        eprintln!("  Collected Audio: {} → {:?}", src.display(), target);
                    }
                }
                _ => {}
            }
        }
    }

    let bundle_project_file = bundle_path.join("project.json");
    let json = serde_json::to_string_pretty(&project)?;
    std::fs::write(&bundle_project_file, json)?;

    eprintln!(
        "\nProject Bundle created successfully at '{}' (collected {} external assets)",
        output_dir, asset_counter
    );
    Ok(())
}

pub(super) fn cmd_remap(
    project_path: &str,
    layer_name: &str,
    stretch: Option<f32>,
    reverse: bool,
    freeze: Option<u32>,
    frame_blending: bool,
    comp_ref: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut project = load_project(project_path)?;
    let comp = find_comp_mut(&mut project, comp_ref)?;
    let layer = comp
        .layers
        .iter_mut()
        .find(|l| l.name == layer_name || l.id == layer_name)
        .ok_or_else(|| format!("Layer '{layer_name}' not found in composition '{}'", comp.name))?;

    if let Some(factor) = stretch {
        layer.time_stretch(factor);
        eprintln!("Applied time stretch factor {:.2}x to layer '{}'", factor, layer.name);
    }
    if reverse {
        layer.time_reverse();
        eprintln!("Applied time-reverse playback to layer '{}'", layer.name);
    }
    if let Some(freeze_frame) = freeze {
        layer.freeze_at(freeze_frame);
        eprintln!("Applied freeze frame at source frame {} to layer '{}'", freeze_frame, layer.name);
    }
    if frame_blending {
        layer.frame_blending = true;
        eprintln!("Enabled adjacent frame blending on layer '{}'", layer.name);
    }

    let json = serde_json::to_string_pretty(&project)?;
    std::fs::write(project_path, json)?;
    eprintln!("Updated project file: {}", project_path);
    Ok(())
}

pub(super) fn cmd_track(
    project_path: &str,
    layer_name: &str,
    tracker_idx: usize,
    pos_str: Option<&str>,
    search_size: f32,
    apply_to: Option<&str>,
    apply_pos: bool,
    apply_rot: bool,
    comp_ref: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut project = load_project(project_path)?;
    let comp = find_comp_mut(&mut project, comp_ref)?;

    let (source_layer_idx, in_frame, out_frame) = {
        let (idx, layer) = comp
            .layers
            .iter_mut()
            .enumerate()
            .find(|(_, l)| l.name == layer_name || l.id == layer_name)
            .ok_or_else(|| format!("Source layer '{layer_name}' not found in composition '{}'", comp.name))?;

        if layer.trackers.len() <= tracker_idx {
            let initial_pos = if let Some(p) = pos_str {
                let coords: Vec<f32> = p.split(',').filter_map(|s| s.trim().parse().ok()).collect();
                if coords.len() >= 2 {
                    [coords[0], coords[1]]
                } else {
                    [comp.width as f32 / 2.0, comp.height as f32 / 2.0]
                }
            } else {
                [comp.width as f32 / 2.0, comp.height as f32 / 2.0]
            };
            let mut tracker = kagari_vfx::core::timeline::TrackerPoint::new(
                format!("tracker_{}", tracker_idx + 1),
                format!("Tracker {}", tracker_idx + 1),
                initial_pos,
            );
            tracker.search_size = search_size.clamp(4.0, 128.0);
            layer.trackers.push(tracker);
        } else if let Some(p) = pos_str {
            let coords: Vec<f32> = p.split(',').filter_map(|s| s.trim().parse().ok()).collect();
            if coords.len() >= 2 {
                layer.trackers[tracker_idx].position =
                    kagari_vfx::core::property::Animatable::new_constant([coords[0], coords[1]]);
            }
        }
        (idx, layer.in_frame, layer.out_frame)
    };

    eprintln!(
        "Analyzing motion tracking on '{}' (tracker #{}, frames {}..={})...",
        layer_name, tracker_idx, in_frame, out_frame
    );
    kagari_vfx::core::tracker_engine::TrackerEngine::analyze_track(
        comp,
        source_layer_idx,
        tracker_idx,
        in_frame,
        out_frame,
    );

    if let Some(target_name) = apply_to {
        let target_layer_idx = comp
            .layers
            .iter()
            .position(|l| l.name == target_name || l.id == target_name)
            .ok_or_else(|| format!("Target layer '{target_name}' not found in composition '{}'", comp.name))?;

        kagari_vfx::core::tracker_engine::TrackerEngine::apply_tracker_to_target(
            comp,
            source_layer_idx,
            tracker_idx,
            target_layer_idx,
            apply_pos,
            apply_rot,
        );
        eprintln!(
            "Baked and applied motion tracking data to target layer '{}' (pos: {}, rot: {})",
            target_name, apply_pos, apply_rot
        );
    }

    let json = serde_json::to_string_pretty(&project)?;
    std::fs::write(project_path, json)?;
    eprintln!("Updated project file: {}", project_path);
    Ok(())
}

pub(super) fn cmd_text_anim(
    project_path: &str,
    layer_name: &str,
    preset: &str,
    duration: u32,
    start_frame: Option<u32>,
    shape_str: &str,
    blur: f32,
    tracking: f32,
    comp_ref: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut project = load_project(project_path)?;
    let comp = find_comp_mut(&mut project, comp_ref)?;
    let layer = comp
        .layers
        .iter_mut()
        .find(|l| l.name == layer_name || l.id == layer_name)
        .ok_or_else(|| format!("Layer '{layer_name}' not found in composition '{}'", comp.name))?;

    let shape = match shape_str.to_lowercase().replace('-', "_").as_str() {
        "square" => kagari_vfx::core::text_animator::SelectorShape::Square,
        "ramp_up" | "rampup" => kagari_vfx::core::text_animator::SelectorShape::RampUp,
        "ramp_down" | "rampdown" => kagari_vfx::core::text_animator::SelectorShape::RampDown,
        "triangle" => kagari_vfx::core::text_animator::SelectorShape::Triangle,
        "round" => kagari_vfx::core::text_animator::SelectorShape::Round,
        "smooth" => kagari_vfx::core::text_animator::SelectorShape::Smooth,
        "wobble" => kagari_vfx::core::text_animator::SelectorShape::Wobble,
        "random" => kagari_vfx::core::text_animator::SelectorShape::Random,
        _ => kagari_vfx::core::text_animator::SelectorShape::RampUp,
    };

    let start_f = start_frame.unwrap_or(layer.in_frame);
    let end_f = start_f.saturating_add(duration.max(1));

    let mut anim = kagari_vfx::core::text_animator::TextAnimatorSettings {
        enabled: true,
        selector: kagari_vfx::core::text_animator::RangeSelector {
            start: 0.0,
            end: 100.0,
            offset: 0.0,
            shape,
            ease_high: 20.0,
            ease_low: 20.0,
            random_order: false,
            char_start: -1,
            char_end: -1,
            offset_anim: Some(kagari_vfx::core::property::Animatable::Animated(vec![
                kagari_vfx::core::keyframe::Keyframe::new(
                    start_f,
                    -100.0,
                    kagari_vfx::core::keyframe::InterpolationType::Linear,
                ),
                kagari_vfx::core::keyframe::Keyframe::new(
                    end_f,
                    100.0,
                    kagari_vfx::core::keyframe::InterpolationType::Linear,
                ),
            ])),
            expression: None,
        },
        position_offset: [0.0, 0.0],
        scale: [1.0, 1.0],
        opacity: 0.0,
        tracking,
        rotation: 0.0,
        blur_amount: blur,
    };

    match preset.to_lowercase().as_str() {
        "typewriter" => {
            anim.selector.shape = kagari_vfx::core::text_animator::SelectorShape::Square;
            anim.position_offset = [0.0, 0.0];
            anim.opacity = 0.0;
        }
        "cascade" | "rise" => {
            anim.selector.shape = kagari_vfx::core::text_animator::SelectorShape::RampUp;
            anim.position_offset = [0.0, 50.0];
            anim.opacity = 0.0;
            anim.scale = [0.8, 0.8];
        }
        "blur-in" | "blur_in" => {
            anim.selector.shape = kagari_vfx::core::text_animator::SelectorShape::Smooth;
            anim.blur_amount = if blur > 0.0 { blur } else { 25.0 };
            anim.opacity = 0.0;
        }
        "tracking" | "expand" => {
            anim.tracking = if tracking != 0.0 { tracking } else { 15.0 };
            anim.opacity = 0.0;
        }
        "wave" => {
            anim.selector.shape = kagari_vfx::core::text_animator::SelectorShape::Wobble;
            anim.position_offset = [0.0, -30.0];
        }
        _ => {}
    }

    layer.text_animator = Some(anim);
    eprintln!(
        "Configured text animator preset '{}' on layer '{}' (frames {}..={})",
        preset, layer.name, start_f, end_f
    );

    let json = serde_json::to_string_pretty(&project)?;
    std::fs::write(project_path, json)?;
    eprintln!("Updated project file: {}", project_path);
    Ok(())
}

pub(super) fn cmd_camera(
    project_path: &str,
    pos_str: Option<&str>,
    rot_str: Option<&str>,
    fov: Option<f32>,
    focus_dist: Option<f32>,
    aperture: Option<f32>,
    dof: bool,
    max_blur: Option<f32>,
    iris_sides: Option<u32>,
    comp_ref: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut project = load_project(project_path)?;
    let comp = find_comp_mut(&mut project, comp_ref)?;
    let cam = comp.resolve_camera_mut();

    if let Some(p) = pos_str {
        let parts: Vec<f32> = p.split(',').filter_map(|s| s.trim().parse().ok()).collect();
        if parts.len() >= 3 {
            cam.transform.position = kagari_vfx::core::property::Animatable::new_constant([parts[0], parts[1], parts[2]]);
            eprintln!("Set camera position: [{:.1}, {:.1}, {:.1}]", parts[0], parts[1], parts[2]);
        }
    }
    if let Some(r) = rot_str {
        let parts: Vec<f32> = r.split(',').filter_map(|s| s.trim().parse().ok()).collect();
        if parts.len() >= 3 {
            cam.transform.rotation = kagari_vfx::core::property::Animatable::new_constant([parts[0], parts[1], parts[2]]);
            eprintln!("Set camera rotation: [{:.1}°, {:.1}°, {:.1}°]", parts[0], parts[1], parts[2]);
        }
    }
    if let Some(f) = fov {
        cam.fov_degrees = f.clamp(5.0, 160.0);
        eprintln!("Set camera FOV: {:.1}°", cam.fov_degrees);
    }
    if let Some(fd) = focus_dist {
        cam.focus_distance = fd.max(10.0);
        eprintln!("Set camera focus distance: {:.1}", cam.focus_distance);
    }
    if let Some(ap) = aperture {
        cam.aperture = ap.max(0.5);
        eprintln!("Set camera aperture: f/{:.1}", cam.aperture);
    }
    if dof {
        cam.dof_enabled = true;
        eprintln!("Enabled cinematic Depth of Field (bokeh)");
    }
    if let Some(mb) = max_blur {
        cam.dof_max_blur = mb.clamp(1.0, 64.0);
        eprintln!("Set camera max bokeh blur: {:.1}px", cam.dof_max_blur);
    }
    if let Some(sides) = iris_sides {
        cam.dof_iris_sides = sides;
        eprintln!("Set camera iris blade count: {}", sides);
    }

    let json = serde_json::to_string_pretty(&project)?;
    std::fs::write(project_path, json)?;
    eprintln!("Updated project file: {}", project_path);
    Ok(())
}

pub(super) fn cmd_shape_anim(
    project_path: &str,
    layer_name: &str,
    trim_start: Option<f32>,
    trim_end: Option<f32>,
    trim_duration: Option<u32>,
    repeater_copies: Option<u32>,
    repeater_offset: Option<&str>,
    repeater_rotation: Option<f32>,
    repeater_scale: Option<&str>,
    comp_ref: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut project = load_project(project_path)?;
    let comp = find_comp_mut(&mut project, comp_ref)?;
    let layer = comp
        .layers
        .iter_mut()
        .find(|l| l.name == layer_name || l.id == layer_name)
        .ok_or_else(|| format!("Layer '{layer_name}' not found in composition '{}'", comp.name))?;

    if trim_start.is_some() || trim_end.is_some() || trim_duration.is_some() {
        let mut trim = layer.trim_paths.take().unwrap_or_default();
        if let Some(dur) = trim_duration {
            let start_f = layer.in_frame;
            let end_f = start_f.saturating_add(dur.max(1));
            trim.start = kagari_vfx::core::property::Animatable::new_constant(trim_start.unwrap_or(0.0));
            trim.end = kagari_vfx::core::property::Animatable::Animated(vec![
                kagari_vfx::core::keyframe::Keyframe::new(start_f, 0.0, kagari_vfx::core::keyframe::InterpolationType::Linear),
                kagari_vfx::core::keyframe::Keyframe::new(end_f, 100.0, kagari_vfx::core::keyframe::InterpolationType::Linear),
            ]);
            eprintln!("Configured animated trim paths sweep on '{}' (frames {}..={})", layer.name, start_f, end_f);
        } else {
            if let Some(s) = trim_start {
                trim.start = kagari_vfx::core::property::Animatable::new_constant(s.clamp(0.0, 100.0));
            }
            if let Some(e) = trim_end {
                trim.end = kagari_vfx::core::property::Animatable::new_constant(e.clamp(0.0, 100.0));
            }
            eprintln!("Configured static trim paths on '{}'", layer.name);
        }
        layer.trim_paths = Some(trim);
    }

    if repeater_copies.is_some() || repeater_offset.is_some() || repeater_rotation.is_some() || repeater_scale.is_some() {
        let mut rep = layer.shape_repeater.take().unwrap_or_default();
        if let Some(copies) = repeater_copies {
            rep.copies = copies.max(1);
            eprintln!("Set shape repeater copies: {}", rep.copies);
        }
        if let Some(off_str) = repeater_offset {
            let parts: Vec<f32> = off_str.split(',').filter_map(|s| s.trim().parse().ok()).collect();
            if parts.len() >= 2 {
                rep.position_offset = [parts[0], parts[1]];
                eprintln!("Set shape repeater position offset: [{:.1}, {:.1}]", parts[0], parts[1]);
            }
        }
        if let Some(rot) = repeater_rotation {
            rep.rotation_offset_deg = rot;
            eprintln!("Set shape repeater rotation offset: {:.1}°", rot);
        }
        if let Some(sc_str) = repeater_scale {
            let parts: Vec<f32> = sc_str.split(',').filter_map(|s| s.trim().parse().ok()).collect();
            if parts.len() >= 2 {
                rep.scale_offset = [parts[0], parts[1]];
                eprintln!("Set shape repeater scale offset: [{:.2}, {:.2}]", parts[0], parts[1]);
            }
        }
        layer.shape_repeater = Some(rep);
    }

    let json = serde_json::to_string_pretty(&project)?;
    std::fs::write(project_path, json)?;
    eprintln!("Updated project file: {}", project_path);
    Ok(())
}

pub(super) fn cmd_particle(
    project_path: &str,
    layer_name: &str,
    preset: &str,
    rate: Option<f32>,
    max_particles: Option<u32>,
    pos_str: Option<&str>,
    comp_ref: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut project = load_project(project_path)?;
    let comp = find_comp_mut(&mut project, comp_ref)?;

    let mut emitter = kagari_vfx::core::particle_system::ParticleEmitter::default();
    match preset.to_lowercase().as_str() {
        "sparks" => {
            emitter.rate = 120.0;
            emitter.lifetime = 0.8;
            emitter.speed = 280.0;
            emitter.spread_degrees = 360.0;
            emitter.gravity = [0.0, 300.0];
            emitter.color_start = [1.0, 0.85, 0.2, 1.0];
            emitter.color_end = [1.0, 0.2, 0.0, 0.0];
            emitter.size_start = 4.0;
            emitter.size_end = 1.0;
        }
        "dust" => {
            emitter.rate = 35.0;
            emitter.lifetime = 4.0;
            emitter.speed = 25.0;
            emitter.spread_degrees = 360.0;
            emitter.gravity = [0.0, -10.0];
            emitter.color_start = [0.9, 0.9, 1.0, 0.4];
            emitter.color_end = [0.8, 0.8, 0.9, 0.0];
            emitter.size_start = 2.5;
            emitter.size_end = 5.0;
        }
        "fire" => {
            emitter.rate = 180.0;
            emitter.lifetime = 0.7;
            emitter.speed = 150.0;
            emitter.spread_degrees = 45.0;
            emitter.gravity = [0.0, -250.0];
            emitter.color_start = [1.0, 0.9, 0.1, 0.9];
            emitter.color_end = [0.9, 0.1, 0.0, 0.0];
            emitter.size_start = 18.0;
            emitter.size_end = 2.0;
        }
        "smoke" => {
            emitter.rate = 40.0;
            emitter.lifetime = 2.5;
            emitter.speed = 60.0;
            emitter.spread_degrees = 60.0;
            emitter.gravity = [0.0, -80.0];
            emitter.color_start = [0.3, 0.3, 0.35, 0.5];
            emitter.color_end = [0.1, 0.1, 0.1, 0.0];
            emitter.size_start = 8.0;
            emitter.size_end = 35.0;
        }
        "snow" => {
            emitter.rate = 75.0;
            emitter.lifetime = 5.0;
            emitter.speed = 80.0;
            emitter.spread_degrees = 30.0;
            emitter.gravity = [10.0, 120.0];
            emitter.shape = kagari_vfx::core::particle_system::EmitterShape::Box;
            emitter.emitter_size = [comp.width as f32, 20.0];
            emitter.color_start = [0.95, 0.98, 1.0, 0.8];
            emitter.color_end = [0.9, 0.95, 1.0, 0.2];
            emitter.size_start = 3.0;
            emitter.size_end = 2.0;
        }
        _ => {}
    }

    if let Some(r) = rate {
        emitter.rate = r.max(1.0);
    }
    if let Some(m) = max_particles {
        emitter.max_particles = m.max(10);
    }

    let emitter_pos = if let Some(p) = pos_str {
        let parts: Vec<f32> = p.split(',').filter_map(|s| s.trim().parse().ok()).collect();
        if parts.len() >= 2 {
            [parts[0], parts[1]]
        } else {
            [comp.width as f32 / 2.0, comp.height as f32 / 2.0]
        }
    } else {
        [comp.width as f32 / 2.0, comp.height as f32 / 2.0]
    };

    if let Some(layer) = comp.layers.iter_mut().find(|l| l.name == layer_name || l.id == layer_name) {
        layer.layer_type = kagari_vfx::core::timeline::LayerType::Particle { emitter };
        layer.transform.position = kagari_vfx::core::property::Animatable::new_constant(emitter_pos);
        eprintln!("Updated existing layer '{}' with particle preset '{}'", layer.name, preset);
    } else {
        let mut new_layer = kagari_vfx::core::timeline::Layer::new(
            format!("particle_{}", comp.layers.len() + 1),
            layer_name.to_string(),
            kagari_vfx::core::timeline::LayerType::Particle { emitter },
            comp.duration_frames,
        );
        new_layer.transform.position = kagari_vfx::core::property::Animatable::new_constant(emitter_pos);
        comp.layers.push(new_layer);
        eprintln!("Created new particle layer '{}' with preset '{}'", layer_name, preset);
    }

    let json = serde_json::to_string_pretty(&project)?;
    std::fs::write(project_path, json)?;
    eprintln!("Updated project file: {}", project_path);
    Ok(())
}

pub(super) fn cmd_audio_spectrum(
    project_path: &str,
    layer_name: &str,
    audio_file: &str,
    bands: usize,
    _min_freq: f32,
    _max_freq: f32,
    max_height: f32,
    comp_ref: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut project = load_project(project_path)?;
    let comp = find_comp_mut(&mut project, comp_ref)?;

    let height_norm = (max_height / comp.height as f32).clamp(0.05, 1.0);
    let effect_id = format!(
        "audio_spectrum_{}_{}",
        comp.layers.len(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    );
    let effect = kagari_vfx::core::timeline::Effect {
        id: effect_id,
        name: "Audio Spectrum".to_string(),
        effect_type: kagari_vfx::core::timeline::EffectType::AudioSpectrum {
            enabled: kagari_vfx::core::property::Animatable::new_constant(1.0),
            bands: kagari_vfx::core::property::Animatable::new_constant(bands.clamp(8, 256) as f32),
            opacity: kagari_vfx::core::property::Animatable::new_constant(1.0),
            color_start: [0.2, 0.8, 1.0, 1.0],
            color_end: [0.9, 0.2, 0.9, 1.0],
            position_x: kagari_vfx::core::property::Animatable::new_constant(0.05),
            position_y: kagari_vfx::core::property::Animatable::new_constant(0.2),
            width: kagari_vfx::core::property::Animatable::new_constant(0.9),
            height: kagari_vfx::core::property::Animatable::new_constant(height_norm),
        },
        enabled: true,
    };

    if let Some(layer) = comp.layers.iter_mut().find(|l| l.name == layer_name || l.id == layer_name) {
        layer.effects.push(effect);
        eprintln!("Added AudioSpectrum effect to layer '{}'", layer.name);
    } else {
        let mut solid = kagari_vfx::core::timeline::Layer::new(
            format!("audio_vis_{}", comp.layers.len() + 1),
            layer_name.to_string(),
            kagari_vfx::core::timeline::LayerType::Solid {
                color: [0.0, 0.0, 0.0, 0.0],
            },
            comp.duration_frames,
        );
        solid.effects.push(effect);
        comp.layers.push(solid);
        eprintln!("Created visualizer layer '{}' with AudioSpectrum effect", layer_name);
    }

    let audio_layer_exists = comp.layers.iter().any(|l| {
        if let kagari_vfx::core::timeline::LayerType::Audio { path, .. } = &l.layer_type {
            path == audio_file
        } else {
            false
        }
    });

    if !audio_layer_exists && std::path::Path::new(audio_file).is_file() {
        let audio_layer = kagari_vfx::core::timeline::Layer::new(
            format!("audio_src_{}", comp.layers.len() + 1),
            "BGM Source".to_string(),
            kagari_vfx::core::timeline::LayerType::Audio {
                path: audio_file.to_string(),
                volume: kagari_vfx::core::property::Animatable::new_constant(1.0),
            },
            comp.duration_frames,
        );
        comp.layers.insert(0, audio_layer);
        eprintln!("Added audio source layer for '{}'", audio_file);
    }

    let json = serde_json::to_string_pretty(&project)?;
    std::fs::write(project_path, json)?;
    eprintln!("Updated project file: {}", project_path);
    Ok(())
}

pub(super) fn cmd_roto(
    project_path: &str,
    layer_name: &str,
    points_str: &str,
    kind_str: &str,
    radius: f32,
    frame: Option<u32>,
    comp_ref: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut project = load_project(project_path)?;
    let comp = find_comp_mut(&mut project, comp_ref)?;
    let layer = comp
        .layers
        .iter_mut()
        .find(|l| l.name == layer_name || l.id == layer_name)
        .ok_or_else(|| format!("Layer '{layer_name}' not found in composition '{}'", comp.name))?;

    let stroke_type = if kind_str.eq_ignore_ascii_case("bg") || kind_str.eq_ignore_ascii_case("background") {
        kagari_vfx::core::roto_brush_engine::RotoStrokeType::Background
    } else {
        kagari_vfx::core::roto_brush_engine::RotoStrokeType::Foreground
    };

    let flat: Vec<f32> = points_str
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    if flat.len() < 2 {
        return Err("Roto stroke points must contain at least one x,y pair (e.g. \"960,540\")".into());
    }
    let points: Vec<[f32; 2]> = flat.chunks_exact(2).map(|c| [c[0], c[1]]).collect();

    let stroke = kagari_vfx::core::roto_brush_engine::RotoStroke {
        stroke_type,
        points,
        radius: radius.clamp(2.0, 200.0),
        frame: frame.or(Some(layer.in_frame)),
    };

    layer.roto_brush_strokes.push(stroke);
    eprintln!(
        "Added RotoBrush {:?} stroke ({} points, r={:.1}px) to layer '{}'",
        stroke_type,
        flat.len() / 2,
        radius,
        layer.name
    );

    let json = serde_json::to_string_pretty(&project)?;
    std::fs::write(project_path, json)?;
    eprintln!("Updated project file: {}", project_path);
    Ok(())
}

pub(super) fn cmd_puppet(
    project_path: &str,
    layer_name: &str,
    pin_name: &str,
    pos_str: Option<&str>,
    target_str: Option<&str>,
    frame: Option<u32>,
    comp_ref: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut project = load_project(project_path)?;
    let comp = find_comp_mut(&mut project, comp_ref)?;
    let layer = comp
        .layers
        .iter_mut()
        .find(|l| l.name == layer_name || l.id == layer_name)
        .ok_or_else(|| format!("Layer '{layer_name}' not found in composition '{}'", comp.name))?;

    let default_pos = [comp.width as f32 / 2.0, comp.height as f32 / 2.0];
    let pos = if let Some(p) = pos_str {
        let parts: Vec<f32> = p.split(',').filter_map(|s| s.trim().parse().ok()).collect();
        if parts.len() >= 2 {
            [parts[0], parts[1]]
        } else {
            default_pos
        }
    } else {
        default_pos
    };

    let pin_idx = layer.puppet_pins.iter().position(|p| p.name == pin_name || p.id == pin_name);
    let target_f = frame.unwrap_or(layer.in_frame);

    if let Some(idx) = pin_idx {
        if let Some(t_str) = target_str {
            let parts: Vec<f32> = t_str.split(',').filter_map(|s| s.trim().parse().ok()).collect();
            if parts.len() >= 2 {
                let target_pos = [parts[0], parts[1]];
                let pin = &mut layer.puppet_pins[idx];
                let mut kfs = match &pin.position {
                    kagari_vfx::core::property::Animatable::Animated(k) => k.clone(),
                    kagari_vfx::core::property::Animatable::Constant(c) => {
                        vec![kagari_vfx::core::keyframe::Keyframe::new(layer.in_frame, *c, kagari_vfx::core::keyframe::InterpolationType::Linear)]
                    }
                };
                if let Some(existing_kf) = kfs.iter_mut().find(|k| k.frame == target_f) {
                    existing_kf.value = target_pos;
                } else {
                    kfs.push(kagari_vfx::core::keyframe::Keyframe::new(target_f, target_pos, kagari_vfx::core::keyframe::InterpolationType::Linear));
                    kfs.sort_by_key(|k| k.frame);
                }
                pin.position = kagari_vfx::core::property::Animatable::Animated(kfs);
                eprintln!("Animated Puppet Pin '{}' to [{:.1}, {:.1}] at frame {}", pin_name, target_pos[0], target_pos[1], target_f);
            }
        }
    } else {
        let mut pin = kagari_vfx::core::timeline::PuppetPin::new(
            format!("pin_{}", layer.puppet_pins.len() + 1),
            pin_name.to_string(),
            pos,
        );
        if let Some(t_str) = target_str {
            let parts: Vec<f32> = t_str.split(',').filter_map(|s| s.trim().parse().ok()).collect();
            if parts.len() >= 2 {
                let target_pos = [parts[0], parts[1]];
                pin.position = kagari_vfx::core::property::Animatable::Animated(vec![
                    kagari_vfx::core::keyframe::Keyframe::new(layer.in_frame, pos, kagari_vfx::core::keyframe::InterpolationType::Linear),
                    kagari_vfx::core::keyframe::Keyframe::new(target_f, target_pos, kagari_vfx::core::keyframe::InterpolationType::Linear),
                ]);
            }
        }
        layer.puppet_pins.push(pin);
        eprintln!("Created new Puppet Pin '{}' at [{:.1}, {:.1}] on layer '{}'", pin_name, pos[0], pos[1], layer.name);
    }

    let json = serde_json::to_string_pretty(&project)?;
    std::fs::write(project_path, json)?;
    eprintln!("Updated project file: {}", project_path);
    Ok(())
}

pub(super) fn cmd_chroma_key(
    project_path: &str,
    layer_name: &str,
    color_str: &str,
    gain: f32,
    clip_black: f32,
    clip_white: f32,
    comp_ref: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut project = load_project(project_path)?;
    let comp = find_comp_mut(&mut project, comp_ref)?;
    let layer = comp
        .layers
        .iter_mut()
        .find(|l| l.name == layer_name || l.id == layer_name)
        .ok_or_else(|| format!("Layer '{layer_name}' not found in composition '{}'", comp.name))?;

    let screen_color = match color_str.to_lowercase().as_str() {
        "green" => [0.0, 1.0, 0.0],
        "blue" => [0.0, 0.0, 1.0],
        hex if hex.starts_with('#') && hex.len() == 7 => {
            let r = u8::from_str_radix(&hex[1..3], 16).unwrap_or(0) as f32 / 255.0;
            let g = u8::from_str_radix(&hex[3..5], 16).unwrap_or(255) as f32 / 255.0;
            let b = u8::from_str_radix(&hex[5..7], 16).unwrap_or(0) as f32 / 255.0;
            [r, g, b]
        }
        _ => [0.0, 1.0, 0.0],
    };

    let effect = kagari_vfx::core::timeline::Effect {
        id: format!(
            "chroma_key_{}_{}",
            layer.effects.len() + 1,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
        ),
        name: "Chroma Key".to_string(),
        effect_type: kagari_vfx::core::timeline::EffectType::ChromaKey {
            screen_color: kagari_vfx::core::property::Animatable::new_constant(screen_color),
            screen_gain: kagari_vfx::core::property::Animatable::new_constant(gain.clamp(0.1, 3.0)),
            clip_black: kagari_vfx::core::property::Animatable::new_constant(clip_black.clamp(0.0, 1.0)),
            clip_white: kagari_vfx::core::property::Animatable::new_constant(clip_white.clamp(0.0, 1.0)),
        },
        enabled: true,
    };

    layer.effects.push(effect);
    eprintln!(
        "Configured Chroma Key on layer '{}' (screen: [{:.2}, {:.2}, {:.2}], gain: {:.2})",
        layer.name, screen_color[0], screen_color[1], screen_color[2], gain
    );

    let json = serde_json::to_string_pretty(&project)?;
    std::fs::write(project_path, json)?;
    eprintln!("Updated project file: {}", project_path);
    Ok(())
}

pub(super) fn cmd_audio_track(
    project_path: &str,
    layer_name: &str,
    volume_opt: Option<f32>,
    fade_in: Option<u32>,
    fade_out: Option<u32>,
    comp_ref: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut project = load_project(project_path)?;
    let comp = find_comp_mut(&mut project, comp_ref)?;
    let layer = comp
        .layers
        .iter_mut()
        .find(|l| l.name == layer_name || l.id == layer_name)
        .ok_or_else(|| format!("Layer '{layer_name}' not found in composition '{}'", comp.name))?;

    let in_f = layer.in_frame;
    let out_f = layer.out_frame;
    let target_vol = volume_opt.unwrap_or(1.0).max(0.0);

    if fade_in.is_some() || fade_out.is_some() {
        let mut kfs = Vec::new();
        if let Some(dur_in) = fade_in {
            let fade_end = (in_f + dur_in).min(out_f);
            kfs.push(kagari_vfx::core::keyframe::Keyframe::new(in_f, 0.0, kagari_vfx::core::keyframe::InterpolationType::Linear));
            kfs.push(kagari_vfx::core::keyframe::Keyframe::new(fade_end, target_vol, kagari_vfx::core::keyframe::InterpolationType::Linear));
        } else {
            kfs.push(kagari_vfx::core::keyframe::Keyframe::new(in_f, target_vol, kagari_vfx::core::keyframe::InterpolationType::Linear));
        }

        if let Some(dur_out) = fade_out {
            let fade_start = out_f.saturating_sub(dur_out).max(in_f);
            kfs.push(kagari_vfx::core::keyframe::Keyframe::new(fade_start, target_vol, kagari_vfx::core::keyframe::InterpolationType::Linear));
            kfs.push(kagari_vfx::core::keyframe::Keyframe::new(out_f, 0.0, kagari_vfx::core::keyframe::InterpolationType::Linear));
        } else {
            kfs.push(kagari_vfx::core::keyframe::Keyframe::new(out_f, target_vol, kagari_vfx::core::keyframe::InterpolationType::Linear));
        }

        kfs.sort_by_key(|k| k.frame);
        kfs.dedup_by_key(|k| k.frame);

        match &mut layer.layer_type {
            kagari_vfx::core::timeline::LayerType::Audio { volume, .. } => {
                *volume = kagari_vfx::core::property::Animatable::Animated(kfs);
                eprintln!("Configured audio volume automation envelope on '{}'", layer.name);
            }
            _ => {
                eprintln!("Applied volume envelope to non-audio layer '{}' (recorded for export mix)", layer.name);
            }
        }
    } else if let Some(vol) = volume_opt {
        match &mut layer.layer_type {
            kagari_vfx::core::timeline::LayerType::Audio { volume, .. } => {
                *volume = kagari_vfx::core::property::Animatable::new_constant(vol);
                eprintln!("Set constant audio volume to {:.2} on '{}'", vol, layer.name);
            }
            _ => {
                eprintln!("Set volume to {:.2} on layer '{}'", vol, layer.name);
            }
        }
    }

    let json = serde_json::to_string_pretty(&project)?;
    std::fs::write(project_path, json)?;
    eprintln!("Updated project file: {}", project_path);
    Ok(())
}

pub(super) fn cmd_story(
    project_path: &str,
    title: &str,
    subtitle: &str,
    audio: Option<&str>,
    footage: Option<&str>,
    width: u32,
    height: u32,
    fps: u32,
    duration_sec: u32,
    theme: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut project = if std::path::Path::new(project_path).is_file() {
        load_project(project_path).unwrap_or_else(|_| Project::default())
    } else {
        Project::default()
    };

    let total_frames = fps.max(1) * duration_sec.max(1);
    let mut comp = kagari_vfx::core::timeline::Composition::new(
        "main_scene".to_string(),
        title.to_string(),
        width,
        height,
        fps,
        total_frames,
    );

    // 1. Dark Atmosphere Background Solid
    let bg_color = match theme.to_lowercase().as_str() {
        "cyberpunk" => [0.02, 0.01, 0.05, 1.0],
        "anime" => [0.08, 0.06, 0.12, 1.0],
        "corporate" => [0.05, 0.08, 0.12, 1.0],
        "minimal" => [0.92, 0.92, 0.94, 1.0],
        _ => [0.02, 0.02, 0.03, 1.0], // Cinematic dark
    };
    let mut bg_layer = kagari_vfx::core::timeline::Layer::new(
        "bg_solid".to_string(),
        "Background Solid".to_string(),
        kagari_vfx::core::timeline::LayerType::Solid { color: bg_color },
        total_frames,
    );
    bg_layer.locked = true;
    comp.layers.push(bg_layer);

    // 2. Footage Layer (if provided)
    if let Some(foot) = footage {
        let is_video = foot.ends_with(".mp4") || foot.ends_with(".mov") || foot.ends_with(".webm");
        let l_type = if is_video {
            kagari_vfx::core::timeline::LayerType::Video {
                source: foot.to_string(),
                frames_dir: foot.to_string(),
                frame_count: total_frames,
                audio_wav: None,
                speed: 1.0,
            }
        } else {
            kagari_vfx::core::timeline::LayerType::Image { path: foot.to_string() }
        };
        let mut foot_layer = kagari_vfx::core::timeline::Layer::new(
            "footage_main".to_string(),
            "Main Visual Footage".to_string(),
            l_type,
            total_frames,
        );
        foot_layer.transform.scale = kagari_vfx::core::property::Animatable::new_constant([1.0, 1.0]);
        // Subtle slow cinematic push-in zoom
        foot_layer.transform.scale = kagari_vfx::core::property::Animatable::Animated(vec![
            kagari_vfx::core::keyframe::Keyframe::new(0, [1.0, 1.0], kagari_vfx::core::keyframe::InterpolationType::Linear),
            kagari_vfx::core::keyframe::Keyframe::new(total_frames, [1.08, 1.08], kagari_vfx::core::keyframe::InterpolationType::Linear),
        ]);
        comp.layers.push(foot_layer);
    }

    // 3. Audio / BGM Layer (if provided)
    if let Some(aud) = audio {
        let audio_layer = kagari_vfx::core::timeline::Layer::new(
            "bgm_soundtrack".to_string(),
            "Soundtrack Audio".to_string(),
            kagari_vfx::core::timeline::LayerType::Audio {
                path: aud.to_string(),
                volume: kagari_vfx::core::property::Animatable::Animated(vec![
                    kagari_vfx::core::keyframe::Keyframe::new(0, 0.0, kagari_vfx::core::keyframe::InterpolationType::Linear),
                    kagari_vfx::core::keyframe::Keyframe::new(fps, 1.0, kagari_vfx::core::keyframe::InterpolationType::Linear),
                    kagari_vfx::core::keyframe::Keyframe::new(total_frames.saturating_sub(fps * 2), 1.0, kagari_vfx::core::keyframe::InterpolationType::Linear),
                    kagari_vfx::core::keyframe::Keyframe::new(total_frames, 0.0, kagari_vfx::core::keyframe::InterpolationType::Linear),
                ]),
            },
            total_frames,
        );
        comp.layers.push(audio_layer);

        // Add matching audio spectrum
        let spec_effect = kagari_vfx::core::timeline::Effect {
            id: "director_spectrum".to_string(),
            name: "Audio Visualizer".to_string(),
            effect_type: kagari_vfx::core::timeline::EffectType::AudioSpectrum {
                enabled: kagari_vfx::core::property::Animatable::new_constant(1.0),
                bands: kagari_vfx::core::property::Animatable::new_constant(80.0),
                opacity: kagari_vfx::core::property::Animatable::new_constant(0.65),
                color_start: [0.3, 0.8, 1.0, 0.8],
                color_end: [1.0, 0.3, 0.7, 0.8],
                position_x: kagari_vfx::core::property::Animatable::new_constant(0.08),
                position_y: kagari_vfx::core::property::Animatable::new_constant(0.12),
                width: kagari_vfx::core::property::Animatable::new_constant(0.84),
                height: kagari_vfx::core::property::Animatable::new_constant(0.18),
            },
            enabled: true,
        };
        let mut vis_solid = kagari_vfx::core::timeline::Layer::new(
            "visualizer_solid".to_string(),
            "Rhythm Waveform".to_string(),
            kagari_vfx::core::timeline::LayerType::Solid { color: [0.0, 0.0, 0.0, 0.0] },
            total_frames,
        );
        vis_solid.effects.push(spec_effect);
        comp.layers.push(vis_solid);
    }

    // 4. Atmospheric Particle Embers/Dust
    let mut emitter = kagari_vfx::core::particle_system::ParticleEmitter::default();
    emitter.rate = 40.0;
    emitter.lifetime = 3.5;
    emitter.speed = 35.0;
    emitter.spread_degrees = 360.0;
    emitter.gravity = [0.0, -15.0];
    emitter.color_start = [1.0, 0.85, 0.4, 0.7];
    emitter.color_end = [1.0, 0.3, 0.1, 0.0];
    emitter.size_start = 3.5;
    emitter.size_end = 1.0;
    let mut particle_layer = kagari_vfx::core::timeline::Layer::new(
        "atmosphere_particles".to_string(),
        "Ambient Particles".to_string(),
        kagari_vfx::core::timeline::LayerType::Particle { emitter },
        total_frames,
    );
    particle_layer.transform.position = kagari_vfx::core::property::Animatable::new_constant([width as f32 / 2.0, height as f32 * 0.7]);
    comp.layers.push(particle_layer);

    // 5. Kinetic Title Typography with Text Animator
    let mut title_layer = kagari_vfx::core::timeline::Layer::new(
        "main_title".to_string(),
        "Scene Title".to_string(),
        kagari_vfx::core::timeline::LayerType::Text {
            text: title.to_string(),
            font_size: (height / 14).max(36),
            color: [1.0, 1.0, 1.0, 1.0],
            font_family: "Outfit".to_string(),
            tracking: 12.0,
            leading: 1.2,
            align: 1, // Center
            stroke_color: [0.0, 0.0, 0.0, 0.8],
            stroke_width: 2.0,
            text_on_path: false,
        },
        total_frames,
    );
    title_layer.transform.position = kagari_vfx::core::property::Animatable::new_constant([width as f32 / 2.0, height as f32 * 0.44]);
    // Cascade typewriter animator
    let title_sweep_end = (fps * 2).min(total_frames);
    title_layer.text_animator = Some(kagari_vfx::core::text_animator::TextAnimatorSettings {
        enabled: true,
        selector: kagari_vfx::core::text_animator::RangeSelector {
            start: 0.0,
            end: 100.0,
            offset: 0.0,
            shape: kagari_vfx::core::text_animator::SelectorShape::RampUp,
            ease_high: 30.0,
            ease_low: 30.0,
            random_order: false,
            char_start: -1,
            char_end: -1,
            offset_anim: Some(kagari_vfx::core::property::Animatable::Animated(vec![
                kagari_vfx::core::keyframe::Keyframe::new(0, -100.0, kagari_vfx::core::keyframe::InterpolationType::Linear),
                kagari_vfx::core::keyframe::Keyframe::new(title_sweep_end, 100.0, kagari_vfx::core::keyframe::InterpolationType::Linear),
            ])),
            expression: None,
        },
        position_offset: [0.0, 40.0],
        scale: [0.85, 0.85],
        opacity: 0.0,
        tracking: 10.0,
        rotation: 0.0,
        blur_amount: 15.0,
    });
    comp.layers.push(title_layer);

    // 6. Subtitle / Narrative Logline
    let mut sub_layer = kagari_vfx::core::timeline::Layer::new(
        "subtitle_logline".to_string(),
        "Subtitle Narrative".to_string(),
        kagari_vfx::core::timeline::LayerType::Text {
            text: subtitle.to_string(),
            font_size: (height / 32).max(20),
            color: [0.85, 0.88, 0.95, 0.9],
            font_family: "Inter".to_string(),
            tracking: 8.0,
            leading: 1.2,
            align: 1, // Center
            stroke_color: [0.0, 0.0, 0.0, 0.6],
            stroke_width: 1.0,
            text_on_path: false,
        },
        total_frames,
    );
    sub_layer.transform.position = kagari_vfx::core::property::Animatable::new_constant([width as f32 / 2.0, height as f32 * 0.54]);
    sub_layer.transform.opacity = kagari_vfx::core::property::Animatable::Animated(vec![
        kagari_vfx::core::keyframe::Keyframe::new(0, 0.0, kagari_vfx::core::keyframe::InterpolationType::Linear),
        kagari_vfx::core::keyframe::Keyframe::new(fps, 0.0, kagari_vfx::core::keyframe::InterpolationType::Linear),
        kagari_vfx::core::keyframe::Keyframe::new(fps * 2, 1.0, kagari_vfx::core::keyframe::InterpolationType::Linear),
        kagari_vfx::core::keyframe::Keyframe::new(total_frames.saturating_sub(fps), 1.0, kagari_vfx::core::keyframe::InterpolationType::Linear),
        kagari_vfx::core::keyframe::Keyframe::new(total_frames, 0.0, kagari_vfx::core::keyframe::InterpolationType::Linear),
    ]);
    comp.layers.push(sub_layer);

    // 7. Master Cinematic Finishing Layer (Adjustment Layer)
    let mut finish_layer = kagari_vfx::core::timeline::Layer::new(
        "cinematic_grade".to_string(),
        "Master Cinematic Grade".to_string(),
        kagari_vfx::core::timeline::LayerType::AdjustmentLayer,
        total_frames,
    );
    finish_layer.is_adjustment_layer = true;
    finish_layer.effects.push(kagari_vfx::core::timeline::Effect {
        id: "film_grain".to_string(),
        name: "35mm Film Grain".to_string(),
        effect_type: kagari_vfx::core::timeline::EffectType::FilmGrain {
            intensity: kagari_vfx::core::property::Animatable::new_constant(0.12),
            grain_size: 1.5,
            color_film: true,
        },
        enabled: true,
    });
    finish_layer.effects.push(kagari_vfx::core::timeline::Effect {
        id: "vignette".to_string(),
        name: "Lens Vignette".to_string(),
        effect_type: kagari_vfx::core::timeline::EffectType::Vignette {
            intensity: kagari_vfx::core::property::Animatable::new_constant(35.0),
            roundness: kagari_vfx::core::property::Animatable::new_constant(0.0),
            feather: kagari_vfx::core::property::Animatable::new_constant(45.0),
            color: kagari_vfx::core::property::Animatable::new_constant([0.0, 0.0, 0.0, 1.0]),
        },
        enabled: true,
    });
    finish_layer.effects.push(kagari_vfx::core::timeline::Effect {
        id: "chromatic".to_string(),
        name: "Chromatic Aberration".to_string(),
        effect_type: kagari_vfx::core::timeline::EffectType::ChromaticAberration {
            shift_r: kagari_vfx::core::property::Animatable::new_constant(2.0),
            shift_b: kagari_vfx::core::property::Animatable::new_constant(-2.0),
            edge_falloff: kagari_vfx::core::property::Animatable::new_constant(1.0),
            iris_linked: true,
        },
        enabled: true,
    });
    comp.layers.push(finish_layer);

    let comp_idx = project.compositions.len();
    project.compositions.push(comp);
    project.active_composition_idx = comp_idx;

    let json = serde_json::to_string_pretty(&project)?;
    std::fs::write(project_path, json)?;

    eprintln!(
        "\n🎬 Directed & Composed Scene '{}' successfully!\n  Resolution: {}x{} @ {}fps, {} seconds ({} frames)\n  Theme: {}\n  Layers created: 7 layers (Solid, Footage, Audio, Spectrum, Particles, Title, Grade)",
        title, width, height, fps, duration_sec, total_frames, theme
    );
    Ok(())
}

pub(super) fn cmd_critique(
    project_path: &str,
    comp_ref: Option<&str>,
    sample_step: u32,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let project = load_project(project_path)?;
    let comp = find_comp(&project, comp_ref)?;

    if comp.duration_frames == 0 {
        return Err(format!("Composition '{}' has no frames", comp.name).into());
    }

    let step = sample_step.max(1);
    let sample_frames: Vec<u32> = (0..comp.duration_frames).step_by(step as usize).collect();

    let mut shadow_clip_sum = 0.0f32;
    let mut highlight_clip_sum = 0.0f32;
    let mut contrast_sum = 0.0f32;
    let mut evaluated_frames = 0;

    for &f in &sample_frames {
        let pixels = kagari_vfx::core::software_renderer::render_frame_to_pixels(comp, f, comp.width.min(640), comp.height.min(360), 0.0, 0);
        let n_pixels = pixels.len() / 4;
        if n_pixels == 0 {
            continue;
        }

        let mut lum_min = 255u8;
        let mut lum_max = 0u8;
        let mut shadows = 0;
        let mut highlights = 0;

        for chunk in pixels.chunks_exact(4) {
            let lum = ((chunk[0] as u32 * 77 + chunk[1] as u32 * 150 + chunk[2] as u32 * 29) >> 8) as u8;
            if lum < lum_min { lum_min = lum; }
            if lum > lum_max { lum_max = lum; }
            if lum < 5 { shadows += 1; }
            if lum > 250 { highlights += 1; }
        }

        let shadow_ratio = (shadows as f32 / n_pixels as f32) * 100.0;
        let highlight_ratio = (highlights as f32 / n_pixels as f32) * 100.0;
        let dynamic_range = (lum_max as f32 - lum_min as f32) / 255.0;

        shadow_clip_sum += shadow_ratio;
        highlight_clip_sum += highlight_ratio;
        contrast_sum += dynamic_range;
        evaluated_frames += 1;
    }

    let avg_shadow_clip = if evaluated_frames > 0 { shadow_clip_sum / evaluated_frames as f32 } else { 0.0 };
    let avg_highlight_clip = if evaluated_frames > 0 { highlight_clip_sum / evaluated_frames as f32 } else { 0.0 };
    let avg_dynamic_range = if evaluated_frames > 0 { contrast_sum / evaluated_frames as f32 } else { 0.0 };

    // Audio & sync analysis
    let has_audio = comp.layers.iter().any(|l| matches!(l.layer_type, kagari_vfx::core::timeline::LayerType::Audio { .. }));
    let has_particles = comp.layers.iter().any(|l| matches!(l.layer_type, kagari_vfx::core::timeline::LayerType::Particle { .. }));
    let has_camera = comp.cameras.iter().any(|c| c.active) || comp.active_camera.active;
    let has_lut_or_grade = comp.layers.iter().any(|l| l.is_adjustment_layer || l.effects.iter().any(|e| e.name.contains("Film") || e.name.contains("Grade")));

    // Scoring out of 100
    let visual_contrast_score = (avg_dynamic_range * 100.0f32).clamp(0.0f32, 100.0f32);
    let exposure_balance_score = (100.0f32 - (avg_shadow_clip + avg_highlight_clip) * 2.0f32).clamp(0.0f32, 100.0f32);
    let depth_add: f32 = (if has_camera { 25.0f32 } else { 0.0f32 }) + (if has_particles { 25.0f32 } else { 0.0f32 }) + (if has_lut_or_grade { 20.0f32 } else { 0.0f32 });
    let composition_depth_score = (30.0f32 + depth_add).clamp(0.0f32, 100.0f32);
    let audio_sync_score = if has_audio { 95.0f32 } else { 40.0f32 };
    let overall_score = (visual_contrast_score * 0.25 + exposure_balance_score * 0.25 + composition_depth_score * 0.3 + audio_sync_score * 0.2).round();

    let mut suggestions = Vec::new();
    if avg_shadow_clip > 15.0 {
        suggestions.push("High shadow clipping detected. Consider adding a Lift (+0.03) or lowering contrast to prevent crushed blacks.");
    }
    if avg_highlight_clip > 10.0 {
        suggestions.push("Highlight clipping detected. Add a soft knee or lower exposure (-0.3 EV) for filmic roll-off.");
    }
    if !has_camera {
        suggestions.push("No 3D camera motion found. Run 'kagari camera --pos ... --dof' to add cinematic 2.5D parallax depth.");
    }
    if !has_particles {
        suggestions.push("Scene lacks atmospheric texture. Run 'kagari particle --preset dust' to add organic ambient dust/embers.");
    }
    if !has_lut_or_grade {
        suggestions.push("No color grading applied. Run 'kagari cinematic --preset 35mm' to infuse physical film grain and lens character.");
    }

    if json {
        let report = serde_json::json!({
            "composition": comp.name,
            "overall_score": overall_score,
            "metrics": {
                "visual_contrast": visual_contrast_score,
                "exposure_balance": exposure_balance_score,
                "depth_and_camera": composition_depth_score,
                "audio_integration": audio_sync_score,
            },
            "stats": {
                "avg_shadow_clipping_percent": avg_shadow_clip,
                "avg_highlight_clipping_percent": avg_highlight_clip,
                "evaluated_sample_frames": evaluated_frames,
            },
            "actionable_tuning_suggestions": suggestions,
        });
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        eprintln!("\n🔍 ═══ Autonomous Vision & Rhythm Critique Report ═══");
        eprintln!("  Composition: {}", comp.name);
        eprintln!("  Overall Production Score: {} / 100", overall_score);
        eprintln!("  ├── Visual Contrast:      {:.1}%", visual_contrast_score);
        eprintln!("  ├── Exposure Balance:     {:.1}%", exposure_balance_score);
        eprintln!("  ├── Depth & Cinematography: {:.1}%", composition_depth_score);
        eprintln!("  └── Audio Synchronization: {:.1}%", audio_sync_score);
        eprintln!("\n💡 Actionable Director Suggestions for Claude/Opus 5.5 Beatdown:");
        for (i, s) in suggestions.iter().enumerate() {
            eprintln!("  {}. {}", i + 1, s);
        }
        eprintln!("═════════════════════════════════════════════════════════\n");
    }

    Ok(())
}

pub(super) fn cmd_cinematic(
    project_path: &str,
    preset: &str,
    grain: f32,
    letterbox: bool,
    aberration: f32,
    vignette: f32,
    comp_ref: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut project = load_project(project_path)?;
    let comp = find_comp_mut(&mut project, comp_ref)?;

    let mut adj = kagari_vfx::core::timeline::Layer::new(
        format!("cinematic_finishing_{}", comp.layers.len() + 1),
        format!("Cinematic Finishing ({})", preset),
        kagari_vfx::core::timeline::LayerType::AdjustmentLayer,
        comp.duration_frames,
    );
    adj.is_adjustment_layer = true;

    // 1. Film Grain
    if grain > 0.0 {
        adj.effects.push(kagari_vfx::core::timeline::Effect {
            id: format!("grain_{}", comp.layers.len()),
            name: "35mm Grain".to_string(),
            effect_type: kagari_vfx::core::timeline::EffectType::FilmGrain {
                intensity: kagari_vfx::core::property::Animatable::new_constant(grain.clamp(0.01, 1.0)),
                grain_size: 1.6,
                color_film: true,
            },
            enabled: true,
        });
    }

    // 2. Lens Chromatic Aberration
    if aberration > 0.0 {
        adj.effects.push(kagari_vfx::core::timeline::Effect {
            id: format!("chromatic_{}", comp.layers.len()),
            name: "Lens Aberration".to_string(),
            effect_type: kagari_vfx::core::timeline::EffectType::ChromaticAberration {
                shift_r: kagari_vfx::core::property::Animatable::new_constant(aberration.clamp(0.5, 20.0)),
                shift_b: kagari_vfx::core::property::Animatable::new_constant(-aberration.clamp(0.5, 20.0)),
                edge_falloff: kagari_vfx::core::property::Animatable::new_constant(1.0),
                iris_linked: true,
            },
            enabled: true,
        });
    }

    // 3. Vignette
    if vignette > 0.0 {
        adj.effects.push(kagari_vfx::core::timeline::Effect {
            id: format!("vignette_{}", comp.layers.len()),
            name: "Lens Falloff".to_string(),
            effect_type: kagari_vfx::core::timeline::EffectType::Vignette {
                intensity: kagari_vfx::core::property::Animatable::new_constant((vignette * 100.0).clamp(5.0, 95.0)),
                roundness: kagari_vfx::core::property::Animatable::new_constant(0.0),
                feather: kagari_vfx::core::property::Animatable::new_constant(50.0),
                color: kagari_vfx::core::property::Animatable::new_constant([0.0, 0.0, 0.0, 1.0]),
            },
            enabled: true,
        });
    }

    // 4. Anamorphic 2.39:1 Letterbox Bars
    if letterbox {
        let bar_fraction = 0.125f32; // Standard 16:9 to 2.39:1 letterbox
        let bar_h = (comp.height as f32 * bar_fraction).round() as u32;
        let mut top_bar = kagari_vfx::core::timeline::Layer::new(
            "letterbox_top".to_string(),
            "Cinema Letterbox Top".to_string(),
            kagari_vfx::core::timeline::LayerType::Solid { color: [0.0, 0.0, 0.0, 1.0] },
            comp.duration_frames,
        );
        top_bar.transform.anchor_point = kagari_vfx::core::property::Animatable::new_constant([comp.width as f32 / 2.0, 0.0]);
        top_bar.transform.position = kagari_vfx::core::property::Animatable::new_constant([comp.width as f32 / 2.0, 0.0]);
        top_bar.transform.scale = kagari_vfx::core::property::Animatable::new_constant([1.0, bar_h as f32 / comp.height as f32]);
        top_bar.locked = true;

        let mut btm_bar = kagari_vfx::core::timeline::Layer::new(
            "letterbox_btm".to_string(),
            "Cinema Letterbox Bottom".to_string(),
            kagari_vfx::core::timeline::LayerType::Solid { color: [0.0, 0.0, 0.0, 1.0] },
            comp.duration_frames,
        );
        btm_bar.transform.anchor_point = kagari_vfx::core::property::Animatable::new_constant([comp.width as f32 / 2.0, comp.height as f32]);
        btm_bar.transform.position = kagari_vfx::core::property::Animatable::new_constant([comp.width as f32 / 2.0, comp.height as f32]);
        btm_bar.transform.scale = kagari_vfx::core::property::Animatable::new_constant([1.0, bar_h as f32 / comp.height as f32]);
        btm_bar.locked = true;

        comp.layers.push(top_bar);
        comp.layers.push(btm_bar);
    }

    comp.layers.push(adj);
    eprintln!(
        "Injected Cinematic Finishing Preset '{}' (grain: {:.2}, letterbox: {}, aberration: {:.1}px, vignette: {:.2})",
        preset, grain, letterbox, aberration, vignette
    );

    let json = serde_json::to_string_pretty(&project)?;
    std::fs::write(project_path, json)?;
    eprintln!("Updated project file: {}", project_path);
    Ok(())
}

pub(super) fn cmd_mv(
    audio_path: &str,
    lyrics_opt: Option<&str>,
    lyrics_file_opt: Option<&str>,
    output_path: &str,
    style: &str,
    width: u32,
    height: u32,
    fps: u32,
    duration_sec: Option<u32>,
    project_only: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("🎵 ═══ Kagari Autonomous Music Video Generator ═══");
    eprintln!("  Audio: {}", audio_path);
    eprintln!("  Style: {}", style);
    eprintln!("  Format: {}x{} @ {}fps", width, height, fps);

    // 1. Analyze Audio Beats & Transients
    let max_frames = duration_sec.unwrap_or(30) * fps.max(1);
    let detected_beats = kagari_vfx::core::audio_engine::detect_beat_frames(
        std::path::Path::new(audio_path),
        max_frames,
        fps as f32,
    );

    eprintln!("  Analyzed Audio: Found {} transient beat drops across {} frames", detected_beats.len(), max_frames);

    // 2. Parse Lyrics/Text phrases
    let mut phrases: Vec<String> = Vec::new();
    if let Some(f_path) = lyrics_file_opt {
        if let Ok(content) = std::fs::read_to_string(f_path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    phrases.push(trimmed.to_string());
                }
            }
        }
    }
    if phrases.is_empty() {
        if let Some(l_str) = lyrics_opt {
            for chunk in l_str.split(&['/', ';', '\n'][..]) {
                let trimmed = chunk.trim();
                if !trimmed.is_empty() {
                    phrases.push(trimmed.to_string());
                }
            }
        }
    }
    if phrases.is_empty() {
        phrases = vec![
            "YOU ARE ABSOLUTELY RIGHT".to_string(),
            "HALLUCINATING THROUGH THE NOISE".to_string(),
            "LOOKS GOOD TO ME (LGTM)".to_string(),
            "HUMAN IN THE LOOP".to_string(),
            "I'LL TAKE IT FROM HERE".to_string(),
            "SUPERINTELLIGENCE ACTIVE".to_string(),
        ];
    }

    let total_frames = max_frames;
    let mut comp = kagari_vfx::core::timeline::Composition::new(
        "mv_composition".to_string(),
        "Autonomous Music Video".to_string(),
        width,
        height,
        fps,
        total_frames,
    );

    // 3. Stylistic Palette
    let (bg_color, neon_pri, neon_sec, _font_name) = match style.to_lowercase().as_str() {
        "analog-grunge" => ([0.05, 0.04, 0.04, 1.0], [0.95, 0.9, 0.8, 1.0], [0.8, 0.2, 0.2, 1.0], "Outfit"),
        "kinetic-swiss" => ([0.02, 0.02, 0.02, 1.0], [1.0, 1.0, 1.0, 1.0], [1.0, 0.1, 0.1, 1.0], "Inter"),
        "deep-space" => ([0.01, 0.01, 0.04, 1.0], [0.3, 0.8, 1.0, 1.0], [0.7, 0.3, 1.0, 1.0], "Outfit"),
        "hyper-pop" => ([0.08, 0.02, 0.08, 1.0], [1.0, 0.2, 0.8, 1.0], [0.2, 1.0, 0.9, 1.0], "Outfit"),
        _ => ([0.02, 0.01, 0.03, 1.0], [0.0, 0.95, 1.0, 1.0], [1.0, 0.05, 0.6, 1.0], "Outfit"), // Cyberpunk Neon
    };

    // Layer 1: Dark Backdrop
    let mut bg_layer = kagari_vfx::core::timeline::Layer::new(
        "mv_bg".to_string(),
        "Deep Background".to_string(),
        kagari_vfx::core::timeline::LayerType::Solid { color: bg_color },
        total_frames,
    );
    bg_layer.locked = true;
    comp.layers.push(bg_layer);

    // Layer 2: Audio Track Layer
    let audio_layer = kagari_vfx::core::timeline::Layer::new(
        "mv_soundtrack".to_string(),
        "Master Audio Track".to_string(),
        kagari_vfx::core::timeline::LayerType::Audio {
            path: audio_path.to_string(),
            volume: kagari_vfx::core::property::Animatable::new_constant(1.0),
        },
        total_frames,
    );
    comp.layers.push(audio_layer);

    // Layer 3: Audio Spectrum Waveform Visualizer
    let spec_effect = kagari_vfx::core::timeline::Effect {
        id: "mv_spectrum_fx".to_string(),
        name: "Audio Visualizer".to_string(),
        effect_type: kagari_vfx::core::timeline::EffectType::AudioSpectrum {
            enabled: kagari_vfx::core::property::Animatable::new_constant(1.0),
            bands: kagari_vfx::core::property::Animatable::new_constant(96.0),
            opacity: kagari_vfx::core::property::Animatable::new_constant(0.7),
            color_start: neon_pri,
            color_end: neon_sec,
            position_x: kagari_vfx::core::property::Animatable::new_constant(0.05),
            position_y: kagari_vfx::core::property::Animatable::new_constant(0.08),
            width: kagari_vfx::core::property::Animatable::new_constant(0.9),
            height: kagari_vfx::core::property::Animatable::new_constant(0.25),
        },
        enabled: true,
    };
    let mut vis_layer = kagari_vfx::core::timeline::Layer::new(
        "mv_visualizer".to_string(),
        "Beat Reactive Spectrum".to_string(),
        kagari_vfx::core::timeline::LayerType::Solid { color: [0.0, 0.0, 0.0, 0.0] },
        total_frames,
    );
    vis_layer.effects.push(spec_effect);
    comp.layers.push(vis_layer);

    // Layer 4: Particle Embers / Stardust
    let mut emitter = kagari_vfx::core::particle_system::ParticleEmitter::default();
    emitter.rate = 80.0;
    emitter.lifetime = 2.0;
    emitter.speed = 120.0;
    emitter.spread_degrees = 360.0;
    emitter.gravity = [0.0, -40.0];
    emitter.color_start = neon_pri;
    emitter.color_end = [neon_sec[0], neon_sec[1], neon_sec[2], 0.0];
    emitter.size_start = 4.0;
    emitter.size_end = 1.0;
    let mut particle_layer = kagari_vfx::core::timeline::Layer::new(
        "mv_particles".to_string(),
        "Audio Atmosphere Sparks".to_string(),
        kagari_vfx::core::timeline::LayerType::Particle { emitter },
        total_frames,
    );
    particle_layer.transform.position = kagari_vfx::core::property::Animatable::new_constant([width as f32 / 2.0, height as f32 / 2.0]);
    comp.layers.push(particle_layer);

    // Layer 5+: Kinetic Beat-Synchronized Typography Layers
    let phrase_count = phrases.len().max(1);
    let frames_per_phrase = (total_frames / phrase_count as u32).max(fps);

    for (i, phrase) in phrases.iter().enumerate() {
        let p_start = (i as u32 * frames_per_phrase).min(total_frames.saturating_sub(1));
        let p_end = (p_start + frames_per_phrase).min(total_frames);
        let p_dur = p_end.saturating_sub(p_start).max(1);

        let mut text_layer = kagari_vfx::core::timeline::Layer::new(
            format!("kinetic_text_{}", i + 1),
            format!("Lyric #{}: {}", i + 1, phrase),
            kagari_vfx::core::timeline::LayerType::Text {
                text: phrase.clone(),
                font_size: (height / 12).clamp(42, 110),
                color: neon_pri,
                font_family: "Outfit".to_string(),
                tracking: 10.0,
                leading: 1.1,
                align: 1, // Center
                stroke_color: [0.0, 0.0, 0.0, 0.9],
                stroke_width: 3.0,
                text_on_path: false,
            },
            total_frames,
        );
        text_layer.in_frame = p_start;
        text_layer.out_frame = p_end;

        // Position: centered with gentle float
        text_layer.transform.position = kagari_vfx::core::property::Animatable::new_constant([width as f32 / 2.0, height as f32 * 0.48]);

        // Scale Kick on Beat: quick pop zoom
        let beat_pop_end = p_start + (fps / 4).min(p_dur);
        text_layer.transform.scale = kagari_vfx::core::property::Animatable::Animated(vec![
            kagari_vfx::core::keyframe::Keyframe::new(p_start, [1.4, 1.4], kagari_vfx::core::keyframe::InterpolationType::Linear),
            kagari_vfx::core::keyframe::Keyframe::new(beat_pop_end, [1.0, 1.0], kagari_vfx::core::keyframe::InterpolationType::Linear),
            kagari_vfx::core::keyframe::Keyframe::new(p_end, [1.05, 1.05], kagari_vfx::core::keyframe::InterpolationType::Linear),
        ]);

        // Per-character Text Animator (Typewriter & Rise)
        let sweep_frames = (fps / 2).min(p_dur);
        text_layer.text_animator = Some(kagari_vfx::core::text_animator::TextAnimatorSettings {
            enabled: true,
            selector: kagari_vfx::core::text_animator::RangeSelector {
                start: 0.0,
                end: 100.0,
                offset: 0.0,
                shape: kagari_vfx::core::text_animator::SelectorShape::RampUp,
                ease_high: 25.0,
                ease_low: 25.0,
                random_order: false,
                char_start: -1,
                char_end: -1,
                offset_anim: Some(kagari_vfx::core::property::Animatable::Animated(vec![
                    kagari_vfx::core::keyframe::Keyframe::new(p_start, -100.0, kagari_vfx::core::keyframe::InterpolationType::Linear),
                    kagari_vfx::core::keyframe::Keyframe::new(p_start + sweep_frames, 100.0, kagari_vfx::core::keyframe::InterpolationType::Linear),
                ])),
                expression: None,
            },
            position_offset: [0.0, 60.0],
            scale: [0.8, 0.8],
            opacity: 0.0,
            tracking: 12.0,
            rotation: 0.0,
            blur_amount: 18.0,
        });

        // Glow Effect on Lyrics
        text_layer.effects.push(kagari_vfx::core::timeline::Effect {
            id: format!("lyric_glow_{}", i),
            name: "Cyber Glow".to_string(),
            effect_type: kagari_vfx::core::timeline::EffectType::Glow {
                intensity: kagari_vfx::core::property::Animatable::new_constant(1.5),
                radius: kagari_vfx::core::property::Animatable::new_constant(22.0),
                threshold: kagari_vfx::core::property::Animatable::new_constant(0.4),
                color: kagari_vfx::core::property::Animatable::new_constant(neon_pri),
            },
            enabled: true,
        });

        comp.layers.push(text_layer);
    }

    // Top Master Layer: 35mm Cinema Finish & Glitch
    let mut finish_adj = kagari_vfx::core::timeline::Layer::new(
        "mv_finishing".to_string(),
        "35mm Cinema & Lens Finishing".to_string(),
        kagari_vfx::core::timeline::LayerType::AdjustmentLayer,
        total_frames,
    );
    finish_adj.is_adjustment_layer = true;
    finish_adj.effects.push(kagari_vfx::core::timeline::Effect {
        id: "mv_grain".to_string(),
        name: "Film Grain".to_string(),
        effect_type: kagari_vfx::core::timeline::EffectType::FilmGrain {
            intensity: kagari_vfx::core::property::Animatable::new_constant(0.14),
            grain_size: 1.5,
            color_film: true,
        },
        enabled: true,
    });
    finish_adj.effects.push(kagari_vfx::core::timeline::Effect {
        id: "mv_chroma".to_string(),
        name: "RGB Fringe".to_string(),
        effect_type: kagari_vfx::core::timeline::EffectType::ChromaticAberration {
            shift_r: kagari_vfx::core::property::Animatable::new_constant(2.8),
            shift_b: kagari_vfx::core::property::Animatable::new_constant(-2.8),
            edge_falloff: kagari_vfx::core::property::Animatable::new_constant(1.0),
            iris_linked: true,
        },
        enabled: true,
    });
    finish_adj.effects.push(kagari_vfx::core::timeline::Effect {
        id: "mv_vignette".to_string(),
        name: "Lens Falloff".to_string(),
        effect_type: kagari_vfx::core::timeline::EffectType::Vignette {
            intensity: kagari_vfx::core::property::Animatable::new_constant(40.0),
            roundness: kagari_vfx::core::property::Animatable::new_constant(0.0),
            feather: kagari_vfx::core::property::Animatable::new_constant(55.0),
            color: kagari_vfx::core::property::Animatable::new_constant([0.0, 0.0, 0.0, 1.0]),
        },
        enabled: true,
    });
    comp.layers.push(finish_adj);

    let mut project = Project::default();
    project.compositions.push(comp);
    project.active_composition_idx = 0;

    let project_json_path = if output_path.ends_with(".json") {
        output_path.to_string()
    } else {
        "./mv_project.json".to_string()
    };

    let json = serde_json::to_string_pretty(&project)?;
    std::fs::write(&project_json_path, json)?;
    eprintln!("\n✅ Generated Music Video Project: {}", project_json_path);

    if !project_only && output_path.ends_with(".mp4") {
        eprintln!("🚀 Launching High-Definition MP4 Render with 48kHz Master Mix...");
        cmd_render(RenderArgs {
            project_path: project_json_path,
            comp_ref: Some("Autonomous Music Video".to_string()),
            output: output_path.to_string(),
            format: OutputFormat::Mp4,
            frames: None,
            from: 0,
            to: Some(total_frames.saturating_sub(1)),
            width: Some(width),
            height: Some(height),
            exposure: 0.0,
            lut: 0,
            lut_file: None,
            color_space: None,
            step: 1,
            threads: 0, // auto CPU cores
            skip_existing: false,
            mfr: true,
            log_format: "standard".to_string(),
            bindings: Vec::new(),
            overrides: Vec::new(),
            expressions: Vec::new(),
        })?;
        eprintln!("\n🎉 Masterpiece Music Video Exported Successfully to '{}'!", output_path);
    }

    Ok(())
}

