//! Toolkit-neutral editing boundary.
//!
//! `EditorSession` wraps `ProjectHistory` and provides a safe mutation API.
//! All frontend/UI code should go through `EditorSession` for ordinary editing
//! operations. This ensures:
//!
//! - Every mutation creates an undo entry (on explicit commit)
//! - Generation is updated
//! - Cache invalidation happens
//! - Dirty-state tracking works
//!
//! ## Semantics
//!
//! - **commit()**: Creates an undo entry with the pre-edit snapshot. Consumes
//!   the session. If no mutation occurred, this is a no-op.
//! - **cancel()**: Reverts to the pre-edit state without creating an undo entry.
//!   Consumes the session.
//! - **Drop**: Reverts to the pre-edit state without creating an undo entry
//!   (safety net for early returns / errors).
//!
//! ## Interaction modes
//!
//! ### Discrete actions (click, toggle, preset apply, delete, reorder)
//!
//! ```text
//! snapshot (lazy) → mutate → commit
//! ```
//!
//! ### Continuous interactions (slider drag, numeric drag)
//!
//! ```text
//! begin_interaction() → [mutate]* → commit()
//! ```
//!
//! One drag must produce exactly one history entry regardless of how many
//! intermediate value updates occur.

use crate::core::history::ProjectHistory;
use crate::core::timeline::Project;

/// Whether the session was committed, cancelled, or dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SessionState {
    /// Session is still active.
    Active,
    /// Session was explicitly committed (created undo entry).
    Committed,
    /// Session was explicitly cancelled or dropped (no undo entry).
    RolledBack,
}

/// A toolkit-neutral editing session that wraps project mutations.
///
/// # Invariants
///
/// - `current()` always returns the current project state.
/// - `current_mut()` returns a mutable reference to the live stack entry.
///   After calling this, you MUST call `commit()` or `cancel()`.
/// - `ensure_snapshot()` must be called BEFORE any mutation to capture the
///   pre-edit state. It is idempotent (only captures once).
/// - `commit()` creates an undo entry with the pre-edit snapshot and consumes
///   the session.
/// - `cancel()` reverts to the pre-edit state without creating an undo entry
///   and consumes the session.
/// - `Drop` reverts to the pre-edit state without creating an undo entry
///   (safety net).
pub struct EditorSession<'a> {
    history: &'a mut ProjectHistory,
    snapshot: Option<Project>,
    label: String,
    state: SessionState,
}

impl<'a> EditorSession<'a> {
    /// Create a new editing session.
    pub fn new(history: &'a mut ProjectHistory, label: impl Into<String>) -> Self {
        Self {
            history,
            snapshot: None,
            label: label.into(),
            state: SessionState::Active,
        }
    }

    /// Get a reference to the current project.
    pub fn current(&self) -> &Project {
        self.history.current()
    }

    /// Get a mutable reference to the current project.
    ///
    /// **WARNING**: This mutates the live stack entry in place. You MUST call
    /// `commit()` or `cancel()` to properly handle the undo entry. If you
    /// haven't called `ensure_snapshot()` yet, this will also capture the
    /// pre-edit snapshot (lazy capture).
    pub fn current_mut(&mut self) -> &mut Project {
        self.ensure_snapshot();
        self.history.current_mut()
    }

    /// Ensure a pre-edit snapshot is captured. Call this BEFORE any mutation.
    ///
    /// This is idempotent — it only captures the snapshot once, on the first
    /// call. Subsequent calls are no-ops.
    pub fn ensure_snapshot(&mut self) {
        if self.snapshot.is_none() {
            self.snapshot = Some(self.history.current().clone());
        }
    }

    /// Whether a mutation has occurred (snapshot was captured).
    pub fn has_mutation(&self) -> bool {
        self.snapshot.is_some()
    }

    /// Whether the session is still active (not yet committed or cancelled).
    pub fn is_active(&self) -> bool {
        self.state == SessionState::Active
    }

    /// Commit the editing session, creating an undo entry.
    ///
    /// If no mutation occurred (no snapshot was captured), this is a no-op.
    /// Consumes the session — further operations are no-ops.
    pub fn commit(mut self) {
        self.finish_commit();
    }

    /// Internal commit logic (needed for Drop which takes &mut self).
    fn finish_commit(&mut self) {
        if self.state != SessionState::Active {
            return;
        }
        self.state = SessionState::Committed;
        if let Some(snapshot) = self.snapshot.take() {
            let generation_before = self.history.generation();
            let current = self.history.current().clone();
            self.history
                .commit_drag_action(snapshot, current, &self.label);
            if self.history.generation() != generation_before {
                crate::core::frame_cache::bump_version();
            }
        }
    }

    /// Cancel the editing session, reverting to the pre-edit state without
    /// creating an undo entry.
    ///
    /// If no mutation occurred, this is a no-op.
    /// Consumes the session — further operations are no-ops.
    pub fn cancel(mut self) {
        self.finish_cancel();
    }

    /// Internal cancel logic (needed for Drop which takes &mut self).
    fn finish_cancel(&mut self) {
        if self.state != SessionState::Active {
            return;
        }
        self.state = SessionState::RolledBack;
        if let Some(snapshot) = self.snapshot.take() {
            self.history.restore_current_without_history(snapshot);
            crate::core::frame_cache::bump_version();
        }
    }

    /// Access the underlying history for read-only queries.
    pub fn history(&self) -> &ProjectHistory {
        self.history
    }
}

impl Drop for EditorSession<'_> {
    fn drop(&mut self) {
        // Safety net: if neither commit nor cancel was called, rollback
        // without creating an undo entry.
        if self.state == SessionState::Active && self.snapshot.is_some() {
            self.finish_cancel();
        }
    }
}
