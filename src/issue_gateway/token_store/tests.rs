use super::*;
use std::sync::{Mutex, Once, OnceLock};
use tempfile::tempdir;

/// Strict env-toggle parsing tests.
mod env_flags;

static MOCK_KEYRING_INIT: Once = Once::new();
static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn enable_mock_keyring() {
    MOCK_KEYRING_INIT.call_once(|| {
        keyring::set_default_credential_builder(keyring::mock::default_credential_builder());
    });
}

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn allow_fallback() {
    unsafe {
        std::env::set_var(FALLBACK_ALLOW_ENV, "1");
    }
}

fn disallow_fallback() {
    unsafe {
        std::env::remove_var(FALLBACK_ALLOW_ENV);
    }
}

fn set_env_key() -> String {
    let env_key = "A".repeat(64);
    unsafe {
        std::env::set_var(FALLBACK_KEY_ENV_VAR, &env_key);
    }
    env_key
}

fn clear_env_key() {
    unsafe {
        std::env::remove_var(FALLBACK_KEY_ENV_VAR);
    }
}

fn reset_cache() {
    *lock_fallback_key_cache() = None;
}

#[test]
fn fallback_key_cache_recovers_after_poison() {
    enable_mock_keyring();
    let _env_guard = env_lock();
    reset_cache();
    unsafe {
        std::env::set_var("SEMPAL_DISABLE_KEYRING", "1");
    }
    allow_fallback();
    set_env_key();
    let base = tempdir().unwrap();
    let _guard = app_dirs::ConfigBaseGuard::set(base.path().to_path_buf());
    let store = IssueTokenStore::new().unwrap();

    let _ = std::panic::catch_unwind(|| {
        let _guard = fallback_key_cache()
            .lock()
            .expect("poison fallback key cache");
        panic!("poison fallback key cache");
    });

    store.set("tok_abcdefghijklmnopqrstuvwxyz").unwrap();
    assert_eq!(
        store.get().unwrap().as_deref(),
        Some("tok_abcdefghijklmnopqrstuvwxyz")
    );
    store.delete().unwrap();
    unsafe {
        std::env::remove_var("SEMPAL_DISABLE_KEYRING");
    }
    disallow_fallback();
    clear_env_key();
}

#[test]
fn fallback_roundtrip_when_keyring_disabled() {
    enable_mock_keyring();
    let _env_guard = env_lock();
    reset_cache();
    unsafe {
        std::env::set_var("SEMPAL_DISABLE_KEYRING", "1");
    }
    allow_fallback();
    set_env_key();
    let base = tempdir().unwrap();
    let _guard = app_dirs::ConfigBaseGuard::set(base.path().to_path_buf());
    let store = IssueTokenStore::new().unwrap();
    assert_eq!(store.get().unwrap(), None);
    store.set("tok_abcdefghijklmnopqrstuvwxyz").unwrap();
    assert_eq!(
        store.get().unwrap().as_deref(),
        Some("tok_abcdefghijklmnopqrstuvwxyz")
    );
    store.delete().unwrap();
    assert_eq!(store.get().unwrap(), None);
    unsafe {
        std::env::remove_var("SEMPAL_DISABLE_KEYRING");
    }
    disallow_fallback();
    clear_env_key();
}

#[test]
fn set_empty_token_clears_storage() {
    enable_mock_keyring();
    let _env_guard = env_lock();
    reset_cache();
    unsafe {
        std::env::set_var("SEMPAL_DISABLE_KEYRING", "1");
    }
    allow_fallback();
    set_env_key();
    let base = tempdir().unwrap();
    let _guard = app_dirs::ConfigBaseGuard::set(base.path().to_path_buf());
    let store = IssueTokenStore::new().unwrap();
    store.set("tok_abcdefghijklmnopqrstuvwxyz").unwrap();
    store.set("").unwrap();
    assert_eq!(store.get().unwrap(), None);
    unsafe {
        std::env::remove_var("SEMPAL_DISABLE_KEYRING");
    }
    disallow_fallback();
    clear_env_key();
}

