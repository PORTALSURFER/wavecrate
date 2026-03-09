//! Durable per-source file-operation journal and crash-recovery helpers.
//!
//! Why this exists:
//! - copy/move flows can crash after filesystem work but before both databases
//!   reflect the final state
//! - the journal records enough metadata to reconcile source and target DB rows
//!   on the next startup
//!
//! Stage contract:
//! - `Intent`: journal row exists, no filesystem mutation is assumed yet
//! - `Staged`: data exists at `staged_relative` and has not been finalized
//! - `TargetDb`: the target-side DB update has committed
//! - `SourceDb`: the source-side DB cleanup has committed for moves

use super::{SourceDatabase, SourceDbError};

mod entry;
mod reconcile;
mod store;
#[cfg(test)]
mod tests;

pub(crate) use entry::{
    FileOpJournalEntry, FileOpStage, MoveJournalEntryInit, new_op_id, staged_relative_for_target,
};
pub(crate) type FileOpReconcileSummary = reconcile::FileOpReconcileSummary;
#[cfg(test)]
pub(crate) type ListedJournalEntries = store::ListedJournalEntries;

/// Insert a new journal entry before mutating the filesystem.
pub(crate) fn insert_entry(
    db: &SourceDatabase,
    entry: &FileOpJournalEntry,
) -> Result<(), SourceDbError> {
    store::insert_entry(db, entry)
}

/// Update a journal entry stage and optional metadata after filesystem work.
pub(crate) fn update_stage(
    db: &SourceDatabase,
    id: &str,
    stage: FileOpStage,
    file_size: Option<u64>,
    modified_ns: Option<i64>,
) -> Result<(), SourceDbError> {
    store::update_stage(db, id, stage, file_size, modified_ns)
}

/// Remove a resolved journal entry after reconciliation.
pub(crate) fn remove_entry(db: &SourceDatabase, id: &str) -> Result<(), SourceDbError> {
    store::remove_entry(db, id)
}

/// Load all pending journal entries for reconciliation.
#[cfg(test)]
pub(crate) fn list_entries(db: &SourceDatabase) -> Result<ListedJournalEntries, SourceDbError> {
    store::list_entries(db)
}

/// Reconcile all pending file ops against the filesystem and database.
pub(crate) fn reconcile_pending_ops(db: &SourceDatabase) -> Result<FileOpReconcileSummary, String> {
    reconcile::reconcile_pending_ops(db)
}
