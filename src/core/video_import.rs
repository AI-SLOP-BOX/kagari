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
use std::process::{Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use std::{io, thread};

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

#[derive(serde::Serialize, serde::Deserialize)]
struct FrameCacheManifest {
    version: u32,
    frame_count: u32,
    width: u32,
    height: u32,
    total_bytes: u64,
    content_hash: u64,
}

static FRAME_CACHE_MANIFEST_SEQUENCE: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);

fn dimensions_within_import_budget(width: u32, height: u32) -> bool {
    if width < 2 || height < 2 || width > MAX_IMPORT_DIMENSION || height > MAX_IMPORT_DIMENSION {
        return false;
    }
    u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .is_some_and(|bytes| bytes <= MAX_IMPORT_DECODED_BYTES)
}

struct FfmpegOutput {
    status: ExitStatus,
    stderr: String,
}

struct ImportSafety<'a> {
    cancel: &'a AtomicBool,
    disk_path: &'a Path,
    minimum_free_bytes: u64,
}

fn run_ffmpeg(
    command: &mut Command,
    safety: Option<&ImportSafety<'_>>,
) -> io::Result<FfmpegOutput> {
    const TRUNCATED: &[u8] = b"\n[FFmpeg diagnostics truncated]";
    let mut child = command
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;
    let Some(mut stderr) = child.stderr.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(io::Error::other("FFmpeg stderr pipe was not available"));
    };
    let reader = thread::Builder::new()
        .name("kagari-ffmpeg-log".into())
        .spawn(move || -> io::Result<Vec<u8>> {
            let keep_limit = MAX_FFMPEG_DIAGNOSTIC_BYTES.saturating_sub(TRUNCATED.len());
            let mut output = Vec::with_capacity(MAX_FFMPEG_DIAGNOSTIC_BYTES);
            let mut buffer = [0u8; 8192];
            let mut truncated = false;
            loop {
                let count = io::Read::read(&mut stderr, &mut buffer)?;
                if count == 0 {
                    break;
                }
                let remaining = keep_limit.saturating_sub(output.len());
                let kept = count.min(remaining);
                output.extend_from_slice(&buffer[..kept]);
                truncated |= kept < count;
            }
            if truncated {
                output.extend_from_slice(TRUNCATED);
            }
            Ok(output)
        });
    let reader = match reader {
        Ok(reader) => reader,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
    };
    let mut last_space_check = Instant::now();
    let status = loop {
        if safety.is_some_and(|safety| safety.cancel.load(Ordering::Relaxed)) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = reader.join();
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "video import cancelled",
            ));
        }
        if last_space_check.elapsed() >= Duration::from_millis(500) {
            last_space_check = Instant::now();
            if safety.is_some_and(|safety| {
                available_disk_space(safety.disk_path)
                    .is_none_or(|available| available < safety.minimum_free_bytes)
            }) {
                let _ = child.kill();
                let _ = child.wait();
                let _ = reader.join();
                return Err(io::Error::other(
                    "video import stopped to preserve the minimum free disk space",
                ));
            }
        }
        if let Some(status) = child.try_wait()? {
            break status;
        }
        thread::sleep(Duration::from_millis(100));
    };
    let stderr = reader
        .join()
        .map_err(|_| io::Error::other("FFmpeg log reader thread panicked"))??;
    Ok(FfmpegOutput {
        status,
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
    })
}

fn available_disk_space(path: &Path) -> Option<u64> {
    let existing_path = path.ancestors().find(|candidate| candidate.exists())?;
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let path = std::ffi::CString::new(existing_path.as_os_str().as_bytes()).ok()?;
        let mut stats: libc::statvfs = unsafe { std::mem::zeroed() };
        if unsafe { libc::statvfs(path.as_ptr(), &mut stats) } != 0 {
            return None;
        }
        u64::from(stats.f_bavail).checked_mul(stats.f_frsize)
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use std::ptr;
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetDiskFreeSpaceExW(
                directory: *const u16,
                free_bytes_available: *mut u64,
                total_bytes: *mut u64,
                total_free_bytes: *mut u64,
            ) -> i32;
        }
        let wide_path = existing_path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let mut available = 0u64;
        let success = unsafe {
            GetDiskFreeSpaceExW(
                wide_path.as_ptr(),
                &mut available,
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };
        (success != 0).then_some(available)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = existing_path;
        None
    }
}

