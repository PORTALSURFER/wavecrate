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
/// Captured bridge metrics counters for one profile log emission.
struct BridgeMetricsSnapshot {
    pull_model_count: u64,
    pull_model_prep_ns: u64,
    pull_model_project_ns: u64,
    pull_motion_count: u64,
    pull_motion_prep_ns: u64,
    pull_motion_project_ns: u64,
    action_count: u64,
    action_duration_ns: u64,
    projection_cache_hit_count: u64,
    projection_cache_miss_count: u64,
    status_segment_hit_count: u64,
    status_segment_miss_count: u64,
    browser_frame_segment_hit_count: u64,
    browser_frame_segment_miss_count: u64,
    browser_rows_segment_hit_count: u64,
    browser_rows_segment_miss_count: u64,
    map_segment_hit_count: u64,
    map_segment_miss_count: u64,
    waveform_segment_hit_count: u64,
    waveform_segment_miss_count: u64,
    wheel_count: u64,
    wheel_duration_ns: u64,
    map_proxy_count: u64,
    map_proxy_duration_ns: u64,
    waveform_count: u64,
    waveform_duration_ns: u64,
    volume_count: u64,
    volume_duration_ns: u64,
    waveform_flush_count: u64,
    waveform_flush_duration_ns: u64,
    waveform_flush_emitted_actions: u64,
    waveform_image_refresh_apply_count: u64,
    waveform_image_refresh_skip_count: u64,
    derived_flush_count: u64,
    derived_flush_duration_ns: u64,
    derived_dirty_source_total: u64,
    derived_dirty_computed_total: u64,
    frame_count: u64,
    frame_anim_count: u64,
    primitive_sum: u64,
    text_run_sum: u64,
    presented_frame_count: u64,
    missed_present_count: u64,
    jank_count: u64,
    frame_total_us: u64,
    present_total_us: u64,
    frame_budget_us: u64,
    browser_row_cache_hit_count: u64,
    browser_row_cache_miss_count: u64,
    projection_key_assert_count: u64,
    projection_key_assert_stale_count: u64,
}

