use super::*;

#[test]
fn sample_selection_loads_selected_file_into_waveform() {
    let mut state = crate::native_app::test_support::NativeAppStateFixture::default()
        .with_synthetic_waveform()
        .with_sample_status("")
        .build();
    let sample_path = first_visible_asset_file_path(&state.library.folder_browser);
    let sample_name = PathBuf::from(&sample_path)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .expect("sample file name");

    let mut context = ui::UpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::GuiMessage::SelectSampleWithModifiers {
            path: sample_path.clone(),
            modifiers: Default::default(),
        },
        &mut context,
    );
    assert_eq!(
        state.waveform.load.label.as_deref(),
        Some(sample_name.as_str())
    );
    assert!(
        state
            .background
            .deferred_sample_load_task
            .active()
            .is_some(),
        "selection should debounce uncached sample loading before queueing decode work"
    );
    start_deferred_sample_load_for_tests(&mut state, sample_path.clone(), true, &mut context);
    let ticket = state
        .background
        .sample_load_task
        .active()
        .expect("sample load queued");
    state.apply_message(
        crate::native_app::test_support::GuiMessage::SampleLoadFinished(ui::TaskCompletion {
            ticket,
            output: crate::native_app::test_support::SampleLoadResult {
                path: sample_path.clone(),
                result: crate::native_app::test_support::WaveformState::load_path(
                    sample_path.clone().into(),
                ),
                autoplay: true,
            },
        }),
        &mut context,
    );

    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(sample_path.as_str())
    );
    assert_eq!(state.waveform.current.file_name(), sample_name);
    assert_eq!(state.waveform.load.label, None);
    assert!(state.waveform.current.frames() > 0);
    assert!(state.ui.status.sample.contains(&sample_name));
    assert!(
        state
            .waveform
            .cache
            .cached_sample_paths
            .contains(&sample_path)
    );

    state.apply_message(
        crate::native_app::test_support::GuiMessage::SelectSampleWithModifiers {
            path: sample_path.clone(),
            modifiers: Default::default(),
        },
        &mut context,
    );

    assert!(
        state
            .background
            .deferred_sample_load_task
            .active()
            .is_none(),
        "repeat selection should use the memory waveform cache without a deferred worker"
    );
    assert!(
        state.background.sample_load_task.active().is_none(),
        "repeat selection must not start decode work"
    );
    assert_eq!(state.waveform.current.file_name(), sample_name);
}

#[test]
fn repeat_sample_selection_uses_memory_waveform_cache_without_worker() {
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("resident.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let sample_path_string = sample_path.display().to_string();

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    let loaded = crate::native_app::test_support::WaveformState::load_path(sample_path.clone())
        .expect("sample loads");
    state.remember_waveform(&loaded);
    state.waveform.current = crate::native_app::test_support::WaveformState::synthetic_for_tests();
    state.waveform.load.label = Some(String::from("previous.wav"));
    state.waveform.load.progress = 0.42;
    state.waveform.load.target_progress = 0.84;

    let mut context = ui::UpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::GuiMessage::SelectSampleWithModifiers {
            path: sample_path_string.clone(),
            modifiers: Default::default(),
        },
        &mut context,
    );

    assert_eq!(state.waveform.current.path(), sample_path);
    assert_eq!(state.waveform.load.label, None);
    assert_eq!(state.waveform.load.progress, 0.0);
    assert_eq!(state.waveform.load.target_progress, 0.0);
    assert!(
        state
            .background
            .deferred_sample_load_task
            .active()
            .is_none(),
        "memory-cached repeat selection should not debounce a reload"
    );
    assert!(
        state.background.sample_load_task.active().is_none(),
        "memory-cached repeat selection should not queue decode work"
    );
    assert!(
        state.ui.status.sample.contains("resident.wav"),
        "cached selection should update the visible status, got {}",
        state.ui.status.sample
    );
    assert!(
        state
            .waveform
            .cache
            .cached_sample_paths
            .contains(&sample_path_string)
    );
}

#[test]
fn memory_cached_load_without_autoplay_stops_current_playback_state() {
    let source_root = tempfile::tempdir().expect("source root");
    let current_path = source_root.path().join("current.wav");
    let cached_path = source_root.path().join("cached.wav");
    write_test_wav_i16(&current_path, &[0, 256, -256, 512]);
    write_test_wav_i16(&cached_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let cached_path_string = cached_path.display().to_string();

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);

    let cached = crate::native_app::test_support::WaveformState::load_path(cached_path.clone())
        .expect("sample loads");
    state.remember_waveform(&cached);

    state.waveform.current =
        crate::native_app::test_support::WaveformState::load_path(current_path)
            .expect("current sample loads");
    state.waveform.current.start_playback(0.25);
    state.audio.current_playback_span = Some((0.25, 1.0));

    let mut context = ui::UpdateContext::default();
    state.load_sample_without_autoplay(cached_path_string, &mut context);

    assert_eq!(state.waveform.current.path(), cached_path);
    assert!(!state.waveform.current.is_playing());
    assert_eq!(state.audio.current_playback_span, None);
    assert!(
        state
            .background
            .deferred_sample_load_task
            .active()
            .is_none(),
        "memory-cached non-autoplay load should not debounce a reload"
    );
    assert!(
        state.background.sample_load_task.active().is_none(),
        "memory-cached non-autoplay load should not queue decode work"
    );
    assert!(
        state.ui.status.sample.contains("Loaded cached.wav"),
        "cached non-autoplay load should update status, got {}",
        state.ui.status.sample
    );
}

