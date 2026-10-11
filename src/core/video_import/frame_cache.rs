use super::*;

#[derive(serde::Serialize, serde::Deserialize)]
struct FrameCacheManifest {
    version: u32,
    frame_count: u32,
    width: u32,
    height: u32,
    total_bytes: u64,
    content_hash: u64,
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

pub(super) fn video_frame_cache_is_complete(frames_dir: &str, frame_count: u32) -> bool {
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

pub(super) fn write_frame_cache_manifest(
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