fn estimate_import_disk_bytes(
    width: u32,
    height: u32,
    frame_count: u64,
    duration_seconds: f64,
    has_audio: bool,
) -> Option<u64> {
    let frame_estimate = u64::from(width)
        .checked_mul(u64::from(height))?
        .checked_mul(frame_count)?
        / 2;
    let audio_estimate = if has_audio {
        (duration_seconds * 176_400.0).ceil() as u64
    } else {
        0
    };
    frame_estimate.checked_add(audio_estimate)
}

fn format_disk_bytes(bytes: u64) -> String {
    const GIB: u64 = 1024 * 1024 * 1024;
    if bytes >= GIB {
        format!("{:.1} GiB", bytes as f64 / GIB as f64)
    } else {
        format!("{:.0} MiB", bytes as f64 / (1024 * 1024) as f64)
    }
}

struct ImportDirectoryGuard {
    path: PathBuf,
    keep: bool,
}

impl Drop for ImportDirectoryGuard {
    fn drop(&mut self) {
        if !self.keep {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
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
    let has_audio = probe_has_audio(src);
    let estimated_disk_bytes = estimate_import_disk_bytes(
        source_width,
        source_height,
        estimated_frames,
        duration,
        has_audio,
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
    let extracted_width = source_width & !1;
    let extracted_height = source_height & !1;
    let mut import_dir_guard = ImportDirectoryGuard {
        path: import_dir.clone(),
        keep: false,
    };
    let frames_dir = import_dir.join("frames");
    std::fs::create_dir_all(&frames_dir)
        .map_err(|e| format!("failed to create media dir: {}", e))?;

    // 1. Decode frames: scale to even dimensions (encoder-safe), numbered from 0.
    // Lossless WebP keeps imported footage substantially smaller than a PNG
    // sequence while remaining directly decodable by image_cache.
    let webp_pattern = frames_dir.join("frame_%05d.webp");
    let mut decode_webp = Command::new("ffmpeg");
    decode_webp
        .arg("-y")
        .args(["-v", "error"])
        .arg("-i")
        .arg(src)
        .args([
            "-vf",
            &format!("fps={},scale=trunc(iw/2)*2:trunc(ih/2)*2", fps),
        ])
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
        let mut decode_png = Command::new("ffmpeg");
        decode_png
            .arg("-y")
            .args(["-v", "error"])
            .arg("-i")
            .arg(src)
            .args([
                "-vf",
                &format!("fps={},scale=trunc(iw/2)*2:trunc(ih/2)*2", fps),
            ])
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
    write_frame_cache_manifest(&frames_dir, frame_count, extracted_width, extracted_height)
        .map_err(|error| format!("could not verify extracted video frames: {error}"))?;

    // 2. Audio extraction (best effort — absence is not fatal)
    let mut audio_wav = None;
    if cancel.load(Ordering::Relaxed) {
        return Err("video import cancelled".into());
    }
    if has_audio {
        let wav = import_dir.join("audio.wav");
        let mut audio = Command::new("ffmpeg");
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
        if audio
            .as_ref()
            .is_err_and(|error| error.to_string().contains("minimum free disk space"))
        {
            return Err("video import stopped to preserve the minimum free disk space".into());
        }
        if let Ok(out) = audio {
            if out.status.success() && wav.exists() {
                audio_wav = Some(wav.to_string_lossy().to_string());
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
    };
    import_dir_guard.keep = true;
    Ok(asset)
}

/// Import a numbered still-image sequence through the same WebP frame cache
/// used by video footage. The selected file determines the directory, prefix,
/// and extension; siblings with the same prefix are sorted by numeric suffix
/// and rewritten as contiguous `frame_%05d.webp` files.
pub fn import_image_sequence(
    src_path: &Path,
    dest_dir: &Path,
    fps: f32,
) -> Result<VideoAsset, String> {
    if !fps.is_finite() || fps < 1.0 || fps > MAX_IMPORT_FPS {
        return Err(format!(
            "fps must be finite and between 1 and {}",
            MAX_IMPORT_FPS
        ));
    }
    if !src_path.is_file() {
        return Err(format!("source file not found: {}", src_path.display()));
    }
    let extension = src_path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| "image sequence source has no extension".to_string())?;
    if !matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "webp") {
        return Err(format!("unsupported image sequence format: .{extension}"));
    }
    let stem = src_path
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "image sequence source has no valid filename".to_string())?;
    let split_at = stem
        .char_indices()
        .rev()
        .find(|(_, ch)| !ch.is_ascii_digit())
        .map(|(index, ch)| index + ch.len_utf8())
        .ok_or_else(|| "image sequence filename must end with frame digits".to_string())?;
    let prefix = &stem[..split_at];
    if prefix.is_empty() || stem[split_at..].is_empty() {
        return Err("image sequence filename must contain a prefix and frame digits".to_string());
    }
    let _source_frame: u32 = stem[split_at..]
        .parse()
        .map_err(|_| "image sequence frame number is out of range".to_string())?;
    let parent = src_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));

