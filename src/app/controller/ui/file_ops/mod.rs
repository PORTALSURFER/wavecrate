//! Apply background file operation results to controller state.

use super::*;
use crate::app::controller::jobs::{
    ClipboardPasteOutcome, ClipboardPasteResult, FileOpMessage, FileOpResult, FolderCreateResult,
    FolderDeleteResult, FolderRenameResult, SampleAutoRenameResult, SampleDeleteResult,
    SampleRenameResult, SelectionEditCommitResult, UndoFileOpResult, UndoFileOutcome,
    WaveformSlideCommitResult,
};
use crate::app::controller::undo::{DeferredUndo, UndoDirection};
use crate::app::controller::undo_jobs;
use std::sync::{Arc, atomic::AtomicBool};
use tracing::warn;

mod browser_apply;
mod clipboard_undo;
mod edit_apply;
mod folder_apply;

impl AppController {
    /// Start a source-writing file-op stream after canceling any live remap generation.
    pub(crate) fn start_file_ops_with_remap_cancellation(
        &mut self,
        rx: std::sync::mpsc::Receiver<FileOpMessage>,
        cancel: Arc<AtomicBool>,
    ) {
        if let Some(source_id) = self
            .runtime
            .source_lane
            .pending_remap
            .as_ref()
            .filter(|pending| !pending.canceled)
            .map(|pending| pending.source.id.clone())
        {
            self.cancel_pending_source_remap_for_mutation(&source_id);
        }
        self.runtime.jobs.start_file_ops(rx, cancel);
    }

    /// Apply a completed background file operation to controller state.
    pub(crate) fn apply_file_op_result(&mut self, result: FileOpResult) {
        match result {
            FileOpResult::ClipboardPaste(result) => self.apply_clipboard_paste_result(result),
            FileOpResult::RetainedDeleteResolution(result) => {
                self.apply_retained_delete_resolution_result(result);
            }
            FileOpResult::DropTargetTransfer(result) => {
                self.drag_drop().apply_drop_target_transfer_result(result);
            }
            FileOpResult::SourceMove(result) => {
                self.drag_drop().apply_source_move_result(result);
            }
            FileOpResult::FolderSampleMove(result) => {
                self.drag_drop().apply_folder_sample_move_result(result);
            }
            FileOpResult::FolderMove(result) => {
                self.drag_drop().apply_folder_move_result(result);
            }
            FileOpResult::SampleDelete(result) => self.apply_sample_delete_result(result),
            FileOpResult::SampleRename(result) => self.apply_sample_rename_result(result),
            FileOpResult::SampleAutoRename(result) => self.apply_sample_auto_rename_result(result),
            FileOpResult::FolderCreate(result) => self.apply_folder_create_result(result),
            FileOpResult::FolderRename(result) => self.apply_folder_rename_result(result),
            FileOpResult::FolderDelete(result) => self.apply_folder_delete_result(result),
            FileOpResult::SelectionEditCommit(result) => {
                self.apply_selection_edit_commit_result(result);
            }
            FileOpResult::WaveformSlideCommit(result) => {
                self.apply_waveform_slide_commit_result(result);
            }
            FileOpResult::UndoFile(result) => self.apply_undo_file_result(result),
        }
    }

    /// Start a deferred undo/redo job and track its completion.
    pub(crate) fn begin_deferred_undo_job(&mut self, pending: DeferredUndo<AppController>) {
        let label = pending.entry.label.clone();
        let direction = pending.direction;
        let job = pending.job.clone();
        let title = match direction {
            UndoDirection::Undo => format!("Undoing {label}"),
            UndoDirection::Redo => format!("Redoing {label}"),
        };
        self.history.pending_undo = Some(pending);
        self.set_status(format!("{title}..."), StatusTone::Busy);
        self.show_status_progress(crate::app::state::ProgressTaskKind::FileOps, title, 1, true);
        let (tx, rx) = std::sync::mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        self.start_file_ops_with_remap_cancellation(rx, cancel.clone());
        std::thread::spawn(move || {
            let result = undo_jobs::run_undo_file_job(job, cancel, Some(&tx));
            let _ = tx.send(FileOpMessage::Finished(FileOpResult::UndoFile(result)));
        });
    }
}
