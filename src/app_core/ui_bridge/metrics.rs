//! Bridge profiling counters, snapshots, and trace hooks.
//!
//! Bridge profiling remains feature- and env-gated. Default `info` logs should
//! stay high-signal, while sampled per-call bridge lifecycle traces remain
//! available as debug-only diagnostics for focused local investigation.

/// Bridge action call counters and fallback call IDs.
mod calls;
/// Bridge call duration timers.
mod durations;
/// Native frame result counters.
mod frame_result;
/// Classified interaction action counters.
mod interaction;
/// Projection cache hit/miss counters.
mod projection;
/// Process-lifetime metric registry and environment switches.
mod registry;
/// Human-readable bridge profile reporting.
mod reporting;
/// Bridge metric snapshot capture.
mod snapshot;
/// Waveform, derived-graph, and projection-key counters.
mod waveform;

pub(super) use self::calls::{trace_action_call, trace_pull_model_call, trace_pull_motion_call};
pub(super) use self::durations::{
    trace_action_duration, trace_pull_model_preparation, trace_pull_model_projection,
    trace_pull_motion_preparation, trace_pull_motion_projection,
};
pub(super) use self::frame_result::trace_frame_result;
pub(super) use self::interaction::trace_action_interaction;
pub(super) use self::projection::{trace_projection_cache_lookup, trace_projection_segment_lookup};
#[cfg(feature = "native-bridge-metrics")]
pub(super) use self::registry::{
    BRIDGE_PROFILE_INTERVAL, bridge_profiling_enabled, projection_key_assertions_enabled,
};
#[cfg(not(feature = "native-bridge-metrics"))]
pub(super) use self::registry::{
    BRIDGE_PROFILE_INTERVAL, bridge_profiling_enabled, projection_key_assertions_enabled,
};
#[cfg(all(test, feature = "native-bridge-metrics"))]
pub(super) use self::registry::{
    PROJECTION_CACHE_HIT_COUNT, PROJECTION_CACHE_MISS_COUNT, WAVEFORM_IMAGE_REFRESH_APPLY_COUNT,
    WAVEFORM_IMAGE_REFRESH_SKIP_COUNT,
};
pub(super) use self::reporting::maybe_log_bridge_profile;
pub(super) use self::waveform::{
    trace_derived_flush, trace_projection_key_assertion, trace_waveform_flush,
    trace_waveform_image_refresh,
};
