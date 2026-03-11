#![deny(missing_docs)]
#![deny(warnings)]
// The map-view repository split is already tracked in `tmp/cleanup_plan.md` item 19.
// Keep this lint narrowly suppressed at the crate boundary until that extraction lands,
// rather than touching the oversized transitional module in unrelated housekeeping work.
#![allow(clippy::type_complexity)]

//! Library exports for reuse in benchmarks and tests.
extern crate alloc;
/// Background analysis helpers.
pub mod analysis;
/// Keep app internals compiled for the binary/runtime while the library target
/// intentionally reuses only a subset of that surface.
#[allow(dead_code)]
mod app;
/// Backend-neutral app-core projection and action helpers used during GUI migration.
pub mod app_core;
/// Application directory helpers.
pub mod app_dirs;
/// Audio playback utilities.
pub mod audio;
/// Shared helpers used by companion binaries such as the installer and updater helper.
pub mod companion_apps;
/// Internal helpers for parsing environment-flag booleans.
mod env_flags;
/// Platform helpers for copying files to the clipboard.
pub mod external_clipboard;
/// Platform helpers for external drag-and-drop.
pub mod external_drag;
/// Backend-agnostic GUI façade for the `radiant`-based UI stack.
///
/// This crate exposes GUI declarations (`radiant` APIs) to application code while
/// keeping widget behavior, layout policy, input semantics, and rendering inside
/// the `radiant` crate.
pub mod gui;
/// Shared runtime host glue that starts native `radiant` hosts.
///
/// The runtime boundary only adapts launch options and forwards lifecycle/error
/// events; it does not define UI widgets, input handling policies, or layout
/// logic.
pub mod gui_runtime;
/// Shared helpers for low-overhead hot-path telemetry instrumentation.
mod hotpath_telemetry;
mod http_client;
/// GitHub issue reporting via the Sempal gateway.
pub mod issue_gateway;
/// Logging setup helpers.
pub mod logging;
/// Sample source management.
pub mod sample_sources;
/// Selection math utilities.
pub mod selection;
/// Optional SQLite extension loader.
pub mod sqlite_ext;
/// Update check + installer helper utilities.
pub mod updater;
/// WAV header sanitization helpers.
pub mod wav_sanitize;
/// Waveform decoding and rendering helpers.
pub mod waveform;