#[cfg(feature = "native-bridge-metrics")]
impl BridgeMetricsSnapshot {
    /// Snapshot process-lifetime bridge counters for one profile log point.
    fn capture() -> Self {
        let (browser_row_cache_hit_count, browser_row_cache_miss_count) =
            native_shell::browser_row_cache_lookup_counts();
        Self {
            pull_model_count: BRIDGE_METRICS.pull_model_count.load(Ordering::Relaxed),
            pull_model_prep_ns: BRIDGE_METRICS.pull_model_prep_ns.load(Ordering::Relaxed),
            pull_model_project_ns: BRIDGE_METRICS.pull_model_project_ns.load(Ordering::Relaxed),
            pull_motion_count: BRIDGE_METRICS.pull_motion_count.load(Ordering::Relaxed),
            pull_motion_prep_ns: BRIDGE_METRICS.pull_motion_prep_ns.load(Ordering::Relaxed),
            pull_motion_project_ns: BRIDGE_METRICS
                .pull_motion_project_ns
                .load(Ordering::Relaxed),
            action_count: BRIDGE_METRICS.action_count.load(Ordering::Relaxed),
            action_duration_ns: BRIDGE_METRICS.action_duration_ns.load(Ordering::Relaxed),
            projection_cache_hit_count: PROJECTION_CACHE_HIT_COUNT.load(Ordering::Relaxed),
            projection_cache_miss_count: PROJECTION_CACHE_MISS_COUNT.load(Ordering::Relaxed),
            status_segment_hit_count: BRIDGE_METRICS
                .projection_status_segment_hit_count
                .load(Ordering::Relaxed),
            status_segment_miss_count: BRIDGE_METRICS
                .projection_status_segment_miss_count
                .load(Ordering::Relaxed),
            browser_frame_segment_hit_count: BRIDGE_METRICS
                .projection_browser_frame_segment_hit_count
                .load(Ordering::Relaxed),
            browser_frame_segment_miss_count: BRIDGE_METRICS
                .projection_browser_frame_segment_miss_count
                .load(Ordering::Relaxed),
            browser_rows_segment_hit_count: BRIDGE_METRICS
                .projection_browser_rows_segment_hit_count
                .load(Ordering::Relaxed),
            browser_rows_segment_miss_count: BRIDGE_METRICS
                .projection_browser_rows_segment_miss_count
                .load(Ordering::Relaxed),
            map_segment_hit_count: BRIDGE_METRICS
                .projection_map_segment_hit_count
                .load(Ordering::Relaxed),
            map_segment_miss_count: BRIDGE_METRICS
                .projection_map_segment_miss_count
                .load(Ordering::Relaxed),
            waveform_segment_hit_count: BRIDGE_METRICS
                .projection_waveform_segment_hit_count
                .load(Ordering::Relaxed),
            waveform_segment_miss_count: BRIDGE_METRICS
                .projection_waveform_segment_miss_count
                .load(Ordering::Relaxed),
            wheel_count: BRIDGE_METRICS.action_wheel_count.load(Ordering::Relaxed),
            wheel_duration_ns: BRIDGE_METRICS
                .action_wheel_duration_ns
                .load(Ordering::Relaxed),
            map_proxy_count: BRIDGE_METRICS
                .action_map_proxy_count
                .load(Ordering::Relaxed),
            map_proxy_duration_ns: BRIDGE_METRICS
                .action_map_proxy_duration_ns
                .load(Ordering::Relaxed),
            waveform_count: BRIDGE_METRICS.action_waveform_count.load(Ordering::Relaxed),
            waveform_duration_ns: BRIDGE_METRICS
                .action_waveform_duration_ns
                .load(Ordering::Relaxed),
            volume_count: BRIDGE_METRICS.action_volume_count.load(Ordering::Relaxed),
            volume_duration_ns: BRIDGE_METRICS
                .action_volume_duration_ns
                .load(Ordering::Relaxed),
            waveform_flush_count: BRIDGE_METRICS.waveform_flush_count.load(Ordering::Relaxed),
            waveform_flush_duration_ns: BRIDGE_METRICS
                .waveform_flush_duration_ns
                .load(Ordering::Relaxed),
            waveform_flush_emitted_actions: BRIDGE_METRICS
                .waveform_flush_emitted_actions_total
                .load(Ordering::Relaxed),
            waveform_image_refresh_apply_count: WAVEFORM_IMAGE_REFRESH_APPLY_COUNT
                .load(Ordering::Relaxed),
            waveform_image_refresh_skip_count: WAVEFORM_IMAGE_REFRESH_SKIP_COUNT
                .load(Ordering::Relaxed),
            derived_flush_count: BRIDGE_METRICS.derived_flush_count.load(Ordering::Relaxed),
            derived_flush_duration_ns: BRIDGE_METRICS
                .derived_flush_duration_ns
                .load(Ordering::Relaxed),
            derived_dirty_source_total: BRIDGE_METRICS
                .derived_dirty_source_total
                .load(Ordering::Relaxed),
            derived_dirty_computed_total: BRIDGE_METRICS
                .derived_dirty_computed_total
                .load(Ordering::Relaxed),
            frame_count: BRIDGE_METRICS.frame_result_count.load(Ordering::Relaxed),
            frame_anim_count: BRIDGE_METRICS
                .frame_result_animation_count
                .load(Ordering::Relaxed),
            primitive_sum: BRIDGE_METRICS
                .frame_result_primitives_total
                .load(Ordering::Relaxed),
            text_run_sum: BRIDGE_METRICS
                .frame_result_text_runs_total
                .load(Ordering::Relaxed),
            presented_frame_count: BRIDGE_METRICS
                .frame_result_presented_count
                .load(Ordering::Relaxed),
            missed_present_count: BRIDGE_METRICS
                .frame_result_missed_present_count
                .load(Ordering::Relaxed),
            jank_count: BRIDGE_METRICS
                .frame_result_jank_count
                .load(Ordering::Relaxed),
            frame_total_us: BRIDGE_METRICS.frame_result_total_us.load(Ordering::Relaxed),
            present_total_us: BRIDGE_METRICS
                .frame_result_present_us_total
                .load(Ordering::Relaxed),
            frame_budget_us: BRIDGE_METRICS
                .frame_result_frame_budget_us
                .load(Ordering::Relaxed),
            browser_row_cache_hit_count,
            browser_row_cache_miss_count,
            projection_key_assert_count: BRIDGE_METRICS
                .projection_key_assert_count
                .load(Ordering::Relaxed),
            projection_key_assert_stale_count: BRIDGE_METRICS
                .projection_key_assert_stale_count
                .load(Ordering::Relaxed),
        }
    }
}

