use super::waveform::{WaveformSelectionEdge, WaveformSelectionKind};
use super::waveform_panel::waveform_loading_visual;
use super::{
    DEFAULT_FOLDER_WIDTH, GuiAppState, MAX_FOLDER_WIDTH, MIN_FOLDER_WIDTH, WaveformInteraction,
};
use radiant::{
    gui::types::{Point, Rect, Vector2},
    prelude::{self as ui, IntoView},
    runtime::{
        DeclarativeOwnedRuntimeBridge, Event, PaintPrimitive, SurfaceRuntime,
        TransientOverlayContext,
    },
    widgets::{DragHandleMessage, PointerButton, PointerModifiers, WidgetInput, WidgetKey},
};
use std::{collections::HashMap, fs, path::PathBuf, sync::mpsc, time::Duration};

mod audio_settings_controls;
mod audio_settings_dropdowns;
mod context_menu;
mod metadata_tag_tests;
mod native_file_drop;
mod sample_browser;
mod shortcuts_context;
mod status_bar;
mod toolbar_playback;

fn selected_asset_file_path(browser: &super::FolderBrowserState, name: &str) -> String {
    browser
        .selected_audio_files()
        .iter()
        .find(|file| file.name == name)
        .unwrap_or_else(|| panic!("expected bundled asset {name} to be visible"))
        .id
        .clone()
}

fn first_visible_asset_file_path(browser: &super::FolderBrowserState) -> String {
    browser
        .selected_audio_files()
        .first()
        .unwrap_or_else(|| panic!("expected at least one visible audio sample"))
        .id
        .clone()
}

fn gui_state_for_span_tests() -> GuiAppState {
    GuiAppState {
        folder_width: DEFAULT_FOLDER_WIDTH,
        folder_resize: None,
        folder_browser: super::FolderBrowserState::load_default(),
        waveform: super::WaveformState::synthetic_for_tests(),
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
        volume: super::DEFAULT_VOLUME,
        volume_persist_deadline: None,
        audio_output_config: super::AudioOutputConfig::default(),
        audio_output_resolved: None,
        audio_hosts: Vec::new(),
        audio_devices: Vec::new(),
        audio_sample_rates: Vec::new(),
        persisted_settings: super::AppSettingsCore::default(),
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
        sample_name_view_mode: super::SampleNameViewMode::DiskFilename,
        startup_auto_load_pending: false,
        waveform_cache: HashMap::new(),
        waveform_cache_order: Default::default(),
        waveform_cache_bytes: 0,
        cached_sample_paths: Default::default(),
    }
}

fn gui_state_with_temp_sample(name: &str) -> (GuiAppState, tempfile::TempDir, String) {
    let mut state = gui_state_for_span_tests();
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join(name);
    fs::write(&sample_path, []).expect("sample file");
    state.folder_browser = super::FolderBrowserState::from_sample_sources(&[
        wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
    ]);
    let selected_file = sample_path.display().to_string();
    state.folder_browser.select_file(selected_file.clone());
    (state, source_root, selected_file)
}

#[test]
fn folder_browser_splitter_resizes_and_clamps_width() {
    let mut state = GuiAppState {
        folder_width: DEFAULT_FOLDER_WIDTH,
        folder_resize: None,
        folder_browser: super::FolderBrowserState::load_default(),
        waveform: super::WaveformState::synthetic_for_tests(),
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
        volume: super::DEFAULT_VOLUME,
        volume_persist_deadline: None,
        audio_output_config: super::AudioOutputConfig::default(),
        audio_output_resolved: None,
        audio_hosts: Vec::new(),
        audio_devices: Vec::new(),
        audio_sample_rates: Vec::new(),
        persisted_settings: super::AppSettingsCore::default(),
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
        sample_name_view_mode: super::SampleNameViewMode::DiskFilename,
        startup_auto_load_pending: false,
        waveform_cache: HashMap::new(),
        waveform_cache_order: Default::default(),
        waveform_cache_bytes: 0,
        cached_sample_paths: Default::default(),
    };
    state.resize_folder_browser(DragHandleMessage::Started {
        position: Point::new(100.0, 0.0),
    });
    state.resize_folder_browser(DragHandleMessage::Moved {
        position: Point::new(160.0, 0.0),
    });

    assert_eq!(state.folder_width, DEFAULT_FOLDER_WIDTH + 60.0);

    state.resize_folder_browser(DragHandleMessage::Moved {
        position: Point::new(900.0, 0.0),
    });
    assert_eq!(state.folder_width, MAX_FOLDER_WIDTH);

    state.resize_folder_browser(DragHandleMessage::Ended {
        position: Point::new(-900.0, 0.0),
    });
    assert_eq!(state.folder_width, MIN_FOLDER_WIDTH);
    assert!(state.folder_resize.is_none());
}

