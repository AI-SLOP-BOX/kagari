//! Kinematic & Rigidbody Physics Simulation Engine for AE Motion Graphics layers,
//! particle dynamics, spring constraints, and keyframe baking.
//! Ported & adapted from NextVFX Sovereign Engine with full 2D rigid body collisions,
//! revolute/distance joints, and keyframe bake generation.

#![allow(dead_code)]

pub mod bake;
pub mod collision;
pub mod kinematic;
pub mod rigidbody;
pub mod world;

#[cfg(test)]
mod tests;

pub use bake::*;
pub use collision::*;
pub use kinematic::*;
pub use rigidbody::*;
pub use world::*;
