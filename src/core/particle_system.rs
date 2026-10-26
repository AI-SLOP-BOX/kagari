//! Basic particle emitter system for After Effects-style particle animations.
//!
//! Supports:
//! - Emitter with configurable rate, lifetime, speed, spread
//! - Gravity (with lifetime curve), wind gusts, turbulence and air drag
//! - Boundary collisions with restitution & friction
//! - Per-particle size, color, and opacity over lifetime
//! - Point and box emitter shapes

pub mod raster;
pub mod simulation;
pub mod types;

#[cfg(test)]
mod tests;

pub use raster::draw_dot;
pub use simulation::ParticleSystem;
pub use types::{CameraProjection, EmitterShape, FadeCurve, Particle, ParticleEmitter};
