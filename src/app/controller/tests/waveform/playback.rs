use super::super::super::test_support::{
    load_waveform_selection, prepare_with_source_and_wav_entries, sample_entry,
};
use crate::selection::SelectionRange;
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn normalize_selection_resumes_playback_when_playing() {
    let Some(player) = crate::audio::AudioPlayer::playing_for_tests() else {
        return;
    };
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "normalize_resume.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.audio.player = Some(Rc::new(RefCell::new(player)));
    load_waveform_selection(
        &mut controller,
        &source,
        "normalize_resume.wav",
        &vec![1.0; 44100],
        SelectionRange::new(0.25, 0.75),
    );
    if controller.play_audio(false, None).is_err() || !controller.is_playing() {
        return;
    }
    controller.ui.waveform.playhead.position = 0.5;

    assert!(controller.normalize_waveform_selection().is_ok());

    assert!(controller.is_playing());
    assert!((controller.ui.waveform.playhead.position - 0.5).abs() < 1e-6);
}

#[test]
fn normalized_audition_applies_to_plain_playback_without_cached_waveform_decode() {
    let Some(player) = crate::audio::AudioPlayer::playing_for_tests() else {
        return;
    };
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "normalized_plain_playback.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.audio.player = Some(Rc::new(RefCell::new(player)));
    load_waveform_selection(
        &mut controller,
        &source,
        "normalized_plain_playback.wav",
        &[0.25, -0.5, 0.1, -0.2],
        SelectionRange::new(0.25, 0.75),
    );
    controller.selection_state.range.set_range(None);
    controller.ui.waveform.selection = None;
    controller.sample_view.waveform.decoded = None;
    controller.ui.waveform.normalized_audition_enabled = true;

    assert!(controller.play_audio(false, None).is_ok());

    let gain = controller
        .audio
        .player
        .as_ref()
        .expect("player")
        .borrow()
        .playback_gain_for_tests();
    assert!(
        (gain - 2.0).abs() < 1.0e-6,
        "expected full-track peak normalization gain, got {gain}"
    );
}
