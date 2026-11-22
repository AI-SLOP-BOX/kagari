//! Video import via FFmpeg.
//!
//! Videos are decoded once at import time into a PNG frame sequence plus an
//! optional WAV audio track under the project's media directory. Rendering then
//! samples the sequence like any other image source — no video decoder needed
//! at runtime or export time.
//!
//! This mirrors how NLEs proxy media: one decode pass up front, cheap random
//! access afterwards.

use std::io;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::core::external_tools::{ffmpeg_command, ffprobe_command};

const MAX_IMPORT_FPS: f32 = 240.0;
const MAX_IMPORT_DURATION_SECONDS: f64 = 24.0 * 60.0 * 60.0;
const MAX_IMPORT_FRAMES: u64 = 2_000_000;
const MAX_IMPORT_DIMENSION: u32 = 16_384;
const MAX_IMPORT_DECODED_BYTES: u64 = 128 * 1024 * 1024;
const MIN_IMPORT_FREE_BYTES: u64 = 512 * 1024 * 1024;
const IMPORT_DISK_HEADROOM_NUMERATOR: u64 = 15;
const IMPORT_DISK_HEADROOM_DENOMINATOR: u64 = 100;
const MAX_FFMPEG_DIAGNOSTIC_BYTES: usize = 64 * 1024;
const FRAME_CACHE_MANIFEST_FILE: &str = ".kagari-frame-cache-v1.json";

mod frame_cache;
#[cfg(test)]
use frame_cache::video_frame_cache_is_complete;
pub use frame_cache::{frame_path, frame_path_in_dir, refresh_missing_video_caches};
use frame_cache::{write_image_sequence_frame_cache_manifest, write_video_frame_cache_manifest};
mod image_sequence;
mod process;
pub use image_sequence::{import_image_sequence, import_image_sequence_with_cancel};
pub(crate) use process::{available_disk_space, estimate_import_disk_bytes};
use process::{
    create_private_import_directory, decode_import_image, dimensions_within_import_budget,
    format_disk_bytes, run_cancellable_probe, run_ffmpeg, ImportDirectoryGuard, ImportSafety,
};

static FRAME_CACHE_MANIFEST_SEQUENCE: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);

pub(crate) fn is_image_sequence_cache(frames_dir: &str, source: &str) -> bool {
    frame_cache::is_image_sequence_cache(frames_dir, source)
}

/// Returns the sequence decode rate, stamping old manifests with the current
/// composition rate so subsequent FPS changes retain the original timebase.
pub fn extraction_fps(frames_dir: &str, fallback_fps: u32) -> f32 {
    frame_cache::extraction_fps(frames_dir, fallback_fps)
}

/// A decoded video asset on disk.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VideoAsset {
    /// Original imported file path.
    pub source_path: String,
    /// Directory containing frame_%05d.png files.
    pub frames_dir: String,
    /// Number of extracted frames.
    pub frame_count: u32,
    /// Frames per second the sequence was extracted at.
    pub fps: f32,
    pub width: u32,
    pub height: u32,
    /// Extracted audio WAV path, if the source had an audio stream.
    pub audio_wav: Option<String>,
    /// Why a detected source audio track could not be extracted, if applicable.
    #[serde(default)]
    pub audio_warning: Option<String>,
}

/// True if ffmpeg is discoverable on PATH or a supported macOS install path.
pub fn ffmpeg_available() -> bool {
    ffmpeg_command()
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn probe_duration_seconds(path: &Path, cancel: &AtomicBool) -> Result<f64, String> {
    let out = run_cancellable_probe(
        ffprobe_command()
            .args([
                "-v",
                "error",
                "-show_entries",
                "format=duration",
                "-of",
                "default=noprint_wrappers=1:nokey=1",
                "--",
            ])
            .arg(path),
        cancel,
    )
    .map_err(|error| format!("could not inspect video duration: {error}"))?;
    if !out.status.success() {
        return Err("could not determine video duration".to_owned());
    }
    String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse::<f64>()
        .map_err(|_| "could not determine video duration".to_owned())
}

fn probe_dimensions(path: &Path, cancel: &AtomicBool) -> Result<(u32, u32), String> {
    let out = run_cancellable_probe(
        ffprobe_command()
            .args([
                "-v",
                "error",
                "-select_streams",
                "v:0",
                "-show_entries",
                "stream=width,height",
                "-of",
                "csv=s=x:p=0",
                "--",
            ])
            .arg(path),
        cancel,
    )
    .map_err(|error| format!("could not inspect video dimensions: {error}"))?;
    if !out.status.success() {
        return Err("could not determine video dimensions".to_owned());
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let mut it = s.split('x');
    let w = it
        .next()
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| "could not determine video dimensions".to_owned())?;
    let h = it
        .next()
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| "could not determine video dimensions".to_owned())?;
    Ok((w, h))
}

