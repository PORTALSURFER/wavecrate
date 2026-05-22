//! Analysis-crate application directory helpers anchored to a single `.wavecrate` folder.
//!
//! The analysis crate only uses this module for ANN index cache placement and
//! associated tests. Keeping a local copy avoids a dependency cycle back into
//! the main application crate. Test executables default to an isolated
//! `automated-tests` profile unless they explicitly request the live profile
//! or another config root.

use std::{
    path::PathBuf,
    sync::{LazyLock, Mutex},
};

#[cfg(test)]
use std::cell::RefCell;

use directories::BaseDirs;
use thiserror::Error;

#[cfg(test)]
mod overrides;

#[cfg(test)]
pub use overrides::ConfigBaseGuard;

/// Name of the application directory that lives under the OS config root.
pub const APP_DIR_NAME: &str = ".wavecrate";
/// Name of the directory that stores explicit non-live persistence profiles.
pub const PROFILE_DIR_NAME: &str = "profiles";
/// Canonical non-live profile name used for sandbox/manual QA runs.
pub const SANDBOX_PROFILE_NAME: &str = "sandbox";
/// Canonical non-live profile name used for automated validation runs.
pub const AUTOMATED_PROFILE_NAME: &str = "automated-tests";

const CONFIG_PROFILE_ENV: &str = "WAVECRATE_CONFIG_PROFILE";
const TEST_EXECUTABLE_DIR_NAME: &str = "deps";

static CONFIG_BASE_OVERRIDE: LazyLock<Mutex<Option<PathBuf>>> = LazyLock::new(|| Mutex::new(None));
static APP_ROOT_OVERRIDE: LazyLock<Mutex<Option<PathBuf>>> = LazyLock::new(|| Mutex::new(None));
static TEST_CONFIG_BASE: LazyLock<PathBuf> = LazyLock::new(|| {
    let dir = tempfile::tempdir().expect("create test config dir");
    let path = dir.path().to_path_buf();
    std::mem::forget(dir);
    path
});

#[cfg(test)]
thread_local! {
    static TEST_CONFIG_OVERRIDE: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}
#[cfg(test)]
thread_local! {
    static SCOPED_APP_ROOT_OVERRIDE: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}
#[cfg(test)]
thread_local! {
    static SCOPED_PROFILE_OVERRIDE: RefCell<Option<ProfileSelection>> = const { RefCell::new(None) };
}

/// Ensure tests do not touch real user config directories.
pub fn ensure_test_config_base() {
    if std::env::var_os("WAVECRATE_CONFIG_HOME").is_some() {
        return;
    }
    let test_base = LazyLock::force(&TEST_CONFIG_BASE).clone();
    let mut guard = CONFIG_BASE_OVERRIDE
        .lock()
        .expect("config base override mutex poisoned");
    if guard.is_none() {
        *guard = Some(test_base);
    }
}

