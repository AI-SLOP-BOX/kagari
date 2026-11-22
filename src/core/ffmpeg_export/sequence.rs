use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::mpsc::Sender;
use std::sync::Arc;

use super::types::{path_file_identity, publish_file_no_replace, FileIdentity};

struct StagingDirectoryGuard {
    path: std::path::PathBuf,
    preserve: bool,
}

impl Drop for StagingDirectoryGuard {
    fn drop(&mut self) {
        if !self.preserve {
            if let Err(error) = std::fs::remove_dir_all(&self.path) {
                if error.kind() != std::io::ErrorKind::NotFound {
                    log::warn!(
                        "could not remove temporary PNG sequence directory {}: {error}",
                        self.path.display()
                    );
                }
            }
        }
    }
}

static PNG_SEQUENCE_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn write_png_frame(
    path: &std::path::Path,
    pixels: &[u8],
    width: u32,
    height: u32,
    overwrite_existing: bool,
) -> Result<(), String> {
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    let expected_bytes = crate::core::software_renderer::rgba_buffer_size(width, height)
        .ok_or_else(|| format!("Invalid PNG dimensions: {width}x{height}"))?;
    if pixels.len() != expected_bytes {
        return Err(format!(
            "PNG pixel data mismatch: expected {expected_bytes} bytes, got {}",
            pixels.len()
        ));
    }
    let file_name = path
        .file_name()
        .ok_or_else(|| "PNG output path must include a filename".to_owned())?
        .to_string_lossy();
    let parent = path.parent().unwrap_or_else(|| std::path::Path::new("."));
    let temporary = parent.join(format!(
        ".{file_name}.kagari-tmp-{}-{}",
        std::process::id(),
        TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));

    let mut owns_temporary = false;
    let result = (|| {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| format!("cannot create temporary PNG: {error}"))?;
        owns_temporary = true;
        let mut writer = std::io::BufWriter::new(file);
        image::write_buffer_with_format(
            &mut writer,
            pixels,
            width,
            height,
            image::ColorType::Rgba8,
            image::ImageFormat::Png,
        )
        .map_err(|error| error.to_string())?;
        writer
            .flush()
            .map_err(|error| format!("cannot flush temporary PNG: {error}"))?;
        writer
            .get_ref()
            .sync_all()
            .map_err(|error| format!("cannot sync temporary PNG: {error}"))?;
        drop(writer);

        if overwrite_existing {
            super::types::replace_existing_file(&temporary, path)
                .map_err(|error| format!("cannot finalize PNG output: {error}"))?;
            owns_temporary = false;
        } else {
            publish_file_no_replace(&temporary, path).map_err(|error| {
                format!("cannot create PNG output without overwriting: {error}")
            })?;
            if std::fs::remove_file(&temporary).is_ok() {
                owns_temporary = false;
            }
        }
        Ok(())
    })();

    if owns_temporary {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

fn find_existing_png_frame(
    directory: &std::path::Path,
    stem: &str,
    first_frame: u32,
    last_frame: u32,
) -> Result<Option<std::path::PathBuf>, std::io::Error> {
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let Some(frame) = entry
            .file_name()
            .to_str()
            .and_then(|name| png_sequence_frame_index(name, stem))
        else {
            continue;
        };
        if (first_frame..=last_frame).contains(&frame) {
            let output = png_sequence_frame_path(directory, stem, frame);
            if std::fs::symlink_metadata(&output).is_ok() {
                return Ok(Some(output));
            }
        }
    }
    Ok(None)
}

fn find_existing_png_frame_numbers(
    directory: &std::path::Path,
    stem: &str,
    first_frame: u32,
    last_frame: u32,
) -> Result<HashMap<u32, FileIdentity>, std::io::Error> {
    let mut existing = HashMap::new();
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let Some(frame) = entry
            .file_name()
            .to_str()
            .and_then(|name| png_sequence_frame_index(name, stem))
        else {
            continue;
        };
        if (first_frame..=last_frame).contains(&frame) {
            let path = png_sequence_frame_path(directory, stem, frame);
            let metadata = std::fs::symlink_metadata(&path)?;
            if !metadata.file_type().is_file() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!(
                        "existing PNG sequence frame is not a regular file: {}",
                        path.display()
                    ),
                ));
            }
            existing.insert(frame, path_file_identity(&path)?);
        }
    }
    Ok(existing)
}

