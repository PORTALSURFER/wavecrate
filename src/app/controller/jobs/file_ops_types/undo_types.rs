//! Undo/redo DTOs for filesystem mutations coordinated by the controller.

use super::*;

/// Request for a background undo/redo filesystem operation.
#[derive(Debug, Clone)]
pub(crate) enum UndoFileJob {
    /// Overwrite an existing file with a backup copy.
    Overwrite {
        /// Source identifier for the sample.
        source_id: crate::sample_sources::SourceId,
        /// Root folder for the source.
        source_root: PathBuf,
        /// Relative path of the sample.
        relative_path: PathBuf,
        /// Absolute destination path to overwrite.
        absolute_path: PathBuf,
        /// Backup file to copy from.
        backup_path: PathBuf,
    },
    /// Remove a sample file and drop its database entry.
    RemoveSample {
        /// Source identifier for the sample.
        source_id: crate::sample_sources::SourceId,
        /// Root folder for the source.
        source_root: PathBuf,
        /// Relative path of the sample.
        relative_path: PathBuf,
        /// Absolute path to delete.
        absolute_path: PathBuf,
    },
    /// Restore a sample file from backup and update its database entry.
    RestoreSample {
        /// Source identifier for the sample.
        source_id: crate::sample_sources::SourceId,
        /// Root folder for the source.
        source_root: PathBuf,
        /// Relative path of the sample.
        relative_path: PathBuf,
        /// Absolute destination path to restore.
        absolute_path: PathBuf,
        /// Backup file to copy from.
        backup_path: PathBuf,
        /// Tag to apply after restoration.
        tag: crate::sample_sources::Rating,
        /// Loop marker state to restore with the sample.
        looped: bool,
        /// Last played timestamp to restore with the sample.
        last_played_at: Option<i64>,
        /// Normal library tag labels to restore with the sample.
        normal_tags: Vec<String>,
    },
}

/// Result of a background undo/redo filesystem operation.
#[derive(Debug)]
pub(crate) struct UndoFileOpResult {
    /// Result of the filesystem operation.
    pub(crate) result: Result<UndoFileOutcome, String>,
    /// Whether the operation was cancelled by the user.
    pub(crate) cancelled: bool,
}

/// Outcome details for an undo/redo filesystem operation.
#[derive(Debug)]
pub(crate) enum UndoFileOutcome {
    /// File overwrite completed with updated metadata.
    Overwrite {
        /// Source identifier for the sample.
        source_id: crate::sample_sources::SourceId,
        /// Relative path of the sample.
        relative_path: PathBuf,
        /// File size in bytes.
        file_size: u64,
        /// Modified time as epoch nanoseconds.
        modified_ns: i64,
        /// Tag associated with the sample.
        tag: crate::sample_sources::Rating,
        /// Loop marker state.
        looped: bool,
        /// Last played timestamp, if any.
        last_played_at: Option<i64>,
        /// Normal library tag labels assigned after overwrite.
        normal_tags: Vec<String>,
    },
    /// File removal completed.
    Removed {
        /// Source identifier for the sample.
        source_id: crate::sample_sources::SourceId,
        /// Relative path of the sample.
        relative_path: PathBuf,
    },
    /// File restoration completed with updated metadata.
    Restored {
        /// Source identifier for the sample.
        source_id: crate::sample_sources::SourceId,
        /// Relative path of the sample.
        relative_path: PathBuf,
        /// File size in bytes.
        file_size: u64,
        /// Modified time as epoch nanoseconds.
        modified_ns: i64,
        /// Tag associated with the sample.
        tag: crate::sample_sources::Rating,
        /// Loop marker state.
        looped: bool,
        /// Last played timestamp, if any.
        last_played_at: Option<i64>,
        /// Normal library tag labels restored with the sample.
        normal_tags: Vec<String>,
    },
}
