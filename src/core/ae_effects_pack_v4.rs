//! AE Effects Pack V4
//!
//! Categorized into:
//! - `color_filters`: exposure, channel mixer, hue/saturation, tint, photo filter, cc color neutralizer, etc.
//! - `distortion`: spherize, ripple, wave warp, pinch, displace, bulge, polar, smear, etc.
//! - `generators`: checkerboard, lens flare, grid, audio waveform, gradient ramp, beam, star burst, rain, etc.
//! - `transitions`: block dissolve, card dance, shatter, iris wipe, radial zoom, glass wipe, jaws, etc.

pub mod color_filters;
pub mod distortion;
pub mod generators;
pub mod transitions;

pub use color_filters::*;
pub use distortion::*;
pub use generators::*;
pub use transitions::*;

#[cfg(test)]
mod tests;
