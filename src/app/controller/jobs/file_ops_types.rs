//! File-operation request/result DTOs used by drag/drop and undo jobs.

use super::*;

/// Progress updates for file operations that should not block the UI thread.
#[derive(Debug)]
pub(crate) enum FileOpMessage {
    /// Incremental progress update for the active file operation.
    Progress {
        /// Completed steps so far.
        completed: usize,
        /// Optional per-item detail label.
        detail: Option<String>,
    },
    /// Final result for the file operation.
    Finished(FileOpResult),
}

/// Outcome for a file operation job.
#[derive(Debug)]
pub(crate) enum FileOpResult {
    /// Clipboard paste or import results.
    ClipboardPaste(ClipboardPasteResult),
    /// Explicit retained-delete restore/purge results.
    RetainedDeleteResolution(RetainedDeleteResolutionResult),
    /// Drop-target copy or move results.
    DropTargetTransfer(DropTargetTransferResult),
    /// Source move results from drag/drop actions.
    SourceMove(SourceMoveResult),
    /// In-source sample move results from folder drag/drop actions.
    FolderSampleMove(FolderSampleMoveResult),
    /// Folder move results from drag/drop actions.
    FolderMove(FolderMoveResult),
    /// Undo/redo filesystem results.
    UndoFile(UndoFileOpResult),
}

/// Copy-vs-move mode for a background drop-target transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DropTargetTransferKind {
    /// Duplicate the source file into the target folder.
    Copy,
    /// Relocate the source file into the target folder.
    Move,
}

impl DropTargetTransferKind {
    /// Human-readable past-tense verb for status reporting.
    pub(crate) const fn action_past_tense(self) -> &'static str {
        match self {
            Self::Copy => "Copied",
            Self::Move => "Moved",
        }
    }

    /// Human-readable present-participle verb for progress detail.
    pub(crate) const fn action_present_participle(self) -> &'static str {
        match self {
            Self::Copy => "Copying",
            Self::Move => "Moving",
        }
    }
}

/// Request payload for a background drop-target copy or move.
#[derive(Debug, Clone)]
pub(crate) struct DropTargetTransferRequest {
    /// Source identifier for the sample.
    pub(crate) source_id: crate::sample_sources::SourceId,
    /// Root folder for the source.
    pub(crate) source_root: PathBuf,
    /// Relative path of the sample to transfer.
    pub(crate) relative_path: PathBuf,
    /// Cache-backed metadata captured on the controller when available.
    pub(crate) metadata: Option<DropTargetTransferMetadata>,
}

/// Metadata snapshot captured before a drop-target transfer starts.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DropTargetTransferMetadata {
    /// Tag associated with the sample.
    pub(crate) tag: crate::sample_sources::Rating,
    /// Loop marker state.
    pub(crate) looped: bool,
    /// Keep/lock marker state.
    pub(crate) locked: bool,
    /// Last played timestamp, if any.
    pub(crate) last_played_at: Option<i64>,
}

/// Result of a background drop-target copy or move operation.
#[derive(Debug)]
pub(crate) struct DropTargetTransferResult {
    /// Transfer mode used for the batch.
    pub(crate) kind: DropTargetTransferKind,
    /// Target source identifier for the transfer.
    pub(crate) target_source_id: crate::sample_sources::SourceId,
    /// Human-readable label for the destination folder.
    pub(crate) target_label: String,
    /// Successfully transferred entries with metadata.
    pub(crate) transferred: Vec<DropTargetTransferSuccess>,
    /// Errors encountered during the transfer.
    pub(crate) errors: Vec<String>,
    /// Whether the operation was cancelled by the user.
    pub(crate) cancelled: bool,
}

/// Record for a successfully copied or moved sample into a drop target.
#[derive(Debug)]
pub(crate) struct DropTargetTransferSuccess {
    /// Original source identifier.
    pub(crate) source_id: crate::sample_sources::SourceId,
    /// Original relative path.
    pub(crate) source_relative: PathBuf,
    /// New relative path at the destination.
    pub(crate) target_relative: PathBuf,
    /// File size in bytes.
    pub(crate) file_size: u64,
    /// Modified time as epoch nanoseconds.
    pub(crate) modified_ns: i64,
    /// Tag associated with the sample.
    pub(crate) tag: crate::sample_sources::Rating,
    /// Loop marker state.
    pub(crate) looped: bool,
    /// Keep/lock marker state.
    pub(crate) locked: bool,
    /// Last played timestamp, if any.
    pub(crate) last_played_at: Option<i64>,
}

/// Successful paste into a source folder with metadata for follow-up updates.
#[derive(Debug)]
pub(crate) struct SourcePasteAdded {
    /// Relative path of the added sample within the source root.
    pub(crate) relative_path: PathBuf,
    /// File size in bytes.
    pub(crate) file_size: u64,
    /// Modified time as epoch nanoseconds.
    pub(crate) modified_ns: i64,
}

