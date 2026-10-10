//! ProjectHistory and HistoryEntry stack management.

use super::snapshot::{serialize_project_bounded, ProjectComparisonWriter};
use crate::core::frame_cache;
use crate::core::timeline::Project;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub(crate) snapshot: Option<Vec<u8>>,
    pub action_name: String,
}

#[derive(Debug, Clone)]
pub struct ProjectHistory {
    // One history entry is materialized as current_project; all others retain
    // bounded serialized snapshots instead of deep Project clones.
    pub(crate) stack: VecDeque<HistoryEntry>,
    pub(crate) current_project: Project,
    pub(crate) current_idx: usize,
    pub(crate) max_history_entries: usize,
    pub(crate) retained_bytes: usize,
    pub(crate) generation: u64,
}

impl ProjectHistory {
    /// Maximum capacity retained by history-owned entry, action, and snapshot buffers.
    /// The live working Project and short-lived serialization scratch are separate.
    pub const MAX_HISTORY_BYTES: usize = 128 * 1024 * 1024;

    pub fn new(initial: Project) -> Self {
        let mut history = Self {
            stack: VecDeque::new(),
            current_project: initial,
            current_idx: 0,
            max_history_entries: 50,
            retained_bytes: 0,
            generation: 0,
        };
        history.stack.push_back(HistoryEntry {
            snapshot: None,
            action_name: "Initial State".to_string(),
        });
        history.recalculate_retained_bytes();
        history
    }

    /// Bytes retained in history-owned entry, action-name, and snapshot buffers.
    /// The live working Project is intentionally excluded.
    pub fn retained_bytes(&self) -> usize {
        self.retained_bytes
    }

    /// Compatibility accessor for UI code that previously showed an estimate.
    pub fn approx_bytes(&self) -> usize {
        self.retained_bytes()
    }

    fn recalculate_retained_bytes(&mut self) {
        self.stack.shrink_to_fit();
        for entry in &mut self.stack {
            entry.action_name.shrink_to_fit();
            if let Some(snapshot) = &mut entry.snapshot {
                snapshot.shrink_to_fit();
            }
        }
        let entry_storage = self
            .stack
            .capacity()
            .saturating_mul(std::mem::size_of::<HistoryEntry>());
        let payload_storage = self.stack.iter().fold(0usize, |total, entry| {
            total
                .saturating_add(entry.action_name.capacity())
                .saturating_add(entry.snapshot.as_ref().map_or(0, Vec::capacity))
        });
        self.retained_bytes = std::mem::size_of::<Self>()
            .saturating_add(entry_storage)
            .saturating_add(payload_storage);
    }

    fn reset_to_project(&mut self, project: Project, action_name: &str) {
        self.stack.clear();
        self.stack.shrink_to_fit();
        self.stack.push_back(HistoryEntry {
            snapshot: None,
            action_name: action_name.to_string(),
        });
        self.current_project = project;
        self.current_idx = 0;
        self.recalculate_retained_bytes();
    }

    fn enforce_budget(&mut self) {
        while self.stack.len() > self.max_history_entries {
            self.stack.pop_front();
            self.current_idx = self.current_idx.saturating_sub(1);
        }
        loop {
            self.recalculate_retained_bytes();
            if self.retained_bytes <= Self::MAX_HISTORY_BYTES || self.stack.len() <= 1 {
                break;
            }
            if self.current_idx > 0 {
                self.stack.pop_front();
                self.current_idx -= 1;
            } else {
                self.stack.pop_back();
            }
        }
    }

    /// Monotonic generation counter. Increments on every commit.
    /// UI code should cache this value to avoid redundant full-project clones.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Complete a drag transaction by storing the pre-edit state and applying
    /// the post-edit state as one undo step.
    pub fn commit_drag_action(&mut self, pre_edit: Project, post_edit: Project, action_name: &str) {
        self.current_project = pre_edit;
        self.commit_action(post_edit, action_name);
    }

    /// Commit a state, retaining a bounded serialized snapshot of the old state.
    /// A previous state that exceeds the cap becomes the new undo boundary.
    pub fn commit_action(&mut self, project: Project, action_name: &str) {
        let Some(current_snapshot) =
            serialize_project_bounded(&self.current_project, Self::MAX_HISTORY_BYTES)
        else {
            self.reset_to_project(project, action_name);
            self.generation = self.generation.wrapping_add(1);
            frame_cache::bump_version();
            return;
        };

        let mut comparison = ProjectComparisonWriter::new(&current_snapshot);
        let unchanged =
            serde_json::to_writer(&mut comparison, &project).is_ok() && comparison.is_equal();
        if unchanged {
            return;
        }

        self.generation = self.generation.wrapping_add(1);
        while self.stack.len() > self.current_idx + 1 {
            self.stack.pop_back();
        }
        self.stack[self.current_idx].snapshot = Some(current_snapshot);
        self.stack.push_back(HistoryEntry {
            snapshot: None,
            action_name: action_name.to_string(),
        });
        self.current_project = project;
        self.current_idx += 1;
        self.enforce_budget();

        frame_cache::bump_version();
        log::debug!(
            "Committed action '{}'. Stack size: {}",
            action_name,
            self.stack.len()
        );
    }

