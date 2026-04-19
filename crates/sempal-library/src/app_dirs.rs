//! Application directory helpers anchored to a single `.sempal` folder.
//!
//! The helpers centralize where config and log files live across platforms,
//! defaulting to the OS config directory (e.g., `%APPDATA%` on Windows) and
//! allowing a `SEMPAL_CONFIG_HOME` override for tests or portable setups.

use std::{
    cell::RefCell,
    path::PathBuf,
    sync::{LazyLock, Mutex},
};

use directories::BaseDirs;
use thiserror::Error;

/// Name of the application directory that lives under the OS config root.
pub const APP_DIR_NAME: &str = ".sempal";
/// Name of the directory that stores explicit non-live persistence profiles.
pub const PROFILE_DIR_NAME: &str = "profiles";

const CONFIG_PROFILE_ENV: &str = "SEMPAL_CONFIG_PROFILE";
#[cfg(test)]
const TEST_PROFILE_NAME: &str = "automated-tests";

static CONFIG_BASE_OVERRIDE: LazyLock<Mutex<Option<PathBuf>>> = LazyLock::new(|| Mutex::new(None));
static APP_ROOT_OVERRIDE: LazyLock<Mutex<Option<PathBuf>>> = LazyLock::new(|| Mutex::new(None));
static TEST_CONFIG_BASE: LazyLock<PathBuf> = LazyLock::new(|| {
    let dir = tempfile::tempdir().expect("create test config dir");
    let path = dir.path().to_path_buf();
    // Keep the directory alive for the test process.
    std::mem::forget(dir);
    path
});

thread_local! {
    static TEST_CONFIG_OVERRIDE: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}
thread_local! {
    static SCOPED_APP_ROOT_OVERRIDE: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}
thread_local! {
    static SCOPED_PROFILE_OVERRIDE: RefCell<Option<PersistenceProfile>> = const { RefCell::new(None) };
}

/// Ensure tests do not touch real user config directories.
pub fn ensure_test_config_base() {
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
    #[error("Invalid Sempal profile name '{profile}'")]
    InvalidProfileName {
        /// Rejected profile name.
        profile: String,
    },
}

/// Persistence profile that controls which app root the process should use.
#[derive(Clone, Debug, PartialEq, Eq)]
enum PersistenceProfile {
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
    previous_profile: Option<PersistenceProfile>,
    previous_root: Option<PathBuf>,
}

impl PersistenceProfileGuard {
    /// Force the current thread onto the live persistence profile.
    pub fn live() -> Self {
        Self::set(PersistenceProfile::Live)
    }

    /// Force the current thread onto one named non-live persistence profile.
    pub fn named(profile: impl Into<String>) -> Self {
        Self::set(PersistenceProfile::Named(profile.into()))
    }

