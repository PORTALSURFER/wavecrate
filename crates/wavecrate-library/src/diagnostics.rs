//! Shared structured debug diagnostics helpers for library-owned storage seams.
//!
//! These helpers mirror the app crate's debug DB event schema so source/library
//! storage code can emit consistent reconstruction events without depending on
//! the binary crate.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Target used for standardized database debug events.
pub const DB_EVENT_TARGET: &str = "wavecrate::debug::db";

static DEBUG_LOGGING_ENABLED: AtomicBool = AtomicBool::new(false);

/// Standardized database event fields for richer debug diagnostics.
///
/// Every emitted DB event includes:
/// - `event="db"`
/// - `operation`
/// - `source`
/// - `outcome`
/// - `elapsed_ms`
/// - `error`
///
/// Sensitive values must never be logged here. Avoid secrets, credentials, raw
/// SQL with interpolated private values, or large user-authored payloads.
#[derive(Clone, Copy, Debug)]
pub struct DbDebugEvent<'a> {
    /// Stable database operation name such as `source_db.open`.
    pub operation: &'a str,
    /// Optional source context such as `library` or a source-root display string.
    pub source: Option<&'a str>,
    /// Outcome classification such as `success`, `error`, `retry`, or `slow`.
    pub outcome: &'a str,
    /// Wall-clock elapsed time for the DB work.
    pub elapsed: Duration,
    /// Sanitized failure text when the operation failed.
    pub error: Option<&'a str>,
}

/// Update whether richer Wavecrate-owned debug diagnostics are enabled.
pub fn set_debug_logging_enabled(enabled: bool) {
    DEBUG_LOGGING_ENABLED.store(enabled, Ordering::Relaxed);
}

/// Return whether richer Wavecrate-owned debug diagnostics are enabled.
pub fn debug_logging_enabled() -> bool {
    DEBUG_LOGGING_ENABLED.load(Ordering::Relaxed)
}

/// Emit one standardized database debug event when debug logging mode is enabled.
pub fn emit_db_debug_event(event: DbDebugEvent<'_>) {
    if !debug_logging_enabled() {
        return;
    }
    tracing::debug!(
        target: DB_EVENT_TARGET,
        event = "db",
        operation = event.operation,
        source = event.source.unwrap_or_default(),
        outcome = event.outcome,
        elapsed_ms = event.elapsed.as_millis() as u64,
        error = event.error.unwrap_or_default(),
        "Database debug event"
    );
}

#[cfg(test)]
mod tests {
    use super::{debug_logging_enabled, set_debug_logging_enabled};

    #[test]
    fn debug_logging_flag_round_trips() {
        set_debug_logging_enabled(true);
        assert!(debug_logging_enabled());
        set_debug_logging_enabled(false);
        assert!(!debug_logging_enabled());
    }
}
