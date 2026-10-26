/// Specialized Color Science modules for professional film and grading workflows.
pub mod cinema_log;
pub mod color_spaces;
pub mod curves_lut;
pub mod hdr_buffer;

#[cfg(test)]
mod tests;

// Re-export all public types and functions for seamless backwards compatibility
pub use cinema_log::*;
pub use color_spaces::*;
pub use curves_lut::*;
pub use hdr_buffer::*;
