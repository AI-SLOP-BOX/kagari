//! Signed Distance Field (SDF) rasterization and geometric primitives.
//!
//! Categorized into:
//! - `primitives`: rectangle, ellipse, star, polygon, freeform bezier tessellation
//! - `ops`: boolean ops (union, subtract, intersect, exclude)
//! - `color`: HSB/RGB color space conversions and gradient sampling
//! - `rasterize`: CPU shape rasterization with stroke, AA, and trim paths

pub mod color;
pub mod ops;
pub mod primitives;
pub mod rasterize;

pub use color::*;
pub use ops::*;
pub use primitives::*;
pub use rasterize::*;
