use super::*;

#[test]
fn looped_waveform_click_resolves_to_full_sample_without_playmark() {
    let mut state = gui_state_for_span_tests();
    state.loop_playback = true;

    let span = state.resolve_playback_span(0.45, 1.0, None);

    assert_eq!(span.start_ratio, 0.0);
    assert_eq!(span.end_ratio, 1.0);
    assert_eq!(span.offset_ratio, 0.45);
}

#[test]
fn looped_waveform_click_resolves_to_playmark_span_when_selected() {
    let mut state = gui_state_for_span_tests();
    state.loop_playback = true;
    state
        .waveform
        .apply_interaction(WaveformInteraction::BeginSelection {
            kind: WaveformSelectionKind::Play,
            visible_ratio: 0.25,
        });
    state
        .waveform
        .apply_interaction(WaveformInteraction::UpdateSelection {
            visible_ratio: 0.60,
        });
    state
        .waveform
        .apply_interaction(WaveformInteraction::FinishSelection {
            visible_ratio: 0.60,
        });

    let inside_span = state.resolve_playback_span(0.45, 1.0, None);
    assert_eq!(inside_span.start_ratio, 0.25);
    assert_eq!(inside_span.end_ratio, 0.60);
    assert_eq!(inside_span.offset_ratio, 0.45);

    let outside_span = state.resolve_playback_span(0.85, 1.0, None);
    assert_eq!(outside_span.start_ratio, 0.25);
    assert_eq!(outside_span.end_ratio, 0.60);
    assert_eq!(outside_span.offset_ratio, 0.25);
}

#[test]
fn random_audition_span_uses_fixed_window_inside_long_sample() {
    let (start, end) = super::super::playback::random_audition_span_for_unit(20.0, 0.5);

    assert!((start - 0.4).abs() < 0.001, "start was {start}");
    assert!((end - 0.6).abs() < 0.001, "end was {end}");
}

#[test]
fn random_audition_span_plays_whole_short_sample() {
    assert_eq!(
        super::super::playback::random_audition_span_for_unit(2.0, 0.75),
        (0.0, 1.0)
    );
}

#[test]
fn random_audition_prefers_marked_play_ranges_and_selects_the_chosen_range() {
    let mut state = gui_state_for_span_tests();

    for (start, end) in [(0.10, 0.20), (0.55, 0.70)] {
        state
            .waveform
            .apply_interaction(WaveformInteraction::BeginSelection {
                kind: WaveformSelectionKind::Play,
                visible_ratio: start,
            });
        state
            .waveform
            .apply_interaction(WaveformInteraction::UpdateSelection { visible_ratio: end });
        state
            .waveform
            .apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: end });
    }

    let span = state.random_audition_span_for_loaded_waveform(0.75);

    assert_eq!(
        span.source,
        super::super::playback::RandomAuditionSource::MarkedRange
    );
    assert!(
        (span.start - 0.55).abs() < 0.001,
        "start was {}",
        span.start
    );
    assert!((span.end - 0.70).abs() < 0.001, "end was {}", span.end);
    assert_eq!(
        state.waveform.play_selection(),
        Some(wavecrate::selection::SelectionRange::new(0.55, 0.70))
    );
}

#[test]
fn random_audition_is_one_shot_even_when_loop_is_enabled() {
    let Ok(player) = wavecrate::audio::AudioPlayer::new() else {
        return;
    };
    let mut state = gui_state_for_span_tests();
    state.audio_player = Some(player);
    state.loop_playback = true;

    let mut context = ui::UpdateContext::default();
    state.play_random_sample_range_with_unit(0.5, &mut context);

    assert!(!state.loop_playback);
    assert!(state.waveform.is_playing());
    assert_eq!(state.current_playback_span, Some((0.0, 1.0)));
    assert!(
        state
            .audio_player
            .as_ref()
            .is_some_and(|player| !player.is_looping())
    );
}

#[test]
fn random_audition_for_unloaded_selection_resumes_after_sample_load() {
    let (mut state, _source_root, selected_file) = gui_state_with_temp_sample("random-load.wav");
    let path = PathBuf::from(&selected_file);
    write_test_wav_i16(&path, &[0, 1024, -2048, 4096, -1024, 512]);
    state.waveform = super::super::WaveformState::empty();
    state.loop_playback = true;
    assert!(!state.waveform.has_loaded_sample());

    let mut context = ui::UpdateContext::default();
    state.play_random_sample_range_with_unit(0.5, &mut context);

    assert!(matches!(
        state.pending_sample_playback,
        Some(super::super::PendingSamplePlayback::RandomAudition { unit })
            if (unit - 0.5).abs() < f32::EPSILON
    ));

    start_deferred_sample_load_for_tests(&mut state, selected_file.clone(), false, &mut context);
    let ticket = state.sample_load_task.active().expect("sample load queued");
    state.apply_message(
        super::super::GuiMessage::SampleLoadFinished(ui::TaskCompletion {
            ticket,
            output: super::super::SampleLoadResult {
                path: selected_file.clone(),
                result: super::super::WaveformState::load_path(path),
                autoplay: false,
            },
        }),
        &mut context,
    );

    assert_eq!(state.pending_sample_playback, None);
    assert!(
        state.pending_playback_start.is_some(),
        "random audition should request playback even when the audio device is still opening"
    );
    assert!(
        !state.loop_playback,
        "random audition should remain one-shot after the selected sample loads"
    );
    assert!(
        state.sample_status.contains("Playback unavailable")
            || state.sample_status.contains("Random audition"),
        "random load completion should route through random playback, got {}",
        state.sample_status
    );
}