fn probe_has_audio(path: &Path, cancel: &AtomicBool) -> Result<bool, String> {
    let output = run_cancellable_probe(
        ffprobe_command()
            .args([
                "-v",
                "error",
                "-select_streams",
                "a:0",
                "-show_entries",
                "stream=codec_type",
                "-of",
                "csv=p=0",
                "--",
            ])
            .arg(path),
        cancel,
    )
    .map_err(|error| format!("could not inspect source audio streams: {error}"))?;
    if !output.status.success() {
        return Err("could not inspect source audio streams".to_owned());
    }
    Ok(String::from_utf8_lossy(&output.stdout).contains("audio"))
}

fn audio_fits_decode_limit(duration_seconds: f64) -> bool {
    let audio_frames = (duration_seconds * 44_100.0).ceil();
    let decoded_bytes = audio_frames * 2.0 * std::mem::size_of::<f32>() as f64;
    decoded_bytes <= crate::core::audio_engine::MAX_DECODED_AUDIO_BYTES as f64
}

fn discard_partial_audio(path: &Path) -> Option<String> {
    match std::fs::remove_file(path) {
        Ok(()) => None,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => Some(format!("; partial audio could not be removed: {error}")),
    }
}

fn create_unique_import_dir(dest_dir: &Path) -> Result<PathBuf, String> {
    let parent = dest_dir
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)
        .map_err(|e| format!("failed to create media parent dir: {}", e))?;

    let stem = dest_dir
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("import");
    let mut candidate = dest_dir.to_path_buf();
    let mut suffix = 1u32;
    loop {
        match create_private_import_directory(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                candidate = parent.join(format!("{stem}_{suffix}"));
                suffix = suffix.saturating_add(1);
            }
            Err(error) => {
                return Err(format!("failed to create media dir: {}", error));
            }
        }
    }
}

/// Decodes `src_path` into `dest_dir` as a WebP frame sequence (+ WAV audio).
/// PNG is used as a compatibility fallback when the installed FFmpeg lacks
/// the WebP encoder.
///
/// `fps` controls extraction rate (use the composition's fps so 1 sequence
/// frame == 1 composition frame).
pub fn import_video(src_path: &str, dest_dir: &Path, fps: f32) -> Result<VideoAsset, String> {
    import_video_with_cancel(src_path, dest_dir, fps, &AtomicBool::new(false))
}

