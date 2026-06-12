//! Async folder availability refresh and row projection orchestration.

/// Filesystem-backed available-folder derivation.
mod available;
#[path = "projection_telemetry.rs"]
/// Folder projection timing telemetry.
mod projection_telemetry;
/// Controller-side queue, apply, and stale-result handling.
mod queue;
/// Worker-side snapshot construction.
mod snapshot;
#[cfg(test)]
/// Test-only async execution override.
mod test_control;
/// Off-thread folder projection worker.
mod worker;

#[cfg(test)]
pub(crate) use test_control::with_folder_projection_async_enabled_for_tests;
