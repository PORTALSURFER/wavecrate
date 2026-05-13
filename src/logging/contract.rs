//! Structured debug event helpers.
//!
//! These helpers define the field schema for richer runtime diagnostics so
//! controller, lifecycle, and DB instrumentation can stay grep-friendly and
//! consistent across subsystems.

use std::time::Duration;

/// Target used for standardized action debug events.
pub const ACTION_EVENT_TARGET: &str = "wavecrate::debug::action";
/// Target used for standardized database debug events.
pub const DB_EVENT_TARGET: &str = "wavecrate::debug::db";

/// Standardized action event fields for richer debug diagnostics.
///
/// Every emitted action event includes:
/// - `event="action"`
/// - `action`
/// - `pane`
/// - `source`
/// - `outcome`
/// - `elapsed_ms`
/// - `error`
///
/// `pane`, `source`, and `error` use the empty string when a value is not
/// relevant for the event so grep/search tooling can rely on stable field
/// names.
///
/// Sensitive values must never be logged here. In particular: secrets, API
/// tokens, auth headers, key material, raw credentials, filesystem paths that
/// expose private user data unless operationally necessary, and large unredacted
/// user-authored payloads.
#[derive(Clone, Copy, Debug)]
pub struct ActionDebugEvent<'a> {
    /// Stable action name such as `browser.focus_preview`.
    pub action: &'a str,
    /// Optional UI pane or surface that handled the action.
    pub pane: Option<&'a str>,
    /// Optional source context such as a fixture tag or source root alias.
    pub source: Option<&'a str>,
    /// Outcome classification such as `success`, `error`, or `cancelled`.
    pub outcome: &'a str,
    /// Wall-clock elapsed time for the action.
    pub elapsed: Duration,
    /// Sanitized failure text when the action failed.
    pub error: Option<&'a str>,
}

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
/// `source` and `error` use the empty string when a value is not relevant for
/// the event.
///
/// Sensitive values must never be logged here. Avoid raw SQL with interpolated
/// values, auth tokens, secrets, full sample metadata payloads, or any user
/// content that is not required to diagnose the failing seam.
#[derive(Clone, Copy, Debug)]
pub struct DbDebugEvent<'a> {
    /// Stable database operation name such as `transaction_begin`.
    pub operation: &'a str,
    /// Optional source context such as a source-root display string or profile.
    pub source: Option<&'a str>,
    /// Outcome classification such as `success`, `error`, or `retry`.
    pub outcome: &'a str,
    /// Wall-clock elapsed time for the DB work.
    pub elapsed: Duration,
    /// Sanitized failure text when the operation failed.
    pub error: Option<&'a str>,
}

/// Emit one standardized action debug event when debug logging mode is enabled.
pub fn emit_action_debug_event(event: ActionDebugEvent<'_>) {
    if !super::debug_logging_enabled() {
        return;
    }
    tracing::debug!(
        target: ACTION_EVENT_TARGET,
        event = "action",
        action = event.action,
        pane = event.pane.unwrap_or_default(),
        source = event.source.unwrap_or_default(),
        outcome = event.outcome,
        elapsed_ms = event.elapsed.as_millis() as u64,
        error = event.error.unwrap_or_default(),
        "Action debug event"
    );
}

/// Emit one standardized database debug event when debug logging mode is enabled.
pub fn emit_db_debug_event(event: DbDebugEvent<'_>) {
    if !super::debug_logging_enabled() {
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
