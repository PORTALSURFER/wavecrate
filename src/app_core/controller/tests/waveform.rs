use super::*;
use crate::app_core::controller::build_named_gui_fixture_controller;
use hound::{SampleFormat, WavReader, WavSpec, WavWriter};
use std::path::Path;

#[test]
fn apply_native_waveform_normalize_routes_to_controller_behavior() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    controller.apply_native_ui_action(NativeUiAction::NormalizeWaveformSelectionOrSample);

    assert!(
        controller
            .ui
            .status
            .text
            .contains("Load a sample to normalize it"),
        "status was {:?}",
        controller.ui.status.text
    );
}

#[test]
fn apply_native_waveform_crop_routes_to_controller_behavior() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    controller.apply_native_ui_action(NativeUiAction::CropWaveformSelection);

    assert!(
        controller
            .ui
            .status
            .text
            .contains("Load a sample to edit it"),
        "status was {:?}",
        controller.ui.status.text
    );
}

#[test]
fn apply_native_waveform_trim_routes_to_controller_behavior() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    controller.apply_native_ui_action(NativeUiAction::TrimWaveformSelection);

    assert!(
        controller
            .ui
            .status
            .text
            .contains("Load a sample to edit it"),
        "status was {:?}",
        controller.ui.status.text
    );
}

#[test]
fn apply_native_waveform_commit_edit_fades_routes_to_controller_behavior() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    controller.set_edit_selection_range(
        crate::selection::SelectionRange::new(0.2, 0.6).with_fade_out(0.25, 0.2),
    );
    controller.apply_native_ui_action(NativeUiAction::CommitWaveformEditFades);

    assert!(
        controller
            .ui
            .status
            .text
            .contains("Load a sample to edit it"),
        "status was {:?}",
        controller.ui.status.text
    );
}

#[test]
fn apply_native_waveform_silence_slice_detect_routes_to_controller_behavior() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    controller.apply_native_ui_action(NativeUiAction::DetectWaveformSilenceSlices);

    assert!(
        controller
            .ui
            .status
            .text
            .contains("Load a sample before slicing"),
        "status was {:?}",
        controller.ui.status.text
    );
}

#[test]
fn apply_native_waveform_exact_duplicate_detect_routes_to_controller_behavior() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    controller.apply_native_ui_action(NativeUiAction::DetectWaveformExactDuplicateSlices);

    assert!(
        controller
            .ui
            .status
            .text
            .contains("Load a sample before slicing"),
        "status was {:?}",
        controller.ui.status.text
    );
}

#[test]
fn apply_native_waveform_clean_duplicates_routes_to_controller_behavior() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    controller.apply_native_ui_action(NativeUiAction::CleanWaveformExactDuplicateSlices);

    assert!(
        controller
            .ui
            .status
            .text
            .contains("Run Exact Dedupe before cleaning duplicates"),
        "status was {:?}",
        controller.ui.status.text
    );
}

#[test]
fn apply_native_waveform_smart_scale_routes_to_controller_behavior() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    controller.set_loaded_audio_duration_for_tests(4.0);
    controller.set_selection_range(crate::selection::SelectionRange::new(0.0, 0.25));
    controller.set_bpm_value(150.0);

    controller.apply_native_ui_action(NativeUiAction::SetWaveformSelectionRangeSmartScale {
        start_micros: 0,
        end_micros: 500_000,
    });

    assert_eq!(
        controller.ui.waveform.selection,
        Some(crate::selection::SelectionRange::new(0.0, 0.5))
    );
    assert_eq!(controller.ui.waveform.bpm_value, Some(120.0));
    assert!((controller.settings.controls.bpm_value - 150.0).abs() < f32::EPSILON);
    assert!(controller.is_selection_dragging());

    controller.apply_native_ui_action(NativeUiAction::FinishWaveformSelectionSmartScaleDrag);

    assert!(!controller.is_selection_dragging());
    assert!((controller.settings.controls.bpm_value - 120.0).abs() < 0.1);
}

#[test]
fn apply_native_waveform_selection_range_finish_commits_one_undo_step() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let before = crate::selection::SelectionRange::new(0.2, 0.6);
    let after = crate::selection::SelectionRange::new(0.2, 0.7);
    controller.set_selection_range(before);

    controller.apply_native_ui_action(NativeUiAction::SetWaveformSelectionRange {
        start_micros: 200_000,
        end_micros: 700_000,
        snap_override: false,
        preserve_view_edge: false,
    });
    controller.apply_native_ui_action(NativeUiAction::FinishWaveformSelectionRangeDrag);

    assert_eq!(controller.ui.waveform.selection, Some(after));

    controller.undo();
    assert_eq!(controller.ui.waveform.selection, Some(before));

    controller.redo();
    assert_eq!(controller.ui.waveform.selection, Some(after));
}

