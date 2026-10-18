use super::*;

#[cfg(test)]
pub(super) fn software_preview_required(
    comp: &crate::core::timeline::Composition,
    frame: u32,
) -> bool {
    crate::ui::viewport_rendering::software_preview_required(comp, frame)
}

fn prepare_roto_brush_source(
    comp: &crate::core::timeline::Composition,
    layer_index: usize,
) -> Option<crate::core::timeline::Composition> {
    let mut source_comp = comp.clone();
    source_comp.background_color = [0.0; 4];
    for layer in &mut source_comp.layers {
        layer.track_matte = crate::core::timeline::TrackMatteMode::None;
    }
    source_comp.layers.get_mut(layer_index)?.masks.clear();
    Some(source_comp)
}

#[cfg(test)]
pub(super) fn render_roto_brush_source_frame(
    comp: &crate::core::timeline::Composition,
    layer_index: usize,
    frame: u32,
) -> Vec<u8> {
    let Some(source_comp) = prepare_roto_brush_source(comp, layer_index) else {
        return Vec::new();
    };
    crate::core::software_renderer::render_frame_to_pixels_filtered(
        &source_comp,
        frame,
        source_comp.width,
        source_comp.height,
        0.0,
        0,
        Some(layer_index),
    )
}

type RotoFlowCompletion = (
    String,
    u32,
    u32,
    u32,
    String,
    String,
    Vec<u8>,
    Vec<u8>,
    Result<crate::core::property::Animatable<Vec<[f32; 2]>>, String>,
);

pub(super) struct RotoFlowJob {
    receiver: std::sync::Mutex<std::sync::mpsc::Receiver<RotoFlowCompletion>>,
    pub(super) cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

struct RotoSegmentCompletion {
    composition_id: String,
    composition_width: u32,
    composition_height: u32,
    composition_fps: u32,
    layer_id: String,
    source_layer_snapshot: Vec<u8>,
    previous_strokes: Vec<EngineRotoStroke>,
    strokes: Vec<EngineRotoStroke>,
    frame: u32,
    feather: f32,
    result: Result<Option<Vec<[f32; 2]>>, String>,
}

pub(super) struct RotoSegmentJob {
    receiver: std::sync::Mutex<std::sync::mpsc::Receiver<RotoSegmentCompletion>>,
}

pub(super) struct RotoSegmentRequest {
    pub(super) layer_index: usize,
    pub(super) frame: u32,
    pub(super) previous_strokes: Vec<EngineRotoStroke>,
    pub(super) strokes: Vec<EngineRotoStroke>,
    pub(super) settings: RotoBrushSettings,
    pub(super) feather: f32,
}

mod flow_bake;
mod interaction;
mod segment_jobs;
pub(super) use flow_bake::{poll_roto_flow_bake, start_roto_flow_bake};
pub(super) use interaction::draw_roto_brush_interaction;
pub(super) use segment_jobs::{poll_roto_segment_job, start_roto_segment_job};
