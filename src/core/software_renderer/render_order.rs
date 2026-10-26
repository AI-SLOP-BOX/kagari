use crate::core::timeline::Layer;

pub(super) fn layer_depth_for_sort(layer: &Layer, frame: u32) -> f32 {
    if layer.is_3d {
        layer.transform_3d.position.evaluate(frame)[2]
    } else {
        0.0
    }
}
