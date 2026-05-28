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
        folder_width: DEFAULT_FOLDER_WIDTH,
        folder_resize: None,
        folder_browser: super::super::FolderBrowserState::load_default(),
        waveform: super::super::WaveformState::synthetic_for_tests(),
        sample_status: String::new(),
        worker_sender: mpsc::channel().0,
        worker_receiver: None,
        next_task_id: 1,
        sample_load_task: ui::LatestTask::new(),
        sample_load_cancel: None,
        audio_open_task: ui::LatestTask::new(),
        audio_open_results: Default::default(),
        folder_progress: None,
        normalization_progress: None,
        progress_tick: 0.0,
        frame_index: 0,
        last_frame_at: None,
        max_frame_delta: Duration::ZERO,
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
        audio_backend_dropdown_open: false,
        audio_output_dropdown_open: false,
        audio_sample_rate_dropdown_open: false,
        job_details_open: false,
        context_menu: None,
        waveform_loading_label: None,
        audio_settings_error: None,
        current_playback_span: None,
        pending_playback_start: None,
        native_file_drop_hover: None,
        metadata_tag_draft: String::new(),
        metadata_tag_tokens: Vec::new(),
        metadata_tag_input_mode: Default::default(),
        metadata_tag_completion_prefix: None,
        metadata_tag_completion_index: 0,
        metadata_tag_dictionary: Default::default(),
        metadata_tag_library_open: false,
        metadata_tag_drag: None,
        metadata_tag_drop_hover: None,
        selected_metadata_tag: None,
        collapsed_metadata_tag_categories: Default::default(),
        metadata_tags_by_file: HashMap::new(),
        sample_name_view_mode: super::super::SampleNameViewMode::DiskFilename,
        startup_auto_load_pending: false,
        waveform_cache: HashMap::new(),
        waveform_cache_order: Default::default(),
        waveform_cache_bytes: 0,
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
        state.sample_load_task.active().is_none(),
        "repeat selection should reuse the in-memory waveform cache instead of queuing decode work"
    );
    assert_eq!(state.waveform_loading_label, None);
    assert_eq!(state.waveform.file_name(), sample_name);
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
    state.metadata_tag_completion_prefix = Some(String::from("ki"));
    state.metadata_tag_completion_index = 2;

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
    assert_eq!(state.metadata_tag_completion_prefix, None);
    assert_eq!(state.metadata_tag_completion_index, 0);
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
    let sample_path = selected_asset_file_path(&state.folder_browser, "portal_SS_kick_003.wav");
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
    let sample_path = selected_asset_file_path(&state.folder_browser, "portal_SS_kick_003.wav");
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
