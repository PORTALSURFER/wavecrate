use super::*;
use std::sync::Once;
use tempfile::tempdir;
use wavecrate_library::test_runtime::TestRuntimeGuard;

mod cleanup_failures;
/// Strict env-toggle parsing tests.
mod env_flags;
mod env_key_fallback;
mod keyring_disabled;
mod legacy_migration;
mod payload_validation;
mod warning_state;

static MOCK_KEYRING_INIT: Once = Once::new();

fn enable_mock_keyring() {
    MOCK_KEYRING_INIT.call_once(|| {
        keyring::set_default_credential_builder(keyring::mock::default_credential_builder());
    });
}

fn env_lock() -> TestRuntimeGuard {
    TestRuntimeGuard::acquire()
}

fn allow_fallback(runtime: &mut TestRuntimeGuard) {
    runtime.set_var(FALLBACK_ALLOW_ENV, "1");
}

fn disallow_fallback(runtime: &mut TestRuntimeGuard) {
    runtime.remove_var(FALLBACK_ALLOW_ENV);
}

fn set_env_key(runtime: &mut TestRuntimeGuard) -> String {
    let env_key = "A".repeat(64);
    runtime.set_var(FALLBACK_KEY_ENV_VAR, &env_key);
    env_key
}

fn clear_env_key(runtime: &mut TestRuntimeGuard) {
    runtime.remove_var(FALLBACK_KEY_ENV_VAR);
}

fn reset_cache() {
    *fallback_key::lock_fallback_key_cache() = None;
}

fn assert_cleanup_failure_for(err: &IssueTokenStoreError, artifact: &'static str) {
    match err {
        IssueTokenStoreError::Cleanup { failures } => {
            assert!(
                failures.iter().any(|failure| failure.artifact == artifact),
                "expected cleanup failure for {artifact}, got {failures:?}"
            );
        }
        other => panic!("expected cleanup failure, got {other:?}"),
    }
}
