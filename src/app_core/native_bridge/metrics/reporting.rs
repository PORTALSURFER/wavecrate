//! Human-readable reporting and tests for bridge profiling metrics.

#[cfg(feature = "native-bridge-metrics")]
use super::snapshot::BridgeMetricsSnapshot;
#[cfg(feature = "native-bridge-metrics")]
use tracing::info;

#[cfg(feature = "native-bridge-metrics")]
/// Compute average duration milliseconds for a counter total/count pair.
pub(crate) fn avg_ms(total_ns: u64, count: u64) -> f64 {
    if count == 0 {
        0.0
    } else {
        total_ns as f64 / 1_000_000.0 / count as f64
    }
}

#[cfg(feature = "native-bridge-metrics")]
/// Compute average scalar value for a total/count pair.
pub(crate) fn avg_value(total: u64, count: u64) -> f64 {
    if count == 0 {
        0.0
    } else {
        total as f64 / count as f64
    }
}

#[cfg(feature = "native-bridge-metrics")]
/// Compute a bounded ratio with zero-denominator fallback.
pub(crate) fn ratio_value(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

#[cfg(feature = "native-bridge-metrics")]
/// Format a human-readable bridge profiling line from one metrics snapshot.
pub(crate) fn format_bridge_profile_message(snapshot: &BridgeMetricsSnapshot) -> String {
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
pub(crate) fn maybe_log_bridge_profile() {
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
pub(crate) fn maybe_log_bridge_profile() {}

#[cfg(all(test, feature = "native-bridge-metrics"))]
mod tests {
    use super::super::{trace_projection_cache_lookup, trace_projection_segment_lookup};
    use super::*;
    use crate::app_core::native_bridge::projection_cache::ProjectionSegment;

    #[test]
    fn averages_return_zero_for_empty_counts() {
        assert_eq!(avg_ms(10_000, 0), 0.0);
        assert_eq!(avg_value(10, 0), 0.0);
    }

    #[test]
    fn ratio_returns_zero_for_empty_denominator() {
        assert_eq!(ratio_value(5, 0), 0.0);
        assert_eq!(ratio_value(5, 10), 0.5);
    }

    #[test]
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