pub fn import_video_with_cancel(
    src_path: &str,
    dest_dir: &Path,
    fps: f32,
    cancel: &AtomicBool,
) -> Result<VideoAsset, String> {
    if cancel.load(Ordering::Relaxed) {
        return Err("video import cancelled".into());
    }
    let ffmpeg_status = run_cancellable_probe(ffmpeg_command().arg("-version"), cancel)
        .map_err(|error| format!("could not check FFmpeg availability: {error}"))?;
    if !ffmpeg_status.status.success() {
        return Err("ffmpeg not found on PATH — install it to import video".into());
    }
    let src = Path::new(src_path);
    if !src.is_file() {
        return Err(format!("source file not found: {}", src_path));
    }
    if !fps.is_finite() || fps < 1.0 || fps > MAX_IMPORT_FPS {
        return Err(format!(
            "fps must be finite and between 1 and {}",
            MAX_IMPORT_FPS
        ));
    }
    let duration = probe_duration_seconds(src, cancel)?;
    if !duration.is_finite() || duration <= 0.0 || duration > MAX_IMPORT_DURATION_SECONDS {
        return Err(format!(
            "video duration must be between 0 and {} hours",
            MAX_IMPORT_DURATION_SECONDS / 3600.0
        ));
    }
    let (source_width, source_height) = probe_dimensions(src, cancel)?;
    if !dimensions_within_import_budget(source_width, source_height) {
        return Err(format!(
            "video dimensions exceed the import limit (maximum side {} px and decoded RGBA size {} MiB)",
            MAX_IMPORT_DIMENSION,
            MAX_IMPORT_DECODED_BYTES / (1024 * 1024)
        ));
    }
    let estimated_frames = (duration * f64::from(fps)).ceil() as u64;
    if estimated_frames > MAX_IMPORT_FRAMES {
        return Err(format!(
            "video would produce too many frames (limit {})",
            MAX_IMPORT_FRAMES
        ));
    }
    let has_audio = probe_has_audio(src, cancel)?;
    let extract_audio = has_audio && audio_fits_decode_limit(duration);
    let estimated_disk_bytes = estimate_import_disk_bytes(
        source_width,
        source_height,
        estimated_frames,
        duration,
        extract_audio,
    )
    .ok_or_else(|| "could not estimate video import disk usage".to_string())?;
    let minimum_free_bytes = MIN_IMPORT_FREE_BYTES.max(
        estimated_disk_bytes.saturating_mul(IMPORT_DISK_HEADROOM_NUMERATOR)
            / IMPORT_DISK_HEADROOM_DENOMINATOR,
    );
    let available_bytes = available_disk_space(dest_dir)
        .ok_or_else(|| "could not determine available disk space for video import".to_string())?;
    let required_bytes = estimated_disk_bytes.saturating_add(minimum_free_bytes);
    if available_bytes < required_bytes {
        return Err(format!(
            "video import is estimated to need {} and preserve {} free, but only {} is available",
            format_disk_bytes(estimated_disk_bytes),
            format_disk_bytes(minimum_free_bytes),
            format_disk_bytes(available_bytes),
        ));
    }
    let safety = ImportSafety {
        cancel,
        disk_path: dest_dir,
        minimum_free_bytes,
    };
    let import_dir = create_unique_import_dir(dest_dir)?;
    let extracted_width = source_width;
    let extracted_height = source_height;
    let mut import_dir_guard = ImportDirectoryGuard {
        path: import_dir.clone(),
        keep: false,
    };
    let frames_dir = import_dir.join("frames");
    std::fs::create_dir_all(&frames_dir)
        .map_err(|e| format!("failed to create media dir: {}", e))?;

    // 1. Decode frames at their source dimensions, numbered from 0. Lossless
    // WebP keeps imported footage substantially smaller than a PNG sequence
    // while preserving every source pixel and remaining directly decodable.
    let webp_pattern = frames_dir.join("frame_%05d.webp");
    let mut decode_webp = ffmpeg_command();
    decode_webp
        .arg("-y")
        .args(["-v", "error"])
        .arg("-i")
        .arg(src)
        .args(["-vf", &format!("fps={}", fps)])
        .args([
            "-c:v",
            "libwebp",
            "-lossless",
            "1",
            "-compression_level",
            "4",
        ])
        .args(["-start_number", "0"])
        .arg("--")
        .arg(&webp_pattern);
    let decode_webp = run_ffmpeg(&mut decode_webp, Some(&safety))
        .map_err(|e| format!("failed to run ffmpeg: {}", e))?;
    if !decode_webp.status.success() {
        if cancel.load(Ordering::Relaxed) {
            return Err("video import cancelled".into());
        }
        if available_disk_space(dest_dir).is_none_or(|available| available < minimum_free_bytes) {
            return Err("video import stopped to preserve the minimum free disk space".into());
        }
        // Some minimal FFmpeg builds omit libwebp. Keep those installations
        // usable by falling back to the legacy PNG sequence.
        let entries = std::fs::read_dir(&frames_dir)
            .map_err(|error| format!("failed to inspect partial WebP frames: {error}"))?;
        for entry in entries {
            let path = entry
                .map_err(|error| format!("failed to inspect partial WebP frame: {error}"))?
                .path();
            if path
                .extension()
                .is_some_and(|extension| extension == "webp")
            {
                std::fs::remove_file(path)
                    .map_err(|error| format!("failed to clear partial WebP frames: {error}"))?;
            }
        }
        let png_pattern = frames_dir.join("frame_%05d.png");
        let mut decode_png = ffmpeg_command();
        decode_png
            .arg("-y")
            .args(["-v", "error"])
            .arg("-i")
            .arg(src)
            .args(["-vf", &format!("fps={}", fps)])
            .args(["-start_number", "0"])
            .arg("--")
            .arg(&png_pattern);
        let decode_png = run_ffmpeg(&mut decode_png, Some(&safety))
            .map_err(|e| format!("failed to run ffmpeg PNG fallback: {}", e))?;
        if !decode_png.status.success() {
            return Err(format!(
                "ffmpeg frame extraction failed (WebP: {}; PNG: {})",
                decode_webp.stderr, decode_png.stderr
            ));
        }
    }

    // Count produced frames
    let mut frame_count = 0u32;
    for entry in
        std::fs::read_dir(&frames_dir).map_err(|e| format!("failed to read frames dir: {}", e))?
    {
        if cancel.load(Ordering::Relaxed) {
            return Err("video import cancelled".into());
        }
        let entry = entry.map_err(|e| format!("failed to finish reading frames dir: {}", e))?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("frame_") {
            let is_frame = name.ends_with(".webp") || name.ends_with(".png");
            if is_frame {
                if let Some(number) = name
                    .strip_prefix("frame_")
                    .and_then(|suffix| suffix.split('.').next())
                    .and_then(|digits| digits.parse::<u32>().ok())
                {
                    frame_count = frame_count.max(number.saturating_add(1));
                }
            }
        }
    }
    if cancel.load(Ordering::Relaxed) {
        return Err("video import cancelled".into());
    }
    if frame_count == 0 {
        return Err("ffmpeg produced no frames".into());
    }
    write_video_frame_cache_manifest(
        &frames_dir,
        frame_count,
        extracted_width,
        extracted_height,
        src,
        fps,
        cancel,
    )
    .map_err(|error| format!("could not verify extracted video frames: {error}"))?;

    // 2. Audio extraction (best effort — absence is not fatal)
    let mut audio_wav = None;
    let mut audio_warning = None;
    if cancel.load(Ordering::Relaxed) {
        return Err("video import cancelled".into());
    }
    if has_audio {
        if !extract_audio {
            audio_warning = Some(
                "source audio exceeds Kagari's 512 MiB decoded-audio limit and was not extracted"
                    .to_owned(),
            );
        } else {
            let wav = import_dir.join("audio.wav");
            let mut audio = ffmpeg_command();
            audio
                .arg("-y")
                .args(["-v", "error"])
                .arg("-i")
                .arg(src)
                .args(["-vn", "-acodec", "pcm_s16le", "-ar", "44100", "-ac", "2"])
                .arg("--")
                .arg(&wav);
            let audio = run_ffmpeg(&mut audio, Some(&safety));
            if cancel.load(Ordering::Relaxed) {
                return Err("video import cancelled".into());
            }
            match audio {
                Err(error) if error.to_string().contains("minimum free disk space") => {
                    return Err(
                        "video import stopped to preserve the minimum free disk space".into(),
                    );
                }
                Err(error) => {
                    let cleanup_warning = discard_partial_audio(&wav).unwrap_or_default();
                    audio_warning = Some(format!(
                        "FFmpeg could not extract the source audio: {error}{cleanup_warning}"
                    ));
                }
                Ok(out)
                    if out.status.success() && wav.metadata().is_ok_and(|meta| meta.len() > 44) =>
                {
                    audio_wav = Some(wav.to_string_lossy().to_string());
                }
                Ok(out) => {
                    let detail = out.stderr.trim();
                    let detail = if detail.is_empty() {
                        "FFmpeg did not produce a valid WAV file".to_owned()
                    } else {
                        detail.chars().take(500).collect::<String>()
                    };
                    let cleanup_warning = discard_partial_audio(&wav).unwrap_or_default();
                    audio_warning = Some(format!(
                        "source audio was not extracted: {detail}{cleanup_warning}"
                    ));
                }
            }
        }
    }
    if cancel.load(Ordering::Relaxed) {
        return Err("video import cancelled".into());
    }

    let asset = VideoAsset {
        source_path: src_path.to_string(),
        frames_dir: frames_dir.to_string_lossy().to_string(),
        frame_count,
        fps,
        width: extracted_width,
        height: extracted_height,
        audio_wav,
        audio_warning,
    };
    import_dir_guard.keep = true;
    Ok(asset)
}

/// Import a numbered still-image sequence through the same WebP frame cache
/// used by video footage. The selected file determines the directory, prefix,
/// and extension; siblings with the same prefix are sorted by numeric suffix
/// and rewritten as contiguous `frame_%05d.webp` files.
#[cfg(test)]
#[path = "video_import/tests.rs"]
mod tests;
