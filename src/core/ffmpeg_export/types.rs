use std::path::PathBuf;
use std::process::{Child, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use crate::core::external_tools::ffmpeg_command;

#[cfg(unix)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FileIdentity(u64, u64);

#[cfg(windows)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FileIdentity(u32, u64);

#[cfg(not(any(unix, windows)))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FileIdentity;

impl FileIdentity {
    #[doc(hidden)]
    pub const RECORD_SIZE: usize = 16;

    #[doc(hidden)]
    pub fn to_record(self) -> [u8; Self::RECORD_SIZE] {
        let mut record = [0; Self::RECORD_SIZE];
        #[cfg(unix)]
        {
            record[..8].copy_from_slice(&self.0.to_le_bytes());
            record[8..].copy_from_slice(&self.1.to_le_bytes());
        }
        #[cfg(windows)]
        {
            record[..4].copy_from_slice(&self.0.to_le_bytes());
            record[8..].copy_from_slice(&self.1.to_le_bytes());
        }
        record
    }

    #[doc(hidden)]
    pub fn from_record(record: [u8; Self::RECORD_SIZE]) -> Self {
        #[cfg(unix)]
        {
            Self(
                u64::from_le_bytes(record[..8].try_into().expect("fixed identity record")),
                u64::from_le_bytes(record[8..].try_into().expect("fixed identity record")),
            )
        }
        #[cfg(windows)]
        {
            Self(
                u32::from_le_bytes(record[..4].try_into().expect("fixed identity record")),
                u64::from_le_bytes(record[8..].try_into().expect("fixed identity record")),
            )
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = record;
            Self
        }
    }
}

#[path = "types/publication.rs"]
mod publication;
pub(crate) use publication::publish_export_output_cancelable;
pub use publication::{
    file_identity, path_file_identity, publish_file_no_replace, snapshot_overwrite_identity,
};
#[cfg(test)]
pub(crate) use publication::{publish_export_output, rename_no_replace, stage_copy_and_publish};

pub(crate) type RenderFrameFn = Arc<dyn Fn(&str, u32) -> Vec<u8> + Send + Sync>;

static TEMP_OUTPUT_SEQUENCE: AtomicU64 = AtomicU64::new(0);
pub(crate) const MAX_FFMPEG_STDERR_BYTES: usize = 1024 * 1024;

fn canonical_output_location(path: &std::path::Path) -> Option<PathBuf> {
    if let Ok(canonical) = std::fs::canonicalize(path) {
        return Some(canonical);
    }
    let parent = path.parent().unwrap_or_else(|| std::path::Path::new("."));
    let file_name = path.file_name()?;
    Some(std::fs::canonicalize(parent).ok()?.join(file_name))
}

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

pub(crate) fn create_temporary_output_path(output_path: &str) -> Result<String, String> {
    for _ in 0..100 {
        let candidate = temporary_output_path(output_path);
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => {
                drop(file);
                return Ok(candidate);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!(
                    "Failed to reserve temporary export file {}: {error}",
                    candidate
                ));
            }
        }
    }
    Err("Could not reserve a unique temporary export file after 100 attempts".to_string())
}

pub(crate) fn reserve_temporary_path(path: &str) -> Result<(), String> {
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map(drop)
        .map_err(|error| format!("Failed to reserve temporary export file {path}: {error}"))
}

pub(crate) fn replace_existing_file(
    source: &std::path::Path,
    destination: &std::path::Path,
) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_REPLACE_EXISTING};

        let source = source
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let destination = destination
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        if unsafe {
            MoveFileExW(
                source.as_ptr(),
                destination.as_ptr(),
                MOVEFILE_REPLACE_EXISTING,
            )
        } == 0
        {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        std::fs::rename(source, destination)
    }
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
        let audio_path = PathBuf::from(wav);
        let output_path = PathBuf::from(&config.output_path);
        if let (Ok(audio), Some(output)) = (
            std::fs::canonicalize(&audio_path),
            canonical_output_location(&output_path),
        ) {
            if audio == output {
                return Err(
                    "Video output cannot overwrite the WAV file used as its audio input".into(),
                );
            }
        }
    }
    if config.total_frames == 0 {
        return Err("Cannot export a composition with no frames".to_string());
    }
    if config.fps == 0 {
        return Err("Frame rate must be greater than zero".to_string());
    }
    if config.quality > 100 {
        return Err("Export quality must be between 0 and 100".to_string());
    }
    if !(32..=512).contains(&config.audio_bitrate_kbps) {
        return Err("Audio bitrate must be between 32 and 512 kbps".to_string());
    }
    if crate::core::software_renderer::rgba_buffer_size(config.width, config.height).is_none() {
        return Err(format!(
            "Invalid export dimensions: {}x{}",
            config.width, config.height
        ));
    }
    if !gif
        && matches!(config.codec, VideoCodec::H264 | VideoCodec::H265)
        && (!config.width.is_multiple_of(2) || !config.height.is_multiple_of(2))
    {
        return Err(format!(
            "H.264/H.265 yuv420p export requires even dimensions; got {}x{}",
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
    permissions_to_restore: Option<std::fs::Permissions>,
    restricted_file_identity: Option<FileIdentity>,
}

impl TemporaryOutputGuard {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            keep: false,
            permissions_to_restore: None,
            restricted_file_identity: None,
        }
    }

    pub fn restrict_while_writing(&mut self) -> std::io::Result<()> {
        #[cfg(unix)]
        {
            let file = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&self.path)?;
            self.restricted_file_identity = Some(file_identity(&file)?);
            let permissions = file.metadata()?.permissions();
            use std::os::unix::fs::PermissionsExt;
            let mut private_permissions = permissions.clone();
            private_permissions.set_mode(0o600);
            file.set_permissions(private_permissions)?;
            self.permissions_to_restore = Some(permissions);
        }
        #[cfg(not(unix))]
        {}
        Ok(())
    }

    pub fn restore_permissions_for_publish(&self) -> std::io::Result<()> {
        #[cfg(unix)]
        {
            if let Some(permissions) = &self.permissions_to_restore {
                let file = std::fs::File::open(&self.path)?;
                if Some(file_identity(&file)?) != self.restricted_file_identity {
                    return Err(std::io::Error::other(
                        "temporary export output changed before publication",
                    ));
                }
                file.set_permissions(permissions.clone())?;
            }
        }
        Ok(())
    }

    pub fn persist(&mut self) {
        self.keep = true;
    }
}

