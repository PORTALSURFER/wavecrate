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
fn playback_history_load_completion_restores_region_without_unneeded_zoom() {
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
    assert_eq!(viewport.start, 0);
    assert_eq!(viewport.end, samples.len() as i64);
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
fn playback_history_completion_refocuses_hidden_curation_entry() {
    let source_root = tempfile::tempdir().expect("source root");
    let kick = source_root.path().join("kick.wav");
    let loop_file = source_root.path().join("loop.wav");
    write_test_wav_i16(&kick, &[0, 1024, -1024, 512]);
    write_test_wav_i16(&loop_file, &[0, 512, -512, 256]);
    let kick_id = kick.display().to_string();
    let loop_id = loop_file.display().to_string();

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state.library.folder_browser.apply_message(
        crate::native_app::test_support::state::FolderBrowserMessage::SetCurationScope(
            crate::native_app::sample_library::folder_browser::model::BrowserCurationScope::All,
            true,
        ),
    );
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    assert!(
        state
            .library
            .folder_browser
            .set_file_last_curated_at(&kick, now)
    );
    state.library.folder_browser.select_file(loop_id.clone());
    state.waveform.current =
        crate::native_app::test_support::state::WaveformState::load_path(loop_file.clone())
            .expect("loop waveform loads");
    state
        .audio
        .playback_history
        .record(kick_id.clone(), 0.25, 0.50);
    state
        .audio
        .playback_history
        .record(loop_id.clone(), 0.0, 1.0);

    let mut context = ui::UiUpdateContext::default();
    state.play_previous_playback_history(&mut context);
    run_command_for_tests(&mut state, context.into_command());
    assert!(
        state
            .library
            .folder_browser
            .selected_audio_file_index_matching_tags(&state.metadata.tags_by_file)
            .is_some(),
        "history navigation should reveal its selected curation target while it loads"
    );
    assert!(
        state
            .library
            .folder_browser
            .selected_audio_files_matching_tags(&state.metadata.tags_by_file)
            .iter()
            .any(|file| file.id == kick_id),
        "the history target should be visible even when curation would normally hide it"
    );

    state.library.folder_browser.select_file(loop_id.clone());
    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(loop_id.as_str())
    );
    let ticket = active_sample_load_ticket(&state).expect("history sample load queued");
    let mut context = ui::UiUpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SampleLoadFinished(
            sample_load_completion(
                ticket,
                kick_id.clone(),
                crate::native_app::test_support::state::WaveformState::load_path(kick.clone()),
                false,
            ),
        ),
        &mut context,
    );

    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(kick_id.as_str())
    );
    assert_eq!(
        state.library.folder_browser.selected_file_paths(),
        vec![kick]
    );
    assert!(
        state
            .library
            .folder_browser
            .selected_audio_file_index_matching_tags(&state.metadata.tags_by_file)
            .is_some(),
        "history load completion should keep the selected curation target visible"
    );
    assert_eq!(
        state.waveform.current.play_selection(),
        Some(wavecrate::selection::SelectionRange::new(0.25, 0.50))
    );
}

#[test]
fn playback_history_replay_focuses_loaded_entry_in_browser_location() {
    let source_root = tempfile::tempdir().expect("source root");
    let kicks = source_root.path().join("drums").join("kicks");
    let loops = source_root.path().join("loops");
    fs::create_dir_all(&kicks).expect("create kicks folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let kick = kicks.join("kick.wav");
    let loop_file = loops.join("loop.wav");
    write_test_wav_i16(&kick, &[0, 1024, -1024, 512]);
    write_test_wav_i16(&loop_file, &[0, 512, -512, 256]);

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    assert!(
        state
            .library
            .folder_browser
            .focus_file_across_sources(&loop_file)
    );
    let kicks_id = kicks.display().to_string();
    let kick_id = kick.display().to_string();
    let loop_id = loop_file.display().to_string();
    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(loop_id.as_str())
    );
    state.waveform.current =
        crate::native_app::test_support::state::WaveformState::load_path(kick.clone())
            .expect("kick waveform loads");

    state
        .audio
        .playback_history
        .record(kick_id.clone(), 0.25, 0.50);
    state
        .audio
        .playback_history
        .record(loop_id.clone(), 0.0, 1.0);

    let mut context = ui::UiUpdateContext::default();
    state.play_previous_playback_history(&mut context);

    assert_eq!(
        state.library.folder_browser.selected_folder_id(),
        Some(kicks_id.as_str())
    );
    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(kick_id.as_str())
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
