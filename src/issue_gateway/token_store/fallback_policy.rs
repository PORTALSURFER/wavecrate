use super::FALLBACK_ALLOW_ENV;
use std::sync::atomic::{AtomicBool, Ordering};

static FALLBACK_WARNING_EMITTED: AtomicBool = AtomicBool::new(false);

pub(super) fn keyring_disabled() -> bool {
    env_var_truthy("WAVECRATE_DISABLE_KEYRING")
}

pub(super) fn fallback_allowed() -> bool {
    env_var_truthy(FALLBACK_ALLOW_ENV)
}

/// Resolve security-sensitive env toggles using strict tokens only.
///
/// This intentionally accepts only `1` and `true` (ASCII case-insensitive).
/// Unlike the broader shared env parser, we do **not** accept aliases like
/// `yes`/`on` to reduce accidental enablement of keyring-bypass and fallback
/// secret-storage paths.
pub(super) fn env_var_truthy(key: &str) -> bool {
    std::env::var(key)
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

pub(super) fn warn_fallback_active() {
    if FALLBACK_WARNING_EMITTED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        tracing::warn!(
            "Fallback token storage enabled; ciphertext is stored on disk and the encryption key is stored in the OS keyring or provided via environment."
        );
    }
}

#[cfg(test)]
pub(super) fn reset_fallback_warning_for_tests() {
    FALLBACK_WARNING_EMITTED.store(false, Ordering::SeqCst);
}

#[cfg(test)]
pub(super) fn fallback_warning_emitted_for_tests() -> bool {
    FALLBACK_WARNING_EMITTED.load(Ordering::SeqCst)
}