#[test]
fn default_gui_starts_without_loading_a_sample() {
    let waveform = super::WaveformState::load_default().expect("default sample loads");
    assert!(!waveform.has_loaded_sample());
    assert_eq!(waveform.file_name(), "No sample loaded");
}

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

    super::normalize_wav_file_in_place(&path).expect("normalize wav");

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
    state.apply_message(super::GuiMessage::NormalizeSelectedSamples, &mut context);

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
        folder_browser: super::FolderBrowserState::load_default(),
        waveform: super::WaveformState::synthetic_for_tests(),
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
        volume: super::DEFAULT_VOLUME,
        volume_persist_deadline: None,
        audio_output_config: super::AudioOutputConfig::default(),
        audio_output_resolved: None,
        audio_hosts: Vec::new(),
        audio_devices: Vec::new(),
        audio_sample_rates: Vec::new(),
        persisted_settings: super::AppSettingsCore::default(),
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
        sample_name_view_mode: super::SampleNameViewMode::DiskFilename,
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
        super::GuiMessage::SelectSampleWithModifiers {
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
        super::GuiMessage::SampleLoadFinished(ui::TaskCompletion {
            ticket,
            output: super::SampleLoadResult {
                path: sample_path.clone(),
                result: super::WaveformState::load_path(sample_path.clone().into()),
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
        super::GuiMessage::SelectSampleWithModifiers {
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
    state.folder_browser = super::FolderBrowserState::from_sample_sources(&[
        wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
    ]);
    let first_file = first_path.display().to_string();
    let second_file = second_path.display().to_string();
    state.folder_browser.select_file(first_file.clone());
    state.metadata_tag_draft = String::from("ki");
    state.metadata_tag_tokens = vec![String::from("warm")];
    state.metadata_tag_input_mode = super::MetadataTagInputMode::Category {
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
        super::MetadataTagInputMode::Tag
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
        super::WaveformState::load_path(sample_path.into()).expect("test sample loads");
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
        super::WaveformState::load_path(sample_path.into()).expect("test sample loads");
    state.loop_playback = true;
    state
        .start_playback_current_span(0.0, 1.0)
        .expect("full sample loop starts");
    assert_player_progress_inside_span(&state, 0.0, 1.0);

    let mut context = ui::UpdateContext::default();
    state.apply_message(
        super::GuiMessage::Waveform(WaveformInteraction::BeginSelection {
            kind: WaveformSelectionKind::Play,
            visible_ratio: 0.25,
        }),
        &mut context,
    );
    state.apply_message(
        super::GuiMessage::Waveform(WaveformInteraction::UpdateSelection {
            visible_ratio: 0.60,
        }),
        &mut context,
    );
    state.apply_message(
        super::GuiMessage::Waveform(WaveformInteraction::FinishSelection {
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
        super::GuiMessage::Waveform(WaveformInteraction::BeginSelectionResize {
            kind: WaveformSelectionKind::Play,
            edge: WaveformSelectionEdge::Start,
            visible_ratio: 0.25,
        }),
        &mut context,
    );
    state.apply_message(
        super::GuiMessage::Waveform(WaveformInteraction::UpdateSelection {
            visible_ratio: 0.10,
        }),
        &mut context,
    );
    state.apply_message(
        super::GuiMessage::Waveform(WaveformInteraction::FinishSelection {
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

fn temp_gui_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "{name}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("create temp root");
    root
}

fn write_test_wav_i16(path: &std::path::Path, samples: &[i16]) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 48_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).expect("create wav");
    for sample in samples {
        writer.write_sample(*sample).expect("write sample");
    }
    writer.finalize().expect("finalize wav");
}

fn read_test_wav_f32(path: &std::path::Path) -> Vec<f32> {
    let mut reader = hound::WavReader::open(path).expect("open wav");
    reader
        .samples::<f32>()
        .collect::<Result<Vec<_>, _>>()
        .expect("read samples")
}

#[test]
fn clear_rebuildable_caches_action_removes_cache_payloads_only() {
    if std::env::var_os("WAVECRATE_CONFIG_HOME").is_some()
        || std::env::var_os("WAVECRATE_CONFIG_PROFILE").is_some()
    {
        return;
    }
    let base = tempfile::tempdir().expect("create config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(base.path().to_path_buf());
    let _profile_guard = wavecrate::app_dirs::PersistenceProfileGuard::live();
    let waveform_cache = wavecrate::app_dirs::waveform_cache_dir().expect("waveform cache dir");
    let cache_payload = waveform_cache.join("cached.bin");
    std::fs::write(&cache_payload, b"cache").expect("write cache payload");
    let handoff_dir = wavecrate::app_dirs::handoff_staging_dir().expect("handoff staging dir");
    let handoff_payload = handoff_dir.join("clip.wav");
    std::fs::write(&handoff_payload, b"clip").expect("write handoff payload");
    let mut state = GuiAppState::load_default().expect("default state loads");
    state.sample_status = String::from("ready");

    state.apply_message(
        super::GuiMessage::ClearRebuildableCaches,
        &mut ui::UpdateContext::default(),
    );

    assert!(!cache_payload.exists());
    assert!(handoff_payload.exists());
    assert_eq!(state.audio_settings_error, None);
    assert!(
        state.sample_status.contains("Rebuildable caches cleared"),
        "{}",
        state.sample_status
    );
}

#[test]
fn default_window_title_marks_alpha_build() {
    assert_eq!(super::launch::DEFAULT_WINDOW_TITLE, "Wavecrate - alpha");
}

#[test]
fn audio_settings_popover_opens_as_centered_floating_window() {
    let mut state = GuiAppState::load_default().expect("default state loads");
    state.audio_settings_error = None;
    let frame = radiant::runtime::UiSurface::new(super::audio_settings_popover(&state).into_node())
        .frame(
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(480.0, 360.0)),
            &radiant::theme::ThemeTokens::default(),
        );
    assert!(
        !frame.paint_plan.primitives.iter().any(|primitive| {
            matches!(
                primitive,
                PaintPrimitive::Text(text) if text.text.as_str() == "Audio Engine"
            )
        }),
        "audio settings should rely on the native window title"
    );
    let backend_rect = frame
        .paint_plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::Text(text) if text.text.as_str() == "Backend" => Some(text.rect),
            _ => None,
        })
        .expect("audio settings backend label paints");

    assert!(
        (66.0..=74.0).contains(&backend_rect.min.x),
        "{backend_rect:?}"
    );
    assert!(
        (41.0..=49.0).contains(&backend_rect.min.y),
        "{backend_rect:?}"
    );
}

#[test]
fn audio_settings_window_does_not_add_full_height_panel_chrome() {
    let mut state = GuiAppState::load_default().expect("default state loads");
    state.audio_settings_open = true;
    let frame = radiant::runtime::UiSurface::new(super::view(&mut state).into_node()).frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 540.0)),
        &radiant::theme::ThemeTokens::default(),
    );
    let audio_panel_fills = frame
        .paint_plan
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill)
                if fill.widget_id == 0
                    && fill.rect.min.x >= 250.0
                    && fill.rect.max.x <= 710.0
                    && fill.rect.width() >= 300.0 =>
            {
                Some(fill.rect)
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    assert!(
        audio_panel_fills
            .iter()
            .all(|rect| rect.height() <= super::AUDIO_SETTINGS_POPUP_HEIGHT + 1.0),
        "{audio_panel_fills:?}"
    );
}

#[test]
fn audio_settings_window_does_not_block_waveform_selection_messages() {
    let mut state = gui_state_for_span_tests();
    state.audio_settings_open = true;
    let mut context = ui::UpdateContext::default();

    state.apply_message(
        super::GuiMessage::Waveform(WaveformInteraction::BeginSelection {
            kind: WaveformSelectionKind::Play,
            visible_ratio: 0.45,
        }),
        &mut context,
    );
    state.apply_message(
        super::GuiMessage::Waveform(WaveformInteraction::UpdateSelection {
            visible_ratio: 0.65,
        }),
        &mut context,
    );
    state.apply_message(
        super::GuiMessage::Waveform(WaveformInteraction::FinishSelection {
            visible_ratio: 0.65,
        }),
        &mut context,
    );

    assert_eq!(state.waveform.play_mark_ratio(), Some(0.45));
    assert_eq!(
        state.waveform.play_selection(),
        Some(wavecrate::selection::SelectionRange::new(0.45, 0.65))
    );
}

#[test]
fn default_folder_browser_loads_assets_root() {
    let browser = super::FolderBrowserState::load_default();
    assert!(browser.root_path().ends_with("assets"));
    assert_eq!(browser.source_labels(), vec![String::from("Assets")]);
    assert!(
        browser
            .selected_files()
            .iter()
            .any(|file| file.name == "portal_SS_kick_003.wav")
    );
    assert!(
        browser
            .selected_audio_files()
            .iter()
            .any(|file| file.name == "portal_SS_kick_003.wav")
    );
}

#[test]
fn sample_browser_toggles_between_disk_and_metadata_label_names() {
    let (mut state, _source_root, tagged_file) = gui_state_with_temp_sample("tag-toggle.wav");
    state.metadata_tags_by_file.insert(
        tagged_file,
        vec![String::from("kick"), String::from("warm")],
    );
    let disk_frame =
        radiant::runtime::UiSurface::new(super::sample_browser(&mut state, false).into_node())
            .frame(
                Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(720.0, 240.0)),
                &radiant::theme::ThemeTokens::default(),
            );
    assert!(frame_has_text(&disk_frame, "Disk"));

    state.apply_message(
        super::GuiMessage::ToggleSampleNameViewMode,
        &mut ui::UpdateContext::default(),
    );
    let label_frame =
        radiant::runtime::UiSurface::new(super::sample_browser(&mut state, false).into_node())
            .frame(
                Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(720.0, 240.0)),
                &radiant::theme::ThemeTokens::default(),
            );

    assert!(frame_has_text(&label_frame, "Label"));
}

#[test]
fn default_gui_loads_persisted_sources_and_audio_output() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let source_id = wavecrate::sample_sources::SourceId::from_string("source_id::gui-test");
    wavecrate::sample_sources::config::save(&super::AppConfig {
        sources: vec![wavecrate::sample_sources::SampleSource::new_with_id(
            source_id,
            source_root.path().to_path_buf(),
        )],
        core: super::AppSettingsCore {
            audio_output: super::AudioOutputConfig {
                host: Some(String::from("test-host")),
                device: Some(String::from("Test Device")),
                sample_rate: Some(48_000),
                buffer_size: Some(256),
            },
            volume: 0.42,
            ..super::AppSettingsCore::default()
        },
    })
    .expect("seed config");

    let state = GuiAppState::load_default().expect("default state loads persisted config");

    assert_eq!(state.folder_browser.root_path(), source_root.path());
    assert_eq!(state.audio_output_config.host.as_deref(), Some("test-host"));
    assert_eq!(
        state.audio_output_config.device.as_deref(),
        Some("Test Device")
    );
    assert_eq!(state.audio_output_config.sample_rate, Some(48_000));
    assert!((state.volume - 0.42).abs() < f32::EPSILON);
}

