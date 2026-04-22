use super::*;
use crate::app::state::ProgressTaskKind;

impl AppController {
    /// Show status-bar progress without the modal overlay.
    pub(crate) fn show_status_progress(
        &mut self,
        task: ProgressTaskKind,
        title: impl Into<String>,
        total: usize,
        cancelable: bool,
    ) {
        self.ui
            .progress
            .show_task(task, false, title, total, cancelable);
    }

    /// Update the current progress detail label without changing counts.
    pub(crate) fn update_progress_detail(&mut self, detail: impl Into<String>) {
        if self.ui.progress.visible {
            self.ui.progress.set_detail(Some(detail.into()));
        }
    }

    /// Update one task's progress detail label without changing counts.
    pub(crate) fn update_progress_detail_for_task(
        &mut self,
        task: ProgressTaskKind,
        detail: impl Into<String>,
    ) {
        self.ui.progress.set_task_detail(task, Some(detail.into()));
    }

    /// Update the current progress title when the expected task owns the footer lane.
    pub(crate) fn update_status_progress_title(
        &mut self,
        task: ProgressTaskKind,
        title: impl Into<String>,
    ) {
        self.ui.progress.set_task_title(task, title);
    }

    /// Clear any active progress overlay.
    pub(crate) fn clear_progress(&mut self) {
        self.ui.progress.reset();
    }

    /// Remove one task from the shared footer-progress contract.
    pub(crate) fn clear_progress_task(&mut self, task: ProgressTaskKind) {
        self.ui.progress.clear_task(task);
    }

    /// Request cancellation of the active progress task.
    pub(crate) fn request_progress_cancel(&mut self) {
        if let Some(task) = self.ui.progress.task {
            self.ui.progress.request_task_cancel(task);
        }
    }
}
