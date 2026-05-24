use std::path::PathBuf;

use super::state::{SCOPED_APP_ROOT_OVERRIDE, SCOPED_PROFILE_OVERRIDE};
use super::{AUTOMATED_PROFILE_NAME, SANDBOX_PROFILE_NAME};

/// High-level persistence mode for the current process.
///
/// This lets runtime code and tooling answer whether a run is using the real
/// live app root, a dedicated sandbox/manual-QA profile, an automated-validation
/// profile, or another explicitly named non-live profile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PersistenceMode {
    /// Use the real user-facing app root.
    Live,
    /// Use the dedicated sandbox/manual-QA profile.
    Sandbox,
    /// Use the dedicated automated-validation profile.
    Automated,
    /// Use another named non-live profile under `.wavecrate/profiles/<name>`.
    Named(String),
}

impl PersistenceMode {
    /// Return the stable identifier stored in logs, manifests, and scripts.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Live => "live",
            Self::Sandbox => SANDBOX_PROFILE_NAME,
            Self::Automated => AUTOMATED_PROFILE_NAME,
            Self::Named(profile) => profile.as_str(),
        }
    }
}

impl std::fmt::Display for PersistenceMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Resolved persistence selection for the current thread/process.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedPersistence {
    /// Base config directory before `.wavecrate`/profile expansion.
    pub config_base: PathBuf,
    /// Fully resolved app root used for config, logs, and library state.
    pub app_root: PathBuf,
    /// High-level persistence mode for this run.
    pub mode: PersistenceMode,
}

/// Persistence profile that controls which app root the process should use.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum ProfileSelection {
    /// Use the real user-facing app root.
    Live,
    /// Use a named non-live profile under the standard app root.
    Named(String),
}

/// Guard that overrides the persistence profile for the current thread.
///
/// GUI automation and manual validation flows use this to keep non-live runs on
/// a dedicated app root without mutating the user's live config or library DB.
pub struct PersistenceProfileGuard {
    previous_profile: Option<ProfileSelection>,
    previous_root: Option<PathBuf>,
}

impl PersistenceProfileGuard {
    /// Force the current thread onto the live persistence profile.
    pub fn live() -> Self {
        Self::set(ProfileSelection::Live)
    }

    /// Force the current thread onto the dedicated sandbox/manual-QA profile.
    pub fn sandbox() -> Self {
        Self::set(ProfileSelection::Named(String::from(SANDBOX_PROFILE_NAME)))
    }

    /// Force the current thread onto the dedicated automated-validation profile.
    pub fn automated() -> Self {
        Self::set(ProfileSelection::Named(String::from(
            AUTOMATED_PROFILE_NAME,
        )))
    }

    /// Force the current thread onto one named non-live persistence profile.
    pub fn named(profile: impl Into<String>) -> Self {
        Self::set(ProfileSelection::Named(profile.into()))
    }

    fn set(profile: ProfileSelection) -> Self {
        let previous_profile = SCOPED_PROFILE_OVERRIDE.with(|override_profile| {
            let mut slot = override_profile.borrow_mut();
            let previous = slot.clone();
            *slot = Some(profile);
            previous
        });
        let previous_root = SCOPED_APP_ROOT_OVERRIDE.with(|override_path| {
            let mut slot = override_path.borrow_mut();
            let previous = slot.clone();
            *slot = None;
            previous
        });
        Self {
            previous_profile,
            previous_root,
        }
    }
}

impl Drop for PersistenceProfileGuard {
    fn drop(&mut self) {
        let previous_profile = self.previous_profile.take();
        SCOPED_PROFILE_OVERRIDE.with(|override_profile| {
            *override_profile.borrow_mut() = previous_profile;
        });
        let previous_root = self.previous_root.take();
        SCOPED_APP_ROOT_OVERRIDE.with(|override_path| {
            *override_path.borrow_mut() = previous_root;
        });
    }
}

pub(super) fn persistence_mode_from_selection(selection: &ProfileSelection) -> PersistenceMode {
    match selection {
        ProfileSelection::Live => PersistenceMode::Live,
        ProfileSelection::Named(profile) if profile.eq_ignore_ascii_case(SANDBOX_PROFILE_NAME) => {
            PersistenceMode::Sandbox
        }
        ProfileSelection::Named(profile)
            if profile.eq_ignore_ascii_case(AUTOMATED_PROFILE_NAME)
                || profile.eq_ignore_ascii_case("automated") =>
        {
            PersistenceMode::Automated
        }
        ProfileSelection::Named(profile) => PersistenceMode::Named(profile.clone()),
    }
}
