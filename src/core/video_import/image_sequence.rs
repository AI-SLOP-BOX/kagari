use super::*;

pub fn import_image_sequence(
    src_path: &Path,
    dest_dir: &Path,
    fps: f32,
) -> Result<VideoAsset, String> {
    let cancel = std::sync::atomic::AtomicBool::new(false);
    import_image_sequence_with_cancel(src_path, dest_dir, fps, &cancel)
}

pub fn import_image_sequence_with_cancel(
    src_path: &Path,
    dest_dir: &Path,
    fps: f32,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<VideoAsset, String> {
    if cancel.load(std::sync::atomic::Ordering::Relaxed) {
        return Err("image sequence import cancelled".to_owned());
    }
    if !fps.is_finite() || fps < 1.0 || fps > MAX_IMPORT_FPS {
        return Err(format!(
            "fps must be finite and between 1 and {}",
            MAX_IMPORT_FPS
        ));
    }
    if !src_path.is_file() {
        return Err(format!("source file not found: {}", src_path.display()));
    }
    let source_filename = src_path
        .file_name()
        .ok_or_else(|| "image sequence source has no valid filename".to_string())?;
    let extension = src_path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| "image sequence source has no extension".to_string())?;
    if !matches!(
        extension.as_str(),
        "png" | "jpg" | "jpeg" | "webp" | "tif" | "tiff" | "bmp"
    ) {
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

    let mut frames: Vec<(u32, PathBuf)> = Vec::new();
    for entry in std::fs::read_dir(parent)
        .map_err(|error| format!("could not scan image sequence directory: {error}"))?
    {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            return Err("image sequence import cancelled".to_owned());
        }
        let entry = entry.map_err(|error| {
            format!("could not finish scanning image sequence directory: {error}")
        })?;
        let path = entry.path();
        let Some(ext) = path.extension().and_then(|value| value.to_str()) else {
            continue;
        };
        if !ext.eq_ignore_ascii_case(&extension) {
            continue;
        }
        let Some(candidate_stem) = path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };
        let Some(suffix) = candidate_stem.strip_prefix(prefix) else {
            continue;
        };
        let Ok(frame) = suffix.parse::<u32>() else {
            continue;
        };
        if frames.len() >= MAX_IMPORT_FRAMES as usize {
            return Err(format!(
                "image sequence contains too many frames (limit {})",
                MAX_IMPORT_FRAMES
            ));
        }
        frames
            .try_reserve(1)
            .map_err(|error| format!("not enough memory to scan image sequence: {error}"))?;
        frames.push((frame, path));
    }
    if cancel.load(std::sync::atomic::Ordering::Relaxed) {
        return Err("image sequence import cancelled".to_owned());
    }
    frames.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    if frames.windows(2).any(|pair| pair[0].0 == pair[1].0) {
        return Err("image sequence contains duplicate frame numbers".to_string());
    }
    if frames.is_empty()
        || !frames
            .iter()
            .any(|(_, path)| path.file_name() == Some(source_filename))
    {
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
    let (width, height) = image::image_dimensions(&frames[0].1)
        .map_err(|error| format!("could not read image sequence dimensions: {error}"))?;
    if !dimensions_within_import_budget(width, height) {
        return Err(format!(
            "image sequence dimensions exceed the import limit (maximum side {} px and decoded RGBA size {} MiB)",
            MAX_IMPORT_DIMENSION,
            MAX_IMPORT_DECODED_BYTES / (1024 * 1024)
        ));
    }

    let estimated_disk_bytes = estimate_import_disk_bytes(
        width,
        height,
        frames.len() as u64,
        frames.len() as f64 / f64::from(fps),
        false,
    )
    .ok_or_else(|| "could not estimate image sequence disk usage".to_owned())?;
    let minimum_free_bytes = MIN_IMPORT_FREE_BYTES.max(
        estimated_disk_bytes.saturating_mul(IMPORT_DISK_HEADROOM_NUMERATOR)
            / IMPORT_DISK_HEADROOM_DENOMINATOR,
    );
    let available_bytes = available_disk_space(dest_dir).ok_or_else(|| {
        "could not determine available disk space for image sequence import".to_owned()
    })?;
    if available_bytes < estimated_disk_bytes.saturating_add(minimum_free_bytes) {
        return Err(format!(
            "image sequence import is estimated to need {} and preserve {} free, but only {} is available",
            format_disk_bytes(estimated_disk_bytes),
            format_disk_bytes(minimum_free_bytes),
            format_disk_bytes(available_bytes),
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
    let mut last_space_check = std::time::Instant::now();
    for (output_index, (_, source)) in frames.iter().enumerate() {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            return Err("image sequence import cancelled".to_owned());
        }
        if last_space_check.elapsed() >= std::time::Duration::from_millis(500) {
            last_space_check = std::time::Instant::now();
            if available_disk_space(dest_dir).is_none_or(|available| available < minimum_free_bytes)
            {
                return Err(
                    "image sequence import stopped to preserve the minimum free disk space"
                        .to_owned(),
                );
            }
        }
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
        let image = match decode_import_image(source) {
            Ok(image) => image,
            Err(error) => {
                return Err(format!(
                    "could not decode sequence frame {}: {error}",
                    source.display()
                ));
            }
        };
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            return Err("image sequence import cancelled".to_owned());
        }
        let target = frames_dir.join(format!("frame_{output_index:05}.webp"));
        if let Err(error) = image.save_with_format(&target, image::ImageFormat::WebP) {
            return Err(format!(
                "could not encode sequence frame {}: {error}",
                source.display()
            ));
        }
    }
    if cancel.load(std::sync::atomic::Ordering::Relaxed) {
        return Err("image sequence import cancelled".to_owned());
    }
    write_image_sequence_frame_cache_manifest(
        &frames_dir,
        frames.len() as u32,
        width,
        height,
        src_path,
        fps,
        cancel,
    )
    .map_err(|error| format!("could not verify imported image sequence: {error}"))?;
    if cancel.load(std::sync::atomic::Ordering::Relaxed) {
        return Err("image sequence import cancelled".to_owned());
    }

    let asset = VideoAsset {
        source_path: src_path.to_string_lossy().into_owned(),
        frames_dir: frames_dir.to_string_lossy().into_owned(),
        frame_count: frames.len() as u32,
        fps,
        width,
        height,
        audio_wav: None,
        audio_warning: None,
    };
    import_dir_guard.keep = true;
    Ok(asset)
}
