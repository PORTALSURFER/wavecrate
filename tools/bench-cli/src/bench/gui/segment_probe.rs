//! Segment hit/miss attribution probes for GUI benchmark reporting.

use super::super::options::BenchOptions;
use super::attribution::{GuiInteractionSegmentAttribution, SegmentAttributionSummary};
use super::interactions::{execute_interaction_step, prime_map_cache_for_benchmark};
use super::workspace::wait_for_rows;
use sempal::app_core::actions::NativeUiAction;
use sempal::app_core::controller::{AppController, AppControllerNativeRuntimeExt};
use sempal::app_core::native_bridge::{
    ProjectionSegmentLookupCount, ProjectionSegmentProbeMeasurement,
    measure_projection_segment_probe,
};

/// Collect retained-projection segment hit/miss counters and measured probe
/// timings from focused action probes.
pub(super) fn collect_interaction_segment_attribution(
    options: &BenchOptions,
    controller: &mut AppController,
) -> Result<GuiInteractionSegmentAttribution, String> {
    let interaction_rows = options.gui_interaction_rows.max(1);
    wait_for_rows(controller, interaction_rows)?;
    let warmup_iters = options.warmup_iters.max(1);
    let measure_iters = options.gui_interaction_iters.max(1);

    let mut interactive_step = 0usize;
    let status_probe = measure_projection_segment_probe(
        controller,
        warmup_iters,
        measure_iters,
        |controller, _| {
            execute_interaction_step(controller, interactive_step);
            interactive_step = interactive_step.saturating_add(1);
        },
    );

    let mut frame_step = 0usize;
    let browser_frame_probe = measure_projection_segment_probe(
        controller,
        warmup_iters,
        measure_iters,
        |controller, _| {
            controller.set_browser_search(format!("frame-{frame_step:03}"));
            frame_step = (frame_step + 1) % 16;
        },
    );

    let mut rows_step = 0usize;
    let browser_rows_probe = measure_projection_segment_probe(
        controller,
        warmup_iters,
        measure_iters,
        |controller, _| {
            let delta = match rows_step % 4 {
                0 => 1,
                1 => -1,
                2 => 2,
                _ => -2,
            };
            rows_step = rows_step.saturating_add(1);
            controller.apply_native_ui_action(NativeUiAction::MoveBrowserFocus { delta });
        },
    );

    prime_map_cache_for_benchmark(controller)?;
    let mut map_step = 0usize;
    let map_probe = measure_projection_segment_probe(
        controller,
        warmup_iters,
        measure_iters,
        |controller, _| {
            let offset = (map_step % 16) as f32;
            map_step = map_step.saturating_add(1);
            controller.ui.map.pan.x = -24.0 + offset * 3.0;
            controller.ui.map.pan.y = 18.0 - offset * 2.0;
            controller.ui.map.zoom = 1.0 + ((map_step % 7) as f32 * 0.1);
            controller.ui.map.cached_points_revision =
                controller.ui.map.cached_points_revision.saturating_add(1);
        },
    );

    let mut waveform_step = 0usize;
    let waveform_probe = measure_projection_segment_probe(
        controller,
        warmup_iters,
        measure_iters,
        |controller, _| {
            controller.apply_native_ui_action(NativeUiAction::SetWaveformCursor {
                position_milli: ((waveform_step % 1000) + 1) as u16,
            });
            waveform_step = waveform_step.saturating_add(37);
        },
    );

    Ok(GuiInteractionSegmentAttribution {
        status_bar: segment_summary(&status_probe, status_probe.lookup_counts.status_bar),
        browser_frame: segment_summary(
            &browser_frame_probe,
            browser_frame_probe.lookup_counts.browser_frame,
        ),
        browser_rows_window: segment_summary(
            &browser_rows_probe,
            browser_rows_probe.lookup_counts.browser_rows_window,
        ),
        map_panel: segment_summary(&map_probe, map_probe.lookup_counts.map_panel),
        waveform_overlay: segment_summary(
            &waveform_probe,
            waveform_probe.lookup_counts.waveform_overlay,
        ),
    })
}

/// Convert one probe measurement into the benchmark report summary shape.
fn segment_summary(
    probe: &ProjectionSegmentProbeMeasurement,
    count: ProjectionSegmentLookupCount,
) -> SegmentAttributionSummary {
    SegmentAttributionSummary {
        hit_count: count.hit_count,
        miss_count: count.miss_count,
        p95_us: probe.projection_p95_us,
    }
}