#[cfg(feature = "native-bridge-metrics")]
/// Compute average duration milliseconds for a counter total/count pair.
fn avg_ms(total_ns: u64, count: u64) -> f64 {
    if count == 0 {
        0.0
    } else {
        ms_from_ns(total_ns) / count as f64
    }
}

#[cfg(feature = "native-bridge-metrics")]
/// Compute average scalar value for a total/count pair.
fn avg_value(total: u64, count: u64) -> f64 {
    if count == 0 {
        0.0
    } else {
        total as f64 / count as f64
    }
}

#[cfg(feature = "native-bridge-metrics")]
/// Compute a bounded ratio with zero-denominator fallback.
fn ratio_value(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

#[cfg(feature = "native-bridge-metrics")]
/// Format a human-readable bridge profiling line from one metrics snapshot.
fn format_bridge_profile_message(snapshot: &BridgeMetricsSnapshot) -> String {
    format!(
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
        avg_ms(snapshot.pull_model_prep_ns, snapshot.pull_model_count),
        avg_ms(snapshot.pull_model_project_ns, snapshot.pull_model_count),
        avg_ms(snapshot.pull_motion_prep_ns, snapshot.pull_motion_count),
        avg_ms(snapshot.pull_motion_project_ns, snapshot.pull_motion_count),
        avg_ms(snapshot.action_duration_ns, snapshot.action_count),
        snapshot.projection_cache_hit_count,
        snapshot.projection_cache_miss_count,
        snapshot.status_segment_hit_count,
        snapshot.status_segment_miss_count,
        snapshot.browser_frame_segment_hit_count,
        snapshot.browser_frame_segment_miss_count,
        snapshot.browser_rows_segment_hit_count,
        snapshot.browser_rows_segment_miss_count,
        snapshot.map_segment_hit_count,
        snapshot.map_segment_miss_count,
        snapshot.waveform_segment_hit_count,
        snapshot.waveform_segment_miss_count,
        avg_ms(snapshot.wheel_duration_ns, snapshot.wheel_count),
        avg_ms(snapshot.map_proxy_duration_ns, snapshot.map_proxy_count),
        avg_ms(snapshot.waveform_duration_ns, snapshot.waveform_count),
        avg_ms(snapshot.volume_duration_ns, snapshot.volume_count),
        avg_ms(
            snapshot.waveform_flush_duration_ns,
            snapshot.waveform_flush_count
        ),
        avg_value(
            snapshot.waveform_flush_emitted_actions,
            snapshot.waveform_flush_count
        ),
        snapshot.waveform_image_refresh_apply_count,
        snapshot.waveform_image_refresh_skip_count,
        avg_ms(
            snapshot.derived_flush_duration_ns,
            snapshot.derived_flush_count
        ),
        avg_value(
            snapshot.derived_dirty_source_total,
            snapshot.derived_flush_count
        ),
        avg_value(
            snapshot.derived_dirty_computed_total,
            snapshot.derived_flush_count
        ),
        avg_value(snapshot.primitive_sum, snapshot.frame_count),
        avg_value(snapshot.text_run_sum, snapshot.frame_count),
        avg_value(snapshot.frame_total_us, snapshot.frame_count) / 1000.0,
        avg_value(snapshot.present_total_us, snapshot.presented_frame_count) / 1000.0,
        snapshot.frame_budget_us,
        snapshot.browser_row_cache_hit_count,
        snapshot.browser_row_cache_miss_count,
        snapshot.projection_key_assert_count,
        snapshot.projection_key_assert_stale_count,
        snapshot.jank_count,
        ratio_value(snapshot.jank_count, snapshot.frame_count),
        snapshot.missed_present_count,
        ratio_value(snapshot.missed_present_count, snapshot.frame_count)
    )
}

#[cfg(feature = "native-bridge-metrics")]
/// Emit one profile log line containing derived bridge metrics summaries.
pub(super) fn maybe_log_bridge_profile() {
    let snapshot = BridgeMetricsSnapshot::capture();
    info!(
        pull_model_count = snapshot.pull_model_count,
        pull_motion_count = snapshot.pull_motion_count,
        action_count = snapshot.action_count,
        wheel_count = snapshot.wheel_count,
        map_proxy_count = snapshot.map_proxy_count,
        waveform_count = snapshot.waveform_count,
        volume_count = snapshot.volume_count,
        frame_count = snapshot.frame_count,
        frame_anim_count = snapshot.frame_anim_count,
        "{}",
        format_bridge_profile_message(&snapshot)
    );
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline]
pub(super) fn maybe_log_bridge_profile() {}

#[cfg(all(test, feature = "native-bridge-metrics"))]
/// Unit tests for bridge metrics profile math helpers.
mod profile_math_tests {
    use super::*;

    #[test]
    /// Average helpers should return zero for empty sample counts.
    fn averages_return_zero_for_empty_counts() {
        assert_eq!(avg_ms(10_000, 0), 0.0);
        assert_eq!(avg_value(10, 0), 0.0);
    }

    #[test]
    /// Ratio helper should return zero when denominator is zero.
    fn ratio_returns_zero_for_empty_denominator() {
        assert_eq!(ratio_value(5, 0), 0.0);
        assert_eq!(ratio_value(5, 10), 0.5);
    }

    #[test]
    /// Profile message formatting should include cache/segment counters and key ratios.
    fn profile_message_includes_projection_cache_and_segment_fields() {
        let baseline = BridgeMetricsSnapshot::capture();
        let snapshot = BridgeMetricsSnapshot {
            pull_model_count: 2,
            pull_model_prep_ns: 4_000_000,
            pull_model_project_ns: 6_000_000,
            pull_motion_count: 2,
            pull_motion_prep_ns: 8_000_000,
            pull_motion_project_ns: 10_000_000,
            action_count: 2,
            action_duration_ns: 12_000_000,
            projection_cache_hit_count: 11,
            projection_cache_miss_count: 4,
            status_segment_hit_count: 3,
            status_segment_miss_count: 1,
            browser_frame_segment_hit_count: 5,
            browser_frame_segment_miss_count: 2,
            browser_rows_segment_hit_count: 7,
            browser_rows_segment_miss_count: 4,
            map_segment_hit_count: 9,
            map_segment_miss_count: 6,
            waveform_segment_hit_count: 11,
            waveform_segment_miss_count: 8,
            browser_row_cache_hit_count: 9,
            browser_row_cache_miss_count: 2,
            projection_key_assert_count: 12,
            projection_key_assert_stale_count: 3,
            frame_count: 10,
            jank_count: 2,
            missed_present_count: 1,
            ..baseline
        };

        let message = format_bridge_profile_message(&snapshot);
        assert!(message.contains("projection_cache hits=11 misses=4"));
        assert!(message.contains(
            "segments status(h/m)=3/1 browser_frame(h/m)=5/2 browser_rows(h/m)=7/4 map(h/m)=9/6 waveform(h/m)=11/8"
        ));
        assert!(message.contains("browser_row_cache hits=9 misses=2"));
        assert!(
            message.contains("projection_key_assert_count=12 projection_key_assert_stale_count=3")
        );
        assert!(message.contains(
            "jank_count=2 jank_ratio=0.200 missed_present_count=1 missed_present_ratio=0.100"
        ));
    }

    #[test]
    /// Projection cache/segment trace calls should increment the captured counters.
    fn trace_projection_cache_counters_increment_snapshot() {
        let before = BridgeMetricsSnapshot::capture();
        trace_projection_cache_lookup(true);
        trace_projection_cache_lookup(false);
        trace_projection_segment_lookup(ProjectionSegment::StatusBar, true);
        trace_projection_segment_lookup(ProjectionSegment::BrowserRowsWindow, false);
        let after = BridgeMetricsSnapshot::capture();

        assert!(after.projection_cache_hit_count >= before.projection_cache_hit_count + 1);
        assert!(after.projection_cache_miss_count >= before.projection_cache_miss_count + 1);
        assert!(after.status_segment_hit_count >= before.status_segment_hit_count + 1);
        assert!(
            after.browser_rows_segment_miss_count >= before.browser_rows_segment_miss_count + 1
        );
    }
}

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
