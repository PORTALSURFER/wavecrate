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
