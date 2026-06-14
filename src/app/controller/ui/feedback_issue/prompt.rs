use super::super::*;
use super::logging::emit_feedback_issue_action;
use std::time::Instant;

impl AppController {
    pub(crate) fn open_feedback_issue_prompt(&mut self) {
        let started_at = Instant::now();
        self.ui.feedback_issue.open = true;
        self.ui.feedback_issue.focus_title_requested = true;
        self.ui.feedback_issue.last_error = None;
        self.ui.feedback_issue.last_success_url = None;
        self.ui.feedback_issue.token_autofill_last = None;
        self.ui.feedback_issue.connecting = false;
        self.ui.feedback_issue.token_status = crate::app::state::IssueTokenStatus::Unknown;
        self.ui.feedback_issue.token_cached = None;
        self.start_issue_token_load();
        emit_feedback_issue_action(
            "feedback_issue.open_prompt",
            None,
            "success",
            started_at,
            None,
        );
    }

    pub(crate) fn close_feedback_issue_prompt(&mut self) {
        let started_at = Instant::now();
        self.ui.feedback_issue.open = false;
        self.ui.feedback_issue.submitting = false;
        self.ui.feedback_issue.focus_title_requested = false;
        self.ui.feedback_issue.token_modal_open = false;
        self.ui.feedback_issue.focus_token_requested = false;
        self.ui.feedback_issue.token_autofill_last = None;
        self.ui.feedback_issue.connecting = false;
        self.ui.feedback_issue.last_error = None;
        self.ui.feedback_issue.last_success_url = None;
        self.runtime.jobs.clear_issue_gateway_poll();
        emit_feedback_issue_action(
            "feedback_issue.close_prompt",
            None,
            "success",
            started_at,
            None,
        );
    }
}
