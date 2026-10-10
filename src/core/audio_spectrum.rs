//! Audio Spectrum & Waveform Visualization Engine (AE Parity).
//!
//! Generates 2D renderable visualizer paths, lines, and polar radial spectrums
//! from audio PCM sample buffers using discrete Fourier frequency analysis.

#![allow(dead_code)]

pub mod analysis;
pub mod bands;
pub mod spectrum;
pub mod types;
pub mod waveform;

#[cfg(test)]
mod tests;

pub use analysis::*;
pub use bands::*;
pub use spectrum::*;
pub use types::*;
pub use waveform::*;
