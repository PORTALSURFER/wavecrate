use super::*;

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
