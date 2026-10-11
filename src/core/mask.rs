//! AE-style Bezier mask system.
//!
//! Each Layer can have multiple named masks. Each mask is a closed or open
//! Bezier path with per-vertex tangent handles, a blend mode, feathering radius,
//! and an expansion/contraction value.
//!
//! Data model follows AE's mask architecture:
//!   `Layer → Vec<Mask> → MaskPath → Vec<MaskVertex>`

mod entity;
mod path;
#[cfg(test)]
mod tests;
mod types;

pub use entity::{point_in_polygon, Mask};
pub use path::MaskPath;
pub use types::{eval_cubic_bezier, MaskMode, MaskVertex};
