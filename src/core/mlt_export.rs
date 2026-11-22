//! MLT XML export — Shotcut / Kdenlive interoperability.
//!
//! Emits an MLT XML document (the format natively opened by Shotcut and
//! Kdenlive) describing the composition's layers as producers attached to a
//! playlist track. Transforms are not representable in MLT, so this is a
//! timeline-level interchange: layer names, in/out points, and source paths.

#![allow(dead_code)]

pub mod export;
pub mod import;
pub mod xml_util;

#[cfg(test)]
mod tests;

pub use export::*;
pub use import::*;
pub use xml_util::*;
