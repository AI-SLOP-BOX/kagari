//! Intelligent RotoBrush & Automated Boundary Propagation Engine (AE Parity).
//!
//! Provides automatic object cutout segmentation, foreground/background color
//! distribution modeling, edge-gradient snap refinement, and temporal frame propagation.

#![allow(dead_code)]

pub mod filter;
pub mod matte;
pub mod types;

#[cfg(test)]
mod tests;

pub use matte::*;
pub use types::*;
