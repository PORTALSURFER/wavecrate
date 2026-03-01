//! GUI-oriented benchmark scenarios for the native controller.
/// Interaction benchmark scenarios split from `gui.rs` to keep modules focused.
mod attribution;
mod interactions;
/// Rebuild-cause probes for retained projection attribution reporting.
mod rebuild_probe;
/// Segment counter probes for retained projection attribution reporting.
mod segment_probe;
use self::attribution::{GuiInteractionRebuildCauseAttribution, GuiInteractionSegmentAttribution};
use self::interactions::{
    bench_browser_filter_churn_latency, bench_browser_focus_commit_latency,
    bench_browser_focus_preview_latency, bench_browser_query_churn_latency,
    bench_browser_sort_toggle_latency, bench_hover_latency, bench_idle_cursor_motion_latency,
    bench_map_pan_proxy_latency, bench_volume_drag_latency, bench_waveform_interactions,
    bench_waveform_pan_zoom_adjacent_latency, bench_wheel_latency, execute_interaction_step,
};
use self::rebuild_probe::collect_interaction_rebuild_cause_attribution;
use self::segment_probe::collect_interaction_segment_attribution;
use super::{options::BenchOptions, stats};
use hound::{SampleFormat, WavSpec, WavWriter};
use sempal::app_core::actions::{NativeAppModel, NativeMotionModel};
use sempal::app_core::controller::{AppController, AppControllerNativeRuntimeExt};
use sempal::waveform::WaveformRenderer;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tempfile::TempDir;

