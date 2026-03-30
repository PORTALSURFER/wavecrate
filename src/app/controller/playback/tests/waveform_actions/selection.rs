use super::*;
use std::path::PathBuf;

#[test]
fn native_waveform_selection_begin_does_not_snap_to_visible_playhead() {
    let (mut controller, _source) = test_support::dummy_controller();
    seed_waveform_for_zoom(&mut controller);
    controller.ui.waveform.view = crate::app::state::WaveformView {
        start: 0.2,
        end: 0.4,
    };
    controller.ui.waveform.playhead.visible = true;
    controller.ui.waveform.playhead.position = 0.95;

    controller.apply_native_ui_action(NativeUiAction::BeginWaveformSelectionAt {
        anchor_micros: 300_000,
    });

    assert!((controller.ui.waveform.view.start - 0.2).abs() < 1.0e-6);
    assert!((controller.ui.waveform.view.end - 0.4).abs() < 1.0e-6);
}

#[test]
fn native_waveform_selection_update_does_not_snap_to_visible_playhead() {
    let (mut controller, _source) = test_support::dummy_controller();
    seed_waveform_for_zoom(&mut controller);
    controller.ui.waveform.view = crate::app::state::WaveformView {
        start: 0.2,
        end: 0.4,
    };
    controller.ui.waveform.playhead.visible = true;
    controller.ui.waveform.playhead.position = 0.95;

    controller.apply_native_ui_action(NativeUiAction::SetWaveformSelectionRange {
        start_micros: 300_000,
        end_micros: 350_000,
        snap_override: false,
        preserve_view_edge: false,
    });

    assert!((controller.ui.waveform.view.start - 0.2).abs() < 1.0e-6);
    assert!((controller.ui.waveform.view.end - 0.4).abs() < 1.0e-6);
}

/// Cursor updates should no-op when the cursor is unchanged and waveform is focused.
#[test]
fn set_waveform_cursor_milli_noops_when_unchanged_and_focused() {
    let (mut controller, _source) = test_support::dummy_controller();
    controller.ui.focus.context = crate::app::state::FocusContext::Waveform;
    controller.ui.waveform.cursor = Some(0.5);
    let previous_nav = std::time::Instant::now() - std::time::Duration::from_millis(2);
    controller.ui.waveform.cursor_last_navigation_at = Some(previous_nav);

    controller.set_waveform_cursor_milli(500);

    assert_eq!(controller.ui.waveform.cursor, Some(0.5));
    assert_eq!(
        controller.ui.waveform.cursor_last_navigation_at,
        Some(previous_nav)
    );
}

/// Selection updates should no-op when the range is unchanged and waveform is focused.
#[test]
fn set_waveform_selection_range_milli_noops_when_unchanged_and_focused() {
    let (mut controller, _source) = test_support::dummy_controller();
    controller.ui.focus.context = crate::app::state::FocusContext::Waveform;
    let range = SelectionRange::new(0.25, 0.75);
    controller.selection_state.range.set_range(Some(range));
    controller.ui.waveform.selection = Some(range);

    controller.set_waveform_selection_range_milli(250, 750);

    assert_eq!(controller.selection_state.range.range(), Some(range));
    assert_eq!(controller.ui.waveform.selection, Some(range));
}

#[test]
/// Selection-edge resize from native milli actions should BPM-snap the moving endpoint.
fn set_waveform_selection_range_milli_snaps_resize_endpoint_when_bpm_snap_enabled() {
    let (mut controller, source) = test_support::dummy_controller();
    controller.sample_view.wav.loaded_audio = Some(LoadedAudio {
        source_id: source.id.clone(),
        root: source.root.clone(),
        relative_path: PathBuf::from("snap.wav"),
        bytes: Vec::new().into(),
        duration_seconds: 4.0,
        sample_rate: 48_000,
    });
    controller.ui.waveform.bpm_snap_enabled = true;
    controller.ui.waveform.relative_bpm_grid_enabled = true;
    controller.ui.waveform.bpm_value = Some(120.0);
    let range = SelectionRange::new(0.2, 0.8);
    controller.selection_state.range.set_range(Some(range));
    controller.ui.waveform.selection = Some(range);

    controller.set_waveform_selection_range_milli(800, 333);

    let updated = controller.ui.waveform.selection;
    assert!(updated.is_some());
    let updated = updated.unwrap_or(range);
    assert!((updated.start() - 0.3).abs() < 0.001);
    assert!((updated.end() - 0.8).abs() < 0.001);
}

