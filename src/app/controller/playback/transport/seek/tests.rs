use super::*;
use crate::app::controller::test_support;
use crate::app::controller::test_support::{
    load_waveform_selection, prepare_with_source_and_wav_entries, sample_entry,
};
use crate::waveform::DecodedWaveform;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

/// Seed minimal waveform state so seek tests exercise cursor updates on a ready waveform.
fn seed_waveform_ready_for_seek(controller: &mut AppController) {
    controller.sample_view.waveform.decoded = Some(Arc::new(DecodedWaveform {
        cache_token: 1,
        samples: Arc::from(vec![0.0; 16]),
        analysis_samples: Arc::from(Vec::new()),
        analysis_sample_rate: 0,
        analysis_stride: 1,
        peaks: None,
        duration_seconds: 1.0,
        sample_rate: 48_000,
        channels: 1,
    }));
}

#[test]
fn queue_waveform_seek_milli_clamps_input() {
    let (mut controller, _source) = test_support::dummy_controller();

    queue_waveform_seek_nanos(&mut controller, 1_500_000_000);

    assert_eq!(
        controller.runtime.pending_waveform_seek_nanos,
        Some(1_000_000_000)
    );
    assert!(
        controller
            .runtime
            .pending_waveform_seek_not_before
            .is_some()
    );
}

#[test]
fn flush_pending_waveform_seek_commit_waits_for_deadline() {
    let (mut controller, _source) = test_support::dummy_controller();
    queue_waveform_seek_nanos(&mut controller, 500_000_000);
    controller.runtime.pending_waveform_seek_not_before =
        Some(Instant::now() + Duration::from_millis(50));

    flush_pending_waveform_seek_commit(&mut controller);

    assert_eq!(
        controller.runtime.pending_waveform_seek_nanos,
        Some(500_000_000)
    );
}

#[test]
fn seek_waveform_nanos_starts_playback_immediately_and_focuses_waveform() {
    let Some(player) = crate::audio::AudioPlayer::playing_for_tests() else {
        return;
    };
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "direct_seek.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.audio.player = Some(Rc::new(RefCell::new(player)));
    load_waveform_selection(
        &mut controller,
        &source,
        "direct_seek.wav",
        &[0.0; 64],
        SelectionRange::new(0.25, 0.75),
    );
    controller
        .audio
        .player
        .as_ref()
        .expect("player")
        .borrow_mut()
        .stop();

    seek_waveform_nanos(&mut controller, 250_000_000);

    assert!(controller.is_playing());
    assert_eq!(controller.ui.waveform.cursor, Some(0.25));
    assert_eq!(controller.ui.waveform.playhead.position, 0.25);
    assert_eq!(
        controller.ui.focus.context,
        crate::app::state::FocusContext::Waveform
    );
}

#[test]
fn queue_waveform_seek_milli_clears_selection_when_target_is_outside_span() {
    let (mut controller, _source) = test_support::dummy_controller();
    seed_waveform_ready_for_seek(&mut controller);
    let selection = SelectionRange::new(0.2, 0.4);
    controller.selection_state.range.set_range(Some(selection));
    controller.apply_selection(Some(selection));

    queue_waveform_seek_nanos(&mut controller, 750_000_000);

    assert!(controller.selection_state.range.range().is_none());
    assert!(controller.ui.waveform.selection.is_none());
    assert_eq!(
        controller.runtime.pending_waveform_seek_nanos,
        Some(750_000_000)
    );
    assert_eq!(controller.ui.waveform.cursor, Some(0.75));
}

#[test]
fn queue_waveform_seek_nanos_cancels_click_armed_selection_drag() {
    let (mut controller, _source) = test_support::dummy_controller();
    seed_waveform_ready_for_seek(&mut controller);
    super::selection::start_selection_drag(&mut controller, 0.25);

    assert!(controller.selection_state.range.is_creating());
    assert!(controller.selection_state.pending_undo.is_some());

    queue_waveform_seek_nanos(&mut controller, 750_000_000);

    assert!(!controller.selection_state.range.is_dragging());
    assert!(controller.selection_state.pending_undo.is_none());
    assert_eq!(
        controller.runtime.pending_waveform_seek_nanos,
        Some(750_000_000)
    );
    assert_eq!(controller.ui.waveform.cursor, Some(0.75));
}

#[test]
fn queue_waveform_seek_nanos_clears_existing_selection_after_canceling_click_arm() {
    let (mut controller, _source) = test_support::dummy_controller();
    seed_waveform_ready_for_seek(&mut controller);
    let selection = SelectionRange::new(0.2, 0.4);
    controller.selection_state.range.set_range(Some(selection));
    controller.apply_selection(Some(selection));
    super::selection::start_selection_drag(&mut controller, 0.7);

    assert!(controller.selection_state.range.is_creating());

    queue_waveform_seek_nanos(&mut controller, 750_000_000);

    assert!(!controller.selection_state.range.is_dragging());
    assert!(controller.selection_state.range.range().is_none());
    assert!(controller.ui.waveform.selection.is_none());
    assert!(controller.selection_state.pending_undo.is_none());
}

