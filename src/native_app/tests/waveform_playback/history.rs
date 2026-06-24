use super::*;

#[test]
fn playback_history_steps_between_regions_without_duplicate_recording() {
    let Some(mut scenario) = WaveformPlaybackScenario::default_loaded_with_player() else {
        return;
    };
    scenario
        .state
        .start_playback_current_span(0.0, 0.25)
        .expect("first region starts");
    scenario
        .state
        .start_playback_current_span(0.5, 0.75)
        .expect("second region starts");
    assert_eq!(scenario.state.audio.playback_history.len(), 2);

    let mut context = ui::UiUpdateContext::default();
    scenario.state.play_previous_playback_history(&mut context);

    assert_eq!(
        scenario.state.audio.current_playback_span,
        Some((0.0, 0.25))
    );
    assert_eq!(
        scenario.state.waveform.current.play_selection(),
        Some(wavecrate::selection::SelectionRange::new(0.0, 0.25))
    );
    assert_eq!(scenario.state.waveform.current.play_mark_ratio(), Some(0.0));
    assert_eq!(scenario.state.audio.playback_history.len(), 2);

    scenario.state.play_next_playback_history(&mut context);

    assert_eq!(
        scenario.state.audio.current_playback_span,
        Some((0.5, 0.75))
    );
    assert_eq!(
        scenario.state.waveform.current.play_selection(),
        Some(wavecrate::selection::SelectionRange::new(0.5, 0.75))
    );
    assert_eq!(scenario.state.waveform.current.play_mark_ratio(), Some(0.5));
    assert_eq!(scenario.state.audio.playback_history.len(), 2);
}

#[test]
fn playback_history_load_completion_restores_region_and_focuses_waveform() {
    let samples = vec![0_i16; 48_000];
    let mut scenario = WaveformPlaybackScenario::with_temp_wav("history-load.wav", &samples)
        .with_unloaded_waveform();
    let path = scenario
        .state
        .library
        .folder_browser
        .selected_file_id()
        .expect("selected temp sample")
        .to_string();

    scenario
        .state
        .audio
        .playback_history
        .record(path.clone(), 0.25, 0.50);
    scenario.state.audio.playback_history.record(
        String::from("/tmp/wavecrate-history-current.wav"),
        0.75,
        0.90,
    );

    let mut context = ui::UiUpdateContext::default();
    scenario.state.play_previous_playback_history(&mut context);
    run_command_for_tests(&mut scenario.state, context.into_command());

    let ticket = active_sample_load_ticket(&scenario.state).expect("history sample load queued");
    let mut context = ui::UiUpdateContext::default();
    scenario.state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SampleLoadFinished(
            sample_load_completion(
                ticket,
                path.clone(),
                crate::native_app::test_support::state::WaveformState::load_path(PathBuf::from(
                    &path,
                )),
                false,
            ),
        ),
        &mut context,
    );

    assert_eq!(scenario.state.audio.pending_sample_playback, None);
    assert_eq!(
        scenario.state.waveform.current.play_selection(),
        Some(wavecrate::selection::SelectionRange::new(0.25, 0.50))
    );
    assert_eq!(
        scenario.state.waveform.current.play_mark_ratio(),
        Some(0.25)
    );
    let viewport = scenario.state.waveform.current.viewport();
    assert_eq!(viewport.start, 12_000);
    assert_eq!(viewport.end, 24_000);
    assert!(
        scenario
            .state
            .waveform
            .current
            .visible_ratio_for_absolute(0.25)
            .is_some()
    );
    assert!(
        scenario
            .state
            .waveform
            .current
            .visible_ratio_for_absolute(0.50)
            .is_some()
    );
}

#[test]
fn playback_history_reports_empty_previous_history() {
    let mut scenario = WaveformPlaybackScenario::synthetic();
    let mut context = ui::UiUpdateContext::default();

    scenario.state.play_previous_playback_history(&mut context);

    assert_eq!(
        scenario.state.ui.status.sample,
        "No earlier playback history"
    );
}
