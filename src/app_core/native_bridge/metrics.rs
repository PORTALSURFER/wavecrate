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
/// Total number of projection-cache lookups that reused a cached model.
pub(super) static PROJECTION_CACHE_HIT_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Total number of projection-cache lookups that required a fresh projection.
pub(super) static PROJECTION_CACHE_MISS_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Total number of waveform image refresh requests applied during derived flush.
pub(super) static WAVEFORM_IMAGE_REFRESH_APPLY_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Total number of waveform image refresh requests skipped as overlay-only.
pub(super) static WAVEFORM_IMAGE_REFRESH_SKIP_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Grouped bridge-metrics counters to reduce registry drift and edit overhead.
struct BridgeMetrics {
    pull_model_count: AtomicU64,
    pull_model_prep_ns: AtomicU64,
    pull_model_project_ns: AtomicU64,
    pull_motion_count: AtomicU64,
    pull_motion_prep_ns: AtomicU64,
    pull_motion_project_ns: AtomicU64,
    action_count: AtomicU64,
    action_duration_ns: AtomicU64,
    projection_status_segment_hit_count: AtomicU64,
    projection_status_segment_miss_count: AtomicU64,
    projection_browser_frame_segment_hit_count: AtomicU64,
    projection_browser_frame_segment_miss_count: AtomicU64,
    projection_browser_rows_segment_hit_count: AtomicU64,
    projection_browser_rows_segment_miss_count: AtomicU64,
    projection_map_segment_hit_count: AtomicU64,
    projection_map_segment_miss_count: AtomicU64,
    projection_waveform_segment_hit_count: AtomicU64,
    projection_waveform_segment_miss_count: AtomicU64,
    action_wheel_count: AtomicU64,
    action_wheel_duration_ns: AtomicU64,
    action_map_proxy_count: AtomicU64,
    action_map_proxy_duration_ns: AtomicU64,
    action_waveform_count: AtomicU64,
    action_waveform_duration_ns: AtomicU64,
    action_volume_count: AtomicU64,
    action_volume_duration_ns: AtomicU64,
    waveform_flush_count: AtomicU64,
    waveform_flush_duration_ns: AtomicU64,
    waveform_flush_emitted_actions_total: AtomicU64,
    derived_flush_count: AtomicU64,
    derived_flush_duration_ns: AtomicU64,
    derived_dirty_source_total: AtomicU64,
    derived_dirty_computed_total: AtomicU64,
    frame_result_count: AtomicU64,
    frame_result_animation_count: AtomicU64,
    frame_result_primitives_total: AtomicU64,
    frame_result_text_runs_total: AtomicU64,
    frame_result_presented_count: AtomicU64,
    frame_result_missed_present_count: AtomicU64,
    frame_result_jank_count: AtomicU64,
    frame_result_total_us: AtomicU64,
    frame_result_present_us_total: AtomicU64,
    frame_result_frame_budget_us: AtomicU64,
    projection_key_assert_count: AtomicU64,
    projection_key_assert_stale_count: AtomicU64,
}

