#![allow(dead_code)]
//! Unified registry for extended CPU effect kernels (Parts 27–28 +
//! color correction). Provides a single serializable enum with an `apply`
//! dispatch so the render pipeline can integrate every effect through one
//! call site, and exposes preset metadata for data-driven UI catalogs
//! (ARCHITECTURE_GUIDELINES Rule 8.2).

pub mod kernel;
pub mod types;

#[cfg(test)]
mod tests;

pub use types::ExtEffect;
