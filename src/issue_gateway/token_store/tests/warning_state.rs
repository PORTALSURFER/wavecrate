use super::*;

#[test]
fn fallback_warns_when_active() {
    enable_mock_keyring();
    let _env_guard = env_lock();
    unsafe {
        std::env::set_var("WAVECRATE_DISABLE_KEYRING", "1");
    }
    allow_fallback();
    set_env_key();
    fallback_policy::reset_fallback_warning_for_tests();
    let base = tempdir().unwrap();
    let _guard = app_dirs::ConfigBaseGuard::set(base.path().to_path_buf());
    let store = IssueTokenStore::new().unwrap();

    store.set("tok_abcdefghijklmnopqrstuvwxyz").unwrap();
    assert!(fallback_policy::fallback_warning_emitted_for_tests());

    unsafe {
        std::env::remove_var("WAVECRATE_DISABLE_KEYRING");
    }
    disallow_fallback();
    clear_env_key();
}
