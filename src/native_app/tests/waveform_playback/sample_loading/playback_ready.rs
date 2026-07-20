use super::*;

#[test]
fn wav_load_reports_playback_ready_before_waveform_summary_completion() {
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("early-ready.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let playback_ready = std::cell::Cell::new(false);

    let waveform = crate::native_app::test_support::state::WaveformState::load_path_with_progress_cancel_and_playback_ready(
        sample_path.clone(),
        |progress| {
            if progress >= 0.62 {
                assert!(
                    playback_ready.get(),
                    "WAV playback samples should be available before waveform summary work"
                );
            }
        },
        || false,
        |ready| {
            assert_eq!(ready.path, sample_path);
            assert_eq!(ready.sample_rate, 48_000);
            assert_eq!(ready.channels, 1);
            assert!(!ready.playback_samples.is_empty());
            playback_ready.set(true);
        },
    )
    .expect("wav should load");

    assert!(playback_ready.get());
    assert!(waveform.playback_samples().is_some());
}

#[test]
fn playback_ready_message_starts_audio_before_waveform_completion() {
    if !test_audio_output_enabled() {
        return;
    }
    let Ok(player) = wavecrate::audio::AudioPlayer::new() else {
        return;
    };
    let output = player.output_details().clone();
    let Ok(runtime) = wavecrate::audio::PlaybackRuntime::spawn(
        player,
        wavecrate::audio::PlaybackRuntimeConfig::default(),
    ) else {
        return;
    };
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("early-message.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let sample_path_string = sample_path.display().to_string();

    let mut state = gui_state_for_span_tests();
    state.audio.output_resolved = Some(output);
    state.audio.playback_runtime = Some(runtime.handle);
    state.audio.playback_events = Some(runtime.events);
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state
        .library
        .folder_browser
        .select_file(sample_path_string.clone());
    let mut context = ui::UiUpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SelectSampleWithModifiers {
            path: sample_path_string.clone(),
            modifiers: Default::default(),
        },
        &mut context,
    );
    run_command_for_tests(&mut state, context.into_command());
    let mut context = ui::UiUpdateContext::default();
    start_deferred_sample_load_for_tests(
        &mut state,
        sample_path_string.clone(),
        true,
        &mut context,
    );
    let ticket = active_sample_load_ticket(&state).expect("sample load queued");
    let samples = std::sync::Arc::from(vec![0.0_f32, 0.25, -0.25, 0.5]);

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SamplePlaybackReady(
            sample_playback_ready_completion(
                ticket,
                sample_path_string.clone(),
                crate::native_app::waveform::WaveformPlaybackReady {
                    path: sample_path.clone(),
                    audio_bytes: std::sync::Arc::from(fs::read(&sample_path).expect("read wav")),
                    playback_samples: samples,
                    sample_rate: 48_000,
                    channels: 1,
                    frames: 4,
                    source_len: fs::metadata(&sample_path).expect("sample metadata").len(),
                    source_modified: fs::metadata(&sample_path)
                        .expect("sample metadata")
                        .modified()
                        .ok(),
                },
                true,
            ),
        ),
        &mut context,
    );

    let session = state
        .audio
        .sample_playback_session
        .as_ref()
        .expect("playback-ready audio should create a session");
    assert_eq!(session.request.path, sample_path_string);
    assert_eq!(session.source_kind, "decoded_samples");
    assert_eq!(state.audio.current_playback_span, Some((0.0, 1.0)));
    assert!(!state.waveform.current.has_loaded_sample());
    assert!(
        !state.waveform.current.is_playing(),
        "waveform visuals should wait for full waveform completion"
    );
    assert_eq!(state.ui.status.sample, "Playing early-message.wav");

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SampleLoadFinished(
            sample_load_completion(
                ticket,
                sample_path_string.clone(),
                crate::native_app::test_support::state::WaveformState::load_path(
                    sample_path.clone(),
                ),
                true,
            ),
        ),
        &mut context,
    );

    assert!(
        state
            .audio
            .active_sample_playback_updates_waveform(&sample_path_string)
    );
    assert!(state.waveform.current.is_playing());
    assert_eq!(state.audio.current_playback_span, Some((0.0, 1.0)));
}

