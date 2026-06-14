//! Typed history helpers for meaningful controller state transitions.
//!
//! This module keeps undoable UI snapshots close to the controller so
//! navigation and selection flows can reuse one consistent history model
//! instead of pushing ad hoc closure entries throughout feature code.

mod catalog;
mod meaningful;
mod pending;
mod transactions;

use super::*;

#[cfg(test)]
pub(crate) use self::catalog::catalog_history_handler_supported;
pub(crate) use self::meaningful::MeaningfulUiSnapshot;
pub(crate) use self::pending::{
    PendingHistoryTransaction, PendingHistoryTransactionKey, PendingSampleCreationTransaction,
    PendingSampleOverwriteTransaction,
};

impl AppController {
    /// Return whether history replay is currently restoring controller state.
    pub(crate) fn history_restoring(&self) -> bool {
        self.history.restoring
    }

    /// Run one controller mutation while suppressing new history capture.
    pub(crate) fn run_history_restore(&mut self, mut restore: impl FnMut(&mut Self)) {
        let was_restoring = self.history.restoring;
        self.history.restoring = true;
        restore(self);
        self.history.restoring = was_restoring;
    }

    /// Run one meaningful UI mutation and push a snapshot-based undo entry when it changes state.
    pub(crate) fn record_meaningful_ui_transaction<R>(
        &mut self,
        label: impl Into<String>,
        action: impl FnOnce(&mut Self) -> R,
    ) -> R {
        if self.history_restoring() {
            return action(self);
        }
        let label = label.into();
        let before = self.capture_meaningful_ui_snapshot();
        let result = action(self);
        self.push_meaningful_ui_undo_if_changed(label, before);
        result
    }

    /// Capture the meaningful UI context that should be restored by undo/redo.
    pub(crate) fn capture_meaningful_ui_snapshot(&self) -> MeaningfulUiSnapshot {
        meaningful::capture_meaningful_ui_snapshot(self)
    }

    /// Restore a previously captured meaningful UI context without recording history.
    pub(crate) fn restore_meaningful_ui_snapshot(&mut self, snapshot: &MeaningfulUiSnapshot) {
        let snapshot = snapshot.clone();
        self.run_history_restore(|controller| {
            meaningful::restore_meaningful_ui_snapshot(controller, &snapshot);
        });
    }

    /// Push one undo entry that restores meaningful UI state before and after an action.
    pub(crate) fn push_meaningful_ui_undo(
        &mut self,
        label: impl Into<String>,
        before: MeaningfulUiSnapshot,
        after: MeaningfulUiSnapshot,
    ) {
        if self.history_restoring() || before == after {
            return;
        }
        let label = label.into();
        self.push_undo_entry(undo::UndoEntry::<AppController>::new(
            label,
            move |controller| {
                controller.restore_meaningful_ui_snapshot(&before);
                Ok(undo::UndoExecution::Applied)
            },
            move |controller| {
                controller.restore_meaningful_ui_snapshot(&after);
                Ok(undo::UndoExecution::Applied)
            },
        ));
    }

    /// Capture the post-action snapshot and push an undo entry when state changed.
    pub(crate) fn push_meaningful_ui_undo_if_changed(
        &mut self,
        label: impl Into<String>,
        before: MeaningfulUiSnapshot,
    ) {
        if self.history_restoring() {
            return;
        }
        let label = label.into();
        let after = self.capture_meaningful_ui_snapshot();
        self.push_meaningful_ui_undo(label, before, after);
    }
}
