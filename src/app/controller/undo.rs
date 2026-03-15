//! Controller-facing undo helpers and file-backed undo backup capture.

#[path = "undo/stack.rs"]
mod stack;

use super::*;
use std::{
    fs,
    path::{Path, PathBuf},
};
use uuid::Uuid;

pub(crate) use stack::*;

impl AppController {
    pub(crate) fn undo(&mut self) {
        if self.history.pending_undo.is_some() {
            self.set_status("Undo already in progress", StatusTone::Warning);
            return;
        }
        if self.runtime.jobs.file_ops_in_progress() {
            self.set_status("File operation already in progress", StatusTone::Warning);
            return;
        }
        let mut stack = std::mem::replace(&mut self.history.undo_stack, UndoStack::new(UNDO_LIMIT));
        let result = stack.undo(self);
        self.history.undo_stack = stack;
        match result {
            Ok(UndoOutcome::Applied(label)) => {
                self.set_status(format!("Undid {label}"), StatusTone::Info);
            }
            Ok(UndoOutcome::Empty) => self.set_status("Nothing to undo", StatusTone::Info),
            Ok(UndoOutcome::Deferred(pending)) => {
                self.begin_deferred_undo_job(*pending);
            }
            Err(err) => self.set_status(format!("Undo failed: {err}"), StatusTone::Error),
        }
    }

    pub(crate) fn redo(&mut self) {
        if self.history.pending_undo.is_some() {
            self.set_status("Redo already in progress", StatusTone::Warning);
            return;
        }
        if self.runtime.jobs.file_ops_in_progress() {
            self.set_status("File operation already in progress", StatusTone::Warning);
            return;
        }
        let mut stack = std::mem::replace(&mut self.history.undo_stack, UndoStack::new(UNDO_LIMIT));
        let result = stack.redo(self);
        self.history.undo_stack = stack;
        match result {
            Ok(UndoOutcome::Applied(label)) => {
                self.set_status(format!("Redid {label}"), StatusTone::Info);
            }
            Ok(UndoOutcome::Empty) => self.set_status("Nothing to redo", StatusTone::Info),
            Ok(UndoOutcome::Deferred(pending)) => {
                self.begin_deferred_undo_job(*pending);
            }
            Err(err) => self.set_status(format!("Redo failed: {err}"), StatusTone::Error),
        }
    }

    pub(crate) fn push_undo_entry(&mut self, entry: UndoEntry<AppController>) {
        self.history.undo_stack.push(entry);
    }

    pub(crate) fn begin_selection_undo(&mut self, label: impl Into<String>) {
        if self.selection_state.pending_undo.is_some() {
            return;
        }
        let before = self
            .selection_state
            .range
            .range()
            .or(self.ui.waveform.selection);
        self.selection_state.pending_undo = Some(SelectionUndoState {
            label: label.into(),
            before,
        });
    }

    pub(crate) fn commit_selection_undo(&mut self) {
        let Some(pending) = self.selection_state.pending_undo.take() else {
            return;
        };
        let after = self
            .selection_state
            .range
            .range()
            .or(self.ui.waveform.selection);
        self.push_selection_undo(pending.label, pending.before, after);
    }

    pub(crate) fn push_selection_undo(
        &mut self,
        label: impl Into<String>,
        before: Option<SelectionRange>,
        after: Option<SelectionRange>,
    ) {
        if before == after {
            return;
        }
        let label = label.into();
        self.push_undo_entry(UndoEntry::<AppController>::new(
            label,
            move |controller| {
                controller.selection_state.range.set_range(before);
                controller.apply_selection(before);
                Ok(UndoExecution::Applied)
            },
            move |controller| {
                controller.selection_state.range.set_range(after);
                controller.apply_selection(after);
                Ok(UndoExecution::Applied)
            },
        ));
    }
}

struct UndoCleanup {
    dir: Option<PathBuf>,
}

impl UndoCleanup {
    fn dir(dir: PathBuf) -> Self {
        Self { dir: Some(dir) }
    }
}

impl Drop for UndoCleanup {
    fn drop(&mut self) {
        let Some(dir) = self.dir.take() else {
            return;
        };
        let _ = fs::remove_dir_all(dir);
    }
}

/// Snapshot pair used by deferred file-backed undo/redo entries.
pub(crate) struct OverwriteBackup {
    /// Temporary folder holding before/after snapshots for one edit.
    pub(crate) dir: PathBuf,
    /// Backup copy captured before the edit is applied.
    pub(crate) before: PathBuf,
    /// Backup copy captured after the edit is applied.
    pub(crate) after: PathBuf,
}

impl OverwriteBackup {
    /// Capture the pre-edit file bytes for a deferred undo/redo entry.
    pub(crate) fn capture_before(target: &Path) -> Result<Self, String> {
        let dir = std::env::temp_dir().join(format!("sempal_undo_{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).map_err(|err| format!("Failed to create undo folder: {err}"))?;
        let before = dir.join("before.wav");
        let after = dir.join("after.wav");
        fs::copy(target, &before).map_err(|err| format!("Failed to snapshot audio file: {err}"))?;
        Ok(Self { dir, before, after })
    }

    /// Capture the post-edit file bytes for a deferred redo entry.
    pub(crate) fn capture_after(&self, target: &Path) -> Result<(), String> {
        fs::copy(target, &self.after)
            .map_err(|err| format!("Failed to snapshot edited audio file: {err}"))?;
        Ok(())
    }
}
