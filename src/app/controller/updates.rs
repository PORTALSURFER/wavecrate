use super::*;

use crate::updater::{
    RuntimeIdentity, UpdateChannel, UpdateCheckOutcome, UpdateCheckRequest, check_for_updates,
};
use std::{fmt, path::Path};

impl AppController {
    pub(super) fn maybe_check_for_updates_on_startup(&mut self) {
        if !self.settings.updates.check_on_startup {
            return;
        }
        if startup_update_check_should_skip_for_current_exe() {
            tracing::debug!("Skipping startup update check for local Cargo target executable");
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
        let update_changed = self.ui.update.status != crate::app::state::UpdateStatus::Idle
            || self.ui.update.available_tag.is_some()
            || self.ui.update.available_url.is_some();
        self.ui.update.status = crate::app::state::UpdateStatus::Idle;
        self.ui.update.available_tag = None;
        self.ui.update.available_url = None;
        self.ui.update.available_published_at = None;
        if update_changed {
            self.mark_update_projection_revision_dirty();
        }
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
                let update_changed = self.ui.update.status != crate::app::state::UpdateStatus::Idle
                    || self.ui.update.available_tag.is_some()
                    || self.ui.update.available_url.is_some();
                self.ui.update.status = crate::app::state::UpdateStatus::Idle;
                self.ui.update.available_tag = None;
                self.ui.update.available_url = None;
                self.ui.update.available_published_at = None;
                if update_changed {
                    self.mark_update_projection_revision_dirty();
                }
            }
            UpdateCheckOutcome::UpdateAvailable {
                tag,
                html_url,
                published_at,
            } => {
                let update_changed = self.ui.update.status
                    != crate::app::state::UpdateStatus::UpdateAvailable
                    || self.ui.update.available_tag.as_deref() != Some(tag.as_str())
                    || self.ui.update.available_url.as_deref() != Some(html_url.as_str());
                self.ui.update.status = crate::app::state::UpdateStatus::UpdateAvailable;
                self.ui.update.available_tag = Some(tag);
                self.ui.update.available_url = Some(html_url);
                self.ui.update.available_published_at = published_at;
                if update_changed {
                    self.mark_update_projection_revision_dirty();
                }
            }
        }
    }

    pub(crate) fn apply_update_check_error(&mut self, err: String) {
        if err.contains("release with required assets found") {
            let update_changed = self.ui.update.status != crate::app::state::UpdateStatus::Idle
                || self.ui.update.last_error.is_some()
                || self.ui.update.available_tag.is_some()
                || self.ui.update.available_url.is_some();
            self.ui.update.status = crate::app::state::UpdateStatus::Idle;
            self.ui.update.last_error = None;
            self.ui.update.available_tag = None;
            self.ui.update.available_url = None;
            self.ui.update.available_published_at = None;
            if update_changed {
                self.mark_update_projection_revision_dirty();
            }
            return;
        }
        let update_changed = self.ui.update.status != crate::app::state::UpdateStatus::Error
            || self.ui.update.last_error.as_deref() != Some(err.as_str())
            || self.ui.update.available_tag.is_some()
            || self.ui.update.available_url.is_some();
        self.ui.update.status = crate::app::state::UpdateStatus::Error;
        self.ui.update.last_error = Some(err.clone());
        self.ui.update.available_tag = None;
        self.ui.update.available_url = None;
        self.ui.update.available_published_at = None;
        if update_changed {
            self.mark_update_projection_revision_dirty();
        }
        self.set_status(format!("Update check failed: {err}"), StatusTone::Warning);
    }

    fn begin_update_check(&mut self) {
        if self.runtime.jobs.update_check_in_progress() {
            return;
        }
        let current_version = match semver::Version::parse(crate::release_metadata::CURRENT.version)
        {
            Ok(v) => v,
            Err(_) => semver::Version::new(0, 0, 0),
        };
        let channel = map_channel(self.settings.updates.channel);
        let request = match update_check_request(
            channel,
            current_version,
            self.ui.update.last_seen_nightly_published_at.clone(),
        ) {
            Ok(request) => request,
            Err(err) => {
                self.apply_unsupported_update_platform(err);
                return;
            }
        };
        let update_changed = self.ui.update.status != crate::app::state::UpdateStatus::Checking
            || self.ui.update.last_error.is_some();
        self.ui.update.status = crate::app::state::UpdateStatus::Checking;
        self.ui.update.last_error = None;
        if update_changed {
            self.mark_update_projection_revision_dirty();
        }
        self.runtime.jobs.begin_update_check(request);
    }

    fn apply_unsupported_update_platform(&mut self, err: UnsupportedUpdatePlatform) {
        let message = err.to_string();
        tracing::info!("{message}");
        let update_changed = self.ui.update.status != crate::app::state::UpdateStatus::Idle
            || self.ui.update.last_error.as_deref() != Some(message.as_str())
            || self.ui.update.available_tag.is_some()
            || self.ui.update.available_url.is_some()
            || self.ui.update.available_published_at.is_some();
        self.ui.update.status = crate::app::state::UpdateStatus::Idle;
        self.ui.update.last_error = Some(message.clone());
        self.ui.update.available_tag = None;
        self.ui.update.available_url = None;
        self.ui.update.available_published_at = None;
        if update_changed {
            self.mark_update_projection_revision_dirty();
        }
        self.set_status(message, StatusTone::Info);
    }
}

