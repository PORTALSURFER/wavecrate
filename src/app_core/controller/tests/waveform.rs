use super::*;

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
    });

    assert!((controller.ui.waveform.view.start - 0.6).abs() < 1.0e-6);
    assert!((controller.ui.waveform.view.end - 0.8).abs() < 1.0e-6);
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

    controller.apply_native_ui_action(NativeUiAction::SetSliceModeEnabled { enabled: false });
    assert!(!controller.ui.waveform.slice_mode_enabled);
    assert!(controller.ui.waveform.selected_slices.is_empty());
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