#[test]
fn apply_native_waveform_edit_selection_finish_commits_one_undo_step() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let before = crate::selection::SelectionRange::new(0.2, 0.6);
    let after = crate::selection::SelectionRange::new(0.2, 0.7);
    controller.set_edit_selection_range(before);

    controller.apply_native_ui_action(NativeUiAction::SetWaveformEditSelectionRange {
        start_micros: 200_000,
        end_micros: 700_000,
        preserve_view_edge: false,
    });
    controller.apply_native_ui_action(NativeUiAction::FinishWaveformEditSelectionDrag);

    assert_eq!(controller.ui.waveform.edit_selection, Some(after));

    controller.undo();
    assert_eq!(controller.ui.waveform.edit_selection, Some(before));

    controller.redo();
    assert_eq!(controller.ui.waveform.edit_selection, Some(after));
}

#[test]
fn apply_native_waveform_edit_fade_finish_commits_one_undo_step() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let before = crate::selection::SelectionRange::new(0.2, 0.6).with_fade_out(0.25, 0.2);
    controller.set_edit_selection_range(before);

    controller
        .apply_native_ui_action(NativeUiAction::SetWaveformEditFadeOutCurve { curve_milli: 750 });
    controller.apply_native_ui_action(NativeUiAction::FinishWaveformEditFadeDrag);

    let updated = controller
        .ui
        .waveform
        .edit_selection
        .and_then(|selection| selection.fade_out())
        .expect("fade-out after drag");
    assert!((updated.curve - 0.75).abs() < 0.001);

    controller.undo();
    let undone = controller
        .ui
        .waveform
        .edit_selection
        .and_then(|selection| selection.fade_out())
        .expect("fade-out after undo");
    assert!((undone.curve - 0.2).abs() < 0.001);

    controller.redo();
    let redone = controller
        .ui
        .waveform
        .edit_selection
        .and_then(|selection| selection.fade_out())
        .expect("fade-out after redo");
    assert!((redone.curve - 0.75).abs() < 0.001);
}

#[test]
fn clear_waveform_edit_selection_is_undoable() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let selection = crate::selection::SelectionRange::new(0.2, 0.6);
    controller.set_edit_selection_range(selection);

    controller.apply_native_ui_action(NativeUiAction::ClearWaveformEditSelection);
    assert!(controller.ui.waveform.edit_selection.is_none());

    controller.undo();
    assert_eq!(controller.ui.waveform.edit_selection, Some(selection));
}

#[test]
fn no_op_waveform_selection_range_drag_does_not_create_undo_entry() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let selection = crate::selection::SelectionRange::new(0.2, 0.6);
    controller.set_selection_range(selection);

    controller.apply_native_ui_action(NativeUiAction::SetWaveformSelectionRange {
        start_micros: 200_000,
        end_micros: 600_000,
        snap_override: false,
        preserve_view_edge: false,
    });
    controller.apply_native_ui_action(NativeUiAction::FinishWaveformSelectionRangeDrag);
    controller.undo();

    assert_eq!(controller.ui.waveform.selection, Some(selection));
    assert!(controller.ui.status.text.contains("Nothing to undo"));
}

#[test]
fn no_op_waveform_edit_fade_drag_does_not_create_undo_entry() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let selection = crate::selection::SelectionRange::new(0.2, 0.6).with_fade_out(0.25, 0.2);
    controller.set_edit_selection_range(selection);

    controller
        .apply_native_ui_action(NativeUiAction::SetWaveformEditFadeOutCurve { curve_milli: 200 });
    controller.apply_native_ui_action(NativeUiAction::FinishWaveformEditFadeDrag);
    controller.undo();

    assert_eq!(controller.ui.waveform.edit_selection, Some(selection));
    assert!(controller.ui.status.text.contains("Nothing to undo"));
}

#[test]
fn apply_native_waveform_view_center_routes_to_controller_behavior() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    controller.ui.waveform.view = crate::app::state::WaveformView {
        start: 0.2,
        end: 0.4,
    };

    controller.apply_native_ui_action(NativeUiAction::SetWaveformViewCenter {
        center_micros: 700_000,
        center_nanos: None,
    });

    assert!((controller.ui.waveform.view.start - 0.6).abs() < 1.0e-6);
    assert!((controller.ui.waveform.view.end - 0.8).abs() < 1.0e-6);
}