fn startup_update_check_should_skip_for_current_exe() -> bool {
    std::env::current_exe()
        .map(|exe| startup_update_check_should_skip_for_exe(&exe))
        .unwrap_or(false)
}

fn startup_update_check_should_skip_for_exe(exe: &Path) -> bool {
    let Some(profile_dir) = exe.parent() else {
        return false;
    };
    let Some(profile_name) = profile_dir.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    if profile_name != "debug" && profile_name != "release" {
        return false;
    }
    profile_dir
        .parent()
        .and_then(|target_dir| target_dir.file_name())
        .and_then(|name| name.to_str())
        == Some("target")
}

fn map_channel(channel: crate::sample_sources::config::UpdateChannel) -> UpdateChannel {
    match channel {
        crate::sample_sources::config::UpdateChannel::Stable => UpdateChannel::Stable,
        crate::sample_sources::config::UpdateChannel::Rc => UpdateChannel::Rc,
        crate::sample_sources::config::UpdateChannel::Nightly => UpdateChannel::Nightly,
    }
}

pub(crate) fn run_update_check(request: UpdateCheckRequest) -> Result<UpdateCheckOutcome, String> {
    check_for_updates(request).map_err(|err| err.to_string())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UnsupportedUpdatePlatform {
    platform: String,
    arch: String,
}

impl fmt::Display for UnsupportedUpdatePlatform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Wavecrate update checks are unavailable on {}/{} because release assets are published only for Windows x86_64 and macOS x86_64/aarch64.",
            self.platform, self.arch
        )
    }
}

fn update_check_request(
    channel: UpdateChannel,
    current_version: semver::Version,
    last_seen_nightly_published_at: Option<String>,
) -> Result<UpdateCheckRequest, UnsupportedUpdatePlatform> {
    let identity = runtime_identity(channel)?;
    Ok(UpdateCheckRequest {
        repo: crate::updater::REPO_SLUG.to_string(),
        channel,
        identity,
        current_version,
        last_seen_nightly_published_at,
    })
}

#[cfg(test)]
fn update_check_request_for_platform_arch(
    channel: UpdateChannel,
    current_version: semver::Version,
    last_seen_nightly_published_at: Option<String>,
    platform: &str,
    arch: &str,
) -> Result<UpdateCheckRequest, UnsupportedUpdatePlatform> {
    let identity = runtime_identity_for_platform_arch(channel, platform, arch)?;
    Ok(UpdateCheckRequest {
        repo: crate::updater::REPO_SLUG.to_string(),
        channel,
        identity,
        current_version,
        last_seen_nightly_published_at,
    })
}

fn runtime_identity_for_platform_arch(
    channel: UpdateChannel,
    platform: &str,
    arch: &str,
) -> Result<RuntimeIdentity, UnsupportedUpdatePlatform> {
    let target = crate::updater::supported_release_target_for_platform_arch(platform, arch)
        .ok_or_else(|| UnsupportedUpdatePlatform {
            platform: platform.to_string(),
            arch: arch.to_string(),
        })?;
    Ok(RuntimeIdentity {
        app: crate::updater::APP_NAME.to_string(),
        channel,
        target: target.to_string(),
        platform: platform.to_string(),
        arch: arch.to_string(),
    })
}

