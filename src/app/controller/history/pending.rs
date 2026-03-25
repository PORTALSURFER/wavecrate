//! Pending async history transaction payloads.
//!
//! These types are shared by the controller history state and the
//! selection-export / normalization workflows, so they live beside the
//! main history module while keeping the snapshot logic file focused.

use super::*;

/// Stable key for one async history transaction awaiting completion.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum PendingHistoryTransactionKey {
    /// Background normalization for one concrete sample path.
    Normalization {
        /// Source owning the normalized sample.
        source_id: SourceId,
        /// Relative path of the normalized sample.
        relative_path: PathBuf,
    },
    /// Background selection export identified by the request id.
    SelectionExport {
        /// Monotonic request id for the queued export job.
        request_id: u64,
    },
}

/// Pending async history entry for one sample-overwrite workflow.
#[derive(Clone, Debug)]
pub(crate) struct PendingSampleOverwriteTransaction {
    /// Undo label to show in the history stack.
    pub label: String,
    /// Snapshot captured before the async edit started.
    pub before: MeaningfulUiSnapshot,
    /// Source owning the overwritten sample.
    pub source_id: SourceId,
    /// Relative path of the overwritten sample.
    pub relative_path: PathBuf,
    /// Absolute path of the overwritten sample.
    pub absolute_path: PathBuf,
    /// Backups captured before the async overwrite completes.
    pub backup: undo::OverwriteBackup,
}

/// Pending async history entry for one sample-creation workflow.
#[derive(Clone, Debug)]
pub(crate) struct PendingSampleCreationTransaction {
    /// Undo label to show in the history stack.
    pub label: String,
    /// Snapshot captured before the async create started.
    pub before: MeaningfulUiSnapshot,
}

/// Async history payload captured before a background job mutates controller-owned state.
#[derive(Clone, Debug)]
pub(crate) enum PendingHistoryTransaction {
    /// A background job will overwrite an existing sample in place.
    SampleOverwrite(PendingSampleOverwriteTransaction),
    /// A background job will create a brand-new sample file.
    SampleCreation(PendingSampleCreationTransaction),
}
