use super::*;

pub(super) fn cmd_frame(
    project_path: &str,
    frame: u32,
    output: &str,
    width: Option<u32>,
    height: Option<u32>,
    gpu: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    ensure_output_does_not_replace_source(project_path, output)?;

    #[cfg(not(feature = "cli-gpu"))]
    if gpu {
        return Err(
            "--gpu requires the optional cli-gpu feature; rebuild with --features cli-gpu".into(),
        );
    }

    let meta = std::fs::metadata(project_path)
        .map_err(|e| format!("Failed to stat project file '{}': {}", project_path, e))?;
    if meta.len() > 100 * 1024 * 1024 {
        return Err(format!(
            "Project file too large ({} MB, limit 100 MB)",
            meta.len() / (1024 * 1024)
        )
        .into());
    }
    let json = kagari_vfx::core::project_migration::read_project_json_file(std::path::Path::new(
        project_path,
    ))
    .map_err(|e| format!("Failed to read project file '{}': {}", project_path, e))?;
    let (project, document) =
        kagari_vfx::core::project_migration::load_project_document_or_legacy(&json)?;

    let comp_idx = project
        .active_composition_idx
        .min(project.compositions.len().saturating_sub(1));
    let comp = if let Some(doc) = document.as_ref() {
        doc.composition_for_frame(comp_idx, frame)
            .ok_or("Composition index out of range")?
    } else {
        project
            .compositions
            .get(comp_idx)
            .cloned()
            .ok_or("Project has no compositions")?
    };

    let w = width.unwrap_or(comp.width);
    let h = height.unwrap_or(comp.height);

    #[cfg(feature = "cli-gpu")]
    if gpu && (w != comp.width || h != comp.height) {
        return Err("--gpu currently renders at the composition's native resolution".into());
    }

    eprintln!(
        "Rendering frame {} from \"{}\" ({}x{})",
        frame, comp.name, w, h
    );

    if output.to_lowercase().ends_with(".exr") {
        let f32_pixels = kagari_vfx::core::software_renderer::render_frame_to_pixels_f32(
            &comp, frame, w, h, 0.0, 0,
        );
        let mut flat = Vec::with_capacity(f32_pixels.len() * 4);
        for px in f32_pixels {
            flat.extend_from_slice(&px);
        }
        let image = kagari_vfx::core::exr_writer::ExrImage::rgba_f32(w, h, &flat)?;
        kagari_vfx::core::exr_writer::write_exr_file(output, &image)
            .map_err(|e| format!("EXR write error: {e}"))?;
    } else {
        #[cfg(feature = "cli-gpu")]
        let pixels = if gpu {
            if kagari_vfx::core::render_capability::software_preview_required(&comp, frame) {
                eprintln!("Composition uses effects or features unsupported by GPU; falling back to software renderer");
                render_frame_to_pixels(&comp, frame, w, h, 0.0, 0)
            } else {
                eprintln!("Using the experimental WGPU preview renderer");
                kagari_vfx::core::headless_gpu::HeadlessGpuRenderer::new()?
                    .render_rgba8(&comp, frame, 0.0, 0)?
            }
        } else {
            render_frame_to_pixels(&comp, frame, w, h, 0.0, 0)
        };
        #[cfg(not(feature = "cli-gpu"))]
        let pixels = render_frame_to_pixels(&comp, frame, w, h, 0.0, 0);
        write_png(output, &pixels, w, h)?;
    }

    eprintln!("Saved: {}", output);
    Ok(())
}

pub(super) fn cmd_lottie(
    project_path: &str,
    output: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    ensure_output_does_not_replace_source(project_path, output)?;
    let project = load_project(project_path)?;
    let json = kagari_vfx::core::lottie_exporter::export_project_to_json(&project)?;

    let effect_count: usize = project
        .compositions
        .iter()
        .flat_map(|c| c.layers.iter())
        .map(|l| l.effects.iter().filter(|e| e.enabled).count())
        .sum();

    write_text_atomically(output, &json)?;
    eprintln!("Lottie exported → {} ({} bytes)", output, json.len());
    if effect_count > 0 {
        eprintln!(
            "Warning: {} enabled effect(s) are NOT part of Lottie output (format limitation)",
            effect_count
        );
    }
    Ok(())
}

