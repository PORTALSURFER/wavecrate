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
fn sample_selection_queues_persisted_cache_load_after_restart() {
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
        state.deferred_sample_load_task.active().is_some(),
        "selection should debounce persisted cache hydration instead of reading it on the UI thread"
    );
    assert!(
        state.sample_load_task.active().is_none(),
        "selection should not start worker loading until the deferred load fires"
    );
    assert!(
        state.waveform_loading_label.as_deref() == Some(sample_name.as_str()),
        "selection may update loading UI, but not hydrate audio cache synchronously"
    );
    assert!(
        !state
            .waveform_cache
            .contains_key(&PathBuf::from(&sample_path)),
        "persisted cache hydration must stay off the UI thread until background loading completes"
    );
}

#[test]
fn playback_ready_persisted_cache_marks_row_without_memory_warm_after_restart() {
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
        state.deferred_sample_load_task.active().is_some(),
        "selection of a playback-ready cached file must still defer load handling off the UI thread"
    );
    assert!(state.sample_load_task.active().is_none());
    assert_ne!(state.waveform.path(), sample_path);
}

#[test]
fn summary_only_persisted_cache_is_not_marked_playback_ready_after_restart() {
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("summary-only.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let sample_path_string = sample_path.display().to_string();
    let sample_path = PathBuf::from(&sample_path_string);

    let waveform =
        super::super::WaveformState::load_path(sample_path.clone()).expect("cache sample");
    let file = waveform.file();
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
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("summary-only-click.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let sample_path_string = sample_path.display().to_string();
    let sample_path = PathBuf::from(&sample_path_string);

    let waveform =
        super::super::WaveformState::load_path(sample_path.clone()).expect("cache sample");
    let file = waveform.file();
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
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("summary-only-warm.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let sample_path_string = sample_path.display().to_string();
    let sample_path = PathBuf::from(&sample_path_string);

    let waveform =
        super::super::WaveformState::load_path(sample_path.clone()).expect("cache sample");
    let file = waveform.file();
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
