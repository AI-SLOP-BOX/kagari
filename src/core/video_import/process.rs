use super::*;
use std::io;
use std::process::{Command, ExitStatus, Output, Stdio};
use std::sync::atomic::Ordering;
use std::thread;
use std::time::{Duration, Instant};

pub(super) fn create_private_import_directory(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        let mut builder = std::fs::DirBuilder::new();
        builder.mode(0o700);
        builder.create(path)
    }
    #[cfg(not(unix))]
    {
        std::fs::create_dir(path)
    }
}

pub(super) fn dimensions_within_import_budget(width: u32, height: u32) -> bool {
    if width < 2 || height < 2 || width > MAX_IMPORT_DIMENSION || height > MAX_IMPORT_DIMENSION {
        return false;
    }
    u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .is_some_and(|bytes| bytes <= MAX_IMPORT_DECODED_BYTES)
}

pub(super) fn decode_import_image(path: &Path) -> Result<image::DynamicImage, String> {
    let reader =
        image::ImageReader::open(path).map_err(|error| format!("could not open image: {error}"))?;
    let dimensions = reader
        .into_dimensions()
        .map_err(|error| format!("could not read image dimensions: {error}"))?;
    if !dimensions_within_import_budget(dimensions.0, dimensions.1) {
        return Err("image dimensions exceed the import decode limit".to_owned());
    }

    let mut reader = image::ImageReader::open(path)
        .map_err(|error| format!("could not reopen image: {error}"))?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_IMPORT_DIMENSION);
    limits.max_image_height = Some(MAX_IMPORT_DIMENSION);
    limits.max_alloc = Some(MAX_IMPORT_DECODED_BYTES);
    reader.limits(limits);
    let decoded = reader
        .decode()
        .map_err(|error| format!("could not decode image within import limits: {error}"))?;
    if (decoded.width(), decoded.height()) != dimensions {
        return Err("image dimensions changed during decoding".to_owned());
    }
    Ok(decoded)
}

pub(super) struct FfmpegOutput {
    pub(super) status: ExitStatus,
    pub(super) stderr: String,
}

pub(super) struct ImportSafety<'a> {
    pub(super) cancel: &'a AtomicBool,
    pub(super) disk_path: &'a Path,
    pub(super) minimum_free_bytes: u64,
}

pub(super) fn run_cancellable_probe(
    command: &mut Command,
    cancel: &AtomicBool,
) -> io::Result<Output> {
    const MAX_PROBE_OUTPUT_BYTES: usize = 64 * 1024;
    const MAX_PROBE_DURATION: Duration = Duration::from_secs(120);
    if cancel.load(Ordering::Relaxed) {
        return Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "video import cancelled",
        ));
    }
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::other("FFprobe stdout pipe was not available"));
        }
    };
    let reader = match thread::Builder::new()
        .name("kagari-ffprobe-output".into())
        .spawn(move || -> io::Result<Vec<u8>> {
            let mut stdout = stdout;
            let mut output = Vec::with_capacity(MAX_PROBE_OUTPUT_BYTES);
            let mut buffer = [0u8; 4096];
            loop {
                let count = io::Read::read(&mut stdout, &mut buffer)?;
                if count == 0 {
                    break;
                }
                let keep = MAX_PROBE_OUTPUT_BYTES
                    .saturating_sub(output.len())
                    .min(count);
                output.extend_from_slice(&buffer[..keep]);
            }
            Ok(output)
        }) {
        Ok(reader) => reader,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
    };
    let started = Instant::now();
    let status = loop {
        if cancel.load(Ordering::Relaxed) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = reader.join();
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "video import cancelled",
            ));
        }
        if started.elapsed() >= MAX_PROBE_DURATION {
            let _ = child.kill();
            let _ = child.wait();
            let _ = reader.join();
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "media metadata probe exceeded the 120 second time limit",
            ));
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => thread::sleep(Duration::from_millis(25)),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = reader.join();
                return Err(error);
            }
        }
    };
    let stdout_bytes = reader
        .join()
        .map_err(|_| io::Error::other("FFprobe output reader thread panicked"))??;
    Ok(Output {
        status,
        stdout: stdout_bytes,
        stderr: Vec::new(),
    })
}

pub(super) fn run_ffmpeg(
    command: &mut Command,
    safety: Option<&ImportSafety<'_>>,
) -> io::Result<FfmpegOutput> {
    const TRUNCATED: &[u8] = b"\n[FFmpeg diagnostics truncated]";
    let mut child = command
        .stdin(Stdio::null())
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
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = reader.join();
                return Err(error);
            }
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

pub(crate) fn available_disk_space(path: &Path) -> Option<u64> {
    let existing_path = path.ancestors().find(|candidate| candidate.exists())?;
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let path = std::ffi::CString::new(existing_path.as_os_str().as_bytes()).ok()?;
        let mut stats: libc::statvfs = unsafe { std::mem::zeroed() };
        if unsafe { libc::statvfs(path.as_ptr(), &mut stats) } != 0 {
            return None;
        }
        // POSIX exposes these fields as different unsigned widths across targets.
        #[allow(clippy::useless_conversion)]
        let available_blocks = u64::from(stats.f_bavail);
        #[allow(clippy::useless_conversion)]
        let fragment_size = u64::from(stats.f_frsize);
        available_blocks.checked_mul(fragment_size)
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

pub(crate) fn estimate_import_disk_bytes(
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

pub(super) fn format_disk_bytes(bytes: u64) -> String {
    const GIB: u64 = 1024 * 1024 * 1024;
    if bytes >= GIB {
        format!("{:.1} GiB", bytes as f64 / GIB as f64)
    } else {
        format!("{:.0} MiB", bytes as f64 / (1024 * 1024) as f64)
    }
}

pub(super) struct ImportDirectoryGuard {
    pub(super) path: PathBuf,
    pub(super) keep: bool,
}

impl Drop for ImportDirectoryGuard {
    fn drop(&mut self) {
        if !self.keep {
            if let Err(error) = std::fs::remove_dir_all(&self.path) {
                if error.kind() != io::ErrorKind::NotFound {
                    log::warn!(
                        "[VideoImport] Failed to remove temporary media directory {}: {error}",
                        self.path.display()
                    );
                }
            }
        }
    }
}
