use super::edit::find_comp_mut;
use super::frame::write_png;
use super::*;

#[path = "render/effects.rs"]
mod effects;
#[path = "render/music_video.rs"]
mod music_video;
#[path = "render/operations.rs"]
mod operations;
#[path = "render/output.rs"]
mod output;
#[path = "render/queue.rs"]
mod queue;
#[path = "render/story.rs"]
mod story;
pub(super) use effects::{
    cmd_audio_spectrum, cmd_audio_track, cmd_chroma_key, cmd_particle, cmd_puppet, cmd_roto,
};
pub(super) use music_video::cmd_mv;
pub(super) use operations::{
    cmd_beats, cmd_bundle, cmd_camera, cmd_preview, cmd_remap, cmd_shape_anim, cmd_text_anim,
    cmd_track,
};
pub(super) use queue::cmd_queue;
pub(super) use story::{cmd_cinematic, cmd_critique, cmd_story};

fn existing_frame_is_complete(
    path: &std::path::Path,
    format: image::ImageFormat,
    width: u32,
    height: u32,
) -> bool {
    let Ok(reader) = image::ImageReader::open(path).and_then(|reader| reader.with_guessed_format())
    else {
        return false;
    };
    if reader.format() != Some(format) || reader.into_dimensions().ok() != Some((width, height)) {
        return false;
    }

    match format {
        image::ImageFormat::Png => existing_png_is_complete(path, width, height),
        image::ImageFormat::OpenExr => {
            kagari_vfx::core::exr_reader::read_exr(path).is_ok_and(|image| {
                image.width == width && image.height == height && image.validate().is_ok()
            })
        }
        _ => false,
    }
}

fn existing_png_is_complete(path: &std::path::Path, width: u32, height: u32) -> bool {
    const MAX_DECODE_BYTES: u64 = 512 * 1024 * 1024;
    if width == 0
        || height == 0
        || width > kagari_vfx::core::software_renderer::MAX_RENDER_DIMENSION
        || height > kagari_vfx::core::software_renderer::MAX_RENDER_DIMENSION
        || u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|pixels| pixels.checked_mul(4))
            .is_none_or(|bytes| bytes > MAX_DECODE_BYTES)
    {
        return false;
    }
    let Ok(reader) = image::ImageReader::open(path) else {
        return false;
    };
    if reader.format() != Some(image::ImageFormat::Png)
        || reader.into_dimensions().ok() != Some((width, height))
    {
        return false;
    }
    let Ok(mut reader) = image::ImageReader::open(path) else {
        return false;
    };
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(kagari_vfx::core::software_renderer::MAX_RENDER_DIMENSION);
    limits.max_image_height = Some(kagari_vfx::core::software_renderer::MAX_RENDER_DIMENSION);
    limits.max_alloc = Some(MAX_DECODE_BYTES);
    reader.limits(limits);
    reader
        .decode()
        .is_ok_and(|image| image.width() == width && image.height() == height)
}

fn save_cli_project(
    path: impl AsRef<std::path::Path>,
    project: &kagari_vfx::core::timeline::Project,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = path.as_ref();
    kagari_vfx::core::project_migration::save_project_atomic(project, path)
        .map_err(|error| format!("Failed to save project '{}': {error}", path.display()).into())
}

fn parse_finite_components(value: &str, description: &str) -> std::io::Result<Vec<f32>> {
    value
        .split(',')
        .map(|component| {
            let parsed = component.trim().parse::<f32>().map_err(|_| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("{description} contains an invalid number: '{component}'"),
                )
            })?;
            if parsed.is_finite() {
                Ok(parsed)
            } else {
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("{description} values must be finite"),
                ))
            }
        })
        .collect()
}

