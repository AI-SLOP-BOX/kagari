use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::Ordering;
use std::sync::mpsc::Sender;
use std::sync::Arc;

use rayon::prelude::*;

use super::types::{
    drain_stderr, is_ffmpeg_available, temporary_output_path, validate_export_config, ChildGuard,
    ExportConfig, ExportEvent, TemporaryOutputGuard, VideoCodec,
};

pub fn start_export_cancelable<F>(
    config: ExportConfig,
    tx: Sender<ExportEvent>,
    cancel_flag: Arc<std::sync::atomic::AtomicBool>,
    render_frame_fn: F,
) -> Result<(), String>
where
    F: Fn(u32) -> Vec<u8> + Send + Sync + 'static,
{
    if let Err(error) = validate_export_config(&config, false) {
        let _ = tx.send(ExportEvent::Error(error.clone()));
        return Err(error);
    }
    if !is_ffmpeg_available() {
        let msg = "FFmpeg not found. Install it via `brew install ffmpeg` (macOS) or your package manager.".to_string();
        let _ = tx.send(ExportEvent::Error(msg.clone()));
        return Err(msg);
    }

    let config_clone = config.clone();
    std::thread::Builder::new()
        .name("ffmpeg_export".to_string())
        .spawn(move || {
            let config = config_clone;
            let temporary_output = temporary_output_path(&config.output_path);
            let mut output_guard = TemporaryOutputGuard::new(&temporary_output);

            let mut cmd = Command::new("ffmpeg");
            cmd.arg("-y");
            if let Some(wav) = &config.audio_wav {
                cmd.args(["-i", wav]);
            }
            cmd.args([
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
            ]);

            match &config.codec {
                VideoCodec::H264 => {
                    cmd.args([
                        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
                    ]);
                }
                VideoCodec::ProRes422 => {
                    cmd.args([
                        "-c:v",
                        "prores_ks",
                        "-profile:v",
                        "3",
                        "-pix_fmt",
                        "yuv422p10le",
                    ]);
                }
                VideoCodec::ProRes4444 => {
                    cmd.args([
                        "-c:v",
                        "prores_ks",
                        "-profile:v",
                        "4",
                        "-pix_fmt",
                        "yuva444p10le",
                    ]);
                }
                VideoCodec::H265 => {
                    cmd.args([
                        "-c:v", "libx265", "-preset", "medium", "-crf", "23", "-pix_fmt", "yuv420p",
                    ]);
                }
                VideoCodec::Gif => {}
                VideoCodec::WebP => {
                    cmd.args([
                        "-c:v",
                        "libwebp",
                        "-quality",
                        "80",
                        "-lossless",
                        "0",
                        "-pix_fmt",
                        "rgba",
                    ]);
                }
            }
            if config.audio_wav.is_some() {
                cmd.args([
                    "-map",
                    "0:a",
                    "-map",
                    "1:v",
                    "-c:a",
                    "aac",
                    "-b:a",
                    "192k",
                    "-shortest",
                ]);
            }
            if matches!(
                PathBuf::from(&config.output_path)
                    .extension()
                    .and_then(|value| value.to_str())
                    .map(str::to_ascii_lowercase)
                    .as_deref(),
                Some("mp4" | "mov" | "m4v" | "ismv")
            ) {
                cmd.arg("-movflags").arg("+faststart");
            }
            cmd.arg(&temporary_output);

            let ffmpeg_result = cmd
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn();

            let mut child = match ffmpeg_result {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(ExportEvent::Error(format!(
                        "Failed to launch FFmpeg: {}",
                        e
                    )));
                    return;
                }
            };

            let (child_with_stderr, stderr_handle) = drain_stderr(child);
            child = child_with_stderr;
            let mut guard = ChildGuard::new(child);

            let mut stdin = match guard.child.as_mut().and_then(|c| c.stdin.take()) {
                Some(s) => s,
                None => {
                    let _ = tx.send(ExportEvent::Error(
                        "FFmpeg stdin pipe was not opened".to_string(),
                    ));
                    return;
                }
            };

            let Some(frame_bytes) =
                crate::core::software_renderer::rgba_buffer_size(config.width, config.height)
            else {
                let _ = tx.send(ExportEvent::Error(format!(
                    "Invalid dimensions: {}x{}",
                    config.width, config.height
                )));
                return;
            };

            let chunk_size = (384 * 1024 * 1024 / frame_bytes.max(1) as u64).clamp(2, 16) as u32;
            let mut next_frame: u32 = 0;
            while next_frame < config.total_frames {
                if cancel_flag.load(Ordering::SeqCst) {
                    log::info!("[FFmpegExport] export canceled by user — terminating process");
                    let _ = tx.send(ExportEvent::Error("Export canceled by user".to_string()));
                    return;
                }

                let chunk_end = (next_frame + chunk_size).min(config.total_frames);
                let rendered_frames: Vec<Result<(u32, Vec<u8>), u32>> = (next_frame..chunk_end)
                    .into_par_iter()
                    .map(|frame_idx| {
                        let flag = cancel_flag.clone();
                        crate::core::software_renderer::set_render_cancel_flag(Some(flag));
                        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            render_frame_fn(frame_idx)
                        }));
                        crate::core::software_renderer::set_render_cancel_flag(None);
                        result
                            .map(|pixels| (frame_idx, pixels))
                            .map_err(|_| frame_idx)
                    })
                    .collect();
                let mut frames = Vec::with_capacity(rendered_frames.len());
                for rendered in rendered_frames {
                    match rendered {
                        Ok(frame) => frames.push(frame),
                        Err(frame_idx) => {
                            let _ = tx.send(ExportEvent::Error(format!(
                                "Render callback failed at frame {}",
                                frame_idx
                            )));
                            return;
                        }
                    }
                }
                frames.sort_by_key(|(frame_idx, _)| *frame_idx);

                for (frame_idx, pixels) in frames {
                    if pixels.len() != frame_bytes {
                        let _ = tx.send(ExportEvent::Error(format!(
                            "Frame {} pixel data mismatch: expected {} bytes, got {}",
                            frame_idx,
                            frame_bytes,
                            pixels.len()
                        )));
                        return;
                    }

                    if let Err(e) = stdin.write_all(&pixels) {
                        let _ = tx.send(ExportEvent::Error(format!(
                            "Pipe write error at frame {}: {}",
                            frame_idx, e
                        )));
                        return;
                    }

                    let progress = (frame_idx + 1) as f32 / config.total_frames as f32;
                    let _ = tx.send(ExportEvent::Progress(
                        progress,
                        format!("Encoding frame {}/{}", frame_idx + 1, config.total_frames),
                    ));
                }
                next_frame = chunk_end;
            }

            drop(stdin);

            let mut child = guard.take().expect("child must still be in guard");
            let stderr_output = stderr_handle.join().unwrap_or_default();
            match child.wait() {
                Ok(status) if status.success() => {
                    if let Err(error) = std::fs::rename(&temporary_output, &config.output_path) {
                        let _ = tx.send(ExportEvent::Error(format!(
                            "Failed to finalize export output: {}",
                            error
                        )));
                        return;
                    }
                    output_guard.persist();
                    let _ = tx.send(ExportEvent::Finished(format!(
                        "Export complete → {}",
                        config.output_path
                    )));
                }
                Ok(status) => {
                    let _ = tx.send(ExportEvent::Error(format!(
                        "FFmpeg exited with code {:?}\n{}",
                        status.code(),
                        &stderr_output[..stderr_output.len().min(500)]
                    )));
                }
                Err(e) => {
                    let _ = tx.send(ExportEvent::Error(format!("FFmpeg wait() error: {}", e)));
                }
            }
        })
        .map_err(|e| format!("Failed to spawn export thread: {}", e))?;

    Ok(())
}
