//! Layer Alignment, Distribution & Fit-to-Comp Engine (AE Align Palette Parity).
//!
//! Provides geometric alignment (Left, Center, Right, Top, Middle, Bottom),
//! equidistant distribution (Edges, Centers, Gaps), and automatic Fit to Comp transforms.

#![allow(dead_code)]

use crate::core::timeline::{Composition, Layer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignOperation {
    Align(AlignMode),
    Distribute(DistributeMode),
}

pub fn apply_to_composition(
    comp: &mut Composition,
    selected_indices: &[usize],
    frame: u32,
    operation: AlignOperation,
    relative_to_composition: bool,
) -> bool {
    let mut indices = selected_indices.to_vec();
    indices.sort_unstable();
    indices.dedup();
    indices.retain(|index| *index < comp.layers.len());
    if indices.is_empty() {
        return false;
    }

    let before: Vec<[f32; 2]> = indices
        .iter()
        .map(|index| comp.layers[*index].transform.position.evaluate(frame))
        .collect();
    let bounds = relative_to_composition.then_some([comp.width as f32, comp.height as f32]);
    let mut layers: Vec<&mut Layer> = comp
        .layers
        .iter_mut()
        .enumerate()
        .filter_map(|(index, layer)| indices.binary_search(&index).ok().map(|_| layer))
        .collect();
    match operation {
        AlignOperation::Align(mode) => align_layers(&mut layers, frame, mode, bounds),
        AlignOperation::Distribute(mode) => distribute_layers(&mut layers, frame, mode),
    }
    drop(layers);

    indices.iter().zip(before).any(|(index, old_position)| {
        comp.layers[*index].transform.position.evaluate(frame) != old_position
    })
}

/// Alignment modes relative to selection bounding box or composition canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignMode {
    Left,
    HorizontalCenter,
    Right,
    Top,
    VerticalCenter,
    Bottom,
}

/// Distribution modes for spreading multiple layers evenly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DistributeMode {
    HorizontalCenters,
    HorizontalGaps,
    VerticalCenters,
    VerticalGaps,
}

/// Aligns a slice of layers according to `AlignMode`.
/// If `comp_bounds` is `Some([w, h])`, aligns relative to composition origin [0, 0] to [w, h].
pub fn align_layers(
    layers: &mut [&mut Layer],
    frame: u32,
    mode: AlignMode,
    comp_bounds: Option<[f32; 2]>,
) {
    if layers.is_empty() {
        return;
    }

    let (target_val, _) = if let Some([cw, ch]) = comp_bounds {
        match mode {
            AlignMode::Left => (0.0, true),
            AlignMode::HorizontalCenter => (cw * 0.5, true),
            AlignMode::Right => (cw, true),
            AlignMode::Top => (0.0, false),
            AlignMode::VerticalCenter => (ch * 0.5, false),
            AlignMode::Bottom => (ch, false),
        }
    } else {
        // Find selection bounding extent
        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;

        for layer in layers.iter() {
            let p = layer.transform.position.evaluate(frame);
            min_x = min_x.min(p[0]);
            max_x = max_x.max(p[0]);
            min_y = min_y.min(p[1]);
            max_y = max_y.max(p[1]);
        }

        match mode {
            AlignMode::Left => (min_x, true),
            AlignMode::HorizontalCenter => ((min_x + max_x) * 0.5, true),
            AlignMode::Right => (max_x, true),
            AlignMode::Top => (min_y, false),
            AlignMode::VerticalCenter => ((min_y + max_y) * 0.5, false),
            AlignMode::Bottom => (max_y, false),
        }
    };

    for layer in layers.iter_mut() {
        let mut p = layer.transform.position.evaluate(frame);
        match mode {
            AlignMode::Left | AlignMode::HorizontalCenter | AlignMode::Right => {
                p[0] = target_val;
            }
            AlignMode::Top | AlignMode::VerticalCenter | AlignMode::Bottom => {
                p[1] = target_val;
            }
        }
        layer.transform.position.set_value_at_frame(frame, p);
    }
}

/// Distributes layers with equal spacing between their centers.
pub fn distribute_layers(layers: &mut [&mut Layer], frame: u32, mode: DistributeMode) {
    if layers.len() < 3 {
        return;
    }

    let is_horizontal = matches!(
        mode,
        DistributeMode::HorizontalCenters | DistributeMode::HorizontalGaps
    );
    let axis = if is_horizontal { 0 } else { 1 };

    // Sort layers by current coordinate along axis
    layers.sort_by(|a, b| {
        let pa = a.transform.position.evaluate(frame)[axis];
        let pb = b.transform.position.evaluate(frame)[axis];
        pa.partial_cmp(&pb).unwrap_or(std::cmp::Ordering::Equal)
    });

    let n = layers.len();
    let start = layers[0].transform.position.evaluate(frame)[axis];
    let end = layers[n - 1].transform.position.evaluate(frame)[axis];
    let step = (end - start) / (n - 1) as f32;

    for (i, layer) in layers.iter_mut().enumerate() {
        let mut p = layer.transform.position.evaluate(frame);
        p[axis] = start + step * i as f32;
        layer.transform.position.set_value_at_frame(frame, p);
    }
}