fn parse_finite_scalar(value: &str, description: &str) -> Result<f32, Box<dyn std::error::Error>> {
    let mut components = parse_finite_components(value, description)?;
    if components.len() != 1 {
        return Err(format!("{description} requires exactly one number").into());
    }
    Ok(components.remove(0))
}

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
            let start: u32 = start_s
                .trim()
                .parse()
                .map_err(|_| format!("Invalid frame number: '{start_s}'"))?;
            let end: u32 = end_s
                .trim()
                .parse()
                .map_err(|_| format!("Invalid frame number: '{end_s}'"))?;
            if start > end {
                return Err(format!("Invalid range: {start}-{end}"));
            }
            if end > max_frame {
                return Err(format!(
                    "Frame {end} exceeds composition duration ({max_frame})"
                ));
            }
            for f in (start..=end).step_by(step) {
                result.insert(f);
            }
        } else {
            let f: u32 = chunk
                .parse()
                .map_err(|_| format!("Invalid frame number: '{chunk}'"))?;
            if f > max_frame {
                return Err(format!(
                    "Frame {f} exceeds composition duration ({max_frame})"
                ));
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
    super::frame::ensure_output_does_not_replace_source(&args.project_path, &args.output)?;

    let json = kagari_vfx::core::project_migration::read_project_json_file(std::path::Path::new(
        &args.project_path,
    ))?;
    let (mut project, production) =
        kagari_vfx::core::project_migration::load_project_document_or_legacy(&json)?;

    // Apply template overrides (text replacement, image/video path replacement)
    for (target, val) in &args.overrides {
        let (layer_name, prop_name) = match target.split_once('.') {
            Some((l, p)) => (l.trim(), Some(p.trim())),
            None => (target.as_str(), None),
        };
        let mut matched = false;
        let mut applied = false;
        for c in &mut project.compositions {
            for layer in &mut c.layers {
                if layer.name == layer_name || layer.id == layer_name {
                    matched = true;
                    match (&mut layer.layer_type, prop_name) {
                        (
                            kagari_vfx::core::timeline::LayerType::Text { text, .. },
                            None | Some("text"),
                        ) => {
                            *text = val.clone();
                            applied = true;
                            eprintln!("Applied override: [{}].text = \"{}\"", layer.name, val);
                        }
                        (
                            kagari_vfx::core::timeline::LayerType::Image { path },
                            None | Some("src" | "path"),
                        ) => {
                            *path = val.clone();
                            applied = true;
                            eprintln!("Applied override: [{}].path = \"{}\"", layer.name, val);
                        }
                        (
                            kagari_vfx::core::timeline::LayerType::Video { source, .. },
                            None | Some("src" | "source"),
                        ) => {
                            *source = val.clone();
                            applied = true;
                            eprintln!("Applied override: [{}].source = \"{}\"", layer.name, val);
                        }
                        (kagari_vfx::core::timeline::LayerType::Solid { color }, Some("color")) => {
                            let parts = if let Some(hex) = val.strip_prefix('#') {
                                if !matches!(hex.len(), 6 | 8) {
                                    return Err(
                                        "Solid color hex override must be #RRGGBB or #RRGGBBAA"
                                            .into(),
                                    );
                                }
                                let mut channels = Vec::with_capacity(hex.len() / 2);
                                for pair in hex.as_bytes().as_chunks::<2>().0 {
                                    let pair = std::str::from_utf8(pair)?;
                                    let value = u8::from_str_radix(pair, 16).map_err(|_| {
                                        std::io::Error::new(
                                            std::io::ErrorKind::InvalidInput,
                                            format!("invalid solid color hex override '{val}'"),
                                        )
                                    })?;
                                    channels.push(value as f32 / 255.0);
                                }
                                channels
                            } else {
                                parse_finite_components(val, "Solid color override")?
                            };
                            if !matches!(parts.len(), 3 | 4) {
                                return Err("Solid color override requires 3 or 4 channels".into());
                            }
                            color[..parts.len()].copy_from_slice(&parts);
                            applied = true;
                            eprintln!("Applied override: [{}].color = {:?}", layer.name, color);
                        }
                        _ => {}
                    }

                    // Direct transform overrides (e.g. Logo.opacity=50 or Text.rotation=45)
                    if let Some(prop) = prop_name {
                        match prop.to_lowercase().as_str() {
                            "opacity" | "alpha" => {
                                let v = parse_finite_scalar(val, "Opacity override")?;
                                layer.transform.opacity =
                                    kagari_vfx::core::timeline::Animatable::new_constant(v);
                                applied = true;
                                eprintln!("Applied override: [{}].opacity = {}", layer.name, v);
                            }
                            "rotation" | "rot" => {
                                let v = parse_finite_scalar(val, "Rotation override")?;
                                layer.transform.rotation =
                                    kagari_vfx::core::timeline::Animatable::new_constant(v);
                                applied = true;
                                eprintln!("Applied override: [{}].rotation = {}", layer.name, v);
                            }
                            "scale" => {
                                let v = parse_finite_scalar(val, "Scale override")?;
                                layer.transform.scale =
                                    kagari_vfx::core::timeline::Animatable::new_constant([v, v]);
                                applied = true;
                                eprintln!(
                                    "Applied override: [{}].scale = [{}, {}]",
                                    layer.name, v, v
                                );
                            }
                            "visible" => {
                                let v = val.parse::<bool>().map_err(|_| {
                                    std::io::Error::new(
                                        std::io::ErrorKind::InvalidInput,
                                        "Visible override must be true or false",
                                    )
                                })?;
                                layer.visible = v;
                                applied = true;
                                eprintln!("Applied override: [{}].visible = {}", layer.name, v);
                            }
                            _ => {}
                        }
                    }

                    // Effect overrides (e.g. Logo.GaussianBlur.radius=15.0 or Background.Glow.intensity=2.0)
                    if let Some(sub) = prop_name {
                        if let Some((eff_name, eff_param)) = sub.split_once('.') {
                            for effect in &mut layer.effects {
                                if effect.name.eq_ignore_ascii_case(eff_name)
                                    || format!("{:?}", effect.effect_type)
                                        .to_lowercase()
                                        .starts_with(&eff_name.to_lowercase())
                                {
                                    match (
                                        &mut effect.effect_type,
                                        eff_param.to_lowercase().as_str(),
                                    ) {
                                        (
                                            kagari_vfx::core::timeline::EffectType::GaussianBlur {
                                                blur_radius,
                                            },
                                            "radius" | "blur_radius",
                                        ) => {
                                            let v =
                                                parse_finite_scalar(val, "Blur radius override")?;
                                            *blur_radius = kagari_vfx::core::timeline::Animatable::new_constant(v);
                                            applied = true;
                                            eprintln!(
                                                "Applied override: [{}.{}].{} = {}",
                                                layer.name, effect.name, eff_param, v
                                            );
                                        }
                                        (
                                            kagari_vfx::core::timeline::EffectType::Glow {
                                                intensity,
                                                ..
                                            },
                                            "intensity",
                                        ) => {
                                            let v = parse_finite_scalar(
                                                val,
                                                "Glow intensity override",
                                            )?;
                                            *intensity = kagari_vfx::core::timeline::Animatable::new_constant(v);
                                            applied = true;
                                            eprintln!(
                                                "Applied override: [{}.{}].intensity = {}",
                                                layer.name, effect.name, v
                                            );
                                        }
                                        (
                                            kagari_vfx::core::timeline::EffectType::Glow {
                                                radius,
                                                ..
                                            },
                                            "radius",
                                        ) => {
                                            let v =
                                                parse_finite_scalar(val, "Glow radius override")?;
                                            *radius = kagari_vfx::core::timeline::Animatable::new_constant(v);
                                            applied = true;
                                            eprintln!(
                                                "Applied override: [{}.{}].radius = {}",
                                                layer.name, effect.name, v
                                            );
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
            eprintln!(
                "Warning: Template override target '{}' not found in any composition",
                target
            );
        } else if !applied {
            return Err(format!(
                "Override target '{target}' matched a layer but no supported property was applied"
            )
            .into());
        }
    }

    // Apply dynamic expressions (e.g. Logo.rotation=wiggle(3.0, 25.0) or Camera.position=wiggle(2.0, 50.0))
    for (target, expr_str) in &args.expressions {
        let (layer_name, prop_name) = match target.split_once('.') {
            Some((l, p)) => (l.trim(), p.trim().to_lowercase()),
            None => (target.as_str(), "rotation".to_string()),
        };
        let expr = if let Some(w) = expr_str
            .strip_prefix("wiggle(")
            .and_then(|s| s.strip_suffix(')'))
        {
            let parts = parse_finite_components(w, "Wiggle expression")?;
            if parts.len() != 2 {
                return Err(
                    "Wiggle expression requires exactly two values: wiggle(frequency,amplitude)"
                        .into(),
                );
            }
            kagari_vfx::core::timeline::Expression::Wiggle {
                frequency: parts[0],
                amplitude: parts[1],
            }
        } else if expr_str.starts_with("wiggle(") {
            return Err("Malformed wiggle expression; use wiggle(frequency,amplitude)".into());
        } else if expr_str.eq_ignore_ascii_case("loopout")
            || expr_str.eq_ignore_ascii_case("loopout()")
        {
            kagari_vfx::core::timeline::Expression::LoopOut
        } else if expr_str.eq_ignore_ascii_case("pingpong")
            || expr_str.eq_ignore_ascii_case("pingpong()")
        {
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
                            eprintln!(
                                "Injected expression: [{}].rotation = {:?}",
                                layer.name, expr
                            );
                        }
                        "position" | "pos" | "p" => {
                            layer.transform.position_expression = Some(expr.clone());
                            eprintln!(
                                "Injected expression: [{}].position = {:?}",
                                layer.name, expr
                            );
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
                            eprintln!(
                                "Injected expression: [{}].anchor_point = {:?}",
                                layer.name, expr
                            );
                        }
                        other => {
                            eprintln!(
                                "Warning: Unsupported property '{}' for expression on layer '{}'",
                                other, layer.name
                            );
                        }
                    }
                }
            }
        }
        if !matched {
            eprintln!(
                "Warning: Expression target layer '{}' not found in any composition",
                layer_name
            );
        }
    }

    if let Some(cs_str) = &args.color_space {
        let cs = match cs_str.to_lowercase().replace(['-', '_', '.'], "").as_str() {
            "rec709" | "srgb" => kagari_vfx::core::color_science::WorkingColorSpace::Rec709,
            "displayp3" | "p3" => kagari_vfx::core::color_science::WorkingColorSpace::DisplayP3,
            "rec2020" | "bt2020" => kagari_vfx::core::color_science::WorkingColorSpace::Rec2020,
            "linear" | "linearsrgb" => {
                kagari_vfx::core::color_science::WorkingColorSpace::LinearSRGB
            }
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
    eprintln!(
        "Multi-Frame Rendering (MFR): {}",
        if args.mfr { "ENABLED" } else { "DISABLED" }
    );

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
            return Err(
                format!("Invalid render range: from {} is after to {to}", args.from).into(),
            );
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
    if args.format == OutputFormat::Exr
        && kagari_vfx::core::software_renderer::exr_rgba_working_set_size(width, height).is_none()
    {
        return Err(format!(
            "EXR export at {width}x{height} exceeds the {} MiB float working-memory limit; reduce the output dimensions",
            kagari_vfx::core::software_renderer::MAX_FLOAT_RENDER_WORKING_BYTES / (1024 * 1024)
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
        eprintln!(
            "  Frames: {} selected (range {}..={})",
            spec.frames.len(),
            spec.from,
            spec.to
        );
        eprintln!("  Format: {:?}", format);
    } else {
        println!(
            "{}",
            serde_json::json!({"event":"start", "composition":comp.name, "width":spec.w, "height":spec.h, "total_frames":spec.frames.len()})
        );
    }

    match format {
        OutputFormat::Mp4 => output::render_to_mp4(comp, &spec, production.as_ref(), comp_index)?,
        OutputFormat::Gif => output::render_to_gif(comp, &spec, production.as_ref(), comp_index)?,
        OutputFormat::Png => {
            if let Some(document) = production.as_ref() {
                output::render_to_png_sequence_with_bindings(document, comp_index, &spec)?;
            } else {
                output::render_to_png_sequence(comp, &spec)?;
            }
        }
        OutputFormat::Exr => {
            output::render_to_exr_sequence(comp, &spec)?;
        }
    }

    Ok(())
}

pub(super) struct QueueArgs {
    pub(super) project_path: String,
    pub(super) output: String,
    pub(super) format: OutputFormat,
    pub(super) threads: usize,
    pub(super) log_format: String,
}