fn png_sequence_frame_path(
    directory: &std::path::Path,
    stem: &str,
    frame: u32,
) -> std::path::PathBuf {
    directory.join(format!("{stem}_{frame:04}.png"))
}

fn png_sequence_frame_index(file_name: &str, stem: &str) -> Option<u32> {
    let path = std::path::Path::new(file_name);
    if path.extension()?.to_str()? != "png" {
        return None;
    }
    let frame = path
        .file_stem()?
        .to_str()?
        .strip_prefix(&format!("{stem}_"))?
        .parse::<u32>()
        .ok()?;
    (format!("{stem}_{frame:04}.png") == file_name).then_some(frame)
}

fn rollback_png_publication(
    published: &[(std::path::PathBuf, FileIdentity)],
    backups: &[(std::path::PathBuf, std::path::PathBuf)],
    staging_guard: &mut StagingDirectoryGuard,
) -> Vec<String> {
    let mut failures = Vec::new();
    for (output, expected_identity) in published.iter().rev() {
        match path_file_identity(output) {
            Ok(identity) if identity == *expected_identity => {
                if let Err(error) = std::fs::remove_file(output) {
                    if error.kind() != std::io::ErrorKind::NotFound {
                        failures.push(format!("could not remove {}: {error}", output.display()));
                    }
                }
            }
            Ok(_) => failures.push(format!(
                "{} changed after publication and was left untouched",
                output.display()
            )),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if std::fs::symlink_metadata(output).is_ok() {
                    failures.push(format!(
                        "could not verify published output {} because its staged file is missing",
                        output.display()
                    ));
                }
            }
            Err(error) => failures.push(format!(
                "could not verify published output {}: {error}",
                output.display()
            )),
        }
    }
    for (backup, output) in backups.iter().rev() {
        match publish_file_no_replace(backup, output) {
            Ok(restored_identity) => {
                let restored_still_owned = std::fs::symlink_metadata(output)
                    .is_ok_and(|metadata| metadata.file_type().is_file())
                    && path_file_identity(output)
                        .is_ok_and(|identity| identity == restored_identity);
                if !restored_still_owned {
                    failures.push(format!(
                        "{} changed during restore; recovery copy retained at {}",
                        output.display(),
                        backup.display()
                    ));
                } else if let Err(error) = std::fs::remove_file(backup) {
                    failures.push(format!(
                        "restored {} but could not remove its recovery copy {}: {error}",
                        output.display(),
                        backup.display()
                    ));
                }
            }
            Err(error) => failures.push(format!(
                "could not restore {} from {}: {error}",
                output.display(),
                backup.display()
            )),
        }
    }
    if !failures.is_empty() {
        staging_guard.preserve = true;
    }
    failures
}

use super::types::{ExportEvent, RenderFrameFn};

