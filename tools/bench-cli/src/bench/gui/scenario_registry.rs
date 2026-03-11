//! Scenario registry for the GUI benchmark harness.

use super::interactions::{
    bench_browser_filter_churn_latency, bench_browser_focus_commit_latency,
    bench_browser_focus_preview_latency, bench_browser_query_churn_latency,
    bench_browser_sort_toggle_latency, bench_hover_latency, bench_idle_cursor_motion_latency,
    bench_map_pan_proxy_latency, bench_volume_drag_latency, bench_waveform_interactions,
    bench_waveform_pan_zoom_adjacent_latency, bench_wheel_latency,
};
use super::{BenchOptions, stats};
use sempal::app_core::actions::{NativeAppModel, NativeMotionModel};
use sempal::app_core::controller::{AppController, AppControllerNativeRuntimeExt};

/// Latency summaries collected for every GUI benchmark scenario.
pub(super) struct GuiScenarioMetrics {
    /// Latency of native app model projection.
    pub(super) app_model_projection: stats::LatencySummary,
    /// Latency of native motion model projection.
    pub(super) motion_model_projection: stats::LatencySummary,
    /// Latency of a mixed interactive projection step sequence.
    pub(super) interactive_projection: stats::StagedLatencySummary,
    /// Latency of hover-style browser focus updates.
    pub(super) hover_latency: stats::StagedLatencySummary,
    /// Latency of wheel-like browser focus updates.
    pub(super) wheel_latency: stats::StagedLatencySummary,
    /// Latency of filter-only browser recompute churn.
    pub(super) browser_filter_churn_latency: stats::StagedLatencySummary,
    /// Latency of query-only browser recompute churn.
    pub(super) browser_query_churn_latency: stats::StagedLatencySummary,
    /// Latency of sort-only browser recompute churn.
    pub(super) browser_sort_toggle_latency: stats::StagedLatencySummary,
    /// Latency of preview-only browser focus navigation.
    pub(super) browser_focus_preview_latency: stats::StagedLatencySummary,
    /// Latency of committed browser focus navigation.
    pub(super) browser_focus_commit_latency: stats::StagedLatencySummary,
    /// Latency of cached map pan/zoom proxy updates.
    pub(super) map_pan_proxy_latency: stats::StagedLatencySummary,
    /// Latency of waveform interactions through projection.
    pub(super) waveform_interaction_latency: stats::StagedLatencySummary,
    /// Latency of repeated volume drag updates through projection.
    pub(super) volume_drag_latency: stats::StagedLatencySummary,
    /// Latency of motion-only cursor updates.
    pub(super) idle_cursor_motion_latency: stats::StagedLatencySummary,
    /// Latency of adjacent waveform pan/zoom actions.
    pub(super) waveform_pan_zoom_adjacent_latency: stats::LatencySummary,
}

/// Run every GUI latency scenario using the shared seeded controller.
pub(super) fn collect_gui_scenario_metrics(
    options: &BenchOptions,
    controller: &mut AppController,
    mut execute_interaction_step: impl FnMut(&mut AppController, usize),
) -> Result<GuiScenarioMetrics, String> {
    Ok(GuiScenarioMetrics {
        app_model_projection: bench_app_model_projection(options, controller)?,
        motion_model_projection: bench_motion_model_projection(options, controller)?,
        interactive_projection: bench_interactive_projection(
            options,
            controller,
            &mut execute_interaction_step,
        )?,
        hover_latency: bench_hover_latency(options, controller)?,
        wheel_latency: bench_wheel_latency(options, controller)?,
        browser_filter_churn_latency: bench_browser_filter_churn_latency(options, controller)?,
        browser_query_churn_latency: bench_browser_query_churn_latency(options, controller)?,
        browser_sort_toggle_latency: bench_browser_sort_toggle_latency(options, controller)?,
        browser_focus_preview_latency: bench_browser_focus_preview_latency(options, controller)?,
        browser_focus_commit_latency: bench_browser_focus_commit_latency(options, controller)?,
        map_pan_proxy_latency: bench_map_pan_proxy_latency(options, controller)?,
        waveform_interaction_latency: bench_waveform_interactions(options, controller)?,
        volume_drag_latency: bench_volume_drag_latency(options, controller)?,
        idle_cursor_motion_latency: bench_idle_cursor_motion_latency(options, controller)?,
        waveform_pan_zoom_adjacent_latency: bench_waveform_pan_zoom_adjacent_latency(
            options, controller,
        )?,
    })
}

fn bench_app_model_projection(
    options: &BenchOptions,
    controller: &mut AppController,
) -> Result<stats::LatencySummary, String> {
    stats::bench_action(options, || {
        controller.prepare_native_frame(false);
        let _: NativeAppModel = controller.project_native_app_model();
        Ok(())
    })
}

fn bench_motion_model_projection(
    options: &BenchOptions,
    controller: &mut AppController,
) -> Result<stats::LatencySummary, String> {
    stats::bench_action(options, || {
        controller.prepare_native_frame(true);
        let _: NativeMotionModel = controller.project_native_motion_model();
        Ok(())
    })
}

fn bench_interactive_projection(
    options: &BenchOptions,
    controller: &mut AppController,
    execute_interaction_step: &mut impl FnMut(&mut AppController, usize),
) -> Result<stats::StagedLatencySummary, String> {
    let mut interaction_step = 0usize;
    stats::bench_staged_action_with_iters(options.warmup_iters, options.measure_iters, |timer| {
        execute_interaction_step(controller, interaction_step);
        interaction_step = interaction_step.saturating_add(1);
        timer.mark_input_done();
        timer.mark_apply_done();
        controller.prepare_native_frame(false);
        timer.mark_pull_done();
        let _: NativeAppModel = controller.project_native_app_model();
        let _: NativeMotionModel = controller.project_native_motion_model();
        Ok(())
    })
}
