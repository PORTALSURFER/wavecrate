use super::*;

#[test]
fn random_audition_span_uses_fixed_window_inside_long_sample() {
    let (start, end) = crate::native_app::audio::playback::random_audition_span_for_unit(20.0, 0.5);

    assert!((start - 0.4).abs() < 0.001, "start was {start}");
    assert!((end - 0.6).abs() < 0.001, "end was {end}");
}

#[test]
fn random_audition_span_plays_whole_short_sample() {
    assert_eq!(
        crate::native_app::audio::playback::random_audition_span_for_unit(2.0, 0.75),
        (0.0, 1.0)
    );
}

#[test]
fn random_audition_prefers_marked_play_ranges_and_selects_the_chosen_range() {
    let mut scenario = WaveformPlaybackScenario::synthetic();

    for (start, end) in [(0.10, 0.20), (0.55, 0.70)] {
        scenario.select_play_range(start, end);
    }

    let span = scenario
        .state
        .random_audition_span_for_loaded_waveform(0.75);

    assert_eq!(
        span.source,
        crate::native_app::audio::playback::RandomAuditionSource::MarkedRange
    );
    assert!(
        (span.start - 0.55).abs() < 0.001,
        "start was {}",
        span.start
    );
    assert!((span.end - 0.70).abs() < 0.001, "end was {}", span.end);
    assert_eq!(
        scenario.state.waveform.current.play_selection(),
        Some(wavecrate::selection::SelectionRange::new(0.55, 0.70))
    );
}

#[test]
fn random_audition_is_one_shot_even_when_loop_is_enabled() {
    let Ok(player) = wavecrate::audio::AudioPlayer::new() else {
        return;
    };
    let mut state = gui_state_for_span_tests();
    state.audio.player = Some(player);
    state.audio.loop_playback = true;

    let mut context = ui::UiUpdateContext::default();
    state.play_random_sample_range_with_unit(0.5, &mut context);

    assert!(!state.audio.loop_playback);
    assert!(state.waveform.current.is_playing());
    assert_eq!(state.audio.current_playback_span, Some((0.0, 1.0)));
    assert!(
        state
            .audio
            .player
            .as_ref()
            .is_some_and(|player| !player.is_looping())
    );
}

#[test]
fn random_audition_for_unloaded_selection_resumes_after_sample_load() {
    let mut scenario = WaveformPlaybackScenario::with_temp_wav(
        "random-load.wav",
        &[0, 1024, -2048, 4096, -1024, 512],
    )
    .with_unloaded_waveform()
    .with_looping();
    assert!(!scenario.state.waveform.current.has_loaded_sample());

    scenario.play_random_range(0.5);

    assert!(matches!(
        scenario.state.audio.pending_sample_playback,
        Some(crate::native_app::test_support::state::PendingSamplePlayback::RandomAudition { unit })
            if (unit - 0.5).abs() < f32::EPSILON
    ));

    scenario.start_deferred_load(false);
    scenario.finish_deferred_load(false);

    assert_eq!(scenario.state.audio.pending_sample_playback, None);
    assert!(
        scenario.state.audio.pending_playback_start.is_some(),
        "random audition should request playback even when the audio device is still opening"
    );
    assert!(
        !scenario.state.audio.loop_playback,
        "random audition should remain one-shot after the selected sample loads"
    );
    assert!(
        scenario
            .state
            .ui
            .status
            .sample
            .contains("Playback unavailable")
            || scenario.state.ui.status.sample.contains("Random audition"),
        "random load completion should route through random playback, got {}",
        scenario.state.ui.status.sample
    );
}
