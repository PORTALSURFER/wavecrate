//! Startup recovery for staged folder deletes.
//!
//! Recovery walks each source-local staging directory and applies the journal contract:
//! - `Intent` or `Staged` means the original folder should exist after recovery
//! - `Deleted` means the staged folder should remain retained as app-owned trash unless
//!   purge already removed the staged folder, in which case recovery finalizes the stale row
//! - `RestorePendingDb` means an explicit retained restore must finish merge/DB replay
//! - staged folders that exist without journal entries are conservatively restored
//! - unreadable journal files leave staging untouched so retained deletes are not misclassified
use super::DELETE_STAGING_DIR;
use super::DeleteStagingInfo;
use super::journal::{
    DeleteJournal, DeleteJournalEntry, DeleteJournalStage, load_journal, remove_entry,
};
use super::restore_merge::restore_retained_folder_with_merge_with_stamp;
use super::retained_restore_reconcile::{
    apply_retained_restore_db_entries, infer_retained_restore_merge_report,
    snapshot_existing_restore_entries,
};
use crate::sample_sources::{SampleSource, SourceId, WavEntry};
use std::{
    fs,
    path::{Path, PathBuf},
};

mod actions;
mod journaled;
mod orchestration;
mod retained;
mod scan;
mod unjournaled;

#[cfg(test)]
use actions::unique_restore_path;
use actions::{recovery_entry, restore_staged_folder};
use journaled::{JournaledRecovery, JournaledRecoveryOutcome, RetainedRecovery};
use scan::{find_unjournaled_staged_roots, journaled_staged_roots};

const DELETE_JOURNAL_FILE: &str = "delete_journal.json";

/// Recovery action taken for a staged delete.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeleteRecoveryAction {
    /// Move the staged folder back into the source tree.
    Restore,
    /// Permanently delete the staged folder.
    Finalize,
}

/// Outcome for a recovery attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeleteRecoveryStatus {
    /// Recovery action succeeded.
    Completed,
    /// Recovery action failed and needs attention.
    Failed,
}

/// Per-folder recovery result for UI reporting.
#[derive(Debug, Clone)]
pub(crate) struct DeleteRecoveryEntry {
    /// Source identifier for the staged folder.
    pub(crate) source_id: SourceId,
    /// Source root for display and follow-up refreshes.
    pub(crate) source_root: PathBuf,
    /// Original relative path within the source.
    pub(crate) original_relative: PathBuf,
    /// Action taken during recovery.
    pub(crate) action: DeleteRecoveryAction,
    /// Outcome of the action.
    pub(crate) status: DeleteRecoveryStatus,
    /// Optional extra detail for the UI.
    pub(crate) detail: Option<String>,
}

/// Retained staged delete that remains recoverable after startup reconciliation.
#[derive(Debug, Clone)]
pub(crate) struct RetainedDeleteEntry {
    /// Stable journal identifier for the retained delete.
    pub(crate) id: String,
    /// Source identifier that owns the retained delete.
    pub(crate) source_id: SourceId,
    /// Source root for restore or purge follow-up work.
    pub(crate) source_root: PathBuf,
    /// Original relative folder path within the source.
    pub(crate) original_relative: PathBuf,
    /// Relative staged path inside `.wavecrate_delete_staging`.
    pub(crate) staged_relative: PathBuf,
    /// Deleted wav metadata snapshot used to reconstruct DB state after restart.
    pub(crate) deleted_entries: Vec<WavEntry>,
}

/// Summary of staged delete recovery across all sources.
#[derive(Debug, Default)]
pub(crate) struct DeleteRecoveryReport {
    /// Per-folder recovery outcomes.
    pub(crate) entries: Vec<DeleteRecoveryEntry>,
    /// Retained deletes that remain available for explicit restore or purge.
    pub(crate) retained_entries: Vec<RetainedDeleteEntry>,
    /// Sources that need a follow-up hard sync after startup recovery.
    pub(crate) scan_sources: Vec<SourceId>,
    /// Non-fatal errors encountered during recovery.
    pub(crate) errors: Vec<String>,
}

/// Recover staged deletes for the provided sources.
pub(crate) fn recover_staged_deletes(sources: &[SampleSource]) -> DeleteRecoveryReport {
    orchestration::recover_staged_deletes(sources)
}

#[cfg(test)]
mod tests;
