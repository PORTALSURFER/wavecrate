use super::action_classification::InteractionActionClass;
use super::projection_cache::ProjectionSegment;
use crate::app_core::actions::NativeFrameBuildResult;
#[cfg(feature = "native-bridge-metrics")]
use crate::app_core::native_shell;
#[cfg(feature = "native-bridge-metrics")]
use std::sync::{
    OnceLock,
    atomic::{AtomicU64, Ordering},
};
use std::time::Duration;
#[cfg(feature = "native-bridge-metrics")]
use tracing::info;

#[cfg(feature = "native-bridge-metrics")]
pub(super) const BRIDGE_PROFILE_INTERVAL: u64 = 240;
#[cfg(not(feature = "native-bridge-metrics"))]
pub(super) const BRIDGE_PROFILE_INTERVAL: u64 = 1;

#[cfg(feature = "native-bridge-metrics")]
const BRIDGE_PROFILE_ENV: &str = "SEMPAL_NATIVE_BRIDGE_PROFILE";
#[cfg(feature = "native-bridge-metrics")]
/// Enable runtime validation that cached projection-key snapshots stay in sync.
const PROJECTION_KEY_ASSERT_ENV: &str = "SEMPAL_NATIVE_BRIDGE_ASSERT_PROJECTION_SNAPSHOT";
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
pub(super) static PROJECTION_CACHE_HIT_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Total number of projection-cache lookups that required a fresh projection.
pub(super) static PROJECTION_CACHE_MISS_COUNT: AtomicU64 = AtomicU64::new(0);
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
pub(super) static WAVEFORM_IMAGE_REFRESH_APPLY_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Total number of waveform image refresh requests skipped as overlay-only.
pub(super) static WAVEFORM_IMAGE_REFRESH_SKIP_COUNT: AtomicU64 = AtomicU64::new(0);
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

#[cfg(feature = "native-bridge-metrics")]
pub(super) fn bridge_profiling_enabled() -> bool {
    *BRIDGE_PROFILE_ENABLED.get_or_init(|| crate::env_flags::env_var_truthy(BRIDGE_PROFILE_ENV))
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline]
pub(super) fn bridge_profiling_enabled() -> bool {
    false
}