#[test]
fn normalize_wav_file_in_place_scales_loaded_sample_peak() {
    let root = std::env::temp_dir().join(format!(
        "wavecrate-default-gui-normalize-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("create temp root");
    let path = root.join("quiet.wav");
    write_test_wav_i16(&path, &[0, 1024, -2048, 4096]);

    super::super::normalize_wav_file_in_place(&path).expect("normalize wav");

    let samples = read_test_wav_f32(&path);
    let peak = samples
        .iter()
        .copied()
        .map(f32::abs)
        .fold(0.0_f32, f32::max);
    assert!((peak - 1.0).abs() < 0.000_001, "peak was {peak}");
    assert!(samples.iter().all(|sample| sample.is_finite()));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn normalize_selected_samples_queues_worker_without_rewriting_on_ui_thread() {
    let (mut state, _source_root, selected_file) = gui_state_with_temp_sample("quiet.wav");
    let path = PathBuf::from(&selected_file);
    write_test_wav_i16(&path, &[0, 1024, -2048, 4096]);
    let before = fs::read(&path).expect("read wav before normalization");

    let mut context = ui::UpdateContext::default();
    state.apply_message(
        super::super::GuiMessage::NormalizeSelectedSamples,
        &mut context,
    );

    assert_eq!(
        fs::read(&path).expect("read wav after queue"),
        before,
        "normalization must not rewrite the selected sample on the UI thread"
    );
    let progress = state
        .normalization_progress
        .as_ref()
        .expect("normalization progress should be visible after queueing");
    assert_eq!(progress.completed, 0);
    assert_eq!(progress.total, 1);
    assert_eq!(progress.detail, "Queued");
    assert!(state.sample_status.contains("Normalizing 1 sample"));
}

#[test]
fn sample_selection_loads_selected_file_into_waveform() {
    let mut state = GuiAppState {
        folder_panel: ui::PanelResizeState::new(DEFAULT_FOLDER_WIDTH),
        folder_browser: super::super::FolderBrowserState::load_default(),
        waveform: super::super::WaveformState::synthetic_for_tests(),
        sample_status: String::new(),
        worker_sender: mpsc::channel().0,
        worker_receiver: None,
        next_task_id: 1,
        deferred_sample_load_task: ui::LatestTask::new(),
        sample_load_task: ui::LatestTask::new(),
        sample_load_cancel: None,
        audio_open_task: ui::LatestTask::new(),
        audio_open_results: Default::default(),
        folder_progress: None,
        normalization_progress: None,
        progress_tick: 0.0,
        frame_cadence: ui::FrameCadenceMonitor::new(),
        waveform_loading_progress: 0.0,
        waveform_loading_target_progress: 0.0,
        audio_player: None,
        loop_playback: false,
        volume: super::super::DEFAULT_VOLUME,
        volume_persist_deadline: None,
        audio_output_config: super::super::AudioOutputConfig::default(),
        audio_output_resolved: None,
        audio_hosts: Vec::new(),
        audio_devices: Vec::new(),
        audio_sample_rates: Vec::new(),
        persisted_settings: super::super::AppSettingsCore::default(),
        audio_settings_open: false,
        app_settings_tab: Default::default(),
        audio_settings_dropdown: ui::ExclusiveOpen::new(),
        job_details_open: false,
        transaction_list_open: false,
        transaction_history: Default::default(),
        transaction_restoring: false,
        context_menu: None,
        waveform_loading_label: None,
        audio_settings_error: None,
        current_playback_span: None,
        pending_playback_start: None,
        pending_sample_playback: None,
        native_file_drop_hover: None,
        pending_internal_file_drag_paths: Default::default(),
        metadata_tag_draft: String::new(),
        metadata_tag_tokens: Vec::new(),
        metadata_tag_input_mode: Default::default(),
        pending_metadata_tag_completion_query: None,
        metadata_tag_completion_cycle: ui::CyclicListSelectionCycle::new(),
        metadata_tag_dictionary: Default::default(),
        metadata_tag_library_open: false,
        metadata_tag_drag: None,
        metadata_tag_drop_hover: None,
        selected_metadata_tag: None,
        collapsed_metadata_tag_categories: Default::default(),
        metadata_tags_by_file: HashMap::new(),
        sample_name_view_mode: super::super::SampleNameViewMode::DiskFilename,
        startup_source_scan_pending: false,
        startup_auto_load_pending: false,
        waveform_cache: HashMap::new(),
        waveform_cache_order: Default::default(),
        waveform_cache_bytes: 0,
        waveform_cache_warm_pending: Default::default(),
        waveform_cache_warm_task: ui::LatestTask::new(),
        waveform_cache_warm_results: Default::default(),
        cached_sample_paths: Default::default(),
    };
    let sample_path = first_visible_asset_file_path(&state.folder_browser);
    let sample_name = PathBuf::from(&sample_path)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .expect("sample file name");

    let mut context = ui::UpdateContext::default();
    state.apply_message(
        super::super::GuiMessage::SelectSampleWithModifiers {
            path: sample_path.clone(),
            modifiers: Default::default(),
        },
        &mut context,
    );
    assert_eq!(
        state.waveform_loading_label.as_deref(),
        Some(sample_name.as_str())
    );
    assert!(
        state.deferred_sample_load_task.active().is_some(),
        "selection should debounce uncached sample loading before queueing decode work"
    );
    start_deferred_sample_load_for_tests(&mut state, sample_path.clone(), true, &mut context);
    let ticket = state.sample_load_task.active().expect("sample load queued");
    state.apply_message(
        super::super::GuiMessage::SampleLoadFinished(ui::TaskCompletion {
            ticket,
            output: super::super::SampleLoadResult {
                path: sample_path.clone(),
                result: super::super::WaveformState::load_path(sample_path.clone().into()),
                autoplay: true,
            },
        }),
        &mut context,
    );

    assert_eq!(
        state.folder_browser.selected_file_id(),
        Some(sample_path.as_str())
    );
    assert_eq!(state.waveform.file_name(), sample_name);
    assert_eq!(state.waveform_loading_label, None);
    assert!(state.waveform.frames() > 0);
    assert!(state.sample_status.contains(&sample_name));
    assert!(state.cached_sample_paths.contains(&sample_path));

    state.apply_message(
        super::super::GuiMessage::SelectSampleWithModifiers {
            path: sample_path.clone(),
            modifiers: Default::default(),
        },
        &mut context,
    );

    assert!(
        state.deferred_sample_load_task.active().is_some(),
        "repeat selection should still defer loading work instead of touching cache on the UI thread"
    );
    assert!(
        state.sample_load_task.active().is_none(),
        "repeat selection must not synchronously start decode work"
    );
    assert_eq!(state.waveform.file_name(), sample_name);
}

#[test]
fn repeat_sample_selection_uses_memory_waveform_cache_without_worker() {
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("resident.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let sample_path_string = sample_path.display().to_string();

    let mut state = gui_state_for_span_tests();
    state.folder_browser = super::super::FolderBrowserState::from_sample_sources(&[
        wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
    ]);
    let loaded = super::super::WaveformState::load_path(sample_path.clone()).expect("sample loads");
    state.remember_waveform(&loaded);
    state.waveform = super::super::WaveformState::synthetic_for_tests();
    state.waveform_loading_label = Some(String::from("previous.wav"));
    state.waveform_loading_progress = 0.42;
    state.waveform_loading_target_progress = 0.84;

    let mut context = ui::UpdateContext::default();
    state.apply_message(
        super::super::GuiMessage::SelectSampleWithModifiers {
            path: sample_path_string.clone(),
            modifiers: Default::default(),
        },
        &mut context,
    );

    assert_eq!(state.waveform.path(), sample_path);
    assert_eq!(state.waveform_loading_label, None);
    assert_eq!(state.waveform_loading_progress, 0.0);
    assert_eq!(state.waveform_loading_target_progress, 0.0);
    assert!(
        state.deferred_sample_load_task.active().is_none(),
        "memory-cached repeat selection should not debounce a reload"
    );
    assert!(
        state.sample_load_task.active().is_none(),
        "memory-cached repeat selection should not queue decode work"
    );
    assert!(
        state.sample_status.contains("resident.wav"),
        "cached selection should update the visible status, got {}",
        state.sample_status
    );
    assert!(state.cached_sample_paths.contains(&sample_path_string));
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
    state.folder_browser = super::super::FolderBrowserState::from_sample_sources(&[
        wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
    ]);

    let cached = super::super::WaveformState::load_path(cached_path.clone()).expect("sample loads");
    state.remember_waveform(&cached);

    state.waveform =
        super::super::WaveformState::load_path(current_path).expect("current sample loads");
    state.waveform.start_playback(0.25);
    state.current_playback_span = Some((0.25, 1.0));

    let mut context = ui::UpdateContext::default();
    state.load_sample_without_autoplay(cached_path_string, &mut context);

    assert_eq!(state.waveform.path(), cached_path);
    assert!(!state.waveform.is_playing());
    assert_eq!(state.current_playback_span, None);
    assert!(
        state.deferred_sample_load_task.active().is_none(),
        "memory-cached non-autoplay load should not debounce a reload"
    );
    assert!(
        state.sample_load_task.active().is_none(),
        "memory-cached non-autoplay load should not queue decode work"
    );
    assert!(
        state.sample_status.contains("Loaded cached.wav"),
        "cached non-autoplay load should update status, got {}",
        state.sample_status
    );
}

#[test]
fn keyboard_navigation_defers_sample_loading_until_navigation_settles() {
    let source_root = tempfile::tempdir().expect("source root");
    for name in ["a.wav", "b.wav", "c.wav"] {
        fs::write(source_root.path().join(name), []).expect("sample file");
    }

    let mut state = gui_state_for_span_tests();
    state.folder_browser = super::super::FolderBrowserState::from_sample_sources(&[
        wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
    ]);
    let files = state.folder_browser.selected_audio_files();
    assert!(files.len() >= 3, "expected three visible samples");
    let first = files[0].id.clone();
    let second = files[1].id.clone();
    let third = files[2].id.clone();
    state.folder_browser.select_file(first);

    let mut context = ui::UpdateContext::default();
    state.apply_message(
        super::super::GuiMessage::NavigateBrowser {
            delta: 1,
            extend: false,
        },
        &mut context,
    );

    assert_eq!(
        state.folder_browser.selected_file_id(),
        Some(second.as_str())
    );
    assert!(
        state.deferred_sample_load_task.active().is_some(),
        "keyboard navigation should queue only a deferred latest load"
    );
    assert!(
        state.sample_load_task.active().is_none(),
        "keyboard navigation must not synchronously start decode work"
    );
    assert_eq!(
        state.waveform_loading_label, None,
        "keyboard navigation should not enter the loading UI until the deferred load fires"
    );
    let stale_ticket = state
        .deferred_sample_load_task
        .active()
        .expect("deferred navigation load ticket");

    state.apply_message(
        super::super::GuiMessage::NavigateBrowser {
            delta: 1,
            extend: false,
        },
        &mut context,
    );

    assert_eq!(
        state.folder_browser.selected_file_id(),
        Some(third.as_str())
    );
    assert!(state.deferred_sample_load_task.active().is_some());
    assert!(state.sample_load_task.active().is_none());

    state.apply_message(
        super::super::GuiMessage::DeferredSampleLoad {
            ticket: stale_ticket,
            path: second,
            autoplay: true,
            check_cache: false,
        },
        &mut context,
    );

    assert_eq!(
        state.folder_browser.selected_file_id(),
        Some(third.as_str())
    );
    assert!(
        state.sample_load_task.active().is_none(),
        "stale deferred navigation loads must not start decode work"
    );
    assert!(state.deferred_sample_load_task.active().is_some());
}

#[test]
fn keyboard_navigation_uses_memory_waveform_cache_without_worker() {
    let source_root = tempfile::tempdir().expect("source root");
    let first_path = source_root.path().join("a.wav");
    let second_path = source_root.path().join("b.wav");
    write_test_wav_i16(&first_path, &[0, 256, -256, 512]);
    write_test_wav_i16(&second_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let first = first_path.display().to_string();
    let second = second_path.display().to_string();

    let mut state = gui_state_for_span_tests();
    state.folder_browser = super::super::FolderBrowserState::from_sample_sources(&[
        wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
    ]);
    state.folder_browser.select_file(first);
    let loaded = super::super::WaveformState::load_path(second_path.clone()).expect("sample loads");
    state.remember_waveform(&loaded);
    state.waveform = super::super::WaveformState::synthetic_for_tests();

    let mut context = ui::UpdateContext::default();
    state.apply_message(
        super::super::GuiMessage::NavigateBrowser {
            delta: 1,
            extend: false,
        },
        &mut context,
    );

    assert_eq!(
        state.folder_browser.selected_file_id(),
        Some(second.as_str())
    );
    assert_eq!(state.waveform.path(), second_path);
    assert!(
        state.deferred_sample_load_task.active().is_none(),
        "memory-cached keyboard navigation should not debounce a reload"
    );
    assert!(
        state.sample_load_task.active().is_none(),
        "memory-cached keyboard navigation should not queue decode work"
    );
    assert!(
        state.sample_status.contains("b.wav"),
        "cached keyboard navigation should update the visible status, got {}",
        state.sample_status
    );
}

#[test]
fn keyboard_navigation_plays_loaded_sample_without_deferred_reload() {
    let Ok(player) = wavecrate::audio::AudioPlayer::new() else {
        return;
    };
    let source_root = tempfile::tempdir().expect("source root");
    for (name, samples) in [
        ("a.wav", &[0, 256, -256, 512][..]),
        ("b.wav", &[0, 1024, -2048, 4096, -1024, 512][..]),
    ] {
        write_test_wav_i16(&source_root.path().join(name), samples);
    }

    let mut state = gui_state_for_span_tests();
    state.audio_player = Some(player);
    state.folder_browser = super::super::FolderBrowserState::from_sample_sources(&[
        wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
    ]);
    let files = state.folder_browser.selected_audio_files();
    assert!(files.len() >= 2, "expected two visible samples");
    let first = files[0].id.clone();
    let second = files[1].id.clone();
    state.folder_browser.select_file(first);
    state.waveform =
        super::super::WaveformState::load_path(PathBuf::from(&second)).expect("sample loads");

    let mut context = ui::UpdateContext::default();
    state.apply_message(
        super::super::GuiMessage::NavigateBrowser {
            delta: 1,
            extend: false,
        },
        &mut context,
    );

    assert_eq!(
        state.folder_browser.selected_file_id(),
        Some(second.as_str())
    );
    assert!(
        state.waveform.is_playing(),
        "resident waveform should audition immediately during keyboard navigation"
    );
    assert_eq!(state.current_playback_span, Some((0.0, 1.0)));
    assert!(
        state.deferred_sample_load_task.active().is_none(),
        "already loaded navigation target should not queue a deferred reload"
    );
    assert!(
        state.sample_load_task.active().is_none(),
        "already loaded navigation target must not start decode work"
    );
}

#[test]
fn file_rename_remaps_loaded_waveform_and_cache_without_reload() {
    let source_root = tempfile::tempdir().expect("source root");
    let old_path = source_root.path().join("loaded.wav");
    write_test_wav_i16(&old_path, &[0, 1024, -1024, 512]);
    let new_path = source_root.path().join("renamed.wav");

    let mut state = gui_state_for_span_tests();
    state.folder_browser = super::super::FolderBrowserState::from_sample_sources(&[
        wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
    ]);
    state
        .folder_browser
        .select_file(old_path.display().to_string());
    state.waveform =
        super::super::WaveformState::load_path(old_path.clone()).expect("sample loads");
    let loaded = state.waveform.clone();
    state.remember_waveform(&loaded);
    assert!(state.waveform_cache.contains_key(&old_path));
    assert!(
        state
            .cached_sample_paths
            .contains(&old_path.display().to_string())
    );

    state
        .folder_browser
        .begin_rename_selected()
        .expect("rename can start")
        .expect("rename input");
    state.apply_folder_browser_rename_input(radiant::widgets::TextInputMessage::Submitted {
        value: String::from("renamed"),
    });

    assert_eq!(state.waveform.path(), new_path);
    assert!(state.waveform.has_loaded_sample());
    assert!(state.waveform_cache.contains_key(&new_path));
    assert!(!state.waveform_cache.contains_key(&old_path));
    assert!(
        state
            .cached_sample_paths
            .contains(&new_path.display().to_string())
    );
    assert!(
        !state
            .cached_sample_paths
            .contains(&old_path.display().to_string())
    );
    assert!(state.deferred_sample_load_task.active().is_none());
    assert!(state.sample_load_task.active().is_none());
    let new_id = new_path.display().to_string();
    assert_eq!(
        state.folder_browser.selected_file_id(),
        Some(new_id.as_str())
    );
}

#[test]
fn folder_rename_remaps_loaded_waveform_and_cache_without_reload() {
    let source_root = tempfile::tempdir().expect("source root");
    let old_folder = source_root.path().join("drums");
    fs::create_dir_all(&old_folder).expect("create source folder");
    let old_path = old_folder.join("loaded.wav");
    write_test_wav_i16(&old_path, &[0, 1024, -1024, 512]);
    let new_folder = source_root.path().join("breaks");
    let new_path = new_folder.join("loaded.wav");

    let mut state = gui_state_for_span_tests();
    state.folder_browser = super::super::FolderBrowserState::from_sample_sources(&[
        wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
    ]);
    let mut context = ui::UpdateContext::default();
    state.apply_message(
        super::super::GuiMessage::FolderBrowser(
            super::super::FolderBrowserMessage::ActivateFolder(old_folder.display().to_string()),
        ),
        &mut context,
    );
    state
        .folder_browser
        .select_file(old_path.display().to_string());
    state.waveform =
        super::super::WaveformState::load_path(old_path.clone()).expect("sample loads");
    let loaded = state.waveform.clone();
    state.remember_waveform(&loaded);

    state.apply_message(
        super::super::GuiMessage::FolderBrowser(
            super::super::FolderBrowserMessage::ActivateFolder(old_folder.display().to_string()),
        ),
        &mut context,
    );
    state
        .folder_browser
        .begin_rename_selected()
        .expect("rename can start")
        .expect("rename input");
    state.apply_folder_browser_rename_input(radiant::widgets::TextInputMessage::Submitted {
        value: String::from("breaks"),
    });

    assert_eq!(state.waveform.path(), new_path);
    assert!(state.waveform.has_loaded_sample());
    assert!(state.waveform_cache.contains_key(&new_path));
    assert!(!state.waveform_cache.contains_key(&old_path));
    assert!(
        state
            .cached_sample_paths
            .contains(&new_path.display().to_string())
    );
    assert!(
        !state
            .cached_sample_paths
            .contains(&old_path.display().to_string())
    );
    assert!(state.deferred_sample_load_task.active().is_none());
    assert!(state.sample_load_task.active().is_none());
}

#[test]
fn sample_selection_starts_playback_ready_persisted_cache_load_after_restart() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("cached.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let sample_path = sample_path.display().to_string();
    let sample_name = PathBuf::from(&sample_path)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .expect("sample file name");

    let waveform =
        super::super::WaveformState::load_path(sample_path.clone().into()).expect("cache sample");
    let file = waveform.file();
    super::super::waveform::store_cached_waveform_file_for_tests(&file);

    let mut state = gui_state_for_span_tests();
    state.folder_browser = super::super::FolderBrowserState::from_sample_sources(&[
        wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
    ]);
    state.refresh_persisted_waveform_cache_indicators();

    assert!(
        state.cached_sample_paths.contains(&sample_path),
        "persisted cache should mark the sample as ready before it enters memory cache"
    );

    let mut context = ui::UpdateContext::default();
    state.apply_message(
        super::super::GuiMessage::SelectSampleWithModifiers {
            path: sample_path.clone(),
            modifiers: Default::default(),
        },
        &mut context,
    );

    assert!(
        state.deferred_sample_load_task.active().is_none(),
        "playback-ready persisted cache should not wait for a debounce after restart"
    );
    assert!(
        state.sample_load_task.active().is_some(),
        "playback-ready persisted cache should start worker loading immediately"
    );
    assert!(
        state.waveform_loading_label.as_deref() == Some(sample_name.as_str()),
        "selection should show loading state while the persisted cache is promoted"
    );
    assert!(
        !state
            .waveform_cache
            .contains_key(&PathBuf::from(&sample_path)),
        "persisted cache promotion must stay off the UI thread until background loading completes"
    );
}

#[test]
fn playback_ready_persisted_cache_marks_row_without_memory_warm_after_restart() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("warm-before-click.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let sample_path_string = sample_path.display().to_string();
    let sample_path = PathBuf::from(&sample_path_string);

    let waveform =
        super::super::WaveformState::load_path(sample_path.clone()).expect("cache sample");
    let file = waveform.file();
    super::super::waveform::store_cached_waveform_file_for_tests(&file);

    let mut state = gui_state_for_span_tests();
    state.folder_browser = super::super::FolderBrowserState::from_sample_sources(&[
        wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
    ]);
    state.refresh_persisted_waveform_cache_indicators();

    assert!(state.cached_sample_paths.contains(&sample_path_string));
    assert!(
        !state.waveform_cache.contains_key(&sample_path),
        "restart indicator refresh should not synchronously deserialize cached waveforms"
    );
    assert!(
        state.waveform_cache_warm_pending.is_empty(),
        "playback-ready persisted caches should not be loaded into memory from UI refresh"
    );

    let mut context = ui::UpdateContext::default();
    state.apply_message(
        super::super::GuiMessage::SelectSampleWithModifiers {
            path: sample_path_string.clone(),
            modifiers: Default::default(),
        },
        &mut context,
    );

    assert!(
        state.deferred_sample_load_task.active().is_none(),
        "selection of a playback-ready cached file should not wait for debounce"
    );
    assert!(state.sample_load_task.active().is_some());
    assert_ne!(state.waveform.path(), sample_path);

    let ticket = state
        .sample_load_task
        .active()
        .expect("persisted cache load queued");
    state.apply_message(
        super::super::GuiMessage::SampleLoadFinished(ui::TaskCompletion {
            ticket,
            output: super::super::SampleLoadResult {
                path: sample_path_string,
                result: super::super::WaveformState::load_persisted_playback_cache(
                    sample_path.clone(),
                ),
                autoplay: false,
            },
        }),
        &mut context,
    );

    assert_eq!(state.waveform.path(), sample_path);
    assert!(
        state.waveform.audio_bytes().is_empty(),
        "playback-ready persisted cache should not reread source WAV bytes"
    );
}

#[test]
fn summary_only_persisted_cache_is_not_marked_playback_ready_after_restart() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("summary-only.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let sample_path_string = sample_path.display().to_string();
    let sample_path = PathBuf::from(&sample_path_string);

    let file = super::super::waveform::test_waveform_file_from_mono_samples(
        sample_path.clone(),
        fs::read(&sample_path).expect("read wav").into(),
        vec![0.0, 0.25, -0.25, 0.5, -0.5, 0.125],
    );
    super::super::waveform::store_summary_only_cached_waveform_file_for_tests(&file);

    let mut state = gui_state_for_span_tests();
    state.folder_browser = super::super::FolderBrowserState::from_sample_sources(&[
        wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
    ]);
    state.refresh_persisted_waveform_cache_indicators();

    assert!(
        !state.cached_sample_paths.contains(&sample_path_string),
        "summary-only persisted cache must not paint the row as playback-ready"
    );
    assert_eq!(
        state.waveform_cache_warm_pending.iter().collect::<Vec<_>>(),
        vec![&sample_path],
        "summary-only persisted cache should still be warmed in the background"
    );
}

#[test]
fn summary_only_persisted_cache_selection_uses_loading_pipeline_after_restart() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("summary-only-click.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let sample_path_string = sample_path.display().to_string();
    let sample_path = PathBuf::from(&sample_path_string);

    let file = super::super::waveform::test_waveform_file_from_mono_samples(
        sample_path.clone(),
        fs::read(&sample_path).expect("read wav").into(),
        vec![0.0, 0.25, -0.25, 0.5, -0.5, 0.125],
    );
    super::super::waveform::store_summary_only_cached_waveform_file_for_tests(&file);

    let mut state = gui_state_for_span_tests();
    state.folder_browser = super::super::FolderBrowserState::from_sample_sources(&[
        wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
    ]);
    state.refresh_persisted_waveform_cache_indicators();

    let mut context = ui::UpdateContext::default();
    state.apply_message(
        super::super::GuiMessage::SelectSampleWithModifiers {
            path: sample_path_string.clone(),
            modifiers: Default::default(),
        },
        &mut context,
    );

    assert!(
        state.deferred_sample_load_task.active().is_some(),
        "summary-only cache selection should not synchronously decode long playback samples"
    );
    assert_eq!(
        state.waveform.path(),
        PathBuf::from("synthetic-waveform"),
        "selection should wait for the normal loading pipeline instead of hydrating a partial cache"
    );
}

#[test]
fn background_warm_upgrades_summary_only_cache_to_playback_ready() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("summary-only-warm.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let sample_path_string = sample_path.display().to_string();
    let sample_path = PathBuf::from(&sample_path_string);

    let file = super::super::waveform::test_waveform_file_from_mono_samples(
        sample_path.clone(),
        fs::read(&sample_path).expect("read wav").into(),
        vec![0.0, 0.25, -0.25, 0.5, -0.5, 0.125],
    );
    super::super::waveform::store_summary_only_cached_waveform_file_for_tests(&file);

    let result =
        super::super::sample_load_actions::warm_persisted_waveform_cache(vec![sample_path.clone()]);
    assert_eq!(result.loaded.len(), 1);

    let mut restarted_state = gui_state_for_span_tests();
    restarted_state.folder_browser = super::super::FolderBrowserState::from_sample_sources(&[
        wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
    ]);
    restarted_state.refresh_persisted_waveform_cache_indicators();

    assert!(
        restarted_state
            .cached_sample_paths
            .contains(&sample_path_string),
        "background warm should persist playback readiness for future restarts"
    );
}

#[test]
fn normal_sample_load_persists_bright_cache_indicator_before_restart() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("fresh-cache.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let sample_path = sample_path.display().to_string();

    let _waveform =
        super::super::WaveformState::load_path(sample_path.clone().into()).expect("load sample");

    wait_for_playback_ready_cache(&sample_path);

    let mut restarted_state = gui_state_for_span_tests();
    restarted_state.folder_browser = super::super::FolderBrowserState::from_sample_sources(&[
        wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
    ]);
    restarted_state.refresh_persisted_waveform_cache_indicators();

    assert!(
        restarted_state.cached_sample_paths.contains(&sample_path),
        "freshly loaded cache indicator should survive immediate restart"
    );
}

fn wait_for_playback_ready_cache(sample_path: &str) {
    let path = PathBuf::from(sample_path);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while std::time::Instant::now() < deadline {
        if super::super::waveform::cached_waveform_file_playback_ready_exists(&path) {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

#[test]
fn selecting_another_sample_cancels_metadata_tag_entry() {
    let source_root = tempfile::tempdir().expect("source root");
    let first_path = source_root.path().join("first.wav");
    let second_path = source_root.path().join("second.wav");
    fs::write(&first_path, []).expect("first sample");
    fs::write(&second_path, []).expect("second sample");

    let mut state = gui_state_for_span_tests();
    state.folder_browser = super::super::FolderBrowserState::from_sample_sources(&[
        wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
    ]);
    let first_file = first_path.display().to_string();
    let second_file = second_path.display().to_string();
    state.folder_browser.select_file(first_file.clone());
    state.metadata_tag_draft = String::from("ki");
    state.metadata_tag_tokens = vec![String::from("warm")];
    state.metadata_tag_input_mode = super::super::MetadataTagInputMode::Category {
        pending_tag: String::from("new-tag"),
    };
    state.metadata_tag_completion_cycle.select("ki", 2, 4);

    state.select_sample_with_modifiers(
        second_file.clone(),
        PointerModifiers::default(),
        &mut ui::UpdateContext::default(),
    );

    assert_eq!(
        state.folder_browser.selected_file_id(),
        Some(second_file.as_str())
    );
    assert!(state.metadata_tag_draft.is_empty());
    assert!(state.metadata_tag_tokens.is_empty());
    assert_eq!(
        state.metadata_tag_input_mode,
        super::super::MetadataTagInputMode::Tag
    );
    assert_eq!(state.metadata_tag_completion_cycle.query_key(), None);
    assert_eq!(state.metadata_tag_completion_cycle.stored_index(), 0);
    assert_eq!(state.pending_metadata_tag_category_tag(), None);
    assert!(!state.metadata_tag_completion_active());
}

#[test]
fn play_selected_sample_uses_active_playmark_selection_span() {
    let Ok(player) = wavecrate::audio::AudioPlayer::new() else {
        return;
    };
    let mut state = GuiAppState::load_default().expect("default state loads");
    state.audio_player = Some(player);
    let sample_path = first_visible_asset_file_path(&state.folder_browser);
    state.waveform =
        super::super::WaveformState::load_path(sample_path.into()).expect("test sample loads");
    state
        .waveform
        .apply_interaction(WaveformInteraction::BeginSelection {
            kind: WaveformSelectionKind::Play,
            visible_ratio: 0.25,
        });
    state
        .waveform
        .apply_interaction(WaveformInteraction::UpdateSelection {
            visible_ratio: 0.60,
        });
    state
        .waveform
        .apply_interaction(WaveformInteraction::FinishSelection {
            visible_ratio: 0.60,
        });
    state.loop_playback = true;

    let mut context = ui::UpdateContext::default();
    state.play_selected_sample(&mut context);

    assert!(state.waveform.is_playing());
    assert_eq!(state.waveform.play_mark_ratio(), Some(0.25));
    assert_eq!(state.current_playback_span, Some((0.25, 0.6)));
    assert!(
        state
            .audio_player
            .as_ref()
            .is_some_and(|player| player.is_looping())
    );
    let progress = state
        .audio_player
        .as_ref()
        .and_then(|player| player.progress())
        .expect("playback progress");
    assert!(
        (0.24..=0.35).contains(&progress),
        "spacebar playback should start inside the playmark selection, got {progress}"
    );
}

#[test]
fn looped_playback_retargets_when_playmark_selection_is_created_and_resized() {
    let Ok(player) = wavecrate::audio::AudioPlayer::new() else {
        return;
    };
    let mut state = gui_state_for_span_tests();
    state.audio_player = Some(player);
    let sample_path = first_visible_asset_file_path(&state.folder_browser);
    state.waveform =
        super::super::WaveformState::load_path(sample_path.into()).expect("test sample loads");
    state.loop_playback = true;
    state
        .start_playback_current_span(0.0, 1.0)
        .expect("full sample loop starts");
    assert_player_progress_inside_span(&state, 0.0, 1.0);

    let mut context = ui::UpdateContext::default();
    state.apply_message(
        super::super::GuiMessage::Waveform(WaveformInteraction::BeginSelection {
            kind: WaveformSelectionKind::Play,
            visible_ratio: 0.25,
        }),
        &mut context,
    );
    state.apply_message(
        super::super::GuiMessage::Waveform(WaveformInteraction::UpdateSelection {
            visible_ratio: 0.60,
        }),
        &mut context,
    );
    state.apply_message(
        super::super::GuiMessage::Waveform(WaveformInteraction::FinishSelection {
            visible_ratio: 0.60,
        }),
        &mut context,
    );

    assert_playback_span_state(&state, 0.25, 0.60);
    assert_player_progress_inside_span(&state, 0.25, 0.60);
    assert!(
        state
            .audio_player
            .as_ref()
            .is_some_and(|player| player.is_looping())
    );

    state.apply_message(
        super::super::GuiMessage::Waveform(WaveformInteraction::BeginSelectionResize {
            kind: WaveformSelectionKind::Play,
            edge: WaveformSelectionEdge::Start,
            visible_ratio: 0.25,
        }),
        &mut context,
    );
    state.apply_message(
        super::super::GuiMessage::Waveform(WaveformInteraction::UpdateSelection {
            visible_ratio: 0.10,
        }),
        &mut context,
    );
    state.apply_message(
        super::super::GuiMessage::Waveform(WaveformInteraction::FinishSelection {
            visible_ratio: 0.10,
        }),
        &mut context,
    );

    assert_playback_span_state(&state, 0.10, 0.60);
    assert_player_progress_inside_span(&state, 0.10, 0.60);
}

fn assert_playback_span_state(state: &GuiAppState, expected_start: f32, expected_end: f32) {
    let (start, end) = state
        .current_playback_span
        .expect("current playback span should be set");
    assert!(
        (start - expected_start).abs() < 0.001,
        "start {start}, expected {expected_start}"
    );
    assert!(
        (end - expected_end).abs() < 0.001,
        "end {end}, expected {expected_end}"
    );
}

fn assert_player_progress_inside_span(state: &GuiAppState, start: f32, end: f32) {
    let progress = state
        .audio_player
        .as_ref()
        .and_then(|player| player.progress())
        .expect("audio player progress should be available");
    assert!(
        progress >= start - 0.02 && progress <= end + 0.02,
        "progress {progress}, expected inside {start}..={end}"
    );
}
