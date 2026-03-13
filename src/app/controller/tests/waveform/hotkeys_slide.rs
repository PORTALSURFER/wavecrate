use super::super::super::test_support::{
    load_waveform_selection, prepare_with_source_and_wav_entries, sample_entry,
};
use crate::app::controller::ui::hotkeys;
use crate::app::state::FocusContext;
use crate::selection::SelectionRange;

#[test]
fn arrow_hotkey_slides_active_playback_selection_by_its_full_width() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "slide_hotkey.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    load_waveform_selection(
        &mut controller,
        &source,
        "slide_hotkey.wav",
        &[0.0, 0.1, 0.2, 0.3],
        SelectionRange::new(0.2, 0.35),
    );

    let action = hotkeys::iter_actions()
        .find(|action| action.id == "slide-selection-right")
        .expect("missing slide-selection-right hotkey");
    controller.handle_hotkey(action, FocusContext::Waveform);

    let range = controller
        .ui
        .waveform
        .selection
        .expect("selection should remain active");
    assert!((range.start() - 0.35).abs() < 1.0e-6);
    assert!((range.end() - 0.5).abs() < 1.0e-6);
}

#[test]
fn arrow_hotkey_slides_ui_playback_selection_and_snaps_translation_to_bpm_grid() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "slide_snap_hotkey.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    load_waveform_selection(
        &mut controller,
        &source,
        "slide_snap_hotkey.wav",
        &[0.0, 0.1, 0.2, 0.3],
        SelectionRange::new(0.2, 0.4),
    );
    controller.selection_state.range.set_range(None);
    controller.ui.waveform.selection = Some(SelectionRange::new(0.2, 0.4));
    controller.ui.waveform.bpm_snap_enabled = true;
    controller.ui.waveform.bpm_value = Some(120.0);
    controller
        .sample_view
        .wav
        .loaded_audio
        .as_mut()
        .expect("loaded audio should be present")
        .duration_seconds = 4.0;

    let action = hotkeys::iter_actions()
        .find(|action| action.id == "slide-selection-right")
        .expect("missing slide-selection-right hotkey");
    controller.handle_hotkey(action, FocusContext::Waveform);

    let range = controller
        .ui
        .waveform
        .selection
        .expect("selection should remain active");
    assert!((range.start() - 0.375).abs() < 1.0e-6);
    assert!((range.end() - 0.575).abs() < 1.0e-6);
}
