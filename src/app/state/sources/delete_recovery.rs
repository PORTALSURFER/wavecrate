use crate::sample_sources::{SourceId, WavEntry};
use std::path::PathBuf;

/// UI state for staged delete recovery.
#[derive(Clone, Debug, Default)]
pub struct FolderDeleteRecoveryUiState {
    /// Whether recovery is currently running in the background.
    pub in_progress: bool,
    /// Entries reported by the last recovery run.
    pub entries: Vec<FolderDeleteRecoveryEntry>,
    /// Retained folder deletes that can still be restored or purged explicitly.
    pub retained_entries: Vec<RetainedFolderDeleteEntry>,
}

/// Display entry for a recovered staged delete.
#[derive(Clone, Debug)]
pub struct FolderDeleteRecoveryEntry {
    /// Display label for the source.
    pub source_label: String,
    /// Original folder path relative to the source root.
    pub relative_path: PathBuf,
    /// Action taken during recovery.
    pub action: FolderDeleteRecoveryAction,
    /// Outcome of the recovery attempt.
    pub status: FolderDeleteRecoveryStatus,
    /// Optional extra detail for the UI.
    pub detail: Option<String>,
}

/// Recoverable retained folder delete stored in the app-owned staging area.
#[derive(Clone, Debug)]
pub struct RetainedFolderDeleteEntry {
    /// Stable journal identifier for the retained delete.
    pub id: String,
    /// Source identifier that owns the retained delete.
    pub source_id: SourceId,
    /// Source root path that owns the retained delete.
    pub source_root: PathBuf,
    /// Display label for the source in the UI.
    pub source_label: String,
    /// Original folder path relative to the source root.
    pub relative_path: PathBuf,
    /// Relative path of the staged folder inside `.wavecrate_delete_staging`.
    pub staged_relative: PathBuf,
    /// Snapshot of deleted wav metadata used to restore DB state after restart.
    pub deleted_entries: Vec<WavEntry>,
}

/// Recovery action taken for a staged delete.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FolderDeleteRecoveryAction {
    /// Restore the staged folder into the source.
    Restore,
    /// Finalize the staged delete by removing the folder.
    Finalize,
}

/// Recovery outcome for a staged delete.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FolderDeleteRecoveryStatus {
    /// Recovery action succeeded.
    Completed,
    /// Recovery action failed.
    Failed,
}