    let mut frames: Vec<(u32, PathBuf)> = std::fs::read_dir(parent)
        .map_err(|error| format!("could not scan image sequence directory: {error}"))?
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let ext = path.extension()?.to_str()?.to_ascii_lowercase();
            if ext != extension {
                return None;
            }
            let candidate_stem = path.file_stem()?.to_str()?;
            if !candidate_stem.starts_with(prefix) {
                return None;
            }
            let suffix = &candidate_stem[prefix.len()..];
            let frame = suffix.parse::<u32>().ok()?;
            Some((frame, path))
        })
        .collect();
    frames.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    if frames.windows(2).any(|pair| pair[0].0 == pair[1].0) {
        return Err("image sequence contains duplicate frame numbers".to_string());
    }
    if frames.is_empty() || !frames.iter().any(|(_, path)| path == src_path) {
        return Err(format!(
            "no numbered image sequence found for {}",
            src_path.display()
        ));
    }
    if frames
        .windows(2)
        .any(|pair| pair[0].0.checked_add(1) != Some(pair[1].0))
    {
        return Err(
            "image sequence has missing frame numbers; fill the gaps before importing".to_string(),
        );
    }
    if frames.len() > MAX_IMPORT_FRAMES as usize {
        return Err(format!(
            "image sequence contains too many frames (limit {})",
            MAX_IMPORT_FRAMES
        ));
    }
    let (width, height) = image::image_dimensions(&frames[0].1)
        .map_err(|error| format!("could not read image sequence dimensions: {error}"))?;
    if !dimensions_within_import_budget(width, height) {
        return Err(format!(
            "image sequence dimensions exceed the import limit (maximum side {} px and decoded RGBA size {} MiB)",
            MAX_IMPORT_DIMENSION,
            MAX_IMPORT_DECODED_BYTES / (1024 * 1024)
        ));
    }

    let import_dir = create_unique_import_dir(dest_dir)?;
    let mut import_dir_guard = ImportDirectoryGuard {
        path: import_dir.clone(),
        keep: false,
    };
    let frames_dir = import_dir.join("frames");
    std::fs::create_dir_all(&frames_dir)
        .map_err(|error| format!("failed to create image sequence cache: {error}"))?;
    for (output_index, (_, source)) in frames.iter().enumerate() {
        let (frame_width, frame_height) = image::image_dimensions(source).map_err(|error| {
            format!(
                "could not read sequence frame dimensions {}: {error}",
                source.display()
            )
        })?;
        if frame_width != width || frame_height != height {
            return Err(format!(
                "image sequence frame {} is {}x{}, expected {}x{}",
                source.display(),
                frame_width,
                frame_height,
                width,
                height
            ));
        }
        if !dimensions_within_import_budget(frame_width, frame_height) {
            return Err(format!(
                "image sequence frame dimensions exceed the import limit (maximum side {} px and decoded RGBA size {} MiB): {} is {}x{}",
                MAX_IMPORT_DIMENSION,
                MAX_IMPORT_DECODED_BYTES / (1024 * 1024),
                source.display(),
                frame_width,
                frame_height
            ));
        }
        let image = match image::open(source) {
            Ok(image) => image,
            Err(error) => {
                return Err(format!(
                    "could not decode sequence frame {}: {error}",
                    source.display()
                ));
            }
        };
        let target = frames_dir.join(format!("frame_{output_index:05}.webp"));
        if let Err(error) = image.save_with_format(&target, image::ImageFormat::WebP) {
            return Err(format!(
                "could not encode sequence frame {}: {error}",
                source.display()
            ));
        }
    }
    write_frame_cache_manifest(&frames_dir, frames.len() as u32, width, height)
        .map_err(|error| format!("could not verify imported image sequence: {error}"))?;

    let asset = VideoAsset {
        source_path: src_path.to_string_lossy().into_owned(),
        frames_dir: frames_dir.to_string_lossy().into_owned(),
        frame_count: frames.len() as u32,
        fps,
        width,
        height,
        audio_wav: None,
    };
    import_dir_guard.keep = true;
    Ok(asset)
}

