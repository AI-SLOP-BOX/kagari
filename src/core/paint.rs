//! Paint stroke rasterization: stamps alpha-over round caps along a
//! polyline into an RGBA layer buffer (straight alpha, premultiplied-safe
//! blending done manually).

pub mod brush;
pub mod clone;
pub mod types;

#[cfg(test)]
mod tests;

pub use brush::*;
pub use clone::*;
pub use types::*;