#[test]
fn display_after_preview_waits_for_settled_full_playback_promotion() {
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("preview-display.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let sample_path_string = sample_path.display().to_string();

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state
        .library
        .folder_browser
        .select_file(sample_path_string.clone());
    crate::native_app::test_support::state::seed_sample_playback_session(
        &mut state,
        sample_path_string.clone(),
        "preview_samples",
    );

    let mut context = ui::UiUpdateContext::default();
    state.load_navigation_sample_validated(
        sample_path_string.clone(),
        &mut context,
        std::time::Instant::now(),
    );
    let ticket = active_sample_load_ticket(&state).expect("display load queued");

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SampleLoadFinished(
            sample_load_completion_with_display_after_instant_audition(
                ticket,
                sample_path_string.clone(),
                crate::native_app::test_support::state::WaveformState::load_path(
                    sample_path.clone(),
                ),
                true,
            ),
        ),
        &mut context,
    );

    assert_eq!(
        state.audio.active_sample_playback_path(),
        Some(sample_path_string.as_str()),
        "display-only completion should keep the preview marker for the settle handoff"
    );
    assert!(
        state
            .audio
            .active_sample_playback_is_preview(&sample_path_string)
    );
    assert!(
        state.ui.status.sample.contains("Preparing"),
        "display-only completion should not claim full playback has started"
    );
    assert!(
        !state.waveform.current.is_playing(),
        "display-only completion must not mark the waveform as playing the full sample"
    );
}

#[test]
fn completed_streamed_navigation_does_not_restart_when_waveform_load_finishes() {
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("completed-stream.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let sample_path_string = sample_path.display().to_string();

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state
        .library
        .folder_browser
        .select_file(sample_path_string.clone());
    crate::native_app::test_support::state::seed_sample_playback_session(
        &mut state,
        sample_path_string.clone(),
        "audio_file",
    );
    state
        .audio
        .sample_playback_session
        .as_mut()
        .expect("streamed navigation session")
        .state = crate::native_app::app::SamplePlaybackSessionState::AudibleTransient;

    let mut context = ui::UiUpdateContext::default();
    state.load_navigation_sample_validated(
        sample_path_string.clone(),
        &mut context,
        std::time::Instant::now(),
    );
    let ticket = active_sample_load_ticket(&state).expect("waveform load queued");
    state.audio.playback_progress = wavecrate::audio::PlaybackRuntimeProgress {
        active: false,
        elapsed: Some(std::time::Duration::from_secs(1)),
        looping: false,
        progress: Some(1.0),
        error: None,
    };
    state.refresh_runtime_playback_progress();

    assert!(
        state.audio.sample_playback_session.is_none(),
        "terminal transient playback should leave no active session"
    );

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SampleLoadFinished(
            sample_load_completion(
                ticket,
                sample_path_string,
                crate::native_app::test_support::state::WaveformState::load_path(
                    sample_path.clone(),
                ),
                true,
            ),
        ),
        &mut context,
    );

    assert!(
        state.audio.pending_playback_start.is_none(),
        "completed streamed playback must not queue a second start from frame zero"
    );
    assert!(state.audio.sample_playback_session.is_none());
    assert_eq!(state.audio.current_playback_span, None);
    assert!(!state.waveform.current.is_playing());
    assert_eq!(
        state.waveform.current.played_ranges(),
        &[wavecrate::selection::SelectionRange::new(0.0, 1.0)]
    );
}

