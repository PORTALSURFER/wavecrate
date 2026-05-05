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
pub use sempal_library::app_dirs;
#[cfg(test)]
mod app_dirs_tests;
/// Audio playback utilities.
pub mod audio;
/// Shared helpers used by companion binaries such as the installer and updater helper.
pub mod companion_apps;
mod compat_app_contract;
/// Internal helpers for parsing environment-flag booleans.
mod env_flags;
/// Platform helpers for copying files to the clipboard.
pub mod external_clipboard;
/// Platform helpers for external drag-and-drop.
pub mod external_drag;
pub(crate) mod layout {
    pub(crate) use radiant::layout::*;
}
pub(crate) mod runtime {
    pub(crate) use radiant::runtime::*;
}
pub(crate) mod widgets {
    pub(crate) use radiant::widgets::*;
}
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
/// GUI test contracts, scenario types, and artifact helpers.
pub mod gui_test;
/// Shared helpers for low-overhead hot-path telemetry instrumentation.
mod hotpath_telemetry;
mod http_client;
#[allow(dead_code)]
pub(crate) mod theme {
    #[cfg(test)]
    pub(crate) use radiant::theme::DEFAULT_UI_SCALE;
    pub(crate) use radiant::theme::{
        ThemeTokens, ViewportScaleTier, clamp_ui_scale, effective_ui_scale,
    };

    pub(crate) struct TierVisualPolicy {
        pub(crate) state_hover_soft: f32,
        pub(crate) state_hover_strong: f32,
        pub(crate) state_selected_blend: f32,
        pub(crate) state_focus_pulse_blend: f32,
        pub(crate) motion_speed_transport: f32,
        pub(crate) motion_speed_idle: f32,
        pub(crate) motion_focus_wave_amp: f32,
        pub(crate) motion_focus_text_wave_amp: f32,
        pub(crate) scrim_soft_alpha: u8,
        pub(crate) scrim_modal_alpha: u8,
    }

    pub(crate) fn visual_policy_for_tier(layout_tier: ViewportScaleTier) -> TierVisualPolicy {
        match layout_tier {
            ViewportScaleTier::Compact => TierVisualPolicy {
                state_hover_soft: 0.10,
                state_hover_strong: 0.16,
                state_selected_blend: 0.10,
                state_focus_pulse_blend: 0.20,
                motion_speed_transport: 2.2,
                motion_speed_idle: 1.0,
                motion_focus_wave_amp: 0.06,
                motion_focus_text_wave_amp: 0.03,
                scrim_soft_alpha: 164,
                scrim_modal_alpha: 180,
            },
            ViewportScaleTier::Wide => TierVisualPolicy {
                state_hover_soft: 0.12,
                state_hover_strong: 0.20,
                state_selected_blend: 0.13,
                state_focus_pulse_blend: 0.25,
                motion_speed_transport: 2.8,
                motion_speed_idle: 1.2,
                motion_focus_wave_amp: 0.08,
                motion_focus_text_wave_amp: 0.04,
                scrim_soft_alpha: 180,
                scrim_modal_alpha: 196,
            },
            ViewportScaleTier::Standard => TierVisualPolicy {
                state_hover_soft: 0.12,
                state_hover_strong: 0.20,
                state_selected_blend: 0.12,
                state_focus_pulse_blend: 0.24,
                motion_speed_transport: 2.6,
                motion_speed_idle: 1.2,
                motion_focus_wave_amp: 0.08,
                motion_focus_text_wave_amp: 0.04,
                scrim_soft_alpha: 172,
                scrim_modal_alpha: 188,
            },
        }
    }
}
/// GitHub issue reporting via the Sempal gateway.
pub mod issue_gateway;
/// Logging setup helpers.
pub mod logging;
/// Sample source management.
pub mod sample_sources;
/// Selection math utilities.
pub mod selection;
/// Optional SQLite extension loader.
pub use sempal_library::sqlite_ext;
/// Update check + installer helper utilities.
pub mod updater;
/// WAV header sanitization helpers.
pub mod wav_sanitize;
/// Waveform decoding and rendering helpers.
pub mod waveform;
