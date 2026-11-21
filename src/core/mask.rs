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

pub(crate) const MASK_CURVE_SEGMENTS: u32 = 16;
pub(crate) const MAX_MASK_PATH_VERTICES: usize = 65_536;
pub(crate) const MAX_MASK_POLYGON_SAMPLES: usize = 4 * 1024 * 1024;

pub use entity::{point_in_polygon, Mask};
pub use path::MaskPath;
pub use types::{eval_cubic_bezier, MaskMode, MaskVertex};
