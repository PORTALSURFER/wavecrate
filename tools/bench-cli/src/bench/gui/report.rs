//! Result/report types and assembly helpers for GUI benchmark runs.

use super::scenario_registry::GuiScenarioMetrics;
use super::{GuiInteractionRebuildCauseAttribution, GuiInteractionSegmentAttribution, stats};
use serde::Serialize;

/// Synthetic workload configuration and results for GUI frame projection.
#[derive(Clone, Debug, Serialize)]
pub(crate) struct GuiBenchResult {
    /// Number of DB rows seeded into the synthetic source.
    pub(super) seeded_rows: usize,
    /// Latency of native app model projection.
    pub(super) app_model_projection: stats::LatencySummary,
    /// Retained-runtime app-model projection p95 measured through the bridge cache path.
    pub(super) retained_app_model_projection_p95_us: u64,
    /// Latency of native motion model projection.
    pub(super) motion_model_projection: stats::LatencySummary,
    /// Latency of a small UI mutation plus projection sequence.
    pub(super) interactive_projection: stats::LatencySummary,
    /// Latency of pointer-hover style row focus changes with projection.
    pub(super) hover_latency: stats::LatencySummary,
    /// Latency of wheel-like row nudges with projection.
    pub(super) wheel_latency: stats::LatencySummary,
    /// Latency of filter-only browser recompute churn.
    pub(super) browser_filter_churn_latency: stats::LatencySummary,
    /// Latency of query-only browser recompute churn.
    pub(super) browser_query_churn_latency: stats::LatencySummary,
    /// Latency of sort-only browser recompute churn.
    pub(super) browser_sort_toggle_latency: stats::LatencySummary,
    /// Latency of browser-row preview focus navigation.
    pub(super) browser_focus_preview_latency: stats::LatencySummary,
    /// Latency of browser-row commit actions after preview focus.
    pub(super) browser_focus_commit_latency: stats::LatencySummary,
    /// Latency of map pan/zoom state changes through model projection.
    pub(super) map_pan_proxy_latency: stats::LatencySummary,
    /// Latency of waveform interaction actions through projection.
    pub(super) waveform_interaction_latency: stats::LatencySummary,
    /// Latency of continuous top-bar volume drag updates through projection.
    pub(super) volume_drag_latency: stats::LatencySummary,
    /// Latency of idle waveform-cursor updates through motion-only projection.
    pub(super) idle_cursor_motion_latency: stats::LatencySummary,
    /// Latency of adjacent waveform pan/zoom interactions.
    pub(super) waveform_pan_zoom_adjacent_latency: stats::LatencySummary,
    /// Stage-attributed latency summaries keyed by interaction scenario.
    pub(super) interaction_stage_attribution: GuiInteractionStageAttribution,
    /// Segment-attributed latency and counter summaries for retained projection slices.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) interaction_segment_attribution: Option<GuiInteractionSegmentAttribution>,
    /// Rebuild-cause attribution proxies for interaction scenarios.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) interaction_rebuild_cause_attribution: Option<GuiInteractionRebuildCauseAttribution>,
}

/// Stage-attributed interaction latency summaries keyed by scenario.
#[derive(Clone, Debug, Serialize)]
pub(super) struct GuiInteractionStageAttribution {
    /// Stage-attributed latency for mixed browser interaction-step churn.
    pub(super) interactive_projection: stats::StageLatencyBreakdown,
    /// Stage-attributed latency for pointer-hover style row focus changes.
    pub(super) hover_latency: stats::StageLatencyBreakdown,
    /// Stage-attributed latency for wheel-like row nudges.
    pub(super) wheel_latency: stats::StageLatencyBreakdown,
    /// Stage-attributed latency for filter-only browser churn.
    pub(super) browser_filter_churn_latency: stats::StageLatencyBreakdown,
    /// Stage-attributed latency for query-only browser churn.
    pub(super) browser_query_churn_latency: stats::StageLatencyBreakdown,
    /// Stage-attributed latency for sort-only browser churn.
    pub(super) browser_sort_toggle_latency: stats::StageLatencyBreakdown,
    /// Stage-attributed latency for browser-row preview focus navigation.
    pub(super) browser_focus_preview_latency: stats::StageLatencyBreakdown,
    /// Stage-attributed latency for browser-row commit actions.
    pub(super) browser_focus_commit_latency: stats::StageLatencyBreakdown,
    /// Stage-attributed latency for map pan/zoom proxy updates.
    pub(super) map_pan_proxy_latency: stats::StageLatencyBreakdown,
    /// Stage-attributed latency for waveform interaction actions.
    pub(super) waveform_interaction_latency: stats::StageLatencyBreakdown,
    /// Stage-attributed latency for volume drag interactions.
    pub(super) volume_drag_latency: stats::StageLatencyBreakdown,
    /// Stage-attributed latency for idle waveform-cursor motion updates.
    pub(super) idle_cursor_motion_latency: stats::StageLatencyBreakdown,
}