    fn set(profile: PersistenceProfile) -> Self {
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

/// Return the root `.sempal` directory, creating it if needed.
pub fn app_root_dir() -> Result<PathBuf, AppDirError> {
    #[cfg(test)]
    if current_profile_override().as_ref() != Some(&PersistenceProfile::Live) {
        ensure_test_config_base();
    }

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

/// Override the resolved application root directory (the `.sempal` folder).
pub fn set_app_root_override(path: PathBuf) -> Result<(), AppDirError> {
    std::fs::create_dir_all(&path).map_err(|source| AppDirError::CreateDir {
        path: path.clone(),
        source,
    })?;
    if TEST_CONFIG_OVERRIDE.with(|override_path| override_path.borrow().is_some())
        || SCOPED_PROFILE_OVERRIDE.with(|override_profile| override_profile.borrow().is_some())
    {
        SCOPED_APP_ROOT_OVERRIDE.with(|override_path| {
            *override_path.borrow_mut() = Some(path);
        });
        return Ok(());
    }
    let mut guard = APP_ROOT_OVERRIDE
        .lock()
        .expect("app root override mutex poisoned");
    *guard = Some(path);
    Ok(())
}

/// Return the logs directory inside the `.sempal` root, creating it if needed.
pub fn logs_dir() -> Result<PathBuf, AppDirError> {
    let path = app_root_dir()?.join("logs");
    std::fs::create_dir_all(&path).map_err(|source| AppDirError::CreateDir {
        path: path.clone(),
        source,
    })?;
    Ok(path)
}

/// Return the base directory used for `.sempal` when no override is set.
pub fn config_base_dir_path() -> Option<PathBuf> {
    config_base_dir()
}

fn config_base_dir() -> Option<PathBuf> {
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
    if let Ok(path) = std::env::var("SEMPAL_CONFIG_HOME") {
        return Some(PathBuf::from(path));
    }
    BaseDirs::new().map(|dirs| dirs.config_dir().to_path_buf())
}

fn resolve_profile_app_root(base: &std::path::Path) -> Result<PathBuf, AppDirError> {
    match current_profile_override()
        .or_else(profile_from_env)
        .unwrap_or_else(default_profile)
    {
        PersistenceProfile::Live => Ok(base.join(APP_DIR_NAME)),
        PersistenceProfile::Named(profile) => {
            let sanitized = sanitize_profile_name(&profile)?;
            Ok(base
                .join(APP_DIR_NAME)
                .join(PROFILE_DIR_NAME)
                .join(sanitized))
        }
    }
}

fn current_profile_override() -> Option<PersistenceProfile> {
    SCOPED_PROFILE_OVERRIDE.with(|override_profile| override_profile.borrow().clone())
}

fn profile_from_env() -> Option<PersistenceProfile> {
    let raw = std::env::var(CONFIG_PROFILE_ENV).ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.eq_ignore_ascii_case("live") {
        return Some(PersistenceProfile::Live);
    }
    Some(PersistenceProfile::Named(trimmed.to_string()))
}

fn default_profile() -> PersistenceProfile {
    #[cfg(test)]
    {
        return PersistenceProfile::Named(String::from(TEST_PROFILE_NAME));
    }
    #[cfg(not(test))]
    {
        PersistenceProfile::Live
    }
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

/// Guard that sets a temporary config base path for tests and restores the prior value.
pub struct ConfigBaseGuard {
    previous: Option<PathBuf>,
    previous_scoped_root: Option<PathBuf>,
    previous_root: Option<PathBuf>,
}

impl ConfigBaseGuard {
    /// Override the config base directory for the lifetime of the guard.
    pub fn set(path: PathBuf) -> Self {
        let previous = TEST_CONFIG_OVERRIDE.with(|override_path| {
            let mut slot = override_path.borrow_mut();
            let prev = slot.clone();
            *slot = Some(path);
            prev
        });
        let previous_scoped_root = SCOPED_APP_ROOT_OVERRIDE.with(|override_path| {
            let mut slot = override_path.borrow_mut();
            let prev = slot.clone();
            *slot = None;
            prev
        });
        let mut root_guard = APP_ROOT_OVERRIDE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let previous_root = root_guard.clone();
        *root_guard = None;
        Self {
            previous,
            previous_root,
            previous_scoped_root,
        }
    }
}

impl Drop for ConfigBaseGuard {
    fn drop(&mut self) {
        let previous = self.previous.take();
        TEST_CONFIG_OVERRIDE.with(|override_path| {
            *override_path.borrow_mut() = previous;
        });
        let previous_scoped_root = self.previous_scoped_root.take();
        SCOPED_APP_ROOT_OVERRIDE.with(|override_path| {
            *override_path.borrow_mut() = previous_scoped_root;
        });
        let mut root_guard = APP_ROOT_OVERRIDE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *root_guard = self.previous_root.take();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn uses_override_for_root_dir() {
        let base = tempdir().unwrap();
        let _guard = ConfigBaseGuard::set(base.path().to_path_buf());
        let root = app_root_dir().unwrap();
        assert_eq!(
            root,
            base.path()
                .join(APP_DIR_NAME)
                .join(PROFILE_DIR_NAME)
                .join(TEST_PROFILE_NAME)
        );
        assert!(root.is_dir());
    }

    #[test]
    fn reapplies_test_override_when_cleared() {
        {
            let mut guard = CONFIG_BASE_OVERRIDE
                .lock()
                .expect("config base override mutex poisoned");
            *guard = None;
        }
        let root = app_root_dir().unwrap();
        assert!(root.ends_with(TEST_PROFILE_NAME));

        {
            let mut guard = CONFIG_BASE_OVERRIDE
                .lock()
                .expect("config base override mutex poisoned");
            *guard = None;
        }
        let root2 = app_root_dir().unwrap();
        assert!(root2.ends_with(TEST_PROFILE_NAME));
    }

    #[test]
    fn named_profile_uses_isolated_profile_root() {
        let base = tempdir().unwrap();
        let _base_guard = ConfigBaseGuard::set(base.path().to_path_buf());
        let _profile_guard = PersistenceProfileGuard::named("gui-test");

        let root = app_root_dir().unwrap();

        assert_eq!(
            root,
            base.path()
                .join(APP_DIR_NAME)
                .join(PROFILE_DIR_NAME)
                .join("gui-test")
        );
        assert!(root.is_dir());
    }

    #[test]
    fn live_profile_override_bypasses_test_isolation() {
        let live_base = tempdir().unwrap();
        let expected = live_base.path().join(APP_DIR_NAME);
        {
            let mut guard = CONFIG_BASE_OVERRIDE
                .lock()
                .expect("config base override mutex poisoned");
            *guard = Some(live_base.path().to_path_buf());
        }
        let _profile_guard = PersistenceProfileGuard::live();

        let root = app_root_dir().unwrap();

        assert_eq!(root, expected);
        assert!(root.is_dir());
    }

    #[test]
    fn rejects_invalid_profile_names() {
        let base = tempdir().unwrap();
        let _base_guard = ConfigBaseGuard::set(base.path().to_path_buf());
        let _profile_guard = PersistenceProfileGuard::named("bad/profile");

        let error = app_root_dir().expect_err("invalid profile should fail");

        assert!(matches!(error, AppDirError::InvalidProfileName { .. }));
    }
}