#[test]
fn settled_preview_promotion_starts_full_playback_for_current_loaded_sample() {
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("settled-full.wav");
    write_sparse_test_wav_i16(&sample_path, 1, 48_000);
    let sample_path_string = sample_path.display().to_string();

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state
        .library
        .folder_browser
        .select_file(sample_path_string.clone());
    state.waveform.current =
        crate::native_app::test_support::state::WaveformState::load_path(sample_path.clone())
            .expect("sample loads");
    crate::native_app::test_support::state::seed_sample_playback_session(
        &mut state,
        sample_path_string.clone(),
        "preview_samples",
    );
    let preview_session = state
        .audio
        .sample_playback_session
        .as_mut()
        .expect("preview session");
    preview_session.state = crate::native_app::app::SamplePlaybackSessionState::AudibleTransient;
    preview_session.audible_started_at =
        Some(std::time::Instant::now() - std::time::Duration::from_millis(110));
    state.audio.playback_progress = wavecrate::audio::PlaybackRuntimeProgress {
        active: true,
        elapsed: Some(std::time::Duration::ZERO),
        looping: false,
        progress: Some(0.0),
        error: None,
    };
    let ticket = state.background.settled_sample_promotion_task.begin();
    let mut context = ui::UiUpdateContext::default();

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SettledSamplePromotion {
            ticket,
            path: sample_path_string.clone(),
            scheduled_at: std::time::Instant::now(),
        },
        &mut context,
    );

    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(sample_path_string.as_str())
    );
    assert_eq!(state.waveform.current.path(), sample_path);
    assert!(
        state.waveform.current.is_playing()
            || state.audio.pending_playback_start.is_some()
            || state.audio.sample_playback_session.is_some()
            || state.audio.current_playback_span == Some((0.0, 1.0)),
        "settled promotion should hand the current loaded sample to full playback"
    );
    let pending = state
        .audio
        .pending_playback_start
        .expect("no-runtime test should queue the promoted full playback start");
    assert!(
        (pending.intent.start_ratio - 0.11).abs() < 0.01,
        "settled preview handoff should continue past the heard preview instead of replaying from the top"
    );
    assert_eq!(pending.intent.end_ratio, 1.0);
    assert_eq!(
        state.waveform.current.play_mark_ratio(),
        Some(0.0),
        "the start marker should describe where the audition began, not the continuation ratio"
    );
    let heard = state
        .waveform
        .current
        .played_ranges()
        .first()
        .expect("preview handoff should preserve the already auditioned prefix");
    assert!((heard.start() - 0.0).abs() < 0.001);
    assert!(
        (heard.end() - pending.intent.start_ratio).abs() < 0.01,
        "played history should begin at the sample start and meet the continuing playhead"
    );
}

#[test]
fn settled_preview_promotion_does_not_restart_already_promoted_full_sample() {
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("already-promoted.wav");
    write_sparse_test_wav_i16(&sample_path, 1, 48_000);
    let sample_path_string = sample_path.display().to_string();

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state
        .library
        .folder_browser
        .select_file(sample_path_string.clone());
    state.waveform.current =
        crate::native_app::test_support::state::WaveformState::load_path(sample_path.clone())
            .expect("sample loads");
    state.audio.pending_playback_start =
        Some(crate::native_app::app::PendingPlaybackStart::record(
            crate::native_app::audio::playback::PlaybackIntent::new(0.11, 1.0),
        ));
    let ticket = state.background.settled_sample_promotion_task.begin();
    let mut context = ui::UiUpdateContext::default();

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SettledSamplePromotion {
            ticket,
            path: sample_path_string.clone(),
            scheduled_at: std::time::Instant::now(),
        },
        &mut context,
    );

    let pending = state
        .audio
        .pending_playback_start
        .expect("already promoted full playback should remain pending");
    assert_eq!(
        pending.intent.start_ratio, 0.11,
        "stale settled promotion must not restart the same full sample"
    );
    assert_eq!(state.waveform.current.path(), sample_path);
}

#[test]
fn stale_settled_preview_promotion_does_not_start_old_full_load() {
    let source_root = tempfile::tempdir().expect("source root");
    let first_path = source_root.path().join("first-preview.wav");
    let second_path = source_root.path().join("second-preview.wav");
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
        .select_file(second_path_string.clone());
    let stale_ticket = state.background.settled_sample_promotion_task.begin();
    let latest_ticket = state.background.settled_sample_promotion_task.begin();
    let mut context = ui::UiUpdateContext::default();

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SettledSamplePromotion {
            ticket: stale_ticket,
            path: first_path_string,
            scheduled_at: std::time::Instant::now(),
        },
        &mut context,
    );

    assert!(
        active_sample_load_ticket(&state).is_none(),
        "stale settle promotion must not queue a full load for a transient row"
    );
    assert_eq!(
        state.background.settled_sample_promotion_task.active(),
        Some(latest_ticket),
        "stale settle promotion must leave the latest settle ticket active"
    );

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SettledSamplePromotion {
            ticket: latest_ticket,
            path: second_path_string.clone(),
            scheduled_at: std::time::Instant::now(),
        },
        &mut context,
    );

    assert!(
        active_sample_load_ticket(&state).is_some(),
        "the latest still-selected row should be promoted to full sample loading"
    );
    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(second_path_string.as_str())
    );
}

