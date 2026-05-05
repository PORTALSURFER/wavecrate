//! Seeded workspace helpers for isolated GUI benchmark runs.

use super::BenchOptions;
use hound::{SampleFormat, WavSpec, WavWriter};
use sempal::app_core::controller::{AppController, AppControllerNativeRuntimeExt};
use sempal::waveform::WaveformRenderer;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tempfile::TempDir;

/// Number of frames written into each synthetic GUI benchmark wav.
///
/// This stays above the dense-column cache threshold for the default 32px
/// waveform renderer so adjacent pan/zoom benchmarks actually exercise the
/// retained zoom-cache path.
const BENCH_WAV_FRAME_COUNT: usize = 256;

/// Scoped benchmark workspace that keeps seed artifacts alive for the benchmark duration.
pub(super) struct BenchWorkspace {
    /// Temporary directory that stores synthetic source files and DB state.
    _temp_root: TempDir,
    /// Controller under test for the current benchmark run.
    pub(super) controller: AppController,
}

/// Build an isolated controller seeded with a synthetic WAV source tree.
pub(super) fn build_controller_with_db_rows(
    options: &BenchOptions,
) -> Result<BenchWorkspace, String> {
    let temp_root =
        tempfile::tempdir().map_err(|err| format!("Create temp source dir failed: {err}"))?;
    configure_benchmark_app_root(&temp_root)?;
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let source_root = create_source_root(temp_root.path())?;
    seed_source_tree(&source_root, seed_row_target(options))?;
    controller
        .add_source_from_path(source_root)
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

/// Ensure at least one synthetic row is visible before the benchmark proceeds.
pub(super) fn seed_rows(controller: &mut AppController, rows: usize) -> Result<usize, String> {
    let effective_rows = rows.max(1);
    wait_for_rows(controller, effective_rows)?;
    Ok(effective_rows)
}

/// Wait for the synthetic source rows to become visible in the projected browser model.
pub(super) fn wait_for_rows(controller: &mut AppController, target: usize) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        controller.prepare_native_frame(false);
        if observed_visible_rows(controller) >= target {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    finalize_wait_for_rows(controller, target)
}

fn configure_benchmark_app_root(_temp_root: &TempDir) -> Result<(), String> {
    #[cfg(not(test))]
    {
        let bench_app_root = _temp_root.path().join(".sempal-bench");
        sempal::app_dirs::set_app_root_override(bench_app_root.clone()).map_err(|err| {
            format!(
                "Configure isolated benchmark app root {} failed: {err}",
                bench_app_root.display()
            )
        })?;
    }
    Ok(())
}

fn create_source_root(temp_root: &Path) -> Result<PathBuf, String> {
    let source_root = temp_root.join("gui-source");
    fs::create_dir_all(&source_root)
        .map_err(|err| format!("Create source dir {} failed: {err}", source_root.display()))?;
    Ok(source_root)
}

fn seed_source_tree(source_root: &Path, row_count: usize) -> Result<(), String> {
    for (row, file_name) in seeded_wav_filenames(row_count).into_iter().enumerate() {
        write_seed_wav(&source_root.join(&file_name), row as i64)
            .map_err(|err| format!("Seed test audio failed: {err}"))?;
    }
    Ok(())
}

fn seed_row_target(options: &BenchOptions) -> usize {
    options.gui_rows.max(options.gui_interaction_rows).max(1)
}

fn finalize_wait_for_rows(controller: &mut AppController, target: usize) -> Result<(), String> {
    if observed_visible_rows(controller) >= target {
        return Ok(());
    }
    let model = controller.project_native_app_model();
    Err(format_timeout_message(
        &model,
        observed_visible_rows(controller),
        target,
    ))
}

fn format_timeout_message(
    model: &sempal::app_core::actions::NativeAppModel,
    visible_rows: usize,
    target: usize,
) -> String {
    format!(
        "Timed out waiting for GUI rows: {} < {} | sources: {}, visible_count: {}, columns: [T:{},N:{},K:{}], selected: {}",
        visible_rows,
        target,
        model.sources.rows.len(),
        model.browser.visible_count,
        model.columns[0].item_count,
        model.columns[1].item_count,
        model.columns[2].item_count,
        model
            .browser
            .selected_visible_row
            .map_or_else(|| "none".to_string(), |row| row.to_string())
    )
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
        sample_rate: BENCH_WAV_FRAME_COUNT as u32,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };
    let mut writer = WavWriter::create(path, spec).map_err(|err| err.to_string())?;
    let phase = (seed as f32 * 0.137).fract() * std::f32::consts::TAU;
    for frame in 0..BENCH_WAV_FRAME_COUNT {
        let t = frame as f32 / BENCH_WAV_FRAME_COUNT as f32;
        let sample = ((t * std::f32::consts::TAU * 3.0 + phase).sin() * 0.72
            + (t * std::f32::consts::TAU * 7.0 + phase * 0.5).sin() * 0.28)
            .clamp(-1.0, 1.0);
        writer.write_sample(sample).map_err(|err| err.to_string())?;
    }
    writer.finalize().map_err(|err| err.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Seeded benchmark wavs should be long enough to enter the dense cache path.
    fn seeded_wavs_exceed_dense_cache_threshold() {
        let root = tempfile::tempdir().expect("tempdir");
        let wav_path = root.path().join("seed.wav");
        write_seed_wav(&wav_path, 7).expect("seed wav");

        let reader = hound::WavReader::open(&wav_path).expect("open seeded wav");
        assert_eq!(reader.duration() as usize, BENCH_WAV_FRAME_COUNT);
        assert!(BENCH_WAV_FRAME_COUNT > 48);
    }
}