#[cfg(feature = "native-bridge-metrics")]
/// Resolve whether projection-key snapshot assertions should run.
pub(super) fn projection_key_assertions_enabled() -> bool {
    *PROJECTION_KEY_ASSERT_ENABLED
        .get_or_init(|| crate::env_flags::env_var_truthy(PROJECTION_KEY_ASSERT_ENV))
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline]
/// Disable projection-key assertions when bridge metrics are compiled out.
pub(super) fn projection_key_assertions_enabled() -> bool {
    false
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

#[cfg(feature = "native-bridge-metrics")]
pub(super) fn maybe_log_bridge_profile() {
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
pub(super) fn maybe_log_bridge_profile() {}

#[cfg(feature = "native-bridge-metrics")]
pub(super) fn trace_pull_model_call() -> u64 {
    PULL_MODEL_COUNT.fetch_add(1, Ordering::Relaxed) + 1
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
pub(super) fn trace_pull_model_call() -> u64 {
    1
}

#[cfg(feature = "native-bridge-metrics")]
pub(super) fn trace_pull_motion_call() -> u64 {
    PULL_MOTION_COUNT.fetch_add(1, Ordering::Relaxed) + 1
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
pub(super) fn trace_pull_motion_call() -> u64 {
    1
}

#[cfg(feature = "native-bridge-metrics")]
pub(super) fn trace_action_call() -> u64 {
    ACTION_COUNT.fetch_add(1, Ordering::Relaxed) + 1
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
pub(super) fn trace_action_call() -> u64 {
    1
}

#[cfg(feature = "native-bridge-metrics")]
pub(super) fn trace_frame_result(result: &NativeFrameBuildResult) -> u64 {
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
pub(super) fn trace_frame_result(_result: &NativeFrameBuildResult) -> u64 {
    1
}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
pub(super) fn trace_pull_model_preparation(duration: Duration) {
    saturating_add_duration(&PULL_MODEL_PREP_NS, duration);
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
pub(super) fn trace_pull_model_preparation(_duration: Duration) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
pub(super) fn trace_pull_model_projection(duration: Duration) {
    saturating_add_duration(&PULL_MODEL_PROJECT_NS, duration);
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
pub(super) fn trace_pull_model_projection(_duration: Duration) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
pub(super) fn trace_pull_motion_preparation(duration: Duration) {
    saturating_add_duration(&PULL_MOTION_PREP_NS, duration);
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
pub(super) fn trace_pull_motion_preparation(_duration: Duration) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
pub(super) fn trace_pull_motion_projection(duration: Duration) {
    saturating_add_duration(&PULL_MOTION_PROJECT_NS, duration);
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
pub(super) fn trace_pull_motion_projection(_duration: Duration) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
pub(super) fn trace_action_duration(duration: Duration) {
    saturating_add_duration(&ACTION_DURATION_NS, duration);
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
pub(super) fn trace_action_duration(_duration: Duration) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
/// Track whether an app-model projection cache lookup hit or missed.
pub(super) fn trace_projection_cache_lookup(hit: bool) {
    if hit {
        PROJECTION_CACHE_HIT_COUNT.fetch_add(1, Ordering::Relaxed);
    } else {
        PROJECTION_CACHE_MISS_COUNT.fetch_add(1, Ordering::Relaxed);
    }
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
/// No-op projection-cache hit/miss tracer for non-profiling builds.
pub(super) fn trace_projection_cache_lookup(_hit: bool) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
/// Track segment-level projection-cache hit/miss decisions.
pub(super) fn trace_projection_segment_lookup(segment: ProjectionSegment, hit: bool) {
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
pub(super) fn trace_projection_segment_lookup(_segment: ProjectionSegment, _hit: bool) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
/// Track classified interaction action timings for bridge profiling logs.
pub(super) fn trace_action_interaction(kind: InteractionActionClass, duration: Duration) {
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
pub(super) fn trace_action_interaction(_kind: InteractionActionClass, _duration: Duration) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
/// Track end-to-end duration and emission count for queued waveform-action flushes.
pub(super) fn trace_waveform_flush(duration: Duration, emitted_actions: u64) {
    WAVEFORM_FLUSH_COUNT.fetch_add(1, Ordering::Relaxed);
    saturating_add_duration(&WAVEFORM_FLUSH_DURATION_NS, duration);
    WAVEFORM_FLUSH_EMITTED_ACTIONS_TOTAL.fetch_add(emitted_actions, Ordering::Relaxed);
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
/// No-op waveform flush tracer for non-profiling builds.
pub(super) fn trace_waveform_flush(_duration: Duration, _emitted_actions: u64) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
/// Track whether waveform image refresh work ran or was skipped as overlay-only.
pub(super) fn trace_waveform_image_refresh(applied: bool) {
    if applied {
        WAVEFORM_IMAGE_REFRESH_APPLY_COUNT.fetch_add(1, Ordering::Relaxed);
    } else {
        WAVEFORM_IMAGE_REFRESH_SKIP_COUNT.fetch_add(1, Ordering::Relaxed);
    }
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
/// No-op waveform image refresh tracer for non-profiling builds.
pub(super) fn trace_waveform_image_refresh(_applied: bool) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
/// Track derived-graph flush timing and dirty-node counts.
pub(super) fn trace_derived_flush(
    duration: Duration,
    dirty_source_count: usize,
    dirty_derived_count: usize,
) {
    DERIVED_FLUSH_COUNT.fetch_add(1, Ordering::Relaxed);
    DERIVED_DIRTY_SOURCE_TOTAL.fetch_add(dirty_source_count as u64, Ordering::Relaxed);
    DERIVED_DIRTY_COMPUTED_TOTAL.fetch_add(dirty_derived_count as u64, Ordering::Relaxed);
    saturating_add_duration(&DERIVED_FLUSH_DURATION_NS, duration);
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
/// No-op derived-graph flush tracer for non-profiling builds.
pub(super) fn trace_derived_flush(
    _duration: Duration,
    _dirty_source_count: usize,
    _dirty_derived_count: usize,
) {
}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
/// Track projection-key snapshot validation checks and stale detections.
pub(super) fn trace_projection_key_assertion(stale: bool) {
    PROJECTION_KEY_ASSERT_COUNT.fetch_add(1, Ordering::Relaxed);
    if stale {
        PROJECTION_KEY_ASSERT_STALE_COUNT.fetch_add(1, Ordering::Relaxed);
    }
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
/// No-op projection-key snapshot assertion tracer for non-profiling builds.
pub(super) fn trace_projection_key_assertion(_stale: bool) {}
