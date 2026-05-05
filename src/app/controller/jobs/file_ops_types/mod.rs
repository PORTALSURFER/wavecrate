//! File-operation request/result DTOs used by drag/drop, edits, and undo jobs.

use super::*;

mod browser_types;
mod edit_types;
mod folder_types;
mod transfer_types;
mod undo_types;

pub(crate) use self::browser_types::*;
pub(crate) use self::edit_types::*;
pub(crate) use self::folder_types::*;
pub(crate) use self::transfer_types::*;
pub(crate) use self::undo_types::*;

/// Progress updates for file operations that should not block the UI thread.
#[derive(Debug)]
pub(crate) enum FileOpMessage {
    /// Incremental progress update for the active file operation.
    Progress {
        /// Completed steps so far.
        completed: usize,
        /// Optional per-item detail label.
        detail: Option<String>,
        /// Optional structured item progress for operation-specific UI state.
        item: Option<SampleAutoRenameProgress>,
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
    /// Browser sample delete results.
    SampleDelete(SampleDeleteResult),
    /// Browser sample rename results.
    SampleRename(SampleRenameResult),
    /// Browser batch auto-rename results.
    SampleAutoRename(SampleAutoRenameResult),
    /// Folder creation results.
    FolderCreate(FolderCreateResult),
    /// Folder rename results.
    FolderRename(FolderRenameResult),
    /// Folder delete results.
    FolderDelete(FolderDeleteResult),
    /// Destructive in-place selection edit results.
    SelectionEditCommit(SelectionEditCommitResult),
    /// Circular waveform slide overwrite results.
    WaveformSlideCommit(WaveformSlideCommitResult),
    /// Undo/redo filesystem results.
    UndoFile(UndoFileOpResult),
}
