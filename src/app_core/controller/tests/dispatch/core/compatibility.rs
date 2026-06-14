use super::*;

/// UI seek actions should queue deferred playback commit work.
#[test]
fn apply_ui_seek_queues_deferred_seek_commit() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);

    controller.apply_ui_action(NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SeekWaveformPrecise {
            position_nanos: 420_000_000,
        },
    ));

    assert_eq!(
        controller.pending_waveform_seek_nanos_for_test(),
        Some(420_000_000)
    );
}

/// Precise UI seek actions should preserve nanounit targets.
#[test]
fn apply_ui_precise_seek_queues_exact_deferred_seek_commit() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);

    controller.apply_ui_action(NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SeekWaveformPrecise {
            position_nanos: 420_123_456,
        },
    ));

    assert_eq!(
        controller.pending_waveform_seek_nanos_for_test(),
        Some(420_123_456)
    );
}
