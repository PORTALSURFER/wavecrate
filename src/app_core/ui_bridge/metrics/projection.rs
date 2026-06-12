//! Projection cache and segment-level hit/miss counters.

use super::super::projection_cache::ProjectionSegment;
#[cfg(feature = "native-bridge-metrics")]
use super::registry::{BRIDGE_METRICS, PROJECTION_CACHE_HIT_COUNT, PROJECTION_CACHE_MISS_COUNT};
#[cfg(feature = "native-bridge-metrics")]
use std::sync::atomic::Ordering;

#[cfg(any(test, feature = "native-bridge-metrics"))]
#[cfg_attr(test, derive(Clone, Copy, Debug, Eq, PartialEq))]
enum ProjectionSegmentMetric {
    StatusHit,
    StatusMiss,
    BrowserFrameHit,
    BrowserFrameMiss,
    BrowserTagSidebarHit,
    BrowserTagSidebarMiss,
    BrowserRowsHit,
    BrowserRowsMiss,
    MapHit,
    MapMiss,
    WaveformHit,
    WaveformMiss,
}

#[cfg(any(test, feature = "native-bridge-metrics"))]
fn projection_segment_metric(segment: ProjectionSegment, hit: bool) -> ProjectionSegmentMetric {
    match (segment, hit) {
        (ProjectionSegment::StatusBar, true) => ProjectionSegmentMetric::StatusHit,
        (ProjectionSegment::StatusBar, false) => ProjectionSegmentMetric::StatusMiss,
        (ProjectionSegment::BrowserFrame, true) => ProjectionSegmentMetric::BrowserFrameHit,
        (ProjectionSegment::BrowserFrame, false) => ProjectionSegmentMetric::BrowserFrameMiss,
        (ProjectionSegment::BrowserTagSidebar, true) => {
            ProjectionSegmentMetric::BrowserTagSidebarHit
        }
        (ProjectionSegment::BrowserTagSidebar, false) => {
            ProjectionSegmentMetric::BrowserTagSidebarMiss
        }
        (ProjectionSegment::BrowserRowsWindow, true) => ProjectionSegmentMetric::BrowserRowsHit,
        (ProjectionSegment::BrowserRowsWindow, false) => ProjectionSegmentMetric::BrowserRowsMiss,
        (ProjectionSegment::MapPanel, true) => ProjectionSegmentMetric::MapHit,
        (ProjectionSegment::MapPanel, false) => ProjectionSegmentMetric::MapMiss,
        (ProjectionSegment::WaveformOverlay, true) => ProjectionSegmentMetric::WaveformHit,
        (ProjectionSegment::WaveformOverlay, false) => ProjectionSegmentMetric::WaveformMiss,
    }
}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
/// Track whether an app-model projection cache lookup hit or missed.
pub(in crate::app_core::ui_bridge) fn trace_projection_cache_lookup(hit: bool) {
    if hit {
        PROJECTION_CACHE_HIT_COUNT.fetch_add(1, Ordering::Relaxed);
    } else {
        PROJECTION_CACHE_MISS_COUNT.fetch_add(1, Ordering::Relaxed);
    }
}

