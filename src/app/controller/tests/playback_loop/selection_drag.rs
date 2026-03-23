use super::*;
use crate::selection::SelectionEdge;
use std::time::{Duration, Instant};

#[test]
fn finish_selection_drag_keeps_playing_when_playhead_inside_loop() {
    let initial_selection = SelectionRange::new(0.1, 0.4);
    let Some(mut controller) = setup_looping_controller(initial_selection) else {
        return;
    };
    let updated_selection = SelectionRange::new(0.2, 0.6);
    controller
        .selection_state
        .range
        .set_range(Some(updated_selection));
    controller.apply_selection(Some(updated_selection));
    controller.ui.waveform.playhead.position = 0.3;

    controller.finish_selection_drag();

    assert!((controller.ui.waveform.playhead.position - 0.3).abs() < 1e-6);
    assert!(controller.audio.pending_loop_retarget.is_some());
}

#[test]
fn finish_selection_drag_restarts_when_playhead_outside_loop() {
    let initial_selection = SelectionRange::new(0.1, 0.4);
    let Some(mut controller) = setup_looping_controller(initial_selection) else {
        return;
    };
    let updated_selection = SelectionRange::new(0.6, 0.8);
    controller
        .selection_state
        .range
        .set_range(Some(updated_selection));
    controller.apply_selection(Some(updated_selection));
    controller.ui.waveform.playhead.position = 0.2;

    controller.finish_selection_drag();

    assert!((controller.ui.waveform.playhead.position - updated_selection.start()).abs() < 1e-6);
    assert!(controller.audio.pending_loop_retarget.is_none());
}

#[test]
fn selection_resize_while_playing_does_not_restart_each_drag_update() {
    let Some(mut controller) = setup_looping_controller(SelectionRange::new(0.1, 0.4)) else {
        return;
    };
    controller.ui.waveform.loop_enabled = false;
    assert!(controller.play_audio(false, Some(0.25)).is_ok());
    controller.ui.waveform.playhead.position = 0.25;

    assert!(controller.start_selection_edge_drag(SelectionEdge::End, false));
    controller.update_selection_drag(0.6, false);

    assert!((controller.ui.waveform.playhead.position - 0.25).abs() < 1e-6);
}

#[test]
fn finish_selection_drag_restarts_once_from_current_playhead_when_non_looped() {
    let Some(mut controller) = setup_looping_controller(SelectionRange::new(0.1, 0.4)) else {
        return;
    };
    controller.ui.waveform.loop_enabled = false;
    assert!(controller.play_audio(false, Some(0.25)).is_ok());
    controller.ui.waveform.playhead.position = 0.25;
    let updated_selection = SelectionRange::new(0.2, 0.6);
    controller
        .selection_state
        .range
        .set_range(Some(updated_selection));
    controller.apply_selection(Some(updated_selection));

    controller.finish_selection_drag();

    assert!((controller.ui.waveform.playhead.position - 0.25).abs() < 1e-6);
    assert_eq!(
        controller.ui.waveform.playhead.active_span_end,
        Some(updated_selection.end())
    );
    assert!(controller.audio.pending_loop_retarget.is_none());
}

#[test]
fn pending_loop_retarget_restarts_from_new_selection_start_at_cycle_boundary() {
    let initial_selection = SelectionRange::new(0.1, 0.4);
    let Some(mut controller) = setup_looping_controller(initial_selection) else {
        return;
    };
    let updated_selection = SelectionRange::new(0.2, 0.6);
    controller
        .selection_state
        .range
        .set_range(Some(updated_selection));
    controller.apply_selection(Some(updated_selection));
    controller.ui.waveform.playhead.position = 0.3;

    controller.finish_selection_drag();
    controller
        .audio
        .pending_loop_retarget
        .as_mut()
        .expect("loop retarget scheduled")
        .deadline = Instant::now() - Duration::from_millis(1);

    controller.tick_playhead();

    assert!((controller.ui.waveform.playhead.position - updated_selection.start()).abs() < 1e-6);
    assert!(controller.audio.pending_loop_retarget.is_none());
}

