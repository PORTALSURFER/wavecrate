#![deny(missing_docs)]
#![deny(warnings)]
// Keep this crate-boundary allowance narrow while compatibility-heavy projection
// and runtime surfaces are still being decomposed across the active cleanup lane.
#![allow(clippy::type_complexity)]
#![allow(
    clippy::cloned_ref_to_slice_refs,
    clippy::cmp_owned,
    clippy::collapsible_if,
    clippy::double_ended_iterator_last,
    clippy::field_reassign_with_default,
    clippy::identity_op,
    clippy::if_same_then_else,
    clippy::items_after_test_module,
    clippy::large_enum_variant,
    clippy::manual_clamp,
    clippy::manual_is_multiple_of,
    clippy::manual_unwrap_or_default,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::ptr_arg,
    clippy::question_mark,
    clippy::result_large_err,
    clippy::single_match,
    clippy::too_many_arguments,
    clippy::unnecessary_get_then_check,
    clippy::unnecessary_literal_unwrap
)]

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
pub use wavecrate_library::app_dirs;
#[cfg(test)]
mod app_dirs_tests;
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
/// GUI test contracts, scenario types, and artifact helpers.
pub mod gui_test;
/// Shared helpers for low-overhead hot-path telemetry instrumentation.
mod hotpath_telemetry;
mod http_client;
/// GitHub issue reporting via the Wavecrate gateway.
pub mod issue_gateway;
/// Logging setup helpers.
pub mod logging;
/// Shared runtime host glue that starts native `radiant` hosts.
///
/// The runtime boundary only adapts launch options and forwards lifecycle/error
/// events; it does not define UI widgets, input handling policies, or layout
/// logic.
pub mod native_runtime;
/// Sample source management.
pub mod sample_sources;
/// Selection math utilities.
pub mod selection;
/// Backend-agnostic GUI façade for the `radiant`-based UI stack.
///
/// This crate exposes GUI declarations (`radiant` APIs) to application code while
/// keeping widget behavior, layout policy, input semantics, and rendering inside
/// the `radiant` crate.
pub mod ui_primitives;
/// Optional SQLite extension loader.
pub use wavecrate_library::sqlite_ext;
/// Update check + installer helper utilities.
pub mod updater;
/// WAV header sanitization helpers.
pub mod wav_sanitize;
/// Waveform decoding and rendering helpers.
pub mod waveform;
