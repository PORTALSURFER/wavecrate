use super::state::{ReleaseOption, ReleaseState, UiStatus, UpdateNativeBridge};
use sempal::updater::{
    ApplyPlan, ReleaseSummary, UpdateChannel, apply_update_with_progress, list_recent_releases,
};
use std::{
    sync::mpsc::{self},
    thread,
};

const MAX_LOG_LINES: usize = 200;
const RELEASES_TO_FETCH: usize = 5;

impl UpdateNativeBridge {
    pub(super) fn refresh_release_list(&mut self) {
        if self.args.identity.channel == UpdateChannel::Nightly {
            self.release_state = ReleaseState::Loaded(vec![ReleaseOption {
                tag: "nightly".to_string(),
                label: "nightly".to_string(),
                html_url: String::new(),
            }]);
            self.selected_tag = Some("nightly".to_string());
            return;
        }
        let repo = self.args.repo.clone();
        let identity = self.args.identity.clone();
        let channel = self.args.identity.channel;
        let (tx, rx) = mpsc::channel();
        self.release_state = ReleaseState::Loading;
        self.release_rx = Some(rx);
        thread::spawn(move || {
            let result = list_recent_releases(&repo, channel, &identity, RELEASES_TO_FETCH)
                .map_err(|err| err.to_string());
            let _ = tx.send(result);
        });
    }

    pub(super) fn start_update(&mut self) {
        if matches!(self.status, UiStatus::Updating) {
            return;
        }
        let mut args = self.args.clone();
        args.requested_tag = self.selected_tag.clone();
        let (progress_tx, progress_rx) = mpsc::channel();
        let (result_tx, result_rx) = mpsc::channel();
        self.progress_rx = Some(progress_rx);
        self.result_rx = Some(result_rx);
        self.log.clear();
        self.push_log("Starting update...");
        self.status = UiStatus::Updating;
        self.show_log_view = true;
        thread::spawn(move || {
            let result = apply_update_with_progress(args, |progress| {
                let _ = progress_tx.send(progress);
            })
            .map_err(|err| err.to_string());
            let _ = result_tx.send(result);
        });
    }

    pub(super) fn poll_background_updates(&mut self) {
        if let Some(rx) = &self.release_rx
            && let Ok(result) = rx.try_recv()
        {
            self.release_rx = None;
            match result {
                Ok(list) => {
                    let options = list.into_iter().map(format_release_option).collect();
                    self.release_state = ReleaseState::Loaded(options);
                    self.ensure_selected_tag();
                }
                Err(err) => {
                    self.release_state = ReleaseState::Error(err);
                }
            }
        }

        if let Some(rx) = &self.progress_rx {
            let messages: Vec<String> = rx.try_iter().map(|progress| progress.message).collect();
            for message in messages {
                self.push_log(message);
            }
        }

        if let Some(rx) = &self.result_rx
            && let Ok(result) = rx.try_recv()
        {
            self.result_rx = None;
            self.progress_rx = None;
            self.finish_update(result);
        }
    }

    fn finish_update(&mut self, result: Result<ApplyPlan, String>) {
        match result {
            Ok(plan) => {
                self.push_log(format!(
                    "Installed {} into {}",
                    plan.release_tag,
                    plan.install_dir.display()
                ));
                if !plan.stale_removal_failures.is_empty() {
                    self.push_log(format!(
                        "Warning: failed to remove {} stale paths",
                        plan.stale_removal_failures.len()
                    ));
                    for failure in &plan.stale_removal_failures {
                        self.push_log(format!(
                            "Stale remove failed: {} ({})",
                            failure.path.display(),
                            failure.error
                        ));
                    }
                }
                self.status = UiStatus::Success(format!("Updated to {}", plan.release_tag));
            }
            Err(err) => {
                self.push_log(format!("Update failed: {err}"));
                self.status = UiStatus::Error(err);
            }
        }
    }

    fn push_log(&mut self, message: impl Into<String>) {
        self.log.push(message.into());
        if self.log.len() > MAX_LOG_LINES {
            let trim = self.log.len() - MAX_LOG_LINES;
            self.log.drain(0..trim);
        }
    }
}

fn format_release_option(summary: ReleaseSummary) -> ReleaseOption {
    let label = match summary.published_at.as_deref() {
        Some(date) => format!("{} ({})", summary.tag, short_date(date)),
        None => summary.tag.clone(),
    };
    ReleaseOption {
        tag: summary.tag,
        label,
        html_url: summary.html_url,
    }
}

fn short_date(value: &str) -> String {
    value.get(0..10).unwrap_or(value).to_string()
}
