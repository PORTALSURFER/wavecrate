use super::*;

#[test]
fn folder_activation_schedules_cache_indicator_refresh_without_ui_thread_probe() {
    let config_base = tempfile::tempdir().expect("config base");
    let (_config_lock, _base_guard) =
        set_waveform_test_config_base(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let folder = source_root.path().join("large-folder");
    fs::create_dir_all(&folder).expect("create folder");
    let sample_path = folder.join("cached.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let sample_path_string = sample_path.display().to_string();

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
    assert!(
        !state
            .waveform
            .cache
            .cached_sample_paths
            .contains(&sample_path_string)
    );

    let mut context = ui::UiUpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::FolderBrowser(
            crate::native_app::test_support::state::FolderBrowserMessage::ActivateFolder(
                folder.display().to_string(),
                Default::default(),
            ),
        ),
        &mut context,
    );

    assert!(
        state
            .waveform
            .cache
            .indicator_refresh_task
            .active()
            .is_some(),
        "folder activation should queue cache indicator probing off the UI thread"
    );
    assert!(
        !state
            .waveform
            .cache
            .cached_sample_paths
            .contains(&sample_path_string),
        "folder activation must not synchronously read persisted cache metadata"
    );
    assert!(
        state.waveform.cache.warm_pending.is_empty(),
        "summary cache warming should wait for the background indicator probe"
    );
}

#[test]
fn folder_activation_delays_active_folder_cache_warm() {
    let config_base = tempfile::tempdir().expect("config base");
    let (_config_lock, _base_guard) =
        set_waveform_test_config_base(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let folder = source_root.path().join("large-folder");
    fs::create_dir_all(&folder).expect("create folder");
    let first = folder.join("first.wav");
    let second = folder.join("second.wav");
    write_test_wav_i16(&first, &[0, 1024, -2048, 4096, -1024, 512]);
    write_test_wav_i16(&second, &[0, 512, -512, 1024, -1024, 0]);

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    let mut context = ui::UiUpdateContext::default();

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::FolderBrowser(
            crate::native_app::test_support::state::FolderBrowserMessage::ActivateFolder(
                folder.display().to_string(),
                Default::default(),
            ),
        ),
        &mut context,
    );

    assert!(
        state
            .waveform
            .cache
            .active_folder_warm_delay_task
            .active()
            .is_some(),
        "folder activation should wait briefly before assuming browse intent"
    );
    assert!(
        active_folder_cache_warm_ticket(&state).is_none(),
        "active folder cache warm must not start during folder activation"
    );
    assert_eq!(state.waveform.cache.active_folder_warm_pending.len(), 2);
}

#[test]
fn folder_activation_queues_entire_source_for_background_cache_warm() {
    let config_base = tempfile::tempdir().expect("config base");
    let (_config_lock, _base_guard) =
        set_waveform_test_config_base(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let folder = source_root.path().join("large-folder");
    let nested = source_root.path().join("nested");
    fs::create_dir_all(&folder).expect("create folder");
    fs::create_dir_all(&nested).expect("create nested folder");
    for index in 0..8 {
        fs::write(folder.join(format!("sample-{index:03}.wav")), []).expect("write sample");
    }
    fs::write(nested.join("nested.wav"), []).expect("write nested sample");

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    let mut context = ui::UiUpdateContext::default();

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::FolderBrowser(
            crate::native_app::test_support::state::FolderBrowserMessage::ActivateFolder(
                folder.display().to_string(),
                Default::default(),
            ),
        ),
        &mut context,
    );

    assert_eq!(
        state.waveform.cache.active_folder_warm_pending.len(),
        9,
        "background cache warming should cover the whole selected source, not only the active folder"
    );
}

#[test]
fn active_folder_cache_warm_tracks_worker_progress() {
    let config_base = tempfile::tempdir().expect("config base");
    let (_config_lock, _base_guard) =
        set_waveform_test_config_base(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    write_test_wav_i16(
        &source_root.path().join("first.wav"),
        &[0, 1024, -2048, 4096],
    );
    write_test_wav_i16(
        &source_root.path().join("second.wav"),
        &[0, 512, -512, 1024],
    );

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    let mut context = ui::UiUpdateContext::default();
    state.schedule_active_folder_cache_warm(&mut context);

    assert_eq!(state.waveform.cache.active_folder_warm_completed, 0);
    assert_eq!(state.waveform.cache.active_folder_warm_total, 2);

    let warm_ticket = state
        .waveform
        .cache
        .active_folder_warm_delay_task
        .active()
        .expect("source warm delay");
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::ActiveFolderCacheWarmReady(warm_ticket),
        &mut context,
    );

    let running_ticket = active_folder_cache_warm_ticket(&state).expect("source warm task");
    assert_eq!(state.waveform.cache.active_folder_warm_completed, 0);
    assert!(state.waveform.cache.active_folder_warm_current.is_some());

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::ActiveFolderCacheWarmFinished(
            active_folder_cache_warm_completion(
                running_ticket,
                source_root.path().display().to_string(),
                Vec::new(),
                false,
            ),
        ),
        &mut context,
    );

    assert_eq!(state.waveform.cache.active_folder_warm_completed, 1);
    assert_eq!(state.waveform.cache.active_folder_warm_total, 2);
    assert!(state.waveform.cache.active_folder_warm_current.is_none());
    assert!(
        active_folder_cache_warm_ticket(&state).is_none(),
        "cache warm should cool down between files instead of chaining immediately"
    );
    assert!(
        state
            .waveform
            .cache
            .active_folder_warm_delay_task
            .active()
            .is_some(),
        "cache warm should schedule the next file after a delay"
    );
}