impl Drop for TemporaryOutputGuard {
    fn drop(&mut self) {
        if !self.keep {
            if let Err(error) = std::fs::remove_file(&self.path) {
                if error.kind() != std::io::ErrorKind::NotFound {
                    log::warn!(
                        "could not remove temporary export file {}: {error}",
                        self.path.display()
                    );
                }
            }
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

pub(crate) fn terminate_child_and_collect_stderr(
    guard: &mut ChildGuard,
    stderr_handle: std::thread::JoinHandle<String>,
) -> String {
    if let Some(mut child) = guard.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
    stderr_handle.join().unwrap_or_default()
}

/// Drain stderr from a child process in a background thread, returning the output.
pub(crate) fn drain_stderr(
    mut child: Child,
) -> std::io::Result<(Child, std::thread::JoinHandle<String>)> {
    let stderr = child.stderr.take();
    let handle = std::thread::Builder::new()
        .name("kagari-ffmpeg-stderr".into())
        .spawn(move || {
            let mut output = Vec::new();
            let mut truncated = false;
            if let Some(mut s) = stderr {
                use std::io::Read;

                let mut buffer = [0u8; 8192];
                loop {
                    let read = match s.read(&mut buffer) {
                        Ok(0) | Err(_) => break,
                        Ok(read) => read,
                    };
                    let remaining = MAX_FFMPEG_STDERR_BYTES.saturating_sub(output.len());
                    let keep = remaining.min(read);
                    output.extend_from_slice(&buffer[..keep]);
                    truncated |= keep < read;
                }
            }
            if truncated {
                output.extend_from_slice(b"\n[FFmpeg stderr truncated]");
            }
            String::from_utf8_lossy(&output).into_owned()
        });
    match handle {
        Ok(handle) => Ok((child, handle)),
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            Err(error)
        }
    }
}

/// Truncate diagnostic text at a UTF-8 character boundary.
pub(crate) fn truncate_diagnostic(text: &str, max_bytes: usize) -> &str {
    let mut end = text.len().min(max_bytes);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

/// Wait for FFmpeg to finish while still honoring cancellation after its input
/// pipe has closed and it is finalizing the output container.
pub(crate) fn wait_for_child_or_cancel(
    child: &mut Child,
    cancel_flag: &AtomicBool,
) -> std::io::Result<Option<std::process::ExitStatus>> {
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(Some(status)),
            Ok(None) => {}
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
        }
        if cancel_flag.load(Ordering::SeqCst) {
            let _ = child.kill();
            let _ = child.wait()?;
            return Ok(None);
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
}

/// Events sent from the export thread to the UI thread.
#[derive(Debug, Clone)]
pub enum ExportEvent {
    Progress(f32, String),
    Finished(String),
    Error(String),
}

pub(crate) fn export_error_or_cancellation(
    cancel_flag: &AtomicBool,
    error: impl Into<String>,
) -> String {
    if cancel_flag.load(Ordering::SeqCst) {
        "Export canceled by user".to_owned()
    } else {
        error.into()
    }
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
    /// Perceptual quality from 0 (lowest) to 100 (highest) for codecs that
    /// expose a quality setting.
    pub quality: u8,
    /// AAC bitrate in kilobits per second when an audio track is included.
    pub audio_bitrate_kbps: u32,
}

/// Check whether `ffmpeg` is discoverable on PATH or a supported macOS install path.
pub fn is_ffmpeg_available() -> bool {
    ffmpeg_command()
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
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        crate::core::software_renderer::with_render_cancel_flag(cancel_flag.clone(), || {
            render_frame_fn(frame_idx)
        })
    }));
    result.map_err(|_| ())
}

#[cfg(test)]
mod publish_tests;