#[test]
fn apply_native_waveform_view_center_uses_precise_nanos_when_available() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    controller.ui.waveform.view = crate::app::state::WaveformView {
        start: 0.5,
        end: 0.500_000_2,
    };

    controller.apply_native_ui_action(NativeUiAction::SetWaveformViewCenter {
        center_micros: 500_000,
        center_nanos: Some(500_000_050),
    });

    assert!((controller.ui.waveform.view.start - 0.499_999_95).abs() < 1.0e-9);
    assert!((controller.ui.waveform.view.end - 0.500_000_15).abs() < 1.0e-9);
}

#[test]
/// Waveform toolbar option actions should update controller waveform state.
fn apply_native_waveform_option_actions_update_waveform_state() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);

    controller.apply_native_ui_action(NativeUiAction::SetWaveformChannelView { stereo: true });
    assert_eq!(
        controller.ui.waveform.channel_view,
        WaveformChannelView::SplitStereo
    );

    controller
        .apply_native_ui_action(NativeUiAction::SetNormalizedAuditionEnabled { enabled: true });
    assert!(controller.ui.waveform.normalized_audition_enabled);

    controller.ui.waveform.bpm_value = Some(120.0);
    controller.apply_native_ui_action(NativeUiAction::AdjustWaveformBpm { delta: 1 });
    assert_eq!(controller.ui.waveform.bpm_value, Some(121.0));
    controller.apply_native_ui_action(NativeUiAction::SetWaveformBpmValue { value_tenths: 1275 });
    assert_eq!(controller.ui.waveform.bpm_value, Some(127.5));

    controller.apply_native_ui_action(NativeUiAction::SetBpmSnapEnabled { enabled: true });
    assert!(controller.ui.waveform.bpm_snap_enabled);

    controller.apply_native_ui_action(NativeUiAction::SetRelativeBpmGridEnabled { enabled: true });
    assert!(controller.ui.waveform.relative_bpm_grid_enabled);

    controller.apply_native_ui_action(NativeUiAction::SetTransientSnapEnabled { enabled: true });
    assert!(controller.ui.waveform.transient_snap_enabled);

    controller
        .apply_native_ui_action(NativeUiAction::SetTransientMarkersEnabled { enabled: false });
    assert!(!controller.ui.waveform.transient_markers_enabled);
    assert!(!controller.ui.waveform.transient_snap_enabled);

    controller.ui.waveform.selected_slices = vec![0, 1];
    controller.apply_native_ui_action(NativeUiAction::SetSliceModeEnabled { enabled: true });
    assert!(controller.ui.waveform.slice_mode_enabled);

    controller.ui.waveform.slices = vec![
        crate::selection::SelectionRange::new(0.1, 0.2),
        crate::selection::SelectionRange::new(0.3, 0.4),
    ];
    controller.ui.waveform.selected_slices.clear();
    controller.apply_native_ui_action(NativeUiAction::ToggleWaveformSliceSelection { index: 1 });
    assert_eq!(controller.ui.waveform.selected_slices, vec![1]);
    controller.start_slice_review();
    controller.apply_native_ui_action(NativeUiAction::MoveWaveformSliceFocus { delta: 1 });
    assert_eq!(controller.ui.waveform.slice_review.focused_index, Some(1));
    controller.apply_native_ui_action(NativeUiAction::ToggleFocusedWaveformSliceExportMark);
    assert_eq!(controller.ui.waveform.slice_review.marked_indices, vec![1]);

    controller.apply_native_ui_action(NativeUiAction::SetSliceModeEnabled { enabled: false });
    assert!(!controller.ui.waveform.slice_mode_enabled);
    assert!(controller.ui.waveform.selected_slices.is_empty());
    assert_eq!(
        controller.ui.waveform.slice_review,
        crate::app::state::WaveformSliceReviewState::default()
    );
}

#[test]
fn handle_escape_exits_slice_review_before_clearing_slice_batch() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    controller.ui.waveform.slices = vec![
        crate::selection::SelectionRange::new(0.1, 0.2),
        crate::selection::SelectionRange::new(0.3, 0.4),
    ];
    controller.start_slice_review();

    controller.apply_native_ui_action(NativeUiAction::HandleEscape);

    assert!(!controller.ui.waveform.slice_review.active);
    assert_eq!(controller.ui.waveform.slices.len(), 2);
}

