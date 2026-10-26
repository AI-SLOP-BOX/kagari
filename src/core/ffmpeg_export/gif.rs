use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;

use super::types::{
    drain_stderr, is_ffmpeg_available, render_with_cancel, temporary_output_path,
    validate_export_config, ChildGuard, ExportConfig, ExportEvent, TemporaryOutputGuard,
};

/// Start an asynchronous FFmpeg GIF export job with a two-pass palette approach.
///
/// Pass 1: Generate an optimal 256-color palette from all frames.
/// Pass 2: Encode the GIF using that palette with sierra2_4a dithering.
pub fn start_gif_export<F>(
    config: ExportConfig,
    tx: Sender<ExportEvent>,
    cancel_flag: Arc<AtomicBool>,
    render_frame_fn: F,
) -> Result<(), String>
where
    F: Fn(u32) -> Vec<u8> + Send + 'static,
{
    if let Err(error) = validate_export_config(&config, true) {
        let _ = tx.send(ExportEvent::Error(error.clone()));
        return Err(error);
    }
    if !is_ffmpeg_available() {
        let msg = "FFmpeg not found. Install it via `brew install ffmpeg` (macOS) or your package manager.".to_string();
        let _ = tx.send(ExportEvent::Error(msg.clone()));
        return Err(msg);
    }

    if config.output_path.starts_with('-') {
        let msg = format!(
            "output_path must not start with '-': {}",
            config.output_path
        );
        let _ = tx.send(ExportEvent::Error(msg.clone()));
        return Err(msg);
    }

    let config_clone = config.clone();

    std::thread::Builder::new()
        .name("ffmpeg_gif_export".to_string())
        .spawn(move || {
            let config = config_clone;
            let temporary_output = temporary_output_path(&config.output_path);
            let mut output_guard = TemporaryOutputGuard::new(&temporary_output);
            let palette_path = format!("{}.palette.png", temporary_output);

            let Some(frame_bytes) =
                crate::core::software_renderer::rgba_buffer_size(config.width, config.height)
            else {
                let _ = tx.send(ExportEvent::Error(format!(
                    "Invalid dimensions: {}x{}",
                    config.width, config.height
                )));
                return;
            };

            // ── Pass 1: Generate palette ──────────────────────────────────
            let _ = tx.send(ExportEvent::Progress(
                0.0,
                "Generating GIF palette...".to_string(),
            ));

            let palette_result = Command::new("ffmpeg")
                .args([
                    "-y",
                    "-f",
                    "rawvideo",
                    "-pix_fmt",
                    "rgba",
                    "-s",
                    &format!("{}x{}", config.width, config.height),
                    "-r",
                    &config.fps.to_string(),
                    "-i",
                    "pipe:0",
                    "-vf",
                    &format!(
                        "fps={},palettegen=max_colors=256:stats_mode=diff",
                        config.fps
                    ),
                    &palette_path,
                ])
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn();

            let palette_child = match palette_result {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(ExportEvent::Error(format!(
                        "Failed to launch FFmpeg (palette pass): {}",
                        e
                    )));
                    return;
                }
            };

            let (palette_child, palette_stderr_handle) = drain_stderr(palette_child);
            let mut palette_guard = ChildGuard::new(palette_child);

            let mut palette_stdin = match palette_guard.child.as_mut().and_then(|c| c.stdin.take())
            {
                Some(s) => s,
                None => {
                    let _ = tx.send(ExportEvent::Error(
                        "FFmpeg stdin pipe was not opened (palette pass)".to_string(),
                    ));
                    return;
                }
            };

            for frame_idx in 0..config.total_frames {
                if cancel_flag.load(Ordering::SeqCst) {
                    let _ = tx.send(ExportEvent::Error("Export canceled by user".to_string()));
                    return;
                }

                let pixels = match render_with_cancel(&cancel_flag, &render_frame_fn, frame_idx) {
                    Ok(pixels) => pixels,
                    Err(()) => {
                        let _ = tx.send(ExportEvent::Error(format!(
                            "Render callback failed during palette pass at frame {}",
                            frame_idx
                        )));
                        let _ = std::fs::remove_file(&palette_path);
                        return;
                    }
                };
                if pixels.len() != frame_bytes {
                    let _ = tx.send(ExportEvent::Error(format!(
                        "Frame {} pixel data mismatch: expected {} bytes, got {}",
                        frame_idx,
                        frame_bytes,
                        pixels.len()
                    )));
                    return;
                }
                if let Err(e) = palette_stdin.write_all(&pixels) {
                    let _ = tx.send(ExportEvent::Error(format!(
                        "Pipe write error (palette pass) at frame {}: {}",
                        frame_idx, e
                    )));
                    return;
                }

                let progress = (frame_idx + 1) as f32 / config.total_frames as f32 * 0.5;
                let _ = tx.send(ExportEvent::Progress(
                    progress,
                    format!(
                        "Palette pass: frame {}/{}",
                        frame_idx + 1,
                        config.total_frames
                    ),
                ));
            }

            drop(palette_stdin);

            let mut palette_child = palette_guard
                .take()
                .expect("palette child must be in guard");
            let stderr_output = palette_stderr_handle.join().unwrap_or_default();
            match palette_child.wait() {
                Ok(status) if status.success() => {}
                Ok(status) => {
                    let _ = tx.send(ExportEvent::Error(format!(
                        "FFmpeg palette pass exited with code {:?}\n{}",
                        status.code(),
                        &stderr_output[..stderr_output.len().min(500)]
                    )));
                    let _ = std::fs::remove_file(&palette_path);
                    return;
                }
                Err(e) => {
                    let _ = tx.send(ExportEvent::Error(format!(
                        "FFmpeg palette pass wait() error: {}",
                        e
                    )));
                    let _ = std::fs::remove_file(&palette_path);
                    return;
                }
            }

            // ── Pass 2: Encode GIF using palette ──────────────────────────
            let _ = tx.send(ExportEvent::Progress(0.5, "Encoding GIF...".to_string()));

            let gif_result = Command::new("ffmpeg")
                .args([
                    "-y",
                    "-f",
                    "rawvideo",
                    "-pix_fmt",
                    "rgba",
                    "-s",
                    &format!("{}x{}", config.width, config.height),
                    "-r",
                    &config.fps.to_string(),
                    "-i",
                    "pipe:0",
                    "-i",
                    &palette_path,
                    "-lavfi",
                    "paletteuse=dither=sierra2_4a",
                    "-loop",
                    "0",
                    &temporary_output,
                ])
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn();

            let gif_child = match gif_result {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(ExportEvent::Error(format!(
                        "Failed to launch FFmpeg (gif pass): {}",
                        e
                    )));
                    let _ = std::fs::remove_file(&palette_path);
                    return;
                }
            };

            let (gif_child, gif_stderr_handle) = drain_stderr(gif_child);
            let mut gif_guard = ChildGuard::new(gif_child);

            let mut gif_stdin = match gif_guard.child.as_mut().and_then(|c| c.stdin.take()) {
                Some(s) => s,
                None => {
                    let _ = tx.send(ExportEvent::Error(
                        "FFmpeg stdin pipe was not opened (gif pass)".to_string(),
                    ));
                    let _ = std::fs::remove_file(&palette_path);
                    return;
                }
            };

            for frame_idx in 0..config.total_frames {
                if cancel_flag.load(Ordering::SeqCst) {
                    let _ = tx.send(ExportEvent::Error("Export canceled by user".to_string()));
                    let _ = std::fs::remove_file(&palette_path);
                    return;
                }

                let pixels = match render_with_cancel(&cancel_flag, &render_frame_fn, frame_idx) {
                    Ok(pixels) => pixels,
                    Err(()) => {
                        let _ = tx.send(ExportEvent::Error(format!(
                            "Render callback failed during GIF pass at frame {}",
                            frame_idx
                        )));
                        let _ = std::fs::remove_file(&palette_path);
                        return;
                    }
                };
                if pixels.len() != frame_bytes {
                    let _ = tx.send(ExportEvent::Error(format!(
                        "Frame {} pixel data mismatch: expected {} bytes, got {}",
                        frame_idx,
                        frame_bytes,
                        pixels.len()
                    )));
                    let _ = std::fs::remove_file(&palette_path);
                    return;
                }
                if let Err(e) = gif_stdin.write_all(&pixels) {
                    let _ = tx.send(ExportEvent::Error(format!(
                        "Pipe write error (gif pass) at frame {}: {}",
                        frame_idx, e
                    )));
                    let _ = std::fs::remove_file(&palette_path);
                    return;
                }

                let progress = 0.5 + (frame_idx + 1) as f32 / config.total_frames as f32 * 0.5;
                let _ = tx.send(ExportEvent::Progress(
                    progress,
                    format!("GIF pass: frame {}/{}", frame_idx + 1, config.total_frames),
                ));
            }

            drop(gif_stdin);
            let mut gif_child = gif_guard.take().expect("gif child must be in guard");
            let stderr_output = gif_stderr_handle.join().unwrap_or_default();
            match gif_child.wait() {
                Ok(status) if status.success() => {
                    if let Err(error) = std::fs::rename(&temporary_output, &config.output_path) {
                        let _ = tx.send(ExportEvent::Error(format!(
                            "Failed to finalize GIF output: {}",
                            error
                        )));
                        let _ = std::fs::remove_file(&palette_path);
                        return;
                    }
                    output_guard.persist();
                    let _ = std::fs::remove_file(&palette_path);
                    let _ = tx.send(ExportEvent::Finished(format!(
                        "GIF export complete → {}",
                        config.output_path
                    )));
                }
                Ok(status) => {
                    let _ = tx.send(ExportEvent::Error(format!(
                        "FFmpeg gif pass exited with code {:?}\n{}",
                        status.code(),
                        &stderr_output[..stderr_output.len().min(500)]
                    )));
                    let _ = std::fs::remove_file(&palette_path);
                }
                Err(e) => {
                    let _ = tx.send(ExportEvent::Error(format!(
                        "FFmpeg gif pass wait() error: {}",
                        e
                    )));
                    let _ = std::fs::remove_file(&palette_path);
                }
            }
        })
        .map_err(|e| format!("Failed to spawn GIF export thread: {}", e))?;

    Ok(())
}
