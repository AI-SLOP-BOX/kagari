use kagari_vfx::core::advanced_motion_blur::*;
use kagari_vfx::core::advanced_particles_3d::*;
use kagari_vfx::core::audio_dsp::*;
use kagari_vfx::core::bend_warp_engine::*;
use kagari_vfx::core::calculations_composite::*;
use kagari_vfx::core::camera_tracker_3d::*;
use kagari_vfx::core::cylinder_sphere_warp::*;
use kagari_vfx::core::icc_color_engine::*;
use kagari_vfx::core::lightning_beam_engine::*;
use kagari_vfx::core::mask::point_in_polygon;
use kagari_vfx::core::obj_loader::*;
use kagari_vfx::core::optical_flow_timewarp::*;
use kagari_vfx::core::roto_brush_engine::*;
use kagari_vfx::core::shape_boolean::*;

fn finite_points(branches: &[LightningBranch]) -> bool {
    branches
        .iter()
        .flat_map(|b| b.points.iter())
        .flatten()
        .all(|v| v.is_finite())
}
include!("adversarial_features/cases_01.rs");
include!("adversarial_features/cases_02.rs");