/// Fits a layer's scale and position to the composition dimensions.
pub fn fit_layer_to_comp(
    layer: &mut Layer,
    frame: u32,
    layer_w: f32,
    layer_h: f32,
    comp_w: f32,
    comp_h: f32,
    keep_aspect_ratio: bool,
) {
    if layer_w <= 0.0 || layer_h <= 0.0 || comp_w <= 0.0 || comp_h <= 0.0 {
        return;
    }

    let scale_x = (comp_w / layer_w) * 100.0;
    let scale_y = (comp_h / layer_h) * 100.0;

    let final_scale = if keep_aspect_ratio {
        let uniform = scale_x.min(scale_y);
        [uniform, uniform]
    } else {
        [scale_x, scale_y]
    };

    // Center position in composition
    layer
        .transform
        .position
        .set_value_at_frame(frame, [comp_w * 0.5, comp_h * 0.5]);
    layer.transform.scale.set_value_at_frame(frame, final_scale);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::keyframe::{InterpolationType, Keyframe};
    use crate::core::property::Animatable;
    use crate::core::timeline::LayerType;

    #[test]
    fn aligning_animated_layer_adds_keyframe_without_collapsing_animation() {
        let mut comp = Composition::new("c".into(), "C".into(), 100, 80, 24, 100);
        let mut layer = Layer::new("l".into(), "L".into(), LayerType::Null, 100);
        layer.transform.position = Animatable::new_animated(vec![
            Keyframe::new(0, [20.0, 30.0], InterpolationType::Linear),
            Keyframe::new(10, [80.0, 50.0], InterpolationType::Linear),
        ]);
        comp.layers.push(layer);

        assert!(apply_to_composition(
            &mut comp,
            &[0],
            5,
            AlignOperation::Align(AlignMode::Left),
            true,
        ));

        let keys = comp.layers[0]
            .transform
            .position
            .keyframes()
            .expect("alignment must preserve the animated property");
        assert_eq!(keys.len(), 3);
        assert_eq!(keys[0].frame, 0);
        assert_eq!(keys[1].frame, 5);
        assert_eq!(keys[1].value, [0.0, 40.0]);
        assert_eq!(keys[2].frame, 10);
        assert_eq!(keys[2].value, [80.0, 50.0]);
    }

    #[test]
    fn distribution_changes_only_selected_layers_and_preserves_keys() {
        let mut comp = Composition::new("c".into(), "C".into(), 100, 80, 24, 100);
        for (index, x) in [0.0, 20.0, 100.0, 77.0].into_iter().enumerate() {
            let mut layer = Layer::new(
                format!("l{index}"),
                format!("L{index}"),
                LayerType::Null,
                100,
            );
            layer.transform.position = Animatable::new_constant([x, 10.0]);
            comp.layers.push(layer);
        }
        comp.layers[1].transform.position = Animatable::new_animated(vec![
            Keyframe::new(0, [20.0, 10.0], InterpolationType::Linear),
            Keyframe::new(10, [30.0, 15.0], InterpolationType::Linear),
        ]);

        assert!(apply_to_composition(
            &mut comp,
            &[2, 0, 1, 1],
            0,
            AlignOperation::Distribute(DistributeMode::HorizontalCenters),
            false,
        ));

        assert_eq!(comp.layers[0].transform.position.evaluate(0)[0], 0.0);
        assert_eq!(comp.layers[1].transform.position.evaluate(0)[0], 50.0);
        assert_eq!(comp.layers[2].transform.position.evaluate(0)[0], 100.0);
        assert_eq!(comp.layers[3].transform.position.evaluate(0)[0], 77.0);
        assert_eq!(
            comp.layers[1].transform.position.keyframes().unwrap().len(),
            2
        );
        assert_eq!(comp.layers[1].transform.position.evaluate(10), [30.0, 15.0]);
    }

    #[test]
    fn test_align_layers_horizontal_center() {
        let mut l1 = Layer::new("1".into(), "L1".into(), LayerType::Null, 10);
        l1.transform.position = crate::core::property::Animatable::new_constant([10.0, 50.0]);

        let mut l2 = Layer::new("2".into(), "L2".into(), LayerType::Null, 10);
        l2.transform.position = crate::core::property::Animatable::new_constant([90.0, 50.0]);

        let mut list = [&mut l1, &mut l2];
        align_layers(&mut list, 0, AlignMode::HorizontalCenter, None);

        assert_eq!(list[0].transform.position.evaluate(0)[0], 50.0);
        assert_eq!(list[1].transform.position.evaluate(0)[0], 50.0);
    }

    #[test]
    fn test_distribute_layers_centers() {
        let mut l1 = Layer::new("1".into(), "L1".into(), LayerType::Null, 10);
        l1.transform.position = crate::core::property::Animatable::new_constant([0.0, 0.0]);

        let mut l2 = Layer::new("2".into(), "L2".into(), LayerType::Null, 10);
        l2.transform.position = crate::core::property::Animatable::new_constant([10.0, 0.0]);

        let mut l3 = Layer::new("3".into(), "L3".into(), LayerType::Null, 10);
        l3.transform.position = crate::core::property::Animatable::new_constant([100.0, 0.0]);

        let mut list = [&mut l1, &mut l2, &mut l3];
        distribute_layers(&mut list, 0, DistributeMode::HorizontalCenters);

        assert_eq!(list[0].transform.position.evaluate(0)[0], 0.0);
        assert_eq!(list[1].transform.position.evaluate(0)[0], 50.0);
        assert_eq!(list[2].transform.position.evaluate(0)[0], 100.0);
    }

    #[test]
    fn test_fit_layer_to_comp() {
        let mut l = Layer::new(
            "1".into(),
            "L1".into(),
            LayerType::Solid {
                color: [1.0, 1.0, 1.0, 1.0],
            },
            10,
        );
        fit_layer_to_comp(&mut l, 0, 500.0, 500.0, 1920.0, 1080.0, true);

        let scale = l.transform.scale.evaluate(0);
        assert!((scale[0] - 216.0).abs() < 1e-3);
        assert!((scale[1] - 216.0).abs() < 1e-3);
    }
}
