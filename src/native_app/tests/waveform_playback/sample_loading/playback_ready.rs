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
fn playback_ready_message_waits_for_waveform_before_starting_audio() {
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
                },
                true,
            ),
        ),
        &mut context,
    );

    assert_eq!(state.audio.early_sample_playback_path, None);
    assert_eq!(state.audio.current_playback_span, None);
    assert!(state.audio.pending_runtime_start.is_none());
    assert!(!state.waveform.current.has_loaded_sample());
    assert!(
        !state.waveform.current.is_playing(),
        "waveform visuals should wait for full waveform completion"
    );
    assert_eq!(state.ui.status.sample, "Preparing early-message.wav");

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

    assert_eq!(state.audio.early_sample_playback_path, None);
    assert!(state.waveform.current.is_playing());
    assert_eq!(state.audio.current_playback_span, Some((0.0, 1.0)));
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
    let frame = crate::native_app::test_support::state::view(&mut state)
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
