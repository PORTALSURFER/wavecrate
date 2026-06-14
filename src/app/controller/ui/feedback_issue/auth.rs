use super::super::*;
use super::logging::emit_feedback_issue_action;
use std::time::Instant;

impl AppController {
    pub(crate) fn connect_github_issue_reporting(&mut self) {
        let started_at = Instant::now();
        if self.ui.feedback_issue.connecting {
            emit_feedback_issue_action(
                "feedback_issue.connect_github",
                None,
                "short_circuit",
                started_at,
                Some("already_connecting"),
            );
            return;
        }
        self.ui.feedback_issue.connecting = true;
        self.ui.feedback_issue.last_error = None;
        self.set_status("Opening GitHub auth page…", StatusTone::Info);

        let request_id = format!("req_{}", uuid::Uuid::new_v4());
        let auth_url = format!(
            "{}?requestId={}",
            crate::issue_gateway::api::AUTH_START_URL,
            request_id
        );

        if let Err(err) = open::that(&auth_url) {
            self.ui.feedback_issue.last_error = Some(format!(
                "Failed to open auth URL. Open it manually and paste the token: {} ({err})",
                crate::issue_gateway::api::AUTH_START_URL
            ));
            self.set_status("GitHub connect failed".to_string(), StatusTone::Error);
            self.ui.feedback_issue.connecting = false;
            self.ui.feedback_issue.token_modal_open = true;
            self.ui.feedback_issue.focus_token_requested = true;
            let error = err.to_string();
            emit_feedback_issue_action(
                "feedback_issue.connect_github",
                None,
                "error",
                started_at,
                Some(&error),
            );
        } else {
            self.runtime
                .jobs
                .begin_issue_gateway_poll(super::super::jobs::IssueGatewayPollJob { request_id });
            emit_feedback_issue_action(
                "feedback_issue.connect_github",
                None,
                "polling",
                started_at,
                None,
            );
        }
    }

    pub(crate) fn complete_issue_gateway_auth(
        &mut self,
        result: Result<String, crate::issue_gateway::api::IssueAuthError>,
    ) {
        self.ui.feedback_issue.connecting = false;
        self.runtime.jobs.clear_issue_gateway_poll();
        match result {
            Ok(token) => {
                if !self.persist_issue_token(&token, false) {
                    self.set_status("Failed to save GitHub token".to_string(), StatusTone::Error);
                }
            }
            Err(err) => {
                self.ui.feedback_issue.last_error =
                    Some(format!("Auto-connect failed: {err}. Use Paste token…"));
                self.set_status(format!("GitHub connect failed: {err}"), StatusTone::Error);
            }
        }
    }
}
