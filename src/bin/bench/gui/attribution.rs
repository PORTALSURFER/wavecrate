//! Attribution payloads emitted by GUI benchmark reports.

use sempal::app_core::native_bridge::ProjectionRebuildCauseCounts;
use serde::Serialize;

/// Segment-level latency/counter summary emitted in benchmark reports.
#[derive(Clone, Debug, Serialize)]
pub(in crate::bench) struct SegmentAttributionSummary {
    /// Segment-level cache hit count when available.
    pub(in crate::bench) hit_count: u64,
    /// Segment-level cache miss count when available.
    pub(in crate::bench) miss_count: u64,
    /// Segment-level p95 latency proxy in microseconds.
    pub(in crate::bench) p95_us: u64,
}

/// Segment-attributed benchmark summaries keyed by projection segment.
#[derive(Clone, Debug, Serialize)]
pub(in crate::bench) struct GuiInteractionSegmentAttribution {
    /// Status-bar segment summary.
    pub(in crate::bench) status_bar: SegmentAttributionSummary,
    /// Browser-frame metadata/chrome segment summary.
    pub(in crate::bench) browser_frame: SegmentAttributionSummary,
    /// Browser row-window segment summary.
    pub(in crate::bench) browser_rows_window: SegmentAttributionSummary,
    /// Map-panel segment summary.
    pub(in crate::bench) map_panel: SegmentAttributionSummary,
    /// Waveform overlay/panel segment summary.
    pub(in crate::bench) waveform_overlay: SegmentAttributionSummary,
}

/// Rebuild-cause counters emitted for one interaction scenario.
#[derive(Clone, Debug, Serialize)]
pub(in crate::bench) struct RebuildCauseAttributionSummary {
    /// Explicit static invalidations observed in this scenario.
    pub(in crate::bench) explicit_static_rebuild_count: u64,
    /// Dirty-mask-driven static rebuilds observed in this scenario.
    pub(in crate::bench) dirty_mask_static_rebuild_count: u64,
    /// Model-pull refresh rebuild count observed in this scenario.
    pub(in crate::bench) bridge_model_pull_rebuild_count: u64,
    /// Motion-pull refresh rebuild count observed in this scenario.
    pub(in crate::bench) bridge_motion_pull_rebuild_count: u64,
    /// Motion pulls that changed waveform-motion layer fields.
    pub(in crate::bench) waveform_motion_pull_rebuild_count: u64,
    /// Motion pulls that changed chrome-motion layer fields.
    pub(in crate::bench) chrome_motion_pull_rebuild_count: u64,
}

/// Rebuild-cause attribution summaries keyed by GUI interaction scenario.
#[derive(Clone, Debug, Serialize)]
pub(in crate::bench) struct GuiInteractionRebuildCauseAttribution {
    /// Mixed browser interaction-step churn.
    pub(in crate::bench) interactive_projection: RebuildCauseAttributionSummary,
    /// Pointer-hover style row focus changes.
    pub(in crate::bench) hover_latency: RebuildCauseAttributionSummary,
    /// Wheel-like row nudges.
    pub(in crate::bench) wheel_latency: RebuildCauseAttributionSummary,
    /// Filter-only browser churn.
    pub(in crate::bench) browser_filter_churn_latency: RebuildCauseAttributionSummary,
    /// Query-only browser churn.
    pub(in crate::bench) browser_query_churn_latency: RebuildCauseAttributionSummary,
    /// Sort-only browser churn.
    pub(in crate::bench) browser_sort_toggle_latency: RebuildCauseAttributionSummary,
    /// Browser-row preview focus navigation.
    pub(in crate::bench) browser_focus_preview_latency: RebuildCauseAttributionSummary,
    /// Browser-row commit actions.
    pub(in crate::bench) browser_focus_commit_latency: RebuildCauseAttributionSummary,
    /// Map pan/zoom proxy updates.
    pub(in crate::bench) map_pan_proxy_latency: RebuildCauseAttributionSummary,
    /// Waveform interaction actions.
    pub(in crate::bench) waveform_interaction_latency: RebuildCauseAttributionSummary,
    /// Volume drag interactions.
    pub(in crate::bench) volume_drag_latency: RebuildCauseAttributionSummary,
    /// Idle waveform-cursor motion interactions.
    pub(in crate::bench) idle_cursor_motion_latency: RebuildCauseAttributionSummary,
    /// Adjacent waveform pan/zoom interactions.
    pub(in crate::bench) waveform_pan_zoom_adjacent_latency: RebuildCauseAttributionSummary,
}

/// Convert measured rebuild-cause probe counters to report summary shape.
pub(in crate::bench) fn rebuild_cause_summary_from_counts(
    counts: ProjectionRebuildCauseCounts,
) -> RebuildCauseAttributionSummary {
    RebuildCauseAttributionSummary {
        explicit_static_rebuild_count: counts.explicit_static_rebuild_count,
        dirty_mask_static_rebuild_count: counts.dirty_mask_static_rebuild_count,
        bridge_model_pull_rebuild_count: counts.bridge_model_pull_rebuild_count,
        bridge_motion_pull_rebuild_count: counts.bridge_motion_pull_rebuild_count,
        waveform_motion_pull_rebuild_count: counts.waveform_motion_pull_rebuild_count,
        chrome_motion_pull_rebuild_count: counts.chrome_motion_pull_rebuild_count,
    }
}
