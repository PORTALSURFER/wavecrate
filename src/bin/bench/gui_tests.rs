use super::interactions::{
    adjacent_waveform_action_for_step, execute_interaction_step, interaction_filter_for_step,
    interaction_query_for_step, interaction_sort_for_step, waveform_action_for_step,
};
use super::*;
use sempal::app_core::actions::NativeUiAction;

/// Panic with context if a GUI benchmark test setup step fails.
fn must<T, E: std::fmt::Display>(result: Result<T, E>, context: &str) -> T {
    match result {
        Ok(value) => value,
        Err(err) => panic!("{context}: {err}"),
    }
}

/// Install an isolated app root for benchmark tests.
fn with_isolated_app_config() {
    let config_root = must(tempfile::tempdir(), "create isolated app config directory");
    must(
        sempal::app_dirs::set_app_root_override(config_root.path().to_path_buf()),
        "configure isolated app root",
    );
    std::mem::forget(config_root);
}

/// Build tiny benchmark options used by GUI benchmark tests.
fn tiny_options() -> BenchOptions {
    BenchOptions {
        gui_rows: 4,
        gui_interaction_rows: 4,
        gui_interaction_iters: 2,
        warmup_iters: 1,
        measure_iters: 1,
        ..BenchOptions::default()
    }
}

/// Ensure GUI benchmark seeding clamps to at least one synthetic row.
#[test]
fn run_gui_benchmark_uses_one_row_when_gui_rows_is_zero() {
    let mut options = tiny_options();
    with_isolated_app_config();
    options.gui_rows = 0;
    options.gui_interaction_rows = 0;
    let report = must(run(&options), "gui benchmark with minimum row count");
    assert_eq!(report.seeded_rows, 1);
    assert_eq!(report.app_model_projection.measure_iters, 1);
    assert_eq!(report.hover_latency.measure_iters, 2);
}

/// Ensure interaction-step sequencing rotates search/filter/sort settings.
#[test]
fn interaction_step_cycles_search_filter_and_sort() {
    let options = tiny_options();
    with_isolated_app_config();
    let mut workspace = must(
        build_controller_with_db_rows(&options),
        "build gui workspace",
    );
    must(
        wait_for_rows(&mut workspace.controller, options.gui_rows),
        "rows seeded",
    );

    for step in 0..6usize {
        execute_interaction_step(&mut workspace.controller, step);
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

/// Ensure waveform interaction benchmark actions cover the expected sequence.
#[test]
fn waveform_action_sequence_covers_expected_native_actions() {
    assert!(matches!(
        waveform_action_for_step(0),
        NativeUiAction::SeekWaveform { .. }
    ));
    assert!(matches!(
        waveform_action_for_step(1),
        NativeUiAction::SetWaveformCursor { .. }
    ));
    assert!(matches!(
        waveform_action_for_step(2),
        NativeUiAction::SetWaveformSelectionRange { .. }
    ));
    assert!(matches!(
        waveform_action_for_step(3),
        NativeUiAction::ZoomWaveform { .. }
    ));
    assert!(matches!(
        waveform_action_for_step(4),
        NativeUiAction::ZoomWaveformToSelection
    ));
    assert!(matches!(
        waveform_action_for_step(5),
        NativeUiAction::ZoomWaveformFull
    ));
}

/// Ensure adjacent waveform benchmark action sequencing is deterministic.
#[test]
fn adjacent_waveform_action_sequence_covers_expected_native_actions() {
    assert!(matches!(
        adjacent_waveform_action_for_step(0),
        NativeUiAction::SeekWaveform {
            position_milli: 380
        }
    ));
    assert!(matches!(
        adjacent_waveform_action_for_step(1),
        NativeUiAction::SeekWaveform {
            position_milli: 410
        }
    ));
    assert!(matches!(
        adjacent_waveform_action_for_step(2),
        NativeUiAction::ZoomWaveform {
            zoom_in: true,
            steps: 1
        }
    ));
    assert!(matches!(
        adjacent_waveform_action_for_step(3),
        NativeUiAction::ZoomWaveform {
            zoom_in: false,
            steps: 1
        }
    ));
}
