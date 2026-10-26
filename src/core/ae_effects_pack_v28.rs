//! After Effects VFX Kernels Part 28 — Stylize & Light Pack Pro.
//!
//! Production-quality implementations of:
//!   * CC Light Sweep      — animated specular highlight band
//!   * CC Radial Fast Blur — zoom blur around a centre point
//!   * CC Bend It          — vertical-axis bend between top/bottom offsets
//!   * CC Tiler Pro        — tiling with repeat/mirror edge modes
//!   * Glow Pro            — threshold bright-pass + separable box blur bloom
//!
//! All functions are deterministic, panic-free, and reuse clamp-to-edge
//! bilinear sampling semantics consistent with Part 27.

pub mod distort_tile;
pub mod glow;
pub mod light_blur;

pub use distort_tile::*;
pub use glow::*;
pub use light_blur::*;

#[cfg(test)]
mod tests;
