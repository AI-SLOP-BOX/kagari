use super::*;

pub(super) fn render_to_png_sequence_with_bindings(
    document: &kagari_vfx::core::production_document::ProductionDocument,
    composition_index: usize,
    spec: &RenderSpec,
) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(spec.output)?;
    let total = spec.frames.len();
    for (idx, &frame) in spec.frames.iter().enumerate() {
        let path = format!("{}/frame_{:05}.png", spec.output, frame);
        if spec.skip_existing
            && existing_frame_is_complete(
                std::path::Path::new(&path),
                image::ImageFormat::Png,
                spec.w,
                spec.h,
            )
        {
            if spec.log_json {
                println!(
                    "{}",
                    serde_json::json!({"event":"skip", "frame":frame, "path":path})
                );
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
                "{}",
                serde_json::json!({"event":"frame_done", "frame":frame, "index":progress, "total":total, "duration_ms":elapsed_ms, "path":path})
            );
        } else if progress % 10 == 0 || progress == total {
            eprint!("\r  Rendered {}/{} frames", progress, total);
        }
    }
    if spec.log_json {
        println!(
            "{}",
            serde_json::json!({"event":"completed", "output":spec.output})
        );
    } else {
        eprintln!("\n  Done! Output: {}/", spec.output);
    }
    Ok(())
}

pub(super) fn render_to_exr_sequence(
    comp: &Composition,
    spec: &RenderSpec,
) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(spec.output)?;
    let total = spec.frames.len();
    for (idx, &frame) in spec.frames.iter().enumerate() {
        let path = format!("{}/frame_{:05}.exr", spec.output, frame);
        if spec.skip_existing
            && existing_frame_is_complete(
                std::path::Path::new(&path),
                image::ImageFormat::OpenExr,
                spec.w,
                spec.h,
            )
        {
            if spec.log_json {
                println!(
                    "{}",
                    serde_json::json!({"event":"skip", "frame":frame, "path":path})
                );
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
        let image = kagari_vfx::core::exr_writer::ExrImage::rgba_f32(spec.w, spec.h, &flat)?;
        kagari_vfx::core::exr_writer::write_exr_file(&path, &image)
            .map_err(|e| format!("EXR write error: {e}"))?;

        let elapsed_ms = t0.elapsed().as_secs_f64() * 1000.0;
        let progress = idx + 1;
        if spec.log_json {
            println!(
                "{}",
                serde_json::json!({"event":"frame_done", "frame":frame, "index":progress, "total":total, "duration_ms":elapsed_ms, "path":path})
            );
        } else if progress % 10 == 0 || progress == total {
            eprint!("\r  Rendered {}/{} frames", progress, total);
        }
    }
    if spec.log_json {
        println!(
            "{}",
            serde_json::json!({"event":"completed", "output":spec.output})
        );
    } else {
        eprintln!("\n  Done! Output: {}/", spec.output);
    }
    Ok(())
}

pub(super) fn render_to_png_sequence(
    comp: &Composition,
    spec: &RenderSpec,
) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(spec.output)?;
    let total = spec.frames.len();
    for (idx, &frame) in spec.frames.iter().enumerate() {
        let path = format!("{}/frame_{:05}.png", spec.output, frame);
        if spec.skip_existing
            && existing_frame_is_complete(
                std::path::Path::new(&path),
                image::ImageFormat::Png,
                spec.w,
                spec.h,
            )
        {
            if spec.log_json {
                println!(
                    "{}",
                    serde_json::json!({"event":"skip", "frame":frame, "path":path})
                );
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
                "{}",
                serde_json::json!({"event":"frame_done", "frame":frame, "index":progress, "total":total, "duration_ms":elapsed_ms, "path":path})
            );
        } else if progress % 10 == 0 || progress == total {
            eprint!("\r  Rendered {}/{} frames", progress, total);
        }
    }
    if spec.log_json {
        println!(
            "{}",
            serde_json::json!({"event":"completed", "output":spec.output})
        );
    } else {
        eprintln!("\n  Done! Output: {}/", spec.output);
    }
    Ok(())
}

