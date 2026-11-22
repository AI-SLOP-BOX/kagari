use std::io::Write;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::Ordering;
use std::sync::mpsc::Sender;
use std::sync::Arc;

use rayon::prelude::*;

use crate::core::external_tools::ffmpeg_command;

use super::types::{
    create_temporary_output_path, drain_stderr, export_error_or_cancellation,
    publish_export_output_cancelable, snapshot_overwrite_identity,
    terminate_child_and_collect_stderr, truncate_diagnostic, validate_export_config,
    wait_for_child_or_cancel, ChildGuard, ExportConfig, ExportEvent, TemporaryOutputGuard,
    VideoCodec,
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
    start_export_cancelable_with_overwrite_tracked(config, tx, cancel_flag, true, render_frame_fn)
        .map(|_| ())
}

pub fn start_export_cancelable_with_overwrite<F>(
    config: ExportConfig,
    tx: Sender<ExportEvent>,
    cancel_flag: Arc<std::sync::atomic::AtomicBool>,
    overwrite_existing: bool,
    render_frame_fn: F,
) -> Result<(), String>
where
    F: Fn(u32) -> Vec<u8> + Send + Sync + 'static,
{
    start_export_cancelable_with_overwrite_tracked(
        config,
        tx,
        cancel_flag,
        overwrite_existing,
        render_frame_fn,
    )
    .map(|_| ())
}

pub fn start_export_cancelable_with_overwrite_tracked<F>(
    config: ExportConfig,
    tx: Sender<ExportEvent>,
    cancel_flag: Arc<std::sync::atomic::AtomicBool>,
    overwrite_existing: bool,
    render_frame_fn: F,
) -> Result<std::thread::JoinHandle<()>, String>
where
    F: Fn(u32) -> Vec<u8> + Send + Sync + 'static,
{
    let overwrite_identity = match snapshot_overwrite_identity(
        std::path::Path::new(&config.output_path),
        overwrite_existing,
    ) {
        Ok(identity) => identity,
        Err(error) => {
            let message = format!("Cannot verify existing export output: {error}");
            let _ = tx.send(ExportEvent::Error(message.clone()));
            return Err(message);
        }
    };
    start_export_cancelable_with_expected_identity_tracked(
        config,
        tx,
        cancel_flag,
        overwrite_identity,
        render_frame_fn,
    )
}

