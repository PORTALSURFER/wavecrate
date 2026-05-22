use super::*;
use std::sync::{LazyLock, Mutex, MutexGuard};
use tempfile::tempdir;

static APP_DIRS_TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

fn app_dirs_test_lock() -> MutexGuard<'static, ()> {
    APP_DIRS_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[test]
fn uses_override_for_root_dir() {
    let _lock = app_dirs_test_lock();
    let base = tempdir().expect("tempdir");
    let _guard = ConfigBaseGuard::set(base.path().to_path_buf());

    let root = app_root_dir().expect("root dir");

    assert_eq!(
        root,
        base.path()
            .join(APP_DIR_NAME)
            .join(PROFILE_DIR_NAME)
            .join(AUTOMATED_PROFILE_NAME)
    );
    assert!(root.is_dir());
}

#[test]
fn reapplies_test_override_when_cleared() {
    let _lock = app_dirs_test_lock();
    {
        let mut guard = CONFIG_BASE_OVERRIDE
            .lock()
            .expect("config base override mutex poisoned");
        *guard = None;
    }

    let root = app_root_dir().expect("root dir");
    assert!(root.ends_with(AUTOMATED_PROFILE_NAME));

    {
        let mut guard = CONFIG_BASE_OVERRIDE
            .lock()
            .expect("config base override mutex poisoned");
        *guard = None;
    }

    let root2 = app_root_dir().expect("root dir");
    assert!(root2.ends_with(AUTOMATED_PROFILE_NAME));
}

#[test]
fn named_profile_uses_isolated_profile_root() {
    let _lock = app_dirs_test_lock();
    let base = tempdir().expect("tempdir");
    let _base_guard = ConfigBaseGuard::set(base.path().to_path_buf());
    let _profile_guard = PersistenceProfileGuard::named("gui-test");

    let root = app_root_dir().expect("root dir");

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
    let _lock = app_dirs_test_lock();
    let live_base = tempdir().expect("tempdir");
    {
        let mut guard = CONFIG_BASE_OVERRIDE
            .lock()
            .expect("config base override mutex poisoned");
        *guard = Some(live_base.path().to_path_buf());
    }
    let _profile_guard = PersistenceProfileGuard::live();

    let root = app_root_dir().expect("root dir");

    assert_eq!(root, live_base.path().join(APP_DIR_NAME));
    assert!(root.is_dir());
}

#[test]
fn sandbox_profile_uses_dedicated_profile_root() {
    let _lock = app_dirs_test_lock();
    let base = tempdir().expect("tempdir");
    let _base_guard = ConfigBaseGuard::set(base.path().to_path_buf());
    let _profile_guard = PersistenceProfileGuard::sandbox();

    let root = app_root_dir().expect("root dir");

    assert_eq!(
        root,
        base.path()
            .join(APP_DIR_NAME)
            .join(PROFILE_DIR_NAME)
            .join(SANDBOX_PROFILE_NAME)
    );
    assert!(root.is_dir());
}

#[test]
fn automated_profile_guard_uses_canonical_profile_root() {
    let _lock = app_dirs_test_lock();
    let base = tempdir().expect("tempdir");
    let _base_guard = ConfigBaseGuard::set(base.path().to_path_buf());
    let _profile_guard = PersistenceProfileGuard::automated();

    let root = app_root_dir().expect("root dir");

    assert_eq!(
        root,
        base.path()
            .join(APP_DIR_NAME)
            .join(PROFILE_DIR_NAME)
            .join(AUTOMATED_PROFILE_NAME)
    );
    assert!(root.is_dir());
}

#[test]
fn rejects_invalid_profile_names() {
    let _lock = app_dirs_test_lock();
    let base = tempdir().expect("tempdir");
    let _base_guard = ConfigBaseGuard::set(base.path().to_path_buf());
    let _profile_guard = PersistenceProfileGuard::named("bad/profile");

    let error = app_root_dir().expect_err("invalid profile should fail");

    assert!(matches!(error, AppDirError::InvalidProfileName { .. }));
}
