use super::super::*;
use super::logging::emit_feedback_issue_action;
use std::time::Instant;

impl AppController {
    pub(crate) fn submit_feedback_issue(&mut self, kind: crate::issue_gateway::api::IssueKind) {
        let started_at = Instant::now();
        if self.ui.feedback_issue.submitting {
            emit_feedback_issue_action(
                "feedback_issue.submit",
                Some(kind.title_prefix()),
                "short_circuit",
                started_at,
                Some("already_submitting"),
            );
            return;
        }

        let title = self.ui.feedback_issue.title.trim();
        if title.len() < 3 || title.len() > 200 {
            self.ui.feedback_issue.last_error = Some("Title must be 3–200 characters.".to_string());
            emit_feedback_issue_action(
                "feedback_issue.submit",
                Some(kind.title_prefix()),
                "validation_error",
                started_at,
                Some("invalid_title"),
            );
            return;
        }

        let token = match self.ui.feedback_issue.token_cached.clone() {
            Some(token) => token,
            None => {
                self.handle_missing_issue_token(kind, started_at);
                return;
            }
        };

        let mut final_title = title.to_string();
        let prefix = kind.title_prefix();
        if !final_title.starts_with(prefix) {
            final_title = format!("{prefix}{final_title}");
        }

        let body = self.compose_issue_body();
        self.ui.feedback_issue.submitting = true;
        self.ui.feedback_issue.last_error = None;
        self.ui.feedback_issue.last_success_url = None;
        self.runtime
            .jobs
            .begin_issue_gateway_create(super::super::jobs::IssueGatewayJob {
                token,
                request: crate::issue_gateway::api::CreateIssueRequest {
                    title: final_title,
                    body,
                },
            });
        emit_feedback_issue_action(
            "feedback_issue.submit",
            Some(kind.title_prefix()),
            "queued",
            started_at,
            None,
        );
    }

    fn handle_missing_issue_token(
        &mut self,
        kind: crate::issue_gateway::api::IssueKind,
        started_at: Instant,
    ) {
        let loading = self.ui.feedback_issue.token_loading
            || self.ui.feedback_issue.token_saving
            || self.ui.feedback_issue.token_deleting;
        if loading {
            self.ui.feedback_issue.last_error =
                Some("GitHub token is still loading. Try again.".to_string());
            emit_feedback_issue_action(
                "feedback_issue.submit",
                Some(kind.title_prefix()),
                "validation_error",
                started_at,
                Some("token_loading"),
            );
            return;
        }

        self.ui.feedback_issue.last_error = Some("Connect GitHub first.".to_string());
        self.ui.feedback_issue.token_modal_open = true;
        self.ui.feedback_issue.focus_token_requested = true;
        emit_feedback_issue_action(
            "feedback_issue.submit",
            Some(kind.title_prefix()),
            "validation_error",
            started_at,
            Some("token_missing"),
        );
    }
}
