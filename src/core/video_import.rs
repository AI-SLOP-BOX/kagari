//! Video import via FFmpeg.
//!
//! Videos are decoded once at import time into a PNG frame sequence plus an
//! optional WAV audio track under the project's media directory. Rendering then
//! samples the sequence like any other image source — no video decoder needed
//! at runtime or export time.
//!
//! This mirrors how NLEs proxy media: one decode pass up front, cheap random
//! access afterwards.

use std::path::{Path, PathBuf};
use std::process::Command;

const MAX_IMPORT_FPS: f32 = 240.0;
const MAX_IMPORT_DURATION_SECONDS: f64 = 24.0 * 60.0 * 60.0;
const MAX_IMPORT_FRAMES: u64 = 2_000_000;
const MAX_IMPORT_DIMENSION: u32 = 16_384;

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
}

/// True if ffmpeg is available on PATH.
pub fn ffmpeg_available() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn probe_duration_seconds(path: &Path) -> Option<f64> {
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            "--",
        ])
        .arg(path)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse::<f64>()
        .ok()
}

fn probe_dimensions(path: &Path) -> Option<(u32, u32)> {
    let out = Command::new("ffprobe")
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
        .arg(path)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let mut it = s.split('x');
    let w = it.next()?.parse().ok()?;
    let h = it.next()?.parse().ok()?;
    Some((w, h))
}

