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
    assert_eq!(scenario.state.audio.playback_history.len(), 2);

    scenario.state.play_next_playback_history(&mut context);

    assert_eq!(
        scenario.state.audio.current_playback_span,
        Some((0.5, 0.75))
    );
    assert_eq!(scenario.state.audio.playback_history.len(), 2);
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
