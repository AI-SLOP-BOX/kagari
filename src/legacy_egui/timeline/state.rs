use eframe::egui;

#[derive(Clone, Copy, Default)]
pub(super) struct LayerBodyDragState {
    pub(super) frame_offset: i32,
    pub(super) previous_y: f32,
    pub(super) y_remainder: f32,
}

#[derive(Clone)]
pub(super) struct TimelineDerivedData {
    pub(super) composition_id: String,
    pub(super) history_generation: u64,
    pub(super) parent_choices: Vec<(String, String)>,
    pub(super) parent_indices: std::collections::HashMap<String, usize>,
    pub(super) parents: std::collections::HashMap<String, String>,
    pub(super) layer_edges: Vec<(u32, usize)>,
    pub(super) keyframe_frames: Vec<u32>,
}

pub(super) fn timeline_derived_data(
    ctx: &egui::Context,
    composition: &crate::core::timeline::Composition,
    history_generation: u64,
    bypass_cache: bool,
) -> std::sync::Arc<TimelineDerivedData> {
    let cache_id = egui::Id::new("timeline_derived_data");
    if !bypass_cache {
        if let Some(cached) =
            ctx.data_mut(|data| data.get_temp::<std::sync::Arc<TimelineDerivedData>>(cache_id))
        {
            if cached.composition_id == composition.id
                && cached.history_generation == history_generation
            {
                return cached;
            }
        }
    }
    let result = std::sync::Arc::new(TimelineDerivedData {
        composition_id: composition.id.clone(),
        history_generation,
        parent_choices: composition
            .layers
            .iter()
            .map(|layer| (layer.id.clone(), layer.name.clone()))
            .collect(),
        parent_indices: composition
            .layers
            .iter()
            .enumerate()
            .map(|(index, layer)| (layer.id.clone(), index))
            .collect(),
        parents: composition
            .layers
            .iter()
            .filter_map(|layer| {
                layer
                    .parent_id
                    .as_ref()
                    .map(|parent| (layer.id.clone(), parent.clone()))
            })
            .collect(),
        layer_edges: {
            let mut edges = composition
                .layers
                .iter()
                .enumerate()
                .flat_map(|(index, layer)| [(layer.in_frame, index), (layer.out_frame, index)])
                .collect::<Vec<_>>();
            edges.sort_unstable();
            edges
        },
        keyframe_frames: super::snapping::collect_all_kf_frames(composition),
    });
    if !bypass_cache {
        ctx.data_mut(|data| data.insert_temp(cache_id, result.clone()));
    }
    result
}

pub(super) fn would_create_parent_cycle(
    layer_id: &str,
    candidate_parent_id: &str,
    parents: &std::collections::HashMap<String, String>,
) -> bool {
    let mut current = Some(candidate_parent_id);
    let mut remaining = parents.len().saturating_add(1);
    while let Some(id) = current {
        if id == layer_id || remaining == 0 {
            return true;
        }
        remaining -= 1;
        current = parents.get(id).map(String::as_str);
    }
    false
}