/// Render a single frame as JPEG with quality setting.
#[allow(dead_code)]
pub(super) fn write_jpeg(
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

pub(super) fn render_to_mp4(
    comp: &Composition,
    spec: &RenderSpec,
    production: Option<&kagari_vfx::core::production_document::ProductionDocument>,
    composition_index: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    use kagari_vfx::core::ffmpeg_export::{
        is_ffmpeg_available, start_export_cancelable, ExportConfig,
    };
    use std::sync::{atomic::AtomicBool, Arc};

    validate_video_frame_selection(spec)?;

    if !is_ffmpeg_available() {
        return Err("ffmpeg is not installed or not in PATH".into());
    }

    let cancel_flag = Arc::new(AtomicBool::new(false));
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
                return Err(
                    format!("Audio mix failed; refusing to export a silent video: {e}").into(),
                );
            }
        }
    };

    let config = ExportConfig {
        output_path: spec.output.to_string(),
        width: spec.w,
        height: spec.h,
        fps: comp.fps,
        total_frames: spec.to.saturating_sub(spec.from) + 1,
        quality: 85,
        audio_bitrate_kbps: 192,
        audio_wav: audio_wav.clone(),
        codec: kagari_vfx::core::ffmpeg_export::VideoCodec::H264,
    };

    let (from, _to, w, h, exposure, lut) =
        (spec.from, spec.to, spec.w, spec.h, spec.exposure, spec.lut);
    let comp_clone = comp.clone();
    let production_clone = production.cloned();
    let binding_values = spec.bindings.clone();
    let worker_result = start_export_cancelable(config, tx, cancel_flag, move |frame_idx| {
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

    // Clean up temporary mixed WAV file
    if let Some(ref path) = audio_wav {
        let _ = std::fs::remove_file(path);
    }

    if let Some(error) = event_error {
        return Err(error.into());
    }
    worker_result.map_err(|error| error.into())
}

pub(super) fn render_to_gif(
    comp: &Composition,
    spec: &RenderSpec,
    production: Option<&kagari_vfx::core::production_document::ProductionDocument>,
    composition_index: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    use kagari_vfx::core::ffmpeg_export::{is_ffmpeg_available, start_gif_export, ExportConfig};
    use std::sync::{atomic::AtomicBool, Arc};

    validate_video_frame_selection(spec)?;

    if !is_ffmpeg_available() {
        return Err("ffmpeg is not installed or not in PATH".into());
    }

    let cancel_flag = Arc::new(AtomicBool::new(false));
    let (tx, rx) = std::sync::mpsc::channel();

    let config = ExportConfig {
        output_path: spec.output.to_string(),
        width: spec.w,
        height: spec.h,
        fps: comp.fps,
        total_frames: spec.to.saturating_sub(spec.from) + 1,
        quality: 100,
        audio_bitrate_kbps: 192,
        audio_wav: None,
        codec: kagari_vfx::core::ffmpeg_export::VideoCodec::Gif,
    };

    let (from, w, h, exposure, lut) = (spec.from, spec.w, spec.h, spec.exposure, spec.lut);
    let comp_clone = comp.clone();
    let production_clone = production.cloned();
    let binding_values = spec.bindings.clone();
    let worker_result = start_gif_export(config, tx, cancel_flag, move |frame_idx| {
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

    if let Some(error) = event_error {
        return Err(error.into());
    }
    worker_result.map_err(|error| error.into())
}

pub(super) fn validate_video_frame_selection(spec: &RenderSpec<'_>) -> Result<(), String> {
    let expected_count = spec.to.saturating_sub(spec.from).saturating_add(1) as usize;
    let is_contiguous = spec.frames.len() == expected_count
        && spec
            .frames
            .windows(2)
            .all(|frames| frames[0].checked_add(1) == Some(frames[1]));
    if is_contiguous {
        Ok(())
    } else {
        Err("MP4/GIF export requires a contiguous frame range; use PNG or EXR output for stepped or sparse frame selections".to_owned())
    }
}
