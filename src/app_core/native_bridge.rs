//! Native runtime bridge implementations for migration-facing runtimes.
//!
//! This module hosts the `radiant` bridge surface so runtime entrypoints can
//! depend on `app_core` instead of legacy runtime module paths.
//!
//! Bridge profiling is opt-in and controlled by the `native-bridge-metrics`
//! cargo feature. When enabled, setting `SEMPAL_NATIVE_BRIDGE_PROFILE`
//! (`1`, `true`, `on`, or `yes`, case-insensitive) enables periodic logging
//! of bridge execution timing and renderer work counts.
//!
//! When the feature is disabled, all profiling calls are compiled out to keep
//! default builds on the hot path with zero added overhead.

use crate::{
    app_core::actions::NativeAppBridge,
    app_core::actions::NativeMotionModel,
    app_core::actions::{
        NativeAppModel, NativeDirtySegments, NativeFrameBuildResult, NativeSegmentRevisions,
        NativeUiAction,
    },
    app_core::app_api::controller_state::{DerivedNodeId, DirtyReason},
    app_core::controller::{
        AppController, AppControllerNativeRuntimeExt, build_native_app_controller,
    },
    app_core::native_shell,
    audio::AudioPlayer,
    waveform::WaveformRenderer,
};
#[cfg(feature = "native-bridge-metrics")]
use std::sync::{
    OnceLock,
    atomic::{AtomicU64, Ordering},
};
use std::{
    cell::RefCell,
    hash::{Hash, Hasher},
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};
use tracing::{error, info};

#[cfg(feature = "native-bridge-metrics")]
const BRIDGE_PROFILE_INTERVAL: u64 = 240;
#[cfg(not(feature = "native-bridge-metrics"))]
const BRIDGE_PROFILE_INTERVAL: u64 = 1;

#[cfg(feature = "native-bridge-metrics")]
const BRIDGE_PROFILE_ENV: &str = "SEMPAL_NATIVE_BRIDGE_PROFILE";
#[cfg(feature = "native-bridge-metrics")]
/// Enable runtime validation that cached projection-key snapshots stay in sync.
const PROJECTION_KEY_ASSERT_ENV: &str = "SEMPAL_NATIVE_BRIDGE_ASSERT_PROJECTION_SNAPSHOT";
/// Toggle immediate application of waveform overlay preview actions.
const IMMEDIATE_WAVEFORM_PREVIEW_ENV: &str = "SEMPAL_NATIVE_BRIDGE_IMMEDIATE_WAVEFORM_PREVIEW";
/// Default mode for immediate waveform overlay preview actions.
const IMMEDIATE_WAVEFORM_PREVIEW_DEFAULT: bool = true;

/// Interaction classes tracked by native bridge profiling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InteractionActionClass {
    /// Wheel-like browser row movement actions.
    Wheel,
    /// Map interaction actions flowing through the bridge.
    MapPanProxy,
    /// Waveform seek/cursor/selection/zoom actions.
    Waveform,
    /// Volume slider interaction actions.
    Volume,
}

/// Projection segments tracked for retained model refresh and profiling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProjectionSegment {
    /// Footer/status string projection.
    StatusBar,
    /// Browser metadata/chrome/action projection.
    BrowserFrame,
    /// Browser visible-row window projection.
    BrowserRowsWindow,
    /// Similarity map panel projection.
    MapPanel,
    /// Waveform panel/chrome projection.
    WaveformOverlay,
}

/// Hit/miss counters for one retained projection segment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProjectionSegmentLookupCount {
    /// Number of model pulls that reused retained projection output.
    pub hit_count: u64,
    /// Number of model pulls that recomputed this projection segment.
    pub miss_count: u64,
}

/// Aggregated hit/miss counters for all retained projection segments.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProjectionSegmentLookupCounts {
    /// Status-bar segment counters.
    pub status_bar: ProjectionSegmentLookupCount,
    /// Browser-frame segment counters.
    pub browser_frame: ProjectionSegmentLookupCount,
    /// Browser rows-window segment counters.
    pub browser_rows_window: ProjectionSegmentLookupCount,
    /// Map-panel segment counters.
    pub map_panel: ProjectionSegmentLookupCount,
    /// Waveform-overlay segment counters.
    pub waveform_overlay: ProjectionSegmentLookupCount,
}

impl ProjectionSegmentLookupCounts {
    /// Record one segment-level lookup decision for the current projection pull.
    fn record_lookup(&mut self, segment: ProjectionSegment, hit: bool) {
        let counts = match segment {
            ProjectionSegment::StatusBar => &mut self.status_bar,
            ProjectionSegment::BrowserFrame => &mut self.browser_frame,
            ProjectionSegment::BrowserRowsWindow => &mut self.browser_rows_window,
            ProjectionSegment::MapPanel => &mut self.map_panel,
            ProjectionSegment::WaveformOverlay => &mut self.waveform_overlay,
        };
        if hit {
            counts.hit_count = counts.hit_count.saturating_add(1);
        } else {
            counts.miss_count = counts.miss_count.saturating_add(1);
        }
    }
}

#[cfg(feature = "native-bridge-metrics")]
static PULL_MODEL_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
static PULL_MODEL_PREP_NS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
static PULL_MODEL_PROJECT_NS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
static PULL_MOTION_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
static PULL_MOTION_PREP_NS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
static PULL_MOTION_PROJECT_NS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
static ACTION_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
static ACTION_DURATION_NS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Total number of projection-cache lookups that reused a cached model.
static PROJECTION_CACHE_HIT_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Total number of projection-cache lookups that required a fresh projection.
static PROJECTION_CACHE_MISS_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Segment-level projection-cache hits for status-bar projection.
static PROJECTION_STATUS_SEGMENT_HIT_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Segment-level projection-cache misses for status-bar projection.
static PROJECTION_STATUS_SEGMENT_MISS_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Segment-level projection-cache hits for browser-frame projection.
static PROJECTION_BROWSER_FRAME_SEGMENT_HIT_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Segment-level projection-cache misses for browser-frame projection.
static PROJECTION_BROWSER_FRAME_SEGMENT_MISS_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Segment-level projection-cache hits for browser-rows projection.
static PROJECTION_BROWSER_ROWS_SEGMENT_HIT_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Segment-level projection-cache misses for browser-rows projection.
static PROJECTION_BROWSER_ROWS_SEGMENT_MISS_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Segment-level projection-cache hits for map-panel projection.
static PROJECTION_MAP_SEGMENT_HIT_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Segment-level projection-cache misses for map-panel projection.
static PROJECTION_MAP_SEGMENT_MISS_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Segment-level projection-cache hits for waveform projection.
static PROJECTION_WAVEFORM_SEGMENT_HIT_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Segment-level projection-cache misses for waveform projection.
static PROJECTION_WAVEFORM_SEGMENT_MISS_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Total count of wheel-class interaction actions.
static ACTION_WHEEL_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Accumulated wheel-class interaction action duration in nanoseconds.
static ACTION_WHEEL_DURATION_NS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Total count of map-proxy-class interaction actions.
static ACTION_MAP_PROXY_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Accumulated map-proxy-class interaction action duration in nanoseconds.
static ACTION_MAP_PROXY_DURATION_NS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Total count of waveform-class interaction actions.
static ACTION_WAVEFORM_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Accumulated waveform-class interaction action duration in nanoseconds.
static ACTION_WAVEFORM_DURATION_NS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Total count of volume-class interaction actions.
static ACTION_VOLUME_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Accumulated volume-class interaction action duration in nanoseconds.
static ACTION_VOLUME_DURATION_NS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Total number of queued waveform flushes applied before projection.
static WAVEFORM_FLUSH_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Accumulated waveform flush duration in nanoseconds.
static WAVEFORM_FLUSH_DURATION_NS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Total number of emitted native waveform actions across queued flushes.
static WAVEFORM_FLUSH_EMITTED_ACTIONS_TOTAL: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Total number of waveform image refresh requests applied during derived flush.
static WAVEFORM_IMAGE_REFRESH_APPLY_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Total number of waveform image refresh requests skipped as overlay-only.
static WAVEFORM_IMAGE_REFRESH_SKIP_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Total number of derived-graph flush passes before projection.
static DERIVED_FLUSH_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Accumulated derived-graph flush duration in nanoseconds.
static DERIVED_FLUSH_DURATION_NS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Total dirty source-node count observed across derived flushes.
static DERIVED_DIRTY_SOURCE_TOTAL: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Total dirty derived-node count observed across derived flushes.
static DERIVED_DIRTY_COMPUTED_TOTAL: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
static FRAME_RESULT_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
static FRAME_RESULT_ANIMATION_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
static FRAME_RESULT_PRIMITIVES_TOTAL: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
static FRAME_RESULT_TEXT_RUNS_TOTAL: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Number of redraws that completed a successful surface present.
static FRAME_RESULT_PRESENTED_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Number of redraws that missed an expected present.
static FRAME_RESULT_MISSED_PRESENT_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Number of presented redraws that exceeded the configured frame budget.
static FRAME_RESULT_JANK_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Sum of reported redraw frame durations in microseconds.
static FRAME_RESULT_TOTAL_US: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Sum of reported present-stage durations in microseconds.
static FRAME_RESULT_PRESENT_US_TOTAL: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Last observed frame budget in microseconds.
static FRAME_RESULT_FRAME_BUDGET_US: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Number of projection-key snapshot validation checks performed.
static PROJECTION_KEY_ASSERT_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Number of stale projection-key snapshots detected by validation checks.
static PROJECTION_KEY_ASSERT_STALE_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
static BRIDGE_PROFILE_ENABLED: OnceLock<bool> = OnceLock::new();
#[cfg(feature = "native-bridge-metrics")]
/// Cached projection-snapshot assertion mode resolved from environment.
static PROJECTION_KEY_ASSERT_ENABLED: OnceLock<bool> = OnceLock::new();
/// Cached immediate-waveform-preview mode resolved from environment.
static IMMEDIATE_WAVEFORM_PREVIEW_ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

#[cfg(feature = "native-bridge-metrics")]
fn parse_bridge_profile_enabled(value: &str) -> bool {
    let normalized = value.trim();
    normalized == "1"
        || normalized.eq_ignore_ascii_case("true")
        || normalized.eq_ignore_ascii_case("on")
        || normalized.eq_ignore_ascii_case("yes")
}

#[cfg(feature = "native-bridge-metrics")]
fn bridge_profiling_enabled() -> bool {
    *BRIDGE_PROFILE_ENABLED.get_or_init(|| {
        std::env::var(BRIDGE_PROFILE_ENV)
            .ok()
            .is_some_and(|value| parse_bridge_profile_enabled(&value))
    })
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline]
fn bridge_profiling_enabled() -> bool {
    false
}

#[cfg(feature = "native-bridge-metrics")]
/// Resolve whether projection-key snapshot assertions should run.
fn projection_key_assertions_enabled() -> bool {
    *PROJECTION_KEY_ASSERT_ENABLED.get_or_init(|| {
        std::env::var(PROJECTION_KEY_ASSERT_ENV)
            .ok()
            .is_some_and(|value| parse_bridge_profile_enabled(&value))
    })
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline]
/// Disable projection-key assertions when bridge metrics are compiled out.
fn projection_key_assertions_enabled() -> bool {
    false
}

/// Parse immediate waveform-preview env values.
fn parse_immediate_waveform_preview(value: &str) -> bool {
    let normalized = value.trim();
    normalized == "1"
        || normalized.eq_ignore_ascii_case("true")
        || normalized.eq_ignore_ascii_case("on")
        || normalized.eq_ignore_ascii_case("yes")
}

/// Resolve whether waveform preview actions should apply immediately.
fn immediate_waveform_preview_enabled() -> bool {
    *IMMEDIATE_WAVEFORM_PREVIEW_ENABLED.get_or_init(|| {
        std::env::var(IMMEDIATE_WAVEFORM_PREVIEW_ENV)
            .ok()
            .map_or(IMMEDIATE_WAVEFORM_PREVIEW_DEFAULT, |value| {
                parse_immediate_waveform_preview(&value)
            })
    })
}

#[cfg(feature = "native-bridge-metrics")]
fn saturating_add_duration(counter: &AtomicU64, duration: Duration) {
    let dur_ns = duration.as_nanos().min(u64::MAX as u128) as u64;
    counter.fetch_add(dur_ns, Ordering::Relaxed);
}

#[cfg(feature = "native-bridge-metrics")]
fn ms_from_ns(ns: u64) -> f64 {
    ns as f64 / 1_000_000.0
}

/// Classify UI actions into focused interaction profile groups.
fn classify_action_interaction(action: &NativeUiAction) -> Option<InteractionActionClass> {
    match action {
        NativeUiAction::MoveBrowserFocus { .. } => Some(InteractionActionClass::Wheel),
        NativeUiAction::SetBrowserTab { map: true } | NativeUiAction::FocusMapSample { .. } => {
            Some(InteractionActionClass::MapPanProxy)
        }
        NativeUiAction::SeekWaveform { .. }
        | NativeUiAction::SetWaveformCursor { .. }
        | NativeUiAction::SetWaveformSelectionRange { .. }
        | NativeUiAction::ClearWaveformSelection
        | NativeUiAction::ZoomWaveform { .. }
        | NativeUiAction::ZoomWaveformToSelection
        | NativeUiAction::ZoomWaveformFull => Some(InteractionActionClass::Waveform),
        NativeUiAction::SetVolume { .. } | NativeUiAction::CommitVolumeSetting => {
            Some(InteractionActionClass::Volume)
        }
        _ => None,
    }
}