#[test]
fn default_gui_saves_sources_and_audio_output_to_app_config() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let mut state = gui_state_for_span_tests();
    state.audio_output_config = super::AudioOutputConfig {
        host: Some(String::from("wasapi")),
        device: Some(String::from("Interface")),
        sample_rate: Some(96_000),
        buffer_size: None,
    };
    state.volume = 0.5;

    let request = state
        .folder_browser
        .begin_add_source_path(source_root.path().to_path_buf(), 100)
        .expect("new source requests scan");
    let result = super::folder_browser::scan_source_with_progress(request, |_| {}, |_| {});
    state.finish_folder_scan(result);

    let loaded = wavecrate::sample_sources::config::load_or_default().expect("reload config");
    assert_eq!(loaded.sources.len(), 1);
    assert_eq!(loaded.sources[0].root, source_root.path());
    assert_eq!(loaded.core.audio_output.host.as_deref(), Some("wasapi"));
    assert_eq!(
        loaded.core.audio_output.device.as_deref(),
        Some("Interface")
    );
    assert_eq!(loaded.core.audio_output.sample_rate, Some(96_000));
    assert!((loaded.core.volume - 0.5).abs() < f32::EPSILON);
}

#[test]
fn default_gui_removes_context_source_from_app_config() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let mut state = gui_state_for_span_tests();
    let request = state
        .folder_browser
        .begin_add_source_path(source_root.path().to_path_buf(), 100)
        .expect("new source requests scan");
    let result = super::folder_browser::scan_source_with_progress(request, |_| {}, |_| {});
    state.finish_folder_scan(result);
    state.context_menu = Some(super::BrowserContextMenu {
        kind: super::BrowserContextTargetKind::Source,
        path: source_root.path().to_path_buf(),
        source_id: Some(source_root.path().to_string_lossy().to_string()),
        metadata_tag: None,
        anchor: Point::new(12.0, 24.0),
        title: String::from("source root"),
    });

    state.remove_context_source();

    let loaded = wavecrate::sample_sources::config::load_or_default().expect("reload config");
    assert!(loaded.sources.is_empty());
    assert!(state.sample_status.contains("Removed source"));
    assert!(state.folder_browser.root_path().ends_with("assets"));
}

