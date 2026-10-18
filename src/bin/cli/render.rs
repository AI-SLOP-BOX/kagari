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
