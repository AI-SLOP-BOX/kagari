//! RotoAssist: tracker-driven mask animation — the practical, non-ML stand-in
//! for Rotobrush-style assistance. Bakes a tracked point's motion onto every
//! vertex of a base polygon so the matte follows the footage automatically.

mod flow_bake;
mod segmentation;
#[cfg(test)]
mod tests;

pub use flow_bake::{
    bake_optical_flow_mask, bake_optical_flow_roto_matte, bake_tracked_mask,
    bake_tracked_mask_from_trackers,
};
pub use segmentation::{
    apply_roto_refinement, refine_edge_guided_filter, segment_roto_brush, set_roto_matte_keyframe,
    trace_contour_to_polygon, RotoStroke,
};
