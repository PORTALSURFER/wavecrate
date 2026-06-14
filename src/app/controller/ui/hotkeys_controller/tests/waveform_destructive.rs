use super::*;

#[test]
fn waveform_hotkey_respects_focus() {
    let (mut controller, source) =
        prepare_with_source_and_wav_entries(vec![sample_entry("one.wav", Rating::NEUTRAL)]);
    load_waveform_selection(
        &mut controller,
        &source,
        "one.wav",
        &[0.1, -0.2, 0.3, -0.4],
        SelectionRange::new(0.0, 0.5),
    );
    controller.set_destructive_yolo_mode(false);
    let action = action_for(|action| {
        matches!(
            action,
            crate::app_core::actions::NativeUiAction::PromptsAndEdits(
                crate::app_core::actions::NativePromptEditAction::CropWaveformSelection
            )
        )
    });

    controller.handle_hotkey(action.clone(), FocusContext::Waveform);
    assert!(controller.ui.waveform.pending_destructive.is_some());

    controller.ui.waveform.pending_destructive = None;
    controller.handle_hotkey(action, FocusContext::SampleBrowser);
    assert!(controller.ui.waveform.pending_destructive.is_none());
}