#[test]
fn uncached_sample_load_clears_previous_waveform_until_current_waveform_finishes() {
    let source_root = tempfile::tempdir().expect("source root");
    let previous_path = source_root.path().join("previous.wav");
    let selected_path = source_root.path().join("selected.wav");
    write_test_wav_i16(&previous_path, &[0, 256, -256, 512]);
    write_test_wav_i16(&selected_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let selected_path_string = selected_path.display().to_string();

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state.waveform.current =
        crate::native_app::test_support::state::WaveformState::load_path(previous_path)
            .expect("previous waveform loads");
    assert!(state.waveform.current.has_loaded_sample());

    let mut context = ui::UiUpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SelectSampleWithModifiers {
            path: selected_path_string.clone(),
            modifiers: Default::default(),
        },
        &mut context,
    );
    run_command_for_tests(&mut state, context.into_command());

    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(selected_path_string.as_str())
    );
    assert!(
        !state.waveform.current.has_loaded_sample(),
        "starting an uncached load must not keep drawing the previous sample waveform"
    );
    assert_eq!(state.waveform.load.label.as_deref(), Some("selected.wav"));
    assert!(state.waveform_input_blocked_by_sample_load());
    let frame = crate::native_app::test_support::state::view(&state)
        .view_frame_at_size_with_default_theme(ui::Vector2::new(900.0, 620.0));
    assert!(
        frame.paint_plan.contains_text("Loading selected.wav"),
        "waveform panel should identify the current sample loading state"
    );
    assert!(
        active_sample_load_ticket(&state).is_some(),
        "uncached sample selection should queue foreground sample loading"
    );
}

#[test]
fn uncached_sample_load_keeps_playing_waveform_visible_until_replacement_is_ready() {
    let source_root = tempfile::tempdir().expect("source root");
    let previous_path = source_root.path().join("previous-playing.wav");
    let selected_path = source_root.path().join("selected.wav");
    write_test_wav_i16(&previous_path, &[0, 256, -256, 512]);
    write_test_wav_i16(&selected_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let selected_path_string = selected_path.display().to_string();

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state.waveform.current =
        crate::native_app::test_support::state::WaveformState::load_path(previous_path.clone())
            .expect("previous waveform loads");
    state.waveform.current.start_playback(0.25);
    state.audio.current_playback_span = Some((0.0, 1.0));

    let mut context = ui::UiUpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SelectSampleWithModifiers {
            path: selected_path_string.clone(),
            modifiers: Default::default(),
        },
        &mut context,
    );
    run_command_for_tests(&mut state, context.into_command());

    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(selected_path_string.as_str())
    );
    assert_eq!(
        state.waveform.current.path(),
        previous_path,
        "the audible sample waveform should remain visible while replacement decode is queued"
    );
    assert!(state.waveform.current.has_loaded_sample());
    assert!(
        !state.waveform.current.is_playing(),
        "the old visual should not keep advertising active waveform playback after stop is requested"
    );
    assert_eq!(state.audio.current_playback_span, None);
    assert_eq!(state.waveform.load.label.as_deref(), Some("selected.wav"));
    let frame = crate::native_app::test_support::state::view(&state)
        .view_frame_at_size_with_default_theme(ui::Vector2::new(900.0, 620.0));
    assert!(
        frame.paint_plan.contains_text("Loading selected.wav"),
        "waveform panel should still show the selected sample loading state"
    );
    assert!(
        active_sample_load_ticket(&state).is_some(),
        "uncached sample selection should queue foreground sample loading"
    );
}
