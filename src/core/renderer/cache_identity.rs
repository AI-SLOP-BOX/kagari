use crate::core::timeline::Composition;

pub(crate) fn composition_cache_key(comp: &Composition) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    comp.id.hash(&mut hasher);
    comp.name.hash(&mut hasher);
    comp.width.hash(&mut hasher);
    comp.height.hash(&mut hasher);
    comp.fps.hash(&mut hasher);
    comp.duration_frames.hash(&mut hasher);
    comp.blend_linear.hash(&mut hasher);
    comp.dither_output.hash(&mut hasher);
    for c in &comp.background_color {
        c.to_bits().hash(&mut hasher);
    }
    comp.motion_blur_shutter_angle.to_bits().hash(&mut hasher);
    comp.motion_blur_shutter_phase.to_bits().hash(&mut hasher);

    comp.layers.len().hash(&mut hasher);
    for layer in &comp.layers {
        layer.id.hash(&mut hasher);
        layer.name.hash(&mut hasher);
        layer.in_frame.hash(&mut hasher);
        layer.out_frame.hash(&mut hasher);
        layer.visible.hash(&mut hasher);
        layer.solo.hash(&mut hasher);
        layer.locked.hash(&mut hasher);
        (layer.blend_mode as u8).hash(&mut hasher);
        (layer.track_matte as u8).hash(&mut hasher);
        layer.track_matte_layer_id.hash(&mut hasher);
        layer.parent_id.hash(&mut hasher);
        layer.is_3d.hash(&mut hasher);
        layer.motion_blur.hash(&mut hasher);
        layer.frame_blending.hash(&mut hasher);
        layer.effects.len().hash(&mut hasher);
        for eff in &layer.effects {
            eff.id.hash(&mut hasher);
            eff.name.hash(&mut hasher);
            eff.enabled.hash(&mut hasher);
        }
        layer.masks.len().hash(&mut hasher);
        for mask in &layer.masks {
            mask.id.hash(&mut hasher);
            mask.enabled.hash(&mut hasher);
            mask.inverted.hash(&mut hasher);
        }
    }
    comp.sub_compositions.len().hash(&mut hasher);
    for sub in &comp.sub_compositions {
        sub.id.hash(&mut hasher);
        sub.name.hash(&mut hasher);
        sub.width.hash(&mut hasher);
        sub.height.hash(&mut hasher);
    }
    hasher.finish()
}
