use super::*;

#[test]
fn play_selected_sample_enables_loop_for_loop_tagged_sample() {
    let Some(mut scenario) = WaveformPlaybackScenario::default_loaded_with_player() else {
        return;
    };
    let file_id = loaded_file_id(&scenario);
    scenario
        .state
        .metadata
        .tags_by_file
        .insert(file_id, vec![String::from("loop")]);
    scenario.state.audio.loop_playback = false;

    scenario.play_selected_sample();

    assert!(scenario.state.audio.loop_playback);
    assert!(
        scenario
            .state
            .audio
            .player
            .as_ref()
            .is_some_and(|player| player.is_looping())
    );
}

#[test]
fn play_selected_sample_disables_loop_for_one_shot_tagged_sample() {
    let Some(mut scenario) = WaveformPlaybackScenario::default_loaded_with_player() else {
        return;
    };
    let file_id = loaded_file_id(&scenario);
    scenario
        .state
        .metadata
        .tags_by_file
        .insert(file_id, vec![String::from("one-shot")]);
    scenario.state.audio.loop_playback = true;

    scenario.play_selected_sample();

    assert!(!scenario.state.audio.loop_playback);
    assert!(
        scenario
            .state
            .audio
            .player
            .as_ref()
            .is_some_and(|player| !player.is_looping())
    );
}

#[test]
fn manual_loop_toggle_overrides_tag_until_sample_changes() {
    let Some(mut scenario) = WaveformPlaybackScenario::default_loaded_with_player() else {
        return;
    };
    let file_id = loaded_file_id(&scenario);
    scenario
        .state
        .metadata
        .tags_by_file
        .insert(file_id, vec![String::from("loop")]);

    scenario.play_selected_sample();
    scenario.state.toggle_loop_playback();
    scenario.play_selected_sample();

    assert!(!scenario.state.audio.loop_playback);
    assert!(
        scenario
            .state
            .audio
            .player
            .as_ref()
            .is_some_and(|player| !player.is_looping())
    );
}

#[test]
fn metadata_tag_change_reconciles_current_loaded_playback_mode() {
    let Some(mut scenario) = WaveformPlaybackScenario::default_loaded_with_player() else {
        return;
    };
    let file_id = loaded_file_id(&scenario);
    scenario.state.audio.loop_playback = true;
    scenario
        .state
        .start_playback_current_span(0.0, 1.0)
        .expect("looped sample playback starts");
    assert!(
        scenario
            .state
            .audio
            .player
            .as_ref()
            .is_some_and(|player| player.is_looping())
    );

    scenario
        .state
        .metadata
        .tags_by_file
        .insert(file_id.clone(), vec![String::from("one-shot")]);
    scenario
        .state
        .reconcile_playback_mode_after_metadata_tag_change(file_id.as_str());

    assert!(!scenario.state.audio.loop_playback);
    assert!(
        scenario
            .state
            .audio
            .player
            .as_ref()
            .is_some_and(|player| !player.is_looping())
    );
}

fn loaded_file_id(scenario: &WaveformPlaybackScenario) -> String {
    scenario.state.waveform.current.path().display().to_string()
}
