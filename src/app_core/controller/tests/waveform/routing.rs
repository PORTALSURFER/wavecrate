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
