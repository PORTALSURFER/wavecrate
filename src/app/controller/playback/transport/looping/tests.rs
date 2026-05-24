use super::*;
use crate::app::controller::test_support;
use std::path::Path;
use std::time::{Duration, Instant};

#[test]
fn toggle_loop_enable_clears_pending_disable_deadline() {
    let (mut controller, _source) = test_support::dummy_controller();
    controller.audio.pending_loop_disable_at = Some(Instant::now() + Duration::from_secs(1));
    controller.ui.waveform.loop_enabled = false;

    toggle_loop(&mut controller);

    assert!(controller.ui.waveform.loop_enabled);
    assert!(controller.audio.pending_loop_disable_at.is_none());
}

#[test]
fn loop_playback_policy_maps_toggle_transitions() {
    assert_eq!(
        loop_playback_policy(LoopToggleState {
            was_looping: false,
            loop_enabled: true,
        }),
        LoopPlaybackPolicy::RestartIfPlaying
    );
    assert_eq!(
        loop_playback_policy(LoopToggleState {
            was_looping: true,
            loop_enabled: false,
        }),
        LoopPlaybackPolicy::DeferDisableAfterCycle
    );
    assert_eq!(
        loop_playback_policy(LoopToggleState {
            was_looping: false,
            loop_enabled: false,
        }),
        LoopPlaybackPolicy::None
    );
}

#[test]
fn toggle_loop_unlocks_before_persisting_normal_toggle() {
    let (mut controller, _source) = test_support::dummy_controller();
    controller.ui.waveform.loop_enabled = true;
    controller.set_loop_lock_enabled(true);

    toggle_loop(&mut controller);

    assert!(!controller.ui.waveform.loop_lock_enabled);
    assert!(!controller.ui.waveform.loop_enabled);
}

#[test]
fn toggle_loop_lock_enters_locked_on_from_unlocked() {
    let (mut controller, _source) = test_support::dummy_controller();
    controller.ui.waveform.loop_enabled = false;
    controller.set_loop_lock_enabled(false);

    toggle_loop_lock(&mut controller);

    assert!(controller.ui.waveform.loop_lock_enabled);
    assert!(controller.ui.waveform.loop_enabled);
}

#[test]
fn toggle_loop_lock_cycles_locked_on_and_off() {
    let (mut controller, _source) = test_support::dummy_controller();
    controller.ui.waveform.loop_enabled = false;

    toggle_loop_lock(&mut controller);
    assert!(controller.ui.waveform.loop_lock_enabled);
    assert!(controller.ui.waveform.loop_enabled);

    toggle_loop_lock(&mut controller);
    assert!(controller.ui.waveform.loop_lock_enabled);
    assert!(!controller.ui.waveform.loop_enabled);

    toggle_loop_lock(&mut controller);
    assert!(controller.ui.waveform.loop_lock_enabled);
    assert!(controller.ui.waveform.loop_enabled);
}

#[test]
fn toggle_loop_lock_does_not_persist_sample_loop_marker() {
    let (mut controller, source) =
        test_support::prepare_with_source_and_wav_entries(vec![test_support::sample_entry(
            "locked_override.wav",
            crate::sample_sources::Rating::NEUTRAL,
        )]);
    let db = controller.cache_db(&source).expect("db");
    db.set_looped(Path::new("locked_override.wav"), false)
        .expect("seed loop marker");
    controller.sample_view.wav.loaded_audio =
        Some(crate::app::controller::state::audio::LoadedAudio {
            source_id: source.id.clone(),
            root: source.root.clone(),
            relative_path: Path::new("locked_override.wav").to_path_buf(),
            bytes: Vec::new().into(),
            duration_seconds: 1.0,
            sample_rate: 48_000,
        });

    toggle_loop_lock(&mut controller);

    assert_eq!(
        db.looped_for_path(Path::new("locked_override.wav"))
            .expect("load loop marker"),
        Some(false)
    );
}

#[test]
fn loop_restart_start_override_uses_selection_span_when_available() {
    let (mut controller, _source) = test_support::dummy_controller();
    controller.ui.waveform.selection = Some(SelectionRange::new(0.2, 0.6));

    let start_override = loop_restart_start_override(&controller, Some(0.4));

    assert_eq!(start_override, None);
}

#[test]
fn loop_restart_start_override_priority_chain_is_stable() {
    let (mut controller, _source) = test_support::dummy_controller();

    let with_progress = loop_restart_start_override(&controller, Some(0.42));
    assert_eq!(with_progress, Some(f64::from(0.42_f32)));

    controller.ui.waveform.playhead.visible = true;
    controller.ui.waveform.playhead.position = 0.37;
    let with_playhead = loop_restart_start_override(&controller, None);
    assert_eq!(
        with_playhead,
        Some(f64::from(controller.ui.waveform.playhead.position))
    );

    controller.ui.waveform.playhead.visible = false;
    controller.ui.waveform.cursor = Some(0.55);
    controller.ui.waveform.last_start_marker = Some(0.19);
    let with_cursor = loop_restart_start_override(&controller, None);
    assert_eq!(with_cursor, controller.ui.waveform.cursor.map(f64::from));

    controller.ui.waveform.cursor = None;
    let with_last_marker = loop_restart_start_override(&controller, None);
    assert_eq!(
        with_last_marker,
        controller.ui.waveform.last_start_marker.map(f64::from)
    );
}