    /// Commit a new project state snapshot.
    pub fn commit(&mut self, project: Project) {
        self.commit_action(project, "Edit Project");
    }

    /// Get current action name.
    pub fn current_action_name(&self) -> &str {
        &self.stack[self.current_idx].action_name
    }

    /// Get descriptive name of action that will be undone if Undo is pressed.
    pub fn undo_action_name(&self) -> Option<&str> {
        if self.can_undo() {
            Some(&self.stack[self.current_idx].action_name)
        } else {
            None
        }
    }

    /// Get descriptive name of action that will be redone if Redo is pressed.
    pub fn redo_action_name(&self) -> Option<&str> {
        if self.can_redo() {
            Some(&self.stack[self.current_idx + 1].action_name)
        } else {
            None
        }
    }

    /// Retrieve the live current project state.
    pub fn current(&self) -> &Project {
        &self.current_project
    }

    /// Retrieve the live current project state mutably.
    pub fn current_mut(&mut self) -> &mut Project {
        &mut self.current_project
    }

    /// Adjust the undo-step cap. Takes effect on the next commit.
    pub fn set_max_history_entries(&mut self, n: usize) {
        self.max_history_entries = n.max(10);
    }

    /// Current undo-step cap.
    pub fn max_history_entries(&self) -> usize {
        self.max_history_entries
    }

    /// Number of stored history steps, including the live current state.
    pub fn len(&self) -> usize {
        self.stack.len()
    }

    /// True when no history entries exist.
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    /// Index of the active entry.
    pub fn current_index(&self) -> usize {
        self.current_idx
    }

    /// Action name of the entry at idx, if in range.
    pub fn action_name_at(&self, idx: usize) -> Option<&str> {
        self.stack.get(idx).map(|entry| entry.action_name.as_str())
    }

    /// Move the live Project to a serialized history entry.
    pub fn jump_to(&mut self, idx: usize) -> bool {
        if idx >= self.stack.len() || idx == self.current_idx {
            return false;
        }
        let Some(target_bytes) = self.stack[idx].snapshot.as_ref() else {
            return false;
        };
        let Ok(target_project) = serde_json::from_slice::<Project>(target_bytes) else {
            return false;
        };
        let target_action = self.stack[idx].action_name.clone();
        if let Some(current_snapshot) =
            serialize_project_bounded(&self.current_project, Self::MAX_HISTORY_BYTES)
        {
            self.stack[self.current_idx].snapshot = Some(current_snapshot);
            self.stack[idx].snapshot = None;
            self.current_project = target_project;
            self.current_idx = idx;
            self.enforce_budget();
        } else {
            self.reset_to_project(target_project, &target_action);
        }
        self.generation = self.generation.wrapping_add(1);
        frame_cache::bump_version();
        true
    }

    /// Restore the current project in-place without creating an undo entry.
    pub fn restore_current_without_history(&mut self, project: Project) {
        self.current_project = project;
        self.generation = self.generation.wrapping_add(1);
        frame_cache::bump_version();
    }

    pub fn can_undo(&self) -> bool {
        self.current_idx > 0
    }

    /// Check if redo is available.
    pub fn can_redo(&self) -> bool {
        self.current_idx < self.stack.len() - 1
    }

    /// Undo to the previous state.
    pub fn undo(&mut self) -> Option<&Project> {
        if self.can_undo() {
            let target = self.current_idx - 1;
            if self.jump_to(target) {
                log::info!("Undo success. Frame position index: {}", self.current_idx);
                Some(self.current())
            } else {
                None
            }
        } else {
            log::warn!("Undo ignored: no history left");
            None
        }
    }

    /// Redo to the next state.
    pub fn redo(&mut self) -> Option<&Project> {
        if self.can_redo() {
            let target = self.current_idx + 1;
            if self.jump_to(target) {
                log::info!("Redo success. Frame position index: {}", self.current_idx);
                Some(self.current())
            } else {
                None
            }
        } else {
            log::warn!("Redo ignored: end of history");
            None
        }
    }
}
