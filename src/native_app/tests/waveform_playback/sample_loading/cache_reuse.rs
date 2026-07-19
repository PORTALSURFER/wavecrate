use super::*;

#[test]
fn sample_selection_loads_selected_file_into_waveform() {
    let source_root = tempfile::tempdir().expect("source root");
    let sample = source_root.path().join("selected.wav");
    write_test_wav_i16(&sample, &[0, 1024, -2048, 4096, -1024, 512]);
    let sample_path = sample.display().to_string();
    let sample_name = PathBuf::from(&sample_path)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .expect("sample file name");
    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);

    let mut context = ui::UiUpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SelectSampleWithModifiers {
            path: sample_path.clone(),
            modifiers: Default::default(),
        },
        &mut context,
    );
    run_command_for_tests(&mut state, context.into_command());
    let mut context = ui::UiUpdateContext::default();
    assert_eq!(
        state.waveform.load.label.as_deref(),
        Some(sample_name.as_str())
    );
    assert!(
        state
            .background
            .deferred_sample_load_task
            .active()
            .is_none(),
        "direct selection should not debounce or probe cache metadata on the UI thread"
    );
    assert!(
        active_sample_load_ticket(&state).is_some(),
        "direct selection should immediately queue foreground sample loading"
    );
    let ticket = active_sample_load_ticket(&state).expect("sample load queued");
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SampleLoadFinished(
            sample_load_completion(
                ticket,
                sample_path.clone(),
                crate::native_app::test_support::state::WaveformState::load_path(
                    sample_path.clone().into(),
                ),
                true,
            ),
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
    run_command_for_tests(&mut state, context.into_command());

    assert!(
        state
            .background
            .deferred_sample_load_task
            .active()
            .is_none(),
        "repeat selection should use the memory waveform cache without a deferred worker"
    );
    assert!(
        active_sample_load_ticket(&state).is_none(),
        "repeat selection must not start decode work"
    );
    assert_eq!(state.waveform.current.file_name(), sample_name);
}

#[test]
fn sample_load_marks_new_harvest_file_seen() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let sample = source_root.path().join("harvest-seen.wav");
    write_test_wav_i16(&sample, &[0, 1024, -2048, 4096, -1024, 512]);
    let sample_path = sample.display().to_string();
    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    let (source, relative_path) = state
        .library
        .folder_browser
        .sample_source_for_file_path(&sample)
        .expect("sample should belong to the active source");
    let harvest_key = wavecrate::sample_sources::HarvestFileKey::new(
        wavecrate::sample_sources::SourceId::from_string(source.id.as_str().to_owned()),
        relative_path,
    );

    let mut context = ui::UiUpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SelectSampleWithModifiers {
            path: sample_path.clone(),
            modifiers: Default::default(),
        },
        &mut context,
    );
    run_command_for_tests(&mut state, context.into_command());
    let ticket = active_sample_load_ticket(&state).expect("sample load queued");
    let mut context = ui::UiUpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SampleLoadFinished(
            sample_load_completion(
                ticket,
                sample_path,
                crate::native_app::test_support::state::WaveformState::load_path(sample),
                false,
            ),
        ),
        &mut context,
    );
    run_command_for_tests(&mut state, context.into_command());

    let harvest_record = wavecrate::sample_sources::library::harvest_file(&harvest_key)
        .expect("load harvest file")
        .expect("harvest file should be persisted after load");
    assert_eq!(
        harvest_record.state,
        wavecrate::sample_sources::HarvestState::Seen
    );
    assert!(harvest_record.seen_at.is_some());
    assert!(harvest_record.touched_at.is_none());
}

