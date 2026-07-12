use super::*;

#[test]
fn stale_playback_ready_message_is_ignored_after_selection_changes() {
    let source_root = tempfile::tempdir().expect("source root");
    let first_path = source_root.path().join("first.wav");
    let second_path = source_root.path().join("second.wav");
    write_test_wav_i16(&first_path, &[0, 1024, -2048, 4096]);
    write_test_wav_i16(&second_path, &[0, 512, -512, 1024]);
    let first_path_string = first_path.display().to_string();
    let second_path_string = second_path.display().to_string();

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state
        .library
        .folder_browser
        .select_file(first_path_string.clone());
    let mut context = ui::UiUpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SelectSampleWithModifiers {
            path: first_path_string.clone(),
            modifiers: Default::default(),
        },
        &mut context,
    );
    run_command_for_tests(&mut state, context.into_command());
    let mut context = ui::UiUpdateContext::default();
    start_deferred_sample_load_for_tests(&mut state, first_path_string.clone(), true, &mut context);
    let ticket = active_sample_load_ticket(&state).expect("sample load queued");
    state.library.folder_browser.select_file(second_path_string);

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SamplePlaybackReady(
            sample_playback_ready_completion(
                ticket,
                first_path_string.clone(),
                crate::native_app::waveform::WaveformPlaybackReady {
                    path: first_path,
                    audio_bytes: std::sync::Arc::from(Vec::<u8>::new()),
                    playback_samples: std::sync::Arc::from(vec![0.0_f32, 0.25, -0.25, 0.5]),
                    sample_rate: 48_000,
                    channels: 1,
                    frames: 4,
                    source_len: 0,
                    source_modified: None,
                },
                true,
            ),
        ),
        &mut context,
    );

    assert_eq!(state.audio.sample_playback_session, None);
    assert_eq!(state.audio.current_playback_span, None);
    assert!(
        !state.ui.status.sample.contains("Playing"),
        "stale playback-ready messages must not start old selection audio"
    );
}

#[test]
fn stale_loaded_sample_result_does_not_replace_newer_selection() {
    let source_root = tempfile::tempdir().expect("source root");
    let first_path = source_root.path().join("first-loaded.wav");
    let second_path = source_root.path().join("second-selected.wav");
    write_test_wav_i16(&first_path, &[0, 1024, -2048, 4096]);
    write_test_wav_i16(&second_path, &[0, 512, -512, 1024]);
    let first_path_string = first_path.display().to_string();
    let second_path_string = second_path.display().to_string();

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state
        .library
        .folder_browser
        .select_file(first_path_string.clone());

    let mut context = ui::UiUpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SelectSampleWithModifiers {
            path: first_path_string.clone(),
            modifiers: Default::default(),
        },
        &mut context,
    );
    run_command_for_tests(&mut state, context.into_command());
    let mut context = ui::UiUpdateContext::default();
    start_deferred_sample_load_for_tests(&mut state, first_path_string.clone(), true, &mut context);
    let ticket = active_sample_load_ticket(&state).expect("sample load queued");
    state
        .waveform
        .load
        .selection
        .start_uncached(second_path_string.as_str());

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SampleLoadFinished(
            sample_load_completion(
                ticket,
                first_path_string,
                crate::native_app::test_support::state::WaveformState::load_path(
                    first_path.clone(),
                ),
                true,
            ),
        ),
        &mut context,
    );

    assert_ne!(
        state.waveform.current.path(),
        first_path,
        "stale sample load completion must not replace the current waveform"
    );
    assert_eq!(
        state.waveform.load.selection.selected_path.as_deref(),
        Some(second_path_string.as_str())
    );
}

#[test]
fn stale_loaded_sample_result_does_not_clear_newer_pending_playback() {
    let source_root = tempfile::tempdir().expect("source root");
    let first_path = source_root.path().join("first-loaded.wav");
    let second_path = source_root.path().join("second-selected.wav");
    write_test_wav_i16(&first_path, &[0, 1024, -2048, 4096]);
    write_test_wav_i16(&second_path, &[0, 512, -512, 1024]);
    let first_path_string = first_path.display().to_string();
    let second_path_string = second_path.display().to_string();

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state
        .library
        .folder_browser
        .select_file(first_path_string.clone());

    let mut context = ui::UiUpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SelectSampleWithModifiers {
            path: first_path_string.clone(),
            modifiers: Default::default(),
        },
        &mut context,
    );
    run_command_for_tests(&mut state, context.into_command());
    let mut context = ui::UiUpdateContext::default();
    start_deferred_sample_load_for_tests(&mut state, first_path_string.clone(), true, &mut context);
    let stale_ticket = active_sample_load_ticket(&state).expect("first sample load queued");

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SelectSampleWithModifiers {
            path: second_path_string.clone(),
            modifiers: Default::default(),
        },
        &mut context,
    );
    run_command_for_tests(&mut state, context.into_command());
    let mut context = ui::UiUpdateContext::default();
    start_deferred_sample_load_for_tests(
        &mut state,
        second_path_string.clone(),
        true,
        &mut context,
    );
    let current_ticket = active_sample_load_ticket(&state).expect("second sample load queued");
    assert_ne!(
        stale_ticket, current_ticket,
        "second foreground load should replace the first worker ticket"
    );
    state.audio.pending_sample_playback = Some(
        crate::native_app::test_support::state::SamplePlaybackRequest::waveform(
            second_path_string.clone(),
            (0.0, 1.0),
            crate::native_app::test_support::state::SamplePlaybackIntent::RandomAudition,
            "random_audition",
            crate::native_app::test_support::state::SamplePlaybackHistory::Record,
        )
        .with_random_units(0.25, 0.5),
    );

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SampleLoadFinished(
            sample_load_completion(
                stale_ticket,
                first_path_string,
                crate::native_app::test_support::state::WaveformState::load_path(
                    first_path.clone(),
                ),
                true,
            ),
        ),
        &mut context,
    );

    assert_eq!(
        state.audio.pending_sample_playback,
        Some(
            crate::native_app::test_support::state::SamplePlaybackRequest::waveform(
                second_path_string,
                (0.0, 1.0),
                crate::native_app::test_support::state::SamplePlaybackIntent::RandomAudition,
                "random_audition",
                crate::native_app::test_support::state::SamplePlaybackHistory::Record,
            )
            .with_random_units(0.25, 0.5)
        ),
        "stale completion must not cancel playback requested by the newer selection"
    );
    assert_eq!(
        active_sample_load_ticket(&state),
        Some(current_ticket),
        "newer sample load should stay active after stale completion"
    );
}
