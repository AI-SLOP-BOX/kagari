use crate::core::timeline::{Composition, TrackMatteMode};
use crate::core::vfx_graph_compiler::{LayerOpType, VfxGraphCompiler};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlowGraphNode {
    pub layer_index: usize,
    pub parent_index: Option<usize>,
    pub matte_index: Option<usize>,
    pub execution_order: Option<usize>,
    pub depth: usize,
}

#[derive(Debug, Clone)]
pub struct FlowGraphModel {
    pub nodes: Vec<FlowGraphNode>,
    pub cycle_detected: bool,
}

impl FlowGraphModel {
    pub fn build(comp: &Composition, frame: u32) -> Self {
        let mut model = Self::build_structure(comp);
        let mut graph = VfxGraphCompiler::new();
        graph.compile(comp, frame);
        for (step_index, step) in graph.steps.iter().enumerate() {
            let layer_idx = match &step.op {
                LayerOpType::RenderLayer { layer_idx }
                | LayerOpType::EvaluateParentTransform { layer_idx }
                | LayerOpType::CompositeTrackMatte { layer_idx, .. }
                | LayerOpType::EvaluatePreComp { layer_idx, .. } => *layer_idx,
            };
            if let Some(node) = model.nodes.get_mut(layer_idx) {
                node.execution_order.get_or_insert(step_index + 1);
            }
        }
        model.cycle_detected = graph.cycle_detected;
        model
    }

    pub fn build_structure(comp: &Composition) -> Self {
        let mut indices = HashMap::with_capacity(comp.layers.len());
        for (index, layer) in comp.layers.iter().enumerate() {
            indices.entry(layer.id.as_str()).or_insert(index);
        }
        let parents: Vec<Option<usize>> = comp
            .layers
            .iter()
            .map(|layer| {
                layer
                    .parent_id
                    .as_deref()
                    .and_then(|id| indices.get(id).copied())
            })
            .collect();
        let depths = parent_depths(&parents);
        let nodes = comp
            .layers
            .iter()
            .enumerate()
            .map(|(layer_index, layer)| FlowGraphNode {
                layer_index,
                parent_index: parents[layer_index],
                matte_index: if layer.track_matte != TrackMatteMode::None {
                    layer
                        .track_matte_layer_id
                        .as_deref()
                        .and_then(|id| indices.get(id).copied())
                        .or_else(|| (layer_index > 0).then_some(layer_index.saturating_sub(1)))
                } else {
                    None
                },
                execution_order: None,
                depth: depths[layer_index],
            })
            .collect();

        Self {
            nodes,
            cycle_detected: false,
        }
    }

    pub fn positions_f64(
        &self,
        origin_x: f64,
        origin_y: f64,
        column_spacing: f64,
        row_spacing: f64,
    ) -> Vec<(f64, f64)> {
        self.nodes
            .iter()
            .map(|node| {
                (
                    origin_x + node.depth as f64 * column_spacing,
                    origin_y + node.layer_index as f64 * row_spacing,
                )
            })
            .collect()
    }
}

fn parent_depths(parents: &[Option<usize>]) -> Vec<usize> {
    let mut depths = vec![0_usize; parents.len()];
    let mut resolved = vec![false; parents.len()];
    let mut active_path = vec![false; parents.len()];
    for start in 0..parents.len() {
        if resolved[start] {
            continue;
        }
        let mut path = Vec::new();
        let mut cursor = start;
        loop {
            if resolved[cursor] {
                break;
            }
            if active_path[cursor] {
                if let Some(cycle_start) = path.iter().position(|index| *index == cursor) {
                    for &cycle_index in &path[cycle_start..] {
                        depths[cycle_index] = 0;
                        resolved[cycle_index] = true;
                        active_path[cycle_index] = false;
                    }
                    path.truncate(cycle_start);
                }
                break;
            }
            active_path[cursor] = true;
            path.push(cursor);
            match parents[cursor] {
                Some(parent) if parent < parents.len() => cursor = parent,
                _ => break,
            }
        }
        while let Some(index) = path.pop() {
            let parent_depth = parents[index]
                .filter(|parent| *parent < depths.len() && *parent != index)
                .map(|parent| depths[parent].saturating_add(1))
                .unwrap_or(0);
            depths[index] = parent_depth;
            resolved[index] = true;
            active_path[index] = false;
        }
    }
    depths
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::timeline::Layer;

    #[test]
    fn shared_model_resolves_edges_depth_and_schedule() {
        let mut comp = Composition::new("comp".into(), "Graph".into(), 1920, 1080, 30, 300);
        let mut root = Layer::new_null("root".into(), "Root".into(), 300);
        root.id = "root-id".into();
        let mut child = Layer::new_null("child".into(), "Child".into(), 300);
        child.parent_id = Some("root-id".into());
        comp.layers = vec![root, child];

        let model = FlowGraphModel::build(&comp, 0);
        assert_eq!(model.nodes[0].depth, 0);
        assert_eq!(model.nodes[1].parent_index, Some(0));
        assert_eq!(model.nodes[1].depth, 1);
        assert_eq!(model.nodes[0].execution_order, Some(1));
        assert_eq!(
            model.positions_f64(0.0, 0.0, 100.0, 20.0)[1],
            (100.0, 20.0)
        );
    }
}
