//! Crash-recovery autosave system.
//!
//! Periodically snapshots the project to a rotating set of recovery files so that
//! a crash (or power loss) loses at most one autosave interval of work. Recovery
//! files are validated on load: a truncated or corrupt snapshot is skipped rather
//! than propagated to the user.

pub mod manager;
pub mod snapshot_writer;
pub mod types;

#[cfg(test)]
mod tests;

pub use manager::{AutosaveManager, RecoveryGeneration, MAX_AUTOSAVE_SLOTS, MAX_RECOVERY_SLOTS};
pub use snapshot_writer::is_valid_project_file;
