//! Shared environment-flag parsing helpers.

/// Return whether the provided environment variable resolves to a truthy token.
pub(crate) fn env_var_truthy(key: &str) -> bool {
    std::env::var(key)
        .ok()
        .is_some_and(|value| is_truthy(&value))
}

fn is_truthy(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}
