//! GUI-oriented benchmark scenarios for the native controller.

use super::{options::BenchOptions, stats};
use hound::{SampleFormat, WavSpec, WavWriter};
use sempal::app_core::actions::{NativeAppModel, NativeMotionModel};
use sempal::app_core::controller::{AppController, AppControllerNativeRuntimeExt};
use sempal::app_core::state::{SampleBrowserSort, TriageFlagFilter};
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
    Ok(GuiBenchResult {
        seeded_rows,
        app_model_projection,
        motion_model_projection,
        interactive_projection,
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
        if controller.visible_browser_len() >= target {
            return Ok(());
        }
        controller.prepare_native_frame(false);
        std::thread::sleep(Duration::from_millis(5));
    }
    if controller.visible_browser_len() >= target {
        return Ok(());
    }
    let model = controller.project_native_app_model();
    Err(format!(
        "Timed out waiting for GUI rows: {} < {}",
        controller.visible_browser_len(),
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

fn build_controller_with_db_rows(options: &BenchOptions) -> Result<BenchWorkspace, String> {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let temp_root =
        tempfile::tempdir().map_err(|err| format!("Create temp source dir failed: {err}"))?;
    let source_root = temp_root.path().join("gui-source");
    fs::create_dir_all(&source_root)
        .map_err(|err| format!("Create source dir {} failed: {err}", source_root.display()))?;

    for (row, file_name) in seeded_wav_filenames(options.gui_rows.max(1))
        .into_iter()
        .enumerate()
    {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn with_isolated_app_config() {
        let config_root = tempfile::tempdir().expect("create isolated app config directory");
        sempal::app_dirs::set_app_root_override(config_root.path().to_path_buf())
            .expect("configure isolated app root");
        std::mem::forget(config_root);
    }

    fn tiny_options() -> BenchOptions {
        BenchOptions {
            gui_rows: 4,
            warmup_iters: 1,
            measure_iters: 1,
            ..BenchOptions::default()
        }
    }

    #[test]
    fn run_gui_benchmark_uses_one_row_when_gui_rows_is_zero() {
        let mut options = tiny_options();
        with_isolated_app_config();
        options.gui_rows = 0;
        let report = run(&options).expect("gui benchmark with minimum row count");
        assert_eq!(report.seeded_rows, 1);
        assert_eq!(report.app_model_projection.measure_iters, 1);
    }

    #[test]
    fn interaction_step_cycles_search_filter_and_sort() {
        let options = tiny_options();
        with_isolated_app_config();
        let mut workspace = build_controller_with_db_rows(&options).expect("build gui workspace");
        wait_for_rows(&mut workspace.controller, options.gui_rows).expect("rows seeded");

        for step in 0..6usize {
            execute_interaction_step(&mut workspace.controller, step).expect("interaction step");
            assert_eq!(
                workspace.controller.ui.browser.search_query,
                interaction_query_for_step(step)
            );
            assert_eq!(
                workspace.controller.ui.browser.filter,
                interaction_filter_for_step(step)
            );
            assert_eq!(
                workspace.controller.ui.browser.sort,
                interaction_sort_for_step(step)
            );
        }
    }
}