#[test]
fn failed_sample_load_status_names_and_focuses_sample() {
    let source_root = tempfile::tempdir().expect("source root");
    let first = source_root.path().join("a-selected.wav");
    let failed = source_root.path().join("b-failed.wav");
    write_test_wav_i16(&first, &[0, 1024, -2048, 4096]);
    write_test_wav_i16(&failed, &[0, 512, -512, 1024]);
    let first_path = first.display().to_string();
    let failed_path = failed.display().to_string();

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state.library.folder_browser.select_file(first_path.clone());

    let mut context = ui::UiUpdateContext::default();
    state.load_sample(failed_path.clone(), &mut context);
    run_command_for_tests(&mut state, context.into_command());
    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(first_path.as_str()),
        "plain load requests should not depend on pre-focused browser state"
    );

    let ticket = active_sample_load_ticket(&state).expect("sample load queued");
    let mut context = ui::UiUpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SampleLoadFinished(
            sample_load_completion(
                ticket,
                failed_path.clone(),
                Err(String::from("synthetic decode failed")),
                true,
            ),
        ),
        &mut context,
    );

    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(failed_path.as_str())
    );
    assert!(
        state
            .ui
            .status
            .sample
            .contains("Could not load b-failed.wav"),
        "{}",
        state.ui.status.sample
    );
    assert!(
        state.ui.status.sample.contains("synthetic decode failed"),
        "{}",
        state.ui.status.sample
    );
    assert_eq!(state.waveform.load.label, None);
    let frame = crate::native_app::app_chrome::waveform_panel::waveform_panel(
        crate::native_app::app_chrome::view_models::waveform_panel::WaveformPanelViewModel::from_app_state(&state),
    )
    .view_frame_at_size_with_default_theme(ui::Vector2::new(900.0, 220.0));
    assert!(
        frame
            .paint_plan
            .contains_text("Could not load b-failed.wav"),
        "failed waveform loads should explain the empty waveform panel"
    );
    assert!(
        !frame.paint_plan.contains_text("No sample loaded"),
        "failed waveform loads should not collapse to a silent empty state"
    );
}