#[test]
fn waveform_loading_visual_paints_full_height_gray_fill_without_chrome() {
    let frame =
        radiant::runtime::UiSurface::new(waveform_loading_visual("kick.wav", 0.25).into_node())
            .frame(
                Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(720.0, 172.0)),
                &radiant::theme::ThemeTokens::default(),
            );

    let fill_rects = frame
        .paint_plan
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::FillRect(rect) => Some(rect),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert!(fill_rects.iter().any(|fill| {
        (fill.rect.width() - 180.0).abs() < 0.01
            && (fill.rect.height() - 172.0).abs() < 0.01
            && fill.rect.min.x == 0.0
            && fill.rect.min.y == 0.0
            && fill.color.r == 174
            && fill.color.g == 178
            && fill.color.b == 181
    }));
    assert!(
        frame
            .paint_plan
            .primitives
            .iter()
            .all(|primitive| !matches!(
                primitive,
                PaintPrimitive::StrokeRect(_) | PaintPrimitive::Text(_)
            ))
    );
}

fn frame_has_text(frame: &ui::SurfaceFrame, expected: &str) -> bool {
    frame
        .paint_plan
        .primitives
        .iter()
        .any(|primitive| match primitive {
            PaintPrimitive::Text(text) => text.text.as_str() == expected,
            _ => false,
        })
}