/// Multi-comp parallel export using `ParallelRenderQueue`.
pub fn start_parallel_export(
    items: Vec<crate::core::parallel_render::RenderQueueItem>,
    tx: Sender<ExportEvent>,
    cancel_flag: Arc<std::sync::atomic::AtomicBool>,
    render_frame_fn: RenderFrameFn,
) -> Result<(), String> {
    if items.is_empty() {
        let _ = tx.send(ExportEvent::Error("No render queue items".to_string()));
        return Err("No items".to_string());
    }

    let total: u64 = items
        .iter()
        .filter(|i| i.start_frame <= i.end_frame)
        .map(|i| u64::from(i.end_frame) - u64::from(i.start_frame) + 1)
        .fold(0u64, u64::saturating_add);
    if total == 0 {
        let _ = tx.send(ExportEvent::Error("Total frames is zero".to_string()));
        return Err("Zero frames".to_string());
    }

    std::thread::Builder::new()
        .name("parallel_export".to_string())
        .spawn(move || {
            let mut queue = crate::core::parallel_render::ParallelRenderQueue::new();
            for item in items {
                queue.add_item(item);
            }
            let tx_clone = tx.clone();
            let total_frames = total;
            queue.set_progress_callback(move |_item_idx, done, _total| {
                let pct = done as f64 / total_frames.max(1) as f64;
                let _ = tx_clone.send(ExportEvent::Progress(
                    pct as f32,
                    format!("Parallel: {}/{} frames", done, total_frames),
                ));
            });

            // MFR on: rayon frames within and across items; MFR off: items in
            // parallel but each item's frames rendered sequentially.
            let render_result = if crate::core::parallel_render::mfr_enabled() {
                queue.render_all_mfr_with_external_cancel_checked(
                    &cancel_flag,
                    |comp_name, frame| render_frame_fn(comp_name, frame),
                )
            } else {
                queue.render_all_with_external_cancel_checked(&cancel_flag, |comp_name, frame| {
                    render_frame_fn(comp_name, frame)
                })
            };

            if let Err(failure) = render_result {
                let _ = tx.send(ExportEvent::Error(format!(
                    "Parallel export failed at item {} frame {}",
                    failure.item_index, failure.frame
                )));
            } else if cancel_flag.load(Ordering::SeqCst) || queue.is_cancelled() {
                let _ = tx.send(ExportEvent::Error("Parallel export canceled".to_string()));
            } else {
                let _ = tx.send(ExportEvent::Finished(format!(
                    "Parallel render complete: {} items, {} frames",
                    queue.items.len(),
                    total_frames
                )));
            }
        })
        .map_err(|e| format!("Failed to spawn parallel export thread: {}", e))?;

    Ok(())
}

/// PNG image-sequence export: renders every frame and writes
/// `{stem}_{frame:04}.png` into `dir` using the `image` crate.
#[allow(clippy::too_many_arguments)]
pub fn start_png_sequence_export<F>(
    dir: std::path::PathBuf,
    stem: String,
    width: u32,
    height: u32,
    total_frames: u32,
    first_index: u32,
    tx: Sender<ExportEvent>,
    cancel_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    render_frame: F,
) where
    F: FnMut(u32) -> Vec<u8> + Send + 'static,
{
    let _ = start_png_sequence_export_tracked(
        dir,
        stem,
        width,
        height,
        total_frames,
        first_index,
        tx,
        cancel_flag,
        render_frame,
    );
}