#[cfg(feature = "native-bridge-metrics")]
impl BridgeMetrics {
    /// Build a zeroed bridge-metrics registry for process-lifetime accumulation.
    const fn new() -> Self {
        Self {
            pull_model_count: AtomicU64::new(0),
            pull_model_prep_ns: AtomicU64::new(0),
            pull_model_project_ns: AtomicU64::new(0),
            pull_motion_count: AtomicU64::new(0),
            pull_motion_prep_ns: AtomicU64::new(0),
            pull_motion_project_ns: AtomicU64::new(0),
            action_count: AtomicU64::new(0),
            action_duration_ns: AtomicU64::new(0),
            projection_status_segment_hit_count: AtomicU64::new(0),
            projection_status_segment_miss_count: AtomicU64::new(0),
            projection_browser_frame_segment_hit_count: AtomicU64::new(0),
            projection_browser_frame_segment_miss_count: AtomicU64::new(0),
            projection_browser_rows_segment_hit_count: AtomicU64::new(0),
            projection_browser_rows_segment_miss_count: AtomicU64::new(0),
            projection_map_segment_hit_count: AtomicU64::new(0),
            projection_map_segment_miss_count: AtomicU64::new(0),
            projection_waveform_segment_hit_count: AtomicU64::new(0),
            projection_waveform_segment_miss_count: AtomicU64::new(0),
            action_wheel_count: AtomicU64::new(0),
            action_wheel_duration_ns: AtomicU64::new(0),
            action_map_proxy_count: AtomicU64::new(0),
            action_map_proxy_duration_ns: AtomicU64::new(0),
            action_waveform_count: AtomicU64::new(0),
            action_waveform_duration_ns: AtomicU64::new(0),
            action_volume_count: AtomicU64::new(0),
            action_volume_duration_ns: AtomicU64::new(0),
            waveform_flush_count: AtomicU64::new(0),
            waveform_flush_duration_ns: AtomicU64::new(0),
            waveform_flush_emitted_actions_total: AtomicU64::new(0),
            derived_flush_count: AtomicU64::new(0),
            derived_flush_duration_ns: AtomicU64::new(0),
            derived_dirty_source_total: AtomicU64::new(0),
            derived_dirty_computed_total: AtomicU64::new(0),
            frame_result_count: AtomicU64::new(0),
            frame_result_animation_count: AtomicU64::new(0),
            frame_result_primitives_total: AtomicU64::new(0),
            frame_result_text_runs_total: AtomicU64::new(0),
            frame_result_presented_count: AtomicU64::new(0),
            frame_result_missed_present_count: AtomicU64::new(0),
            frame_result_jank_count: AtomicU64::new(0),
            frame_result_total_us: AtomicU64::new(0),
            frame_result_present_us_total: AtomicU64::new(0),
            frame_result_frame_budget_us: AtomicU64::new(0),
            projection_key_assert_count: AtomicU64::new(0),
            projection_key_assert_stale_count: AtomicU64::new(0),
        }
    }
}

