//! GUI-oriented benchmark scenarios for the native controller.

use super::{options::BenchOptions, stats};
use hound::{SampleFormat, WavSpec, WavWriter};
use sempal::app_core::actions::{NativeAppModel, NativeMotionModel, NativeUiAction};
use sempal::app_core::controller::{AppController, AppControllerNativeRuntimeExt};
use sempal::app_core::state::{
    MapBounds, MapPoint, MapQueryBounds, SampleBrowserSort, TriageFlagFilter,
};
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
    /// Latency of map pan/zoom state changes through model projection.
    pub(super) map_pan_proxy_latency: stats::LatencySummary,
    /// Latency of waveform interaction actions through projection.
    pub(super) waveform_interaction_latency: stats::LatencySummary,
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
    let interactive_projection = stats::bench_action(options, || {
        execute_interaction_step(&mut workspace.controller, interaction_step)?;
        interaction_step = interaction_step.saturating_add(1);
        workspace.controller.prepare_native_frame(false);
        let _: NativeAppModel = workspace.controller.project_native_app_model();
        let _: NativeMotionModel = workspace.controller.project_native_motion_model();
        Ok(())
    })?;
    let hover_latency = bench_hover_latency(options, &mut workspace.controller)?;
    let wheel_latency = bench_wheel_latency(options, &mut workspace.controller)?;
    let map_pan_proxy_latency = bench_map_pan_proxy_latency(options, &mut workspace.controller)?;
    let waveform_interaction_latency =
        bench_waveform_interactions(options, &mut workspace.controller)?;
    Ok(GuiBenchResult {
        seeded_rows,
        app_model_projection,
        motion_model_projection,
        interactive_projection,
        hover_latency,
        wheel_latency,
        map_pan_proxy_latency,
        waveform_interaction_latency,
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

/// Return a resilient browser visible-row count for benchmark readiness checks.
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

fn execute_interaction_step(controller: &mut AppController, step: usize) -> Result<(), String> {
    controller.set_browser_search(interaction_query_for_step(step));
    controller.set_browser_filter(interaction_filter_for_step(step));
    controller.set_browser_sort(interaction_sort_for_step(step));
    if controller.visible_browser_len() > 0 {
        controller.select_column_by_index(step % 3);
    }
    Ok(())
}

/// Resolve measured-iteration count for focused interaction scenarios.
fn interaction_iters(options: &BenchOptions) -> usize {
    options.gui_interaction_iters.max(1)
}

/// Resolve warmup iterations for focused interaction scenarios.
fn interaction_warmup(options: &BenchOptions) -> usize {
    options.warmup_iters.clamp(1, 3)
}

/// Measure pointer-hover style row focus update latency.
fn bench_hover_latency(
    options: &BenchOptions,
    controller: &mut AppController,
) -> Result<stats::LatencySummary, String> {
    let interaction_rows = options.gui_interaction_rows.max(1);
    wait_for_rows(controller, interaction_rows)?;
    let mut step = 0usize;
    stats::bench_action_with_iters(
        interaction_warmup(options),
        interaction_iters(options),
        || {
            let row = step % interaction_rows;
            step = step.saturating_add(1);
            controller.focus_browser_row(row);
            controller.prepare_native_frame(false);
            let _: NativeAppModel = controller.project_native_app_model();
            Ok(())
        },
    )
}

/// Measure wheel-like row navigation latency.
fn bench_wheel_latency(
    options: &BenchOptions,
    controller: &mut AppController,
) -> Result<stats::LatencySummary, String> {
    let interaction_rows = options.gui_interaction_rows.max(1);
    wait_for_rows(controller, interaction_rows)?;
    let mut step = 0usize;
    stats::bench_action_with_iters(
        interaction_warmup(options),
        interaction_iters(options),
        || {
            let delta = match step % 4 {
                0 => 1,
                1 => -1,
                2 => 2,
                _ => -2,
            };
            step = step.saturating_add(1);
            controller.apply_native_ui_action(NativeUiAction::MoveBrowserFocus { delta });
            controller.prepare_native_frame(false);
            let _: NativeAppModel = controller.project_native_app_model();
            Ok(())
        },
    )
}

/// Measure map pan/zoom projection latency using cached map state.
fn bench_map_pan_proxy_latency(
    options: &BenchOptions,
    controller: &mut AppController,
) -> Result<stats::LatencySummary, String> {
    prime_map_cache_for_benchmark(controller)?;
    let mut step = 0usize;
    stats::bench_action_with_iters(
        interaction_warmup(options),
        interaction_iters(options),
        || {
            let offset = (step % 16) as f32;
            step = step.saturating_add(1);
            controller.ui.map.pan.x = -24.0 + offset * 3.0;
            controller.ui.map.pan.y = 18.0 - offset * 2.0;
            controller.ui.map.zoom = 1.0 + ((step % 7) as f32 * 0.1);
            controller.ui.map.cached_points_revision =
                controller.ui.map.cached_points_revision.saturating_add(1);
            controller.prepare_native_frame(false);
            let _: NativeAppModel = controller.project_native_app_model();
            Ok(())
        },
    )
}

/// Measure waveform interaction latency across seek/cursor/selection/zoom actions.
fn bench_waveform_interactions(
    options: &BenchOptions,
    controller: &mut AppController,
) -> Result<stats::LatencySummary, String> {
    let mut step = 0usize;
    stats::bench_action_with_iters(
        interaction_warmup(options),
        interaction_iters(options),
        || {
            let action = waveform_action_for_step(step);
            step = step.saturating_add(1);
            controller.apply_native_ui_action(action);
            controller.prepare_native_frame(false);
            let _: NativeAppModel = controller.project_native_app_model();
            let _: NativeMotionModel = controller.project_native_motion_model();
            Ok(())
        },
    )
}

/// Return a deterministic waveform action for a benchmark step index.
fn waveform_action_for_step(step: usize) -> NativeUiAction {
    match step % 6 {
        0 => NativeUiAction::SeekWaveform {
            position_milli: 320,
        },
        1 => NativeUiAction::SetWaveformCursor {
            position_milli: 480,
        },
        2 => NativeUiAction::SetWaveformSelectionRange {
            start_milli: 220,
            end_milli: 660,
        },
        3 => NativeUiAction::ZoomWaveform {
            zoom_in: true,
            steps: 2,
        },
        4 => NativeUiAction::ZoomWaveformToSelection,
        _ => NativeUiAction::ZoomWaveformFull,
    }
}

/// Prime map cache fields so interaction benchmarks avoid cold-start query cost.
fn prime_map_cache_for_benchmark(controller: &mut AppController) -> Result<(), String> {
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
            sample_id: String::from("sample-000000"),
            x: -0.4,
            y: 0.6,
            cluster_id: Some(1),
        },
        MapPoint {
            sample_id: String::from("sample-000001"),
            x: 0.5,
            y: -0.35,
            cluster_id: Some(2),
        },
    ];
    controller.ui.map.cached_points_revision = 1;
    Ok(())
}

fn interaction_query_for_step(step: usize) -> &'static str {
    const SEARCH_QUERIES: [&str; 4] = ["sample_", "sample_00", "sample_000", "sample_001"];
    SEARCH_QUERIES[step % SEARCH_QUERIES.len()]
}

fn interaction_filter_for_step(step: usize) -> TriageFlagFilter {
    match step % 3 {
        0 => TriageFlagFilter::All,
        1 => TriageFlagFilter::Keep,
        _ => TriageFlagFilter::Trash,
    }
}

fn interaction_sort_for_step(step: usize) -> SampleBrowserSort {
    if step.is_multiple_of(2) {
        SampleBrowserSort::ListOrder
    } else {
        SampleBrowserSort::PlaybackAgeDesc
    }
}

/// GUI benchmark behavior and interaction sequencing tests.
#[cfg(test)]
#[path = "gui_tests.rs"]
mod tests;
