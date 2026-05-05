//! Issue gateway and issue-token async job runners for [`ControllerJobs`].

use super::*;

impl ControllerJobs {
    pub(crate) fn begin_issue_gateway_create(&mut self, job: IssueGatewayJob) {
        if self.in_progress.issue_gateway {
            return;
        }
        self.in_progress.issue_gateway = true;
        self.spawn_one_shot_job(
            false,
            move || IssueGatewayCreateResult {
                result: crate::issue_gateway::api::create_issue(&job.token, &job.request),
            },
            JobMessage::IssueGatewayCreated,
        );
    }

    pub(crate) fn clear_issue_gateway_create(&mut self) {
        self.in_progress.issue_gateway = false;
    }

    pub(crate) fn clear_issue_gateway_auth(&mut self) {
        self.in_progress.issue_gateway_auth = false;
    }

    pub(crate) fn begin_issue_gateway_poll(&mut self, job: IssueGatewayPollJob) {
        if self.in_progress.issue_gateway_poll {
            return;
        }
        self.in_progress.issue_gateway_poll = true;
        let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
        self.cancel_handles.issue_gateway_poll = Some(cancel.clone());
        self.spawn_optional_one_shot_job(false, move || {
            let config = issue_gateway_poll_config();
            let result = poll_issue_gateway_with_backoff(
                &job.request_id,
                &cancel,
                crate::issue_gateway::api::poll_issue_token,
                config,
                thread::sleep,
            );
            result.map(JobMessage::IssueGatewayAuthed)
        });
    }

    pub(crate) fn clear_issue_gateway_poll(&mut self) {
        self.in_progress.issue_gateway_poll = false;
        if let Some(cancel) = self.cancel_handles.issue_gateway_poll.take() {
            cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }

    /// Begin loading the persisted GitHub issue token on a background thread.
    pub(crate) fn begin_issue_token_load(&mut self) {
        if self.in_progress.issue_token_load {
            return;
        }
        self.in_progress.issue_token_load = true;
        self.spawn_one_shot_job(
            true,
            move || IssueTokenLoadResult {
                result: crate::issue_gateway::IssueTokenStore::new().and_then(|store| store.get()),
            },
            JobMessage::IssueTokenLoaded,
        );
    }

    /// Clear the in-progress flag for issue token loads.
    pub(crate) fn clear_issue_token_load(&mut self) {
        self.in_progress.issue_token_load = false;
    }

    /// Begin persisting a GitHub issue token on a background thread.
    pub(crate) fn begin_issue_token_save(&mut self, job: IssueTokenSaveJob) {
        if self.in_progress.issue_token_save {
            return;
        }
        self.in_progress.issue_token_save = true;
        self.spawn_one_shot_job(
            true,
            move || IssueTokenSaveResult {
                token: job.token.clone(),
                reopen_modal: job.reopen_modal,
                result: crate::issue_gateway::IssueTokenStore::new()
                    .and_then(|store| store.set_and_verify(&job.token)),
            },
            JobMessage::IssueTokenSaved,
        );
    }

    /// Clear the in-progress flag for issue token saves.
    pub(crate) fn clear_issue_token_save(&mut self) {
        self.in_progress.issue_token_save = false;
    }

    /// Begin deleting the persisted GitHub issue token on a background thread.
    pub(crate) fn begin_issue_token_delete(&mut self) {
        if self.in_progress.issue_token_delete {
            return;
        }
        self.in_progress.issue_token_delete = true;
        self.spawn_one_shot_job(
            true,
            move || IssueTokenDeleteResult {
                result: crate::issue_gateway::IssueTokenStore::new()
                    .and_then(|store| store.delete()),
            },
            JobMessage::IssueTokenDeleted,
        );
    }

    /// Clear the in-progress flag for issue token deletes.
    pub(crate) fn clear_issue_token_delete(&mut self) {
        self.in_progress.issue_token_delete = false;
    }
}