#[cfg(feature = "native-bridge-metrics")]
/// Process-lifetime grouped counter registry used by bridge profiling hooks.
static BRIDGE_METRICS: BridgeMetrics = BridgeMetrics::new();
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
    let pull_model_count = BRIDGE_METRICS.pull_model_count.load(Ordering::Relaxed);
    let pull_model_prep = BRIDGE_METRICS.pull_model_prep_ns.load(Ordering::Relaxed);
    let pull_model_project = BRIDGE_METRICS.pull_model_project_ns.load(Ordering::Relaxed);
    let pull_motion_count = BRIDGE_METRICS.pull_motion_count.load(Ordering::Relaxed);
    let pull_motion_prep = BRIDGE_METRICS.pull_motion_prep_ns.load(Ordering::Relaxed);
    let pull_motion_project = BRIDGE_METRICS
        .pull_motion_project_ns
        .load(Ordering::Relaxed);
    let action_count = BRIDGE_METRICS.action_count.load(Ordering::Relaxed);
    let action_ns = BRIDGE_METRICS.action_duration_ns.load(Ordering::Relaxed);
    let projection_cache_hit_count = PROJECTION_CACHE_HIT_COUNT.load(Ordering::Relaxed);
    let projection_cache_miss_count = PROJECTION_CACHE_MISS_COUNT.load(Ordering::Relaxed);
    let status_segment_hit_count = BRIDGE_METRICS
        .projection_status_segment_hit_count
        .load(Ordering::Relaxed);
    let status_segment_miss_count = BRIDGE_METRICS
        .projection_status_segment_miss_count
        .load(Ordering::Relaxed);
    let browser_frame_segment_hit_count = BRIDGE_METRICS
        .projection_browser_frame_segment_hit_count
        .load(Ordering::Relaxed);
    let browser_frame_segment_miss_count = BRIDGE_METRICS
        .projection_browser_frame_segment_miss_count
        .load(Ordering::Relaxed);
    let browser_rows_segment_hit_count = BRIDGE_METRICS
        .projection_browser_rows_segment_hit_count
        .load(Ordering::Relaxed);
    let browser_rows_segment_miss_count = BRIDGE_METRICS
        .projection_browser_rows_segment_miss_count
        .load(Ordering::Relaxed);
    let map_segment_hit_count = BRIDGE_METRICS
        .projection_map_segment_hit_count
        .load(Ordering::Relaxed);
    let map_segment_miss_count = BRIDGE_METRICS
        .projection_map_segment_miss_count
        .load(Ordering::Relaxed);
    let waveform_segment_hit_count = BRIDGE_METRICS
        .projection_waveform_segment_hit_count
        .load(Ordering::Relaxed);
    let waveform_segment_miss_count = BRIDGE_METRICS
        .projection_waveform_segment_miss_count
        .load(Ordering::Relaxed);
    let wheel_count = BRIDGE_METRICS.action_wheel_count.load(Ordering::Relaxed);
    let wheel_ns = BRIDGE_METRICS
        .action_wheel_duration_ns
        .load(Ordering::Relaxed);
    let map_proxy_count = BRIDGE_METRICS
        .action_map_proxy_count
        .load(Ordering::Relaxed);
    let map_proxy_ns = BRIDGE_METRICS
        .action_map_proxy_duration_ns
        .load(Ordering::Relaxed);
    let waveform_count = BRIDGE_METRICS.action_waveform_count.load(Ordering::Relaxed);
    let waveform_ns = BRIDGE_METRICS
        .action_waveform_duration_ns
        .load(Ordering::Relaxed);
    let volume_count = BRIDGE_METRICS.action_volume_count.load(Ordering::Relaxed);
    let volume_ns = BRIDGE_METRICS
        .action_volume_duration_ns
        .load(Ordering::Relaxed);
    let waveform_flush_count = BRIDGE_METRICS.waveform_flush_count.load(Ordering::Relaxed);
    let waveform_flush_ns = BRIDGE_METRICS
        .waveform_flush_duration_ns
        .load(Ordering::Relaxed);
    let waveform_flush_emitted_actions = BRIDGE_METRICS
        .waveform_flush_emitted_actions_total
        .load(Ordering::Relaxed);
    let waveform_image_refresh_apply_count =
        WAVEFORM_IMAGE_REFRESH_APPLY_COUNT.load(Ordering::Relaxed);
    let waveform_image_refresh_skip_count =
        WAVEFORM_IMAGE_REFRESH_SKIP_COUNT.load(Ordering::Relaxed);
    let derived_flush_count = BRIDGE_METRICS.derived_flush_count.load(Ordering::Relaxed);
    let derived_flush_ns = BRIDGE_METRICS
        .derived_flush_duration_ns
        .load(Ordering::Relaxed);
    let derived_dirty_source_total = BRIDGE_METRICS
        .derived_dirty_source_total
        .load(Ordering::Relaxed);
    let derived_dirty_computed_total = BRIDGE_METRICS
        .derived_dirty_computed_total
        .load(Ordering::Relaxed);
    let frame_count = BRIDGE_METRICS.frame_result_count.load(Ordering::Relaxed);
    let frame_anim_count = BRIDGE_METRICS
        .frame_result_animation_count
        .load(Ordering::Relaxed);
    let primitive_sum = BRIDGE_METRICS
        .frame_result_primitives_total
        .load(Ordering::Relaxed);
    let text_run_sum = BRIDGE_METRICS
        .frame_result_text_runs_total
        .load(Ordering::Relaxed);
    let presented_frame_count = BRIDGE_METRICS
        .frame_result_presented_count
        .load(Ordering::Relaxed);
    let missed_present_count = BRIDGE_METRICS
        .frame_result_missed_present_count
        .load(Ordering::Relaxed);
    let jank_count = BRIDGE_METRICS
        .frame_result_jank_count
        .load(Ordering::Relaxed);
    let frame_total_us = BRIDGE_METRICS.frame_result_total_us.load(Ordering::Relaxed);
    let present_total_us = BRIDGE_METRICS
        .frame_result_present_us_total
        .load(Ordering::Relaxed);
    let frame_budget_us = BRIDGE_METRICS
        .frame_result_frame_budget_us
        .load(Ordering::Relaxed);
    let projection_key_assert_count = BRIDGE_METRICS
        .projection_key_assert_count
        .load(Ordering::Relaxed);
    let projection_key_assert_stale_count = BRIDGE_METRICS
        .projection_key_assert_stale_count
        .load(Ordering::Relaxed);
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
    BRIDGE_METRICS
        .pull_model_count
        .fetch_add(1, Ordering::Relaxed)
        + 1
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
pub(super) fn trace_pull_model_call() -> u64 {
    1
}

