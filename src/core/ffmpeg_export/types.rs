use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

pub(crate) type RenderFrameFn = Arc<dyn Fn(&str, u32) -> Vec<u8> + Send + Sync>;

static TEMP_OUTPUT_SEQUENCE: AtomicU64 = AtomicU64::new(0);
pub(crate) const MAX_FFMPEG_STDERR_BYTES: usize = 1024 * 1024;

pub(crate) fn temporary_output_path(output_path: &str) -> String {
    let output = PathBuf::from(output_path);
    let parent = output.parent().unwrap_or_else(|| std::path::Path::new(""));
    let stem = output
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("kagari-output");
    let extension = output
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| format!(".{value}"))
        .unwrap_or_default();
    let name = format!(
        "{stem}.kagari-tmp-{}-{}{extension}",
        std::process::id(),
        TEMP_OUTPUT_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    );
    parent.join(name).to_string_lossy().into_owned()
}

pub(crate) fn validate_export_config(config: &ExportConfig, gif: bool) -> Result<(), String> {
    if config.output_path.starts_with('-') {
        return Err(format!(
            "output_path must not start with '-': {}",
            config.output_path
        ));
    }
    if let Some(wav) = &config.audio_wav {
        if wav.starts_with('-') {
            return Err(format!("audio_wav path must not start with '-': {wav}"));
        }
    }
    if config.total_frames == 0 {
        return Err("Cannot export a composition with no frames".to_string());
    }
    if config.fps == 0 {
        return Err("Frame rate must be greater than zero".to_string());
    }
    if crate::core::software_renderer::rgba_buffer_size(config.width, config.height).is_none() {
        return Err(format!(
            "Invalid export dimensions: {}x{}",
            config.width, config.height
        ));
    }
    let extension = PathBuf::from(&config.output_path)
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| "Output filename must include a media extension".to_string())?;
    if gif && extension != "gif" {
        return Err("GIF export requires a .gif output filename".to_string());
    }
    if gif && config.audio_wav.is_some() {
        return Err("GIF export does not support an audio track".to_string());
    }
    if !gif && matches!(&config.codec, &VideoCodec::Gif) {
        return Err("GIF export must use the GIF palette export pipeline".to_string());
    }
    if !gif && matches!(&config.codec, &VideoCodec::WebP) && extension != "webp" {
        return Err("WebP export requires a .webp output filename".to_string());
    }
    if !gif && matches!(&config.codec, &VideoCodec::WebP) && config.audio_wav.is_some() {
        return Err("WebP export does not support an audio track".to_string());
    }
    Ok(())
}

pub(crate) struct TemporaryOutputGuard {
    pub path: PathBuf,
    pub keep: bool,
}

impl TemporaryOutputGuard {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            keep: false,
        }
    }

    pub fn persist(&mut self) {
        self.keep = true;
    }
}

impl Drop for TemporaryOutputGuard {
    fn drop(&mut self) {
        if !self.keep {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

/// RAII guard that ensures a child process is killed and waited on drop.
pub(crate) struct ChildGuard {
    pub(crate) child: Option<Child>,
}

impl ChildGuard {
    pub fn new(child: Child) -> Self {
        Self { child: Some(child) }
    }

    pub fn take(&mut self) -> Option<Child> {
        self.child.take()
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// Drain stderr from a child process in a background thread, returning the output.
pub(crate) fn drain_stderr(mut child: Child) -> (Child, std::thread::JoinHandle<String>) {
    let stderr = child.stderr.take();
    let handle = std::thread::spawn(move || {
        let mut output = Vec::new();
        if let Some(mut s) = stderr {
            let mut limited = std::io::Read::take(&mut s, (MAX_FFMPEG_STDERR_BYTES + 1) as u64);
            let _ = std::io::Read::read_to_end(&mut limited, &mut output);
        }
        if output.len() > MAX_FFMPEG_STDERR_BYTES {
            output.truncate(MAX_FFMPEG_STDERR_BYTES);
            output.extend_from_slice(b"\n[FFmpeg stderr truncated]");
        }
        String::from_utf8_lossy(&output).into_owned()
    });
    (child, handle)
}

/// Events sent from the export thread to the UI thread.
#[derive(Debug, Clone)]
pub enum ExportEvent {
    Progress(f32, String),
    Finished(String),
    Error(String),
}

/// Configuration for the export job.
#[derive(Clone, PartialEq, Eq, Default)]
pub enum VideoCodec {
    /// H.264 — universal compatibility, small files
    #[default]
    H264,
    /// Apple ProRes 422 — professional editing codec, large files
    ProRes422,
    /// Apple ProRes 4444 — highest quality, alpha channel support
    ProRes4444,
    /// HEVC / H.265 — next-gen compression, smaller files than H.264
    H265,
    /// GIF animation
    Gif,
    /// WebP animation
    WebP,
}

#[derive(Clone)]
pub struct ExportConfig {
    pub output_path: String,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub total_frames: u32,
    pub audio_wav: Option<String>,
    pub codec: VideoCodec,
}

/// Check whether `ffmpeg` is available in PATH.
pub fn is_ffmpeg_available() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub(crate) fn render_with_cancel<F>(
    cancel_flag: &Arc<AtomicBool>,
    render_frame_fn: &F,
    frame_idx: u32,
) -> Result<Vec<u8>, ()>
where
    F: Fn(u32) -> Vec<u8>,
{
    crate::core::software_renderer::set_render_cancel_flag(Some(cancel_flag.clone()));
    let result =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| render_frame_fn(frame_idx)));
    crate::core::software_renderer::set_render_cancel_flag(None);
    result.map_err(|_| ())
}
