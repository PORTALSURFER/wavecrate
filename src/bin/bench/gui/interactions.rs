//! Focused interaction latency scenarios used by the GUI benchmark harness.

/// Deterministic step-selection helpers shared by interaction scenarios and tests.
pub(super) mod step_patterns;

use super::{BenchOptions, stats, wait_for_rows};
use sempal::app_core::actions::{NativeAppModel, NativeMotionModel, NativeUiAction};
use sempal::app_core::controller::{AppController, AppControllerNativeRuntimeExt};
use sempal::app_core::state::{
    MapBounds, MapPoint, MapQueryBounds, SampleBrowserSort, TriageFlagFilter,
};

use self::step_patterns::{
    adjacent_waveform_action_for_step, interaction_filter_for_step, interaction_query_for_step,
    interaction_sort_for_step, volume_milli_for_step, waveform_action_for_step,
};

/// Apply one deterministic interaction cycle across browser query/filter/sort knobs.
pub(super) fn execute_interaction_step(controller: &mut AppController, step: usize) {
    controller.set_browser_search(interaction_query_for_step(step));
    controller.set_browser_filter(interaction_filter_for_step(step));
    controller.set_browser_sort(interaction_sort_for_step(step));
    if controller.visible_browser_len() > 0 {
        controller.select_column_by_index(step % 3);
    }
}

/// Measure pointer-hover style row focus update latency.
pub(super) fn bench_hover_latency(
    options: &BenchOptions,
    controller: &mut AppController,
) -> Result<stats::StagedLatencySummary, String> {
    let interaction_rows = options.gui_interaction_rows.max(1);
    wait_for_rows(controller, interaction_rows)?;
    let mut step = 0usize;
    stats::bench_staged_action_with_iters(
        interaction_warmup(options),
        interaction_iters(options),
        |timer| {
            let row = step % interaction_rows;
            step = step.saturating_add(1);
            timer.mark_input_done();
            controller.focus_browser_row_only(row);
            timer.mark_apply_done();
            controller.prepare_native_frame(false);
            timer.mark_pull_done();
            let _: NativeAppModel = controller.project_native_app_model();
            Ok(())
        },
    )
}

/// Measure wheel-like row navigation latency.
pub(super) fn bench_wheel_latency(
    options: &BenchOptions,
    controller: &mut AppController,
) -> Result<stats::StagedLatencySummary, String> {
    let interaction_rows = options.gui_interaction_rows.max(1);
    wait_for_rows(controller, interaction_rows)?;
    let mut step = 0usize;
    stats::bench_staged_action_with_iters(
        interaction_warmup(options),
        interaction_iters(options),
        |timer| {
            let delta = match step % 4 {
                0 => 1,
                1 => -1,
                2 => 2,
                _ => -2,
            };
            step = step.saturating_add(1);
            timer.mark_input_done();
            controller.apply_native_ui_action(NativeUiAction::MoveBrowserFocus { delta });
            timer.mark_apply_done();
            controller.prepare_native_frame(false);
            timer.mark_pull_done();
            let _: NativeAppModel = controller.project_native_app_model();
            Ok(())
        },
    )
}

/// Measure filter-only browser recompute latency.
pub(super) fn bench_browser_filter_churn_latency(
    options: &BenchOptions,
    controller: &mut AppController,
) -> Result<stats::StagedLatencySummary, String> {
    wait_for_rows(controller, options.gui_interaction_rows.max(1))?;
    controller.set_browser_search("");
    controller.set_browser_sort(SampleBrowserSort::ListOrder);
    let mut step = 0usize;
    stats::bench_staged_action_with_iters(
        interaction_warmup(options),
        interaction_iters(options),
        |timer| {
            timer.mark_input_done();
            controller.set_browser_filter(interaction_filter_for_step(step));
            step = step.saturating_add(1);
            timer.mark_apply_done();
            controller.prepare_native_frame(false);
            timer.mark_pull_done();
            let _: NativeAppModel = controller.project_native_app_model();
            Ok(())
        },
    )
}

/// Measure query-only browser recompute latency.
pub(super) fn bench_browser_query_churn_latency(
    options: &BenchOptions,
    controller: &mut AppController,
) -> Result<stats::StagedLatencySummary, String> {
    wait_for_rows(controller, options.gui_interaction_rows.max(1))?;
    controller.set_browser_filter(TriageFlagFilter::All);
    controller.set_browser_sort(SampleBrowserSort::ListOrder);
    let mut step = 0usize;
    stats::bench_staged_action_with_iters(
        interaction_warmup(options),
        interaction_iters(options),
        |timer| {
            timer.mark_input_done();
            controller.set_browser_search(interaction_query_for_step(step));
            step = step.saturating_add(1);
            timer.mark_apply_done();
            controller.prepare_native_frame(false);
            timer.mark_pull_done();
            let _: NativeAppModel = controller.project_native_app_model();
            Ok(())
        },
    )
}

