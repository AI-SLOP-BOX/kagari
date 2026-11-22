//! Toolkit-neutral editing boundary.
//!
//! Provides `EditorSession` for safe project mutations with guaranteed undo integration,
//! generational tracking, and automatic rollback on cancel or uncommitted drop.

mod session;
#[cfg(test)]
mod tests;

pub use session::EditorSession;
