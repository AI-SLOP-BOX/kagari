#![allow(dead_code)]
//! Professional color correction kernels: spline tone Curves, three-way
//! Color Balance wheels and the Channel Mixer matrix — matching AE/Photoshop
//! behaviour closely enough for grading workflows.
//!
//! All functions are pure, deterministic, panic-free and operate on packed
//! RGBA8 buffers.

pub mod adjustments;
pub mod balance_mixer;
pub mod curves;

#[cfg(test)]
mod tests;

pub use adjustments::*;
pub use balance_mixer::*;
pub use curves::*;