#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
/// No-op projection-cache hit/miss tracer for non-profiling builds.
pub(in crate::app_core::ui_bridge) fn trace_projection_cache_lookup(_hit: bool) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
/// Track segment-level projection-cache hit/miss decisions.
pub(in crate::app_core::ui_bridge) fn trace_projection_segment_lookup(
    segment: ProjectionSegment,
    hit: bool,
) {
    match projection_segment_metric(segment, hit) {
        ProjectionSegmentMetric::StatusHit => {
            BRIDGE_METRICS
                .projection_status_segment_hit_count
                .fetch_add(1, Ordering::Relaxed);
        }
        ProjectionSegmentMetric::StatusMiss => {
            BRIDGE_METRICS
                .projection_status_segment_miss_count
                .fetch_add(1, Ordering::Relaxed);
        }
        ProjectionSegmentMetric::BrowserFrameHit => {
            BRIDGE_METRICS
                .projection_browser_frame_segment_hit_count
                .fetch_add(1, Ordering::Relaxed);
        }
        ProjectionSegmentMetric::BrowserFrameMiss => {
            BRIDGE_METRICS
                .projection_browser_frame_segment_miss_count
                .fetch_add(1, Ordering::Relaxed);
        }
        ProjectionSegmentMetric::BrowserTagSidebarHit => {
            BRIDGE_METRICS
                .projection_browser_tag_sidebar_segment_hit_count
                .fetch_add(1, Ordering::Relaxed);
        }
        ProjectionSegmentMetric::BrowserTagSidebarMiss => {
            BRIDGE_METRICS
                .projection_browser_tag_sidebar_segment_miss_count
                .fetch_add(1, Ordering::Relaxed);
        }
        ProjectionSegmentMetric::BrowserRowsHit => {
            BRIDGE_METRICS
                .projection_browser_rows_segment_hit_count
                .fetch_add(1, Ordering::Relaxed);
        }
        ProjectionSegmentMetric::BrowserRowsMiss => {
            BRIDGE_METRICS
                .projection_browser_rows_segment_miss_count
                .fetch_add(1, Ordering::Relaxed);
        }
        ProjectionSegmentMetric::MapHit => {
            BRIDGE_METRICS
                .projection_map_segment_hit_count
                .fetch_add(1, Ordering::Relaxed);
        }
        ProjectionSegmentMetric::MapMiss => {
            BRIDGE_METRICS
                .projection_map_segment_miss_count
                .fetch_add(1, Ordering::Relaxed);
        }
        ProjectionSegmentMetric::WaveformHit => {
            BRIDGE_METRICS
                .projection_waveform_segment_hit_count
                .fetch_add(1, Ordering::Relaxed);
        }
        ProjectionSegmentMetric::WaveformMiss => {
            BRIDGE_METRICS
                .projection_waveform_segment_miss_count
                .fetch_add(1, Ordering::Relaxed);
        }
    }
}

#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
/// No-op segment-level projection-cache tracer for non-profiling builds.
pub(in crate::app_core::ui_bridge) fn trace_projection_segment_lookup(
    _segment: ProjectionSegment,
    _hit: bool,
) {
}

#[cfg(test)]
mod tests {
    use super::{ProjectionSegmentMetric, projection_segment_metric};
    #[cfg(not(feature = "native-bridge-metrics"))]
    use super::{trace_projection_cache_lookup, trace_projection_segment_lookup};
    use crate::app_core::ui_bridge::projection_cache::ProjectionSegment;

    #[test]
    fn segment_metric_mapping_covers_hit_and_miss_families() {
        let cases = [
            (
                ProjectionSegment::StatusBar,
                true,
                ProjectionSegmentMetric::StatusHit,
            ),
            (
                ProjectionSegment::StatusBar,
                false,
                ProjectionSegmentMetric::StatusMiss,
            ),
            (
                ProjectionSegment::BrowserFrame,
                true,
                ProjectionSegmentMetric::BrowserFrameHit,
            ),
            (
                ProjectionSegment::BrowserFrame,
                false,
                ProjectionSegmentMetric::BrowserFrameMiss,
            ),
            (
                ProjectionSegment::BrowserTagSidebar,
                true,
                ProjectionSegmentMetric::BrowserTagSidebarHit,
            ),
            (
                ProjectionSegment::BrowserTagSidebar,
                false,
                ProjectionSegmentMetric::BrowserTagSidebarMiss,
            ),
            (
                ProjectionSegment::BrowserRowsWindow,
                true,
                ProjectionSegmentMetric::BrowserRowsHit,
            ),
            (
                ProjectionSegment::BrowserRowsWindow,
                false,
                ProjectionSegmentMetric::BrowserRowsMiss,
            ),
            (
                ProjectionSegment::MapPanel,
                true,
                ProjectionSegmentMetric::MapHit,
            ),
            (
                ProjectionSegment::MapPanel,
                false,
                ProjectionSegmentMetric::MapMiss,
            ),
            (
                ProjectionSegment::WaveformOverlay,
                true,
                ProjectionSegmentMetric::WaveformHit,
            ),
            (
                ProjectionSegment::WaveformOverlay,
                false,
                ProjectionSegmentMetric::WaveformMiss,
            ),
        ];

        for (segment, hit, expected_metric) in cases {
            assert_eq!(projection_segment_metric(segment, hit), expected_metric);
        }
    }

    #[cfg(not(feature = "native-bridge-metrics"))]
    #[test]
    fn disabled_feature_projection_tracers_are_noops() {
        trace_projection_cache_lookup(true);
        trace_projection_cache_lookup(false);
        trace_projection_segment_lookup(ProjectionSegment::StatusBar, true);
        trace_projection_segment_lookup(ProjectionSegment::WaveformOverlay, false);
    }
}