/// Rebuilds missing decoded frame caches for video layers after a project
/// relink. The source movie is the authoritative media; an old or missing
/// `frames_dir` must never leave a relinked layer showing stale footage.
///
/// Returns the number of refreshed layers and non-fatal errors for sources
/// that could not be decoded. Errors are collected so one offline clip does
/// not prevent the rest of a project from opening.
pub fn refresh_missing_video_caches(
    project: &mut crate::core::timeline::Project,
    destination_root: &Path,
) -> (usize, Vec<String>) {
    use std::collections::HashMap;

    let mut cache: HashMap<(String, u32), VideoAsset> = HashMap::new();
    let mut refreshed = 0usize;
    let mut errors = Vec::new();
    for comp in &mut project.compositions {
        refresh_missing_video_caches_in_comp(
            comp,
            destination_root,
            &mut cache,
            &mut refreshed,
            &mut errors,
        );
    }
    (refreshed, errors)
}

fn refresh_missing_video_caches_in_comp(
    comp: &mut crate::core::timeline::Composition,
    destination_root: &Path,
    cache: &mut std::collections::HashMap<(String, u32), VideoAsset>,
    refreshed: &mut usize,
    errors: &mut Vec<String>,
) {
    for layer in &mut comp.layers {
        let Some((source, frames_dir, frame_count)) = (match &layer.layer_type {
            crate::core::timeline::LayerType::Video {
                source,
                frames_dir,
                frame_count,
                ..
            } => Some((source.clone(), frames_dir.clone(), *frame_count)),
            _ => None,
        }) else {
            continue;
        };

        let cache_is_valid = video_frame_cache_is_complete(&frames_dir, frame_count);
        if cache_is_valid {
            continue;
        }
        if !Path::new(&source).is_file() {
            errors.push(format!(
                "{}: decoded frame cache is incomplete and the source video is unavailable",
                source
            ));
            continue;
        }

        let key = (source.clone(), comp.fps.max(1));
        let asset = if let Some(existing) = cache.get(&key) {
            existing.clone()
        } else {
            let stem = Path::new(&source)
                .file_stem()
                .map(|value| value.to_string_lossy())
                .unwrap_or_else(|| std::borrow::Cow::Borrowed("video"));
            let safe_stem: String = stem
                .chars()
                .map(|ch| {
                    if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                        ch
                    } else {
                        '_'
                    }
                })
                .collect();
            let destination =
                destination_root.join(format!("{}_{}fps", safe_stem, comp.fps.max(1)));
            match import_video(&source, &destination, comp.fps.max(1) as f32) {
                Ok(asset) => {
                    cache.insert(key.clone(), asset.clone());
                    asset
                }
                Err(error) => {
                    errors.push(format!("{}: {}", source, error));
                    continue;
                }
            }
        };

        if let crate::core::timeline::LayerType::Video {
            source: layer_source,
            frames_dir: layer_frames_dir,
            frame_count: layer_frame_count,
            audio_wav: layer_audio_wav,
            ..
        } = &mut layer.layer_type
        {
            *layer_source = asset.source_path;
            *layer_frames_dir = asset.frames_dir;
            *layer_frame_count = asset.frame_count;
            *layer_audio_wav = asset.audio_wav;
            *refreshed += 1;
        }
    }
    for sub in &mut comp.sub_compositions {
        refresh_missing_video_caches_in_comp(sub, destination_root, cache, refreshed, errors);
    }
}

