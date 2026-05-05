use super::*;

/// Security toggles must only accept strict `1`/`true` tokens.
#[test]
fn strict_env_var_truthy_only_accepts_one_and_true() {
    let _env_guard = env_lock();
    /// Test-only env var for strict token parsing behavior.
    const STRICT_ENV: &str = "SEMPAL_TOKEN_STORE_STRICT_PARSE_TEST";
    unsafe {
        std::env::remove_var(STRICT_ENV);
    }
    assert!(!env_var_truthy(STRICT_ENV));

    unsafe {
        std::env::set_var(STRICT_ENV, "1");
    }
    assert!(env_var_truthy(STRICT_ENV));

    unsafe {
        std::env::set_var(STRICT_ENV, "TrUe");
    }
    assert!(env_var_truthy(STRICT_ENV));

    unsafe {
        std::env::set_var(STRICT_ENV, "on");
    }
    assert!(!env_var_truthy(STRICT_ENV));

    unsafe {
        std::env::set_var(STRICT_ENV, "yes");
    }
    assert!(!env_var_truthy(STRICT_ENV));

    unsafe {
        std::env::set_var(STRICT_ENV, "true ");
    }
    assert!(!env_var_truthy(STRICT_ENV));

    unsafe {
        std::env::remove_var(STRICT_ENV);
    }
}
