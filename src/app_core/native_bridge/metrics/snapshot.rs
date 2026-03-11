//! Snapshot capture for bridge profiling counters.

#[cfg(feature = "native-bridge-metrics")]
use super::registry::{
    BRIDGE_METRICS, PROJECTION_CACHE_HIT_COUNT, PROJECTION_CACHE_MISS_COUNT,
    WAVEFORM_IMAGE_REFRESH_APPLY_COUNT, WAVEFORM_IMAGE_REFRESH_SKIP_COUNT,
};
#[cfg(feature = "native-bridge-metrics")]
use crate::app_core::native_shell;
#[cfg(feature = "native-bridge-metrics")]
use std::sync::atomic::Ordering;

#[cfg(feature = "native-bridge-metrics")]
/// Captured bridge metrics counters for one profile log emission.
pub(crate) struct BridgeMetricsSnapshot {
    pub(super) pull_model_count: u64,
    pub(super) pull_model_prep_ns: u64,
    pub(super) pull_model_project_ns: u64,
    pub(super) pull_motion_count: u64,
    pub(super) pull_motion_prep_ns: u64,
    pub(super) pull_motion_project_ns: u64,
    pub(super) action_count: u64,
    pub(super) action_duration_ns: u64,
    pub(super) projection_cache_hit_count: u64,
    pub(super) projection_cache_miss_count: u64,
    pub(super) status_segment_hit_count: u64,
    pub(super) status_segment_miss_count: u64,
    pub(super) browser_frame_segment_hit_count: u64,
    pub(super) browser_frame_segment_miss_count: u64,
    pub(super) browser_rows_segment_hit_count: u64,
    pub(super) browser_rows_segment_miss_count: u64,
    pub(super) map_segment_hit_count: u64,
    pub(super) map_segment_miss_count: u64,
    pub(super) waveform_segment_hit_count: u64,
    pub(super) waveform_segment_miss_count: u64,
    pub(super) wheel_count: u64,
    pub(super) wheel_duration_ns: u64,
    pub(super) map_proxy_count: u64,
    pub(super) map_proxy_duration_ns: u64,
    pub(super) waveform_count: u64,
    pub(super) waveform_duration_ns: u64,
    pub(super) volume_count: u64,
    pub(super) volume_duration_ns: u64,
    pub(super) waveform_flush_count: u64,
    pub(super) waveform_flush_duration_ns: u64,
    pub(super) waveform_flush_emitted_actions: u64,
    pub(super) waveform_image_refresh_apply_count: u64,
    pub(super) waveform_image_refresh_skip_count: u64,
    pub(super) derived_flush_count: u64,
    pub(super) derived_flush_duration_ns: u64,
    pub(super) derived_dirty_source_total: u64,
    pub(super) derived_dirty_computed_total: u64,
    pub(super) frame_count: u64,
    pub(super) frame_anim_count: u64,
    pub(super) primitive_sum: u64,
    pub(super) text_run_sum: u64,
    pub(super) presented_frame_count: u64,
    pub(super) missed_present_count: u64,
    pub(super) jank_count: u64,
    pub(super) frame_total_us: u64,
    pub(super) present_total_us: u64,
    pub(super) frame_budget_us: u64,
    pub(super) browser_row_cache_hit_count: u64,
    pub(super) browser_row_cache_miss_count: u64,
    pub(super) projection_key_assert_count: u64,
    pub(super) projection_key_assert_stale_count: u64,
}

#[cfg(feature = "native-bridge-metrics")]
impl BridgeMetricsSnapshot {
    /// Snapshot process-lifetime bridge counters for one profile log point.
    pub(crate) fn capture() -> Self {
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
