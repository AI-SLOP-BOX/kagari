//! Project loading, saving, recent-files management, and user preferences.
//!
//! Submodules handle specific aspects:
//! - [`prefs`]: Preferences file read/write, recents, starred projects.
//! - [`load_save`]: Async project load/save and LUT loading.

pub mod load_save;
pub mod prefs;

#[cfg(test)]
mod tests;

pub use load_save::*;
pub use prefs::*;
