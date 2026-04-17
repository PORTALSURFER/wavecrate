//! Scenario registry for the GUI benchmark harness.

use super::interactions::{
    bench_browser_filter_churn_latency, bench_browser_focus_commit_latency,
    bench_browser_focus_preview_latency, bench_browser_query_churn_latency,
    bench_browser_sort_toggle_latency, bench_hover_latency, bench_idle_cursor_motion_latency,
    bench_map_pan_proxy_latency, bench_volume_drag_latency, bench_waveform_interactions,
    bench_waveform_pan_zoom_adjacent_latency, bench_wheel_latency,
};
use super::{BenchOptions, stats};
use sempal::app_core::actions::{NativeAppBridge, NativeAppModel, NativeMotionModel};
use sempal::app_core::controller::{AppController, AppControllerNativeRuntimeExt};
use sempal::app_core::native_bridge::{SempalNativeBridge, measure_projection_segment_probe};

/// Latency summaries collected for every GUI benchmark scenario.
pub(super) struct GuiScenarioMetrics {
    /// Latency of retained app-model pull and projection through the shipped cache path.
    pub(super) app_model_projection: stats::LatencySummary,
    /// Latency of controller-mode app-model projection kept for diagnostics.
    pub(super) controller_app_model_projection: stats::LatencySummary,
    /// Retained-runtime app-model projection p95 captured through the bridge cache path.
    pub(super) retained_app_model_projection_p95_us: u64,
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
    controller: AppController,
    mut execute_interaction_step: impl FnMut(&mut AppController, usize),
) -> Result<GuiScenarioMetrics, String> {
    let mut bridge = SempalNativeBridge::from_fixture_controller(controller);
    Ok(GuiScenarioMetrics {
        app_model_projection: bench_retained_app_model_projection(options, &mut bridge)?,
        controller_app_model_projection: bench_controller_app_model_projection(
            options,
            &mut bridge,
        )?,
        retained_app_model_projection_p95_us: bench_retained_app_model_projection_p95_us(
            options,
            &mut bridge,
        ),
        motion_model_projection: bench_motion_model_projection(options, &mut bridge)?,
        interactive_projection: bench_interactive_projection(
            options,
            &mut bridge,
            &mut execute_interaction_step,
        )?,
        hover_latency: bench_hover_latency(options, &mut bridge)?,
        wheel_latency: bench_wheel_latency(options, &mut bridge)?,
        browser_filter_churn_latency: bench_browser_filter_churn_latency(options, &mut bridge)?,
        browser_query_churn_latency: bench_browser_query_churn_latency(options, &mut bridge)?,
        browser_sort_toggle_latency: bench_browser_sort_toggle_latency(options, &mut bridge)?,
        browser_focus_preview_latency: bench_browser_focus_preview_latency(options, &mut bridge)?,
        browser_focus_commit_latency: bench_browser_focus_commit_latency(options, &mut bridge)?,
        map_pan_proxy_latency: bench_map_pan_proxy_latency(options, &mut bridge)?,
        waveform_interaction_latency: bench_waveform_interactions(options, &mut bridge)?,
        volume_drag_latency: bench_volume_drag_latency(options, &mut bridge)?,
        idle_cursor_motion_latency: bench_idle_cursor_motion_latency(options, &mut bridge)?,
        waveform_pan_zoom_adjacent_latency: bench_waveform_pan_zoom_adjacent_latency(
            options,
            &mut bridge,
        )?,
    })
}

fn bench_retained_app_model_projection_p95_us(
    options: &BenchOptions,
    bridge: &mut SempalNativeBridge,
) -> u64 {
    bridge.mutate_controller(|controller| {
        measure_projection_segment_probe(
            controller,
            options.warmup_iters,
            options.measure_iters,
            |_controller, _| {},
        )
        .projection_p95_us
    })
}

fn bench_retained_app_model_projection(
    options: &BenchOptions,
    bridge: &mut SempalNativeBridge,
) -> Result<stats::LatencySummary, String> {
    stats::bench_action(options, || {
        let _: std::sync::Arc<NativeAppModel> = bridge.project_model();
        Ok(())
    })
}

fn bench_controller_app_model_projection(
    options: &BenchOptions,
    bridge: &mut SempalNativeBridge,
) -> Result<stats::LatencySummary, String> {
    stats::bench_action(options, || {
        bridge.mutate_controller(|controller| {
            controller.prepare_native_frame(false);
            let _: NativeAppModel = controller.project_native_app_model();
        });
        Ok(())
    })
}

fn bench_motion_model_projection(
    options: &BenchOptions,
    bridge: &mut SempalNativeBridge,
) -> Result<stats::LatencySummary, String> {
    stats::bench_action(options, || {
        let _: Option<NativeMotionModel> = bridge.project_motion_model();
        Ok(())
    })
}

fn bench_interactive_projection(
    options: &BenchOptions,
    bridge: &mut SempalNativeBridge,
    execute_interaction_step: &mut impl FnMut(&mut AppController, usize),
) -> Result<stats::StagedLatencySummary, String> {
    let mut interaction_step = 0usize;
    stats::bench_staged_action_with_iters(options.warmup_iters, options.measure_iters, |timer| {
        bridge
            .mutate_controller(|controller| execute_interaction_step(controller, interaction_step));
        interaction_step = interaction_step.saturating_add(1);
        timer.mark_input_done();
        timer.mark_apply_done();
        let _: std::sync::Arc<NativeAppModel> = bridge.project_model();
        timer.mark_pull_done();
        let _: Option<NativeMotionModel> = bridge.project_motion_model();
        Ok(())
    })
}
