use crate::core::timeline::Composition;

pub(crate) fn composition_cache_key(comp: &Composition) -> u64 {
    let serialized = serde_json::to_vec(comp).unwrap_or_else(|_| comp.id.as_bytes().to_vec());
    serialized
        .into_iter()
        .fold(0xcbf29ce484222325u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
        })
}

pub(crate) fn media_file_revision(path: &std::path::Path) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut hasher);
    match std::fs::metadata(path) {
        Ok(metadata) => {
            metadata.len().hash(&mut hasher);
            metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|duration| duration.as_nanos())
                .hash(&mut hasher);
        }
        Err(error) => error.kind().hash(&mut hasher),
    }
    hasher.finish()
}

pub(crate) fn composition_media_revision(comp: &Composition, frame: u32) -> u64 {
    use std::hash::{Hash, Hasher};
    fn hash_media(
        comp: &Composition,
        frame: u32,
        visited: &mut std::collections::HashSet<String>,
        hasher: &mut std::collections::hash_map::DefaultHasher,
    ) {
        if !visited.insert(comp.id.clone()) {
            return;
        }
        for layer in &comp.layers {
            match &layer.layer_type {
                crate::core::timeline::LayerType::Image { path } => {
                    media_file_revision(std::path::Path::new(path)).hash(hasher);
                }
                crate::core::timeline::LayerType::Video { frames_dir, .. } => {
                    let sequence_frame = frame.saturating_sub(layer.in_frame);
                    let path =
                        crate::core::video_import::frame_path_in_dir(frames_dir, sequence_frame);
                    media_file_revision(&path).hash(hasher);
                }
                crate::core::timeline::LayerType::PreComp { comp_id } => {
                    if let Some(sub_comp) = comp
                        .sub_compositions
                        .iter()
                        .find(|candidate| candidate.id == *comp_id)
                    {
                        hash_media(
                            sub_comp,
                            frame.saturating_sub(layer.in_frame),
                            visited,
                            hasher,
                        );
                    }
                }
                _ => {}
            }
        }
    }
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hash_media(
        comp,
        frame,
        &mut std::collections::HashSet::new(),
        &mut hasher,
    );
    hasher.finish()
}
