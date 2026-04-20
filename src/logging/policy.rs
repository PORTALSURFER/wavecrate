//! Debug logging policy resolution.
//!
//! This module keeps the enablement contract for richer per-launch diagnostics
//! narrow and explicit so later instrumentation can rely on one Sempal-owned
//! switch instead of subsystem-specific ad-hoc toggles.

use std::ffi::OsString;

use tracing_subscriber::EnvFilter;

/// Environment variable that opt-ins Sempal-owned debug diagnostics.
pub const DEBUG_LOGGING_ENV_VAR: &str = "SEMPAL_DEBUG_LOGGING";
/// Command-line argument that opt-ins Sempal-owned debug diagnostics.
pub const DEBUG_LOGGING_ARG: &str = "--log";
/// Legacy short command-line argument that opt-ins Sempal-owned debug diagnostics.
pub const DEBUG_LOGGING_SHORT_ARG: &str = "-log";
const RUST_LOG_ENV_VAR: &str = "RUST_LOG";
const DEFAULT_FILTER: &str = "info";
const DEBUG_FILTER: &str = "sempal=debug,info";

/// Resolved mode for Sempal-owned debug diagnostics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DebugLoggingMode {
    /// Rich debug diagnostics remain disabled.
    Standard,
    /// Rich debug diagnostics are enabled for Sempal-owned events.
    Enabled,
}

impl DebugLoggingMode {
    /// Returns `true` when richer debug diagnostics are enabled.
    pub const fn enabled(self) -> bool {
        matches!(self, Self::Enabled)
    }

    /// Human-readable mode label for structured startup logs.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Enabled => "debug",
        }
    }
}

/// Resolved logging settings for one application launch.
#[derive(Clone, Debug)]
pub struct DebugLoggingSettings {
    mode: DebugLoggingMode,
    env_filter: EnvFilter,
    filter_source: &'static str,
    filter_description: String,
    enabled_by_launch_arg: bool,
    invalid_debug_value: Option<String>,
}

impl DebugLoggingSettings {
    /// Resolve settings from launch arguments plus the current process environment.
    pub fn from_process<I>(args: I) -> Self
    where
        I: IntoIterator<Item = OsString>,
    {
        Self::from_values(
            launch_arg_present(args),
            std::env::var(DEBUG_LOGGING_ENV_VAR).ok(),
            std::env::var(RUST_LOG_ENV_VAR).ok(),
        )
    }

    /// Returns the resolved Sempal-owned debug logging mode.
    pub const fn mode(&self) -> DebugLoggingMode {
        self.mode
    }

    /// Returns the tracing filter that should be installed globally.
    pub fn env_filter(&self) -> EnvFilter {
        self.env_filter.clone()
    }

    /// Returns a short label that explains where the active filter came from.
    pub const fn filter_source(&self) -> &'static str {
        self.filter_source
    }

    /// Returns the human-readable filter expression used for startup logs.
    pub fn filter_description(&self) -> &str {
        &self.filter_description
    }

    /// Returns `true` when a launch argument enabled debug diagnostics.
    pub const fn enabled_by_launch_arg(&self) -> bool {
        self.enabled_by_launch_arg
    }

    /// Returns the rejected debug env value when parsing fell back to disabled.
    pub fn invalid_debug_value(&self) -> Option<&str> {
        self.invalid_debug_value.as_deref()
    }

    fn from_values(
        enabled_by_launch_arg: bool,
        debug_value: Option<String>,
        rust_log_value: Option<String>,
    ) -> Self {
        let debug_flag = parse_debug_logging_flag(debug_value.as_deref());
        let mode = if enabled_by_launch_arg || debug_flag.enabled {
            DebugLoggingMode::Enabled
        } else {
            DebugLoggingMode::Standard
        };
        if let Some(raw_filter) = rust_log_value {
            return Self {
                mode,
                env_filter: EnvFilter::new(raw_filter.clone()),
                filter_source: RUST_LOG_ENV_VAR,
                filter_description: raw_filter,
                enabled_by_launch_arg,
                invalid_debug_value: debug_flag.invalid_value,
            };
        }

        let filter_description = if mode.enabled() {
            DEBUG_FILTER
        } else {
            DEFAULT_FILTER
        };
        Self {
            mode,
            env_filter: EnvFilter::new(filter_description),
            filter_source: "default",
            filter_description: filter_description.to_string(),
            enabled_by_launch_arg,
            invalid_debug_value: debug_flag.invalid_value,
        }
    }
}

#[derive(Debug, Default)]
struct DebugFlagParse {
    enabled: bool,
    invalid_value: Option<String>,
}

