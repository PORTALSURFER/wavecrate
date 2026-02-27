#![deny(missing_docs)]
#![deny(warnings)]

//! Library exports for reuse in benchmarks and tests.
extern crate alloc;
/// Background analysis helpers.
pub mod analysis;
#[allow(dead_code)]
mod app;
/// Backend-neutral app-core projection and action helpers used during GUI migration.
pub mod app_core;
/// Application directory helpers.
pub mod app_dirs;
/// Audio playback utilities.
pub mod audio;
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
