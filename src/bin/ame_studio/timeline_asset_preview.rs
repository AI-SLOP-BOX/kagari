#![allow(clippy::too_many_arguments)]
pub(crate) fn find_video_preview_frame(
    compositions: &[kagari_vfx::core::timeline::Composition],
    source: &str,
) -> Option<std::path::PathBuf> {
    for composition in compositions {
        for layer in &composition.layers {
            if let kagari_vfx::core::timeline::LayerType::Video {
                source: layer_source,
                frames_dir,
                ..
            } = &layer.layer_type
            {
                if layer_source == source && !frames_dir.is_empty() {
                    for extension in ["webp", "png"] {
                        let frame = std::path::Path::new(frames_dir)
                            .join(format!("frame_00000.{extension}"));
                        if frame.is_file() {
                            return Some(frame);
                        }
                    }
                }
            }
        }
        if let Some(frame) = find_video_preview_frame(&composition.sub_compositions, source) {
            return Some(frame);
        }
    }
    None
}
