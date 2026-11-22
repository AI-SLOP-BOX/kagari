//! Vector Shape Pathfinder & Boolean Operations Engine (AE Parity).
//!
//! Implements 2D polygon and Bézier vector boolean geometry algorithms:
//! - Union (Add)
//! - Intersection (Intersect)
//! - Difference (Subtract / Cutout)
//! - Exclusion (XOR / Symmetric Difference)
//! - Offset Paths (Expansion / Deflation with Miter/Bevel joins)

#![allow(dead_code)]

pub mod geom;
pub mod modifiers;
pub mod ops;

#[cfg(test)]
mod tests;

pub use modifiers::*;
pub use ops::*;
