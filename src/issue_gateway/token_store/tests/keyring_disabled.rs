use super::*;

#[test]
fn fallback_key_cache_recovers_after_poison() {
    enable_mock_keyring();
    let _env_guard = env_lock();
    reset_cache();
    unsafe {
        std::env::set_var("WAVECRATE_DISABLE_KEYRING", "1");
    }
    allow_fallback();
    set_env_key();
    let base = tempdir().unwrap();
    let _guard = app_dirs::ConfigBaseGuard::set(base.path().to_path_buf());
    let store = IssueTokenStore::new().unwrap();

    let _ = std::panic::catch_unwind(|| {
        let _guard = fallback_key::fallback_key_cache()
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
        std::env::remove_var("WAVECRATE_DISABLE_KEYRING");
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
        std::env::set_var("WAVECRATE_DISABLE_KEYRING", "1");
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
        std::env::remove_var("WAVECRATE_DISABLE_KEYRING");
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
        std::env::set_var("WAVECRATE_DISABLE_KEYRING", "1");
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
        std::env::remove_var("WAVECRATE_DISABLE_KEYRING");
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
        std::env::set_var("WAVECRATE_DISABLE_KEYRING", "1");
    }
    let base = tempdir().unwrap();
    let _guard = crate::app_dirs::ConfigBaseGuard::set(base.path().to_path_buf());
    let store = IssueTokenStore::new().unwrap();

    let err = store.set("tok_abcdefghijklmnopqrstuvwxyz").unwrap_err();
    match err {
        IssueTokenStoreError::Unavailable(message) => {
            assert!(message.contains(FALLBACK_ALLOW_ENV));
        }
        other => panic!("expected unavailable error, got {other:?}"),
    }
    assert!(!store.fallback_token_path().exists());

    unsafe {
        std::env::remove_var("WAVECRATE_DISABLE_KEYRING");
    }
    clear_env_key();
}