pub(super) fn cmd_otio(
    project_path: &str,
    comp_ref: Option<&str>,
    output: &str,
    import: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut project = load_project(project_path)?;

    if import {
        let json = std::fs::read_to_string(output)?;
        let otio = kagari_vfx::core::integration::OtioTimeline::from_json(&json)?;
        let imported_comp = otio.to_composition()?;
        let comp_name = imported_comp.name.clone();
        project.compositions.push(imported_comp);
        project.active_composition_idx = project.compositions.len() - 1;
        kagari_vfx::core::project_migration::save_project_atomic(&project, project_path)
            .map_err(|error| format!("Failed to save project after OTIO import: {error}"))?;
        eprintln!(
            "Imported OTIO timeline '{}' from {} into {} (active comp index: {})",
            comp_name, output, project_path, project.active_composition_idx
        );
        return Ok(());
    }

    let comp = if let Some(ref_name) = comp_ref {
        if let Ok(idx) = ref_name.parse::<usize>() {
            project
                .compositions
                .get(idx)
                .cloned()
                .ok_or_else(|| format!("Composition index {idx} not found"))?
        } else {
            project
                .compositions
                .iter()
                .find(|c| c.name == ref_name || c.id == ref_name)
                .cloned()
                .ok_or_else(|| format!("Composition '{ref_name}' not found"))?
        }
    } else {
        let idx = project
            .active_composition_idx
            .min(project.compositions.len().saturating_sub(1));
        project
            .compositions
            .get(idx)
            .cloned()
            .ok_or("No compositions in project")?
    };

    let otio = kagari_vfx::core::integration::OtioTimeline::from_composition(&comp);
    let json = otio.to_json()?;
    ensure_output_does_not_replace_source(project_path, output)?;
    write_text_atomically(output, &json)?;
    eprintln!(
        "OpenTimelineIO exported: \"{}\" → {} ({} bytes)",
        comp.name,
        output,
        json.len()
    );
    Ok(())
}

pub(super) fn ensure_output_does_not_replace_source(
    source: &str,
    output: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    ensure_output_does_not_replace_sources(output, &[std::path::Path::new(source).canonicalize()?])
}

pub(super) fn ensure_output_does_not_replace_sources(
    output: &str,
    sources: &[std::path::PathBuf],
) -> Result<(), Box<dyn std::error::Error>> {
    let output_path = std::path::Path::new(output);
    let output_identity = if output_path.exists() {
        output_path.canonicalize()?
    } else {
        let file_name = output_path
            .file_name()
            .ok_or("output path has no filename")?;
        output_path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| std::path::Path::new("."))
            .canonicalize()?
            .join(file_name)
    };
    if sources.iter().any(|source| source == &output_identity) {
        return Err("output path resolves to an input file; choose a different file".into());
    }
    Ok(())
}

fn write_text_atomically(path: &str, text: &str) -> Result<(), Box<dyn std::error::Error>> {
    write_bytes_atomically(path, text.as_bytes())
}

pub(super) fn write_bytes_atomically(
    path: &str,
    bytes: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let target = std::path::Path::new(path);
    kagari_vfx::core::atomic_file::write_atomic(target, bytes)?;
    Ok(())
}

pub(super) fn write_png(
    path: &str,
    rgba: &[u8],
    width: u32,
    height: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    let expected_bytes = kagari_vfx::core::software_renderer::rgba_buffer_size(width, height)
        .ok_or("invalid PNG dimensions")?;
    if rgba.len() != expected_bytes {
        return Err(format!(
            "PNG pixel data mismatch: expected {expected_bytes} bytes, got {}",
            rgba.len()
        )
        .into());
    }
    let target = std::path::Path::new(path);
    let file_name = target
        .file_name()
        .ok_or("PNG output path has no filename")?;
    let parent = target.parent().unwrap_or_else(|| std::path::Path::new("."));
    let temporary = parent.join(format!(
        ".{}.kagari-cli-tmp-{}-{}",
        file_name.to_string_lossy(),
        std::process::id(),
        TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));

    let mut owns_temporary = false;
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options.open(&temporary)?;
        owns_temporary = true;
        let mut encoder = png::Encoder::new(file, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(rgba)?;
        writer.finish()?;
        let staged = std::fs::OpenOptions::new().write(true).open(&temporary)?;
        #[cfg(unix)]
        match std::fs::metadata(target) {
            Ok(metadata) => staged.set_permissions(metadata.permissions())?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        staged.sync_all()?;
        kagari_vfx::core::atomic_file::replace_file(&temporary, target)?;
        owns_temporary = false;
        if let Ok(directory) = std::fs::File::open(parent) {
            let _ = directory.sync_all();
        }
        Ok(())
    })();

    if result.is_err() && owns_temporary {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}