#[test]
fn fallback_is_only_used_when_explicitly_allowed() {
    enable_mock_keyring();
    let _env_guard = env_lock();
    reset_cache();
    disallow_fallback();
    clear_env_key();
    unsafe {
        std::env::set_var("SEMPAL_DISABLE_KEYRING", "1");
    }
    let base = tempdir().unwrap();
    let _guard = crate::app_dirs::ConfigBaseGuard::set(base.path().to_path_buf());
    let store = IssueTokenStore::new().unwrap();

    // Fallback should be disabled unless explicitly allowed.
    let err = store.set("tok_abcdefghijklmnopqrstuvwxyz").unwrap_err();
    match err {
        IssueTokenStoreError::Unavailable(message) => {
            assert!(message.contains(FALLBACK_ALLOW_ENV));
        }
        other => panic!("expected unavailable error, got {other:?}"),
    }
    assert!(!store.fallback_token_path().exists());

    unsafe {
        std::env::remove_var("SEMPAL_DISABLE_KEYRING");
    }
    clear_env_key();
}

#[test]
fn fallback_get_rejects_corrupted_payload() {
    enable_mock_keyring();
    let _env_guard = env_lock();
    reset_cache();
    unsafe {
        std::env::set_var("SEMPAL_DISABLE_KEYRING", "1");
    }
    allow_fallback();
    set_env_key();
    let base = tempdir().unwrap();
    let _guard = app_dirs::ConfigBaseGuard::set(base.path().to_path_buf());
    let store = IssueTokenStore::new().unwrap();

    std::fs::write(store.fallback_token_path(), b"short").unwrap();
    let err = store.fallback_get().unwrap_err();
    match err {
        IssueTokenStoreError::Decode(_) => {}
        other => panic!("expected decode error, got {other:?}"),
    }

    unsafe {
        std::env::remove_var("SEMPAL_DISABLE_KEYRING");
    }
    disallow_fallback();
    clear_env_key();
}

#[cfg(unix)]
#[test]
fn fallback_token_file_is_private_on_unix() {
    enable_mock_keyring();
    let _env_guard = env_lock();
    reset_cache();
    use std::os::unix::fs::PermissionsExt;
    unsafe {
        std::env::set_var("SEMPAL_DISABLE_KEYRING", "1");
    }
    allow_fallback();
    set_env_key();
    let base = tempdir().unwrap();
    let _guard = app_dirs::ConfigBaseGuard::set(base.path().to_path_buf());
    let store = IssueTokenStore::new().unwrap();

    store.set("tok_abcdefghijklmnopqrstuvwxyz").unwrap();
    let token_mode = std::fs::metadata(store.fallback_token_path())
        .unwrap()
        .permissions()
        .mode()
        & 0o777;

    assert_eq!(token_mode, 0o600);

    unsafe {
        std::env::remove_var("SEMPAL_DISABLE_KEYRING");
    }
    disallow_fallback();
    clear_env_key();
}

#[test]
fn fallback_get_rejects_oversized_payload() {
    enable_mock_keyring();
    let _env_guard = env_lock();
    reset_cache();
    unsafe {
        std::env::set_var("SEMPAL_DISABLE_KEYRING", "1");
    }
    allow_fallback();
    set_env_key();
    let base = tempdir().unwrap();
    let _guard = app_dirs::ConfigBaseGuard::set(base.path().to_path_buf());
    let store = IssueTokenStore::new().unwrap();

    let oversized = vec![0u8; (MAX_FALLBACK_TOKEN_BYTES + 1) as usize];
    std::fs::write(store.fallback_token_path(), oversized).unwrap();
    let err = store.fallback_get().unwrap_err();
    match err {
        IssueTokenStoreError::Decode(message) => {
            assert!(message.contains("exceeds"));
        }
        other => panic!("expected decode error, got {other:?}"),
    }

    unsafe {
        std::env::remove_var("SEMPAL_DISABLE_KEYRING");
    }
    disallow_fallback();
    clear_env_key();
}