#[cfg(feature = "native-bridge-metrics")]
fn maybe_log_bridge_profile() {
    let pull_model_count = PULL_MODEL_COUNT.load(Ordering::Relaxed);
    let pull_model_prep = PULL_MODEL_PREP_NS.load(Ordering::Relaxed);
    let pull_model_project = PULL_MODEL_PROJECT_NS.load(Ordering::Relaxed);
    let pull_motion_count = PULL_MOTION_COUNT.load(Ordering::Relaxed);
    let pull_motion_prep = PULL_MOTION_PREP_NS.load(Ordering::Relaxed);
    let pull_motion_project = PULL_MOTION_PROJECT_NS.load(Ordering::Relaxed);
    let action_count = ACTION_COUNT.load(Ordering::Relaxed);
    let action_ns = ACTION_DURATION_NS.load(Ordering::Relaxed);
    let projection_cache_hit_count = PROJECTION_CACHE_HIT_COUNT.load(Ordering::Relaxed);
    let projection_cache_miss_count = PROJECTION_CACHE_MISS_COUNT.load(Ordering::Relaxed);
    let status_segment_hit_count = PROJECTION_STATUS_SEGMENT_HIT_COUNT.load(Ordering::Relaxed);
    let status_segment_miss_count = PROJECTION_STATUS_SEGMENT_MISS_COUNT.load(Ordering::Relaxed);
    let browser_frame_segment_hit_count =
        PROJECTION_BROWSER_FRAME_SEGMENT_HIT_COUNT.load(Ordering::Relaxed);
    let browser_frame_segment_miss_count =
        PROJECTION_BROWSER_FRAME_SEGMENT_MISS_COUNT.load(Ordering::Relaxed);
    let browser_rows_segment_hit_count =
        PROJECTION_BROWSER_ROWS_SEGMENT_HIT_COUNT.load(Ordering::Relaxed);
    let browser_rows_segment_miss_count =
        PROJECTION_BROWSER_ROWS_SEGMENT_MISS_COUNT.load(Ordering::Relaxed);
    let map_segment_hit_count = PROJECTION_MAP_SEGMENT_HIT_COUNT.load(Ordering::Relaxed);
    let map_segment_miss_count = PROJECTION_MAP_SEGMENT_MISS_COUNT.load(Ordering::Relaxed);
    let waveform_segment_hit_count = PROJECTION_WAVEFORM_SEGMENT_HIT_COUNT.load(Ordering::Relaxed);
    let waveform_segment_miss_count =
        PROJECTION_WAVEFORM_SEGMENT_MISS_COUNT.load(Ordering::Relaxed);
    let wheel_count = ACTION_WHEEL_COUNT.load(Ordering::Relaxed);
    let wheel_ns = ACTION_WHEEL_DURATION_NS.load(Ordering::Relaxed);
    let map_proxy_count = ACTION_MAP_PROXY_COUNT.load(Ordering::Relaxed);
    let map_proxy_ns = ACTION_MAP_PROXY_DURATION_NS.load(Ordering::Relaxed);
    let waveform_count = ACTION_WAVEFORM_COUNT.load(Ordering::Relaxed);
    let waveform_ns = ACTION_WAVEFORM_DURATION_NS.load(Ordering::Relaxed);
    let volume_count = ACTION_VOLUME_COUNT.load(Ordering::Relaxed);
    let volume_ns = ACTION_VOLUME_DURATION_NS.load(Ordering::Relaxed);
    let waveform_flush_count = WAVEFORM_FLUSH_COUNT.load(Ordering::Relaxed);
    let waveform_flush_ns = WAVEFORM_FLUSH_DURATION_NS.load(Ordering::Relaxed);
    let waveform_flush_emitted_actions =
        WAVEFORM_FLUSH_EMITTED_ACTIONS_TOTAL.load(Ordering::Relaxed);
    let waveform_image_refresh_apply_count =
        WAVEFORM_IMAGE_REFRESH_APPLY_COUNT.load(Ordering::Relaxed);
    let waveform_image_refresh_skip_count =
        WAVEFORM_IMAGE_REFRESH_SKIP_COUNT.load(Ordering::Relaxed);
    let derived_flush_count = DERIVED_FLUSH_COUNT.load(Ordering::Relaxed);
    let derived_flush_ns = DERIVED_FLUSH_DURATION_NS.load(Ordering::Relaxed);
    let derived_dirty_source_total = DERIVED_DIRTY_SOURCE_TOTAL.load(Ordering::Relaxed);
    let derived_dirty_computed_total = DERIVED_DIRTY_COMPUTED_TOTAL.load(Ordering::Relaxed);
    let frame_count = FRAME_RESULT_COUNT.load(Ordering::Relaxed);
    let frame_anim_count = FRAME_RESULT_ANIMATION_COUNT.load(Ordering::Relaxed);
    let primitive_sum = FRAME_RESULT_PRIMITIVES_TOTAL.load(Ordering::Relaxed);
    let text_run_sum = FRAME_RESULT_TEXT_RUNS_TOTAL.load(Ordering::Relaxed);
    let presented_frame_count = FRAME_RESULT_PRESENTED_COUNT.load(Ordering::Relaxed);
    let missed_present_count = FRAME_RESULT_MISSED_PRESENT_COUNT.load(Ordering::Relaxed);
    let jank_count = FRAME_RESULT_JANK_COUNT.load(Ordering::Relaxed);
    let frame_total_us = FRAME_RESULT_TOTAL_US.load(Ordering::Relaxed);
    let present_total_us = FRAME_RESULT_PRESENT_US_TOTAL.load(Ordering::Relaxed);
    let frame_budget_us = FRAME_RESULT_FRAME_BUDGET_US.load(Ordering::Relaxed);
    let projection_key_assert_count = PROJECTION_KEY_ASSERT_COUNT.load(Ordering::Relaxed);
    let projection_key_assert_stale_count =
        PROJECTION_KEY_ASSERT_STALE_COUNT.load(Ordering::Relaxed);
    let (browser_row_cache_hit_count, browser_row_cache_miss_count) =
        native_shell::browser_row_cache_lookup_counts();
    let pull_model_avg_prep_ms = if pull_model_count == 0 {
        0.0
    } else {
        ms_from_ns(pull_model_prep) / pull_model_count as f64
    };
    let pull_model_avg_project_ms = if pull_model_count == 0 {
        0.0
    } else {
        ms_from_ns(pull_model_project) / pull_model_count as f64
    };
    let pull_motion_avg_prep_ms = if pull_motion_count == 0 {
        0.0
    } else {
        ms_from_ns(pull_motion_prep) / pull_motion_count as f64
    };
    let pull_motion_avg_project_ms = if pull_motion_count == 0 {
        0.0
    } else {
        ms_from_ns(pull_motion_project) / pull_motion_count as f64
    };
    let action_avg_ms = if action_count == 0 {
        0.0
    } else {
        ms_from_ns(action_ns) / action_count as f64
    };
    let wheel_avg_ms = if wheel_count == 0 {
        0.0
    } else {
        ms_from_ns(wheel_ns) / wheel_count as f64
    };
    let map_proxy_avg_ms = if map_proxy_count == 0 {
        0.0
    } else {
        ms_from_ns(map_proxy_ns) / map_proxy_count as f64
    };
    let waveform_avg_ms = if waveform_count == 0 {
        0.0
    } else {
        ms_from_ns(waveform_ns) / waveform_count as f64
    };
    let volume_avg_ms = if volume_count == 0 {
        0.0
    } else {
        ms_from_ns(volume_ns) / volume_count as f64
    };
    let waveform_flush_avg_ms = if waveform_flush_count == 0 {
        0.0
    } else {
        ms_from_ns(waveform_flush_ns) / waveform_flush_count as f64
    };
    let waveform_flush_avg_actions = if waveform_flush_count == 0 {
        0.0
    } else {
        waveform_flush_emitted_actions as f64 / waveform_flush_count as f64
    };
    let derived_flush_avg_ms = if derived_flush_count == 0 {
        0.0
    } else {
        ms_from_ns(derived_flush_ns) / derived_flush_count as f64
    };
    let derived_flush_avg_dirty_sources = if derived_flush_count == 0 {
        0.0
    } else {
        derived_dirty_source_total as f64 / derived_flush_count as f64
    };
    let derived_flush_avg_dirty_computed = if derived_flush_count == 0 {
        0.0
    } else {
        derived_dirty_computed_total as f64 / derived_flush_count as f64
    };
    let avg_primitives_per_frame = if frame_count == 0 {
        0.0
    } else {
        primitive_sum as f64 / frame_count as f64
    };
    let avg_text_runs_per_frame = if frame_count == 0 {
        0.0
    } else {
        text_run_sum as f64 / frame_count as f64
    };
    let frame_total_avg_ms = if frame_count == 0 {
        0.0
    } else {
        frame_total_us as f64 / frame_count as f64 / 1000.0
    };
    let present_avg_ms = if presented_frame_count == 0 {
        0.0
    } else {
        present_total_us as f64 / presented_frame_count as f64 / 1000.0
    };
    let jank_ratio = if frame_count == 0 {
        0.0
    } else {
        jank_count as f64 / frame_count as f64
    };
    let missed_present_ratio = if frame_count == 0 {
        0.0
    } else {
        missed_present_count as f64 / frame_count as f64
    };
    info!(
        pull_model_count,
        pull_motion_count,
        action_count,
        wheel_count,
        map_proxy_count,
        waveform_count,
        volume_count,
        frame_count,
        frame_anim_count,
        "native bridge profiling: pull_model prep_ms={:.3} project_ms={:.3} \
         pull_motion prep_ms={:.3} project_ms={:.3} action_ms={:.3} \
         projection_cache hits={} misses={} \
         segments status(h/m)={}/{} browser_frame(h/m)={}/{} browser_rows(h/m)={}/{} map(h/m)={}/{} waveform(h/m)={}/{} \
         wheel_action_ms={:.3} map_proxy_action_ms={:.3} waveform_action_ms={:.3} volume_action_ms={:.3} \
         waveform_flush_ms={:.3} waveform_flush_avg_actions={:.2} \
         waveform_image_refresh apply={} skip={} \
         derived_flush_ms={:.3} derived_dirty_sources={:.2} derived_dirty_computed={:.2} \
         avg_primitives_per_frame={:.2} avg_text_runs_per_frame={:.2} \
         frame_avg_ms={:.3} present_avg_ms={:.3} frame_budget_us={} \
         browser_row_cache hits={} misses={} \
         projection_key_assert_count={} projection_key_assert_stale_count={} \
         jank_count={} jank_ratio={:.3} missed_present_count={} missed_present_ratio={:.3}",
        pull_model_avg_prep_ms,
        pull_model_avg_project_ms,
        pull_motion_avg_prep_ms,
        pull_motion_avg_project_ms,
        action_avg_ms,
        projection_cache_hit_count,
        projection_cache_miss_count,
        status_segment_hit_count,
        status_segment_miss_count,
        browser_frame_segment_hit_count,
        browser_frame_segment_miss_count,
        browser_rows_segment_hit_count,
        browser_rows_segment_miss_count,
        map_segment_hit_count,
        map_segment_miss_count,
        waveform_segment_hit_count,
        waveform_segment_miss_count,
        wheel_avg_ms,
        map_proxy_avg_ms,
        waveform_avg_ms,
        volume_avg_ms,
        waveform_flush_avg_ms,
        waveform_flush_avg_actions,
        waveform_image_refresh_apply_count,
        waveform_image_refresh_skip_count,
        derived_flush_avg_ms,
        derived_flush_avg_dirty_sources,
        derived_flush_avg_dirty_computed,
        avg_primitives_per_frame,
        avg_text_runs_per_frame,
        frame_total_avg_ms,
        present_avg_ms,
        frame_budget_us,
        browser_row_cache_hit_count,
        browser_row_cache_miss_count,
        projection_key_assert_count,
        projection_key_assert_stale_count,
        jank_count,
        jank_ratio,
        missed_present_count,
        missed_present_ratio
    );
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline]
fn maybe_log_bridge_profile() {}