/// Measure sort-only browser recompute latency.
pub(super) fn bench_browser_sort_toggle_latency(
    options: &BenchOptions,
    controller: &mut AppController,
) -> Result<stats::StagedLatencySummary, String> {
    wait_for_rows(controller, options.gui_interaction_rows.max(1))?;
    controller.set_browser_filter(TriageFlagFilter::All);
    controller.set_browser_search(interaction_query_for_step(0));
    let mut step = 0usize;
    stats::bench_staged_action_with_iters(
        interaction_warmup(options),
        interaction_iters(options),
        |timer| {
            timer.mark_input_done();
            controller.set_browser_sort(interaction_sort_for_step(step));
            step = step.saturating_add(1);
            timer.mark_apply_done();
            controller.prepare_native_frame(false);
            timer.mark_pull_done();
            let _: NativeAppModel = controller.project_native_app_model();
            Ok(())
        },
    )
}

/// Measure lightweight preview-focus navigation latency.
pub(super) fn bench_browser_focus_preview_latency(
    options: &BenchOptions,
    controller: &mut AppController,
) -> Result<stats::StagedLatencySummary, String> {
    let interaction_rows = options.gui_interaction_rows.max(1);
    wait_for_rows(controller, interaction_rows)?;
    let mut step = 0usize;
    stats::bench_staged_action_with_iters(
        interaction_warmup(options),
        interaction_iters(options),
        |timer| {
            let row = step % interaction_rows;
            step = step.saturating_add(1);
            timer.mark_input_done();
            controller.focus_browser_row_only(row);
            timer.mark_apply_done();
            controller.prepare_native_frame(false);
            timer.mark_pull_done();
            let _: NativeAppModel = controller.project_native_app_model();
            Ok(())
        },
    )
}

/// Measure commit-focus latency after preview navigation.
pub(super) fn bench_browser_focus_commit_latency(
    options: &BenchOptions,
    controller: &mut AppController,
) -> Result<stats::StagedLatencySummary, String> {
    let interaction_rows = options.gui_interaction_rows.max(1);
    wait_for_rows(controller, interaction_rows)?;
    let mut step = 0usize;
    stats::bench_staged_action_with_iters(
        interaction_warmup(options),
        interaction_iters(options),
        |timer| {
            let row = step % interaction_rows;
            step = step.saturating_add(1);
            timer.mark_input_done();
            controller.focus_browser_row_only(row);
            let _ = controller.commit_focused_browser_row();
            timer.mark_apply_done();
            controller.prepare_native_frame(false);
            timer.mark_pull_done();
            let _: NativeAppModel = controller.project_native_app_model();
            Ok(())
        },
    )
}

/// Measure map pan/zoom projection latency using cached map state.
pub(super) fn bench_map_pan_proxy_latency(
    options: &BenchOptions,
    controller: &mut AppController,
) -> Result<stats::StagedLatencySummary, String> {
    prime_map_cache_for_benchmark(controller)?;
    let mut step = 0usize;
    stats::bench_staged_action_with_iters(
        interaction_warmup(options),
        interaction_iters(options),
        |timer| {
            let offset = (step % 16) as f32;
            step = step.saturating_add(1);
            timer.mark_input_done();
            controller.ui.map.pan.x = -24.0 + offset * 3.0;
            controller.ui.map.pan.y = 18.0 - offset * 2.0;
            controller.ui.map.zoom = 1.0 + ((step % 7) as f32 * 0.1);
            controller.ui.map.cached_points_revision =
                controller.ui.map.cached_points_revision.saturating_add(1);
            timer.mark_apply_done();
            controller.prepare_native_frame(false);
            timer.mark_pull_done();
            let _: NativeAppModel = controller.project_native_app_model();
            Ok(())
        },
    )
}

/// Measure waveform interaction latency across seek/cursor/selection/zoom actions.
pub(super) fn bench_waveform_interactions(
    options: &BenchOptions,
    controller: &mut AppController,
) -> Result<stats::StagedLatencySummary, String> {
    let mut step = 0usize;
    stats::bench_staged_action_with_iters(
        interaction_warmup(options),
        interaction_iters(options),
        |timer| {
            let action = waveform_action_for_step(step);
            step = step.saturating_add(1);
            timer.mark_input_done();
            controller.apply_native_ui_action(action);
            timer.mark_apply_done();
            controller.prepare_native_frame(false);
            timer.mark_pull_done();
            let _: NativeAppModel = controller.project_native_app_model();
            let _: NativeMotionModel = controller.project_native_motion_model();
            Ok(())
        },
    )
}