fn frame_has_text_after_x(frame: &ui::SurfaceFrame, expected: &str, min_x: f32) -> bool {
    frame
        .paint_plan
        .primitives
        .iter()
        .any(|primitive| match primitive {
            PaintPrimitive::Text(text) => {
                text.text.as_str() == expected && text.rect.min.x >= min_x
            }
            _ => false,
        })
}

fn frame_has_clip_height(frame: &ui::SurfaceFrame, expected: f32) -> bool {
    frame
        .paint_plan
        .primitives
        .iter()
        .any(|primitive| match primitive {
            PaintPrimitive::ClipStart(clip) => (clip.rect.height() - expected).abs() < 0.01,
            _ => false,
        })
}

fn text_rect(frame: &ui::SurfaceFrame, expected: &str) -> Option<Rect> {
    frame
        .paint_plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::Text(text) if text.text.as_str() == expected => Some(text.rect),
            _ => None,
        })
}

fn text_color(frame: &ui::SurfaceFrame, expected: &str) -> Option<radiant::gui::types::Rgba8> {
    frame
        .paint_plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::Text(text) if text.text.as_str() == expected => Some(text.color),
            _ => None,
        })
}

fn text_input_widget_id(frame: &ui::SurfaceFrame) -> Option<u64> {
    frame
        .paint_plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::TextInput(input) => Some(input.widget_id),
            _ => None,
        })
}