fn video_frame_cache_is_complete(frames_dir: &str, frame_count: u32) -> bool {
    if frame_count == 0 || frame_count as u64 > MAX_IMPORT_FRAMES {
        return false;
    }
    let Ok(entries) = std::fs::read_dir(frames_dir) else {
        return false;
    };
    let mut seen = vec![0u64; (frame_count as usize).div_ceil(64)];
    let mut found = 0u32;
    for entry in entries {
        let Ok(entry) = entry else {
            return false;
        };
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let Some(number) = name.strip_prefix("frame_").and_then(|name| {
            name.strip_suffix(".webp")
                .or_else(|| name.strip_suffix(".png"))
        }) else {
            continue;
        };
        let Ok(index) = number.parse::<u32>() else {
            continue;
        };
        if index >= frame_count
            || entry
                .metadata()
                .map_or(true, |metadata| metadata.len() == 0)
        {
            return false;
        }
        let word = index as usize / 64;
        let mask = 1u64 << (index % 64);
        if seen[word] & mask != 0 {
            return false;
        }
        seen[word] |= mask;
        found += 1;
    }
    if found != frame_count
        || !(0..frame_count).all(|index| seen[index as usize / 64] & (1u64 << (index % 64)) != 0)
    {
        return false;
    }

    let frames_dir = Path::new(frames_dir);
    let manifest_path = frames_dir.join(FRAME_CACHE_MANIFEST_FILE);
    match std::fs::metadata(&manifest_path) {
        Ok(metadata) if metadata.len() <= 4096 => {
            let Ok(source) =
                crate::core::project_migration::read_bounded_text_file(&manifest_path, 4096)
            else {
                return false;
            };
            let Ok(manifest) = serde_json::from_str::<FrameCacheManifest>(&source) else {
                return false;
            };
            if manifest.version != 1
                || manifest.frame_count != frame_count
                || !dimensions_within_import_budget(manifest.width, manifest.height)
            {
                return false;
            }
            frame_cache_fingerprint(frames_dir, frame_count).is_some_and(|(total, hash)| {
                total == manifest.total_bytes && hash == manifest.content_hash
            })
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let Some((width, height)) = validate_legacy_frame_cache(frames_dir, frame_count) else {
                return false;
            };
            let _ = write_frame_cache_manifest(frames_dir, frame_count, width, height);
            true
        }
        Err(_) => false,
        Ok(_) => false,
    }
}

