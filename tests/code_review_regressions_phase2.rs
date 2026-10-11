//! Phase 2: Comprehensive regression tests from code review.
//!
//! Covers the remaining bug patterns: echo/set_matte edge cases, history
//! NaN dedup, keyframe interpolation, shape boolean, particle forces,
//! expression edge cases, and serialization invariants.

use kagari_vfx::core::echo_effect::{blend_echo_frame, EchoOperator};
use kagari_vfx::core::expression_engine;
use kagari_vfx::core::keyframe::{InterpolationType, Keyframe};
use kagari_vfx::core::particle_forces::LifeCurve;
use kagari_vfx::core::particle_system::*;
use kagari_vfx::core::property::Animatable;
use kagari_vfx::core::set_matte::{
    apply_set_matte, MatteCompositeMode, MatteSourceChannel, SetMatteParams,
};
use kagari_vfx::core::shape_boolean::{apply_polygon_boolean, BooleanOp};
use kagari_vfx::core::timeline::*;
use rhai::Scope;

// ─── Echo Effect: All Operators ─────────────────────────────────────────────

include!("code_review_regressions_phase2/cases_01.rs");
include!("code_review_regressions_phase2/cases_02.rs");
include!("code_review_regressions_phase2/cases_03.rs");
