use crate::sample_sources::WavEntry;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Journal stage for a staged folder delete.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DeleteJournalStage {
    /// Intent was recorded before the filesystem rename completed.
    Intent,
    /// Folder data has been moved into the staging area.
    Staged,
    /// Database state was committed, so the staged folder now represents an app-owned delete.
    #[serde(alias = "db_committed")]
    Deleted,
    /// Filesystem restore started and DB metadata replay still needs durable completion.
    RestorePendingDb,
}

/// Metadata for a folder staged for deletion.
#[derive(Debug, Clone)]
pub(crate) struct DeleteStagingInfo {
    /// Unique journal identifier for this staged delete.
    pub(crate) id: String,
    /// Relative path of the original folder within the source.
    pub(crate) original_relative: PathBuf,
    /// Relative path inside the staging root.
    pub(crate) staged_relative: PathBuf,
    /// Absolute staged path on disk.
    pub(crate) staged_absolute: PathBuf,
}

/// Persistent journal entry for a staged folder delete.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DeleteJournalEntry {
    pub(crate) id: String,
    pub(crate) original_relative: String,
    pub(crate) staged_relative: String,
    #[serde(default)]
    pub(crate) deleted_entries: Vec<WavEntry>,
    pub(crate) stage: DeleteJournalStage,
    #[serde(default)]
    pub(crate) restore_stamp: Option<String>,
    pub(crate) created_at: i64,
}

/// Journal container stored on disk for one source root.
#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct DeleteJournal {
    pub(crate) entries: Vec<DeleteJournalEntry>,
}
