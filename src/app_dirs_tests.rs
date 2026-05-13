use crate::app_dirs::{self, ConfigBaseGuard, PersistenceProfileGuard, set_app_root_override};
use std::path::Path;
use tempfile::tempdir;

fn explicit_persistence_env_present() -> bool {
    std::env::var_os("WAVECRATE_CONFIG_HOME").is_some()
        || std::env::var_os("WAVECRATE_CONFIG_PROFILE").is_some()
}

#[test]
fn dependency_app_root_defaults_to_automated_test_profile() {
    if explicit_persistence_env_present() {
        return;
    }

    let root = app_dirs::app_root_dir().expect("resolve app root under test harness");

    assert!(
        root.ends_with(
            Path::new(".wavecrate")
                .join("profiles")
                .join("automated-tests")
        )
    );
    assert!(root.is_dir());
}

#[test]
fn dependency_live_profile_override_keeps_live_root_shape() {
    if explicit_persistence_env_present() {
        return;
    }
    let base = tempdir().expect("create base dir");
    let _base_guard = ConfigBaseGuard::set(base.path().to_path_buf());
    let _profile_guard = PersistenceProfileGuard::live();

    let root = app_dirs::app_root_dir().expect("resolve live app root");

    assert_eq!(root, base.path().join(".wavecrate"));
    assert!(root.is_dir());
}

#[test]
fn dependency_live_logs_dir_stays_under_live_app_root() {
    if explicit_persistence_env_present() {
        return;
    }
    let base = tempdir().expect("create base dir");
    let _base_guard = ConfigBaseGuard::set(base.path().to_path_buf());
    let _profile_guard = PersistenceProfileGuard::live();

    let logs_dir = app_dirs::logs_dir().expect("resolve live logs dir");

    assert_eq!(logs_dir, base.path().join(".wavecrate").join("logs"));
    assert!(logs_dir.is_dir());
}

#[test]
fn dependency_sandbox_logs_dir_stays_under_sandbox_profile_root() {
    if explicit_persistence_env_present() {
        return;
    }
    let base = tempdir().expect("create base dir");
    let _base_guard = ConfigBaseGuard::set(base.path().to_path_buf());
    let _profile_guard = PersistenceProfileGuard::sandbox();

    let logs_dir = app_dirs::logs_dir().expect("resolve sandbox logs dir");

    assert_eq!(
        logs_dir,
        base.path()
            .join(".wavecrate")
            .join("profiles")
            .join("sandbox")
            .join("logs")
    );
    assert!(logs_dir.is_dir());
}

#[test]
fn dependency_explicit_app_root_override_wins_over_test_default() {
    if explicit_persistence_env_present() {
        return;
    }
    let base = tempdir().expect("create config base");
    let override_parent = tempdir().expect("create override parent");
    let override_root = override_parent.path().join("custom-app-root");
    let _base_guard = ConfigBaseGuard::set(base.path().to_path_buf());
    set_app_root_override(override_root.clone()).expect("set app root override");

    let root = app_dirs::app_root_dir().expect("resolve explicit app root");

    assert_eq!(root, override_root);
    assert!(root.is_dir());
}