#[test]
fn active_folder_cache_warm_waits_while_sample_load_is_foreground() {
    let source_root = tempfile::tempdir().expect("source root");
    let folder = source_root.path().join("large-folder");
    fs::create_dir_all(&folder).expect("create folder");
    let first = folder.join("first.wav");
    let second = folder.join("second.wav");
    write_test_wav_i16(&first, &[0, 1024, -2048, 4096]);
    write_test_wav_i16(&second, &[0, 512, -512, 1024]);

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    let mut context = ui::UiUpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::FolderBrowser(
            crate::native_app::test_support::state::FolderBrowserMessage::ActivateFolder(
                folder.display().to_string(),
                Default::default(),
            ),
        ),
        &mut context,
    );
    let warm_ticket = state
        .waveform
        .cache
        .active_folder_warm_delay_task
        .active()
        .expect("folder warm delay");

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SelectSampleWithModifiers {
            path: first.display().to_string(),
            modifiers: Default::default(),
        },
        &mut context,
    );
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::ActiveFolderCacheWarmReady(warm_ticket),
        &mut context,
    );

    assert!(
        active_folder_cache_warm_ticket(&state).is_none(),
        "background folder cache warm must not start while a foreground sample load is pending"
    );
    assert!(
        state
            .waveform
            .cache
            .active_folder_warm_delay_task
            .active()
            .is_some(),
        "folder cache warm should retry later instead of competing with selection"
    );
    assert_eq!(
        state.waveform.cache.active_folder_warm_pending.len(),
        2,
        "foreground selection must not drain warm candidates"
    );
}

