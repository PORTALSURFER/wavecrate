//! Trash-move and file-operation dispatch stream lifecycle helpers.

use super::*;

impl ControllerJobs {
    /// Return whether a trash-move job is currently running.
    pub(in super::super::super) fn trash_move_in_progress(&self) -> bool {
        self.in_progress.trash_move
    }

    /// Begin forwarding trash-move progress from a background worker.
    pub(in super::super::super) fn start_trash_move(
        &mut self,
        rx: Receiver<trash_move::TrashMoveMessage>,
        cancel: Arc<AtomicBool>,
    ) {
        self.in_progress.trash_move = true;
        self.cancel_handles.trash_move = Some(cancel);
        self.start_progress_stream(rx, JobMessage::TrashMove, trash_move_message_is_finished);
    }

    /// Return the cooperative cancel handle for the active trash move.
    pub(in super::super::super) fn trash_move_cancel(&self) -> Option<Arc<AtomicBool>> {
        self.cancel_handles.trash_move.clone()
    }

    /// Clear the in-progress state for the current trash-move job.
    pub(in super::super::super) fn clear_trash_move(&mut self) {
        self.in_progress.trash_move = false;
        self.cancel_handles.trash_move = None;
    }

    /// Return whether a background file operation is currently running.
    pub(in super::super::super) fn file_ops_in_progress(&self) -> bool {
        self.in_progress.file_ops
    }

    /// Begin forwarding file operation progress messages from a background worker.
    pub(in super::super::super) fn start_file_ops(
        &mut self,
        rx: Receiver<FileOpMessage>,
        cancel: Arc<AtomicBool>,
    ) {
        self.in_progress.file_ops = true;
        self.cancel_handles.file_ops = Some(cancel);
        self.start_progress_stream(rx, JobMessage::FileOps, file_op_message_is_finished);
    }

    /// Queue one one-shot file operation onto the reusable file-op worker lane.
    pub(in super::super::super) fn begin_one_shot_file_op(
        &mut self,
        run: impl FnOnce(Arc<AtomicBool>) -> FileOpResult + Send + 'static,
    ) -> Result<(), String> {
        if self.in_progress.file_ops {
            return Err("File operation already in progress".to_string());
        }
        let cancel = Arc::new(AtomicBool::new(false));
        self.in_progress.file_ops = true;
        self.cancel_handles.file_ops = Some(cancel.clone());
        if let Err(err) = self
            .file_op_worker
            .send(file_op_worker::QueuedFileOpTask::new(cancel, run))
        {
            self.in_progress.file_ops = false;
            self.cancel_handles.file_ops = None;
            return Err(err);
        }
        Ok(())
    }

    /// Queue one one-shot file operation that may stream best-effort progress updates.
    pub(in super::super::super) fn begin_one_shot_file_op_with_progress(
        &mut self,
        run: impl FnOnce(Arc<AtomicBool>, FileOpProgressSender) -> FileOpResult + Send + 'static,
    ) -> Result<(), String> {
        if self.in_progress.file_ops {
            return Err("File operation already in progress".to_string());
        }
        let cancel = Arc::new(AtomicBool::new(false));
        self.in_progress.file_ops = true;
        self.cancel_handles.file_ops = Some(cancel.clone());
        if let Err(err) =
            self.file_op_worker
                .send(file_op_worker::QueuedFileOpTask::new_with_progress(
                    cancel, run,
                ))
        {
            self.in_progress.file_ops = false;
            self.cancel_handles.file_ops = None;
            return Err(err);
        }
        Ok(())
    }

    /// Return the cooperative cancel handle for the active file operation.
    pub(in super::super::super) fn file_ops_cancel(&self) -> Option<Arc<AtomicBool>> {
        self.cancel_handles.file_ops.clone()
    }

    /// Clear the in-progress state for the current file operation job.
    pub(in super::super::super) fn clear_file_ops(&mut self) {
        self.in_progress.file_ops = false;
        self.cancel_handles.file_ops = None;
    }
}

fn trash_move_message_is_finished(message: &trash_move::TrashMoveMessage) -> bool {
    matches!(message, trash_move::TrashMoveMessage::Finished(_))
}

fn file_op_message_is_finished(message: &FileOpMessage) -> bool {
    matches!(message, FileOpMessage::Finished(_))
}
