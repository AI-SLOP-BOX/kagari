//! ProjectHistory undo/redo management with bounded serialization.

#![allow(dead_code)]

pub mod manager;
pub mod snapshot;

#[cfg(test)]
mod tests;

pub use manager::*;
