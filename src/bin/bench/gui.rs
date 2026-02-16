//! GUI-oriented benchmark scenarios for the native controller.

use super::{options::BenchOptions, stats};
use sempal::app_core::actions::{NativeAppModel, NativeMotionModel};
use sempal::app_core::controller::{AppController, AppControllerNativeRuntimeExt};
use sempal::app_core::state::{SampleBrowserSort, TriageFlagFilter};
use sempal::sample_sources::SourceDatabase;
use sempal::waveform::WaveformRenderer;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

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

/// Run GUI benchmark actions and summarize performance characteristics.
pub(super) fn run(options: &BenchOptions) -> Result<GuiBenchResult, String> {
    let mut controller = build_controller_with_db_rows(options)?;
    let seeded_rows = seed_rows(&mut controller, options.gui_rows)?;
    let app_model_projection = stats::bench_action(options, || {
        controller.prepare_native_frame();
        let _: NativeAppModel = controller.project_native_app_model();
        Ok(())
    })?;
    let motion_model_projection = stats::bench_action(options, || {
        controller.prepare_native_frame();
        let _: NativeMotionModel = controller.project_native_motion_model();
        Ok(())
    })?;
    let mut interaction_step = 0usize;
    let interactive_projection = stats::bench_action(options, || {
        execute_interaction_step(&mut controller, interaction_step)?;
        interaction_step = interaction_step.saturating_add(1);
        controller.prepare_native_frame();
        let _: NativeAppModel = controller.project_native_app_model();
        let _: NativeMotionModel = controller.project_native_motion_model();
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
        if controller.wav_entries_len() >= target {
            return Ok(());
        }
        controller.poll_background_jobs();
        std::thread::sleep(Duration::from_millis(5));
    }
    if controller.wav_entries_len() >= target {
        return Ok(());
    }
    Err(format!(
        "Timed out waiting for GUI rows: {} < {}",
        controller.wav_entries_len(),
        target
    ))
}

fn build_controller_with_db_rows(options: &BenchOptions) -> Result<AppController, String> {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let temp_root = tempfile::tempdir()
        .map_err(|err| format!("Create temp source dir failed: {err}"))?
        .into_path();
    let source_root = temp_root.join("gui-source");
    fs::create_dir_all(&source_root)
        .map_err(|err| format!("Create source dir {} failed: {err}", source_root.display()))?;

    let db = SourceDatabase::open(&source_root)
        .map_err(|err| format!("Create benchmark source db failed: {err}"))?;
    let target_rows = options.gui_rows.max(1);
    {
        let mut batch = db
            .write_batch()
            .map_err(|err| format!("Open DB write batch failed: {err}"))?;
        for row in 0..target_rows {
            let path = PathBuf::from(format!("sample_{row:06}.wav"));
            batch
                .upsert_file(&path, 1_024, row as i64 * 1_000_000)
                .map_err(|err| format!("Seed DB row {row} failed: {err}"))?;
        }
        batch
            .commit()
            .map_err(|err| format!("Commit DB seed batch failed: {err}"))?;
    }

    controller
        .add_source_from_path(source_root)
        .map_err(|err| format!("Add benchmark source failed: {err}"))?;
    Ok(controller)
}

fn execute_interaction_step(controller: &mut AppController, step: usize) -> Result<(), String> {
    const SEARCH_QUERIES: [&str; 4] = ["sample_", "sample_00", "sample_000", "sample_001"];
    let total_rows = controller.wav_entries_len();
    if total_rows == 0 {
        return Err("No GUI rows available for interaction bench".to_string());
    }

    let row = step % total_rows;
    let query = SEARCH_QUERIES[row % SEARCH_QUERIES.len()];
    controller.set_browser_search(query);

    let filter = if step % 3 == 0 {
        TriageFlagFilter::All
    } else if step % 3 == 1 {
        TriageFlagFilter::Keep
    } else {
        TriageFlagFilter::Trash
    };
    controller.set_browser_filter(filter);

    if step % 2 == 0 {
        controller.set_browser_sort(SampleBrowserSort::ListOrder);
    } else {
        controller.set_browser_sort(SampleBrowserSort::PlaybackAgeDesc);
    }

    controller.select_column_by_index(step % 3);
    Ok(())
}