#[cfg(feature = "native-bridge-metrics")]
fn trace_pull_model_call() -> u64 {
    PULL_MODEL_COUNT.fetch_add(1, Ordering::Relaxed) + 1
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
fn trace_pull_model_call() -> u64 {
    1
}

#[cfg(feature = "native-bridge-metrics")]
fn trace_pull_motion_call() -> u64 {
    PULL_MOTION_COUNT.fetch_add(1, Ordering::Relaxed) + 1
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
fn trace_pull_motion_call() -> u64 {
    1
}

#[cfg(feature = "native-bridge-metrics")]
fn trace_action_call() -> u64 {
    ACTION_COUNT.fetch_add(1, Ordering::Relaxed) + 1
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
fn trace_action_call() -> u64 {
    1
}

#[cfg(feature = "native-bridge-metrics")]
fn trace_frame_result(result: &NativeFrameBuildResult) -> u64 {
    let frame_count = FRAME_RESULT_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    if result.needs_animation {
        FRAME_RESULT_ANIMATION_COUNT.fetch_add(1, Ordering::Relaxed);
    }
    if result.presented {
        FRAME_RESULT_PRESENTED_COUNT.fetch_add(1, Ordering::Relaxed);
    }
    if result.missed_present {
        FRAME_RESULT_MISSED_PRESENT_COUNT.fetch_add(1, Ordering::Relaxed);
    }
    if result.jank {
        FRAME_RESULT_JANK_COUNT.fetch_add(1, Ordering::Relaxed);
    }
    FRAME_RESULT_TOTAL_US.fetch_add(result.frame_total_us as u64, Ordering::Relaxed);
    FRAME_RESULT_PRESENT_US_TOTAL.fetch_add(result.present_us as u64, Ordering::Relaxed);
    FRAME_RESULT_FRAME_BUDGET_US.store(result.frame_budget_us as u64, Ordering::Relaxed);
    FRAME_RESULT_PRIMITIVES_TOTAL.fetch_add(result.primitive_count as u64, Ordering::Relaxed);
    FRAME_RESULT_TEXT_RUNS_TOTAL.fetch_add(result.text_run_count as u64, Ordering::Relaxed);
    frame_count
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
fn trace_frame_result(_result: &NativeFrameBuildResult) -> u64 {
    1
}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
fn trace_pull_model_preparation(duration: Duration) {
    saturating_add_duration(&PULL_MODEL_PREP_NS, duration);
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
fn trace_pull_model_preparation(_duration: Duration) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
fn trace_pull_model_projection(duration: Duration) {
    saturating_add_duration(&PULL_MODEL_PROJECT_NS, duration);
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
fn trace_pull_model_projection(_duration: Duration) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
fn trace_pull_motion_preparation(duration: Duration) {
    saturating_add_duration(&PULL_MOTION_PREP_NS, duration);
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
fn trace_pull_motion_preparation(_duration: Duration) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
fn trace_pull_motion_projection(duration: Duration) {
    saturating_add_duration(&PULL_MOTION_PROJECT_NS, duration);
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
fn trace_pull_motion_projection(_duration: Duration) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
fn trace_action_duration(duration: Duration) {
    saturating_add_duration(&ACTION_DURATION_NS, duration);
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
fn trace_action_duration(_duration: Duration) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
/// Track whether an app-model projection cache lookup hit or missed.
fn trace_projection_cache_lookup(hit: bool) {
    if hit {
        PROJECTION_CACHE_HIT_COUNT.fetch_add(1, Ordering::Relaxed);
    } else {
        PROJECTION_CACHE_MISS_COUNT.fetch_add(1, Ordering::Relaxed);
    }
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
/// No-op projection-cache hit/miss tracer for non-profiling builds.
fn trace_projection_cache_lookup(_hit: bool) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
/// Track segment-level projection-cache hit/miss decisions.
fn trace_projection_segment_lookup(segment: ProjectionSegment, hit: bool) {
    match (segment, hit) {
        (ProjectionSegment::StatusBar, true) => {
            PROJECTION_STATUS_SEGMENT_HIT_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        (ProjectionSegment::StatusBar, false) => {
            PROJECTION_STATUS_SEGMENT_MISS_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        (ProjectionSegment::BrowserFrame, true) => {
            PROJECTION_BROWSER_FRAME_SEGMENT_HIT_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        (ProjectionSegment::BrowserFrame, false) => {
            PROJECTION_BROWSER_FRAME_SEGMENT_MISS_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        (ProjectionSegment::BrowserRowsWindow, true) => {
            PROJECTION_BROWSER_ROWS_SEGMENT_HIT_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        (ProjectionSegment::BrowserRowsWindow, false) => {
            PROJECTION_BROWSER_ROWS_SEGMENT_MISS_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        (ProjectionSegment::MapPanel, true) => {
            PROJECTION_MAP_SEGMENT_HIT_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        (ProjectionSegment::MapPanel, false) => {
            PROJECTION_MAP_SEGMENT_MISS_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        (ProjectionSegment::WaveformOverlay, true) => {
            PROJECTION_WAVEFORM_SEGMENT_HIT_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        (ProjectionSegment::WaveformOverlay, false) => {
            PROJECTION_WAVEFORM_SEGMENT_MISS_COUNT.fetch_add(1, Ordering::Relaxed);
        }
    }
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
/// No-op segment-level projection-cache tracer for non-profiling builds.
fn trace_projection_segment_lookup(_segment: ProjectionSegment, _hit: bool) {}

#[inline(always)]
/// Track and locally record one segment-level projection-cache lookup decision.
fn trace_and_record_projection_segment_lookup(
    counters: &mut ProjectionSegmentLookupCounts,
    segment: ProjectionSegment,
    hit: bool,
) {
    trace_projection_segment_lookup(segment, hit);
    counters.record_lookup(segment, hit);
}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
/// Track classified interaction action timings for bridge profiling logs.
fn trace_action_interaction(kind: InteractionActionClass, duration: Duration) {
    match kind {
        InteractionActionClass::Wheel => {
            ACTION_WHEEL_COUNT.fetch_add(1, Ordering::Relaxed);
            saturating_add_duration(&ACTION_WHEEL_DURATION_NS, duration);
        }
        InteractionActionClass::MapPanProxy => {
            ACTION_MAP_PROXY_COUNT.fetch_add(1, Ordering::Relaxed);
            saturating_add_duration(&ACTION_MAP_PROXY_DURATION_NS, duration);
        }
        InteractionActionClass::Waveform => {
            ACTION_WAVEFORM_COUNT.fetch_add(1, Ordering::Relaxed);
            saturating_add_duration(&ACTION_WAVEFORM_DURATION_NS, duration);
        }
        InteractionActionClass::Volume => {
            ACTION_VOLUME_COUNT.fetch_add(1, Ordering::Relaxed);
            saturating_add_duration(&ACTION_VOLUME_DURATION_NS, duration);
        }
    }
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
/// No-op classified interaction recorder for non-profiling builds.
fn trace_action_interaction(_kind: InteractionActionClass, _duration: Duration) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
/// Track end-to-end duration and emission count for queued waveform-action flushes.
fn trace_waveform_flush(duration: Duration, emitted_actions: u64) {
    WAVEFORM_FLUSH_COUNT.fetch_add(1, Ordering::Relaxed);
    saturating_add_duration(&WAVEFORM_FLUSH_DURATION_NS, duration);
    WAVEFORM_FLUSH_EMITTED_ACTIONS_TOTAL.fetch_add(emitted_actions, Ordering::Relaxed);
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
/// No-op waveform flush tracer for non-profiling builds.
fn trace_waveform_flush(_duration: Duration, _emitted_actions: u64) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
/// Track whether waveform image refresh work ran or was skipped as overlay-only.
fn trace_waveform_image_refresh(applied: bool) {
    if applied {
        WAVEFORM_IMAGE_REFRESH_APPLY_COUNT.fetch_add(1, Ordering::Relaxed);
    } else {
        WAVEFORM_IMAGE_REFRESH_SKIP_COUNT.fetch_add(1, Ordering::Relaxed);
    }
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
/// No-op waveform image refresh tracer for non-profiling builds.
fn trace_waveform_image_refresh(_applied: bool) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
/// Track derived-graph flush timing and dirty-node counts.
fn trace_derived_flush(duration: Duration, dirty_source_count: usize, dirty_derived_count: usize) {
    DERIVED_FLUSH_COUNT.fetch_add(1, Ordering::Relaxed);
    DERIVED_DIRTY_SOURCE_TOTAL.fetch_add(dirty_source_count as u64, Ordering::Relaxed);
    DERIVED_DIRTY_COMPUTED_TOTAL.fetch_add(dirty_derived_count as u64, Ordering::Relaxed);
    saturating_add_duration(&DERIVED_FLUSH_DURATION_NS, duration);
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
/// No-op derived-graph flush tracer for non-profiling builds.
fn trace_derived_flush(
    _duration: Duration,
    _dirty_source_count: usize,
    _dirty_derived_count: usize,
) {
}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
/// Track projection-key snapshot validation checks and stale detections.
fn trace_projection_key_assertion(stale: bool) {
    PROJECTION_KEY_ASSERT_COUNT.fetch_add(1, Ordering::Relaxed);
    if stale {
        PROJECTION_KEY_ASSERT_STALE_COUNT.fetch_add(1, Ordering::Relaxed);
    }
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
/// No-op projection-key snapshot assertion tracer for non-profiling builds.
fn trace_projection_key_assertion(_stale: bool) {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MapQueryBoundsKey {
    min_x_bits: u32,
    max_x_bits: u32,
    min_y_bits: u32,
    max_y_bits: u32,
}

/// Hash an arbitrary projection field into a compact cache-key scalar.
fn hash_projection_field<T: Hash + ?Sized>(value: &T) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

/// Hash an optional string field into an optional cache-key scalar.
fn hash_optional_string(value: Option<&str>) -> Option<u64> {
    value.map(hash_projection_field)
}

impl MapQueryBoundsKey {
    fn from_bounds(bounds: crate::app_core::state::MapQueryBounds) -> Self {
        Self {
            min_x_bits: bounds.min_x.to_bits(),
            max_x_bits: bounds.max_x.to_bits(),
            min_y_bits: bounds.min_y.to_bits(),
            max_y_bits: bounds.max_y.to_bits(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct NativeProjectionCacheKey {
    status_text_hash: u64,
    status_tone: u8,
    sources_selected: Option<usize>,
    sources_len: usize,
    folder_rows_len: usize,
    folder_focused: Option<usize>,
    folder_search_query_hash: u64,
    browser_visible_len: usize,
    browser_visible_rows_revision: u64,
    browser_selected_visible: Option<usize>,
    browser_anchor_visible: Option<usize>,
    browser_selected_paths_len: usize,
    browser_selected_paths_revision: u64,
    browser_search_query_hash: u64,
    browser_filter: u8,
    browser_sort: u8,
    browser_tab: u8,
    progress_visible: bool,
    progress_completed: usize,
    progress_total: usize,
    prompt_active: bool,
    drag_active: bool,
    waveform_signature: Option<u64>,
    waveform_cursor_milli: Option<u16>,
    waveform_playhead_milli: Option<u16>,
    waveform_selection_start_milli: Option<u16>,
    waveform_selection_end_milli: Option<u16>,
    waveform_view_start_milli: u16,
    waveform_view_end_milli: u16,
    waveform_loop_enabled: bool,
    waveform_bpm_bits: Option<u32>,
    map_open: bool,
    map_zoom_bits: u32,
    map_pan_x_bits: u32,
    map_pan_y_bits: u32,
    map_selected_sample_id_hash: Option<u64>,
    map_hovered_sample_id_hash: Option<u64>,
    map_umap_version_hash: u64,
    map_bounds_source_id_hash: Option<u64>,
    map_bounds_umap_version_hash: Option<u64>,
    map_points_source_id_hash: Option<u64>,
    map_points_umap_version_hash: Option<u64>,
    map_last_query: Option<MapQueryBoundsKey>,
    map_points_revision: u64,
    update_status: u8,
    update_available_tag_hash: Option<u64>,
    update_available_url_hash: Option<u64>,
    update_last_error_hash: Option<u64>,
    loaded_wav_hash: Option<u64>,
    volume_milli: u16,
    transport_running: bool,
}

/// Status-bar projection key scoped to status and footer-affecting state.
#[derive(Clone, Debug, PartialEq, Eq)]
struct StatusProjectionCacheKey {
    status_text_hash: u64,
    status_tone: u8,
    browser_visible_len: usize,
    browser_selected_paths_len: usize,
    browser_anchor_visible: Option<usize>,
    browser_search_query_hash: u64,
    browser_search_busy: bool,
    selected_column: usize,
}

/// Browser metadata/chrome projection key scoped to non-row browser state.
#[derive(Clone, Debug, PartialEq, Eq)]
struct BrowserFrameProjectionCacheKey {
    browser_visible_len: usize,
    browser_selected_visible: Option<usize>,
    browser_anchor_visible: Option<usize>,
    browser_selected_paths_len: usize,
    browser_search_query_hash: u64,
    browser_search_busy: bool,
    browser_sort: u8,
    browser_tab: u8,
    browser_similarity_follow_loaded: bool,
    loaded_wav_hash: Option<u64>,
}

/// Browser rows projection key scoped to windowed row content.
#[derive(Clone, Debug, PartialEq, Eq)]
struct BrowserRowsProjectionCacheKey {
    browser_visible_rows_revision: u64,
    browser_visible_len: usize,
    browser_selected_visible: Option<usize>,
    browser_anchor_visible: Option<usize>,
    browser_selected_paths_len: usize,
    browser_selected_paths_revision: u64,
    browser_tab: u8,
}

/// Map-panel projection key scoped to similarity-map-affecting state.
#[derive(Clone, Debug, PartialEq, Eq)]
struct MapProjectionCacheKey {
    map_open: bool,
    map_zoom_bits: u32,
    map_pan_x_bits: u32,
    map_pan_y_bits: u32,
    map_selected_sample_id_hash: Option<u64>,
    map_hovered_sample_id_hash: Option<u64>,
    map_umap_version_hash: u64,
    map_bounds_source_id_hash: Option<u64>,
    map_bounds_umap_version_hash: Option<u64>,
    map_points_source_id_hash: Option<u64>,
    map_points_umap_version_hash: Option<u64>,
    map_last_query: Option<MapQueryBoundsKey>,
    map_points_revision: u64,
    browser_tab: u8,
}

/// Waveform projection key scoped to waveform panel/chrome state.
#[derive(Clone, Debug, PartialEq, Eq)]
struct WaveformProjectionCacheKey {
    waveform_signature: Option<u64>,
    waveform_cursor_milli: Option<u16>,
    waveform_playhead_milli: Option<u16>,
    waveform_selection_start_milli: Option<u16>,
    waveform_selection_end_milli: Option<u16>,
    waveform_view_start_milli: u16,
    waveform_view_end_milli: u16,
    waveform_loop_enabled: bool,
    waveform_bpm_bits: Option<u32>,
    loaded_wav_hash: Option<u64>,
    transport_running: bool,
}

/// Projection key for static fields that are not part of explicit segment buckets.
#[derive(Clone, Debug, PartialEq, Eq)]
struct NonSegmentStaticProjectionCacheKey {
    sources_selected: Option<usize>,
    sources_len: usize,
    folder_rows_len: usize,
    folder_focused: Option<usize>,
    folder_search_query_hash: u64,
    update_status: u8,
    update_available_tag_hash: Option<u64>,
    update_available_url_hash: Option<u64>,
    update_last_error_hash: Option<u64>,
    volume_milli: u16,
    transport_running: bool,
    trash_count: usize,
    neutral_count: usize,
    keep_count: usize,
}

#[derive(Clone, Debug, Default)]
struct NativeProjectionCache {
    app_key: Option<NativeProjectionCacheKey>,
    app_model: Option<Arc<NativeAppModel>>,
    status_key: Option<StatusProjectionCacheKey>,
    browser_frame_key: Option<BrowserFrameProjectionCacheKey>,
    browser_rows_key: Option<BrowserRowsProjectionCacheKey>,
    map_key: Option<MapProjectionCacheKey>,
    waveform_key: Option<WaveformProjectionCacheKey>,
    non_segment_static_key: Option<NonSegmentStaticProjectionCacheKey>,
    segment_lookup_counts: ProjectionSegmentLookupCounts,
}

impl NativeProjectionCache {
    /// Record one projection segment lookup decision.
    fn record_segment_lookup(&mut self, segment: ProjectionSegment, hit: bool) {
        trace_and_record_projection_segment_lookup(&mut self.segment_lookup_counts, segment, hit);
    }

    /// Return and clear segment lookup counters accumulated so far.
    fn take_segment_lookup_counts(&mut self) -> ProjectionSegmentLookupCounts {
        std::mem::take(&mut self.segment_lookup_counts)
    }

    /// Copy browser metadata fields while preserving any retained row vector.
    fn apply_browser_frame(
        model: &mut NativeAppModel,
        frame: crate::app_core::actions::NativeBrowserPanelModel,
    ) {
        model.browser.visible_count = frame.visible_count;
        model.browser.selected_visible_row = frame.selected_visible_row;
        model.browser.selected_path_count = frame.selected_path_count;
        model.browser.search_query = frame.search_query;
        model.browser.search_placeholder = frame.search_placeholder;
        model.browser.busy = frame.busy;
        model.browser.sort_label = frame.sort_label;
        model.browser.active_tab_label = frame.active_tab_label;
        model.browser.focused_sample_label = frame.focused_sample_label;
        model.browser.anchor_visible_row = frame.anchor_visible_row;
    }

    /// Refresh non-segmented app-model fields from current controller state.
    fn refresh_non_segment_fields(model: &mut NativeAppModel, controller: &mut AppController) {
        let selected_column = native_shell::selected_column_index(&controller.ui);
        model.selected_column = selected_column;
        model.transport_running = controller.is_playing();
        model.volume = controller.ui.volume.clamp(0.0, 1.0);
        model.sources = native_shell::project_sources_model(&controller.ui);
        model.sources_label = format!("Sources ({})", model.sources.rows.len());
        model.columns = [
            crate::app_core::actions::NativeColumnModel::new(
                "Trash",
                controller.ui.browser.trash.len(),
            ),
            crate::app_core::actions::NativeColumnModel::new(
                "Samples",
                controller.ui.browser.neutral.len(),
            ),
            crate::app_core::actions::NativeColumnModel::new(
                "Keep",
                controller.ui.browser.keep.len(),
            ),
        ];
        model.progress_overlay = native_shell::project_progress_overlay_model(&controller.ui);
        model.confirm_prompt = native_shell::project_confirm_prompt_model(&controller.ui);
        model.drag_overlay = native_shell::project_drag_overlay_model(&controller.ui);
        model.update = native_shell::project_update_model(&controller.ui);
    }

    fn resolve_or_project(
        &mut self,
        controller: &mut AppController,
        project: impl FnOnce(&mut AppController) -> NativeAppModel,
    ) -> (Arc<NativeAppModel>, NativeDirtySegments) {
        let key = build_projection_cache_key(controller);
        self.resolve_or_project_with_key(controller, &key, project)
    }

    /// Resolve retained projection output using a caller-provided cache key.
    fn resolve_or_project_with_key(
        &mut self,
        controller: &mut AppController,
        key: &NativeProjectionCacheKey,
        project: impl FnOnce(&mut AppController) -> NativeAppModel,
    ) -> (Arc<NativeAppModel>, NativeDirtySegments) {
        if self.app_key.as_ref() == Some(key)
            && let Some(model) = self.app_model.as_ref().map(Arc::clone)
        {
            trace_projection_cache_lookup(true);
            self.record_segment_lookup(ProjectionSegment::StatusBar, true);
            self.record_segment_lookup(ProjectionSegment::BrowserFrame, true);
            self.record_segment_lookup(ProjectionSegment::BrowserRowsWindow, true);
            self.record_segment_lookup(ProjectionSegment::MapPanel, true);
            self.record_segment_lookup(ProjectionSegment::WaveformOverlay, true);
            return (model, NativeDirtySegments::empty());
        }
        trace_projection_cache_lookup(false);
        let selected_column = native_shell::selected_column_index(&controller.ui);
        let status_key = build_status_projection_key(controller, selected_column);
        let browser_frame_key = build_browser_frame_projection_key(controller);
        let browser_rows_key = build_browser_rows_projection_key(controller);
        let map_key = build_map_projection_key(controller);
        let waveform_key = build_waveform_projection_key(controller);
        let non_segment_static_key = build_non_segment_static_projection_key(controller);
        let init_full_projection =
            |cache: &mut Self, controller: &mut AppController| -> Arc<NativeAppModel> {
                cache.record_segment_lookup(ProjectionSegment::StatusBar, false);
                cache.record_segment_lookup(ProjectionSegment::BrowserFrame, false);
                cache.record_segment_lookup(ProjectionSegment::BrowserRowsWindow, false);
                cache.record_segment_lookup(ProjectionSegment::MapPanel, false);
                cache.record_segment_lookup(ProjectionSegment::WaveformOverlay, false);
                Arc::new(project(controller))
            };
        let mut model = if let Some(existing) = self.app_model.take() {
            if let Some(model) = Arc::into_inner(existing) {
                model
            } else {
                let model = init_full_projection(self, controller);
                self.app_key = Some(key.clone());
                self.app_model = Some(Arc::clone(&model));
                self.status_key = Some(status_key);
                self.browser_frame_key = Some(browser_frame_key);
                self.browser_rows_key = Some(browser_rows_key);
                self.map_key = Some(map_key);
                self.waveform_key = Some(waveform_key);
                self.non_segment_static_key = Some(non_segment_static_key);
                return (model, NativeDirtySegments::all());
            }
        } else {
            let model = init_full_projection(self, controller);
            self.app_key = Some(key.clone());
            self.app_model = Some(Arc::clone(&model));
            self.status_key = Some(status_key);
            self.browser_frame_key = Some(browser_frame_key);
            self.browser_rows_key = Some(browser_rows_key);
            self.map_key = Some(map_key);
            self.waveform_key = Some(waveform_key);
            self.non_segment_static_key = Some(non_segment_static_key);
            return (model, NativeDirtySegments::all());
        };

        let mut dirty_segments = NativeDirtySegments::empty();

        if self.status_key.as_ref() == Some(&status_key) {
            self.record_segment_lookup(ProjectionSegment::StatusBar, true);
        } else {
            self.record_segment_lookup(ProjectionSegment::StatusBar, false);
            model.status = native_shell::project_status_model(controller, selected_column);
            model.status_text = controller.ui.status.text.clone();
            self.status_key = Some(status_key);
            dirty_segments.insert(NativeDirtySegments::STATUS_BAR);
        }

        let browser_frame_changed = self.browser_frame_key.as_ref() != Some(&browser_frame_key);
        if browser_frame_changed {
            self.record_segment_lookup(ProjectionSegment::BrowserFrame, false);
            let frame = native_shell::project_browser_panel_frame_model(controller);
            Self::apply_browser_frame(&mut model, frame);
            model.browser_chrome = native_shell::project_browser_chrome_model(
                &controller.ui,
                model.browser.visible_count,
            );
            model.browser_actions = native_shell::project_browser_actions_model(&controller.ui);
            self.browser_frame_key = Some(browser_frame_key);
            dirty_segments.insert(NativeDirtySegments::BROWSER_FRAME);
        } else {
            self.record_segment_lookup(ProjectionSegment::BrowserFrame, true);
        }

        let browser_rows_changed = self.browser_rows_key.as_ref() != Some(&browser_rows_key);
        if browser_frame_changed || browser_rows_changed {
            self.record_segment_lookup(ProjectionSegment::BrowserRowsWindow, false);
            let mut rows = std::mem::take(&mut model.browser.rows);
            native_shell::project_browser_rows_model_into(
                controller,
                model.browser.visible_count,
                model.browser.selected_visible_row,
                model.browser.anchor_visible_row,
                &mut rows,
            );
            model.browser.rows = rows;
            self.browser_rows_key = Some(browser_rows_key);
            dirty_segments.insert(NativeDirtySegments::BROWSER_ROWS_WINDOW);
        } else {
            self.record_segment_lookup(ProjectionSegment::BrowserRowsWindow, true);
        }

        if self.map_key.as_ref() == Some(&map_key) {
            self.record_segment_lookup(ProjectionSegment::MapPanel, true);
        } else {
            self.record_segment_lookup(ProjectionSegment::MapPanel, false);
            model.map = native_shell::project_map_model(controller);
            self.map_key = Some(map_key);
            dirty_segments.insert(NativeDirtySegments::MAP_PANEL);
        }

        if self.waveform_key.as_ref() == Some(&waveform_key) {
            self.record_segment_lookup(ProjectionSegment::WaveformOverlay, true);
        } else {
            self.record_segment_lookup(ProjectionSegment::WaveformOverlay, false);
            model.waveform = native_shell::project_waveform_model(controller);
            model.waveform_chrome = native_shell::project_waveform_chrome_model(&controller.ui);
            self.waveform_key = Some(waveform_key);
            dirty_segments.insert(NativeDirtySegments::WAVEFORM_OVERLAY);
        }

        let non_segment_static_changed =
            self.non_segment_static_key.as_ref() != Some(&non_segment_static_key);
        if non_segment_static_changed {
            dirty_segments.insert(NativeDirtySegments::GLOBAL_STATIC);
        }
        self.non_segment_static_key = Some(non_segment_static_key);

        Self::refresh_non_segment_fields(&mut model, controller);
        self.app_key = Some(key.clone());
        let model = Arc::new(model);
        self.app_model = Some(Arc::clone(&model));
        (model, dirty_segments)
    }

    #[cfg(test)]
    /// Fully clear retained projection cache state.
    fn invalidate(&mut self) {
        self.app_key = None;
        self.app_model = None;
        self.status_key = None;
        self.browser_frame_key = None;
        self.browser_rows_key = None;
        self.map_key = None;
        self.waveform_key = None;
        self.non_segment_static_key = None;
    }

    /// Invalidate only the global key so the next pull runs segment refresh.
    fn invalidate_key_only(&mut self) {
        self.app_key = None;
    }
}

fn build_projection_cache_key(controller: &AppController) -> NativeProjectionCacheKey {
    use crate::app_core::state::{
        MapQueryBounds, SampleBrowserSort, SampleBrowserTab, StatusTone, TriageFlagFilter,
        UpdateStatus,
    };
    let map_last_query = controller
        .ui
        .map
        .last_query
        .map(|bounds: MapQueryBounds| MapQueryBoundsKey::from_bounds(bounds));
    let waveform_cursor_milli = controller
        .ui
        .waveform
        .cursor
        .map(|value| (value.clamp(0.0, 1.0) * 1000.0).round() as u16);
    let waveform_playhead_milli = controller.ui.waveform.playhead.visible.then_some(
        (controller.ui.waveform.playhead.position.clamp(0.0, 1.0) * 1000.0).round() as u16,
    );
    let (waveform_selection_start_milli, waveform_selection_end_milli) = controller
        .ui
        .waveform
        .selection
        .map(|selection| {
            let start = (selection.start().clamp(0.0, 1.0) * 1000.0).round() as u16;
            let end = (selection.end().clamp(0.0, 1.0) * 1000.0).round() as u16;
            (Some(start.min(end)), Some(start.max(end)))
        })
        .unwrap_or((None, None));
    NativeProjectionCacheKey {
        status_text_hash: hash_projection_field(&controller.ui.status.text),
        status_tone: match controller.ui.status.status_tone {
            StatusTone::Idle => 0,
            StatusTone::Busy => 1,
            StatusTone::Info => 2,
            StatusTone::Warning => 3,
            StatusTone::Error => 4,
        },
        sources_selected: controller.ui.sources.selected,
        sources_len: controller.ui.sources.rows.len(),
        folder_rows_len: controller.ui.sources.folders.rows.len(),
        folder_focused: controller.ui.sources.folders.focused,
        folder_search_query_hash: hash_projection_field(
            &controller.ui.sources.folders.search_query,
        ),
        browser_visible_len: controller.ui.browser.visible.len(),
        browser_visible_rows_revision: controller.ui.browser.visible_rows_revision,
        browser_selected_visible: controller.ui.browser.selected_visible,
        browser_anchor_visible: controller.ui.browser.selection_anchor_visible,
        browser_selected_paths_len: controller.ui.browser.selected_paths.len(),
        browser_selected_paths_revision: controller.ui.browser.selected_paths_revision,
        browser_search_query_hash: hash_projection_field(&controller.ui.browser.search_query),
        browser_filter: match controller.ui.browser.filter {
            TriageFlagFilter::All => 0,
            TriageFlagFilter::Keep => 1,
            TriageFlagFilter::Trash => 2,
            TriageFlagFilter::Untagged => 3,
        },
        browser_sort: match controller.ui.browser.sort {
            SampleBrowserSort::ListOrder => 0,
            SampleBrowserSort::Similarity => 1,
            SampleBrowserSort::PlaybackAgeAsc => 2,
            SampleBrowserSort::PlaybackAgeDesc => 3,
        },
        browser_tab: match controller.ui.browser.active_tab {
            SampleBrowserTab::List => 0,
            SampleBrowserTab::Map => 1,
        },
        progress_visible: controller.ui.progress.visible,
        progress_completed: controller.ui.progress.completed,
        progress_total: controller.ui.progress.total,
        prompt_active: controller.ui.browser.pending_action.is_some()
            || controller.ui.sources.folders.pending_action.is_some()
            || controller.ui.sources.folders.new_folder.is_some()
            || controller.ui.waveform.pending_destructive.is_some(),
        drag_active: controller.ui.drag.payload.is_some(),
        waveform_signature: controller.ui.waveform.waveform_image_signature,
        waveform_cursor_milli,
        waveform_playhead_milli,
        waveform_selection_start_milli,
        waveform_selection_end_milli,
        waveform_view_start_milli: (controller.ui.waveform.view.start.clamp(0.0, 1.0) * 1000.0)
            .round() as u16,
        waveform_view_end_milli: (controller.ui.waveform.view.end.clamp(0.0, 1.0) * 1000.0).round()
            as u16,
        waveform_loop_enabled: controller.ui.waveform.loop_enabled,
        waveform_bpm_bits: controller.ui.waveform.bpm_value.map(f32::to_bits),
        map_open: controller.ui.map.open,
        map_zoom_bits: controller.ui.map.zoom.to_bits(),
        map_pan_x_bits: controller.ui.map.pan.x.to_bits(),
        map_pan_y_bits: controller.ui.map.pan.y.to_bits(),
        map_selected_sample_id_hash: hash_optional_string(
            controller.ui.map.selected_sample_id.as_deref(),
        ),
        map_hovered_sample_id_hash: hash_optional_string(
            controller.ui.map.hovered_sample_id.as_deref(),
        ),
        map_umap_version_hash: hash_projection_field(&controller.ui.map.umap_version),
        map_bounds_source_id_hash: hash_optional_string(
            controller.ui.map.cached_bounds_source_id.as_deref(),
        ),
        map_bounds_umap_version_hash: hash_optional_string(
            controller.ui.map.cached_bounds_umap_version.as_deref(),
        ),
        map_points_source_id_hash: hash_optional_string(
            controller.ui.map.cached_points_source_id.as_deref(),
        ),
        map_points_umap_version_hash: hash_optional_string(
            controller.ui.map.cached_points_umap_version.as_deref(),
        ),
        map_last_query,
        map_points_revision: controller.ui.map.cached_points_revision,
        update_status: match controller.ui.update.status {
            UpdateStatus::Idle => 0,
            UpdateStatus::Checking => 1,
            UpdateStatus::UpdateAvailable => 2,
            UpdateStatus::Error => 3,
        },
        update_available_tag_hash: hash_optional_string(
            controller.ui.update.available_tag.as_deref(),
        ),
        update_available_url_hash: hash_optional_string(
            controller.ui.update.available_url.as_deref(),
        ),
        update_last_error_hash: hash_optional_string(controller.ui.update.last_error.as_deref()),
        loaded_wav_hash: controller
            .ui
            .loaded_wav
            .as_ref()
            .map(|path| hash_projection_field(path.as_os_str())),
        volume_milli: (controller.ui.volume.clamp(0.0, 1.0) * 1000.0).round() as u16,
        transport_running: controller.is_playing(),
    }
}

/// Build a status-bar projection key from the current controller snapshot.
fn build_status_projection_key(
    controller: &AppController,
    selected_column: usize,
) -> StatusProjectionCacheKey {
    use crate::app_core::state::StatusTone;
    StatusProjectionCacheKey {
        status_text_hash: hash_projection_field(&controller.ui.status.text),
        status_tone: match controller.ui.status.status_tone {
            StatusTone::Idle => 0,
            StatusTone::Busy => 1,
            StatusTone::Info => 2,
            StatusTone::Warning => 3,
            StatusTone::Error => 4,
        },
        browser_visible_len: controller.ui.browser.visible.len(),
        browser_selected_paths_len: controller.ui.browser.selected_paths.len(),
        browser_anchor_visible: controller.ui.browser.selection_anchor_visible,
        browser_search_query_hash: hash_projection_field(&controller.ui.browser.search_query),
        browser_search_busy: controller.ui.browser.search_busy,
        selected_column,
    }
}

/// Build a browser-frame projection key from the current controller snapshot.
fn build_browser_frame_projection_key(
    controller: &AppController,
) -> BrowserFrameProjectionCacheKey {
    use crate::app_core::state::{SampleBrowserSort, SampleBrowserTab};
    BrowserFrameProjectionCacheKey {
        browser_visible_len: controller.ui.browser.visible.len(),
        browser_selected_visible: controller.ui.browser.selected_visible,
        browser_anchor_visible: controller.ui.browser.selection_anchor_visible,
        browser_selected_paths_len: controller.ui.browser.selected_paths.len(),
        browser_search_query_hash: hash_projection_field(&controller.ui.browser.search_query),
        browser_search_busy: controller.ui.browser.search_busy,
        browser_sort: match controller.ui.browser.sort {
            SampleBrowserSort::ListOrder => 0,
            SampleBrowserSort::Similarity => 1,
            SampleBrowserSort::PlaybackAgeAsc => 2,
            SampleBrowserSort::PlaybackAgeDesc => 3,
        },
        browser_tab: match controller.ui.browser.active_tab {
            SampleBrowserTab::List => 0,
            SampleBrowserTab::Map => 1,
        },
        browser_similarity_follow_loaded: controller.ui.browser.similarity_sort_follow_loaded,
        loaded_wav_hash: controller
            .ui
            .loaded_wav
            .as_ref()
            .map(|path| hash_projection_field(path.as_os_str())),
    }
}

/// Build a browser-rows projection key from the current controller snapshot.
fn build_browser_rows_projection_key(controller: &AppController) -> BrowserRowsProjectionCacheKey {
    use crate::app_core::state::SampleBrowserTab;
    BrowserRowsProjectionCacheKey {
        browser_visible_rows_revision: controller.ui.browser.visible_rows_revision,
        browser_visible_len: controller.ui.browser.visible.len(),
        browser_selected_visible: controller.ui.browser.selected_visible,
        browser_anchor_visible: controller.ui.browser.selection_anchor_visible,
        browser_selected_paths_len: controller.ui.browser.selected_paths.len(),
        browser_selected_paths_revision: controller.ui.browser.selected_paths_revision,
        browser_tab: match controller.ui.browser.active_tab {
            SampleBrowserTab::List => 0,
            SampleBrowserTab::Map => 1,
        },
    }
}

/// Build a map-panel projection key from the current controller snapshot.
fn build_map_projection_key(controller: &AppController) -> MapProjectionCacheKey {
    use crate::app_core::state::{MapQueryBounds, SampleBrowserTab};
    MapProjectionCacheKey {
        map_open: controller.ui.map.open,
        map_zoom_bits: controller.ui.map.zoom.to_bits(),
        map_pan_x_bits: controller.ui.map.pan.x.to_bits(),
        map_pan_y_bits: controller.ui.map.pan.y.to_bits(),
        map_selected_sample_id_hash: hash_optional_string(
            controller.ui.map.selected_sample_id.as_deref(),
        ),
        map_hovered_sample_id_hash: hash_optional_string(
            controller.ui.map.hovered_sample_id.as_deref(),
        ),
        map_umap_version_hash: hash_projection_field(&controller.ui.map.umap_version),
        map_bounds_source_id_hash: hash_optional_string(
            controller.ui.map.cached_bounds_source_id.as_deref(),
        ),
        map_bounds_umap_version_hash: hash_optional_string(
            controller.ui.map.cached_bounds_umap_version.as_deref(),
        ),
        map_points_source_id_hash: hash_optional_string(
            controller.ui.map.cached_points_source_id.as_deref(),
        ),
        map_points_umap_version_hash: hash_optional_string(
            controller.ui.map.cached_points_umap_version.as_deref(),
        ),
        map_last_query: controller
            .ui
            .map
            .last_query
            .map(|bounds: MapQueryBounds| MapQueryBoundsKey::from_bounds(bounds)),
        map_points_revision: controller.ui.map.cached_points_revision,
        browser_tab: match controller.ui.browser.active_tab {
            SampleBrowserTab::List => 0,
            SampleBrowserTab::Map => 1,
        },
    }
}

/// Build a waveform projection key from the current controller snapshot.
fn build_waveform_projection_key(controller: &AppController) -> WaveformProjectionCacheKey {
    let waveform_cursor_milli = controller
        .ui
        .waveform
        .cursor
        .map(|value| (value.clamp(0.0, 1.0) * 1000.0).round() as u16);
    let waveform_playhead_milli = controller.ui.waveform.playhead.visible.then_some(
        (controller.ui.waveform.playhead.position.clamp(0.0, 1.0) * 1000.0).round() as u16,
    );
    let (waveform_selection_start_milli, waveform_selection_end_milli) = controller
        .ui
        .waveform
        .selection
        .map(|selection| {
            let start = (selection.start().clamp(0.0, 1.0) * 1000.0).round() as u16;
            let end = (selection.end().clamp(0.0, 1.0) * 1000.0).round() as u16;
            (Some(start.min(end)), Some(start.max(end)))
        })
        .unwrap_or((None, None));
    WaveformProjectionCacheKey {
        waveform_signature: controller.ui.waveform.waveform_image_signature,
        waveform_cursor_milli,
        waveform_playhead_milli,
        waveform_selection_start_milli,
        waveform_selection_end_milli,
        waveform_view_start_milli: (controller.ui.waveform.view.start.clamp(0.0, 1.0) * 1000.0)
            .round() as u16,
        waveform_view_end_milli: (controller.ui.waveform.view.end.clamp(0.0, 1.0) * 1000.0).round()
            as u16,
        waveform_loop_enabled: controller.ui.waveform.loop_enabled,
        waveform_bpm_bits: controller.ui.waveform.bpm_value.map(f32::to_bits),
        loaded_wav_hash: controller
            .ui
            .loaded_wav
            .as_ref()
            .map(|path| hash_projection_field(path.as_os_str())),
        transport_running: controller.is_playing(),
    }
}

/// Build a projection key for static model fields outside explicit segment keys.
fn build_non_segment_static_projection_key(
    controller: &AppController,
) -> NonSegmentStaticProjectionCacheKey {
    use crate::app_core::state::UpdateStatus;
    NonSegmentStaticProjectionCacheKey {
        sources_selected: controller.ui.sources.selected,
        sources_len: controller.ui.sources.rows.len(),
        folder_rows_len: controller.ui.sources.folders.rows.len(),
        folder_focused: controller.ui.sources.folders.focused,
        folder_search_query_hash: hash_projection_field(
            &controller.ui.sources.folders.search_query,
        ),
        update_status: match controller.ui.update.status {
            UpdateStatus::Idle => 0,
            UpdateStatus::Checking => 1,
            UpdateStatus::UpdateAvailable => 2,
            UpdateStatus::Error => 3,
        },
        update_available_tag_hash: hash_optional_string(
            controller.ui.update.available_tag.as_deref(),
        ),
        update_available_url_hash: hash_optional_string(
            controller.ui.update.available_url.as_deref(),
        ),
        update_last_error_hash: hash_optional_string(controller.ui.update.last_error.as_deref()),
        volume_milli: (controller.ui.volume.clamp(0.0, 1.0) * 1000.0).round() as u16,
        transport_running: controller.is_playing(),
        trash_count: controller.ui.browser.trash.len(),
        neutral_count: controller.ui.browser.neutral.len(),
        keep_count: controller.ui.browser.keep.len(),
    }
}

/// Measure retained projection segment hit/miss counters over a fixed action loop.
///
/// The callback mutates controller state once per iteration. After each action
/// mutation, this helper runs native frame preparation and retained projection.
/// Warmup iterations are excluded from the returned counters.
pub fn measure_projection_segment_lookup_counts(
    controller: &mut AppController,
    warmup_iters: usize,
    measure_iters: usize,
    mut apply_step: impl FnMut(&mut AppController, usize),
) -> ProjectionSegmentLookupCounts {
    let mut cache = NativeProjectionCache::default();
    for step in 0..warmup_iters.max(1) {
        apply_step(controller, step);
        controller.prepare_native_frame(false);
        let _ = cache.resolve_or_project(controller, |controller| {
            controller.project_native_app_model()
        });
    }
    let _ = cache.take_segment_lookup_counts();
    for step in 0..measure_iters.max(1) {
        apply_step(controller, step);
        controller.prepare_native_frame(false);
        let _ = cache.resolve_or_project(controller, |controller| {
            controller.project_native_app_model()
        });
    }
    cache.take_segment_lookup_counts()
}

/// Return whether an action requires unconditional projection-cache invalidation.
fn action_requires_projection_cache_invalidation(action: &NativeUiAction) -> bool {
    !matches!(
        action,
        NativeUiAction::SeekWaveform { .. }
            | NativeUiAction::SetWaveformCursor { .. }
            | NativeUiAction::SetWaveformSelectionRange { .. }
            | NativeUiAction::ClearWaveformSelection
            | NativeUiAction::ZoomWaveform { .. }
            | NativeUiAction::ZoomWaveformToSelection
            | NativeUiAction::ZoomWaveformFull
            | NativeUiAction::SetVolume { .. }
            | NativeUiAction::CommitVolumeSetting
    )
}

/// Conservative source-node set used for broad invalidation actions.
const BROAD_DIRTY_SOURCES: [DerivedNodeId; 4] = [
    DerivedNodeId::BrowserState,
    DerivedNodeId::MapState,
    DerivedNodeId::TransportState,
    DerivedNodeId::StatusState,
];

/// Resolve the primary dirty source node and reason for one native action.
fn classify_dirty_source(action: &NativeUiAction) -> Option<(DerivedNodeId, DirtyReason)> {
    match action {
        NativeUiAction::SeekWaveform { .. }
        | NativeUiAction::SetWaveformCursor { .. }
        | NativeUiAction::SetWaveformSelectionRange { .. }
        | NativeUiAction::ClearWaveformSelection => Some((
            DerivedNodeId::WaveformState,
            DirtyReason::WaveformOverlayAction,
        )),
        NativeUiAction::ZoomWaveform { .. }
        | NativeUiAction::ZoomWaveformToSelection
        | NativeUiAction::ZoomWaveformFull => Some((
            DerivedNodeId::WaveformState,
            DirtyReason::WaveformViewAction,
        )),
        NativeUiAction::MoveBrowserFocus { .. }
        | NativeUiAction::FocusBrowserRow { .. }
        | NativeUiAction::CommitFocusedBrowserRow
        | NativeUiAction::ToggleBrowserRowSelection { .. }
        | NativeUiAction::ExtendBrowserSelectionToRow { .. }
        | NativeUiAction::AddRangeBrowserSelection { .. }
        | NativeUiAction::ExtendBrowserSelectionFromFocus { .. }
        | NativeUiAction::AddRangeBrowserSelectionFromFocus { .. }
        | NativeUiAction::ToggleFocusedBrowserRowSelection
        | NativeUiAction::SelectAllBrowserRows
        | NativeUiAction::SetBrowserSearch { .. }
        | NativeUiAction::FocusBrowserPanel
        | NativeUiAction::FocusBrowserSearch
        | NativeUiAction::FocusLoadedSampleInBrowser
        | NativeUiAction::StartBrowserRename
        | NativeUiAction::ConfirmBrowserRename
        | NativeUiAction::CancelBrowserRename
        | NativeUiAction::TagBrowserSelection { .. }
        | NativeUiAction::DeleteBrowserSelection
        | NativeUiAction::SetBrowserTab { map: false } => {
            Some((DerivedNodeId::BrowserState, DirtyReason::BrowserAction))
        }
        NativeUiAction::SetBrowserTab { map: true } | NativeUiAction::FocusMapSample { .. } => {
            Some((DerivedNodeId::MapState, DirtyReason::MapAction))
        }
        NativeUiAction::ToggleTransport
        | NativeUiAction::ToggleLoopPlayback
        | NativeUiAction::SetVolume { .. }
        | NativeUiAction::CommitVolumeSetting => {
            Some((DerivedNodeId::TransportState, DirtyReason::TransportAction))
        }
        NativeUiAction::CheckForUpdates
        | NativeUiAction::OpenUpdateLink
        | NativeUiAction::InstallUpdate
        | NativeUiAction::DismissUpdate
        | NativeUiAction::ConfirmPrompt
        | NativeUiAction::CancelPrompt
        | NativeUiAction::CancelProgress
        | NativeUiAction::SetPromptInput { .. } => {
            Some((DerivedNodeId::StatusState, DirtyReason::StatusAction))
        }
        _ => None,
    }
}

/// Return whether dirty waveform render inputs require a full image refresh.
fn waveform_render_inputs_require_refresh(reason: Option<DirtyReason>) -> bool {
    !matches!(reason, Some(DirtyReason::WaveformOverlayAction))
}

/// Return whether a waveform action should apply immediately for smooth preview.
///
/// These actions update overlay state frequently (cursor and selection edits) and
/// benefit from immediate feedback more than queue coalescing.
fn is_immediate_waveform_preview_action(action: &NativeUiAction) -> bool {
    matches!(
        action,
        NativeUiAction::SetWaveformCursor { .. }
            | NativeUiAction::SetWaveformSelectionRange { .. }
            | NativeUiAction::ClearWaveformSelection
    )
}

/// Queue of high-frequency waveform actions that can be coalesced per pull frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct PendingWaveformActions {
    /// Latest seek target in normalized milli space.
    seek_milli: Option<u16>,
    /// Latest cursor target in normalized milli space.
    cursor_milli: Option<u16>,
    /// Latest explicit selection range in normalized milli space.
    selection_range_milli: Option<(u16, u16)>,
    /// Whether selection should be cleared when no range override is queued.
    clear_selection: bool,
    /// Net signed waveform zoom step delta accumulated this frame.
    zoom_steps_delta: i16,
    /// Whether `ZoomWaveformToSelection` is queued for this frame.
    zoom_to_selection: bool,
    /// Whether `ZoomWaveformFull` is queued for this frame.
    zoom_full: bool,
}

impl PendingWaveformActions {
    /// Return true when at least one queued waveform action is present.
    fn has_pending(&self) -> bool {
        self.seek_milli.is_some()
            || self.cursor_milli.is_some()
            || self.selection_range_milli.is_some()
            || self.clear_selection
            || self.zoom_steps_delta != 0
            || self.zoom_to_selection
            || self.zoom_full
    }

    /// Queue a coalescable waveform action and return true when absorbed.
    fn enqueue(&mut self, action: &NativeUiAction) -> bool {
        match action {
            NativeUiAction::SeekWaveform { position_milli } => {
                self.seek_milli = Some(*position_milli);
                true
            }
            NativeUiAction::SetWaveformCursor { position_milli } => {
                self.cursor_milli = Some(*position_milli);
                true
            }
            NativeUiAction::SetWaveformSelectionRange {
                start_milli,
                end_milli,
            } => {
                self.selection_range_milli = Some((*start_milli, *end_milli));
                self.clear_selection = false;
                true
            }
            NativeUiAction::ClearWaveformSelection => {
                self.selection_range_milli = None;
                self.clear_selection = true;
                true
            }
            NativeUiAction::ZoomWaveform { zoom_in, steps } => {
                if self.zoom_full || self.zoom_to_selection {
                    return true;
                }
                let signed_steps = if *zoom_in {
                    i16::from(*steps)
                } else {
                    -i16::from(*steps)
                };
                self.zoom_steps_delta = self.zoom_steps_delta.saturating_add(signed_steps);
                true
            }
            NativeUiAction::ZoomWaveformToSelection => {
                self.zoom_steps_delta = 0;
                self.zoom_to_selection = true;
                self.zoom_full = false;
                true
            }
            NativeUiAction::ZoomWaveformFull => {
                self.zoom_steps_delta = 0;
                self.zoom_to_selection = false;
                self.zoom_full = true;
                true
            }
            _ => false,
        }
    }

    /// Return the derived-graph dirty reason represented by this pending batch.
    fn dirty_reason(&self) -> DirtyReason {
        if self.zoom_full || self.zoom_to_selection || self.zoom_steps_delta != 0 {
            DirtyReason::WaveformViewAction
        } else {
            DirtyReason::WaveformOverlayAction
        }
    }

    /// Return the cursor update after removing redundant cursor+seek pairs.
    ///
    /// A queued seek already updates cursor position, so sending both actions at
    /// the same normalized milli target adds no behavior but does add apply cost.
    fn deduped_cursor_milli(&self) -> Option<u16> {
        if self.cursor_milli.is_some() && self.cursor_milli == self.seek_milli {
            None
        } else {
            self.cursor_milli
        }
    }
}

/// Host bridge used by the native `radiant` runtime.
pub struct SempalNativeBridge {
    controller: AppController,
    projection_cache: NativeProjectionCache,
    /// Lazily recomputed projection cache key snapshot for controller state.
    projection_key_snapshot: Option<NativeProjectionCacheKey>,
    /// Dirty segments produced by the latest `pull_model` projection update.
    last_dirty_segments: NativeDirtySegments,
    /// Monotonic static-segment revisions from projection updates.
    segment_revisions: NativeSegmentRevisions,
    /// Coalesced pending waveform actions from high-frequency drag/wheel input.
    pending_waveform_actions: PendingWaveformActions,
}

impl SempalNativeBridge {
    /// Build a new native bridge initialized with persisted sempal configuration.
    pub fn new(
        renderer: WaveformRenderer,
        player: Option<Rc<RefCell<AudioPlayer>>>,
    ) -> Result<Self, String> {
        info!("Building native bridge controller");
        let controller = build_native_app_controller(renderer, player).map_err(|err| {
            error!(err = %err, "Failed to build native app controller");
            err
        })?;
        info!("Native bridge controller ready");
        Ok(Self {
            controller,
            projection_cache: NativeProjectionCache::default(),
            projection_key_snapshot: None,
            last_dirty_segments: NativeDirtySegments::all(),
            segment_revisions: NativeSegmentRevisions::default(),
            pending_waveform_actions: PendingWaveformActions::default(),
        })
    }

    /// Mark the cached projection key snapshot stale after controller mutation.
    fn invalidate_projection_key_snapshot(&mut self) {
        self.projection_key_snapshot = None;
    }

    /// Return a cached projection key snapshot, recomputing only when stale.
    fn projection_key_snapshot(&mut self) -> NativeProjectionCacheKey {
        if let Some(key) = self.projection_key_snapshot.as_ref().cloned() {
            return key;
        }
        let key = build_projection_cache_key(&self.controller);
        self.projection_key_snapshot = Some(key.clone());
        key
    }

    /// Return a projection key snapshot and optionally validate it against fresh state.
    ///
    /// Validation is opt-in (`SEMPAL_NATIVE_BRIDGE_ASSERT_PROJECTION_SNAPSHOT`) so
    /// production paths avoid extra hash work. When enabled, stale snapshots are
    /// counted and immediately corrected to protect correctness during perf audits.
    fn projection_key_snapshot_for_pull(&mut self) -> NativeProjectionCacheKey {
        let mut key = self.projection_key_snapshot();
        if !projection_key_assertions_enabled() {
            return key;
        }
        let fresh_key = build_projection_cache_key(&self.controller);
        let stale = key != fresh_key;
        trace_projection_key_assertion(stale);
        if stale {
            self.projection_key_snapshot = Some(fresh_key.clone());
            key = fresh_key;
        }
        key
    }

    /// Apply browser-focus movement immediately so wheel/arrow nudges are visible
    /// in the same frame instead of waiting for a pending-input flush boundary.
    fn apply_browser_focus_delta_immediately(&mut self, delta: i8) {
        if delta == 0 {
            return;
        }
        let action = NativeUiAction::MoveBrowserFocus { delta };
        let before_key = self.projection_key_snapshot();
        self.controller.focus_browser_delta_action(delta);
        self.invalidate_projection_key_snapshot();
        let after_key = self.projection_key_snapshot();
        if before_key != after_key {
            self.mark_dirty_for_action(&action);
            self.projection_cache.invalidate_key_only();
        }
    }

    /// Queue a coalescable waveform action and return whether it was absorbed.
    fn enqueue_waveform_action(&mut self, action: &NativeUiAction) -> bool {
        self.pending_waveform_actions.enqueue(action)
    }

    /// Apply one action immediately using the standard dirty + queue-flush flow.
    fn apply_action_immediately(&mut self, action: NativeUiAction) {
        self.mark_dirty_for_action(&action);
        self.flush_pending_input_actions();
        self.controller.apply_native_ui_action(action);
        self.invalidate_projection_key_snapshot();
    }

    /// Mark derived graph sources affected by one action.
    fn mark_dirty_for_action(&mut self, action: &NativeUiAction) {
        if let Some((source, reason)) = classify_dirty_source(action) {
            self.controller.mark_derived_source_dirty(source, reason);
        }
        if action_requires_projection_cache_invalidation(action) {
            self.controller
                .mark_derived_sources_dirty(&BROAD_DIRTY_SOURCES, DirtyReason::BroadInvalidation);
        }
    }

    /// Apply queued waveform actions in deterministic order before projection.
    fn flush_pending_waveform_actions(&mut self) {
        if !self.pending_waveform_actions.has_pending() {
            return;
        }
        let pending = std::mem::take(&mut self.pending_waveform_actions);
        let cursor_milli = pending.deduped_cursor_milli();
        let profiling = bridge_profiling_enabled();
        let flush_start = profiling.then(Instant::now);
        let before_key = self.projection_key_snapshot();
        let mut emitted_actions = 0u64;

        self.controller.begin_waveform_refresh_batch();
        if pending.zoom_full {
            self.controller
                .apply_native_ui_action(NativeUiAction::ZoomWaveformFull);
            emitted_actions = emitted_actions.saturating_add(1);
        } else if pending.zoom_to_selection {
            self.controller
                .apply_native_ui_action(NativeUiAction::ZoomWaveformToSelection);
            emitted_actions = emitted_actions.saturating_add(1);
        } else if pending.zoom_steps_delta != 0 {
            let zoom_in = pending.zoom_steps_delta.is_positive();
            let steps = pending
                .zoom_steps_delta
                .unsigned_abs()
                .min(u16::from(u8::MAX)) as u8;
            self.controller
                .apply_native_ui_action(NativeUiAction::ZoomWaveform { zoom_in, steps });
            emitted_actions = emitted_actions.saturating_add(1);
        }

        if let Some((start_milli, end_milli)) = pending.selection_range_milli {
            self.controller
                .apply_native_ui_action(NativeUiAction::SetWaveformSelectionRange {
                    start_milli,
                    end_milli,
                });
            emitted_actions = emitted_actions.saturating_add(1);
        } else if pending.clear_selection {
            self.controller
                .apply_native_ui_action(NativeUiAction::ClearWaveformSelection);
            emitted_actions = emitted_actions.saturating_add(1);
        }

        if let Some(position_milli) = cursor_milli {
            self.controller
                .apply_native_ui_action(NativeUiAction::SetWaveformCursor { position_milli });
            emitted_actions = emitted_actions.saturating_add(1);
        }
        if let Some(position_milli) = pending.seek_milli {
            self.controller
                .apply_native_ui_action(NativeUiAction::SeekWaveform { position_milli });
            emitted_actions = emitted_actions.saturating_add(1);
        }
        self.controller.end_waveform_refresh_batch();
        if emitted_actions == 0 {
            if profiling {
                let flush_duration = flush_start.map_or(Duration::ZERO, |start| start.elapsed());
                trace_waveform_flush(flush_duration, emitted_actions);
            }
            return;
        }
        let after_key = build_projection_cache_key(&self.controller);
        if before_key != after_key {
            self.projection_key_snapshot = Some(after_key);
            self.controller
                .mark_derived_source_dirty(DerivedNodeId::WaveformState, pending.dirty_reason());
        }

        if profiling {
            let flush_duration = flush_start.map_or(Duration::ZERO, |start| start.elapsed());
            trace_waveform_flush(flush_duration, emitted_actions);
        }
    }

    /// Flush all coalesced high-frequency waveform action queues before projection.
    fn flush_pending_input_actions(&mut self) {
        self.flush_pending_waveform_actions();
    }

    /// Recompute dirty derived nodes and invalidate projection cache when required.
    fn flush_derived_updates_before_pull(&mut self, animation_only: bool) {
        if animation_only || !self.controller.has_dirty_derived_nodes() {
            return;
        }
        let dirty_sources = self.controller.dirty_derived_source_count();
        let dirty_computed = self.controller.dirty_derived_computed_count();
        let profiling = bridge_profiling_enabled();
        let flush_start = profiling.then(Instant::now);
        let dirty_nodes = self.controller.dirty_derived_nodes_in_topo_order();
        let mut projection_key_dirty = false;
        for node in dirty_nodes {
            if node == DerivedNodeId::WaveformRenderInputs {
                let should_refresh = waveform_render_inputs_require_refresh(
                    self.controller.derived_dirty_reason(node),
                );
                if should_refresh {
                    self.controller.refresh_waveform_image();
                    self.invalidate_projection_key_snapshot();
                }
                trace_waveform_image_refresh(should_refresh);
            }
            if node == DerivedNodeId::NativeAppProjectionKey {
                projection_key_dirty = true;
            }
            self.controller.clear_derived_dirty_node(node);
        }
        if projection_key_dirty {
            self.projection_cache.invalidate_key_only();
            self.invalidate_projection_key_snapshot();
        }
        if profiling {
            let flush_duration = flush_start.map_or(Duration::ZERO, |start| start.elapsed());
            trace_derived_flush(flush_duration, dirty_sources, dirty_computed);
        }
    }

    /// Pull and project the latest app model snapshot as a shared retained arc.
    fn pull_model_arc_snapshot(&mut self) -> Arc<NativeAppModel> {
        let call = trace_pull_model_call();
        let profiling = bridge_profiling_enabled();
        let prepare_start = profiling.then(Instant::now);
        if call <= 24 {
            info!(call, "native bridge: pull_model start");
        }
        self.flush_pending_input_actions();
        self.controller.prepare_native_frame(false);
        self.flush_derived_updates_before_pull(false);
        let prepare_duration = prepare_start.map_or(Duration::ZERO, |start| start.elapsed());
        if profiling {
            trace_pull_model_preparation(prepare_duration);
        }
        let project_start = profiling.then(Instant::now);
        let projection_key = self.projection_key_snapshot_for_pull();
        let (model, dirty_segments) = self.projection_cache.resolve_or_project_with_key(
            &mut self.controller,
            &projection_key,
            |controller| controller.project_native_app_model(),
        );
        self.last_dirty_segments = dirty_segments;
        self.segment_revisions
            .bump_for_dirty_segments(self.last_dirty_segments);
        let project_duration = project_start.map_or(Duration::ZERO, |start| start.elapsed());
        if profiling {
            trace_pull_model_projection(project_duration);
        }
        if call <= 24 {
            info!(
                call,
                transport_running = model.transport_running,
                browser_visible = model.browser.visible_count,
                status_len = model.status_text.len(),
                "native bridge: pull_model completed"
            );
        }
        if profiling && call.is_multiple_of(BRIDGE_PROFILE_INTERVAL) {
            maybe_log_bridge_profile();
        }
        model
    }
}

impl NativeAppBridge for SempalNativeBridge {
    /// Pull the latest app model snapshot for runtimes expecting owned values.
    ///
    /// This compatibility path may clone when shared ownership exists; native
    /// Vello consumes `pull_model_arc` to avoid full-model clone churn.
    fn pull_model(&mut self) -> NativeAppModel {
        Arc::unwrap_or_clone(self.pull_model_arc_snapshot())
    }

    /// Pull the latest app model snapshot as a shared immutable arc.
    ///
    /// Returning shared ownership lets retained projection caches reuse model
    /// snapshots across pulls without cloning the full `AppModel`.
    fn pull_model_arc(&mut self) -> Arc<NativeAppModel> {
        self.pull_model_arc_snapshot()
    }

    /// Return and clear the bridge segment mask from the most recent model pull.
    fn take_dirty_segments(&mut self) -> NativeDirtySegments {
        std::mem::replace(&mut self.last_dirty_segments, NativeDirtySegments::empty())
    }

    /// Return the latest static-segment revision snapshot.
    fn take_segment_revisions(&mut self) -> NativeSegmentRevisions {
        self.segment_revisions
    }

    fn pull_motion_model(&mut self) -> Option<NativeMotionModel> {
        let call = trace_pull_motion_call();
        let profiling = bridge_profiling_enabled();
        let prepare_start = profiling.then(Instant::now);
        if call <= 24 {
            info!(call, "native bridge: pull_motion_model start");
        }
        self.flush_pending_input_actions();
        self.controller.prepare_native_frame(true);
        self.flush_derived_updates_before_pull(true);
        let prepare_duration = prepare_start.map_or(Duration::ZERO, |start| start.elapsed());
        if profiling {
            trace_pull_motion_preparation(prepare_duration);
        }
        let project_start = profiling.then(Instant::now);
        let model = Some(self.controller.project_native_motion_model());
        let project_duration = project_start.map_or(Duration::ZERO, |start| start.elapsed());
        if profiling {
            trace_pull_motion_projection(project_duration);
        }
        if call <= 24 {
            info!(call, "native bridge: pull_motion_model completed");
        }
        if profiling && call.is_multiple_of(BRIDGE_PROFILE_INTERVAL) {
            maybe_log_bridge_profile();
        }
        model
    }

    fn on_action(&mut self, action: NativeUiAction) {
        if let NativeUiAction::MoveBrowserFocus { delta } = action {
            let call = trace_action_call();
            let profiling = bridge_profiling_enabled();
            let action_start = profiling.then(Instant::now);
            if call <= 64 {
                info!(call, delta, "native bridge: apply MoveBrowserFocus");
            }
            self.apply_browser_focus_delta_immediately(delta);
            if profiling {
                let action_duration = action_start.map_or(Duration::ZERO, |start| start.elapsed());
                trace_action_duration(action_duration);
                trace_action_interaction(InteractionActionClass::Wheel, action_duration);
            }
            return;
        }
        if is_immediate_waveform_preview_action(&action) && immediate_waveform_preview_enabled() {
            let call = trace_action_call();
            let profiling = bridge_profiling_enabled();
            let action_start = profiling.then(Instant::now);
            if call <= 64 {
                info!(call, action = ?action, "native bridge: apply waveform preview action");
            }
            self.apply_action_immediately(action);
            if profiling {
                let action_duration = action_start.map_or(Duration::ZERO, |start| start.elapsed());
                trace_action_duration(action_duration);
                trace_action_interaction(InteractionActionClass::Waveform, action_duration);
            }
            return;
        }
        if self.enqueue_waveform_action(&action) {
            let call = trace_action_call();
            let profiling = bridge_profiling_enabled();
            let action_start = profiling.then(Instant::now);
            if call <= 64 {
                info!(call, action = ?action, "native bridge: queue waveform action");
            }
            if profiling {
                let action_duration = action_start.map_or(Duration::ZERO, |start| start.elapsed());
                trace_action_duration(action_duration);
                trace_action_interaction(InteractionActionClass::Waveform, action_duration);
            }
            return;
        }
        let call = trace_action_call();
        let profiling = bridge_profiling_enabled();
        let interaction_class = classify_action_interaction(&action);
        let action_start = profiling.then(Instant::now);
        if call <= 64 {
            info!(call, action = ?action, "native bridge: on_action");
        }
        self.apply_action_immediately(action);
        if profiling {
            let action_duration = action_start.map_or(Duration::ZERO, |start| start.elapsed());
            trace_action_duration(action_duration);
            if let Some(kind) = interaction_class {
                trace_action_interaction(kind, action_duration);
            }
        }
    }

    fn on_frame_result(&mut self, result: NativeFrameBuildResult) {
        let profiling = bridge_profiling_enabled();
        if !profiling {
            return;
        }
        let frame_count = trace_frame_result(&result);
        if frame_count.is_multiple_of(BRIDGE_PROFILE_INTERVAL) {
            maybe_log_bridge_profile();
        }
    }

    fn on_exit(&mut self) {
        self.flush_pending_input_actions();
        if let Err(err) = self.controller.persist_native_exit_config() {
            error!(err = %err, "Failed to persist config on native exit");
            eprintln!("{err}");
            return;
        }
        info!("Persisted config on native exit");
    }
}

/// Construct a native runtime bridge for the current `sempal` controller stack.
///
/// This is the application-facing constructor used by host integrations. It
/// keeps bridge construction in `app_core` and returns a controller-backed
/// implementation of `NativeAppBridge` that delegates all GUI work to `radiant`.
pub fn new_native_bridge(
    renderer: WaveformRenderer,
    player: Option<Rc<RefCell<AudioPlayer>>>,
) -> Result<SempalNativeBridge, String> {
    SempalNativeBridge::new(renderer, player)
}

#[cfg(test)]
mod tests {
    use super::{
        DerivedNodeId, NativeProjectionCache, PendingWaveformActions, SempalNativeBridge,
        build_projection_cache_key,
    };
    use crate::app_core::actions::{
        NativeAppBridge, NativeDirtySegments, NativeSegmentRevisions, NativeUiAction,
    };
    use crate::app_core::controller::{AppController, AppControllerNativeRuntimeExt};
    use crate::app_core::state::UpdateStatus;
    use crate::waveform::WaveformRenderer;
    use std::sync::Arc;

    #[test]
    fn projection_cache_key_changes_when_map_cache_revision_changes() {
        let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
        let first = build_projection_cache_key(&controller);
        controller.ui.map.cached_points_revision += 1;
        let second = build_projection_cache_key(&controller);
        assert_ne!(first, second);
    }

    #[test]
    fn projection_cache_key_changes_when_update_status_changes() {
        let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
        let first = build_projection_cache_key(&controller);
        controller.ui.update.status = UpdateStatus::Checking;
        let second = build_projection_cache_key(&controller);
        assert_ne!(first, second);
    }

    #[test]
    /// Projection cache keys must change when selected-path revisions change.
    fn projection_cache_key_changes_when_selected_path_revision_changes() {
        let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
        controller.ui.browser.selected_paths = vec![std::path::PathBuf::from("first.wav")];
        let first = build_projection_cache_key(&controller);
        controller.ui.browser.selected_paths = vec![std::path::PathBuf::from("second.wav")];
        controller.mark_browser_selected_paths_changed();
        let second = build_projection_cache_key(&controller);
        assert_ne!(first, second);
    }

    #[test]
    fn projection_cache_reuses_model_when_key_unchanged() {
        let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
        let mut cache = NativeProjectionCache::default();
        let mut projections = 0usize;

        let _ = cache.resolve_or_project(&mut controller, |controller| {
            projections += 1;
            controller.project_native_app_model()
        });
        let _ = cache.resolve_or_project(&mut controller, |controller| {
            projections += 1;
            controller.project_native_app_model()
        });
        assert_eq!(projections, 1);

        controller.ui.status.text = String::from("changed");
        let refreshed = cache.resolve_or_project(&mut controller, |controller| {
            projections += 1;
            controller.project_native_app_model()
        });
        assert_eq!(projections, 1);
        assert_eq!(refreshed.0.status_text.as_str(), "changed");
    }

    #[test]
    fn projection_cache_invalidate_forces_refresh() {
        let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
        let mut cache = NativeProjectionCache::default();
        let mut projections = 0usize;

        let _ = cache.resolve_or_project(&mut controller, |controller| {
            projections += 1;
            controller.project_native_app_model()
        });
        cache.invalidate();
        let _ = cache.resolve_or_project(&mut controller, |controller| {
            projections += 1;
            controller.project_native_app_model()
        });

        assert_eq!(projections, 2);
    }

    /// Initial full projection should bump all static segment revisions.
    #[test]
    fn pull_model_bumps_segment_revisions_on_first_projection() {
        let controller = AppController::new(WaveformRenderer::new(16, 16), None);
        let mut bridge = SempalNativeBridge {
            controller,
            projection_cache: NativeProjectionCache::default(),
            projection_key_snapshot: None,
            last_dirty_segments: NativeDirtySegments::all(),
            segment_revisions: NativeSegmentRevisions::default(),
            pending_waveform_actions: PendingWaveformActions::default(),
        };

        let _ = bridge.pull_model();
        let revisions = bridge.take_segment_revisions();

        assert!(revisions.has_static_revisions());
        assert!(revisions.status_bar > 0);
        assert!(revisions.browser_frame > 0);
        assert!(revisions.browser_rows_window > 0);
        assert!(revisions.map_panel > 0);
        assert!(revisions.waveform_overlay > 0);
        assert!(revisions.global_static > 0);
    }

    /// No-op immediate focus movement should keep projection cache keys intact.
    #[test]
    fn apply_browser_focus_delta_immediate_noop_keeps_projection_cache_key() {
        let controller = AppController::new(WaveformRenderer::new(16, 16), None);
        let key = build_projection_cache_key(&controller);
        let cache = NativeProjectionCache {
            app_key: Some(key.clone()),
            ..NativeProjectionCache::default()
        };

        let mut bridge = SempalNativeBridge {
            controller,
            projection_cache: cache,
            projection_key_snapshot: None,
            last_dirty_segments: NativeDirtySegments::all(),
            segment_revisions: NativeSegmentRevisions::default(),
            pending_waveform_actions: PendingWaveformActions::default(),
        };
        bridge.apply_browser_focus_delta_immediately(1);
        assert_eq!(bridge.projection_cache.app_key, Some(key));
    }

    /// Waveform preview-class actions should bypass queueing for immediate feedback.
    #[test]
    fn on_action_applies_waveform_preview_actions_immediately() {
        let controller = AppController::new(WaveformRenderer::new(16, 16), None);
        let mut bridge = SempalNativeBridge {
            controller,
            projection_cache: NativeProjectionCache::default(),
            projection_key_snapshot: None,
            last_dirty_segments: NativeDirtySegments::all(),
            segment_revisions: NativeSegmentRevisions::default(),
            pending_waveform_actions: PendingWaveformActions::default(),
        };

        bridge.on_action(NativeUiAction::SetWaveformCursor {
            position_milli: 420,
        });

        assert_eq!(bridge.pending_waveform_actions.cursor_milli, None);
        assert!(
            bridge
                .controller
                .is_derived_node_dirty_for_test(DerivedNodeId::WaveformState)
        );
    }

    /// Seek actions should remain coalesced in the queue to cap apply-stage cost.
    #[test]
    fn on_action_keeps_seek_actions_queued() {
        let controller = AppController::new(WaveformRenderer::new(16, 16), None);
        let mut bridge = SempalNativeBridge {
            controller,
            projection_cache: NativeProjectionCache::default(),
            projection_key_snapshot: None,
            last_dirty_segments: NativeDirtySegments::all(),
            segment_revisions: NativeSegmentRevisions::default(),
            pending_waveform_actions: PendingWaveformActions::default(),
        };

        bridge.on_action(NativeUiAction::SeekWaveform {
            position_milli: 333,
        });

        assert_eq!(bridge.pending_waveform_actions.seek_milli, Some(333));
    }

    /// Queued waveform actions should coalesce to last-write-wins semantics.
    #[test]
    fn waveform_action_queue_last_write_wins() {
        let mut queue = PendingWaveformActions::default();
        assert!(queue.enqueue(&NativeUiAction::SeekWaveform {
            position_milli: 100,
        }));
        assert!(queue.enqueue(&NativeUiAction::SeekWaveform {
            position_milli: 220,
        }));
        assert!(queue.enqueue(&NativeUiAction::SetWaveformCursor {
            position_milli: 300,
        }));
        assert!(queue.enqueue(&NativeUiAction::SetWaveformCursor {
            position_milli: 420,
        }));
        assert_eq!(queue.seek_milli, Some(220));
        assert_eq!(queue.cursor_milli, Some(420));
    }

    /// Cursor updates should be dropped when seek targets the same milli value.
    #[test]
    fn waveform_action_queue_dedupes_cursor_when_seek_matches() {
        let mut queue = PendingWaveformActions::default();
        assert!(queue.enqueue(&NativeUiAction::SetWaveformCursor {
            position_milli: 420,
        }));
        assert!(queue.enqueue(&NativeUiAction::SeekWaveform {
            position_milli: 420,
        }));
        assert_eq!(queue.deduped_cursor_milli(), None);
    }

    /// Zoom-to-selection and zoom-full should override discrete zoom deltas.
    #[test]
    fn waveform_action_queue_zoom_overrides_delta() {
        let mut queue = PendingWaveformActions::default();
        assert!(queue.enqueue(&NativeUiAction::ZoomWaveform {
            zoom_in: true,
            steps: 3,
        }));
        assert!(queue.enqueue(&NativeUiAction::ZoomWaveformToSelection));
        assert_eq!(queue.zoom_steps_delta, 0);
        assert!(queue.zoom_to_selection);
        assert!(!queue.zoom_full);

        assert!(queue.enqueue(&NativeUiAction::ZoomWaveformFull));
        assert_eq!(queue.zoom_steps_delta, 0);
        assert!(!queue.zoom_to_selection);
        assert!(queue.zoom_full);
    }

    /// Clear-selection requests should yield to later explicit range updates.
    #[test]
    fn waveform_action_queue_selection_range_overrides_clear() {
        let mut queue = PendingWaveformActions::default();
        assert!(queue.enqueue(&NativeUiAction::ClearWaveformSelection));
        assert!(queue.clear_selection);
        assert!(queue.selection_range_milli.is_none());
        assert!(queue.enqueue(&NativeUiAction::SetWaveformSelectionRange {
            start_milli: 120,
            end_milli: 400,
        }));
        assert!(!queue.clear_selection);
        assert_eq!(queue.selection_range_milli, Some((120, 400)));
    }

    /// Pending queue dirty reasons should distinguish overlay-only from view edits.
    #[test]
    fn waveform_queue_dirty_reason_matches_enqueued_actions() {
        let mut queue = PendingWaveformActions::default();
        assert!(queue.enqueue(&NativeUiAction::SetWaveformCursor {
            position_milli: 400,
        }));
        assert_eq!(
            queue.dirty_reason(),
            super::DirtyReason::WaveformOverlayAction
        );

        assert!(queue.enqueue(&NativeUiAction::ZoomWaveform {
            zoom_in: true,
            steps: 1,
        }));
        assert_eq!(queue.dirty_reason(), super::DirtyReason::WaveformViewAction);
    }

    /// Overlay-only dirty reasons should skip waveform image refresh work.
    #[test]
    fn waveform_render_inputs_refresh_policy_skips_overlay_only() {
        assert!(!super::waveform_render_inputs_require_refresh(Some(
            super::DirtyReason::WaveformOverlayAction
        )));
        assert!(super::waveform_render_inputs_require_refresh(Some(
            super::DirtyReason::WaveformViewAction
        )));
        assert!(super::waveform_render_inputs_require_refresh(None));
    }

    /// Flushing queued waveform actions should clear queue state and mark waveform dirties.
    #[test]
    fn flush_pending_waveform_actions_clears_queue_and_marks_waveform_dirty() {
        let controller = AppController::new(WaveformRenderer::new(16, 16), None);
        let cache = NativeProjectionCache {
            app_key: Some(build_projection_cache_key(&controller)),
            ..NativeProjectionCache::default()
        };
        let mut bridge = SempalNativeBridge {
            controller,
            projection_cache: cache,
            projection_key_snapshot: None,
            last_dirty_segments: NativeDirtySegments::all(),
            segment_revisions: NativeSegmentRevisions::default(),
            pending_waveform_actions: PendingWaveformActions::default(),
        };

        assert!(
            bridge.enqueue_waveform_action(&NativeUiAction::SetWaveformCursor {
                position_milli: 500,
            })
        );
        bridge.flush_pending_waveform_actions();

        assert!(!bridge.pending_waveform_actions.has_pending());
        assert!(
            bridge
                .controller
                .is_derived_node_dirty_for_test(DerivedNodeId::WaveformState)
        );
        assert!(bridge.projection_cache.app_key.is_some());
    }

    /// No-op queued waveform actions should not dirty the derived graph.
    #[test]
    fn flush_pending_waveform_actions_noop_skips_dirty_marking() {
        let mut bridge = SempalNativeBridge {
            controller: AppController::new(WaveformRenderer::new(16, 16), None),
            projection_cache: NativeProjectionCache::default(),
            projection_key_snapshot: None,
            last_dirty_segments: NativeDirtySegments::all(),
            segment_revisions: NativeSegmentRevisions::default(),
            pending_waveform_actions: PendingWaveformActions::default(),
        };

        assert!(
            bridge.enqueue_waveform_action(&NativeUiAction::SetWaveformCursor {
                position_milli: 500,
            })
        );
        bridge.flush_pending_waveform_actions();
        let Some(first_snapshot) = bridge.projection_key_snapshot.as_ref().cloned() else {
            panic!("waveform flush should retain a projection key snapshot");
        };
        bridge.flush_derived_updates_before_pull(false);
        assert!(!bridge.controller.has_dirty_derived_nodes());

        assert!(
            bridge.enqueue_waveform_action(&NativeUiAction::SetWaveformCursor {
                position_milli: 500,
            })
        );
        bridge.flush_pending_waveform_actions();

        assert!(
            !bridge
                .controller
                .is_derived_node_dirty_for_test(DerivedNodeId::WaveformState)
        );
        assert_eq!(
            bridge.projection_key_snapshot.as_ref(),
            Some(&first_snapshot)
        );
    }

    /// Action classification should mark waveform source and projection-key nodes dirty.
    #[test]
    fn mark_dirty_for_waveform_action_marks_graph_nodes() {
        let controller = AppController::new(WaveformRenderer::new(16, 16), None);
        let mut bridge = SempalNativeBridge {
            controller,
            projection_cache: NativeProjectionCache::default(),
            projection_key_snapshot: None,
            last_dirty_segments: NativeDirtySegments::all(),
            segment_revisions: NativeSegmentRevisions::default(),
            pending_waveform_actions: PendingWaveformActions::default(),
        };

        bridge.mark_dirty_for_action(&NativeUiAction::SeekWaveform {
            position_milli: 250,
        });

        assert!(
            bridge
                .controller
                .is_derived_node_dirty_for_test(DerivedNodeId::WaveformState)
        );
        assert!(
            bridge
                .controller
                .is_derived_node_dirty_for_test(DerivedNodeId::NativeAppProjectionKey)
        );
    }

    /// Flushing derived updates should clear graph dirties and invalidate projection cache key.
    #[test]
    fn flush_derived_updates_clears_nodes_and_invalidates_key() {
        let controller = AppController::new(WaveformRenderer::new(16, 16), None);
        let cache = NativeProjectionCache {
            app_key: Some(build_projection_cache_key(&controller)),
            ..NativeProjectionCache::default()
        };
        let mut bridge = SempalNativeBridge {
            controller,
            projection_cache: cache,
            projection_key_snapshot: None,
            last_dirty_segments: NativeDirtySegments::all(),
            segment_revisions: NativeSegmentRevisions::default(),
            pending_waveform_actions: PendingWaveformActions::default(),
        };
        let _ = bridge.projection_key_snapshot();
        assert!(bridge.projection_key_snapshot.is_some());

        bridge.mark_dirty_for_action(&NativeUiAction::SetBrowserSearch {
            query: String::from("kick"),
        });
        bridge.flush_derived_updates_before_pull(false);

        assert!(!bridge.controller.has_dirty_derived_nodes());
        assert!(bridge.projection_cache.app_key.is_none());
        assert!(bridge.projection_key_snapshot.is_none());
    }

    /// Repeated no-op pulls should preserve snapshot/cache reuse and avoid full reprojection.
    #[test]
    fn pull_model_snapshot_noop_pull_reuses_cached_projection() {
        let mut bridge = SempalNativeBridge {
            controller: AppController::new(WaveformRenderer::new(16, 16), None),
            projection_cache: NativeProjectionCache::default(),
            projection_key_snapshot: None,
            last_dirty_segments: NativeDirtySegments::all(),
            segment_revisions: NativeSegmentRevisions::default(),
            pending_waveform_actions: PendingWaveformActions::default(),
        };

        let first_model = bridge.pull_model_arc_snapshot();
        let Some(first_snapshot) = bridge.projection_key_snapshot.as_ref().cloned() else {
            panic!("pull should populate projection key snapshot");
        };
        let Some(first_cache_key) = bridge.projection_cache.app_key.as_ref().cloned() else {
            panic!("pull should populate projection cache key");
        };
        assert_eq!(first_snapshot, first_cache_key);

        let second_model = bridge.pull_model_arc_snapshot();
        assert!(Arc::ptr_eq(&first_model, &second_model));
        assert_eq!(
            bridge.projection_key_snapshot.as_ref(),
            Some(&first_snapshot)
        );
    }

    #[cfg(feature = "native-bridge-metrics")]
    #[test]
    fn parse_bridge_profile_enabled_is_case_insensitive() {
        assert!(super::parse_bridge_profile_enabled("TRUE"));
        assert!(super::parse_bridge_profile_enabled("on"));
        assert!(super::parse_bridge_profile_enabled("Yes"));
        assert!(super::parse_bridge_profile_enabled("  true  "));
        assert!(!super::parse_bridge_profile_enabled("0"));
        assert!(!super::parse_bridge_profile_enabled("no"));
        assert!(!super::parse_bridge_profile_enabled(""));
    }

    /// Immediate waveform preview parser should accept canonical truthy variants.
    #[test]
    fn parse_immediate_waveform_preview_is_case_insensitive() {
        assert!(super::parse_immediate_waveform_preview("TRUE"));
        assert!(super::parse_immediate_waveform_preview("on"));
        assert!(super::parse_immediate_waveform_preview("Yes"));
        assert!(super::parse_immediate_waveform_preview("  true  "));
        assert!(!super::parse_immediate_waveform_preview("0"));
        assert!(!super::parse_immediate_waveform_preview("no"));
        assert!(!super::parse_immediate_waveform_preview(""));
    }

    #[cfg(feature = "native-bridge-metrics")]
    #[test]
    /// Bridge metrics should record projection cache and waveform refresh decisions.
    fn bridge_metrics_track_projection_cache_and_waveform_refresh_paths() {
        let projection_hit_before =
            super::PROJECTION_CACHE_HIT_COUNT.load(std::sync::atomic::Ordering::Relaxed);
        let projection_miss_before =
            super::PROJECTION_CACHE_MISS_COUNT.load(std::sync::atomic::Ordering::Relaxed);
        let refresh_apply_before =
            super::WAVEFORM_IMAGE_REFRESH_APPLY_COUNT.load(std::sync::atomic::Ordering::Relaxed);
        let refresh_skip_before =
            super::WAVEFORM_IMAGE_REFRESH_SKIP_COUNT.load(std::sync::atomic::Ordering::Relaxed);

        let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
        let mut cache = NativeProjectionCache::default();
        let _ = cache.resolve_or_project(&mut controller, |controller| {
            controller.project_native_app_model()
        });
        let _ = cache.resolve_or_project(&mut controller, |controller| {
            controller.project_native_app_model()
        });

        let mut bridge = SempalNativeBridge {
            controller,
            projection_cache: NativeProjectionCache::default(),
            projection_key_snapshot: None,
            last_dirty_segments: NativeDirtySegments::all(),
            segment_revisions: NativeSegmentRevisions::default(),
            pending_waveform_actions: PendingWaveformActions::default(),
        };
        bridge.controller.mark_derived_source_dirty(
            DerivedNodeId::WaveformState,
            super::DirtyReason::WaveformOverlayAction,
        );
        bridge.flush_derived_updates_before_pull(false);
        bridge.controller.mark_derived_source_dirty(
            DerivedNodeId::WaveformState,
            super::DirtyReason::WaveformViewAction,
        );
        bridge.flush_derived_updates_before_pull(false);

        let projection_hit_after =
            super::PROJECTION_CACHE_HIT_COUNT.load(std::sync::atomic::Ordering::Relaxed);
        let projection_miss_after =
            super::PROJECTION_CACHE_MISS_COUNT.load(std::sync::atomic::Ordering::Relaxed);
        let refresh_apply_after =
            super::WAVEFORM_IMAGE_REFRESH_APPLY_COUNT.load(std::sync::atomic::Ordering::Relaxed);
        let refresh_skip_after =
            super::WAVEFORM_IMAGE_REFRESH_SKIP_COUNT.load(std::sync::atomic::Ordering::Relaxed);

        assert!(projection_hit_after >= projection_hit_before.saturating_add(1));
        assert!(projection_miss_after >= projection_miss_before.saturating_add(1));
        assert!(refresh_apply_after >= refresh_apply_before.saturating_add(1));
        assert!(refresh_skip_after >= refresh_skip_before.saturating_add(1));
    }
}
