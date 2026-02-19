use super::*;

use crate::updater::{
    RuntimeIdentity, UpdateChannel, UpdateCheckOutcome, UpdateCheckRequest, check_for_updates,
};

impl AppController {
    pub(super) fn maybe_check_for_updates_on_startup(&mut self) {
        if !self.settings.updates.check_on_startup {
            return;
        }
        if self.ui.update.status != crate::app::state::UpdateStatus::Idle {
            return;
        }
        self.begin_update_check();
    }

    /// Start an update check regardless of user settings.
    pub fn check_for_updates_now(&mut self) {
        self.begin_update_check();
    }

    /// Open the currently available update URL in the OS browser (if present).
    pub fn open_update_link(&mut self) {
        let Some(url) = self.ui.update.available_url.clone() else {
            return;
        };
        if let Err(err) = crate::updater::open_release_page(&url) {
            self.set_status(
                format!("Could not open update link: {err}"),
                StatusTone::Error,
            );
        }
    }

    /// Dismiss the current update notification (nightly dismisses are persisted).
    pub fn dismiss_update_notification(&mut self) {
        if let Some(published) = self.ui.update.available_published_at.clone() {
            self.ui.update.last_seen_nightly_published_at = Some(published.clone());
            self.settings.updates.last_seen_nightly_published_at = Some(published);
        }
        self.ui.update.status = crate::app::state::UpdateStatus::Idle;
        self.ui.update.available_tag = None;
        self.ui.update.available_url = None;
        self.ui.update.available_published_at = None;
        let _ = self.save_full_config();
    }

    /// Open the release page and require manual update installation.
    ///
    /// This intentionally avoids in-app binary replacement to keep the update
    /// path explicit and easier to validate with platform trust tooling.
    pub fn install_update_and_exit(&mut self) {
        if self.ui.update.available_url.is_none() {
            self.set_status("No update available", StatusTone::Info);
            return;
        }
        self.set_status(
            "Manual secure update required; opening release page",
            StatusTone::Info,
        );
        self.open_update_link();
    }

    pub(crate) fn apply_update_check_result(&mut self, result: UpdateCheckOutcome) {
        match result {
            UpdateCheckOutcome::UpToDate => {
                self.ui.update.status = crate::app::state::UpdateStatus::Idle;
                self.ui.update.available_tag = None;
                self.ui.update.available_url = None;
                self.ui.update.available_published_at = None;
            }
            UpdateCheckOutcome::UpdateAvailable {
                tag,
                html_url,
                published_at,
            } => {
                self.ui.update.status = crate::app::state::UpdateStatus::UpdateAvailable;
                self.ui.update.available_tag = Some(tag);
                self.ui.update.available_url = Some(html_url);
                self.ui.update.available_published_at = published_at;
            }
        }
    }

    pub(crate) fn apply_update_check_error(&mut self, err: String) {
        if err.contains("release with required assets found") {
            self.ui.update.status = crate::app::state::UpdateStatus::Idle;
            self.ui.update.last_error = None;
            self.ui.update.available_tag = None;
            self.ui.update.available_url = None;
            self.ui.update.available_published_at = None;
            return;
        }
        self.ui.update.status = crate::app::state::UpdateStatus::Error;
        self.ui.update.last_error = Some(err.clone());
        self.ui.update.available_tag = None;
        self.ui.update.available_url = None;
        self.ui.update.available_published_at = None;
        self.set_status(format!("Update check failed: {err}"), StatusTone::Warning);
    }

    fn begin_update_check(&mut self) {
        if self.runtime.jobs.update_check_in_progress() {
            return;
        }
        let current_version = match semver::Version::parse(env!("CARGO_PKG_VERSION")) {
            Ok(v) => v,
            Err(_) => semver::Version::new(0, 0, 0),
        };
        let request = UpdateCheckRequest {
            repo: crate::updater::REPO_SLUG.to_string(),
            channel: map_channel(self.settings.updates.channel),
            identity: runtime_identity(map_channel(self.settings.updates.channel)),
            current_version,
            last_seen_nightly_published_at: self.ui.update.last_seen_nightly_published_at.clone(),
        };
        self.ui.update.status = crate::app::state::UpdateStatus::Checking;
        self.ui.update.last_error = None;
        self.runtime.jobs.begin_update_check(request);
    }
}

fn map_channel(channel: crate::sample_sources::config::UpdateChannel) -> UpdateChannel {
    match channel {
        crate::sample_sources::config::UpdateChannel::Stable => UpdateChannel::Stable,
        crate::sample_sources::config::UpdateChannel::Nightly => UpdateChannel::Nightly,
    }
}

pub(crate) fn run_update_check(request: UpdateCheckRequest) -> Result<UpdateCheckOutcome, String> {
    check_for_updates(request).map_err(|err| err.to_string())
}

fn runtime_identity(channel: UpdateChannel) -> RuntimeIdentity {
    let platform_raw = std::env::consts::OS;
    let platform = platform_raw;
    let arch_raw = std::env::consts::ARCH;
    let arch = arch_raw;
    let target = match (platform, arch) {
        ("windows", "x86_64") => "x86_64-pc-windows-msvc",
        ("windows", "aarch64") => "aarch64-pc-windows-msvc",
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
        ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("macos", "aarch64") => "aarch64-apple-darwin",
        _ => "unknown",
    };
    RuntimeIdentity {
        app: crate::updater::APP_NAME.to_string(),
        channel,
        target: target.to_string(),
        platform: platform.to_string(),
        arch: arch.to_string(),
    }
}
