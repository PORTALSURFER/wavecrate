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
        state
            .waveform
            .cache
            .active_folder_warm_task
            .active()
            .is_none(),
        "active folder cache warm must not start during folder activation"
    );
    assert_eq!(state.waveform.cache.active_folder_warm_pending.len(), 2);
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
    assert!(
        state
            .waveform
            .cache
            .active_folder_warm_task
            .active()
            .is_some()
    );

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::FolderBrowser(
            crate::native_app::test_support::state::FolderBrowserMessage::ActivateFolder(
                second_folder.display().to_string(),
            ),
        ),
        &mut context,
    );

    assert!(
        state
            .waveform
            .cache
            .active_folder_warm_task
            .active()
            .is_none(),
        "changing folders should cancel the active warm worker"
    );
    let second_folder_id = second_folder.display().to_string();
    assert_eq!(
        state.waveform.cache.active_folder_warm_folder_id.as_deref(),
        Some(second_folder_id.as_str())
    );
    assert_eq!(state.waveform.cache.active_folder_warm_pending.len(), 1);
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
            .is_some(),
        "summary-only cache selection should not synchronously decode long playback samples"
    );
    assert_eq!(
        state.waveform.current.path(),
        PathBuf::from("synthetic-waveform"),
        "selection should wait for the normal loading pipeline instead of hydrating a partial cache"
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

    let result =
        crate::native_app::audio::sample_load_actions::warm_persisted_waveform_cache(vec![
            sample_path.clone(),
        ]);
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
