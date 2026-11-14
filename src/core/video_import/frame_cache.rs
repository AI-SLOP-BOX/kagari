use super::*;

mod fingerprint;
use fingerprint::{frame_cache_fingerprint, image_sequence_source_fingerprint, source_fingerprint};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct SourceFingerprint {
    canonical_path: String,
    size_bytes: u64,
    modified_unix_nanos: u128,
    #[serde(default)]
    media_revision: u64,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct FrameCacheManifest {
    version: u32,
    #[serde(default)]
    media_kind: String,
    frame_count: u32,
    width: u32,
    height: u32,
    total_bytes: u64,
    content_hash: u64,
    /// Rate of the extracted frame sequence. Zero denotes a legacy cache.
    #[serde(default)]
    extraction_fps: f32,
    #[serde(default)]
    source_fingerprint: Option<SourceFingerprint>,
}

static FRAME_CACHE_FPS: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashMap<String, f32>>,
> = std::sync::OnceLock::new();
const FRAME_CACHE_RATE_FILE: &str = ".kagari-frame-rate-v1.json";

#[derive(serde::Serialize, serde::Deserialize)]
struct FrameCacheRate {
    version: u32,
    fps: f32,
}

/// Returns the extraction rate embedded in an imported frame cache. Older
/// manifests are upgraded with the composition rate on first use.
pub(super) fn extraction_fps(frames_dir: &str, fallback_fps: u32) -> f32 {
    let cache = FRAME_CACHE_FPS.get_or_init(Default::default);
    if let Ok(cache) = cache.lock() {
        if let Some(fps) = cache.get(frames_dir) {
            return *fps;
        }
    }
    let manifest_path = Path::new(frames_dir).join(FRAME_CACHE_MANIFEST_FILE);
    let manifest = crate::core::project_migration::read_bounded_text_file(&manifest_path, 4096)
        .ok()
        .and_then(|json| serde_json::from_str::<FrameCacheManifest>(&json).ok());
    let fallback_fps = fallback_fps.clamp(1, MAX_IMPORT_FPS as u32) as f32;
    let stored_fps = manifest
        .as_ref()
        .map(|manifest| manifest.extraction_fps)
        .filter(|fps| valid_extraction_fps(*fps))
        .or_else(|| read_frame_cache_rate(frames_dir));
    let fps = stored_fps.unwrap_or(fallback_fps);
    if let Some(mut manifest) = manifest {
        if stored_fps.is_none() && manifest.version == 1 && manifest.frame_count > 0 {
            manifest.extraction_fps = fps;
            let _ = write_frame_cache_manifest_with_source(
                Path::new(frames_dir),
                manifest.frame_count,
                manifest.width,
                manifest.height,
                manifest.source_fingerprint.clone(),
                &manifest.media_kind,
                fps,
                None,
            );
        }
    } else if stored_fps.is_none() && frame_path_in_dir(frames_dir, 0).is_file() {
        let _ = write_frame_cache_rate(frames_dir, fps);
    }
    if let Ok(mut cache) = cache.lock() {
        if cache.len() >= 256 {
            if let Some(oldest_key) = cache.keys().next().cloned() {
                cache.remove(&oldest_key);
            }
        }
        cache.insert(frames_dir.to_owned(), fps);
    }
    fps
}

fn read_frame_cache_rate(frames_dir: &str) -> Option<f32> {
    let path = Path::new(frames_dir).join(FRAME_CACHE_RATE_FILE);
    let json = crate::core::project_migration::read_bounded_text_file(&path, 256).ok()?;
    let rate = serde_json::from_str::<FrameCacheRate>(&json).ok()?;
    (rate.version == 1 && valid_extraction_fps(rate.fps)).then_some(rate.fps)
}

fn valid_extraction_fps(fps: f32) -> bool {
    fps.is_finite() && (1.0..=MAX_IMPORT_FPS).contains(&fps)
}

fn write_frame_cache_rate(frames_dir: &str, fps: f32) -> io::Result<()> {
    let rate = FrameCacheRate { version: 1, fps };
    let bytes = serde_json::to_vec(&rate)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let frames_dir = Path::new(frames_dir);
    for _ in 0..100 {
        let temporary = frames_dir.join(format!(
            "{FRAME_CACHE_RATE_FILE}.tmp.{}.{}",
            std::process::id(),
            FRAME_CACHE_MANIFEST_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let mut file = match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
        {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        };
        let result = (|| {
            use io::Write;
            file.write_all(&bytes)?;
            file.sync_all()?;
            drop(file);
            crate::core::atomic_file::replace_file(
                &temporary,
                &frames_dir.join(FRAME_CACHE_RATE_FILE),
            )?;
            remember_frame_cache_fps(&frames_dir.to_string_lossy(), fps);
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary);
        }
        return result;
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not reserve a unique frame-cache rate file after 100 attempts",
    ))
}

fn remember_frame_cache_fps(frames_dir: &str, fps: f32) {
    if !valid_extraction_fps(fps) {
        return;
    }
    let cache = FRAME_CACHE_FPS.get_or_init(Default::default);
    if let Ok(mut cache) = cache.lock() {
        if cache.len() >= 256 && !cache.contains_key(frames_dir) {
            if let Some(oldest_key) = cache.keys().next().cloned() {
                cache.remove(&oldest_key);
            }
        }
        cache.insert(frames_dir.to_owned(), fps);
    }
}

fn forget_frame_cache_fps(frames_dir: &str) {
    if let Some(cache) = FRAME_CACHE_FPS.get() {
        if let Ok(mut cache) = cache.lock() {
            cache.remove(frames_dir);
        }
    }
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
    let mut cache_validity: HashMap<(String, String, u32), bool> = HashMap::new();
    let mut refreshed = 0usize;
    let mut errors = Vec::new();
    for comp in &mut project.compositions {
        refresh_missing_video_caches_in_comp(
            comp,
            destination_root,
            &mut cache,
            &mut cache_validity,
            &mut refreshed,
            &mut errors,
        );
    }
    (refreshed, errors)
}

pub(super) fn is_image_sequence_cache(frames_dir: &str, source: &str) -> bool {
    let manifest_path = Path::new(frames_dir).join(FRAME_CACHE_MANIFEST_FILE);
    if let Ok(json) = crate::core::project_migration::read_bounded_text_file(&manifest_path, 4096) {
        if let Ok(manifest) = serde_json::from_str::<FrameCacheManifest>(&json) {
            if manifest.media_kind == "image_sequence" {
                return true;
            }
            if manifest.media_kind == "video" {
                return false;
            }
        }
    }

    // Legacy caches did not record their media kind. Numbered still sources
    // distinguish image sequences from decoded video frame directories.
    is_numbered_still_sequence(source)
}

fn refresh_missing_video_caches_in_comp(
    comp: &mut crate::core::timeline::Composition,
    destination_root: &Path,
    cache: &mut std::collections::HashMap<(String, u32), VideoAsset>,
    cache_validity: &mut std::collections::HashMap<(String, String, u32), bool>,
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

        let validity_key = (frames_dir.clone(), source.clone(), frame_count);
        let cache_is_valid = *cache_validity.entry(validity_key).or_insert_with(|| {
            video_frame_cache_is_complete(&frames_dir, frame_count)
                && source_matches_frame_cache(Path::new(&frames_dir), &source)
        });
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
            let import = if is_numbered_still_sequence(&source) {
                import_image_sequence(Path::new(&source), &destination, comp.fps.max(1) as f32)
            } else {
                import_video(&source, &destination, comp.fps.max(1) as f32)
            };
            match import {
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
        if let Some(warning) = asset.audio_warning.as_deref() {
            errors.push(format!("{}: {}", source, warning));
        }

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
        refresh_missing_video_caches_in_comp(
            sub,
            destination_root,
            cache,
            cache_validity,
            refreshed,
            errors,
        );
    }
}

fn is_numbered_still_sequence(source: &str) -> bool {
    let path = Path::new(source);
    let has_sequence_number = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .is_some_and(|stem| {
            let digit_count = stem
                .as_bytes()
                .iter()
                .rev()
                .take_while(|byte| byte.is_ascii_digit())
                .count();
            digit_count > 0 && digit_count < stem.len()
        });
    has_sequence_number
        && path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                matches!(
                    extension.to_ascii_lowercase().as_str(),
                    "png" | "jpg" | "jpeg" | "webp" | "tif" | "tiff" | "bmp"
                )
            })
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
            frame_cache_fingerprint(frames_dir, frame_count, None).is_some_and(|(total, hash)| {
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

fn source_matches_frame_cache(frames_dir: &Path, source: &str) -> bool {
    match std::fs::metadata(source) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            // A complete frame cache remains usable when the original media is offline.
            return true;
        }
        Err(_) => return false,
        Ok(_) => {}
    }
    let source_path = Path::new(source);
    let current_source = if is_numbered_still_sequence(source) {
        image_sequence_source_fingerprint(source_path, None)
    } else {
        source_fingerprint(source_path)
    };
    let Some(current_source) = current_source else {
        return false;
    };
    let manifest_path = frames_dir.join(FRAME_CACHE_MANIFEST_FILE);
    let Ok(json) = crate::core::project_migration::read_bounded_text_file(&manifest_path, 4096)
    else {
        return false;
    };
    serde_json::from_str::<FrameCacheManifest>(&json)
        .ok()
        .and_then(|manifest| manifest.source_fingerprint)
        .is_some_and(|cached_source| {
            cached_source.canonical_path == current_source.canonical_path
                && cached_source.size_bytes == current_source.size_bytes
                && cached_source.modified_unix_nanos == current_source.modified_unix_nanos
                && cached_source.media_revision == current_source.media_revision
        })
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
        super::process::decode_import_image(&path).ok()?;
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
    write_frame_cache_manifest_with_source(
        frames_dir,
        frame_count,
        width,
        height,
        None,
        "image_sequence",
        0.0,
        None,
    )
}

pub(super) fn write_image_sequence_frame_cache_manifest(
    frames_dir: &Path,
    frame_count: u32,
    width: u32,
    height: u32,
    source: &Path,
    extraction_fps: f32,
    cancel: &AtomicBool,
) -> io::Result<()> {
    let source_fingerprint =
        image_sequence_source_fingerprint(source, Some(cancel)).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "could not fingerprint image sequence sources",
            )
        })?;
    write_frame_cache_manifest_with_source(
        frames_dir,
        frame_count,
        width,
        height,
        Some(source_fingerprint),
        "image_sequence",
        extraction_fps,
        Some(cancel),
    )
}