#[allow(clippy::too_many_arguments)]
pub fn start_png_sequence_export_tracked<F>(
    dir: std::path::PathBuf,
    stem: String,
    width: u32,
    height: u32,
    total_frames: u32,
    first_index: u32,
    tx: Sender<ExportEvent>,
    cancel_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    render_frame: F,
) -> Option<std::thread::JoinHandle<()>>
where
    F: FnMut(u32) -> Vec<u8> + Send + 'static,
{
    start_png_sequence_export_with_overwrite_tracked(
        dir,
        stem,
        width,
        height,
        total_frames,
        first_index,
        tx,
        cancel_flag,
        false,
        render_frame,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn start_png_sequence_export_with_overwrite<F>(
    dir: std::path::PathBuf,
    stem: String,
    width: u32,
    height: u32,
    total_frames: u32,
    first_index: u32,
    tx: Sender<ExportEvent>,
    cancel_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    overwrite_existing: bool,
    render_frame: F,
) where
    F: FnMut(u32) -> Vec<u8> + Send + 'static,
{
    let _ = start_png_sequence_export_with_overwrite_tracked(
        dir,
        stem,
        width,
        height,
        total_frames,
        first_index,
        tx,
        cancel_flag,
        overwrite_existing,
        render_frame,
    );
}

#[allow(clippy::too_many_arguments)]
pub fn start_png_sequence_export_with_overwrite_tracked<F>(
    dir: std::path::PathBuf,
    stem: String,
    width: u32,
    height: u32,
    total_frames: u32,
    first_index: u32,
    tx: Sender<ExportEvent>,
    cancel_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    overwrite_existing: bool,
    mut render_frame: F,
) -> Option<std::thread::JoinHandle<()>>
where
    F: FnMut(u32) -> Vec<u8> + Send + 'static,
{
    if stem.is_empty()
        || stem == "."
        || stem == ".."
        || stem.chars().any(|character| {
            character.is_control()
                || matches!(
                    character,
                    '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'
                )
        })
    {
        let _ = tx.send(ExportEvent::Error(
            "PNG sequence name contains invalid characters".into(),
        ));
        return None;
    }
    if total_frames == 0 {
        let _ = tx.send(ExportEvent::Error(
            "PNG sequence must contain at least one frame".into(),
        ));
        return None;
    }
    if first_index.checked_add(total_frames - 1).is_none() {
        let _ = tx.send(ExportEvent::Error(
            "PNG sequence frame range exceeds the supported index".into(),
        ));
        return None;
    }
    if crate::core::software_renderer::rgba_buffer_size(width, height).is_none() {
        let _ = tx.send(ExportEvent::Error(format!(
            "Invalid PNG dimensions: {width}x{height}"
        )));
        return None;
    }

    let spawn_error_tx = tx.clone();
    let worker = std::thread::Builder::new()
        .name("png_sequence_export".to_owned())
        .spawn(move || {
            if let Err(e) = std::fs::create_dir_all(&dir) {
                let _ = tx.send(ExportEvent::Error(format!(
                    "Cannot create dir {}: {}",
                    dir.display(),
                    e
                )));
                return;
            }
            let last_frame = first_index + total_frames - 1;
            let overwrite_frames = if overwrite_existing {
                match find_existing_png_frame_numbers(&dir, &stem, first_index, last_frame) {
                    Ok(existing) => existing,
                    Err(error) => {
                        let _ = tx.send(ExportEvent::Error(format!(
                            "Cannot inspect PNG sequence directory: {error}"
                        )));
                        return;
                    }
                }
            } else {
                match find_existing_png_frame(&dir, &stem, first_index, last_frame) {
                    Ok(Some(path)) => {
                        let _ = tx.send(ExportEvent::Error(format!(
                            "PNG sequence frame already exists: {}",
                            path.display()
                        )));
                        return;
                    }
                    Ok(None) => HashMap::new(),
                    Err(error) => {
                        let _ = tx.send(ExportEvent::Error(format!(
                            "Cannot inspect PNG sequence directory: {error}"
                        )));
                        return;
                    }
                }
            };
            let staging_dir = dir.join(format!(
                ".{stem}.kagari-sequence-{}-{}",
                std::process::id(),
                PNG_SEQUENCE_ID.fetch_add(1, Ordering::Relaxed)
            ));
            #[cfg(unix)]
            let mut staging_directory = std::fs::DirBuilder::new();
            #[cfg(not(unix))]
            let staging_directory = std::fs::DirBuilder::new();
            #[cfg(unix)]
            std::os::unix::fs::DirBuilderExt::mode(&mut staging_directory, 0o700);
            if let Err(error) = staging_directory.create(&staging_dir) {
                let _ = tx.send(ExportEvent::Error(format!(
                    "Cannot create temporary PNG sequence directory: {error}"
                )));
                return;
            }
            let mut staging_guard = StagingDirectoryGuard {
                path: staging_dir.clone(),
                preserve: false,
            };
            let last = total_frames.saturating_sub(1);
            for i in 0..total_frames {
                let f = i + first_index;
                if cancel_flag.load(Ordering::SeqCst) {
                    let _ = tx.send(ExportEvent::Error("Export canceled".to_string()));
                    return;
                }
                let pixels = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    render_frame(f)
                })) {
                    Ok(pixels) => pixels,
                    Err(_) => {
                        let _ = tx.send(ExportEvent::Error(format!(
                            "Render callback failed at frame {}",
                            f
                        )));
                        return;
                    }
                };
                let path = staging_dir.join(format!("{}_{:04}.png", stem, f));
                if let Err(e) = write_png_frame(&path, &pixels, width, height, true) {
                    let _ = tx.send(ExportEvent::Error(format!(
                        "Failed rendering PNG sequence frame {}: {}",
                        f, e
                    )));
                    return;
                }
                if i % 2 == 0 || i == last {
                    let _ = tx.send(ExportEvent::Progress(
                        (i + 1) as f32 / total_frames.max(1) as f32 * 0.9,
                        format!("Rendering frame {} / {}", i + 1, total_frames),
                    ));
                }
            }
            let mut published: Vec<(std::path::PathBuf, FileIdentity)> = Vec::new();
            let mut backups = Vec::new();
            let backup_dir = staging_dir.join(".originals");
            if !overwrite_frames.is_empty() {
                if let Err(error) = std::fs::create_dir(&backup_dir) {
                    let _ = tx.send(ExportEvent::Error(format!(
                        "Cannot create PNG sequence recovery directory: {error}"
                    )));
                    return;
                }
            }
            let publish_result = (|| -> Result<(), String> {
                for i in 0..total_frames {
                    let f = first_index + i;
                    if cancel_flag.load(Ordering::SeqCst) {
                        return Err("Export canceled".to_owned());
                    }
                    let staged = staging_dir.join(format!("{}_{:04}.png", stem, f));
                    let output = dir.join(format!("{}_{:04}.png", stem, f));
                    if let Some(expected_identity) = overwrite_frames.get(&f) {
                        let current_identity = path_file_identity(&output).map_err(|error| {
                            format!("Cannot verify existing PNG sequence frame {}: {error}", output.display())
                        })?;
                        if current_identity != *expected_identity {
                            return Err(format!(
                                "Existing PNG sequence frame changed during export; leaving it untouched: {}",
                                output.display()
                            ));
                        }
                        let backup = backup_dir.join(format!("{f:010}.png"));
                        std::fs::rename(&output, &backup).map_err(|error| {
                            format!(
                                "Cannot preserve existing PNG sequence frame {}: {error}",
                                output.display()
                            )
                        })?;
                        let backup_matches = matches!(
                            path_file_identity(&backup),
                            Ok(identity) if identity == *expected_identity
                        );
                        if !backup_matches {
                            staging_guard.preserve = true;
                            return Err(format!(
                                "Existing PNG sequence frame changed during replacement; recovery copy remains at {}",
                                backup.display()
                            ));
                        }
                        backups.push((backup, output.clone()));
                    }
                    let identity = publish_file_no_replace(&staged, &output).map_err(|error| {
                        format!(
                            "Cannot publish PNG sequence frame {}: {error}",
                            output.display()
                        )
                    })?;
                    published.push((output, identity));
                    if i % 16 == 15 || i + 1 == total_frames {
                        let _ = tx.send(ExportEvent::Progress(
                            0.9 + (i + 1) as f32 / total_frames as f32 * 0.1,
                            format!("Publishing frame {} / {}", i + 1, total_frames),
                        ));
                    }
                }
                if cancel_flag.load(Ordering::SeqCst) {
                    return Err("Export canceled".to_owned());
                }
                Ok(())
            })();
            if let Err(error) = publish_result {
                let rollback_failures =
                    rollback_png_publication(&published, &backups, &mut staging_guard);
                let message = if rollback_failures.is_empty() {
                    error
                } else {
                    format!(
                        "{error}. Rollback was incomplete: {}. Recovery data remains at {}",
                        rollback_failures.join("; "),
                        staging_dir.display()
                    )
                };
                let _ = tx.send(ExportEvent::Error(message));
                return;
            }
            let _ = tx.send(ExportEvent::Finished(format!(
                "PNG sequence exported: {} frames → {}",
                total_frames,
                dir.join(format!("{}_", stem)).display()
            )));
        });
    match worker {
        Ok(handle) => Some(handle),
        Err(error) => {
            let _ = spawn_error_tx.send(ExportEvent::Error(format!(
                "Cannot start PNG sequence export: {error}"
            )));
            None
        }
    }
}
