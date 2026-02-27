//! Issue gateway and issue-token async job runners for [`ControllerJobs`].

use super::*;

impl ControllerJobs {
    pub(crate) fn begin_issue_gateway_create(&mut self, job: IssueGatewayJob) {
        if self.issue_gateway_in_progress {
            return;
        }
        self.issue_gateway_in_progress = true;
        let tx = self.message_tx.clone();
        thread::spawn(move || {
            let result = crate::issue_gateway::api::create_issue(&job.token, &job.request);
            let _ = tx.send(JobMessage::IssueGatewayCreated(IssueGatewayCreateResult {
                result,
            }));
        });
    }

    pub(crate) fn clear_issue_gateway_create(&mut self) {
        self.issue_gateway_in_progress = false;
    }

    pub(crate) fn clear_issue_gateway_auth(&mut self) {
        self.issue_gateway_auth_in_progress = false;
    }

    pub(crate) fn begin_issue_gateway_poll(&mut self, job: IssueGatewayPollJob) {
        if self.issue_gateway_poll_in_progress {
            return;
        }
        self.issue_gateway_poll_in_progress = true;
        let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
        self.issue_gateway_poll_cancel = Some(cancel.clone());
        let tx = self.message_tx.clone();
        thread::spawn(move || {
            let config = issue_gateway_poll_config();
            let result = poll_issue_gateway_with_backoff(
                &job.request_id,
                &cancel,
                crate::issue_gateway::api::poll_issue_token,
                config,
                thread::sleep,
            );
            if let Some(message) = result {
                let _ = tx.send(JobMessage::IssueGatewayAuthed(message));
            }
        });
    }

    pub(crate) fn clear_issue_gateway_poll(&mut self) {
        self.issue_gateway_poll_in_progress = false;
        if let Some(cancel) = self.issue_gateway_poll_cancel.take() {
            cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }

    /// Begin loading the persisted GitHub issue token on a background thread.
    pub(crate) fn begin_issue_token_load(&mut self) {
        if self.issue_token_load_in_progress {
            return;
        }
        self.issue_token_load_in_progress = true;
        let tx = self.message_tx.clone();
        let signal = self.repaint_signal.clone();
        thread::spawn(move || {
            let result = crate::issue_gateway::IssueTokenStore::new().and_then(|store| store.get());
            let _ = tx.send(JobMessage::IssueTokenLoaded(IssueTokenLoadResult {
                result,
            }));
            signal.request_repaint();
        });
    }

    /// Clear the in-progress flag for issue token loads.
    pub(crate) fn clear_issue_token_load(&mut self) {
        self.issue_token_load_in_progress = false;
    }

    /// Begin persisting a GitHub issue token on a background thread.
    pub(crate) fn begin_issue_token_save(&mut self, job: IssueTokenSaveJob) {
        if self.issue_token_save_in_progress {
            return;
        }
        self.issue_token_save_in_progress = true;
        let tx = self.message_tx.clone();
        let signal = self.repaint_signal.clone();
        thread::spawn(move || {
            let result = crate::issue_gateway::IssueTokenStore::new()
                .and_then(|store| store.set_and_verify(&job.token));
            let _ = tx.send(JobMessage::IssueTokenSaved(IssueTokenSaveResult {
                token: job.token,
                reopen_modal: job.reopen_modal,
                result,
            }));
            signal.request_repaint();
        });
    }

    /// Clear the in-progress flag for issue token saves.
    pub(crate) fn clear_issue_token_save(&mut self) {
        self.issue_token_save_in_progress = false;
    }

    /// Begin deleting the persisted GitHub issue token on a background thread.
    pub(crate) fn begin_issue_token_delete(&mut self) {
        if self.issue_token_delete_in_progress {
            return;
        }
        self.issue_token_delete_in_progress = true;
        let tx = self.message_tx.clone();
        let signal = self.repaint_signal.clone();
        thread::spawn(move || {
            let result =
                crate::issue_gateway::IssueTokenStore::new().and_then(|store| store.delete());
            let _ = tx.send(JobMessage::IssueTokenDeleted(IssueTokenDeleteResult {
                result,
            }));
            signal.request_repaint();
        });
    }

    /// Clear the in-progress flag for issue token deletes.
    pub(crate) fn clear_issue_token_delete(&mut self) {
        self.issue_token_delete_in_progress = false;
    }
}
