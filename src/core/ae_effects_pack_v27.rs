#![allow(dead_code)]
//! After Effects VFX Kernels Part 27 — Distortion Pack Pro.
//!
//! High-fidelity rebuilds of Wave Warp, CC Lens, Polar Coordinates and Optics
//! Compensation. Unlike the simplified kernels in earlier packs, these versions
//! feature:
//!   * Sub-pixel accurate bilinear sampling (no blocky nearest-neighbour artifacts)
//!   * Multiple wave shapes (Sine / Triangle / Square / Sawtooth)
//!   * Edge pinning modes matching AE (All / Left-Right / Top-Bottom / None)
//!   * Interpolated Rect <-> Polar conversion (AE "Interpolation" parameter)
//!
//! All functions are deterministic, panic-free and allocation-light (single
//! source snapshot per invocation).

pub mod lens;
pub mod wave_warp;

#[cfg(test)]
mod tests;

pub use lens::{
    apply_cc_lens_pro, apply_optics_compensation, apply_polar_coordinates_pro, CcLensParams,
    OpticsCompensationParams, PolarMode,
};
pub use wave_warp::{apply_wave_warp_pro, PinKind, WaveType, WaveWarpParams};