#[test]
fn queue_waveform_seek_milli_preserves_selection_when_target_is_inside_span() {
    let (mut controller, _source) = test_support::dummy_controller();
    seed_waveform_ready_for_seek(&mut controller);
    let selection = SelectionRange::new(0.2, 0.4);
    controller.selection_state.range.set_range(Some(selection));
    controller.apply_selection(Some(selection));

    queue_waveform_seek_nanos(&mut controller, 300_000_000);

    assert_eq!(controller.selection_state.range.range(), Some(selection));
    assert_eq!(controller.ui.waveform.selection, Some(selection));
    assert_eq!(
        controller.runtime.pending_waveform_seek_nanos,
        Some(300_000_000)
    );
    assert_eq!(controller.ui.waveform.cursor, Some(0.3));
}

#[test]
fn queue_waveform_seek_milli_starts_immediately_when_stopped_with_loaded_audio() {
    let Some(player) = crate::audio::AudioPlayer::playing_for_tests() else {
        return;
    };
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "instant_seek.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.audio.player = Some(Rc::new(RefCell::new(player)));
    load_waveform_selection(
        &mut controller,
        &source,
        "instant_seek.wav",
        &[0.0; 64],
        SelectionRange::new(0.25, 0.75),
    );
    controller.selection_state.range.set_range(None);
    controller.ui.waveform.selection = None;
    controller
        .audio
        .player
        .as_ref()
        .expect("player")
        .borrow_mut()
        .stop();

    queue_waveform_seek_nanos(&mut controller, 500_000_000);

    assert!(controller.runtime.pending_waveform_seek_nanos.is_none());
    assert!(
        controller
            .runtime
            .pending_waveform_seek_not_before
            .is_none()
    );
    assert!(controller.is_playing());
    assert_eq!(controller.ui.waveform.cursor, Some(0.5));
    assert_eq!(controller.ui.waveform.playhead.position, 0.5);
    assert_eq!(controller.ui.waveform.last_start_marker, Some(0.5));
}

#[test]
fn queue_waveform_seek_milli_still_defers_commit_while_playing() {
    let Some(player) = crate::audio::AudioPlayer::playing_for_tests() else {
        return;
    };
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "deferred_seek.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.audio.player = Some(Rc::new(RefCell::new(player)));
    load_waveform_selection(
        &mut controller,
        &source,
        "deferred_seek.wav",
        &[0.0; 64],
        SelectionRange::new(0.25, 0.75),
    );
    assert!(controller.play_audio(false, None).is_ok());
    assert!(controller.is_playing());

    queue_waveform_seek_nanos(&mut controller, 750_000_000);

    assert_eq!(
        controller.runtime.pending_waveform_seek_nanos,
        Some(750_000_000)
    );
    assert!(
        controller
            .runtime
            .pending_waveform_seek_not_before
            .is_some()
    );
    assert_eq!(controller.ui.waveform.cursor, Some(0.75));
}

#[test]
fn immediate_waveform_seek_preserves_panned_view_when_starting_playback() {
    let Some(player) = crate::audio::AudioPlayer::playing_for_tests() else {
        return;
    };
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "seek_preserves_panned_view.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.audio.player = Some(Rc::new(RefCell::new(player)));
    load_waveform_selection(
        &mut controller,
        &source,
        "seek_preserves_panned_view.wav",
        &[0.0; 64],
        SelectionRange::new(0.25, 0.75),
    );
    controller.selection_state.range.set_range(None);
    controller.ui.waveform.selection = None;
    controller.ui.waveform.view = crate::app::state::WaveformView {
        start: 0.1,
        end: 0.2,
    };
    controller
        .audio
        .player
        .as_ref()
        .expect("player")
        .borrow_mut()
        .stop();

    queue_waveform_seek_nanos(&mut controller, 750_000_000);

    assert_eq!(
        controller.ui.focus.context,
        crate::app::state::FocusContext::Waveform
    );
    assert!((controller.ui.waveform.view.start - 0.1).abs() < 1.0e-9);
    assert!((controller.ui.waveform.view.end - 0.2).abs() < 1.0e-9);
    assert!((controller.ui.waveform.playhead.position - 0.75).abs() < 1.0e-6);
}

#[test]
fn deferred_waveform_seek_commit_preserves_panned_view_when_playing() {
    let Some(player) = crate::audio::AudioPlayer::playing_for_tests() else {
        return;
    };
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "deferred_seek_preserves_panned_view.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.audio.player = Some(Rc::new(RefCell::new(player)));
    load_waveform_selection(
        &mut controller,
        &source,
        "deferred_seek_preserves_panned_view.wav",
        &[0.0; 64],
        SelectionRange::new(0.25, 0.75),
    );
    controller.selection_state.range.set_range(None);
    controller.ui.waveform.selection = None;
    controller.ui.waveform.view = crate::app::state::WaveformView {
        start: 0.1,
        end: 0.2,
    };
    assert!(controller.play_audio(false, None).is_ok());

    queue_waveform_seek_nanos(&mut controller, 750_000_000);
    controller.runtime.pending_waveform_seek_not_before =
        Some(Instant::now() - Duration::from_millis(1));

    flush_pending_waveform_seek_commit(&mut controller);

    assert_eq!(
        controller.ui.focus.context,
        crate::app::state::FocusContext::Waveform
    );
    assert!((controller.ui.waveform.view.start - 0.1).abs() < 1.0e-9);
    assert!((controller.ui.waveform.view.end - 0.2).abs() < 1.0e-9);
    assert!((controller.ui.waveform.playhead.position - 0.75).abs() < 1.0e-6);
}
