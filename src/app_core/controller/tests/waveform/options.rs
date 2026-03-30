use super::*;

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