/// Measure continuous volume-drag update latency.
pub(super) fn bench_volume_drag_latency(
    options: &BenchOptions,
    controller: &mut AppController,
) -> Result<stats::StagedLatencySummary, String> {
    wait_for_rows(controller, options.gui_interaction_rows.max(1))?;
    let mut step = 0usize;
    stats::bench_staged_action_with_iters(
        interaction_warmup(options),
        interaction_iters(options),
        |timer| {
            timer.mark_input_done();
            controller.apply_native_ui_action(NativeUiAction::SetVolume {
                value_milli: volume_milli_for_step(step),
            });
            step = step.saturating_add(1);
            timer.mark_apply_done();
            controller.prepare_native_frame(false);
            timer.mark_pull_done();
            let _: NativeAppModel = controller.project_native_app_model();
            Ok(())
        },
    )
}

/// Measure idle cursor-motion latency using motion-only frame preparation.
pub(super) fn bench_idle_cursor_motion_latency(
    options: &BenchOptions,
    controller: &mut AppController,
) -> Result<stats::StagedLatencySummary, String> {
    let mut step = 0usize;
    stats::bench_staged_action_with_iters(
        interaction_warmup(options),
        interaction_iters(options),
        |timer| {
            timer.mark_input_done();
            controller.apply_native_ui_action(NativeUiAction::SetWaveformCursor {
                position_milli: ((step.saturating_mul(37) % 1000) + 1) as u16,
            });
            step = step.saturating_add(1);
            timer.mark_apply_done();
            controller.prepare_native_frame(true);
            timer.mark_pull_done();
            let _: NativeMotionModel = controller.project_native_motion_model();
            Ok(())
        },
    )
}

/// Measure adjacent waveform pan/zoom interactions.
pub(super) fn bench_waveform_pan_zoom_adjacent_latency(
    options: &BenchOptions,
    controller: &mut AppController,
) -> Result<stats::LatencySummary, String> {
    let mut step = 0usize;
    stats::bench_action_with_iters(
        interaction_warmup(options),
        interaction_iters(options),
        || {
            controller.apply_native_ui_action(adjacent_waveform_action_for_step(step));
            step = step.saturating_add(1);
            controller.prepare_native_frame(false);
            let _: NativeAppModel = controller.project_native_app_model();
            let _: NativeMotionModel = controller.project_native_motion_model();
            Ok(())
        },
    )
}

/// Resolve measured-iteration count for focused interaction scenarios.
fn interaction_iters(options: &BenchOptions) -> usize {
    options.gui_interaction_iters.max(1)
}

/// Resolve warmup iterations for focused interaction scenarios.
fn interaction_warmup(options: &BenchOptions) -> usize {
    options.warmup_iters.clamp(1, 3)
}

/// Prime map cache fields so interaction benchmarks avoid cold-start query cost.
pub(super) fn prime_map_cache_for_benchmark(controller: &mut AppController) -> Result<(), String> {
    controller.apply_native_ui_action(NativeUiAction::SetBrowserTab { map: true });
    let source_id = controller
        .ui
        .sources
        .selected
        .and_then(|index| controller.ui.sources.rows.get(index))
        .map(|row| row.id.as_str().to_string())
        .ok_or_else(|| String::from("Map benchmark requires an active source"))?;
    let umap_version = controller.ui.map.umap_version.clone();
    let bounds = MapBounds {
        min_x: -1.0,
        max_x: 1.0,
        min_y: -1.0,
        max_y: 1.0,
    };
    let query = MapQueryBounds {
        min_x: -1.0,
        max_x: 1.0,
        min_y: -1.0,
        max_y: 1.0,
    };
    controller.ui.map.bounds = Some(bounds);
    controller.ui.map.last_query = Some(query);
    controller.ui.map.cached_bounds_source_id = Some(source_id.clone());
    controller.ui.map.cached_bounds_umap_version = Some(umap_version.clone());
    controller.ui.map.cached_points_source_id = Some(source_id);
    controller.ui.map.cached_points_umap_version = Some(umap_version);
    controller.ui.map.cached_points = vec![
        MapPoint {
            sample_id: std::sync::Arc::<str>::from("sample-000000"),
            x: -0.4,
            y: 0.6,
            cluster_id: Some(1),
        },
        MapPoint {
            sample_id: std::sync::Arc::<str>::from("sample-000001"),
            x: 0.5,
            y: -0.35,
            cluster_id: Some(2),
        },
    ];
    controller.ui.map.cached_points_revision = 1;
    Ok(())
}
