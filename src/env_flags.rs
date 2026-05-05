//! Shared environment-flag parsing helpers.
//!
//! This centralizes permissive boolean parsing so runtime/config paths do not
//! duplicate token handling across modules.

/// Parse a permissive boolean token from an environment-variable-like string.
///
/// Accepted truthy values are `1`, `true`, `yes`, and `on` (case-insensitive).
/// Accepted falsy values are `0`, `false`, `no`, and `off` (case-insensitive).
/// Returns `None` when the token is unrecognized.
pub(crate) fn parse_env_bool(value: &str) -> Option<bool> {
    let normalized = value.trim();
    if normalized.eq_ignore_ascii_case("1")
        || normalized.eq_ignore_ascii_case("true")
        || normalized.eq_ignore_ascii_case("yes")
        || normalized.eq_ignore_ascii_case("on")
    {
        Some(true)
    } else if normalized.eq_ignore_ascii_case("0")
        || normalized.eq_ignore_ascii_case("false")
        || normalized.eq_ignore_ascii_case("no")
        || normalized.eq_ignore_ascii_case("off")
    {
        Some(false)
    } else {
        None
    }
}

/// Return whether the input is one of the accepted truthy tokens.
pub(crate) fn is_truthy(value: &str) -> bool {
    matches!(parse_env_bool(value), Some(true))
}

/// Return whether the provided environment variable resolves to a truthy token.
///
/// Missing or unrecognized values resolve to `false`.
pub(crate) fn env_var_truthy(key: &str) -> bool {
    std::env::var(key)
        .ok()
        .is_some_and(|value| is_truthy(&value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_env_bool_accepts_truthy_and_falsy_tokens() {
        assert_eq!(parse_env_bool("1"), Some(true));
        assert_eq!(parse_env_bool("TRUE"), Some(true));
        assert_eq!(parse_env_bool(" yes "), Some(true));
        assert_eq!(parse_env_bool("on"), Some(true));
        assert_eq!(parse_env_bool("0"), Some(false));
        assert_eq!(parse_env_bool("False"), Some(false));
        assert_eq!(parse_env_bool(" NO "), Some(false));
        assert_eq!(parse_env_bool("off"), Some(false));
    }

    #[test]
    fn parse_env_bool_rejects_unknown_tokens() {
        assert_eq!(parse_env_bool(""), None);
        assert_eq!(parse_env_bool("2"), None);
        assert_eq!(parse_env_bool("enabled"), None);
    }

    #[test]
    fn is_truthy_only_accepts_truthy_tokens() {
        assert!(is_truthy("yes"));
        assert!(!is_truthy("no"));
        assert!(!is_truthy("invalid"));
    }
}
