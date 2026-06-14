use super::*;

#[test]
fn sample_selection_starts_foreground_load_for_persisted_cache_row_after_restart() {
    let config_base = tempfile::tempdir().expect("config base");
    let (_config_lock, _base_guard) =
        set_waveform_test_config_base(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("cached.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let sample_path = sample_path.display().to_string();
    let sample_name = PathBuf::from(&sample_path)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .expect("sample file name");

    let waveform = crate::native_app::test_support::state::WaveformState::load_path(
        sample_path.clone().into(),
    )
    .expect("cache sample");
    let file = waveform.file();
    crate::native_app::waveform::store_cached_waveform_file_for_tests(&file);
    wait_for_playback_ready_cache(&sample_path);

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state.refresh_persisted_waveform_cache_indicators();

    assert!(
        state
            .waveform
            .cache
            .cached_sample_paths
            .contains(&sample_path),
        "persisted cache should mark the sample as ready before it enters memory cache"
    );

    let mut context = ui::UiUpdateContext::default();
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
        "playback-ready persisted cache should not wait for a debounce after restart"
    );
    assert!(
        state.background.sample_load_task.active().is_some(),
        "playback-ready persisted cache rows should start foreground loading immediately"
    );
    assert!(
        state.waveform.load.label.as_deref() == Some(sample_name.as_str()),
        "selection should show loading state while foreground loading runs"
    );
    assert!(
        !state
            .waveform
            .cache
            .entries
            .contains_key(&PathBuf::from(&sample_path)),
        "persisted cache payloads must stay off the UI thread during selection"
    );
}

#[test]
fn sample_selection_cancels_running_persisted_cache_warm() {
    let source_root = tempfile::tempdir().expect("source root");
    let warm_path = source_root.path().join("warm.wav");
    let selected_path = source_root.path().join("selected.wav");
    write_test_wav_i16(&warm_path, &[0, 1024, -2048, 4096]);
    write_test_wav_i16(&selected_path, &[0, 512, -512, 1024]);
    let selected_path = selected_path.display().to_string();

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state
        .waveform
        .cache
        .warm_pending
        .push_back(warm_path.clone());
    let mut context = ui::UiUpdateContext::default();
    state.maybe_start_waveform_cache_warm(&mut context);
    assert!(
        state.waveform.cache.warm_task.active().is_some(),
        "test setup should start persisted cache warming"
    );
    assert!(
        state.waveform.cache.warm_cancel.is_some(),
        "persisted cache warming should be cancellable"
    );

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SelectSampleWithModifiers {
            path: selected_path,
            modifiers: Default::default(),
        },
        &mut context,
    );

    assert!(
        state.waveform.cache.warm_task.active().is_none(),
        "foreground selection must cancel an already-running persisted cache warm"
    );
    assert!(
        state.waveform.cache.warm_cancel.is_none(),
        "foreground selection must cancel the persisted warm token"
    );
    assert!(
        state.background.sample_load_task.active().is_some(),
        "foreground sample load should be queued after cancelling persisted warm work"
    );
}

#[test]
fn playback_ready_persisted_cache_marks_row_without_memory_warm_after_restart() {
    let config_base = tempfile::tempdir().expect("config base");
    let (_config_lock, _base_guard) =
        set_waveform_test_config_base(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("warm-before-click.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let sample_path_string = sample_path.display().to_string();
    let sample_path = PathBuf::from(&sample_path_string);

    let waveform =
        crate::native_app::test_support::state::WaveformState::load_path(sample_path.clone())
            .expect("cache sample");
    let file = waveform.file();
    crate::native_app::waveform::store_cached_waveform_file_for_tests(&file);

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state.refresh_persisted_waveform_cache_indicators();

    assert!(
        state
            .waveform
            .cache
            .cached_sample_paths
            .contains(&sample_path_string)
    );
    assert!(
        !state.waveform.cache.entries.contains_key(&sample_path),
        "restart indicator refresh should not synchronously deserialize cached waveforms"
    );
    assert!(
        state.waveform.cache.warm_pending.is_empty(),
        "playback-ready persisted caches should not be loaded into memory from UI refresh"
    );

    let mut context = ui::UiUpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SelectSampleWithModifiers {
            path: sample_path_string.clone(),
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
        "selection of a playback-ready cached file should not wait for debounce"
    );
    assert!(state.background.sample_load_task.active().is_some());
    assert_ne!(state.waveform.current.path(), sample_path);

    let ticket = state
        .background
        .sample_load_task
        .active()
        .expect("foreground load queued");
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SampleLoadFinished(ui::TaskCompletion {
            ticket,
            output: crate::native_app::test_support::state::SampleLoadResult {
                path: sample_path_string,
                result: crate::native_app::test_support::state::WaveformState::load_path_for_foreground_audition(
                    sample_path.clone(),
                    |_| {},
                    || false,
                    |_| {},
                ),
                autoplay: false,
            },
        }),
        &mut context,
    );

    assert_eq!(state.waveform.current.path(), sample_path);
    assert!(
        !state.waveform.current.audio_bytes().is_empty(),
        "foreground selection should decode source bytes instead of hydrating persisted cache payloads"
    );
}
