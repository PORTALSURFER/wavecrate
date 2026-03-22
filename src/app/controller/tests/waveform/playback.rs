use super::super::super::test_support::{
    load_waveform_selection, prepare_with_source_and_wav_entries, sample_entry,
};
use crate::app::state::WaveformView;
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

#[test]
fn playback_start_refreshes_stale_zoomed_waveform_image_before_showing_selection() {
    let Some(player) = crate::audio::AudioPlayer::playing_for_tests() else {
        return;
    };
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "playback_refreshes_zoomed_waveform.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.audio.player = Some(Rc::new(RefCell::new(player)));
    controller.sample_view.waveform.size = [320, 32];
    let selection = SelectionRange::new(0.5004, 0.5006);
    load_waveform_selection(
        &mut controller,
        &source,
        "playback_refreshes_zoomed_waveform.wav",
        &vec![0.5; 48_000],
        selection,
    );
    controller.selection_state.range.set_range(Some(selection));
    controller.ui.waveform.view = WaveformView {
        start: 0.500_0,
        end: 0.501_0,
    };
    controller.refresh_waveform_image();
    let before = *controller
        .sample_view
        .waveform
        .render_meta
        .as_ref()
        .expect("initial waveform render");
    assert!((before.view_start - 0.500_0).abs() < 1.0e-9);
    assert!((before.view_end - 0.501_0).abs() < 1.0e-9);

    controller.ui.waveform.view = WaveformView {
        start: 0.500_2,
        end: 0.501_2,
    };

    assert!(controller.play_audio(false, None).is_ok());

    let after = *controller
        .sample_view
        .waveform
        .render_meta
        .as_ref()
        .expect("refreshed waveform render");
    assert!((after.view_start - 0.500_2).abs() < 1.0e-9);
    assert!((after.view_end - 0.501_2).abs() < 1.0e-9);
}
