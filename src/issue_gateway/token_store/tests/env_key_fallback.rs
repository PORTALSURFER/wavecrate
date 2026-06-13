use super::*;

#[test]
fn fallback_requires_env_key_without_keyring() {
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

    let err = store.set("tok_file_fallback").unwrap_err();
    match err {
        IssueTokenStoreError::Unavailable(message) => {
            assert!(message.contains(FALLBACK_KEY_ENV_VAR));
        }
        other => panic!("expected unavailable error, got {other:?}"),
    }

    unsafe {
        std::env::remove_var("WAVECRATE_DISABLE_KEYRING");
    }
    disallow_fallback();
}

#[test]
fn malformed_env_fallback_key_is_rejected() {
    enable_mock_keyring();
    let _env_guard = env_lock();
    reset_cache();
    unsafe {
        std::env::set_var("WAVECRATE_DISABLE_KEYRING", "1");
        std::env::set_var(FALLBACK_KEY_ENV_VAR, "not-hex");
    }
    allow_fallback();
    let base = tempdir().unwrap();
    let _guard = app_dirs::ConfigBaseGuard::set(base.path().to_path_buf());
    let store = IssueTokenStore::new().unwrap();

    let err = store.set("tok_file_fallback").unwrap_err();
    match err {
        IssueTokenStoreError::Decode(message) => {
            assert!(message.contains(FALLBACK_KEY_ENV_VAR));
        }
        other => panic!("expected decode error, got {other:?}"),
    }

    unsafe {
        std::env::remove_var("WAVECRATE_DISABLE_KEYRING");
    }
    disallow_fallback();
    clear_env_key();
}

#[test]
fn fallback_works_with_env_key() {
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

    store.set("tok_env_fallback").unwrap();
    assert_eq!(store.get().unwrap().as_deref(), Some("tok_env_fallback"));

    assert!(!store.legacy_fallback_key_path().exists());

    store.delete().unwrap();

    unsafe {
        std::env::remove_var("WAVECRATE_DISABLE_KEYRING");
    }
    disallow_fallback();
    clear_env_key();
}
