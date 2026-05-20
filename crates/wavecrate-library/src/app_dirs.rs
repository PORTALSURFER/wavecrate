//! Application directory helpers anchored to a single `.wavecrate` folder.
//!
//! The helpers centralize where config and log files live across platforms,
//! defaulting to the OS config directory (e.g., `%APPDATA%` on Windows) and
//! allowing a `WAVECRATE_CONFIG_HOME` override for tests or portable setups.
//! Test executables default to an isolated `automated-tests` profile unless
//! they explicitly request the live profile or another config root.

use std::{
    cell::RefCell,
    path::PathBuf,
    sync::{LazyLock, Mutex},
};

use directories::BaseDirs;
use thiserror::Error;

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
enum ProfileSelection {
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

/// Return the root `.wavecrate` directory, creating it if needed.
pub fn app_root_dir() -> Result<PathBuf, AppDirError> {
    if should_auto_isolate_test_config_base() {
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
    Ok(resolve_persistence()?.app_root)
}

/// Resolve the current config base, app root, and high-level persistence mode.
pub fn resolve_persistence() -> Result<ResolvedPersistence, AppDirError> {
    if should_auto_isolate_test_config_base() {
        ensure_test_config_base();
    }
    if let Some(path) =
        SCOPED_APP_ROOT_OVERRIDE.with(|override_path| override_path.borrow().clone())
    {
        std::fs::create_dir_all(&path).map_err(|source| AppDirError::CreateDir {
            path: path.clone(),
            source,
        })?;
        return Ok(ResolvedPersistence {
            config_base: path.parent().map_or_else(|| path.clone(), PathBuf::from),
            app_root: path,
            mode: persistence_mode_from_selection(&effective_profile()),
        });
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
        return Ok(ResolvedPersistence {
            config_base: path.parent().map_or_else(|| path.clone(), PathBuf::from),
            app_root: path,
            mode: persistence_mode_from_selection(&effective_profile()),
        });
    }
    let base = config_base_dir().ok_or(AppDirError::NoBaseDir)?;
    let selection = effective_profile();
    let app_root = resolve_profile_app_root(&base, &selection)?;
    std::fs::create_dir_all(&app_root).map_err(|source| AppDirError::CreateDir {
        path: app_root.clone(),
        source,
    })?;
    Ok(ResolvedPersistence {
        config_base: base,
        app_root,
        mode: persistence_mode_from_selection(&selection),
    })
}

/// Return the high-level persistence mode that applies to the current run.
pub fn persistence_mode() -> PersistenceMode {
    persistence_mode_from_selection(&effective_profile())
}

/// Override the resolved application root directory (the `.wavecrate` folder).
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

/// Return the logs directory inside the `.wavecrate` root, creating it if needed.
pub fn logs_dir() -> Result<PathBuf, AppDirError> {
    let path = app_root_dir()?.join("logs");
    std::fs::create_dir_all(&path).map_err(|source| AppDirError::CreateDir {
        path: path.clone(),
        source,
    })?;
    Ok(path)
}

/// Return the global handoff staging directory inside the `.wavecrate` root.
pub fn handoff_staging_dir() -> Result<PathBuf, AppDirError> {
    let path = app_root_dir()?.join("handoff_staging");
    std::fs::create_dir_all(&path).map_err(|source| AppDirError::CreateDir {
        path: path.clone(),
        source,
    })?;
    Ok(path)
}

/// Return the base directory used for `.wavecrate` when no override is set.
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
    if let Ok(path) = std::env::var("WAVECRATE_CONFIG_HOME") {
        return Some(PathBuf::from(path));
    }
    BaseDirs::new().map(|dirs| dirs.config_dir().to_path_buf())
}

fn resolve_profile_app_root(
    base: &std::path::Path,
    selection: &ProfileSelection,
) -> Result<PathBuf, AppDirError> {
    match selection {
        ProfileSelection::Live => Ok(base.join(APP_DIR_NAME)),
        ProfileSelection::Named(profile) => {
            let sanitized = sanitize_profile_name(profile)?;
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

fn current_profile_override() -> Option<ProfileSelection> {
    SCOPED_PROFILE_OVERRIDE.with(|override_profile| override_profile.borrow().clone())
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

fn persistence_mode_from_selection(selection: &ProfileSelection) -> PersistenceMode {
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
                .join(AUTOMATED_PROFILE_NAME)
        );
        assert!(root.is_dir());
        assert_eq!(persistence_mode(), PersistenceMode::Automated);
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
        assert!(root.ends_with(AUTOMATED_PROFILE_NAME));

        {
            let mut guard = CONFIG_BASE_OVERRIDE
                .lock()
                .expect("config base override mutex poisoned");
            *guard = None;
        }
        let root2 = app_root_dir().unwrap();
        assert!(root2.ends_with(AUTOMATED_PROFILE_NAME));
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
        assert_eq!(persistence_mode(), PersistenceMode::Live);
    }

    #[test]
    fn sandbox_profile_uses_dedicated_mode_and_root() {
        let base = tempdir().unwrap();
        let _base_guard = ConfigBaseGuard::set(base.path().to_path_buf());
        let _profile_guard = PersistenceProfileGuard::sandbox();

        let resolved = resolve_persistence().expect("resolve sandbox persistence");

        assert_eq!(resolved.mode, PersistenceMode::Sandbox);
        assert_eq!(
            resolved.app_root,
            base.path()
                .join(APP_DIR_NAME)
                .join(PROFILE_DIR_NAME)
                .join(SANDBOX_PROFILE_NAME)
        );
    }

    #[test]
    fn automated_profile_guard_uses_canonical_mode() {
        let base = tempdir().unwrap();
        let _base_guard = ConfigBaseGuard::set(base.path().to_path_buf());
        let _profile_guard = PersistenceProfileGuard::automated();

        let resolved = resolve_persistence().expect("resolve automated persistence");

        assert_eq!(resolved.mode, PersistenceMode::Automated);
        assert_eq!(
            resolved.app_root,
            base.path()
                .join(APP_DIR_NAME)
                .join(PROFILE_DIR_NAME)
                .join(AUTOMATED_PROFILE_NAME)
        );
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