#[test]
fn sample_selection_cancels_running_active_folder_cache_warm() {
    let source_root = tempfile::tempdir().expect("source root");
    let folder = source_root.path().join("large-folder");
    fs::create_dir_all(&folder).expect("create folder");
    let first = folder.join("first.wav");
    let second = folder.join("second.wav");
    write_test_wav_i16(&first, &[0, 1024, -2048, 4096]);
    write_test_wav_i16(&second, &[0, 512, -512, 1024]);

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    let mut context = ui::UiUpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::FolderBrowser(
            crate::native_app::test_support::state::FolderBrowserMessage::ActivateFolder(
                folder.display().to_string(),
                Default::default(),
            ),
        ),
        &mut context,
    );
    let warm_ticket = state
        .waveform
        .cache
        .active_folder_warm_delay_task
        .active()
        .expect("folder warm delay");
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::ActiveFolderCacheWarmReady(warm_ticket),
        &mut context,
    );
    assert!(
        active_folder_cache_warm_ticket(&state).is_some(),
        "test setup should start active-folder cache warming"
    );

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SelectSampleWithModifiers {
            path: first.display().to_string(),
            modifiers: Default::default(),
        },
        &mut context,
    );

    assert!(
        active_folder_cache_warm_ticket(&state).is_none(),
        "foreground sample selection must cancel an already-running active-folder cache warm"
    );
    assert!(
        state.waveform.cache.active_folder_warm_cancel.is_none(),
        "foreground selection must cancel the active-folder worker token"
    );
    assert!(
        active_sample_load_ticket(&state).is_some(),
        "foreground sample load should be queued after cancelling background warm work"
    );
}

#[test]
fn active_folder_cache_warm_yields_while_normalization_is_active() {
    let config_base = tempfile::tempdir().expect("config base");
    let (_config_lock, _base_guard) =
        set_waveform_test_config_base(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("normalize-yield.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096]);

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    let mut context = ui::UiUpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::FolderBrowser(
            crate::native_app::test_support::state::FolderBrowserMessage::ActivateFolder(
                source_root.path().display().to_string(),
                Default::default(),
            ),
        ),
        &mut context,
    );
    state.background.normalization_progress = Some(
        crate::native_app::test_support::state::NormalizationProgress {
            task_id: 12,
            label: String::from("1 sample"),
            completed: 0,
            total: 1,
            work_completed: 100,
            work_total: 1_000,
            queued: 0,
            detail: String::from("normalize-yield.wav"),
        },
    );

    let warm_ticket = state
        .waveform
        .cache
        .active_folder_warm_delay_task
        .active()
        .expect("folder warm delay");
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::ActiveFolderCacheWarmReady(warm_ticket),
        &mut context,
    );

    assert!(
        active_folder_cache_warm_ticket(&state).is_none(),
        "normalization should keep active-folder cache warm from starting"
    );
    assert!(
        state
            .waveform
            .cache
            .active_folder_warm_delay_task
            .active()
            .is_some(),
        "cache warm should be delayed for later instead of competing with normalization"
    );
}

#[test]
fn changing_folder_cancels_previous_active_folder_cache_warm() {
    let config_base = tempfile::tempdir().expect("config base");
    let (_config_lock, _base_guard) =
        set_waveform_test_config_base(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let first_folder = source_root.path().join("first-folder");
    let second_folder = source_root.path().join("second-folder");
    fs::create_dir_all(&first_folder).expect("create first folder");
    fs::create_dir_all(&second_folder).expect("create second folder");
    write_test_wav_i16(&first_folder.join("first.wav"), &[0, 1024, -2048, 4096]);
    write_test_wav_i16(&second_folder.join("second.wav"), &[0, 512, -512, 1024]);

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    let mut context = ui::UiUpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::FolderBrowser(
            crate::native_app::test_support::state::FolderBrowserMessage::ActivateFolder(
                first_folder.display().to_string(),
                Default::default(),
            ),
        ),
        &mut context,
    );
    let first_ticket = state
        .waveform
        .cache
        .active_folder_warm_delay_task
        .active()
        .expect("first folder warm delay");
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::ActiveFolderCacheWarmReady(
            first_ticket,
        ),
        &mut context,
    );
    assert!(active_folder_cache_warm_ticket(&state).is_some());

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::FolderBrowser(
            crate::native_app::test_support::state::FolderBrowserMessage::ActivateFolder(
                second_folder.display().to_string(),
                Default::default(),
            ),
        ),
        &mut context,
    );

    assert!(
        active_folder_cache_warm_ticket(&state).is_none(),
        "changing folders should cancel the active warm worker"
    );
    let source_warm_id = source_root.path().display().to_string();
    assert_eq!(
        state.waveform.cache.active_folder_warm_folder_id.as_deref(),
        Some(source_warm_id.as_str())
    );
    assert_eq!(state.waveform.cache.active_folder_warm_pending.len(), 2);
}

