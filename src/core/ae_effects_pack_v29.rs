//! After Effects VFX Kernels Part 29 — Lens, Smear, Card Wipe, Timecode & Vignette Pack.
//!
//! Production-quality implementations of:
//!   * CC Lens              — spherical fisheye distortion (convex/concave convergence)
//!   * Directional Smear    — directional trail / smear blur along an arbitrary angle
//!   * Card Wipe Dissolve   — grid-based 3D rotating card transition
//!   * SMPTE Timecode       — crisp rasterized timecode overlay burn-in
//!   * CC Vignette Advanced — oval/circular falloff with tint & highlight preservation
//!
//! All functions are deterministic, panic-free, and bilinear-sampled.

pub mod card_wipe;
pub mod lens_smear;
pub mod timecode;
pub mod vignette;

pub use card_wipe::*;
pub use lens_smear::*;
pub use timecode::*;
pub use vignette::*;

#[cfg(test)]
mod tests;