pub fn start_export_cancelable_with_expected_identity_tracked<F>(
    config: ExportConfig,
    tx: Sender<ExportEvent>,
    cancel_flag: Arc<std::sync::atomic::AtomicBool>,
    overwrite_identity: Option<super::types::FileIdentity>,
    render_frame_fn: F,
) -> Result<std::thread::JoinHandle<()>, String>
where
    F: Fn(u32) -> Vec<u8> + Send + Sync + 'static,
{
    if let Err(error) = validate_export_config(&config, false) {
        let _ = tx.send(ExportEvent::Error(error.clone()));
        return Err(error);
    }
    let config_clone = config.clone();
    std::thread::Builder::new()
        .name("ffmpeg_export".to_string())
        .spawn(move || {
            let config = config_clone;
            let temporary_output = match create_temporary_output_path(&config.output_path) {
                Ok(path) => path,
                Err(error) => {
                    let _ = tx.send(ExportEvent::Error(error));
                    return;
                }
            };
            let mut output_guard = TemporaryOutputGuard::new(&temporary_output);
            if let Err(error) = output_guard.restrict_while_writing() {
                let _ = tx.send(ExportEvent::Error(format!(
                    "Failed to secure temporary export output: {error}"
                )));
                return;
            }

            let mut cmd = ffmpeg_command();
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
                    let crf = 51u8 - ((u16::from(config.quality) * 39 + 50) / 100) as u8;
                    cmd.args(["-c:v", "libx264", "-preset", "fast", "-crf"])
                        .arg(crf.to_string())
                        .args(["-pix_fmt", "yuv420p"]);
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
                    let crf = 51u8 - ((u16::from(config.quality) * 33 + 50) / 100) as u8;
                    cmd.args(["-c:v", "libx265", "-preset", "medium", "-crf"])
                        .arg(crf.to_string())
                        .args(["-pix_fmt", "yuv420p"]);
                }
                VideoCodec::Gif => {}
                VideoCodec::WebP => {
                    cmd.args(["-c:v", "libwebp", "-quality"])
                        .arg(config.quality.to_string())
                        .args(["-lossless", "0", "-pix_fmt", "rgba"]);
                }
            }
            if config.audio_wav.is_some() {
                let audio_bitrate = format!("{}k", config.audio_bitrate_kbps);
                cmd.args(["-map", "0:a", "-map", "1:v", "-c:a", "aac", "-b:a"])
                    .arg(audio_bitrate)
                    .arg("-shortest");
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

            let (child_with_stderr, stderr_handle) = match drain_stderr(child) {
                Ok(result) => result,
                Err(error) => {
                    let _ = tx.send(ExportEvent::Error(format!(
                        "Failed to start FFmpeg stderr reader: {error}"
                    )));
                    return;
                }
            };
            child = child_with_stderr;
            let mut guard = ChildGuard::new(child);

            let mut stdin = match guard.child.as_mut().and_then(|c| c.stdin.take()) {
                Some(s) => s,
                None => {
                    let _ = terminate_child_and_collect_stderr(&mut guard, stderr_handle);
                    let _ = tx.send(ExportEvent::Error(
                        "FFmpeg stdin pipe was not opened".to_string(),
                    ));
                    return;
                }
            };

            let Some(frame_bytes) =
                crate::core::software_renderer::rgba_buffer_size(config.width, config.height)
            else {
                let _ = terminate_child_and_collect_stderr(&mut guard, stderr_handle);
                let _ = tx.send(ExportEvent::Error(format!(
                    "Invalid dimensions: {}x{}",
                    config.width, config.height
                )));
                return;
            };

            let chunk_size = (384 * 1024 * 1024 / frame_bytes.max(1) as u64).clamp(1, 16) as u32;
            let mut next_frame: u32 = 0;
            while next_frame < config.total_frames {
                if cancel_flag.load(Ordering::SeqCst) {
                    log::info!("[FFmpegExport] export canceled by user — terminating process");
                    let _ = terminate_child_and_collect_stderr(&mut guard, stderr_handle);
                    let _ = tx.send(ExportEvent::Error("Export canceled by user".to_string()));
                    return;
                }

                let chunk_end = next_frame
                    .saturating_add(chunk_size)
                    .min(config.total_frames);
                let render_one = |frame_idx: u32| {
                    let flag = cancel_flag.clone();
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        crate::core::software_renderer::with_render_cancel_flag(flag, || {
                            render_frame_fn(frame_idx)
                        })
                    }));
                    result
                        .map(|pixels| (frame_idx, pixels))
                        .map_err(|_| frame_idx)
                };
                // MFR toggle: parallel frame rendering when enabled, sequential
                // (single-threaded) when disabled. Pixels are identical either way.
                let rendered_frames: Vec<Result<(u32, Vec<u8>), u32>> =
                    if crate::core::parallel_render::mfr_enabled() {
                        (next_frame..chunk_end)
                            .into_par_iter()
                            .map(&render_one)
                            .collect()
                    } else {
                        (next_frame..chunk_end)
                            .into_iter()
                            .map(&render_one)
                            .collect()
                    };
                let mut frames = Vec::with_capacity(rendered_frames.len());
                for rendered in rendered_frames {
                    match rendered {
                        Ok(frame) => frames.push(frame),
                        Err(frame_idx) => {
                            let _ = terminate_child_and_collect_stderr(&mut guard, stderr_handle);
                            let _ = tx.send(ExportEvent::Error(export_error_or_cancellation(
                                &cancel_flag,
                                format!("Render callback failed at frame {}", frame_idx),
                            )));
                            return;
                        }
                    }
                }
                frames.sort_by_key(|(frame_idx, _)| *frame_idx);

                for (frame_idx, pixels) in frames {
                    if pixels.len() != frame_bytes {
                        let _ = terminate_child_and_collect_stderr(&mut guard, stderr_handle);
                        let _ = tx.send(ExportEvent::Error(export_error_or_cancellation(
                            &cancel_flag,
                            format!(
                                "Frame {} pixel data mismatch: expected {} bytes, got {}",
                                frame_idx,
                                frame_bytes,
                                pixels.len()
                            ),
                        )));
                        return;
                    }

                    if let Err(e) = stdin.write_all(&pixels) {
                        drop(stdin);
                        let stderr_output =
                            terminate_child_and_collect_stderr(&mut guard, stderr_handle);
                        let diagnostic = truncate_diagnostic(&stderr_output, 500);
                        let detail = if diagnostic.is_empty() {
                            String::new()
                        } else {
                            format!("\n{}", diagnostic)
                        };
                        let _ = tx.send(ExportEvent::Error(export_error_or_cancellation(
                            &cancel_flag,
                            format!("Pipe write error at frame {}: {}{}", frame_idx, e, detail),
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

            let wait_result = wait_for_child_or_cancel(
                guard.child.as_mut().expect("child must still be in guard"),
                &cancel_flag,
            );
            let stderr_output = stderr_handle.join().unwrap_or_default();
            let mut child = guard.take().expect("child must still be in guard");
            let _ = child.wait();
            match wait_result {
                Ok(Some(status)) if status.success() => {
                    let finalize_result =
                        output_guard
                            .restore_permissions_for_publish()
                            .and_then(|()| {
                                publish_export_output_cancelable(
                                    std::path::Path::new(&temporary_output),
                                    std::path::Path::new(&config.output_path),
                                    overwrite_identity,
                                    &cancel_flag,
                                )
                            });
                    if let Err(error) = finalize_result {
                        let message = if cancel_flag.load(Ordering::SeqCst) {
                            "Export canceled by user".to_owned()
                        } else {
                            format!("Failed to finalize export output: {error}")
                        };
                        let _ = tx.send(ExportEvent::Error(message));
                        return;
                    }
                    if overwrite_identity.is_some() {
                        output_guard.persist();
                    }
                    let _ = tx.send(ExportEvent::Finished(format!(
                        "Export complete → {}",
                        config.output_path
                    )));
                }
                Ok(Some(status)) => {
                    let _ = tx.send(ExportEvent::Error(format!(
                        "FFmpeg exited with code {:?}\n{}",
                        status.code(),
                        truncate_diagnostic(&stderr_output, 500)
                    )));
                }
                Ok(None) => {
                    let _ = tx.send(ExportEvent::Error("Export canceled by user".to_string()));
                }
                Err(e) => {
                    let _ = tx.send(ExportEvent::Error(format!("FFmpeg wait() error: {}", e)));
                }
            }
        })
        .map_err(|e| format!("Failed to spawn export thread: {}", e))
}