/// Synthetic workload configuration and results for GUI frame projection.
#[derive(Clone, Debug, Serialize)]
pub(super) struct GuiBenchResult {
    /// Number of DB rows seeded into the synthetic source.
    pub(super) seeded_rows: usize,
    /// Latency of native app model projection.
    pub(super) app_model_projection: stats::LatencySummary,
    /// Latency of native motion model projection.
    pub(super) motion_model_projection: stats::LatencySummary,
    /// Latency of a small UI mutation + projection sequence.
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
    /// Stage-attributed latency summaries for focused interaction scenarios.
    pub(super) interaction_stage_attribution: GuiInteractionStageAttribution,
    /// Segment-attributed latency/counter summaries for retained projection slices.
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

/// Scoped benchmark workspace that keeps seed artifacts alive for the benchmark
/// duration while preventing accidental cleanup races.
struct BenchWorkspace {
    /// Temporary directory that stores synthetic source files and DB state.
    _temp_root: TempDir,
    controller: AppController,
}

/// Run GUI benchmark actions and summarize performance characteristics.
pub(super) fn run(options: &BenchOptions) -> Result<GuiBenchResult, String> {
    let mut workspace = build_controller_with_db_rows(options)?;
    let seeded_rows = seed_rows(&mut workspace.controller, options.gui_rows)?;
    let app_model_projection = stats::bench_action(options, || {
        workspace.controller.prepare_native_frame(false);
        let _: NativeAppModel = workspace.controller.project_native_app_model();
        Ok(())
    })?;
    let motion_model_projection = stats::bench_action(options, || {
        workspace.controller.prepare_native_frame(true);
        let _: NativeMotionModel = workspace.controller.project_native_motion_model();
        Ok(())
    })?;
    let mut interaction_step = 0usize;
    let interactive_projection = stats::bench_staged_action_with_iters(
        options.warmup_iters,
        options.measure_iters,
        |timer| {
            execute_interaction_step(&mut workspace.controller, interaction_step);
            interaction_step = interaction_step.saturating_add(1);
            timer.mark_input_done();
            timer.mark_apply_done();
            workspace.controller.prepare_native_frame(false);
            timer.mark_pull_done();
            let _: NativeAppModel = workspace.controller.project_native_app_model();
            let _: NativeMotionModel = workspace.controller.project_native_motion_model();
            Ok(())
        },
    )?;
    let hover_latency = bench_hover_latency(options, &mut workspace.controller)?;
    let wheel_latency = bench_wheel_latency(options, &mut workspace.controller)?;
    let browser_filter_churn_latency =
        bench_browser_filter_churn_latency(options, &mut workspace.controller)?;
    let browser_query_churn_latency =
        bench_browser_query_churn_latency(options, &mut workspace.controller)?;
    let browser_sort_toggle_latency =
        bench_browser_sort_toggle_latency(options, &mut workspace.controller)?;
    let browser_focus_preview_latency =
        bench_browser_focus_preview_latency(options, &mut workspace.controller)?;
    let browser_focus_commit_latency =
        bench_browser_focus_commit_latency(options, &mut workspace.controller)?;
    let map_pan_proxy_latency = bench_map_pan_proxy_latency(options, &mut workspace.controller)?;
    let waveform_interaction_latency =
        bench_waveform_interactions(options, &mut workspace.controller)?;
    let volume_drag_latency = bench_volume_drag_latency(options, &mut workspace.controller)?;
    let idle_cursor_motion_latency =
        bench_idle_cursor_motion_latency(options, &mut workspace.controller)?;
    let waveform_pan_zoom_adjacent_latency =
        bench_waveform_pan_zoom_adjacent_latency(options, &mut workspace.controller)?;

    let stats::StagedLatencySummary {
        total: interactive_projection_total,
        stages: interactive_projection_stages,
    } = interactive_projection;
    let stats::StagedLatencySummary {
        total: hover_latency_total,
        stages: hover_latency_stages,
    } = hover_latency;
    let stats::StagedLatencySummary {
        total: wheel_latency_total,
        stages: wheel_latency_stages,
    } = wheel_latency;
    let stats::StagedLatencySummary {
        total: browser_filter_churn_latency_total,
        stages: browser_filter_churn_latency_stages,
    } = browser_filter_churn_latency;
    let stats::StagedLatencySummary {
        total: browser_query_churn_latency_total,
        stages: browser_query_churn_latency_stages,
    } = browser_query_churn_latency;
    let stats::StagedLatencySummary {
        total: browser_sort_toggle_latency_total,
        stages: browser_sort_toggle_latency_stages,
    } = browser_sort_toggle_latency;
    let stats::StagedLatencySummary {
        total: browser_focus_preview_latency_total,
        stages: browser_focus_preview_latency_stages,
    } = browser_focus_preview_latency;
    let stats::StagedLatencySummary {
        total: browser_focus_commit_latency_total,
        stages: browser_focus_commit_latency_stages,
    } = browser_focus_commit_latency;
    let stats::StagedLatencySummary {
        total: map_pan_proxy_latency_total,
        stages: map_pan_proxy_latency_stages,
    } = map_pan_proxy_latency;
    let stats::StagedLatencySummary {
        total: waveform_interaction_latency_total,
        stages: waveform_interaction_latency_stages,
    } = waveform_interaction_latency;
    let stats::StagedLatencySummary {
        total: volume_drag_latency_total,
        stages: volume_drag_latency_stages,
    } = volume_drag_latency;
    let stats::StagedLatencySummary {
        total: idle_cursor_motion_latency_total,
        stages: idle_cursor_motion_latency_stages,
    } = idle_cursor_motion_latency;
    let interaction_stage_attribution = GuiInteractionStageAttribution {
        interactive_projection: interactive_projection_stages,
        hover_latency: hover_latency_stages,
        wheel_latency: wheel_latency_stages,
        browser_filter_churn_latency: browser_filter_churn_latency_stages,
        browser_query_churn_latency: browser_query_churn_latency_stages,
        browser_sort_toggle_latency: browser_sort_toggle_latency_stages,
        browser_focus_preview_latency: browser_focus_preview_latency_stages,
        browser_focus_commit_latency: browser_focus_commit_latency_stages,
        map_pan_proxy_latency: map_pan_proxy_latency_stages,
        waveform_interaction_latency: waveform_interaction_latency_stages,
        volume_drag_latency: volume_drag_latency_stages,
        idle_cursor_motion_latency: idle_cursor_motion_latency_stages,
    };
    let interaction_segment_attribution = Some(collect_interaction_segment_attribution(
        options,
        &mut workspace.controller,
        &interaction_stage_attribution,
    )?);
    let interaction_rebuild_cause_attribution = Some(
        collect_interaction_rebuild_cause_attribution(options, &mut workspace.controller)?,
    );
    Ok(GuiBenchResult {
        seeded_rows,
        app_model_projection,
        motion_model_projection,
        interactive_projection: interactive_projection_total,
        hover_latency: hover_latency_total,
        wheel_latency: wheel_latency_total,
        browser_filter_churn_latency: browser_filter_churn_latency_total,
        browser_query_churn_latency: browser_query_churn_latency_total,
        browser_sort_toggle_latency: browser_sort_toggle_latency_total,
        browser_focus_preview_latency: browser_focus_preview_latency_total,
        browser_focus_commit_latency: browser_focus_commit_latency_total,
        map_pan_proxy_latency: map_pan_proxy_latency_total,
        waveform_interaction_latency: waveform_interaction_latency_total,
        volume_drag_latency: volume_drag_latency_total,
        idle_cursor_motion_latency: idle_cursor_motion_latency_total,
        waveform_pan_zoom_adjacent_latency,
        interaction_stage_attribution,
        interaction_segment_attribution,
        interaction_rebuild_cause_attribution,
    })
}

fn seed_rows(controller: &mut AppController, rows: usize) -> Result<usize, String> {
    let effective_rows = rows.max(1);
    wait_for_rows(controller, effective_rows)?;
    Ok(effective_rows)
}

fn wait_for_rows(controller: &mut AppController, target: usize) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        controller.prepare_native_frame(false);
        if observed_visible_rows(controller) >= target {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    if observed_visible_rows(controller) >= target {
        return Ok(());
    }
    let model = controller.project_native_app_model();
    Err(format!(
        "Timed out waiting for GUI rows: {} < {}",
        observed_visible_rows(controller),
        target
    ) + &format!(
        " | sources: {}, visible_count: {}, columns: [T:{},N:{},K:{}], selected: {}",
        model.sources.rows.len(),
        model.browser.visible_count,
        model.columns[0].item_count,
        model.columns[1].item_count,
        model.columns[2].item_count,
        model
            .browser
            .selected_visible_row
            .map_or_else(|| "none".to_string(), |row| row.to_string())
    ))
}

fn observed_visible_rows(controller: &mut AppController) -> usize {
    let direct = controller.visible_browser_len();
    let projected = controller.project_native_app_model();
    let projected_visible = projected.browser.visible_count;
    let projected_columns_total = projected
        .columns
        .iter()
        .map(|column| column.item_count)
        .sum::<usize>();
    direct.max(projected_visible).max(projected_columns_total)
}

fn build_controller_with_db_rows(options: &BenchOptions) -> Result<BenchWorkspace, String> {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let temp_root =
        tempfile::tempdir().map_err(|err| format!("Create temp source dir failed: {err}"))?;
    let source_root = temp_root.path().join("gui-source");
    fs::create_dir_all(&source_root)
        .map_err(|err| format!("Create source dir {} failed: {err}", source_root.display()))?;

    let seed_rows = options.gui_rows.max(options.gui_interaction_rows).max(1);
    for (row, file_name) in seeded_wav_filenames(seed_rows).into_iter().enumerate() {
        write_seed_wav(&source_root.join(&file_name), row as i64)
            .map_err(|err| format!("Seed test audio failed: {err}"))?;
    }
    let source_dir = source_root.clone();
    controller
        .add_source_from_path(source_dir)
        .map_err(|err| format!("Add benchmark source failed: {err}"))?;
    controller.select_first_source();
    controller
        .refresh_wavs()
        .map_err(|err| format!("Refresh benchmark wavs failed: {err}"))?;
    Ok(BenchWorkspace {
        _temp_root: temp_root,
        controller,
    })
}

fn seeded_wav_filenames(target_rows: usize) -> Vec<PathBuf> {
    (0..target_rows)
        .map(|row| PathBuf::from(format!("sample_{row:06}.wav")))
        .collect()
}

fn write_seed_wav(path: &Path, seed: i64) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("Create wav parent {} failed: {err}", parent.display()))?;
    }
    let spec = WavSpec {
        channels: 1,
        sample_rate: 8,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };
    let sample = [seed as f32 % 1.0];
    let mut writer = WavWriter::create(path, spec).map_err(|err| err.to_string())?;
    writer
        .write_sample(sample[0])
        .map_err(|err| err.to_string())?;
    writer.finalize().map_err(|err| err.to_string())?;
    Ok(())
}

/// GUI benchmark behavior and interaction sequencing tests.
#[cfg(test)]
#[path = "gui_tests.rs"]
mod tests;