#[test]
fn duplicate_preview_actions_focus_audition_and_toggle_exemption() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let range = crate::selection::SelectionRange::new(0.1, 0.2);
    controller.ui.waveform.slice_batch_profile =
        crate::app::state::WaveformSliceBatchProfile::ExactDuplicateBeats;
    controller.ui.waveform.slices = vec![range];
    controller.ui.waveform.duplicate_cleanup =
        Some(crate::app::state::WaveformDuplicateCleanupState {
            group_count: 1,
            previews: vec![crate::app::state::WaveformDuplicateCleanupPreview {
                range,
                group_id: 0,
                exempted: false,
                represented_window_count: 1,
            }],
        });
    controller.ui.waveform.slice_batch_beat_count = 1;

    controller.apply_native_ui_action(NativeUiAction::AuditionWaveformDuplicateSlice { index: 0 });
    assert_eq!(controller.ui.waveform.slice_review.focused_index, Some(0));

    controller
        .apply_native_ui_action(NativeUiAction::ToggleWaveformDuplicateSliceExemption { index: 0 });
    assert_eq!(controller.ui.waveform.slice_batch_beat_count, 0);
    assert!(
        controller
            .ui
            .waveform
            .duplicate_cleanup
            .as_ref()
            .is_some_and(|state| state.previews[0].exempted)
    );
}

#[test]
/// Native options panel actions should update UI settings state.
fn apply_native_options_panel_actions_update_ui_state() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);

    controller.apply_native_ui_action(NativeUiAction::OpenOptionsMenu);
    assert!(controller.ui.options_panel.open);

    controller
        .apply_native_ui_action(NativeUiAction::SetAdvanceAfterRatingEnabled { enabled: false });
    assert!(!controller.ui.controls.advance_after_rating);

    controller.apply_native_ui_action(NativeUiAction::SetDestructiveYoloMode { enabled: true });
    assert!(controller.ui.controls.destructive_yolo_mode);

    controller.apply_native_ui_action(NativeUiAction::SetInvertWaveformScroll { enabled: false });
    assert!(!controller.ui.controls.invert_waveform_scroll);

    controller.apply_native_ui_action(NativeUiAction::CloseOptionsPanel);
    assert!(!controller.ui.options_panel.open);
}

#[test]
fn apply_native_waveform_circular_slide_rotates_sample_and_clears_drag_state() {
    with_waveform_fixture_controller(|controller, source, wav_path| {
        write_test_wav(&wav_path, &[1.0, 2.0, 3.0, 4.0]);
        controller
            .load_waveform_for_selection(&source, Path::new("kick_one.wav"))
            .unwrap();

        controller.apply_native_ui_action(NativeUiAction::BeginWaveformCircularSlide {
            anchor_micros: 500_000,
        });
        assert!(controller.is_waveform_circular_slide_active());

        controller.apply_native_ui_action(NativeUiAction::UpdateWaveformCircularSlide {
            position_micros: 0,
        });
        assert!(controller.is_waveform_circular_slide_active());

        controller.apply_native_ui_action(NativeUiAction::FinishWaveformCircularSlide);

        assert!(!controller.is_waveform_circular_slide_active());
        assert_eq!(read_test_wav_samples(&wav_path), vec![3.0, 4.0, 1.0, 2.0]);
    });
}

#[test]
fn apply_native_waveform_circular_slide_no_op_finish_leaves_file_unchanged() {
    with_waveform_fixture_controller(|controller, source, wav_path| {
        write_test_wav(&wav_path, &[1.0, 2.0, 3.0, 4.0]);
        controller
            .load_waveform_for_selection(&source, Path::new("kick_one.wav"))
            .unwrap();

        controller.apply_native_ui_action(NativeUiAction::BeginWaveformCircularSlide {
            anchor_micros: 500_000,
        });
        controller.apply_native_ui_action(NativeUiAction::FinishWaveformCircularSlide);

        assert!(!controller.is_waveform_circular_slide_active());
        assert_eq!(read_test_wav_samples(&wav_path), vec![1.0, 2.0, 3.0, 4.0]);
    });
}

fn write_test_wav(path: &Path, samples: &[f32]) {
    let spec = WavSpec {
        channels: 1,
        sample_rate: 8,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };
    let mut writer = WavWriter::create(path, spec).unwrap();
    for sample in samples {
        writer.write_sample(*sample).unwrap();
    }
    writer.finalize().unwrap();
}

fn read_test_wav_samples(path: &Path) -> Vec<f32> {
    WavReader::open(path)
        .unwrap()
        .samples::<f32>()
        .map(|sample| sample.unwrap())
        .collect()
}

fn with_waveform_fixture_controller(
    run: impl FnOnce(&mut AppController, crate::sample_sources::SampleSource, std::path::PathBuf),
) {
    let mut bundle = build_named_gui_fixture_controller(WaveformRenderer::new(16, 16), "waveform")
        .unwrap_or_else(|err| panic!("failed to build waveform fixture: {err}"));
    let source = bundle
        .controller
        .current_source()
        .expect("waveform fixture should select a source");
    let wav_path = source.root.join("kick_one.wav");
    run(&mut bundle.controller, source, wav_path);
}
