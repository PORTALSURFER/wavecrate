use super::*;

#[test]
fn fallback_warns_when_active() {
    enable_mock_keyring();
    let mut runtime = env_lock();
    runtime.set_var("WAVECRATE_DISABLE_KEYRING", "1");
    allow_fallback(&mut runtime);
    set_env_key(&mut runtime);
    fallback_policy::reset_fallback_warning_for_tests();
    let base = tempdir().unwrap();
    let _guard = app_dirs::ConfigBaseGuard::set(base.path().to_path_buf());
    let store = IssueTokenStore::new().unwrap();

    store.set("tok_abcdefghijklmnopqrstuvwxyz").unwrap();
    assert!(fallback_policy::fallback_warning_emitted_for_tests());
}
