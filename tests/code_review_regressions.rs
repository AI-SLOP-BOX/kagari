//! Regression tests derived from the code review.
//!
//! These tests guard against specific bugs and anti-patterns identified during
//! the review. Each test is named after the issue it prevents from recurring.

use kagari_vfx::core::ae_effects_pack::*;
use kagari_vfx::core::echo_effect::{blend_echo_frame, EchoOperator};
use kagari_vfx::core::expression_engine;
use kagari_vfx::core::keyframe::{InterpolationType, Keyframe};
use kagari_vfx::core::particle_system::*;
use kagari_vfx::core::property::Animatable;
use kagari_vfx::core::timeline::*;
use rhai::Scope;

// ─── Particle System ────────────────────────────────────────────────────────

include!("code_review_regressions/cases_01.rs");
include!("code_review_regressions/cases_02.rs");
include!("code_review_regressions/cases_03.rs");