fn frame_cache_fingerprint(frames_dir: &Path, frame_count: u32) -> Option<(u64, u64)> {
    use io::Read;

    let mut total_bytes = 0u64;
    let mut hash = 0xcbf29ce484222325u64;
    let mut buffer = [0u8; 64 * 1024];
    for frame in 0..frame_count {
        let path = frame_path_in_dir(&frames_dir.to_string_lossy(), frame);
        let mut file = std::fs::File::open(&path).ok()?;
        hash = fnv1a_update(hash, &frame.to_le_bytes());
        loop {
            let count = file.read(&mut buffer).ok()?;
            if count == 0 {
                break;
            }
            total_bytes = total_bytes.checked_add(count as u64)?;
            hash = fnv1a_update(hash, &buffer[..count]);
        }
    }
    Some((total_bytes, hash))
}

fn fnv1a_update(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn validate_legacy_frame_cache(frames_dir: &Path, frame_count: u32) -> Option<(u32, u32)> {
    let mut dimensions = None;
    for frame in 0..frame_count {
        let path = frame_path_in_dir(&frames_dir.to_string_lossy(), frame);
        let current = image::image_dimensions(&path).ok()?;
        if !dimensions_within_import_budget(current.0, current.1)
            || dimensions.is_some_and(|expected| expected != current)
        {
            return None;
        }
        image::open(path).ok()?;
        dimensions = Some(current);
    }
    dimensions
}

fn write_frame_cache_manifest(
    frames_dir: &Path,
    frame_count: u32,
    width: u32,
    height: u32,
) -> io::Result<()> {
    if frame_count == 0
        || frame_count as u64 > MAX_IMPORT_FRAMES
        || !dimensions_within_import_budget(width, height)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "frame cache metadata is outside supported limits",
        ));
    }
    let (total_bytes, content_hash) = frame_cache_fingerprint(frames_dir, frame_count)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "frame cache is incomplete"))?;
    let manifest = FrameCacheManifest {
        version: 1,
        frame_count,
        width,
        height,
        total_bytes,
        content_hash,
    };
    let bytes = serde_json::to_vec(&manifest)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let temporary = frames_dir.join(format!(
        "{FRAME_CACHE_MANIFEST_FILE}.tmp.{}.{}",
        std::process::id(),
        FRAME_CACHE_MANIFEST_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let result = (|| {
        let mut file = std::fs::File::create(&temporary)?;
        use io::Write;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temporary, frames_dir.join(FRAME_CACHE_MANIFEST_FILE))
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(temporary);
    }
    result
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

    fn test_cache_dir(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "kagari_{name}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).expect("test cache directory should be created");
        root
    }

    #[test]
    fn legacy_nonempty_but_corrupt_frames_are_rejected() {
        let root = test_cache_dir("corrupt_video_cache");
        std::fs::write(root.join("frame_00000.webp"), b"nonempty but not an image")
            .expect("corrupt test frame should be written");

        assert!(!video_frame_cache_is_complete(
            root.to_str().expect("temporary path should be UTF-8"),
            1
        ));

        std::fs::remove_dir_all(root).expect("test cache directory should be removable");
    }

    #[test]
    fn cache_manifest_detects_frame_content_changes() {
        let root = test_cache_dir("video_cache_manifest");
        let frame = root.join("frame_00000.png");
        image::RgbaImage::from_pixel(4, 4, image::Rgba([30, 80, 120, 255]))
            .save(&frame)
            .expect("legacy frame should be written");

        let cache_path = root.to_str().expect("temporary path should be UTF-8");
        assert!(video_frame_cache_is_complete(cache_path, 1));
        assert!(root.join(FRAME_CACHE_MANIFEST_FILE).is_file());

        std::fs::write(&frame, b"different nonempty content")
            .expect("mutated frame should be written");
        assert!(!video_frame_cache_is_complete(cache_path, 1));

        std::fs::remove_dir_all(root).expect("test cache directory should be removable");
    }

    #[test]
    fn import_dimensions_respect_decoded_byte_budget() {
        assert!(dimensions_within_import_budget(8192, 4096));
        assert!(!dimensions_within_import_budget(8192, 4097));
        assert!(!dimensions_within_import_budget(16_384, 16_384));
        assert!(!dimensions_within_import_budget(16_385, 1));
    }

    #[test]
    fn disk_estimate_scales_with_frames_and_includes_audio() {
        let still_video = estimate_import_disk_bytes(1920, 1080, 30, 1.0, false)
            .expect("bounded dimensions should produce an estimate");
        let longer_video = estimate_import_disk_bytes(1920, 1080, 60, 2.0, false)
            .expect("bounded duration should produce an estimate");
        let with_audio = estimate_import_disk_bytes(1920, 1080, 30, 1.0, true)
            .expect("bounded duration should include audio estimate");

        assert_eq!(longer_video, still_video * 2);
        assert!(with_audio > still_video);
    }

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

    #[test]
    fn image_sequence_import_reindexes_frames_as_webp() {
        let root = std::env::temp_dir().join(format!(
            "kagari_image_sequence_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).expect("test root should be created");
        for frame in [1u32, 2, 3] {
            let image = image::RgbaImage::from_pixel(8, 4, image::Rgba([frame as u8, 20, 40, 255]));
            image
                .save(root.join(format!("shot_{frame:04}.png")))
                .expect("source frame should be writable");
        }

        let source = root.join("shot_0002.png");
        let asset = import_image_sequence(&source, &root.join("cache"), 24.0)
            .expect("numbered stills should import");
        assert_eq!(asset.frame_count, 3);
        assert!(frame_path(&asset, 0).ends_with("frame_00000.webp"));
        assert!(frame_path(&asset, 2).is_file());
        assert_eq!(
            image::image_dimensions(frame_path(&asset, 1)).unwrap(),
            (8, 4)
        );

        std::fs::remove_dir_all(root).expect("test root should be removable");
    }

    #[test]
    fn refresh_rebuilds_missing_video_cache_from_source() {
        if !ffmpeg_available() {
            return;
        }

        let root = std::env::temp_dir().join(format!(
            "kagari_video_relink_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).expect("test root should be created");
        let source = root.join("clip.mp4");
        let encoded = Command::new("ffmpeg")
            .args([
                "-y",
                "-f",
                "lavfi",
                "-i",
                "color=c=red:s=32x32:r=10",
                "-t",
                "0.2",
                "-an",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
            ])
            .arg(&source)
            .output()
            .expect("ffmpeg should be executable when reported available");
        if !encoded.status.success() {
            std::fs::remove_dir_all(root).expect("test root should be removable");
            return;
        }

        let mut project = crate::core::timeline::Project::default();
        project.compositions[0]
            .layers
            .push(crate::core::timeline::Layer::new(
                "video".into(),
                "Clip".into(),
                crate::core::timeline::LayerType::Video {
                    source: source.to_string_lossy().into_owned(),
                    frames_dir: root.join("missing").to_string_lossy().into_owned(),
                    frame_count: 0,
                    audio_wav: None,
                    speed: 1.0,
                },
                30,
            ));

        let cache_root = root.join("cache");
        let (refreshed, errors) = refresh_missing_video_caches(&mut project, &cache_root);
        assert_eq!(refreshed, 1);
        assert!(errors.is_empty(), "unexpected cache errors: {errors:?}");
        let crate::core::timeline::LayerType::Video {
            frames_dir,
            frame_count,
            audio_wav,
            ..
        } = &project.compositions[0].layers.last().unwrap().layer_type
        else {
            panic!("expected refreshed video layer");
        };
        assert!(*frame_count > 0);
        assert!(frame_path_in_dir(frames_dir, 0).is_file());
        assert!(audio_wav.is_none());

        std::fs::remove_dir_all(root).expect("test root should be removable");
    }
}
