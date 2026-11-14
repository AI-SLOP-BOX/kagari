use crate::core::timeline::{Composition, LayerType};

pub(crate) fn composition_content_revision(comp: &Composition, frame: u32) -> u64 {
    use std::hash::{Hash, Hasher};

    fn hash_media_path(
        path: &std::path::Path,
        hasher: &mut std::collections::hash_map::DefaultHasher,
    ) {
        crate::core::media_revision::media_file_revision(path).hash(hasher);
    }

    fn hash_composition_media(
        comp: &Composition,
        frame: u32,
        recursion_stack: &mut std::collections::HashSet<String>,
        hasher: &mut std::collections::hash_map::DefaultHasher,
    ) {
        if !recursion_stack.insert(comp.id.clone()) {
            return;
        }
        let has_solo = comp
            .layers
            .iter()
            .any(|layer| layer.is_active(frame) && layer.solo);
        for layer in &comp.layers {
            if !layer.is_active(frame)
                || !layer.visible
                || layer.is_guide_layer
                || (has_solo && !layer.solo)
            {
                continue;
            }
            let source_frame = layer.effective_render_frame(frame, comp.fps);
            match &layer.layer_type {
                LayerType::Image { path } => {
                    hash_media_path(std::path::Path::new(path), hasher);
                    if super::super::preview_proxy_active() {
                        if let Some(proxy) = super::super::preview_proxy_path(layer, 0) {
                            hash_media_path(std::path::Path::new(&proxy), hasher);
                        }
                    }
                }
                LayerType::Video {
                    frames_dir,
                    frame_count,
                    speed,
                    ..
                } => {
                    let remapped_source_frame = match &layer.posterize_time {
                        Some(settings) if settings.enabled => source_frame as f32,
                        _ => layer.remap_frame_f32(frame),
                    };
                    let source_position = crate::core::timeline::video_sequence_frame(
                        remapped_source_frame,
                        *speed,
                        frames_dir,
                        comp.fps,
                    );
                    let last_frame = frame_count.saturating_sub(1);
                    let first_frame = (source_position.floor() as u32).min(last_frame);
                    let second_frame = first_frame.saturating_add(1).min(last_frame);
                    let proxy = super::super::preview_proxy_active()
                        .then(|| super::super::preview_proxy_path(layer, first_frame))
                        .flatten();
                    let first_path = proxy.as_deref().map_or_else(
                        || crate::core::video_import::frame_path_in_dir(frames_dir, first_frame),
                        std::path::PathBuf::from,
                    );
                    hash_media_path(&first_path, hasher);
                    if layer.frame_blending && proxy.is_none() && second_frame != first_frame {
                        let second_path =
                            crate::core::video_import::frame_path_in_dir(frames_dir, second_frame);
                        hash_media_path(&second_path, hasher);
                    }
                }
                LayerType::Model3D { path } => {
                    hash_media_path(std::path::Path::new(path), hasher);
                }
                LayerType::PreComp { comp_id } => {
                    if let Some(nested) = comp.find_sub_comp(comp_id) {
                        hash_composition_media(nested, source_frame, recursion_stack, hasher);
                    }
                }
                _ => {}
            }
        }
        recursion_stack.remove(&comp.id);
    }

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    crate::core::frame_cache::current_version().hash(&mut hasher);
    super::super::preview_proxy_active().hash(&mut hasher);
    hash_composition_media(
        comp,
        frame,
        &mut std::collections::HashSet::new(),
        &mut hasher,
    );
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::composition_content_revision;
    use crate::core::timeline::Composition;

    #[test]
    fn revision_tracks_project_render_generation_without_serializing_composition() {
        let comp = Composition::new(
            "revision-test".into(),
            "Revision Test".into(),
            64,
            64,
            24,
            24,
        );
        let before = composition_content_revision(&comp, 0);
        crate::core::frame_cache::bump_version();
        let after = composition_content_revision(&comp, 0);
        assert_ne!(before, after);
    }
}
