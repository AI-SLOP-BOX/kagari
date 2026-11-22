use crate::core::timeline::Composition;
use std::hash::{Hash, Hasher};

pub(crate) const MAX_COMPOSITION_NESTING_DEPTH: usize = 128;

type MediaRevisionCache = std::sync::Mutex<
    std::collections::HashMap<std::path::PathBuf, (u64, bool, std::time::Instant)>,
>;

fn media_revision_cache() -> &'static MediaRevisionCache {
    static REVISION_CACHE: std::sync::OnceLock<MediaRevisionCache> = std::sync::OnceLock::new();
    REVISION_CACHE.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

fn cached_media_file_revision(path: &std::path::Path) -> (u64, bool) {
    let cache_mutex = media_revision_cache();
    let now = std::time::Instant::now();
    if let Ok(cache) = cache_mutex.lock() {
        if let Some((revision, exists, timestamp)) = cache.get(path) {
            if now.duration_since(*timestamp).as_millis() < 1000 {
                return (*revision, *exists);
            }
        }
    }

    let (revision, exists) = media_file_revision_uncached_with_exists(path);

    if let Ok(mut cache) = cache_mutex.lock() {
        if cache.len() > 1000 {
            cache.clear();
        }
        cache.insert(path.to_path_buf(), (revision, exists, now));
    }

    (revision, exists)
}

pub fn media_file_revision(path: &std::path::Path) -> u64 {
    cached_media_file_revision(path).0
}

pub(crate) fn existing_media_file_revision(path: &std::path::Path) -> Option<u64> {
    let (revision, exists) = cached_media_file_revision(path);
    exists.then_some(revision)
}

#[cfg(test)]
pub(crate) fn existing_media_file_revision_uncached(path: &std::path::Path) -> Option<u64> {
    let (revision, exists) = media_file_revision_uncached_with_exists(path);
    exists.then_some(revision)
}

fn media_file_revision_uncached_with_exists(path: &std::path::Path) -> (u64, bool) {
    match std::fs::metadata(path) {
        Ok(metadata) => (media_file_revision_from_metadata(path, &metadata), true),
        Err(error) => {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            path.hash(&mut hasher);
            error.kind().hash(&mut hasher);
            (hasher.finish(), false)
        }
    }
}

pub(crate) fn media_file_revision_from_metadata(
    path: &std::path::Path,
    metadata: &std::fs::Metadata,
) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut hasher);
    metadata.len().hash(&mut hasher);
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos())
        .hash(&mut hasher);
    metadata
        .created()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos())
        .hash(&mut hasher);
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        metadata.dev().hash(&mut hasher);
        metadata.ino().hash(&mut hasher);
        metadata.ctime().hash(&mut hasher);
        metadata.ctime_nsec().hash(&mut hasher);
    }
    hasher.finish()
}

pub fn composition_media_revision(comp: &Composition, frame: u32) -> u64 {
    fn proxy_path(
        layer: &crate::core::timeline::Layer,
        sequence_frame: u32,
    ) -> Option<std::path::PathBuf> {
        if !layer.proxy.enabled {
            return None;
        }
        let raw_path = layer.proxy.proxy_path.as_deref()?;
        let path = std::path::Path::new(raw_path);
        if path.is_file() {
            return Some(path.to_path_buf());
        }
        if path.is_dir() {
            let frame = if matches!(
                layer.layer_type,
                crate::core::timeline::LayerType::Video { .. }
            ) {
                crate::core::video_import::frame_path_in_dir(raw_path, sequence_frame)
            } else {
                crate::core::video_import::frame_path_in_dir(raw_path, 0)
            };
            if frame.is_file() {
                return Some(frame);
            }
        }
        None
    }

    fn hash_media(
        comp: &Composition,
        frame: u32,
        depth: usize,
        visited: &mut std::collections::HashSet<String>,
        hasher: &mut std::collections::hash_map::DefaultHasher,
    ) {
        if depth > MAX_COMPOSITION_NESTING_DEPTH {
            0x6d65_6469_615f_6465u64.hash(hasher);
            return;
        }
        if !visited.insert(comp.id.clone()) {
            return;
        }
        let has_solo = comp
            .layers
            .iter()
            .any(|layer| layer.is_active(frame) && layer.solo);
        for layer in &comp.layers {
            if !layer.is_active(frame) || (has_solo && !layer.solo) {
                continue;
            }
            match &layer.layer_type {
                crate::core::timeline::LayerType::Image { path } => {
                    media_file_revision(std::path::Path::new(path)).hash(hasher);
                    if let Some(proxy) = proxy_path(layer, 0) {
                        media_file_revision(&proxy).hash(hasher);
                    }
                }
                crate::core::timeline::LayerType::Video {
                    frames_dir,
                    frame_count,
                    speed,
                    ..
                } => {
                    let source_frame = match &layer.posterize_time {
                        Some(settings) if settings.enabled => {
                            layer.effective_render_frame(frame, comp.fps) as f32
                        }
                        _ => layer.remap_frame_f32(frame),
                    };
                    let source_position = crate::core::timeline::video_sequence_frame(
                        source_frame,
                        *speed,
                        frames_dir,
                        comp.fps,
                    );
                    let last_frame = frame_count.saturating_sub(1);
                    let first_frame = (source_position.floor() as u32).min(last_frame);
                    let proxy = proxy_path(layer, first_frame);
                    let first_path = proxy.clone().unwrap_or_else(|| {
                        crate::core::video_import::frame_path_in_dir(frames_dir, first_frame)
                    });
                    media_file_revision(&first_path).hash(hasher);
                    if proxy.is_none()
                        && layer.frame_blending
                        && first_frame < last_frame
                        && source_position.fract() > f32::EPSILON
                    {
                        let second_path = crate::core::video_import::frame_path_in_dir(
                            frames_dir,
                            first_frame + 1,
                        );
                        media_file_revision(&second_path).hash(hasher);
                    }
                }
                crate::core::timeline::LayerType::Model3D { path } => {
                    media_file_revision(std::path::Path::new(path)).hash(hasher);
                }
                crate::core::timeline::LayerType::PreComp { comp_id } => {
                    if let Some(sub_comp) = comp
                        .sub_compositions
                        .iter()
                        .find(|candidate| candidate.id == *comp_id)
                    {
                        hash_media(
                            sub_comp,
                            layer.effective_render_frame(frame, comp.fps),
                            depth + 1,
                            visited,
                            hasher,
                        );
                    }
                }
                _ => {}
            }
        }
        visited.remove(&comp.id);
    }

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hash_media(
        comp,
        frame,
        0,
        &mut std::collections::HashSet::new(),
        &mut hasher,
    );
    hasher.finish()
}