fn probe_has_audio(path: &Path) -> bool {
    Command::new("ffprobe")
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
        .arg(path)
        .output()
        .map(|o| o.status.success() && String::from_utf8_lossy(&o.stdout).contains("audio"))
        .unwrap_or(false)
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
        match std::fs::create_dir(&candidate) {
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
    if !ffmpeg_available() {
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
    let duration = probe_duration_seconds(src)
        .ok_or_else(|| "could not determine video duration".to_string())?;
    if !duration.is_finite() || duration <= 0.0 || duration > MAX_IMPORT_DURATION_SECONDS {
        return Err(format!(
            "video duration must be between 0 and {} hours",
            MAX_IMPORT_DURATION_SECONDS / 3600.0
        ));
    }
    let (source_width, source_height) =
        probe_dimensions(src).ok_or_else(|| "could not determine video dimensions".to_string())?;
    if source_width == 0
        || source_height == 0
        || source_width > MAX_IMPORT_DIMENSION
        || source_height > MAX_IMPORT_DIMENSION
    {
        return Err(format!(
            "video dimensions must be within {}x{}",
            MAX_IMPORT_DIMENSION, MAX_IMPORT_DIMENSION
        ));
    }
    let estimated_frames = (duration * f64::from(fps)).ceil() as u64;
    if estimated_frames > MAX_IMPORT_FRAMES {
        return Err(format!(
            "video would produce too many frames (limit {})",
            MAX_IMPORT_FRAMES
        ));
    }
    let import_dir = create_unique_import_dir(dest_dir)?;
    let frames_dir = import_dir.join("frames");
    std::fs::create_dir_all(&frames_dir)
        .map_err(|e| format!("failed to create media dir: {}", e))?;

    // 1. Decode frames: scale to even dimensions (encoder-safe), numbered from 0.
    // Lossless WebP keeps imported footage substantially smaller than a PNG
    // sequence while remaining directly decodable by image_cache.
    let webp_pattern = frames_dir.join("frame_%05d.webp");
    let decode_webp = Command::new("ffmpeg")
        .arg("-y")
        .arg("-i")
        .arg(src)
        .args([
            "-vf",
            &format!("fps={},scale=trunc(iw/2)*2:trunc(ih/2)*2", fps),
        ])
        .args(["-c:v", "libwebp", "-lossless", "1", "-compression_level", "4"])
        .args(["-start_number", "0"])
        .arg("--")
        .arg(&webp_pattern)
        .output()
        .map_err(|e| format!("failed to run ffmpeg: {}", e))?;
    if !decode_webp.status.success() {
        // Some minimal FFmpeg builds omit libwebp. Keep those installations
        // usable by falling back to the legacy PNG sequence.
        let png_pattern = frames_dir.join("frame_%05d.png");
        let decode_png = Command::new("ffmpeg")
            .arg("-y")
            .arg("-i")
            .arg(src)
            .args([
                "-vf",
                &format!("fps={},scale=trunc(iw/2)*2:trunc(ih/2)*2", fps),
            ])
            .args(["-start_number", "0"])
            .arg("--")
            .arg(&png_pattern)
            .output()
            .map_err(|e| format!("failed to run ffmpeg PNG fallback: {}", e))?;
        if !decode_png.status.success() {
            return Err(format!(
                "ffmpeg frame extraction failed (WebP: {}; PNG: {})",
                String::from_utf8_lossy(&decode_webp.stderr),
                String::from_utf8_lossy(&decode_png.stderr)
            ));
        }
    }

    // Count produced frames
    let mut frame_count = 0u32;
    let entries =
        std::fs::read_dir(&frames_dir).map_err(|e| format!("failed to read frames dir: {}", e))?;
    for entry in entries.flatten() {
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
    if frame_count == 0 {
        return Err("ffmpeg produced no frames".into());
    }

    // 2. Audio extraction (best effort — absence is not fatal)
    let mut audio_wav = None;
    if probe_has_audio(src) {
        let wav = import_dir.join("audio.wav");
        let audio = Command::new("ffmpeg")
            .arg("-y")
            .arg("-i")
            .arg(src)
            .args(["-vn", "-acodec", "pcm_s16le", "-ar", "44100", "-ac", "2"])
            .arg("--")
            .arg(&wav)
            .output();
        if let Ok(out) = audio {
            if out.status.success() && wav.exists() {
                audio_wav = Some(wav.to_string_lossy().to_string());
            }
        }
    }

    Ok(VideoAsset {
        source_path: src_path.to_string(),
        frames_dir: frames_dir.to_string_lossy().to_string(),
        frame_count,
        fps,
        width: source_width,
        height: source_height,
        audio_wav,
    })
}

/// Returns the extracted frame path (WebP preferred, PNG fallback), clamped to
/// the imported sequence range.
pub fn frame_path(asset: &VideoAsset, frame: u32) -> PathBuf {
    let clamped = frame.min(asset.frame_count.saturating_sub(1));
    frame_path_in_dir(&asset.frames_dir, clamped)
}

/// Resolve an extracted video frame while supporting both the current WebP
/// sequence and older projects that still contain PNG frames.
pub fn frame_path_in_dir(frames_dir: &str, frame: u32) -> PathBuf {
    let dir = Path::new(frames_dir);
    let webp = dir.join(format!("frame_{:05}.webp", frame));
    if webp.is_file() {
        webp
    } else {
        dir.join(format!("frame_{:05}.png", frame))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_path_clamps_to_range() {
        let asset = VideoAsset {
            source_path: "src.mp4".into(),
            frames_dir: "/tmp/media/frames".into(),
            frame_count: 10,
            fps: 30.0,
            width: 640,
            height: 360,
            audio_wav: None,
        };
        assert!(frame_path(&asset, 0).ends_with("frame_00000.png"));
        assert!(frame_path(&asset, 5).ends_with("frame_00005.png"));
        // Out-of-range clamps to last frame (freeze-frame behavior)
        assert_eq!(frame_path(&asset, 999), frame_path(&asset, 9));
    }

    #[test]
    fn test_import_rejects_missing_source() {
        let result = import_video(
            "/nonexistent/video.mp4",
            Path::new("/tmp/kagari_vid_test"),
            30.0,
        );
        // Missing source must be rejected whether or not ffmpeg exists
        if ffmpeg_available() {
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("not found"));
        }
    }

    #[test]
    fn test_import_rejects_without_ffmpeg_gracefully() {
        // If ffmpeg is absent we get a clean error, never a panic
        let result = import_video("/dev/null", Path::new("/tmp/kagari_vid_test2"), 30.0);
        let err = result.expect_err("/dev/null is never a valid video source");
        assert!(
            err.contains("ffmpeg not found")
                || err.contains("source file not found")
                || err.contains("could not determine video"),
            "unexpected import failure mode: {}",
            err
        );
    }

    #[test]
    fn import_directory_is_unique_without_overwriting_existing_media() {
        let root = std::env::temp_dir().join(format!(
            "kagari_video_import_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after unix epoch")
                .as_nanos()
        ));
        let requested = root.join("clip");
        let first = create_unique_import_dir(&requested).expect("first directory should be made");
        let second = create_unique_import_dir(&requested).expect("second directory should be made");
        assert_eq!(first, requested);
        assert_eq!(second, root.join("clip_1"));
        assert!(first.is_dir());
        assert!(second.is_dir());
        std::fs::remove_dir_all(root).expect("test directories should be removable");
    }
}