#[test]
fn fallback_get_clears_unreadable_payload() {
    enable_mock_keyring();
    let _env_guard = env_lock();
    reset_cache();
    unsafe {
        std::env::set_var("SEMPAL_DISABLE_KEYRING", "1");
    }
    allow_fallback();
    set_env_key();
    let base = tempdir().unwrap();
    let _guard = app_dirs::ConfigBaseGuard::set(base.path().to_path_buf());
    let store = IssueTokenStore::new().unwrap();

    let mut payload = vec![0u8; 12];
    payload.extend_from_slice(&[1u8; 16]);
    std::fs::write(store.fallback_token_path(), payload).unwrap();
    assert_eq!(store.fallback_get().unwrap(), None);
    assert!(!store.fallback_token_path().exists());

    unsafe {
        std::env::remove_var("SEMPAL_DISABLE_KEYRING");
    }
    disallow_fallback();
    clear_env_key();
}

#[test]
fn fallback_get_migrates_legacy_key_file() {
    enable_mock_keyring();
    let _env_guard = env_lock();
    reset_cache();
    allow_fallback();
    let base = tempdir().unwrap();
    let _guard = app_dirs::ConfigBaseGuard::set(base.path().to_path_buf());
    let store = IssueTokenStore::new().unwrap();

    let legacy_key_bytes = random_bytes(32).unwrap();
    let mut legacy_key = [0u8; 32];
    legacy_key.copy_from_slice(&legacy_key_bytes);
    write_private_file(&store.legacy_fallback_key_path(), &legacy_key_bytes).unwrap();
    let legacy_payload = store
        .encrypt_fallback_payload(&legacy_key, b"tok_legacy")
        .unwrap();
    write_private_file(&store.fallback_token_path(), &legacy_payload).unwrap();

    // Should successfully read using the file-based key
    assert_eq!(store.fallback_get().unwrap().as_deref(), Some("tok_legacy"));
    // The key file should be removed after migrating to keyring
    assert!(!store.legacy_fallback_key_path().exists());

    disallow_fallback();
}

#[test]
fn fallback_requires_env_key_without_keyring() {
    enable_mock_keyring();
    let _env_guard = env_lock();
    reset_cache();
    unsafe {
        std::env::set_var("SEMPAL_DISABLE_KEYRING", "1");
    }
    allow_fallback();
    let base = tempdir().unwrap();
    let _guard = app_dirs::ConfigBaseGuard::set(base.path().to_path_buf());
    let store = IssueTokenStore::new().unwrap();

    let err = store.set("tok_file_fallback").unwrap_err();
    match err {
        IssueTokenStoreError::Unavailable(message) => {
            assert!(message.contains(FALLBACK_KEY_ENV_VAR));
        }
        other => panic!("expected unavailable error, got {other:?}"),
    }

    unsafe {
        std::env::remove_var("SEMPAL_DISABLE_KEYRING");
    }
    disallow_fallback();
}

#[test]
fn fallback_works_with_env_key() {
    enable_mock_keyring();
    let _env_guard = env_lock();
    reset_cache();
    unsafe {
        std::env::set_var("SEMPAL_DISABLE_KEYRING", "1");
    }
    allow_fallback();
    set_env_key();

    let base = tempdir().unwrap();
    let _guard = app_dirs::ConfigBaseGuard::set(base.path().to_path_buf());
    let store = IssueTokenStore::new().unwrap();

    store.set("tok_env_fallback").unwrap();
    assert_eq!(store.get().unwrap().as_deref(), Some("tok_env_fallback"));

    // Key file should NOT exist because we provided env var
    assert!(!store.legacy_fallback_key_path().exists());

    store.delete().unwrap();

    unsafe {
        std::env::remove_var("SEMPAL_DISABLE_KEYRING");
    }
    disallow_fallback();
    clear_env_key();
}

#[test]
fn fallback_warns_when_active() {
    enable_mock_keyring();
    let _env_guard = env_lock();
    unsafe {
        std::env::set_var("SEMPAL_DISABLE_KEYRING", "1");
    }
    allow_fallback();
    set_env_key();
    FALLBACK_WARNING_EMITTED.store(false, Ordering::SeqCst);
    let base = tempdir().unwrap();
    let _guard = app_dirs::ConfigBaseGuard::set(base.path().to_path_buf());
    let store = IssueTokenStore::new().unwrap();

    store.set("tok_abcdefghijklmnopqrstuvwxyz").unwrap();
    assert!(FALLBACK_WARNING_EMITTED.load(Ordering::SeqCst));

    unsafe {
        std::env::remove_var("SEMPAL_DISABLE_KEYRING");
    }
    disallow_fallback();
    clear_env_key();
}