/// Result of pasting or importing files from the clipboard into a target.
#[derive(Debug)]
pub(crate) struct ClipboardPasteResult {
    /// Destination that received the pasted files.
    pub(crate) outcome: ClipboardPasteOutcome,
    /// Number of skipped files that were unsupported or missing.
    pub(crate) skipped: usize,
    /// Errors encountered while processing files.
    pub(crate) errors: Vec<String>,
    /// Whether the operation was cancelled by the user.
    pub(crate) cancelled: bool,
    /// Human-readable label for the target destination.
    pub(crate) target_label: String,
    /// Past-tense label for status reporting (e.g., "Pasted", "Imported").
    pub(crate) action_past_tense: &'static str,
}

/// Target-specific clipboard paste outcomes.
#[derive(Debug)]
pub(crate) enum ClipboardPasteOutcome {
    /// Paste into a source folder.
    Source {
        /// Source receiving the files.
        source_id: crate::sample_sources::SourceId,
        /// Added samples with metadata.
        added: Vec<SourcePasteAdded>,
    },
}

/// Request payload for a background source move operation.
#[derive(Debug, Clone)]
pub(crate) struct SourceMoveRequest {
    /// Source identifier for the sample.
    pub(crate) source_id: crate::sample_sources::SourceId,
    /// Root folder for the source.
    pub(crate) source_root: PathBuf,
    /// Relative path of the sample to move.
    pub(crate) relative_path: PathBuf,
}

/// Result of a background source move operation.
#[derive(Debug)]
pub(crate) struct SourceMoveResult {
    /// Target source identifier for the move.
    pub(crate) target_source_id: crate::sample_sources::SourceId,
    /// Successful moves with metadata.
    pub(crate) moved: Vec<SourceMoveSuccess>,
    /// Errors encountered during the move.
    pub(crate) errors: Vec<String>,
    /// Whether the operation was cancelled by the user.
    pub(crate) cancelled: bool,
}

/// Record for a successfully moved sample.
#[derive(Debug)]
pub(crate) struct SourceMoveSuccess {
    /// Original source identifier.
    pub(crate) source_id: crate::sample_sources::SourceId,
    /// Original relative path.
    pub(crate) relative_path: PathBuf,
    /// New relative path at the destination.
    pub(crate) target_relative: PathBuf,
    /// File size in bytes.
    pub(crate) file_size: u64,
    /// Modified time as epoch nanoseconds.
    pub(crate) modified_ns: i64,
    /// Tag associated with the sample.
    pub(crate) tag: crate::sample_sources::Rating,
    /// Loop marker state.
    pub(crate) looped: bool,
    /// Keep/lock marker state.
    pub(crate) locked: bool,
    /// Last played timestamp, if any.
    pub(crate) last_played_at: Option<i64>,
}

/// Request payload for a background in-source folder sample move.
#[derive(Debug, Clone)]
pub(crate) struct FolderSampleMoveRequest {
    /// Relative path of the sample to move.
    pub(crate) relative_path: PathBuf,
    /// Relative destination path within the same source.
    pub(crate) target_relative: PathBuf,
}

/// Metadata describing a moved entry within a source.
#[derive(Debug, Clone)]
pub(crate) struct FolderEntryMove {
    /// Original relative path before the move.
    pub(crate) old_relative: PathBuf,
    /// New relative path after the move.
    pub(crate) new_relative: PathBuf,
    /// File size in bytes.
    pub(crate) file_size: u64,
    /// Modified time as epoch nanoseconds.
    pub(crate) modified_ns: i64,
    /// Tag associated with the sample.
    pub(crate) tag: crate::sample_sources::Rating,
    /// Loop marker state.
    pub(crate) looped: bool,
    /// Keep/lock marker state.
    pub(crate) locked: bool,
    /// Last played timestamp, if any.
    pub(crate) last_played_at: Option<i64>,
}

/// Result of a background in-source folder sample move operation.
#[derive(Debug)]
pub(crate) struct FolderSampleMoveResult {
    /// Source identifier for the moved samples.
    pub(crate) source_id: crate::sample_sources::SourceId,
    /// Successful moves with metadata.
    pub(crate) moved: Vec<FolderEntryMove>,
    /// Errors encountered during the move.
    pub(crate) errors: Vec<String>,
    /// Whether the operation was cancelled by the user.
    pub(crate) cancelled: bool,
}

/// Request payload for a background folder move within a source.
#[derive(Debug, Clone)]
pub(crate) struct FolderMoveRequest {
    /// Source identifier for the folder.
    pub(crate) source_id: crate::sample_sources::SourceId,
    /// Root folder for the source.
    pub(crate) source_root: PathBuf,
    /// Folder path relative to the source root.
    pub(crate) folder: PathBuf,
    /// Target parent folder relative to the source root.
    pub(crate) target_folder: PathBuf,
}

/// Result of a background folder move within a source.
#[derive(Debug)]
pub(crate) struct FolderMoveResult {
    /// Source identifier for the moved folder.
    pub(crate) source_id: crate::sample_sources::SourceId,
    /// Original folder path relative to the source root.
    pub(crate) old_folder: PathBuf,
    /// New folder path relative to the source root.
    pub(crate) new_folder: PathBuf,
    /// True when the folder move completed successfully.
    pub(crate) folder_moved: bool,
    /// Successful entry moves with metadata.
    pub(crate) moved: Vec<FolderEntryMove>,
    /// Errors encountered during the move.
    pub(crate) errors: Vec<String>,
    /// Whether the operation was cancelled by the user.
    pub(crate) cancelled: bool,
}

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
    },
}
