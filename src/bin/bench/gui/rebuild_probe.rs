//! Rebuild-cause attribution probes for GUI benchmark reporting.

use super::attribution::{
    GuiInteractionRebuildCauseAttribution, rebuild_cause_summary_from_counts,
};
use super::interactions::{
    execute_interaction_step, prime_map_cache_for_benchmark,
    step_patterns::{
        adjacent_waveform_action_for_step, interaction_filter_for_step, interaction_query_for_step,
        interaction_sort_for_step, volume_milli_for_step, waveform_action_for_step,
    },
};
use super::{BenchOptions, wait_for_rows};
use sempal::app_core::actions::NativeUiAction;
use sempal::app_core::controller::{AppController, AppControllerNativeRuntimeExt};
use sempal::app_core::native_bridge::{
    ProjectionRebuildCauseCounts, measure_projection_rebuild_cause_counts,
};
use sempal::app_core::state::{SampleBrowserSort, TriageFlagFilter};

/// Collect rebuild-cause counters from focused controller-mode action probes.
pub(super) fn collect_interaction_rebuild_cause_attribution(
    options: &BenchOptions,
    controller: &mut AppController,
) -> Result<GuiInteractionRebuildCauseAttribution, String> {
    let interaction_rows = options.gui_interaction_rows.max(1);
    wait_for_rows(controller, interaction_rows)?;
    let warmup_iters = options.warmup_iters.max(1);
    let measure_iters = options.gui_interaction_iters.max(1);

    let interactive_projection =
        probe_interactive(controller, interaction_rows, warmup_iters, measure_iters);
    let hover_latency = probe_hover(controller, interaction_rows, warmup_iters, measure_iters);
    let wheel_latency = probe_wheel(controller, warmup_iters, measure_iters);
    let browser_filter_churn_latency = probe_filter(controller, warmup_iters, measure_iters);
    let browser_query_churn_latency = probe_query(controller, warmup_iters, measure_iters);
    let browser_sort_toggle_latency = probe_sort(controller, warmup_iters, measure_iters);
    let browser_focus_preview_latency =
        probe_focus_preview(controller, interaction_rows, warmup_iters, measure_iters);
    let browser_focus_commit_latency =
        probe_focus_commit(controller, interaction_rows, warmup_iters, measure_iters);
    let map_pan_proxy_latency = probe_map(controller, warmup_iters, measure_iters)?;
    let waveform_interaction_latency = probe_waveform(controller, warmup_iters, measure_iters);
    let volume_drag_latency = probe_volume(controller, warmup_iters, measure_iters);
    let waveform_pan_zoom_adjacent_latency =
        probe_waveform_adjacent(controller, warmup_iters, measure_iters);

    Ok(GuiInteractionRebuildCauseAttribution {
        interactive_projection: rebuild_cause_summary_from_counts(interactive_projection),
        hover_latency: rebuild_cause_summary_from_counts(hover_latency),
        wheel_latency: rebuild_cause_summary_from_counts(wheel_latency),
        browser_filter_churn_latency: rebuild_cause_summary_from_counts(
            browser_filter_churn_latency,
        ),
        browser_query_churn_latency: rebuild_cause_summary_from_counts(browser_query_churn_latency),
        browser_sort_toggle_latency: rebuild_cause_summary_from_counts(browser_sort_toggle_latency),
        browser_focus_preview_latency: rebuild_cause_summary_from_counts(
            browser_focus_preview_latency,
        ),
        browser_focus_commit_latency: rebuild_cause_summary_from_counts(
            browser_focus_commit_latency,
        ),
        map_pan_proxy_latency: rebuild_cause_summary_from_counts(map_pan_proxy_latency),
        waveform_interaction_latency: rebuild_cause_summary_from_counts(
            waveform_interaction_latency,
        ),
        volume_drag_latency: rebuild_cause_summary_from_counts(volume_drag_latency),
        waveform_pan_zoom_adjacent_latency: rebuild_cause_summary_from_counts(
            waveform_pan_zoom_adjacent_latency,
        ),
    })
}

fn probe_interactive(
    controller: &mut AppController,
    _interaction_rows: usize,
    warmup_iters: usize,
    measure_iters: usize,
) -> ProjectionRebuildCauseCounts {
    let mut step = 0usize;
    measure_projection_rebuild_cause_counts(
        controller,
        warmup_iters,
        measure_iters,
        true,
        |controller, _| {
            execute_interaction_step(controller, step);
            step = step.saturating_add(1);
        },
    )
}

fn probe_hover(
    controller: &mut AppController,
    interaction_rows: usize,
    warmup_iters: usize,
    measure_iters: usize,
) -> ProjectionRebuildCauseCounts {
    let mut step = 0usize;
    measure_projection_rebuild_cause_counts(
        controller,
        warmup_iters,
        measure_iters,
        false,
        |controller, _| {
            controller.focus_browser_row_only(step % interaction_rows);
            step = step.saturating_add(1);
        },
    )
}

