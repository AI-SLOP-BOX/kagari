//! Keyframe and easing evaluation primitives.
//!
//! Submodules:
//! - `types`: Core structs including `Keyframe`, `InterpolationType`, `BezierControlPoint`, `GraphEditorMode`
//! - `presets`: 19 standard and extended easing presets (`EasePreset`)
//! - `eval`: Cubic bezier solver (`solve_bezier_eased_time`), speed evaluation, and keyframe sequence interpolator
//! - `handles`: Bezier tangent handles and interpolation

pub mod eval;
pub mod handles;
pub mod presets;
pub mod types;

pub use eval::*;
pub use handles::*;
pub use presets::*;
pub use types::*;

#[cfg(test)]
mod tests;
