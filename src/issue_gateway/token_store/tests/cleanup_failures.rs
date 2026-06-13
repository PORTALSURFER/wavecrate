use super::*;

#[test]
fn fallback_get_reports_cleanup_failure_after_decrypt_failure() {
    enable_mock_keyring();
    let _env_guard = env_lock();
    reset_cache();
    unsafe {
        std::env::set_var("WAVECRATE_DISABLE_KEYRING", "1");
    }
    allow_fallback();
    let base = tempdir().unwrap();
    let _guard = app_dirs::ConfigBaseGuard::set(base.path().to_path_buf());
    let store = IssueTokenStore::new().unwrap();

    store.cache_fallback_key([7u8; 32]);
    let mut payload = vec![0u8; 12];
    payload.extend_from_slice(&[1u8; 16]);
    std::fs::write(store.fallback_token_path(), payload).unwrap();
    std::fs::create_dir(store.legacy_fallback_key_path()).unwrap();

    let err = store.fallback_get().unwrap_err();

    assert_cleanup_failure_for(&err, "legacy fallback key");
    assert!(
        !store.fallback_token_path().exists(),
        "token payload should be removed even when another cleanup artifact fails"
    );
    assert!(store.legacy_fallback_key_path().is_dir());

    unsafe {
        std::env::remove_var("WAVECRATE_DISABLE_KEYRING");
    }
    disallow_fallback();
}

#[test]
fn env_fallback_key_reports_legacy_key_cleanup_failure() {
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
    std::fs::create_dir(store.legacy_fallback_key_path()).unwrap();

    let err = store.set("tok_abcdefghijklmnopqrstuvwxyz").unwrap_err();

    assert_cleanup_failure_for(&err, "legacy fallback key");
    assert!(store.legacy_fallback_key_path().is_dir());
    assert!(
        store.cached_fallback_key().is_none(),
        "failed cleanup should not cache the env key as fully resolved"
    );

    unsafe {
        std::env::remove_var("WAVECRATE_DISABLE_KEYRING");
    }
    disallow_fallback();
    clear_env_key();
}

#[test]
fn fallback_delete_reports_legacy_key_cleanup_failure() {
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
    store.cache_fallback_key([3u8; 32]);
    std::fs::write(store.fallback_token_path(), b"payload").unwrap();
    std::fs::create_dir(store.legacy_fallback_key_path()).unwrap();

    let err = store.fallback_delete().unwrap_err();

    assert_cleanup_failure_for(&err, "legacy fallback key");
    assert!(!store.fallback_token_path().exists());
    assert!(store.cached_fallback_key().is_none());

    unsafe {
        std::env::remove_var("WAVECRATE_DISABLE_KEYRING");
    }
    disallow_fallback();
    clear_env_key();
}

#[test]
fn delete_removes_fallback_payload_and_clears_key_cache() {
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
    assert!(store.fallback_token_path().exists());
    assert!(store.cached_fallback_key().is_some());

    store.delete().unwrap();

    assert!(!store.fallback_token_path().exists());
    assert!(store.cached_fallback_key().is_none());

    unsafe {
        std::env::remove_var("WAVECRATE_DISABLE_KEYRING");
    }
    disallow_fallback();
    clear_env_key();
}
