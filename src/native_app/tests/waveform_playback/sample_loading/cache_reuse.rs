use super::*;

#[test]
fn sample_selection_loads_selected_file_into_waveform() {
    let mut state = crate::native_app::test_support::state::NativeAppStateFixture::default()
        .with_synthetic_waveform()
        .with_sample_status("")
        .build();
    let sample_path = first_visible_asset_file_path(&state.library.folder_browser);
    let sample_name = PathBuf::from(&sample_path)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .expect("sample file name");

    let mut context = ui::UiUpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SelectSampleWithModifiers {
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
        crate::native_app::test_support::state::GuiMessage::SampleLoadFinished(
            ui::TaskCompletion {
                ticket,
                output: crate::native_app::test_support::state::SampleLoadResult {
                    path: sample_path.clone(),
                    result: crate::native_app::test_support::state::WaveformState::load_path(
                        sample_path.clone().into(),
                    ),
                    autoplay: true,
                },
            },
        ),
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
        crate::native_app::test_support::state::GuiMessage::SelectSampleWithModifiers {
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
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    let loaded =
        crate::native_app::test_support::state::WaveformState::load_path(sample_path.clone())
            .expect("sample loads");
    state.remember_waveform(&loaded);
    state.waveform.current =
        crate::native_app::test_support::state::WaveformState::synthetic_for_tests();
    state.waveform.load.label = Some(String::from("previous.wav"));
    state.waveform.load.progress = 0.42;
    state.waveform.load.target_progress = 0.84;

    let mut context = ui::UiUpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SelectSampleWithModifiers {
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
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);

    let cached =
        crate::native_app::test_support::state::WaveformState::load_path(cached_path.clone())
            .expect("sample loads");
    state.remember_waveform(&cached);

    state.waveform.current =
        crate::native_app::test_support::state::WaveformState::load_path(current_path)
            .expect("current sample loads");
    state.waveform.current.start_playback(0.25);
    state.audio.current_playback_span = Some((0.25, 1.0));

    let mut context = ui::UiUpdateContext::default();
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