#[test]
fn set_selection_range_while_looping_schedules_retarget_when_playhead_inside_loop() {
    let initial_selection = SelectionRange::new(0.1, 0.4);
    let Some(mut controller) = setup_looping_controller(initial_selection) else {
        return;
    };
    let updated_selection = SelectionRange::new(0.2, 0.6);
    controller.ui.waveform.playhead.position = 0.3;

    controller.set_selection_range(updated_selection);

    assert!((controller.ui.waveform.playhead.position - 0.3).abs() < 1e-6);
    let pending = controller
        .audio
        .pending_loop_retarget
        .expect("loop retarget scheduled");
    assert!((pending.start_override - f64::from(updated_selection.start())).abs() < 1e-6);
}

#[test]
fn set_selection_range_restarts_from_new_selection_start_at_cycle_boundary() {
    let initial_selection = SelectionRange::new(0.1, 0.4);
    let Some(mut controller) = setup_looping_controller(initial_selection) else {
        return;
    };
    let updated_selection = SelectionRange::new(0.2, 0.6);
    controller.ui.waveform.playhead.position = 0.3;

    controller.set_selection_range(updated_selection);
    controller
        .audio
        .pending_loop_retarget
        .as_mut()
        .expect("loop retarget scheduled")
        .deadline = Instant::now() - Duration::from_millis(1);

    controller.tick_playhead();

    assert!((controller.ui.waveform.playhead.position - updated_selection.start()).abs() < 1e-6);
    assert!(controller.audio.pending_loop_retarget.is_none());
}

#[test]
fn set_selection_range_restarts_immediately_when_playhead_outside_updated_loop() {
    let initial_selection = SelectionRange::new(0.1, 0.8);
    let Some(mut controller) = setup_looping_controller(initial_selection) else {
        return;
    };
    let updated_selection = SelectionRange::new(0.2, 0.6);
    controller.ui.waveform.playhead.position = 0.7;

    controller.set_selection_range(updated_selection);

    assert!((controller.ui.waveform.playhead.position - updated_selection.start()).abs() < 1e-6);
    assert!(controller.audio.pending_loop_retarget.is_none());
}

#[test]
fn mutating_selection_clears_pending_loop_retarget() {
    let initial_selection = SelectionRange::new(0.1, 0.4);
    let Some(mut controller) = setup_looping_controller(initial_selection) else {
        return;
    };
    let updated_selection = SelectionRange::new(0.2, 0.6);
    controller
        .selection_state
        .range
        .set_range(Some(updated_selection));
    controller.apply_selection(Some(updated_selection));
    controller.ui.waveform.playhead.position = 0.3;

    controller.finish_selection_drag();
    assert!(controller.audio.pending_loop_retarget.is_some());

    controller.set_selection_range(SelectionRange::new(0.25, 0.55));
    assert!(controller.audio.pending_loop_retarget.is_none());

    controller.finish_selection_drag();
    assert!(controller.audio.pending_loop_retarget.is_none());

    controller.clear_selection();
    assert!(controller.audio.pending_loop_retarget.is_none());
}

#[test]
/// Defensive zero-width playback ranges should not clamp seek playback to a tiny span.
fn zero_width_selection_does_not_truncate_seek_playback() {
    let Some(mut player) = crate::audio::AudioPlayer::playing_for_tests() else {
        return;
    };
    let dir = tempdir().expect("tempdir");
    let wav_path = dir.path().join("click_seek_span.wav");
    let long_samples = vec![0.1_f32; 240];
    write_test_wav(&wav_path, &long_samples);
    let bytes = std::fs::read(&wav_path).expect("wav bytes");
    let duration = 30.0;
    player.set_audio(bytes, duration);

    let (mut controller, source) = dummy_controller();
    controller.sample_view.wav.loaded_audio = Some(LoadedAudio {
        source_id: source.id.clone(),
        root: source.root.clone(),
        relative_path: PathBuf::from("click_seek_span.wav"),
        bytes: std::fs::read(&wav_path).expect("wav bytes").into(),
        duration_seconds: duration,
        sample_rate: 8,
    });
    controller.audio.player = Some(std::rc::Rc::new(std::cell::RefCell::new(player)));

    let click_marker = SelectionRange::new(0.5, 0.5);
    controller
        .selection_state
        .range
        .set_range(Some(click_marker));
    controller.apply_selection(Some(click_marker));

    assert!(controller.play_audio(false, Some(0.5)).is_ok());
    let (start, end) = controller
        .audio
        .player
        .as_ref()
        .expect("player")
        .borrow()
        .play_span()
        .expect("play span set");
    assert!(
        end - start > 1.0,
        "unexpected tiny span: start={start} end={end}"
    );
    assert_eq!(controller.ui.waveform.playhead.active_span_end, Some(1.0));
}