#[test]
fn set_waveform_selection_range_milli_snaps_resize_endpoint_to_global_grid_when_relative_mode_off()
{
    let (mut controller, source) = test_support::dummy_controller();
    controller.sample_view.wav.loaded_audio = Some(LoadedAudio {
        source_id: source.id.clone(),
        root: source.root.clone(),
        relative_path: PathBuf::from("snap_resize_global.wav"),
        bytes: Vec::new().into(),
        duration_seconds: 4.0,
        sample_rate: 48_000,
    });
    controller.ui.waveform.bpm_snap_enabled = true;
    controller.ui.waveform.relative_bpm_grid_enabled = false;
    controller.ui.waveform.bpm_value = Some(120.0);
    let range = SelectionRange::new(0.2, 0.8);
    controller.selection_state.range.set_range(Some(range));
    controller.ui.waveform.selection = Some(range);

    controller.set_waveform_selection_range_milli(800, 333);

    let updated = controller
        .ui
        .waveform
        .selection
        .expect("selection should remain");
    assert!((updated.start() - 0.375).abs() < 0.001);
    assert!((updated.end() - 0.8).abs() < 0.001);
}

#[test]
fn set_waveform_selection_range_milli_snaps_new_selection_from_exact_anchor_when_bpm_enabled() {
    let (mut controller, source) = test_support::dummy_controller();
    controller.sample_view.wav.loaded_audio = Some(LoadedAudio {
        source_id: source.id.clone(),
        root: source.root.clone(),
        relative_path: PathBuf::from("snap_new_selection.wav"),
        bytes: Vec::new().into(),
        duration_seconds: 4.0,
        sample_rate: 48_000,
    });
    controller.ui.waveform.bpm_snap_enabled = true;
    controller.ui.waveform.relative_bpm_grid_enabled = true;
    controller.ui.waveform.bpm_value = Some(120.0);

    controller.set_waveform_selection_range_milli(310, 440);

    let updated = controller
        .ui
        .waveform
        .selection
        .expect("selection should be created");
    assert!((updated.start() - 0.31).abs() < 0.001);
    assert!((updated.end() - 0.435).abs() < 0.001);
    assert!((controller.ui.waveform.last_bpm_grid_origin - 0.31).abs() < 1.0e-6);
}

#[test]
fn set_waveform_selection_range_milli_snaps_new_selection_to_global_grid_when_relative_mode_off() {
    let (mut controller, source) = test_support::dummy_controller();
    controller.sample_view.wav.loaded_audio = Some(LoadedAudio {
        source_id: source.id.clone(),
        root: source.root.clone(),
        relative_path: PathBuf::from("snap_new_selection_global.wav"),
        bytes: Vec::new().into(),
        duration_seconds: 4.0,
        sample_rate: 48_000,
    });
    controller.ui.waveform.bpm_snap_enabled = true;
    controller.ui.waveform.relative_bpm_grid_enabled = false;
    controller.ui.waveform.bpm_value = Some(120.0);

    controller.set_waveform_selection_range_milli(310, 440);

    let updated = controller
        .ui
        .waveform
        .selection
        .expect("selection should be created");
    assert!((updated.start() - 0.31).abs() < 0.001);
    assert!((updated.end() - 0.5).abs() < 0.001);
}

/// In-bounds selection updates should still BPM-snap even when they land on the visible edge.
#[test]
fn set_waveform_selection_range_milli_snaps_visible_edge_without_preserve_flag() {
    let (mut controller, source) = test_support::dummy_controller();
    controller.sample_view.wav.loaded_audio = Some(LoadedAudio {
        source_id: source.id.clone(),
        root: source.root.clone(),
        relative_path: PathBuf::from("snap_view_edge.wav"),
        bytes: Vec::new().into(),
        duration_seconds: 4.0,
        sample_rate: 48_000,
    });
    controller.ui.waveform.bpm_snap_enabled = true;
    controller.ui.waveform.bpm_value = Some(60.0);
    controller.ui.waveform.view = crate::app::state::WaveformView {
        start: 0.18,
        end: 0.68,
    };
    let range = SelectionRange::new(0.3, 0.5);
    controller.selection_state.range.set_range(Some(range));
    controller.ui.waveform.selection = Some(range);

    controller.set_waveform_selection_range_milli(500, 180);

    let updated = controller
        .ui
        .waveform
        .selection
        .expect("selection should remain active");
    assert!((updated.start() - 0.25).abs() < 0.001);
    assert!((updated.end() - 0.5).abs() < 0.001);
}