/// Errors that can occur while resolving or preparing application directories.
#[derive(Debug, Error)]
pub enum AppDirError {
    /// No suitable base config directory could be resolved.
    #[error("No suitable base config directory available for application files")]
    NoBaseDir,
    /// Failed to create the application directory.
    #[error("Failed to create application directory at {path}: {source}")]
    CreateDir {
        /// Path that told the directory to be created.
        path: PathBuf,
        /// Underlying IO error.
        source: std::io::Error,
    },
    /// The configured profile name cannot be represented safely on disk.
    #[error("Invalid Wavecrate profile name '{profile}'")]
    InvalidProfileName {
        /// Rejected profile name.
        profile: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ProfileSelection {
    Live,
    Named(String),
}

/// Guard that overrides the persistence profile for the current test thread.
#[cfg(test)]
pub struct PersistenceProfileGuard {
    previous_profile: Option<ProfileSelection>,
    previous_root: Option<PathBuf>,
}

#[cfg(test)]
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

#[cfg(test)]
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

/// Return the root `.wavecrate` directory, creating it if needed.
pub fn app_root_dir() -> Result<PathBuf, AppDirError> {
    if should_auto_isolate_test_config_base() {
        ensure_test_config_base();
    }
    #[cfg(test)]
    if let Some(path) =
        SCOPED_APP_ROOT_OVERRIDE.with(|override_path| override_path.borrow().clone())
    {
        std::fs::create_dir_all(&path).map_err(|source| AppDirError::CreateDir {
            path: path.clone(),
            source,
        })?;
        return Ok(path);
    }
    if let Some(path) = APP_ROOT_OVERRIDE
        .lock()
        .expect("app root override mutex poisoned")
        .clone()
    {
        std::fs::create_dir_all(&path).map_err(|source| AppDirError::CreateDir {
            path: path.clone(),
            source,
        })?;
        return Ok(path);
    }
    let base = config_base_dir().ok_or(AppDirError::NoBaseDir)?;
    let path = resolve_profile_app_root(&base)?;
    std::fs::create_dir_all(&path).map_err(|source| AppDirError::CreateDir {
        path: path.clone(),
        source,
    })?;
    Ok(path)
}

fn config_base_dir() -> Option<PathBuf> {
    #[cfg(test)]
    if let Some(path) = TEST_CONFIG_OVERRIDE.with(|override_path| override_path.borrow().clone()) {
        return Some(path);
    }
    if let Some(path) = CONFIG_BASE_OVERRIDE
        .lock()
        .ok()
        .and_then(|guard| guard.clone())
    {
        return Some(path);
    }
    if let Ok(path) = std::env::var("WAVECRATE_CONFIG_HOME") {
        return Some(PathBuf::from(path));
    }
    BaseDirs::new().map(|dirs| dirs.config_dir().to_path_buf())
}

fn resolve_profile_app_root(base: &std::path::Path) -> Result<PathBuf, AppDirError> {
    match effective_profile() {
        ProfileSelection::Live => Ok(base.join(APP_DIR_NAME)),
        ProfileSelection::Named(profile) => {
            let sanitized = sanitize_profile_name(&profile)?;
            Ok(base
                .join(APP_DIR_NAME)
                .join(PROFILE_DIR_NAME)
                .join(sanitized))
        }
    }
}

fn effective_profile() -> ProfileSelection {
    current_profile_override()
        .or_else(profile_from_env)
        .unwrap_or_else(default_profile)
}

fn should_auto_isolate_test_config_base() -> bool {
    std::env::var_os("WAVECRATE_CONFIG_HOME").is_none()
        && effective_profile() != ProfileSelection::Live
        && running_under_test_harness()
}

fn profile_from_env() -> Option<ProfileSelection> {
    let raw = std::env::var(CONFIG_PROFILE_ENV).ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.eq_ignore_ascii_case("live") {
        return Some(ProfileSelection::Live);
    }
    if trimmed.eq_ignore_ascii_case("automated") {
        return Some(ProfileSelection::Named(String::from(
            AUTOMATED_PROFILE_NAME,
        )));
    }
    if trimmed.eq_ignore_ascii_case(SANDBOX_PROFILE_NAME) {
        return Some(ProfileSelection::Named(String::from(SANDBOX_PROFILE_NAME)));
    }
    Some(ProfileSelection::Named(trimmed.to_string()))
}

fn default_profile() -> ProfileSelection {
    if running_under_test_harness() {
        return ProfileSelection::Named(String::from(AUTOMATED_PROFILE_NAME));
    }
    ProfileSelection::Live
}

fn running_under_test_harness() -> bool {
    cfg!(test)
        || std::env::current_exe().ok().is_some_and(|path| {
            path.parent()
                .and_then(std::path::Path::file_name)
                .is_some_and(|name| name == TEST_EXECUTABLE_DIR_NAME)
        })
}

fn sanitize_profile_name(profile: &str) -> Result<String, AppDirError> {
    let trimmed = profile.trim();
    if trimmed.is_empty() {
        return Err(AppDirError::InvalidProfileName {
            profile: profile.to_string(),
        });
    }
    let mut sanitized = String::with_capacity(trimmed.len());
    for ch in trimmed.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
            sanitized.push(ch);
        } else {
            return Err(AppDirError::InvalidProfileName {
                profile: profile.to_string(),
            });
        }
    }
    Ok(sanitized)
}

#[cfg(test)]
fn current_profile_override() -> Option<ProfileSelection> {
    SCOPED_PROFILE_OVERRIDE.with(|override_profile| override_profile.borrow().clone())
}

#[cfg(not(test))]
fn current_profile_override() -> Option<ProfileSelection> {
    None
}

#[cfg(test)]
mod tests;