#[test]
fn active_folder_cache_warm_does_not_chain_batches_while_playing() {
    let source_root = tempfile::tempdir().expect("source root");
    let folder = source_root.path().join("large-folder");
    fs::create_dir_all(&folder).expect("create folder");
    write_test_wav_i16(&folder.join("first.wav"), &[0, 1024, -2048, 4096]);
    write_test_wav_i16(&folder.join("second.wav"), &[0, 512, -512, 1024]);

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    let mut context = ui::UiUpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::FolderBrowser(
            crate::native_app::test_support::state::FolderBrowserMessage::ActivateFolder(
                folder.display().to_string(),
                Default::default(),
            ),
        ),
        &mut context,
    );
    let warm_ticket = state
        .waveform
        .cache
        .active_folder_warm_delay_task
        .active()
        .expect("folder warm delay");
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::ActiveFolderCacheWarmReady(warm_ticket),
        &mut context,
    );
    let running_ticket = active_folder_cache_warm_ticket(&state).expect("folder warm task");

    state.waveform.current.start_playback(0.0);
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::ActiveFolderCacheWarmFinished(
            active_folder_cache_warm_completion(
                running_ticket,
                source_root.path().display().to_string(),
                Vec::new(),
                false,
            ),
        ),
        &mut context,
    );

    assert!(
        active_folder_cache_warm_ticket(&state).is_none(),
        "completed warm batches must not immediately start another batch during playback"
    );
    assert!(
        state
            .waveform
            .cache
            .active_folder_warm_delay_task
            .active()
            .is_some(),
        "active folder cache warm should wait until playback is idle before resuming"
    );
    assert_eq!(
        state.waveform.cache.active_folder_warm_pending.len(),
        1,
        "only the already-started single-file batch may be drained"
    );
}

