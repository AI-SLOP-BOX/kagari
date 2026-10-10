//! Extended particle forces: lifetime curves, wind gusts, air drag and
//! boundary collisions. Pure deterministic functions consumed by
//! [`crate::core::particle_system::ParticleSystem`].

pub mod collision;
pub mod curve;
pub mod dynamics;

pub use collision::*;
pub use curve::*;
pub use dynamics::*;

#[cfg(test)]
mod tests;