fn runtime_identity(channel: UpdateChannel) -> Result<RuntimeIdentity, UnsupportedUpdatePlatform> {
    let platform_raw = std::env::consts::OS;
    let platform = platform_raw;
    let arch_raw = std::env::consts::ARCH;
    let arch = arch_raw;
    runtime_identity_for_platform_arch(channel, platform, arch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn startup_update_check_skips_local_cargo_target_exes() {
        assert!(startup_update_check_should_skip_for_exe(
            &Path::new("dev")
                .join("wavecrate")
                .join("target")
                .join("release")
                .join("wavecrate")
        ));
        assert!(startup_update_check_should_skip_for_exe(
            &Path::new("dev")
                .join("wavecrate")
                .join("target")
                .join("debug")
                .join("wavecrate")
        ));
    }

    #[test]
    fn startup_update_check_runs_for_installed_or_non_cargo_exes() {
        assert!(!startup_update_check_should_skip_for_exe(
            &Path::new("opt").join("Wavecrate").join("wavecrate")
        ));
        assert!(!startup_update_check_should_skip_for_exe(
            &Path::new("dev")
                .join("wavecrate")
                .join("target")
                .join("custom")
                .join("wavecrate")
        ));
        assert!(!startup_update_check_should_skip_for_exe(
            &Path::new("dev")
                .join("wavecrate")
                .join("target-release")
                .join("wavecrate")
        ));
    }

    #[test]
    fn runtime_identity_keeps_supported_release_targets() {
        let windows =
            runtime_identity_for_platform_arch(UpdateChannel::Stable, "windows", "x86_64")
                .expect("windows x86_64 release target");
        assert_eq!(windows.app, crate::updater::APP_NAME);
        assert_eq!(windows.channel, UpdateChannel::Stable);
        assert_eq!(windows.target, "x86_64-pc-windows-msvc");
        assert_eq!(windows.platform, "windows");
        assert_eq!(windows.arch, "x86_64");

        let macos_intel = runtime_identity_for_platform_arch(UpdateChannel::Rc, "macos", "x86_64")
            .expect("macos x86_64 release target");
        assert_eq!(macos_intel.channel, UpdateChannel::Rc);
        assert_eq!(macos_intel.target, "x86_64-apple-darwin");
        assert_eq!(macos_intel.platform, "macos");
        assert_eq!(macos_intel.arch, "x86_64");

        let macos_arm =
            runtime_identity_for_platform_arch(UpdateChannel::Nightly, "macos", "aarch64")
                .expect("macos aarch64 release target");
        assert_eq!(macos_arm.channel, UpdateChannel::Nightly);
        assert_eq!(macos_arm.target, "aarch64-apple-darwin");
        assert_eq!(macos_arm.platform, "macos");
        assert_eq!(macos_arm.arch, "aarch64");
    }

    #[test]
    fn runtime_identity_rejects_unsupported_release_platforms() {
        for (platform, arch) in [
            ("linux", "x86_64"),
            ("linux", "aarch64"),
            ("windows", "aarch64"),
            ("macos", "riscv64"),
        ] {
            let err = runtime_identity_for_platform_arch(UpdateChannel::Stable, platform, arch)
                .expect_err("unsupported runtime identity should be rejected");
            assert_eq!(err.platform, platform);
            assert_eq!(err.arch, arch);
            let message = err.to_string();
            assert!(message.contains(platform));
            assert!(message.contains(arch));
            assert!(message.contains("Windows x86_64 and macOS x86_64/aarch64"));
        }
    }

    #[test]
    fn update_check_request_rejects_linux_before_asset_lookup() {
        let err = update_check_request_for_platform_arch(
            UpdateChannel::Nightly,
            semver::Version::new(19, 1, 0),
            Some("2026-07-01T00:00:00Z".to_string()),
            "linux",
            "x86_64",
        )
        .expect_err("linux should not create an update-check request");

        assert_eq!(err.platform, "linux");
        assert_eq!(err.arch, "x86_64");
    }

    #[test]
    fn update_check_request_preserves_supported_identity_and_channel() {
        let request = update_check_request_for_platform_arch(
            UpdateChannel::Rc,
            semver::Version::new(19, 1, 0),
            Some("2026-07-01T00:00:00Z".to_string()),
            "macos",
            "aarch64",
        )
        .expect("supported macos request");

        assert_eq!(request.repo, crate::updater::REPO_SLUG);
        assert_eq!(request.channel, UpdateChannel::Rc);
        assert_eq!(request.identity.channel, UpdateChannel::Rc);
        assert_eq!(request.identity.platform, "macos");
        assert_eq!(request.identity.arch, "aarch64");
        assert_eq!(request.identity.target, "aarch64-apple-darwin");
        assert_eq!(request.current_version, semver::Version::new(19, 1, 0));
        assert_eq!(
            request.last_seen_nightly_published_at.as_deref(),
            Some("2026-07-01T00:00:00Z")
        );
    }
}