/// Assemble the serialized GUI benchmark report from collected scenario metrics.
pub(super) fn assemble_gui_bench_result(
    seeded_rows: usize,
    scenario_metrics: GuiScenarioMetrics,
    interaction_segment_attribution: Option<GuiInteractionSegmentAttribution>,
    interaction_rebuild_cause_attribution: Option<GuiInteractionRebuildCauseAttribution>,
) -> GuiBenchResult {
    let GuiScenarioMetrics {
        app_model_projection,
        retained_app_model_projection_p95_us,
        motion_model_projection,
        interactive_projection,
        hover_latency,
        wheel_latency,
        browser_filter_churn_latency,
        browser_query_churn_latency,
        browser_sort_toggle_latency,
        browser_focus_preview_latency,
        browser_focus_commit_latency,
        map_pan_proxy_latency,
        waveform_interaction_latency,
        volume_drag_latency,
        idle_cursor_motion_latency,
        waveform_pan_zoom_adjacent_latency,
    } = scenario_metrics;
    let interactive_projection = split_staged_summary(interactive_projection);
    let hover_latency = split_staged_summary(hover_latency);
    let wheel_latency = split_staged_summary(wheel_latency);
    let browser_filter_churn_latency = split_staged_summary(browser_filter_churn_latency);
    let browser_query_churn_latency = split_staged_summary(browser_query_churn_latency);
    let browser_sort_toggle_latency = split_staged_summary(browser_sort_toggle_latency);
    let browser_focus_preview_latency = split_staged_summary(browser_focus_preview_latency);
    let browser_focus_commit_latency = split_staged_summary(browser_focus_commit_latency);
    let map_pan_proxy_latency = split_staged_summary(map_pan_proxy_latency);
    let waveform_interaction_latency = split_staged_summary(waveform_interaction_latency);
    let volume_drag_latency = split_staged_summary(volume_drag_latency);
    let idle_cursor_motion_latency = split_staged_summary(idle_cursor_motion_latency);
    GuiBenchResult {
        seeded_rows,
        app_model_projection,
        retained_app_model_projection_p95_us,
        motion_model_projection,
        interactive_projection: interactive_projection.total,
        hover_latency: hover_latency.total,
        wheel_latency: wheel_latency.total,
        browser_filter_churn_latency: browser_filter_churn_latency.total,
        browser_query_churn_latency: browser_query_churn_latency.total,
        browser_sort_toggle_latency: browser_sort_toggle_latency.total,
        browser_focus_preview_latency: browser_focus_preview_latency.total,
        browser_focus_commit_latency: browser_focus_commit_latency.total,
        map_pan_proxy_latency: map_pan_proxy_latency.total,
        waveform_interaction_latency: waveform_interaction_latency.total,
        volume_drag_latency: volume_drag_latency.total,
        idle_cursor_motion_latency: idle_cursor_motion_latency.total,
        waveform_pan_zoom_adjacent_latency,
        interaction_stage_attribution: GuiInteractionStageAttribution {
            interactive_projection: interactive_projection.stages,
            hover_latency: hover_latency.stages,
            wheel_latency: wheel_latency.stages,
            browser_filter_churn_latency: browser_filter_churn_latency.stages,
            browser_query_churn_latency: browser_query_churn_latency.stages,
            browser_sort_toggle_latency: browser_sort_toggle_latency.stages,
            browser_focus_preview_latency: browser_focus_preview_latency.stages,
            browser_focus_commit_latency: browser_focus_commit_latency.stages,
            map_pan_proxy_latency: map_pan_proxy_latency.stages,
            waveform_interaction_latency: waveform_interaction_latency.stages,
            volume_drag_latency: volume_drag_latency.stages,
            idle_cursor_motion_latency: idle_cursor_motion_latency.stages,
        },
        interaction_segment_attribution,
        interaction_rebuild_cause_attribution,
    }
}

/// Split a staged latency summary into its serialized total and attribution parts.
fn split_staged_summary(summary: stats::StagedLatencySummary) -> SplitStagedSummary {
    let stats::StagedLatencySummary { total, stages } = summary;
    SplitStagedSummary { total, stages }
}

/// Paired total/stage view of a staged latency summary used during report assembly.
struct SplitStagedSummary {
    /// Aggregate latency summary across the measured iterations.
    total: stats::LatencySummary,
    /// Per-stage latency attribution for the same measured iterations.
    stages: stats::StageLatencyBreakdown,
}