pub(super) fn write_video_frame_cache_manifest(
    frames_dir: &Path,
    frame_count: u32,
    width: u32,
    height: u32,
    source: &Path,
    extraction_fps: f32,
    cancel: &AtomicBool,
) -> io::Result<()> {
    let source_fingerprint = source_fingerprint(source);
    write_frame_cache_manifest_with_source(
        frames_dir,
        frame_count,
        width,
        height,
        source_fingerprint,
        "video",
        extraction_fps,
        Some(cancel),
    )
}

#[allow(clippy::too_many_arguments)]
fn write_frame_cache_manifest_with_source(
    frames_dir: &Path,
    frame_count: u32,
    width: u32,
    height: u32,
    source_fingerprint: Option<SourceFingerprint>,
    media_kind: &str,
    extraction_fps: f32,
    cancel: Option<&AtomicBool>,
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
    let (total_bytes, content_hash) = frame_cache_fingerprint(frames_dir, frame_count, cancel)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "frame cache is incomplete"))?;
    let stored_extraction_fps = if valid_extraction_fps(extraction_fps) {
        extraction_fps
    } else {
        0.0
    };
    let manifest = FrameCacheManifest {
        version: 1,
        media_kind: media_kind.to_owned(),
        frame_count,
        width,
        height,
        total_bytes,
        content_hash,
        extraction_fps: stored_extraction_fps,
        source_fingerprint,
    };
    let bytes = serde_json::to_vec(&manifest)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    for _ in 0..100 {
        let temporary = frames_dir.join(format!(
            "{FRAME_CACHE_MANIFEST_FILE}.tmp.{}.{}",
            std::process::id(),
            FRAME_CACHE_MANIFEST_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let mut file = match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
        {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        };
        let result = (|| {
            use io::Write;
            file.write_all(&bytes)?;
            file.sync_all()?;
            drop(file);
            crate::core::atomic_file::replace_file(
                &temporary,
                &frames_dir.join(FRAME_CACHE_MANIFEST_FILE),
            )?;
            if valid_extraction_fps(stored_extraction_fps) {
                remember_frame_cache_fps(&frames_dir.to_string_lossy(), stored_extraction_fps);
            } else {
                forget_frame_cache_fps(&frames_dir.to_string_lossy());
            }
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary);
        }
        return result;
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not reserve a unique frame-cache manifest after 100 attempts",
    ))
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