fn probe_wheel(
    controller: &mut AppController,
    warmup_iters: usize,
    measure_iters: usize,
) -> ProjectionRebuildCauseCounts {
    let mut step = 0usize;
    measure_projection_rebuild_cause_counts(
        controller,
        warmup_iters,
        measure_iters,
        false,
        |controller, _| {
            let delta = match step % 4 {
                0 => 1,
                1 => -1,
                2 => 2,
                _ => -2,
            };
            step = step.saturating_add(1);
            controller.apply_native_ui_action(NativeUiAction::MoveBrowserFocus { delta });
        },
    )
}

fn probe_filter(
    controller: &mut AppController,
    warmup_iters: usize,
    measure_iters: usize,
) -> ProjectionRebuildCauseCounts {
    controller.set_browser_search("");
    controller.set_browser_sort(SampleBrowserSort::ListOrder);
    measure_projection_rebuild_cause_counts(
        controller,
        warmup_iters,
        measure_iters,
        false,
        |controller, step| {
            controller.set_browser_filter(interaction_filter_for_step(step));
        },
    )
}

fn probe_query(
    controller: &mut AppController,
    warmup_iters: usize,
    measure_iters: usize,
) -> ProjectionRebuildCauseCounts {
    controller.set_browser_filter(TriageFlagFilter::All);
    controller.set_browser_sort(SampleBrowserSort::ListOrder);
    measure_projection_rebuild_cause_counts(
        controller,
        warmup_iters,
        measure_iters,
        false,
        |controller, step| {
            controller.set_browser_search(interaction_query_for_step(step));
        },
    )
}

fn probe_sort(
    controller: &mut AppController,
    warmup_iters: usize,
    measure_iters: usize,
) -> ProjectionRebuildCauseCounts {
    controller.set_browser_filter(TriageFlagFilter::All);
    controller.set_browser_search(interaction_query_for_step(0));
    measure_projection_rebuild_cause_counts(
        controller,
        warmup_iters,
        measure_iters,
        false,
        |controller, step| {
            controller.set_browser_sort(interaction_sort_for_step(step));
        },
    )
}

fn probe_focus_preview(
    controller: &mut AppController,
    interaction_rows: usize,
    warmup_iters: usize,
    measure_iters: usize,
) -> ProjectionRebuildCauseCounts {
    let mut step = 0usize;
    measure_projection_rebuild_cause_counts(
        controller,
        warmup_iters,
        measure_iters,
        false,
        |controller, _| {
            controller.focus_browser_row_only(step % interaction_rows);
            step = step.saturating_add(1);
        },
    )
}

fn probe_focus_commit(
    controller: &mut AppController,
    interaction_rows: usize,
    warmup_iters: usize,
    measure_iters: usize,
) -> ProjectionRebuildCauseCounts {
    let mut step = 0usize;
    measure_projection_rebuild_cause_counts(
        controller,
        warmup_iters,
        measure_iters,
        false,
        |controller, _| {
            controller.focus_browser_row_only(step % interaction_rows);
            let _ = controller.commit_focused_browser_row();
            step = step.saturating_add(1);
        },
    )
}

fn probe_map(
    controller: &mut AppController,
    warmup_iters: usize,
    measure_iters: usize,
) -> Result<ProjectionRebuildCauseCounts, String> {
    prime_map_cache_for_benchmark(controller)?;
    let mut step = 0usize;
    Ok(measure_projection_rebuild_cause_counts(
        controller,
        warmup_iters,
        measure_iters,
        false,
        |controller, _| {
            let offset = (step % 16) as f32;
            step = step.saturating_add(1);
            controller.ui.map.pan.x = -24.0 + offset * 3.0;
            controller.ui.map.pan.y = 18.0 - offset * 2.0;
            controller.ui.map.zoom = 1.0 + ((step % 7) as f32 * 0.1);
            controller.ui.map.cached_points_revision =
                controller.ui.map.cached_points_revision.saturating_add(1);
        },
    ))
}

fn probe_waveform(
    controller: &mut AppController,
    warmup_iters: usize,
    measure_iters: usize,
) -> ProjectionRebuildCauseCounts {
    measure_projection_rebuild_cause_counts(
        controller,
        warmup_iters,
        measure_iters,
        true,
        |controller, step| {
            controller.apply_native_ui_action(waveform_action_for_step(step));
        },
    )
}

fn probe_volume(
    controller: &mut AppController,
    warmup_iters: usize,
    measure_iters: usize,
) -> ProjectionRebuildCauseCounts {
    measure_projection_rebuild_cause_counts(
        controller,
        warmup_iters,
        measure_iters,
        false,
        |controller, step| {
            controller.apply_native_ui_action(NativeUiAction::SetVolume {
                value_milli: volume_milli_for_step(step),
            });
        },
    )
}

fn probe_waveform_adjacent(
    controller: &mut AppController,
    warmup_iters: usize,
    measure_iters: usize,
) -> ProjectionRebuildCauseCounts {
    measure_projection_rebuild_cause_counts(
        controller,
        warmup_iters,
        measure_iters,
        true,
        |controller, step| {
            controller.apply_native_ui_action(adjacent_waveform_action_for_step(step));
        },
    )
}