#[test]
fn active_folder_cache_warm_generates_playback_ready_cache_for_uncached_file() {
    let config_base = tempfile::tempdir().expect("config base");
    let (_config_lock, _base_guard) =
        set_waveform_test_config_base(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("uncached-warm.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let sample_path = PathBuf::from(sample_path.display().to_string());

    assert!(!crate::native_app::waveform::cached_waveform_file_playback_ready_exists(&sample_path));

    let loaded = crate::native_app::audio::sample_load_actions::warm_active_folder_waveform_cache(
        vec![sample_path.clone()],
        || false,
    );
    crate::native_app::waveform::flush_background_waveform_cache_stores_for_shutdown();

    assert_eq!(loaded.len(), 1);
    assert!(
        crate::native_app::waveform::cached_waveform_file_playback_ready_exists(&sample_path),
        "active folder warm should persist playback readiness for future selection"
    );
}

#[test]
fn summary_only_persisted_cache_is_not_marked_playback_ready_after_restart() {
    let config_base = tempfile::tempdir().expect("config base");
    let (_config_lock, _base_guard) =
        set_waveform_test_config_base(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("summary-only.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let sample_path_string = sample_path.display().to_string();
    let sample_path = PathBuf::from(&sample_path_string);

    let file = crate::native_app::waveform::test_waveform_file_from_mono_samples(
        sample_path.clone(),
        fs::read(&sample_path).expect("read wav").into(),
        vec![0.0, 0.25, -0.25, 0.5, -0.5, 0.125],
    );
    crate::native_app::waveform::store_summary_only_cached_waveform_file_for_tests(&file);

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state.refresh_persisted_waveform_cache_indicators();

    assert!(
        !state
            .waveform
            .cache
            .cached_sample_paths
            .contains(&sample_path_string),
        "summary-only persisted cache must not paint the row as playback-ready"
    );
    assert_eq!(
        state.waveform.cache.warm_pending.iter().collect::<Vec<_>>(),
        vec![&sample_path],
        "summary-only persisted cache should still be warmed in the background"
    );
}

#[test]
fn summary_only_persisted_cache_selection_uses_loading_pipeline_after_restart() {
    let config_base = tempfile::tempdir().expect("config base");
    let (_config_lock, _base_guard) =
        set_waveform_test_config_base(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("summary-only-click.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let sample_path_string = sample_path.display().to_string();
    let sample_path = PathBuf::from(&sample_path_string);

    let file = crate::native_app::waveform::test_waveform_file_from_mono_samples(
        sample_path.clone(),
        fs::read(&sample_path).expect("read wav").into(),
        vec![0.0, 0.25, -0.25, 0.5, -0.5, 0.125],
    );
    crate::native_app::waveform::store_summary_only_cached_waveform_file_for_tests(&file);

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state.refresh_persisted_waveform_cache_indicators();

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
        "summary-only cache selection should not debounce or probe cache metadata on the UI thread"
    );
    assert!(
        active_sample_load_ticket(&state).is_some(),
        "summary-only cache selection should queue foreground loading off the UI thread"
    );
    assert_eq!(
        state.waveform.current.path(),
        PathBuf::new(),
        "selection should wait for worker completion instead of hydrating a partial cache on the UI thread"
    );

    let ticket = active_sample_load_ticket(&state).expect("foreground load queued");
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SampleLoadFinished(
            sample_load_completion(
                ticket,
                sample_path_string,
                crate::native_app::test_support::state::WaveformState::load_persisted_playback_cache(
                    sample_path.clone(),
                ),
                false,
            ),
        ),
        &mut context,
    );

    assert_eq!(state.waveform.current.path(), sample_path);
    assert!(
        state.waveform.current.playback_samples().is_some(),
        "summary-only persisted cache should be upgraded to playback-ready in the foreground worker"
    );
}

#[test]
fn background_warm_upgrades_summary_only_cache_to_playback_ready() {
    let config_base = tempfile::tempdir().expect("config base");
    let (_config_lock, _base_guard) =
        set_waveform_test_config_base(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("summary-only-warm.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let sample_path_string = sample_path.display().to_string();
    let sample_path = PathBuf::from(&sample_path_string);

    let file = crate::native_app::waveform::test_waveform_file_from_mono_samples(
        sample_path.clone(),
        fs::read(&sample_path).expect("read wav").into(),
        vec![0.0, 0.25, -0.25, 0.5, -0.5, 0.125],
    );
    crate::native_app::waveform::store_summary_only_cached_waveform_file_for_tests(&file);

    let result = crate::native_app::audio::sample_load_actions::warm_persisted_waveform_cache(
        vec![sample_path.clone()],
        || false,
    );
    assert_eq!(result.loaded.len(), 1);

    let mut restarted_state = gui_state_for_span_tests();
    restarted_state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    restarted_state.refresh_persisted_waveform_cache_indicators();

    assert!(
        restarted_state
            .waveform
            .cache
            .cached_sample_paths
            .contains(&sample_path_string),
        "background warm should persist playback readiness for future restarts"
    );
}

#[test]
fn normal_sample_load_persists_bright_cache_indicator_before_restart() {
    let config_base = tempfile::tempdir().expect("config base");
    let (_config_lock, _base_guard) =
        set_waveform_test_config_base(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("fresh-cache.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let sample_path = sample_path.display().to_string();

    let _waveform = crate::native_app::test_support::state::WaveformState::load_path(
        sample_path.clone().into(),
    )
    .expect("load sample");

    wait_for_playback_ready_cache(&sample_path);

    let mut restarted_state = gui_state_for_span_tests();
    restarted_state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    restarted_state.refresh_persisted_waveform_cache_indicators();

    assert!(
        restarted_state
            .waveform
            .cache
            .cached_sample_paths
            .contains(&sample_path),
        "freshly loaded cache indicator should survive immediate restart"
    );
}
