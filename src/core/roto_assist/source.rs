use crate::core::timeline::{Composition, TrackMatteMode};

pub fn prepare_roto_brush_source(
    composition: &Composition,
    layer_index: usize,
) -> Option<Composition> {
    let mut source = composition.clone();
    source.background_color = [0.0; 4];
    for layer in &mut source.layers {
        layer.track_matte = TrackMatteMode::None;
    }
    source.layers.get_mut(layer_index)?.masks.clear();
    Some(source)
}