#[cfg(feature = "native-bridge-metrics")]
pub(super) fn trace_pull_motion_call() -> u64 {
    BRIDGE_METRICS
        .pull_motion_count
        .fetch_add(1, Ordering::Relaxed)
        + 1
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
pub(super) fn trace_pull_motion_call() -> u64 {
    1
}

#[cfg(feature = "native-bridge-metrics")]
pub(super) fn trace_action_call() -> u64 {
    BRIDGE_METRICS.action_count.fetch_add(1, Ordering::Relaxed) + 1
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
pub(super) fn trace_action_call() -> u64 {
    1
}

#[cfg(feature = "native-bridge-metrics")]
pub(super) fn trace_frame_result(result: &NativeFrameBuildResult) -> u64 {
    let frame_count = BRIDGE_METRICS
        .frame_result_count
        .fetch_add(1, Ordering::Relaxed)
        + 1;
    if result.needs_animation {
        BRIDGE_METRICS
            .frame_result_animation_count
            .fetch_add(1, Ordering::Relaxed);
    }
    if result.presented {
        BRIDGE_METRICS
            .frame_result_presented_count
            .fetch_add(1, Ordering::Relaxed);
    }
    if result.missed_present {
        BRIDGE_METRICS
            .frame_result_missed_present_count
            .fetch_add(1, Ordering::Relaxed);
    }
    if result.jank {
        BRIDGE_METRICS
            .frame_result_jank_count
            .fetch_add(1, Ordering::Relaxed);
    }
    BRIDGE_METRICS
        .frame_result_total_us
        .fetch_add(result.frame_total_us as u64, Ordering::Relaxed);
    BRIDGE_METRICS
        .frame_result_present_us_total
        .fetch_add(result.present_us as u64, Ordering::Relaxed);
    BRIDGE_METRICS
        .frame_result_frame_budget_us
        .store(result.frame_budget_us as u64, Ordering::Relaxed);
    BRIDGE_METRICS
        .frame_result_primitives_total
        .fetch_add(result.primitive_count as u64, Ordering::Relaxed);
    BRIDGE_METRICS
        .frame_result_text_runs_total
        .fetch_add(result.text_run_count as u64, Ordering::Relaxed);
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
    saturating_add_duration(&BRIDGE_METRICS.pull_model_prep_ns, duration);
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
pub(super) fn trace_pull_model_preparation(_duration: Duration) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
pub(super) fn trace_pull_model_projection(duration: Duration) {
    saturating_add_duration(&BRIDGE_METRICS.pull_model_project_ns, duration);
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
pub(super) fn trace_pull_model_projection(_duration: Duration) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
pub(super) fn trace_pull_motion_preparation(duration: Duration) {
    saturating_add_duration(&BRIDGE_METRICS.pull_motion_prep_ns, duration);
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
pub(super) fn trace_pull_motion_preparation(_duration: Duration) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
pub(super) fn trace_pull_motion_projection(duration: Duration) {
    saturating_add_duration(&BRIDGE_METRICS.pull_motion_project_ns, duration);
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
pub(super) fn trace_pull_motion_projection(_duration: Duration) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
pub(super) fn trace_action_duration(duration: Duration) {
    saturating_add_duration(&BRIDGE_METRICS.action_duration_ns, duration);
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
            BRIDGE_METRICS
                .projection_status_segment_hit_count
                .fetch_add(1, Ordering::Relaxed);
        }
        (ProjectionSegment::StatusBar, false) => {
            BRIDGE_METRICS
                .projection_status_segment_miss_count
                .fetch_add(1, Ordering::Relaxed);
        }
        (ProjectionSegment::BrowserFrame, true) => {
            BRIDGE_METRICS
                .projection_browser_frame_segment_hit_count
                .fetch_add(1, Ordering::Relaxed);
        }
        (ProjectionSegment::BrowserFrame, false) => {
            BRIDGE_METRICS
                .projection_browser_frame_segment_miss_count
                .fetch_add(1, Ordering::Relaxed);
        }
        (ProjectionSegment::BrowserRowsWindow, true) => {
            BRIDGE_METRICS
                .projection_browser_rows_segment_hit_count
                .fetch_add(1, Ordering::Relaxed);
        }
        (ProjectionSegment::BrowserRowsWindow, false) => {
            BRIDGE_METRICS
                .projection_browser_rows_segment_miss_count
                .fetch_add(1, Ordering::Relaxed);
        }
        (ProjectionSegment::MapPanel, true) => {
            BRIDGE_METRICS
                .projection_map_segment_hit_count
                .fetch_add(1, Ordering::Relaxed);
        }
        (ProjectionSegment::MapPanel, false) => {
            BRIDGE_METRICS
                .projection_map_segment_miss_count
                .fetch_add(1, Ordering::Relaxed);
        }
        (ProjectionSegment::WaveformOverlay, true) => {
            BRIDGE_METRICS
                .projection_waveform_segment_hit_count
                .fetch_add(1, Ordering::Relaxed);
        }
        (ProjectionSegment::WaveformOverlay, false) => {
            BRIDGE_METRICS
                .projection_waveform_segment_miss_count
                .fetch_add(1, Ordering::Relaxed);
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
            BRIDGE_METRICS
                .action_wheel_count
                .fetch_add(1, Ordering::Relaxed);
            saturating_add_duration(&BRIDGE_METRICS.action_wheel_duration_ns, duration);
        }
        InteractionActionClass::MapPanProxy => {
            BRIDGE_METRICS
                .action_map_proxy_count
                .fetch_add(1, Ordering::Relaxed);
            saturating_add_duration(&BRIDGE_METRICS.action_map_proxy_duration_ns, duration);
        }
        InteractionActionClass::Waveform => {
            BRIDGE_METRICS
                .action_waveform_count
                .fetch_add(1, Ordering::Relaxed);
            saturating_add_duration(&BRIDGE_METRICS.action_waveform_duration_ns, duration);
        }
        InteractionActionClass::Volume => {
            BRIDGE_METRICS
                .action_volume_count
                .fetch_add(1, Ordering::Relaxed);
            saturating_add_duration(&BRIDGE_METRICS.action_volume_duration_ns, duration);
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
    BRIDGE_METRICS
        .waveform_flush_count
        .fetch_add(1, Ordering::Relaxed);
    saturating_add_duration(&BRIDGE_METRICS.waveform_flush_duration_ns, duration);
    BRIDGE_METRICS
        .waveform_flush_emitted_actions_total
        .fetch_add(emitted_actions, Ordering::Relaxed);
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
    BRIDGE_METRICS
        .derived_flush_count
        .fetch_add(1, Ordering::Relaxed);
    BRIDGE_METRICS
        .derived_dirty_source_total
        .fetch_add(dirty_source_count as u64, Ordering::Relaxed);
    BRIDGE_METRICS
        .derived_dirty_computed_total
        .fetch_add(dirty_derived_count as u64, Ordering::Relaxed);
    saturating_add_duration(&BRIDGE_METRICS.derived_flush_duration_ns, duration);
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
    BRIDGE_METRICS
        .projection_key_assert_count
        .fetch_add(1, Ordering::Relaxed);
    if stale {
        BRIDGE_METRICS
            .projection_key_assert_stale_count
            .fetch_add(1, Ordering::Relaxed);
    }
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
/// No-op projection-key snapshot assertion tracer for non-profiling builds.
pub(super) fn trace_projection_key_assertion(_stale: bool) {}