fn parse_debug_logging_flag(raw: Option<&str>) -> DebugFlagParse {
    let Some(value) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
        return DebugFlagParse::default();
    };
    if is_truthy(value) {
        return DebugFlagParse {
            enabled: true,
            invalid_value: None,
        };
    }
    if is_falsy(value) {
        return DebugFlagParse::default();
    }
    DebugFlagParse {
        enabled: false,
        invalid_value: Some(value.to_string()),
    }
}

fn is_truthy(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn is_falsy(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "0" | "false" | "no" | "off"
    )
}

fn launch_arg_present<I>(args: I) -> bool
where
    I: IntoIterator<Item = OsString>,
{
    args.into_iter().any(|arg| {
        arg == OsString::from(DEBUG_LOGGING_ARG) || arg == OsString::from(DEBUG_LOGGING_SHORT_ARG)
    })
}

#[cfg(test)]
mod tests {
    use super::{
        DEBUG_FILTER, DEBUG_LOGGING_SHORT_ARG, DEFAULT_FILTER, DebugLoggingMode,
        DebugLoggingSettings,
    };
    use std::ffi::OsString;

    #[test]
    fn defaults_to_standard_info_logging() {
        let settings = DebugLoggingSettings::from_values(false, None, None);
        assert_eq!(settings.mode(), DebugLoggingMode::Standard);
        assert_eq!(settings.filter_source(), "default");
        assert_eq!(settings.filter_description(), DEFAULT_FILTER);
        assert!(!settings.enabled_by_launch_arg());
        assert!(settings.invalid_debug_value().is_none());
    }

    #[test]
    fn launch_arg_enables_debug_mode_with_sempal_owned_filter() {
        let settings = DebugLoggingSettings::from_values(true, None, None);
        assert_eq!(settings.mode(), DebugLoggingMode::Enabled);
        assert_eq!(settings.filter_description(), DEBUG_FILTER);
        assert!(settings.enabled_by_launch_arg());
    }

    #[test]
    fn legacy_short_launch_arg_enables_debug_mode() {
        let settings = DebugLoggingSettings::from_process([
            OsString::from("sempal"),
            OsString::from(DEBUG_LOGGING_SHORT_ARG),
        ]);
        assert_eq!(settings.mode(), DebugLoggingMode::Enabled);
        assert!(settings.enabled_by_launch_arg());
    }

    #[test]
    fn canonical_launch_arg_enables_debug_mode() {
        let settings =
            DebugLoggingSettings::from_process([OsString::from("sempal"), OsString::from("--log")]);
        assert_eq!(settings.mode(), DebugLoggingMode::Enabled);
        assert!(settings.enabled_by_launch_arg());
    }

    #[test]
    fn env_var_enables_debug_mode_with_sempal_owned_filter() {
        let settings = DebugLoggingSettings::from_values(false, Some("1".to_string()), None);
        assert_eq!(settings.mode(), DebugLoggingMode::Enabled);
        assert_eq!(settings.filter_description(), DEBUG_FILTER);
    }

    #[test]
    fn launch_arg_still_enables_debug_mode_when_env_var_is_false() {
        let settings = DebugLoggingSettings::from_values(true, Some("off".to_string()), None);
        assert_eq!(settings.mode(), DebugLoggingMode::Enabled);
        assert_eq!(settings.filter_description(), DEBUG_FILTER);
        assert!(settings.enabled_by_launch_arg());
        assert!(settings.invalid_debug_value().is_none());
    }

    #[test]
    fn explicit_false_env_var_keeps_standard_mode() {
        let settings = DebugLoggingSettings::from_values(false, Some("false".to_string()), None);
        assert_eq!(settings.mode(), DebugLoggingMode::Standard);
        assert_eq!(settings.filter_description(), DEFAULT_FILTER);
    }

    #[test]
    fn keeps_debug_mode_when_rust_log_overrides_filter() {
        let settings = DebugLoggingSettings::from_values(
            false,
            Some("yes".to_string()),
            Some("warn,wgpu=error".to_string()),
        );
        assert_eq!(settings.mode(), DebugLoggingMode::Enabled);
        assert_eq!(settings.filter_source(), "RUST_LOG");
        assert_eq!(settings.filter_description(), "warn,wgpu=error");
    }

    #[test]
    fn rejects_unknown_debug_logging_values_without_enabling_mode() {
        let settings = DebugLoggingSettings::from_values(false, Some("verbose".to_string()), None);
        assert_eq!(settings.mode(), DebugLoggingMode::Standard);
        assert_eq!(settings.invalid_debug_value(), Some("verbose"));
    }
}
