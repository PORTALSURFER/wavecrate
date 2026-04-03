//! Background-job polling loop and message-specific handler modules.

mod audio;
mod helpers;
mod library_handlers;
mod message_router;
mod runtime_handlers;

use super::*;

/// Maximum queued background messages to apply during one controller poll pass.
const MAX_BACKGROUND_MESSAGES_PER_POLL: usize = 32;

impl AppController {
    /// Drain queued background job messages and apply their side effects.
    pub(crate) fn poll_background_jobs(&mut self) {
        self.apply_progress_cancel_request();
        if self.has_pending_browser_focus_commit() {
            self.flush_pending_browser_focus_commit();
        }
        let mut applied = 0usize;
        while applied < MAX_BACKGROUND_MESSAGES_PER_POLL {
            let Some(message) = self.try_next_background_job_message() else {
                return;
            };
            self.handle_background_job_message(message);
            applied += 1;
        }
        self.runtime.jobs.request_repaint();
    }

    /// Propagate any UI-issued cancellation request to the active worker.
    fn apply_progress_cancel_request(&mut self) {
        if !self.ui.progress.cancel_requested {
            return;
        }
        match helpers::cancel_request_action(self.ui.progress.task) {
            helpers::CancelRequestAction::TrashMove => {
                if let Some(cancel) = self.runtime.jobs.trash_move_cancel().as_ref() {
                    cancel.store(true, Ordering::Relaxed);
                }
            }
            helpers::CancelRequestAction::Scan => {
                if let Some(cancel) = self.runtime.jobs.scan_cancel().as_ref() {
                    cancel.store(true, Ordering::Relaxed);
                }
            }
            helpers::CancelRequestAction::Analysis => {
                self.runtime.analysis.cancel();
                self.clear_progress();
            }
            helpers::CancelRequestAction::FileOps => {
                if let Some(cancel) = self.runtime.jobs.file_ops_cancel().as_ref() {
                    cancel.store(true, Ordering::Relaxed);
                }
            }
            helpers::CancelRequestAction::None => {}
        }
    }

    /// Try to dequeue the next controller background-job message.
    fn try_next_background_job_message(&self) -> Option<JobMessage> {
        self.runtime.jobs.try_recv_message().ok()
    }
}

#[cfg(test)]
mod tests;