/// Native out-of-bounds drags should pin to the visible left waveform edge.
#[test]
fn set_waveform_selection_range_milli_preserves_left_view_edge_when_requested() {
    let (mut controller, source) = test_support::dummy_controller();
    controller.sample_view.wav.loaded_audio = Some(LoadedAudio {
        source_id: source.id.clone(),
        root: source.root.clone(),
        relative_path: PathBuf::from("snap_view_edge_preserved.wav"),
        bytes: Vec::new().into(),
        duration_seconds: 4.0,
        sample_rate: 48_000,
    });
    controller.ui.waveform.bpm_snap_enabled = true;
    controller.ui.waveform.bpm_value = Some(60.0);
    controller.ui.waveform.view = crate::app::state::WaveformView {
        start: 0.18,
        end: 0.68,
    };
    let range = SelectionRange::new(0.3, 0.5);
    controller.selection_state.range.set_range(Some(range));
    controller.ui.waveform.selection = Some(range);

    controller.set_waveform_selection_range_milli_with_drag_policy(500, 180, false, true);

    let updated = controller
        .ui
        .waveform
        .selection
        .expect("selection should remain active");
    assert!((updated.start() - 0.18).abs() < 0.001);
    assert!((updated.end() - 0.5).abs() < 0.001);
}

/// Selection-translation updates should snap the moved range to BPM steps.
#[test]
fn set_waveform_selection_range_milli_snaps_translated_range_when_bpm_snap_enabled() {
    let (mut controller, source) = test_support::dummy_controller();
    controller.sample_view.wav.loaded_audio = Some(LoadedAudio {
        source_id: source.id.clone(),
        root: source.root.clone(),
        relative_path: PathBuf::from("snap.wav"),
        bytes: Vec::new().into(),
        duration_seconds: 4.0,
        sample_rate: 48_000,
    });
    controller.ui.waveform.bpm_snap_enabled = true;
    controller.ui.waveform.relative_bpm_grid_enabled = true;
    controller.ui.waveform.bpm_value = Some(120.0);
    let range = SelectionRange::new(0.2, 0.4);
    controller.selection_state.range.set_range(Some(range));
    controller.ui.waveform.selection = Some(range);

    controller.set_waveform_selection_range_milli(260, 460);

    let updated = controller.ui.waveform.selection;
    assert!(updated.is_some());
    let updated = updated.unwrap_or(range);
    assert!((updated.start() - 0.2).abs() < 0.001);
    assert!((updated.end() - 0.4).abs() < 0.001);
}

#[test]
fn set_waveform_selection_range_milli_snaps_translated_range_to_global_grid_when_relative_mode_off()
{
    let (mut controller, source) = test_support::dummy_controller();
    controller.sample_view.wav.loaded_audio = Some(LoadedAudio {
        source_id: source.id.clone(),
        root: source.root.clone(),
        relative_path: PathBuf::from("snap_global_translate.wav"),
        bytes: Vec::new().into(),
        duration_seconds: 4.0,
        sample_rate: 48_000,
    });
    controller.ui.waveform.bpm_snap_enabled = true;
    controller.ui.waveform.relative_bpm_grid_enabled = false;
    controller.ui.waveform.bpm_value = Some(120.0);
    let range = SelectionRange::new(0.2, 0.4);
    controller.selection_state.range.set_range(Some(range));
    controller.ui.waveform.selection = Some(range);

    controller.set_waveform_selection_range_milli(260, 460);

    let updated = controller
        .ui
        .waveform
        .selection
        .expect("selection should remain");
    assert!((updated.start() - 0.25).abs() < 0.001);
    assert!((updated.end() - 0.45).abs() < 0.001);
}
