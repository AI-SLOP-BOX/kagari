use crate::core::expression_engine;
use crate::core::property::Animatable;
use crate::core::text_animator_advanced::AnimatorStack;
use serde::{Deserialize, Serialize};

mod effects;
pub use effects::*;
mod layer_style;
pub use layer_style::*;
mod data;
pub use data::*;
mod transforms;
pub use transforms::*;
mod layer;
pub use layer::*;
mod composition;
pub use composition::*;
mod project;
pub use project::*;

fn default_true() -> bool {
    true
}

fn default_shadow_darkness() -> f32 {
    60.0
}

/// Bounding box: (min_xy, max_xy, width, height)
type BoundingBox = ([f32; 2], [f32; 2], f32, f32);

// Thread-local cached Rhai engine — building it is expensive (~1ms), reusing is free.
thread_local! {
    static RHAI_ENGINE: rhai::Engine = expression_engine::build_engine();
}

#[cfg(test)]
#[path = "timeline/tests.rs"]
mod timeline_tests;
