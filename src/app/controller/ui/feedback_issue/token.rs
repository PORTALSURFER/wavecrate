use super::super::*;
use super::logging::emit_feedback_issue_action;
use std::time::Instant;

impl AppController {
    pub(crate) fn save_github_issue_token(&mut self, token: &str) {
        self.persist_issue_token(token, true);
    }

    pub(crate) fn disconnect_github_issue_reporting(&mut self) {
        let started_at = Instant::now();
        if self.ui.feedback_issue.token_deleting {
            emit_feedback_issue_action(
                "feedback_issue.disconnect_github",
                None,
                "short_circuit",
                started_at,
                Some("delete_in_progress"),
            );
            return;
        }
        self.ui.feedback_issue.token_deleting = true;
        self.ui.feedback_issue.last_error = None;
        self.runtime.jobs.begin_issue_token_delete();
        emit_feedback_issue_action(
            "feedback_issue.disconnect_github",
            None,
            "queued",
            started_at,
            None,
        );
    }

    pub(super) fn persist_issue_token(&mut self, token: &str, reopen_modal: bool) -> bool {
        let token = token.trim();
        if token.len() < 20 {
            self.ui.feedback_issue.last_error =
                Some("Invalid token (must be at least 20 characters).".to_string());
            return false;
        }
        if self.ui.feedback_issue.token_saving {
            return false;
        }
        self.ui.feedback_issue.last_error = None;
        self.ui.feedback_issue.token_saving = true;
        self.runtime
            .jobs
            .begin_issue_token_save(super::super::jobs::IssueTokenSaveJob {
                token: token.to_string(),
                reopen_modal,
            });
        true
    }

    pub(super) fn start_issue_token_load(&mut self) {
        let started_at = Instant::now();
        if self.ui.feedback_issue.token_loading {
            emit_feedback_issue_action(
                "feedback_issue.load_token",
                None,
                "short_circuit",
                started_at,
                Some("already_loading"),
            );
            return;
        }
        self.ui.feedback_issue.token_loading = true;
        self.runtime.jobs.begin_issue_token_load();
        emit_feedback_issue_action(
            "feedback_issue.load_token",
            None,
            "queued",
            started_at,
            None,
        );
    }
}