#[test]
fn foreground_sample_load_persists_compact_waveform_summary() {
    let config_base = tempfile::tempdir().expect("config base");
    let (_config_lock, _base_guard) =
        set_waveform_test_config_base(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("foreground.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096, -1024, 512]);

    let loaded =
        crate::native_app::test_support::state::WaveformState::load_path_for_foreground_audition(
            sample_path.clone(),
            |_| {},
            || false,
            |_| {},
        )
        .expect("foreground sample load");
    crate::native_app::waveform::flush_background_waveform_cache_stores_for_shutdown();

    assert_eq!(loaded.path(), sample_path);
    assert!(
        crate::native_app::waveform::cached_waveform_file_exists(&sample_path),
        "foreground audition should persist a compact visual summary"
    );
    assert!(
        !crate::native_app::waveform::cached_waveform_file_playback_ready_exists(&sample_path),
        "foreground audition must not persist decoded playback"
    );
}

#[test]
fn foreground_sample_load_reuses_persisted_summary_with_original_file_playback() {
    let config_base = tempfile::tempdir().expect("config base");
    let (_config_lock, _base_guard) =
        set_waveform_test_config_base(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("cached-foreground.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096, -1024, 512]);

    let cached =
        crate::native_app::test_support::state::WaveformState::load_path(sample_path.clone())
            .expect("cache seed loads");
    assert!(cached.playback_samples().is_none());
    crate::native_app::waveform::flush_background_waveform_cache_stores_for_shutdown();
    let file = cached.file();
    crate::native_app::waveform::store_cached_waveform_file_for_tests(&file);
    assert!(crate::native_app::waveform::cached_waveform_file_exists(
        &sample_path
    ));

    let loaded =
        crate::native_app::test_support::state::WaveformState::load_path_for_foreground_audition(
            sample_path.clone(),
            |_| {},
            || false,
            |_| {},
        )
        .expect("foreground sample load");

    assert_eq!(loaded.path(), sample_path);
    assert!(
        loaded.audio_bytes().is_empty(),
        "foreground audition should hydrate the compact summary without rereading source bytes"
    );
    assert!(
        loaded.playback_cache_file().is_none(),
        "foreground audition must not use a persisted PCM sidecar"
    );
    assert_eq!(
        loaded.playback_source_file().as_deref(),
        Some(sample_path.as_path())
    );
}

#[test]
fn instant_audition_display_load_reuses_summary_without_decoded_playback() {
    let config_base = tempfile::tempdir().expect("config base");
    let (_config_lock, _base_guard) =
        set_waveform_test_config_base(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("display-after-audition.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096, -1024, 512]);

    let cached =
        crate::native_app::test_support::state::WaveformState::load_path(sample_path.clone())
            .expect("cache seed loads");
    crate::native_app::waveform::flush_background_waveform_cache_stores_for_shutdown();
    let file = cached.file();
    crate::native_app::waveform::store_cached_waveform_file_for_tests(&file);
    assert!(crate::native_app::waveform::cached_waveform_file_exists(
        &sample_path
    ));

    let displayed =
        crate::native_app::test_support::state::WaveformState::load_path_for_instant_audition_display(
            sample_path.clone(),
            |_| {},
            || false,
        )
        .expect("display waveform loads");

    assert_eq!(displayed.path(), sample_path);
    assert!(
        displayed.audio_bytes().is_empty(),
        "post-audition display load should reuse persisted summary metadata"
    );
    assert!(
        displayed.playback_samples().is_none(),
        "post-audition display load should not rebuild decoded playback samples"
    );
}

#[test]
fn large_foreground_sample_load_reuses_file_backed_summary_cache() {
    let config_base = tempfile::tempdir().expect("config base");
    let (_config_lock, _base_guard) =
        set_waveform_test_config_base(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("large-foreground.wav");
    write_sparse_test_wav_i16(&sample_path, 1, 700);

    let loaded =
        crate::native_app::test_support::state::WaveformState::load_path_for_foreground_audition(
            sample_path.clone(),
            |_| {},
            || false,
            |_| {},
        )
        .expect("large foreground sample load");
    crate::native_app::waveform::flush_background_waveform_cache_stores_for_shutdown();

    assert_eq!(loaded.path(), sample_path);
    assert!(loaded.audio_bytes().is_empty());
    assert!(loaded.playback_samples().is_none());
    assert_eq!(
        loaded.playback_source_file().as_deref(),
        Some(sample_path.as_path())
    );
    assert!(crate::native_app::waveform::cached_waveform_file_exists(
        &sample_path
    ));
    assert!(
        !crate::native_app::waveform::cached_waveform_file_playback_ready_exists(&sample_path),
        "large foreground load should persist a summary cache without a full playback sidecar"
    );

    let reloaded =
        crate::native_app::test_support::state::WaveformState::load_path_for_foreground_audition(
            sample_path.clone(),
            |_| {},
            || false,
            |_| {},
        )
        .expect("large foreground summary cache reload");

    assert_eq!(reloaded.path(), sample_path);
    assert!(reloaded.audio_bytes().is_empty());
    assert!(reloaded.playback_samples().is_none());
    assert_eq!(
        reloaded.playback_source_file().as_deref(),
        Some(sample_path.as_path())
    );
}

#[test]
fn large_wav_instant_audition_descriptor_reads_source_header() {
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("large-header.wav");
    write_sparse_test_wav_i16(&sample_path, 2, 700);

    let descriptor = crate::native_app::waveform::file_backed_wav_playback_descriptor(&sample_path)
        .expect("large WAV descriptor");

    assert_eq!(descriptor.path, sample_path);
    assert_eq!(descriptor.sample_rate, 48_000);
    assert_eq!(descriptor.channels, 2);
    assert_eq!(descriptor.frames, 700);
    assert!((descriptor.duration - (700.0 / 48_000.0)).abs() < f32::EPSILON);
}

#[test]
fn foreground_sample_load_stays_file_backed_after_loop_decode() {
    let config_base = tempfile::tempdir().expect("config base");
    let (_config_lock, _base_guard) =
        set_waveform_test_config_base(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root
        .path()
        .join("large-with-legacy-playback-cache.wav");
    write_sparse_test_wav_i16(&sample_path, 1, 700);

    let decoded = crate::native_app::test_support::state::WaveformState::load_path_for_looped_foreground_audition(
        sample_path.clone(),
        |_| {},
        || false,
        |_| {},
    )
    .expect("decoded foreground load");
    crate::native_app::waveform::flush_background_waveform_cache_stores_for_shutdown();
    assert!(decoded.playback_samples().is_some());
    assert!(
        !crate::native_app::waveform::cached_waveform_file_playback_ready_exists(&sample_path),
        "loop decoding must remain memory-only"
    );

    let reloaded =
        crate::native_app::test_support::state::WaveformState::load_path_for_foreground_audition(
            sample_path.clone(),
            |_| {},
            || false,
            |_| {},
        )
        .expect("foreground audition reload");

    assert_eq!(reloaded.path(), sample_path);
    assert!(
        reloaded.audio_bytes().is_empty(),
        "large non-looped foreground navigation should stay file-backed instead of deserializing the legacy playback cache"
    );
    assert!(reloaded.playback_samples().is_none());
}

#[test]
fn large_navigation_sample_starts_source_file_audition_before_background_waveform_load() {
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("large-navigation.wav");
    write_sparse_test_wav_i16(&sample_path, 1, 700);
    let sample_path_string = sample_path.display().to_string();

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    let playback_runtime_installed = install_playback_runtime_for_tests(&mut state);

    let mut context = ui::UiUpdateContext::default();
    state.load_navigation_sample_validated(
        sample_path_string.clone(),
        &mut context,
        std::time::Instant::now(),
    );
    let command = context.into_command();

    if playback_runtime_installed {
        assert_eq!(
            command.business_task_priority("gui-sample-load"),
            Some(ui::TaskPriority::Background),
            "waveform loading should move behind immediate source-file playback"
        );
        let session = state
            .audio
            .sample_playback_session
            .as_ref()
            .expect("source-file navigation should create a playback session");
        assert_eq!(session.request.path, sample_path_string);
        assert_eq!(
            session.request.visibility,
            crate::native_app::app::SamplePlaybackVisibility::Transient
        );
        assert_eq!(
            session.request.stream_policy,
            wavecrate::audio::PlaybackRuntimeStreamPolicy::transient_navigation()
        );
    } else {
        assert_eq!(
            command.business_task_priority("gui-sample-load"),
            Some(ui::TaskPriority::Interactive),
            "without an installed runtime, the selected sample still needs foreground loading"
        );
    }
}

#[test]
fn navigation_sample_skips_persistent_sidecar_lookup_when_source_file_can_play() {
    let config_base = tempfile::tempdir().expect("config base");
    let (_config_lock, _base_guard) =
        set_waveform_test_config_base(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("large-navigation-legacy.wav");
    write_sparse_test_wav_i16(&sample_path, 1, 700);
    let sample_path_string = sample_path.display().to_string();

    let decoded = crate::native_app::test_support::state::WaveformState::load_path_for_looped_foreground_audition(
        sample_path.clone(),
        |_| {},
        || false,
        |_| {},
    )
    .expect("decoded foreground load");
    crate::native_app::waveform::flush_background_waveform_cache_stores_for_shutdown();
    assert!(decoded.playback_samples().is_some());
    assert!(
        !crate::native_app::waveform::cached_waveform_file_playback_ready_exists(&sample_path),
        "loop decoding must not seed a persistent playback sidecar"
    );

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    let playback_runtime_installed = install_playback_runtime_for_tests(&mut state);

    let mut context = ui::UiUpdateContext::default();
    state.load_navigation_sample_validated(
        sample_path_string.clone(),
        &mut context,
        std::time::Instant::now(),
    );
    let command = context.into_command();

    assert!(
        !state
            .waveform
            .cache
            .instant_audition_descriptors
            .contains_key(&sample_path),
        "large source-file playback should not synchronously read and retain a sidecar descriptor"
    );
    if playback_runtime_installed {
        assert_eq!(
            command.business_task_priority("gui-sample-load"),
            Some(ui::TaskPriority::Background),
            "visual waveform loading should move behind source-file playback"
        );
        assert_eq!(
            state.audio.active_sample_playback_path(),
            Some(sample_path_string.as_str())
        );
    } else {
        assert_eq!(
            command.business_task_priority("gui-sample-load"),
            Some(ui::TaskPriority::Interactive)
        );
    }
}

#[test]
fn looped_foreground_sample_load_bypasses_file_backed_summary_cache() {
    let config_base = tempfile::tempdir().expect("config base");
    let (_config_lock, _base_guard) =
        set_waveform_test_config_base(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("large-loop.wav");
    write_sparse_test_wav_i16(&sample_path, 1, 700);

    let summary =
        crate::native_app::test_support::state::WaveformState::load_path_for_foreground_audition(
            sample_path.clone(),
            |_| {},
            || false,
            |_| {},
        )
        .expect("large foreground summary load");
    crate::native_app::waveform::flush_background_waveform_cache_stores_for_shutdown();
    assert_eq!(
        summary.playback_source_file().as_deref(),
        Some(sample_path.as_path())
    );
    assert!(
        !crate::native_app::waveform::cached_waveform_file_playback_ready_exists(&sample_path),
        "summary-only cache seed should not create a playback-ready sidecar"
    );

    let playback_ready = std::cell::Cell::new(false);
    let loaded = crate::native_app::test_support::state::WaveformState::load_path_for_looped_foreground_audition(
        sample_path.clone(),
        |_| {},
        || false,
        |ready| {
            assert_eq!(ready.path, sample_path);
            assert!(!ready.playback_samples.is_empty());
            playback_ready.set(true);
        },
    )
    .expect("looped foreground load");
    crate::native_app::waveform::flush_background_waveform_cache_stores_for_shutdown();

    assert!(
        playback_ready.get(),
        "looped foreground load should surface decoded playback samples before completion"
    );
    assert_eq!(loaded.path(), sample_path);
    assert!(!loaded.audio_bytes().is_empty());
    assert!(loaded.playback_samples().is_some());
    assert_eq!(loaded.playback_source_file(), None);
    assert!(
        !crate::native_app::waveform::cached_waveform_file_playback_ready_exists(&sample_path),
        "decoded loop playback must remain memory-only"
    );
}

#[test]
fn loop_tagged_selection_skips_summary_only_memory_cache() {
    let config_base = tempfile::tempdir().expect("config base");
    let (_config_lock, _base_guard) =
        set_waveform_test_config_base(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("memory-summary-loop.wav");
    write_sparse_test_wav_i16(&sample_path, 1, 700);
    let sample_path_string = sample_path.display().to_string();

    let summary =
        crate::native_app::test_support::state::WaveformState::load_path_for_foreground_audition(
            sample_path.clone(),
            |_| {},
            || false,
            |_| {},
        )
        .expect("summary waveform loads");
    assert_eq!(
        summary.playback_source_file().as_deref(),
        Some(sample_path.as_path())
    );

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state
        .metadata
        .tags_by_file
        .insert(sample_path_string.clone(), vec![String::from("loop")]);
    state.remember_waveform(&summary);

    let mut context = ui::UiUpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SelectSampleWithModifiers {
            path: sample_path_string.clone(),
            modifiers: Default::default(),
        },
        &mut context,
    );
    run_command_for_tests(&mut state, context.into_command());

    assert!(
        active_sample_load_ticket(&state).is_some(),
        "loop-tagged selection should queue a decoded foreground load instead of playing a summary-only cache entry"
    );
    assert_eq!(
        state.waveform.load.label.as_deref(),
        Some("memory-summary-loop.wav")
    );
    assert_ne!(
        state.waveform.current.path(),
        sample_path,
        "summary-only cache must not replace the waveform for looped autoplay"
    );
}

#[test]
fn loop_toggle_selection_skips_summary_only_memory_cache_for_untagged_sample() {
    let config_base = tempfile::tempdir().expect("config base");
    let (_config_lock, _base_guard) =
        set_waveform_test_config_base(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("memory-summary-manual-loop.wav");
    write_sparse_test_wav_i16(&sample_path, 1, 700);
    let sample_path_string = sample_path.display().to_string();

    let summary =
        crate::native_app::test_support::state::WaveformState::load_path_for_foreground_audition(
            sample_path.clone(),
            |_| {},
            || false,
            |_| {},
        )
        .expect("summary waveform loads");
    assert_eq!(
        summary.playback_source_file().as_deref(),
        Some(sample_path.as_path())
    );

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state.audio.loop_playback = true;
    state.remember_waveform(&summary);

    let mut context = ui::UiUpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SelectSampleWithModifiers {
            path: sample_path_string.clone(),
            modifiers: Default::default(),
        },
        &mut context,
    );
    run_command_for_tests(&mut state, context.into_command());

    assert!(
        active_sample_load_ticket(&state).is_some(),
        "manual loop playback should queue a decoded foreground load instead of playing a summary-only cache entry"
    );
    assert_eq!(
        state.waveform.load.label.as_deref(),
        Some("memory-summary-manual-loop.wav")
    );
    assert_ne!(
        state.waveform.current.path(),
        sample_path,
        "summary-only cache must not replace the waveform when manual loop playback is enabled"
    );
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
    run_command_for_tests(&mut state, context.into_command());

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
        active_sample_load_ticket(&state).is_none(),
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
fn long_display_after_instant_audition_completion_shows_nonblank_waveform_panel() {
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("long-summary-miss.wav");
    write_pulsed_long_test_wav_i16(&sample_path, 1, 700);
    let sample_path_string = sample_path.display().to_string();

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);

    let mut context = ui::UiUpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SelectSampleWithModifiers {
            path: sample_path_string.clone(),
            modifiers: Default::default(),
        },
        &mut context,
    );
    run_command_for_tests(&mut state, context.into_command());
    let ticket = active_sample_load_ticket(&state).expect("long sample load queued");

    let loaded =
        crate::native_app::test_support::state::WaveformState::load_path_for_instant_audition_display(
            sample_path.clone(),
            |_| {},
            || false,
        )
        .expect("long display summary load");
    assert_eq!(
        loaded.playback_source_file().as_deref(),
        Some(sample_path.as_path()),
        "long non-looped samples should be displayable from a file-backed summary"
    );
    assert!(loaded.playback_samples().is_none());
    assert!(
        loaded.signal_summary_peak_for_tests() > 0.0,
        "post-audition display loads must retain drawable waveform signal data"
    );

    let mut context = ui::UiUpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SampleLoadFinished(
            sample_load_completion(ticket, sample_path_string.clone(), Ok(loaded), true),
        ),
        &mut context,
    );

    assert_eq!(state.waveform.current.path(), sample_path);
    assert!(state.waveform.current.has_loaded_sample());
    assert!(
        state.waveform.current.signal_summary_peak_for_tests() > 0.0,
        "loaded long sample should not leave the retained waveform surface blank"
    );
    assert_eq!(state.waveform.load.label, None);
    let frame = crate::native_app::app_chrome::waveform_panel::waveform_panel(
        crate::native_app::app_chrome::view_models::waveform_panel::WaveformPanelViewModel::from_app_state(&state),
    )
    .view_frame_at_size_with_default_theme(ui::Vector2::new(900.0, 220.0));
    assert!(
        frame
            .paint_plan
            .text_runs()
            .any(|run| run.text.starts_with("long-summary-miss.wav |")),
        "loaded long samples should identify the waveform instead of leaving a blank panel"
    );
    assert!(!frame.paint_plan.contains_text("No sample loaded"));
}

#[test]
fn long_summary_memory_cache_hit_shows_loaded_waveform_without_worker() {
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("long-summary-hit.wav");
    write_pulsed_long_test_wav_i16(&sample_path, 1, 700);
    let sample_path_string = sample_path.display().to_string();

    let loaded =
        crate::native_app::test_support::state::WaveformState::load_path_for_foreground_audition(
            sample_path.clone(),
            |_| {},
            || false,
            |_| {},
        )
        .expect("long foreground summary load");
    assert_eq!(
        loaded.playback_source_file().as_deref(),
        Some(sample_path.as_path())
    );
    assert!(
        loaded.signal_summary_peak_for_tests() > 0.0,
        "summary cache seed should contain drawable waveform signal data"
    );

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state.remember_waveform(&loaded);

    let mut context = ui::UiUpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SelectSampleWithModifiers {
            path: sample_path_string.clone(),
            modifiers: Default::default(),
        },
        &mut context,
    );
    run_command_for_tests(&mut state, context.into_command());

    assert_eq!(state.waveform.current.path(), sample_path);
    assert!(state.waveform.current.has_loaded_sample());
    assert!(
        state.waveform.current.signal_summary_peak_for_tests() > 0.0,
        "memory-cached long summary should keep drawable waveform signal data"
    );
    assert_eq!(state.waveform.load.label, None);
    assert!(
        active_sample_load_ticket(&state).is_none(),
        "long summary memory cache hits should not queue foreground decode"
    );
    let frame = crate::native_app::app_chrome::waveform_panel::waveform_panel(
        crate::native_app::app_chrome::view_models::waveform_panel::WaveformPanelViewModel::from_app_state(&state),
    )
    .view_frame_at_size_with_default_theme(ui::Vector2::new(900.0, 220.0));
    assert!(
        frame
            .paint_plan
            .text_runs()
            .any(|run| run.text.starts_with("long-summary-hit.wav |"))
    );
    assert!(!frame.paint_plan.contains_text("No sample loaded"));
}

fn write_pulsed_long_test_wav_i16(path: &std::path::Path, channels: u16, frames: usize) {
    let channels = channels.max(1);
    let spec = hound::WavSpec {
        channels,
        sample_rate: 48_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).expect("create wav");
    for frame in 0..frames {
        let pulse = if frame % 97 == 0 {
            10_000
        } else if frame % 53 == 0 {
            -8_000
        } else {
            0
        };
        for _ in 0..channels {
            writer.write_sample::<i16>(pulse).expect("write sample");
        }
    }
    writer.finalize().expect("finalize wav");
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
    run_command_for_tests(&mut state, context.into_command());

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
        active_sample_load_ticket(&state).is_none(),
        "memory-cached non-autoplay load should not queue decode work"
    );
    assert!(
        state.ui.status.sample.contains("Loaded cached.wav"),
        "cached non-autoplay load should update status, got {}",
        state.ui.status.sample
    );
}