#[test]
fn keyboard_and_mouse_uncached_selection_use_same_fast_debounce() {
    assert_eq!(
        crate::native_app::test_support::KEYBOARD_SAMPLE_LOAD_DEBOUNCE,
        crate::native_app::test_support::UNCACHED_SAMPLE_LOAD_DEBOUNCE,
        "keyboard navigation should not wait longer than mouse selection before audition loading"
    );
}

#[test]
fn uncached_selected_sample_load_uses_foreground_priority() {
    assert_eq!(
        crate::native_app::audio::sample_load_actions::foreground_sample_load_priority(),
        ui::TaskPriority::Interactive,
        "selected uncached audition loads must outrank background cache warming"
    );
}

#[test]
fn active_folder_cache_warm_uses_lower_priority_than_selected_sample_load() {
    assert_eq!(
        crate::native_app::audio::sample_load_actions::active_folder_cache_warm_priority(),
        ui::TaskPriority::Idle
    );
    assert_ne!(
        crate::native_app::audio::sample_load_actions::foreground_sample_load_priority(),
        crate::native_app::audio::sample_load_actions::active_folder_cache_warm_priority(),
        "background folder warming must not share the foreground audition lane"
    );
}

#[test]
fn frame_queues_audio_output_warm_up_before_explicit_playback() {
    let mut state = gui_state_for_span_tests();
    assert!(state.audio.player.is_none());
    assert!(state.background.audio_open_task.active().is_none());

    let mut context = ui::UpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::GuiMessage::Frame,
        &mut context,
    );

    assert!(
        state.background.audio_open_task.active().is_some(),
        "frame processing should begin audio output warm-up before the first explicit playback"
    );
}

#[test]
fn wav_load_reports_playback_ready_before_waveform_summary_completion() {
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("early-ready.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let playback_ready = std::cell::Cell::new(false);

    let waveform = crate::native_app::test_support::WaveformState::load_path_with_progress_cancel_and_playback_ready(
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
fn playback_ready_message_starts_audio_before_full_waveform_finish() {
    let Ok(player) = wavecrate::audio::AudioPlayer::new() else {
        return;
    };
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("early-message.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let sample_path_string = sample_path.display().to_string();

    let mut state = gui_state_for_span_tests();
    state.audio.player = Some(player);
    state.library.folder_browser =
        crate::native_app::test_support::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state
        .library
        .folder_browser
        .select_file(sample_path_string.clone());
    let mut context = ui::UpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::GuiMessage::SelectSampleWithModifiers {
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
    let ticket = state
        .background
        .sample_load_task
        .active()
        .expect("sample load queued");
    let samples = std::sync::Arc::from(vec![0.0_f32, 0.25, -0.25, 0.5]);

    state.apply_message(
        crate::native_app::test_support::GuiMessage::SamplePlaybackReady(ui::TaskCompletion {
            ticket,
            output: crate::native_app::test_support::SamplePlaybackReady {
                path: sample_path_string.clone(),
                audio: crate::native_app::waveform::WaveformPlaybackReady {
                    path: sample_path.clone(),
                    audio_bytes: std::sync::Arc::from(fs::read(&sample_path).expect("read wav")),
                    playback_samples: samples,
                    sample_rate: 48_000,
                    channels: 1,
                    frames: 4,
                },
                autoplay: true,
            },
        }),
        &mut context,
    );

    assert_eq!(
        state.audio.early_sample_playback_path.as_deref(),
        Some(sample_path_string.as_str())
    );
    assert_eq!(state.audio.current_playback_span, Some((0.0, 1.0)));
    assert!(
        !state.waveform.current.is_playing(),
        "waveform visuals should wait for full waveform completion"
    );

    state.apply_message(
        crate::native_app::test_support::GuiMessage::SampleLoadFinished(ui::TaskCompletion {
            ticket,
            output: crate::native_app::test_support::SampleLoadResult {
                path: sample_path_string.clone(),
                result: crate::native_app::test_support::WaveformState::load_path(
                    sample_path.clone(),
                ),
                autoplay: true,
            },
        }),
        &mut context,
    );

    assert_eq!(state.audio.early_sample_playback_path, None);
    assert!(state.waveform.current.is_playing());
    assert_eq!(state.audio.current_playback_span, Some((0.0, 1.0)));
}

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
        crate::native_app::test_support::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state
        .library
        .folder_browser
        .select_file(first_path_string.clone());
    let mut context = ui::UpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::GuiMessage::SelectSampleWithModifiers {
            path: first_path_string.clone(),
            modifiers: Default::default(),
        },
        &mut context,
    );
    start_deferred_sample_load_for_tests(&mut state, first_path_string.clone(), true, &mut context);
    let ticket = state
        .background
        .sample_load_task
        .active()
        .expect("sample load queued");
    state.library.folder_browser.select_file(second_path_string);

    state.apply_message(
        crate::native_app::test_support::GuiMessage::SamplePlaybackReady(ui::TaskCompletion {
            ticket,
            output: crate::native_app::test_support::SamplePlaybackReady {
                path: first_path_string.clone(),
                audio: crate::native_app::waveform::WaveformPlaybackReady {
                    path: first_path,
                    audio_bytes: std::sync::Arc::from(Vec::<u8>::new()),
                    playback_samples: std::sync::Arc::from(vec![0.0_f32, 0.25, -0.25, 0.5]),
                    sample_rate: 48_000,
                    channels: 1,
                    frames: 4,
                },
                autoplay: true,
            },
        }),
        &mut context,
    );

    assert_eq!(state.audio.early_sample_playback_path, None);
    assert_eq!(state.audio.current_playback_span, None);
    assert!(
        !state.ui.status.sample.contains("Playing"),
        "stale playback-ready messages must not start old selection audio"
    );
}
