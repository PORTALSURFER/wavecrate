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
        self.ui.progress = ProgressOverlayState::new(task, title, total, cancelable);
        self.ui.progress.modal = false;
    }

    /// Update the current progress detail label without changing counts.
    pub(crate) fn update_progress_detail(&mut self, detail: impl Into<String>) {
        if self.ui.progress.visible {
            self.ui.progress.set_detail(Some(detail.into()));
        }
    }

    /// Update the current progress title when the expected task owns the footer lane.
    pub(crate) fn update_status_progress_title(
        &mut self,
        task: ProgressTaskKind,
        title: impl Into<String>,
    ) {
        if self.ui.progress.task == Some(task) {
            self.ui.progress.set_title(title);
        }
    }

    /// Clear any active progress overlay.
    pub(crate) fn clear_progress(&mut self) {
        self.ui.progress.reset();
    }

    /// Request cancellation of the active progress task.
    pub(crate) fn request_progress_cancel(&mut self) {
        if self.ui.progress.cancelable {
            self.ui.progress.cancel_requested = true;
        }
    }
}
