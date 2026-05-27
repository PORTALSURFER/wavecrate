use super::waveform::{WaveformSelectionEdge, WaveformSelectionKind};
use super::waveform_panel::waveform_loading_visual;
use super::{
    DEFAULT_FOLDER_WIDTH, GuiAppState, MAX_FOLDER_WIDTH, MIN_FOLDER_WIDTH, WaveformInteraction,
};
use radiant::{
    gui::types::{Point, Rect, Vector2},
    prelude::{self as ui, IntoView},
    runtime::{DeclarativeOwnedRuntimeBridge, Event, PaintPrimitive, SurfaceRuntime},
    widgets::{DragHandleMessage, PointerButton, PointerModifiers},
};
use std::{collections::HashMap, fs, path::PathBuf, sync::mpsc};

mod audio_settings_controls;
mod audio_settings_dropdowns;
mod native_file_drop;
mod sample_browser;
mod shortcuts_context;
mod status_bar;

fn selected_asset_file_path(browser: &super::FolderBrowserState, name: &str) -> String {
    browser
        .selected_audio_files()
        .iter()
        .find(|file| file.name == name)
        .unwrap_or_else(|| panic!("expected bundled asset {name} to be visible"))
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
        folder_progress: None,
        progress_tick: 0.0,
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
        native_file_drop_hover: None,
        metadata_tag_draft: String::new(),
        metadata_tag_tokens: Vec::new(),
        metadata_tag_input_mode: Default::default(),
        metadata_tag_completion_prefix: None,
        metadata_tag_completion_index: 0,
        metadata_tag_dictionary: Default::default(),
        metadata_tag_library_open: false,
        collapsed_metadata_tag_categories: Default::default(),
        metadata_tags_by_file: HashMap::new(),
        sample_name_view_mode: super::SampleNameViewMode::DiskFilename,
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
        folder_progress: None,
        progress_tick: 0.0,
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
        native_file_drop_hover: None,
        metadata_tag_draft: String::new(),
        metadata_tag_tokens: Vec::new(),
        metadata_tag_input_mode: Default::default(),
        metadata_tag_completion_prefix: None,
        metadata_tag_completion_index: 0,
        metadata_tag_dictionary: Default::default(),
        metadata_tag_library_open: false,
        collapsed_metadata_tag_categories: Default::default(),
        metadata_tags_by_file: HashMap::new(),
        sample_name_view_mode: super::SampleNameViewMode::DiskFilename,
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
        folder_progress: None,
        progress_tick: 0.0,
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
        native_file_drop_hover: None,
        metadata_tag_draft: String::new(),
        metadata_tag_tokens: Vec::new(),
        metadata_tag_input_mode: Default::default(),
        metadata_tag_completion_prefix: None,
        metadata_tag_completion_index: 0,
        metadata_tag_dictionary: Default::default(),
        metadata_tag_library_open: false,
        collapsed_metadata_tag_categories: Default::default(),
        metadata_tags_by_file: HashMap::new(),
        sample_name_view_mode: super::SampleNameViewMode::DiskFilename,
    };
    let sample_path = selected_asset_file_path(&state.folder_browser, "portal_SS_kick_003.wav");

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
        Some("portal_SS_kick_003.wav")
    );
    let ticket = state.sample_load_task.active().expect("sample load queued");
    state.apply_message(
        super::GuiMessage::SampleLoadFinished(ui::TaskCompletion {
            ticket,
            output: super::SampleLoadResult {
                path: sample_path.clone(),
                result: super::WaveformState::load_path(sample_path.clone().into()),
            },
        }),
        &mut context,
    );

    assert_eq!(
        state.folder_browser.selected_file_id(),
        Some(sample_path.as_str())
    );
    assert_eq!(state.waveform.file_name(), "portal_SS_kick_003.wav");
    assert_eq!(state.waveform_loading_label, None);
    assert!(state.waveform.frames() > 0);
    assert!(state.sample_status.contains("portal_SS_kick_003.wav"));
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

#[test]
fn toolbar_icon_assets_parse_and_paint_through_radiant_icon_button() {
    for icon in [
        super::ToolbarIcon::Loop,
        super::ToolbarIcon::Play,
        super::ToolbarIcon::Stop,
    ] {
        let enabled_svg = super::toolbar_icon_svg(icon, true, false);
        let active_svg = super::toolbar_icon_svg(icon, true, true);
        let disabled_svg = super::toolbar_icon_svg(icon, false, false);
        assert!(enabled_svg.contains(r##"fill="#eeeeee""##));
        assert!(active_svg.contains(r##"fill="#ffa052""##));
        assert!(disabled_svg.contains(r##"fill="#919191""##));
        assert!(!enabled_svg.contains("currentColor"));
        assert!(radiant::gui::svg::SvgIcon::from_svg(&enabled_svg).is_some());
        let frame = radiant::runtime::UiSurface::new(
            super::toolbar_icon_button(101, icon, true, false).into_node(),
        )
        .frame(
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(28.0, 24.0)),
            &radiant::theme::ThemeTokens::default(),
        );
        assert!(
            frame
                .paint_plan
                .primitives
                .iter()
                .any(|primitive| matches!(primitive, PaintPrimitive::Svg(_))),
            "toolbar icon should paint as a retained Radiant SVG"
        );
    }
}

#[test]
fn toolbar_icon_button_routes_transport_message_through_radiant_builder() {
    let surface = radiant::runtime::UiSurface::new(
        super::toolbar_icon_button(101, super::ToolbarIcon::Loop, true, false).into_node(),
    );

    assert_eq!(
        surface.dispatch_widget_output(
            101,
            radiant::widgets::WidgetOutput::typed(radiant::widgets::ButtonMessage::Activate),
        ),
        Some(super::GuiMessage::ToggleLoopPlayback)
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
fn folder_browser_sidebar_paints_filter_and_metadata_sections() {
    let browser = super::FolderBrowserState::load_default();
    let tags = vec![String::from("kick")];
    let frame = radiant::runtime::UiSurface::new(
        super::folder_browser::folder_browser_view(
            &browser,
            260.0,
            true,
            "",
            &[],
            None,
            "add tag",
            None,
            &[],
            &tags,
        )
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(260.0, 620.0)),
        &radiant::theme::ThemeTokens::default(),
    );

    assert!(frame_has_text(&frame, "Filter"));
    assert!(frame_has_text(&frame, "Metadata"));
    assert!(!frame_has_text(&frame, "Tagging"));
    assert!(frame_has_text(&frame, "kick"));
    assert!(frame_has_text(&frame, ">"));
}

#[test]
fn default_gui_tag_library_opens_beside_folder_sidebar() {
    let (mut state, _source_root, selected_file) = gui_state_with_temp_sample("tag-target.wav");
    state.metadata_tags_by_file.insert(
        selected_file.clone(),
        vec![String::from("hat"), String::from("seq")],
    );
    state.metadata_tags_by_file.insert(
        String::from("other.wav"),
        vec![String::from("bass"), String::from("hat")],
    );
    state.metadata_tag_library_open = true;

    let frame = radiant::runtime::UiSurface::new(super::view(&mut state).into_node()).frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(900.0, 620.0)),
        &radiant::theme::ThemeTokens::default(),
    );

    assert!(frame_has_text(&frame, "Tag Editor"));
    assert!(frame_has_text(&frame, "v Playback Type"));
    assert!(frame_has_text(&frame, "v Sound Type (2)"));
    assert!(frame_has_text(&frame, "v Character (1)"));
    assert!(frame_has_text(&frame, "v Prefix"));
    assert!(frame_has_text(&frame, "v Tuning/Scale"));
    assert!(frame_has_text(&frame, "[x] hat"));
    assert!(frame_has_text(&frame, "[ ] bass"));
    assert!(frame_has_text(&frame, "[x] seq"));
}

#[test]
fn default_gui_tag_library_button_adds_existing_tag() {
    let (mut state, _source_root, selected_file) = gui_state_with_temp_sample("tag-target.wav");
    state.metadata_tags_by_file.insert(
        String::from("other.wav"),
        vec![String::from("bass"), String::from("hat")],
    );

    state.apply_message(
        super::GuiMessage::ToggleMetadataTagLibrary,
        &mut ui::UpdateContext::default(),
    );
    state.apply_message(
        super::GuiMessage::ToggleMetadataTag(String::from("bass")),
        &mut ui::UpdateContext::default(),
    );

    assert!(state.metadata_tag_library_open);
    assert_eq!(
        state.metadata_tags_by_file.get(&selected_file),
        Some(&vec![String::from("bass")])
    );
}

#[test]
fn default_gui_tag_library_button_removes_selected_tag() {
    let (mut state, _source_root, selected_file) = gui_state_with_temp_sample("tag-target.wav");
    state.metadata_tags_by_file.insert(
        selected_file.clone(),
        vec![String::from("bass"), String::from("hat")],
    );
    state
        .metadata_tags_by_file
        .insert(String::from("other.wav"), vec![String::from("bass")]);

    state.apply_message(
        super::GuiMessage::ToggleMetadataTagLibrary,
        &mut ui::UpdateContext::default(),
    );
    state.apply_message(
        super::GuiMessage::ToggleMetadataTag(String::from("bass")),
        &mut ui::UpdateContext::default(),
    );

    assert!(state.metadata_tag_library_open);
    assert_eq!(
        state.metadata_tags_by_file.get(&selected_file),
        Some(&vec![String::from("hat")])
    );
    assert_eq!(state.sample_status, "Removed tag bass");
}

#[test]
fn default_gui_tag_library_category_headers_collapse_groups() {
    let (mut state, _source_root, selected_file) = gui_state_with_temp_sample("tag-target.wav");
    state
        .metadata_tags_by_file
        .insert(selected_file, vec![String::from("hat")]);
    state.metadata_tag_library_open = true;

    state.apply_message(
        super::GuiMessage::ToggleMetadataTagCategory(String::from("sound-type")),
        &mut ui::UpdateContext::default(),
    );

    let frame = radiant::runtime::UiSurface::new(super::view(&mut state).into_node()).frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(900.0, 620.0)),
        &radiant::theme::ThemeTokens::default(),
    );

    assert!(frame_has_text(&frame, "> Sound Type (1)"));
    assert!(!frame_has_text(&frame, "[x] hat"));
}

#[test]
fn default_gui_tag_library_uses_custom_dictionary_categories() {
    let (mut state, _source_root, selected_file) = gui_state_with_temp_sample("tag-target.wav");
    state
        .metadata_tags_by_file
        .insert(selected_file, vec![String::from("deep-kick")]);
    state
        .metadata_tag_dictionary
        .insert(String::from("deep-kick"), String::from("sound-type"));
    state.metadata_tag_library_open = true;

    let frame = radiant::runtime::UiSurface::new(super::view(&mut state).into_node()).frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(900.0, 620.0)),
        &radiant::theme::ThemeTokens::default(),
    );

    assert!(frame_has_text(&frame, "v Sound Type (1)"));
    assert!(frame_has_text(&frame, "[x] deep-kick"));
    assert!(frame_has_text(&frame, "v Character"));
    assert!(!frame_has_text(&frame, "v Character (1)"));
}

#[test]
fn folder_browser_metadata_hides_tag_entry_when_no_file_is_selected() {
    let browser = super::FolderBrowserState::load_default();
    let tags = vec![String::from("kick")];
    let frame = radiant::runtime::UiSurface::new(
        super::folder_browser::folder_browser_view(
            &browser,
            260.0,
            false,
            "",
            &[],
            None,
            "add tag",
            None,
            &[],
            &tags,
        )
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(260.0, 620.0)),
        &radiant::theme::ThemeTokens::default(),
    );

    assert!(frame_has_text(&frame, "Metadata"));
    assert!(!frame_has_text(&frame, "Tags (1)"));
    assert!(!frame_has_text(&frame, "kick"));
    assert!(
        !frame
            .paint_plan
            .primitives
            .iter()
            .any(|primitive| matches!(primitive, PaintPrimitive::TextInput(_)))
    );
}

#[test]
fn folder_browser_metadata_tags_grow_combined_entry_field() {
    let browser = super::FolderBrowserState::load_default();
    let small_tags = vec![String::from("kick")];
    let larger_tags = vec![
        String::from("kick"),
        String::from("warm"),
        String::from("one-shot"),
        String::from("distorted"),
    ];
    let small = radiant::runtime::UiSurface::new(
        super::folder_browser::folder_browser_view(
            &browser,
            260.0,
            true,
            "",
            &[],
            None,
            "add tag",
            None,
            &[],
            &small_tags,
        )
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(260.0, 620.0)),
        &radiant::theme::ThemeTokens::default(),
    );
    let larger = radiant::runtime::UiSurface::new(
        super::folder_browser::folder_browser_view(
            &browser,
            260.0,
            true,
            "",
            &[],
            None,
            "add tag",
            None,
            &[],
            &larger_tags,
        )
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(260.0, 620.0)),
        &radiant::theme::ThemeTokens::default(),
    );

    assert!(frame_has_text(&larger, "distorted"));
    assert!(!frame_has_text(&larger, "More"));
    assert!(frame_has_clip_height(&small, 24.0));
    assert!(frame_has_clip_height(&larger, 66.0));
}

#[test]
fn folder_browser_metadata_tag_field_caps_at_six_rows_then_scrolls() {
    let browser = super::FolderBrowserState::load_default();
    let tags = (0..24)
        .map(|index| format!("tag-{index:02}"))
        .collect::<Vec<_>>();
    let frame = radiant::runtime::UiSurface::new(
        super::folder_browser::folder_browser_view(
            &browser,
            260.0,
            true,
            "",
            &[],
            None,
            "add tag",
            None,
            &[],
            &tags,
        )
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(260.0, 620.0)),
        &radiant::theme::ThemeTokens::default(),
    );

    let tag_clip = frame.paint_plan.primitives.iter().find_map(|primitive| {
        if let PaintPrimitive::ClipStart(clip) = primitive
            && (clip.rect.height() - 129.0).abs() < 0.01
        {
            return Some(clip.rect);
        }
        None
    });
    assert!(
        tag_clip.is_some(),
        "combined tag field should clip overflowing tag rows"
    );
}

#[test]
fn metadata_tag_input_prompts_for_category_before_adding_new_tag() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let (mut state, _source_root, selected_file) = gui_state_with_temp_sample("tag-target.wav");

    state.apply_message(
        super::GuiMessage::MetadataTagInput(radiant::widgets::TextInputMessage::Submitted {
            value: String::from("Deep Kick"),
        }),
        &mut ui::UpdateContext::default(),
    );

    assert_eq!(state.metadata_tags_by_file.get(&selected_file), None);
    assert_eq!(state.pending_metadata_tag_category_tag(), Some("deep-kick"));
    assert_eq!(
        state.metadata_tag_input_placeholder(),
        "select group/parent tag"
    );
    assert_eq!(state.sample_status, "Choose a category for deep-kick");

    state.apply_message(
        super::GuiMessage::MetadataTagInput(radiant::widgets::TextInputMessage::Changed {
            value: String::from("sound"),
        }),
        &mut ui::UpdateContext::default(),
    );
    assert_eq!(
        state
            .metadata_tag_completion_options()
            .iter()
            .find(|option| option.selected)
            .map(|option| option.tag.as_str()),
        Some("Sound Type")
    );

    state.apply_message(
        super::GuiMessage::MetadataTagInput(radiant::widgets::TextInputMessage::Submitted {
            value: String::from("sound"),
        }),
        &mut ui::UpdateContext::default(),
    );

    assert_eq!(
        state.metadata_tags_by_file.get(&selected_file),
        Some(&vec![String::from("deep-kick")])
    );
    assert_eq!(
        state
            .metadata_tag_dictionary
            .get("deep-kick")
            .map(String::as_str),
        Some("sound-type")
    );
    assert_eq!(state.pending_metadata_tag_category_tag(), None);
    assert!(state.metadata_tag_draft.is_empty());
    assert_eq!(state.sample_status, "Added tag deep-kick");
}

#[test]
fn metadata_tag_input_persists_tag_assignments_and_removals_to_source_database() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("persistent-tag.wav");
    fs::write(&sample_path, []).expect("sample file");
    wavecrate::sample_sources::config::save(&super::AppConfig {
        sources: vec![wavecrate::sample_sources::SampleSource::new(
            source_root.path().to_path_buf(),
        )],
        core: super::AppSettingsCore::default(),
    })
    .expect("seed config");
    let selected_file = sample_path.display().to_string();
    let mut state = GuiAppState::load_default().expect("default state loads");
    state.folder_browser.select_file(selected_file.clone());

    state.apply_message(
        super::GuiMessage::MetadataTagInput(radiant::widgets::TextInputMessage::Submitted {
            value: String::from("Deep Kick, Warm Tone"),
        }),
        &mut ui::UpdateContext::default(),
    );
    assert_eq!(
        state.metadata_tags_by_file.get(&selected_file),
        Some(&vec![String::from("deep-kick"), String::from("warm-tone")])
    );

    super::metadata_tags::persist_metadata_tag_additions_for_tests(
        sample_path.clone(),
        source_root.path().to_path_buf(),
        PathBuf::from("persistent-tag.wav"),
        vec![String::from("deep-kick"), String::from("warm-tone")],
    )
    .expect("persist tags");

    let db = wavecrate::sample_sources::SourceDatabase::open(source_root.path())
        .expect("open source db");
    assert_eq!(
        db.tag_labels_for_path(std::path::Path::new("persistent-tag.wav"))
            .expect("tag labels"),
        vec![String::from("deep-kick"), String::from("warm-tone")]
    );

    super::metadata_tags::persist_metadata_tag_removals_for_tests(
        sample_path.clone(),
        source_root.path().to_path_buf(),
        PathBuf::from("persistent-tag.wav"),
        vec![String::from("deep-kick")],
    )
    .expect("persist tag removal");

    assert_eq!(
        db.tag_labels_for_path(std::path::Path::new("persistent-tag.wav"))
            .expect("tag labels after removal"),
        vec![String::from("warm-tone")]
    );

    let reloaded = GuiAppState::load_default().expect("default state reloads");
    assert_eq!(
        reloaded.metadata_tags_by_file.get(&selected_file),
        Some(&vec![String::from("warm-tone")])
    );
}

#[test]
fn metadata_tag_input_keeps_delimiters_while_editing() {
    let mut state = gui_state_for_span_tests();

    state.apply_message(
        super::GuiMessage::MetadataTagInput(radiant::widgets::TextInputMessage::Changed {
            value: String::from("kick, warm tone"),
        }),
        &mut ui::UpdateContext::default(),
    );

    assert!(state.metadata_tags_by_file.is_empty());
    assert_eq!(state.metadata_tag_draft, "kick, warm tone");
}

#[test]
fn metadata_tag_input_enters_selected_known_prefix() {
    let (mut state, _source_root, selected_file) = gui_state_with_temp_sample("tag-target.wav");
    state.metadata_tags_by_file.insert(
        String::from("known-file"),
        vec![String::from("kick"), String::from("warm")],
    );

    state.apply_message(
        super::GuiMessage::MetadataTagInput(radiant::widgets::TextInputMessage::Changed {
            value: String::from("ki"),
        }),
        &mut ui::UpdateContext::default(),
    );
    assert_eq!(
        state.metadata_tag_completion_suffix().as_deref(),
        Some("ck")
    );

    state.apply_message(
        super::GuiMessage::MetadataTagInput(radiant::widgets::TextInputMessage::Submitted {
            value: String::from("ki"),
        }),
        &mut ui::UpdateContext::default(),
    );

    assert_eq!(
        state.metadata_tags_by_file.get(&selected_file),
        Some(&vec![String::from("kick")])
    );
}

#[test]
fn metadata_tag_input_arrows_through_multiple_known_prefix_matches() {
    let (mut state, _source_root, selected_file) = gui_state_with_temp_sample("tag-target.wav");
    state.metadata_tags_by_file.insert(
        String::from("known-file"),
        vec![
            String::from("kick"),
            String::from("kicker"),
            String::from("kind"),
        ],
    );

    state.apply_message(
        super::GuiMessage::MetadataTagInput(radiant::widgets::TextInputMessage::Changed {
            value: String::from("ki"),
        }),
        &mut ui::UpdateContext::default(),
    );
    assert_eq!(
        state
            .metadata_tag_completion_options()
            .iter()
            .find(|option| option.selected)
            .map(|option| option.tag.as_str()),
        Some("kick")
    );

    state.apply_message(
        super::GuiMessage::MoveMetadataTagCompletion(1),
        &mut ui::UpdateContext::default(),
    );
    assert_eq!(
        state
            .metadata_tag_completion_options()
            .iter()
            .find(|option| option.selected)
            .map(|option| option.tag.as_str()),
        Some("kicker")
    );

    state.apply_message(
        super::GuiMessage::MoveMetadataTagCompletion(1),
        &mut ui::UpdateContext::default(),
    );
    assert_eq!(
        state
            .metadata_tag_completion_options()
            .iter()
            .find(|option| option.selected)
            .map(|option| option.tag.as_str()),
        Some("kind")
    );

    state.apply_message(
        super::GuiMessage::MetadataTagInput(radiant::widgets::TextInputMessage::Submitted {
            value: String::from("ki"),
        }),
        &mut ui::UpdateContext::default(),
    );

    assert_eq!(
        state.metadata_tags_by_file.get(&selected_file),
        Some(&vec![String::from("kind")])
    );
    assert!(state.metadata_tag_draft.is_empty());
}

#[test]
fn folder_browser_metadata_tag_field_renders_completion_suffix_and_options() {
    let browser = super::FolderBrowserState::load_default();
    let completion_options = vec![
        super::metadata_tags::MetadataTagCompletionOption {
            tag: String::from("kick"),
            category: "Sound Type",
            selected: true,
        },
        super::metadata_tags::MetadataTagCompletionOption {
            tag: String::from("kicker"),
            category: "Character",
            selected: false,
        },
    ];
    let frame = radiant::runtime::UiSurface::new(
        super::folder_browser::folder_browser_view(
            &browser,
            260.0,
            true,
            "ki",
            &[String::from("kick")],
            None,
            "add tag",
            Some("ck"),
            completion_options.as_slice(),
            &[String::from("warm")],
        )
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(260.0, 620.0)),
        &radiant::theme::ThemeTokens::default(),
    );

    assert!(frame_has_text(&frame, "kick"));
    assert!(frame_has_text(&frame, "ck"));
    assert!(frame_has_text(&frame, "Sound Type"));
    assert!(frame_has_text(&frame, "kicker"));
    assert!(frame_has_text(&frame, "Character"));
    assert!(!frame_has_text(&frame, "Tab kick"));
    assert!(frame_has_text(&frame, "warm"));
    assert!(frame.paint_plan.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            PaintPrimitive::TextInput(input) if input.rect.height() <= 14.0
        )
    }));
    assert!(frame.paint_plan.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            PaintPrimitive::FillRect(fill) if (fill.rect.height() - 18.0).abs() < 0.01
        )
    }));
}

#[test]
fn metadata_autocomplete_does_not_block_sidebar_button_clicks() {
    let (mut state, _source_root, _selected_file) = gui_state_with_temp_sample("tag-target.wav");
    state
        .metadata_tags_by_file
        .insert(String::from("known.wav"), vec![String::from("kick")]);
    state.metadata_tag_draft = String::from("ki");

    let bridge = DeclarativeOwnedRuntimeBridge::new(
        state,
        |state| radiant::runtime::UiSurface::new(super::view(state).into_node()),
        |state, message| state.apply_message(message, &mut ui::UpdateContext::default()),
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(900.0, 620.0));
    let frame = runtime.frame(&radiant::theme::ThemeTokens::default());
    let input_rect = frame
        .paint_plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::TextInput(input) => Some(input.rect),
            _ => None,
        })
        .expect("metadata tag input should paint");
    let input_point = Point::new(
        (input_rect.min.x + input_rect.max.x) * 0.5,
        (input_rect.min.y + input_rect.max.y) * 0.5,
    );
    runtime.dispatch_event(Event::PointerPress {
        position: input_point,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });
    runtime.dispatch_event(Event::PointerRelease {
        position: input_point,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });
    assert!(runtime.focused_widget().is_some());

    let toggle_rect = text_rect(&runtime.frame(&radiant::theme::ThemeTokens::default()), ">")
        .expect("tag library toggle should paint");
    let point = Point::new(
        (toggle_rect.min.x + toggle_rect.max.x) * 0.5,
        (toggle_rect.min.y + toggle_rect.max.y) * 0.5,
    );

    runtime.dispatch_event(Event::PointerPress {
        position: point,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });
    runtime.dispatch_event(Event::PointerRelease {
        position: point,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });

    assert!(
        runtime.bridge().state().metadata_tag_library_open,
        "autocomplete popup must not prevent clicking the sidebar tag editor button"
    );
}

#[test]
fn metadata_autocomplete_does_not_block_folder_tree_clicks() {
    let mut state = gui_state_for_span_tests();
    let selected_file = state
        .folder_browser
        .selected_audio_files()
        .first()
        .expect("default browser should expose audio files")
        .id
        .clone();
    state.folder_browser.select_file(selected_file);
    state
        .metadata_tags_by_file
        .insert(String::from("known.wav"), vec![String::from("kick")]);
    state.metadata_tag_draft = String::from("ki");
    let selected_folder = state
        .folder_browser
        .selected_folder_path()
        .expect("selected folder")
        .display()
        .to_string();

    let bridge = DeclarativeOwnedRuntimeBridge::new(
        state,
        |state| radiant::runtime::UiSurface::new(super::view(state).into_node()),
        |state, message| state.apply_message(message, &mut ui::UpdateContext::default()),
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(900.0, 620.0));
    let frame = runtime.frame(&radiant::theme::ThemeTokens::default());
    let input_rect = frame
        .paint_plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::TextInput(input) => Some(input.rect),
            _ => None,
        })
        .expect("metadata tag input should paint");
    let input_point = Point::new(
        (input_rect.min.x + input_rect.max.x) * 0.5,
        (input_rect.min.y + input_rect.max.y) * 0.5,
    );
    runtime.dispatch_event(Event::PointerPress {
        position: input_point,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });
    runtime.dispatch_event(Event::PointerRelease {
        position: input_point,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });
    assert!(runtime.focused_widget().is_some());

    let frame = runtime.frame(&radiant::theme::ThemeTokens::default());
    let (label, folder_rect) = frame
        .paint_plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::Text(text) if text.text.as_str().starts_with("[-] ") => {
                Some((text.text.to_string(), text.rect))
            }
            _ => None,
        })
        .expect("expanded selected root folder should paint");
    let point = Point::new(
        (folder_rect.min.x + folder_rect.max.x) * 0.5,
        (folder_rect.min.y + folder_rect.max.y) * 0.5,
    );

    runtime.dispatch_event(Event::PointerPress {
        position: point,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });
    runtime.dispatch_event(Event::PointerRelease {
        position: point,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });

    assert_eq!(
        runtime
            .bridge()
            .state()
            .folder_browser
            .folder_expansion_for_tests(&selected_folder),
        Some(false),
        "autocomplete popup must not prevent clicking folder row {label}"
    );
}

#[test]
fn metadata_autocomplete_does_not_block_tag_library_clicks() {
    let (mut state, _source_root, selected_file) = gui_state_with_temp_sample("tag-target.wav");
    state.metadata_tags_by_file.insert(
        String::from("known.wav"),
        vec![String::from("kick"), String::from("bass")],
    );
    state.metadata_tag_draft = String::from("ki");
    state.metadata_tag_library_open = true;

    let bridge = DeclarativeOwnedRuntimeBridge::new(
        state,
        |state| radiant::runtime::UiSurface::new(super::view(state).into_node()),
        |state, message| state.apply_message(message, &mut ui::UpdateContext::default()),
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(900.0, 620.0));
    let frame = runtime.frame(&radiant::theme::ThemeTokens::default());
    let input_rect = frame
        .paint_plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::TextInput(input) => Some(input.rect),
            _ => None,
        })
        .expect("metadata tag input should paint");
    let input_point = Point::new(
        (input_rect.min.x + input_rect.max.x) * 0.5,
        (input_rect.min.y + input_rect.max.y) * 0.5,
    );
    runtime.dispatch_event(Event::PointerPress {
        position: input_point,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });
    runtime.dispatch_event(Event::PointerRelease {
        position: input_point,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });
    assert!(runtime.focused_widget().is_some());

    let tag_rect = text_rect(&runtime.frame(&radiant::theme::ThemeTokens::default()), "[ ] bass")
        .expect("available tag should paint");
    let point = Point::new(
        (tag_rect.min.x + tag_rect.max.x) * 0.5,
        (tag_rect.min.y + tag_rect.max.y) * 0.5,
    );

    runtime.dispatch_event(Event::PointerPress {
        position: point,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });
    runtime.dispatch_event(Event::PointerRelease {
        position: point,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });

    assert_eq!(
        runtime.bridge().state().metadata_tags_by_file.get(&selected_file),
        Some(&vec![String::from("bass")]),
        "autocomplete popup must not prevent clicking tags in the tag library"
    );
}

#[test]
fn metadata_autocomplete_does_not_block_source_row_clicks_with_tag_library_open() {
    let source_base = tempfile::tempdir().expect("source base");
    let first_root = source_base.path().join("Alpha Samples");
    let second_root = source_base.path().join("Beta Samples");
    fs::create_dir_all(&first_root).expect("first source");
    fs::create_dir_all(&second_root).expect("second source");
    fs::write(first_root.join("alpha.wav"), []).expect("first sample");
    fs::write(second_root.join("beta.wav"), []).expect("second sample");

    let mut state = gui_state_for_span_tests();
    state.folder_browser = super::FolderBrowserState::from_sample_sources(&[
        wavecrate::sample_sources::SampleSource::new(first_root.clone()),
        wavecrate::sample_sources::SampleSource::new(second_root.clone()),
    ]);
    let first_file = first_root.join("alpha.wav").display().to_string();
    state.folder_browser.select_file(first_file);
    state
        .metadata_tags_by_file
        .insert(String::from("known.wav"), vec![String::from("kick")]);
    state.metadata_tag_draft = String::from("ki");
    state.metadata_tag_library_open = true;

    let bridge = DeclarativeOwnedRuntimeBridge::new(
        state,
        |state| radiant::runtime::UiSurface::new(super::view(state).into_node()),
        |state, message| state.apply_message(message, &mut ui::UpdateContext::default()),
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(589.0, 571.0));
    let frame = runtime.frame(&radiant::theme::ThemeTokens::default());
    let input_rect = frame
        .paint_plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::TextInput(input) => Some(input.rect),
            _ => None,
        })
        .expect("metadata tag input should paint");
    let input_point = Point::new(
        (input_rect.min.x + input_rect.max.x) * 0.5,
        (input_rect.min.y + input_rect.max.y) * 0.5,
    );
    runtime.dispatch_event(Event::PointerPress {
        position: input_point,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });
    runtime.dispatch_event(Event::PointerRelease {
        position: input_point,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });
    assert!(runtime.focused_widget().is_some());

    let source_rect = text_rect(
        &runtime.frame(&radiant::theme::ThemeTokens::default()),
        "Beta Samples",
    )
    .expect("second source should paint");
    let point = Point::new(
        (source_rect.min.x + source_rect.max.x) * 0.5,
        (source_rect.min.y + source_rect.max.y) * 0.5,
    );
    runtime.dispatch_event(Event::PointerPress {
        position: point,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });
    runtime.dispatch_event(Event::PointerRelease {
        position: point,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });

    assert_eq!(
        runtime.bridge().state().folder_browser.selected_folder_path(),
        Some(second_root),
        "autocomplete popup and tag library must not prevent clicking source rows"
    );
}

#[test]
fn folder_browser_metadata_tag_field_renders_pending_category_prompt() {
    let browser = super::FolderBrowserState::load_default();
    let completion_options = vec![super::metadata_tags::MetadataTagCompletionOption {
        tag: String::from("Sound Type"),
        category: "Group",
        selected: true,
    }];
    let frame = radiant::runtime::UiSurface::new(
        super::folder_browser::folder_browser_view(
            &browser,
            260.0,
            true,
            "sound",
            &[],
            Some("deep-kick"),
            "select group/parent tag",
            Some("-type"),
            completion_options.as_slice(),
            &[],
        )
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(260.0, 620.0)),
        &radiant::theme::ThemeTokens::default(),
    );

    assert!(frame_has_text(&frame, "deep-kick ->"));
    assert!(frame_has_text(&frame, "Sound Type"));
    assert!(frame_has_text(&frame, "Group"));
    let pending_tag_rect = text_rect(&frame, "deep-kick ->").expect("pending tag should paint");
    let suffix_rect = text_rect(&frame, "-type").expect("completion suffix should paint");
    assert!(
        suffix_rect.min.x > pending_tag_rect.max.x,
        "category input should stay on the same row after the pending tag arrow"
    );
    let sound_type_rect = text_rect(&frame, "Sound Type").expect("completion option should paint");
    assert!(
        sound_type_rect.max.y < pending_tag_rect.min.y,
        "completion popup should expand upward above the tag input"
    );
}

#[test]
fn folder_browser_metadata_tag_input_moves_to_next_row_when_crowded() {
    let browser = super::FolderBrowserState::load_default();
    let tags = vec![
        String::from("test"),
        String::from("another"),
        String::from("cool-tag"),
    ];
    let frame = radiant::runtime::UiSurface::new(
        super::folder_browser::folder_browser_view(
            &browser,
            260.0,
            true,
            "wow",
            &[],
            None,
            "add tag",
            None,
            &[],
            &tags,
        )
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(260.0, 620.0)),
        &radiant::theme::ThemeTokens::default(),
    );

    let tag_clip = frame.paint_plan.primitives.iter().find_map(|primitive| {
        if let PaintPrimitive::ClipStart(clip) = primitive
            && (clip.rect.height() - 45.0).abs() < 0.01
        {
            return Some(clip.rect);
        }
        None
    });
    let tag_clip = tag_clip.expect("combined tag field should have a clipped viewport");
    let first_tag_y = frame.paint_plan.primitives.iter().find_map(|primitive| {
        if let PaintPrimitive::FillRect(fill) = primitive
            && (fill.rect.height() - 18.0).abs() < 0.01
            && fill.rect.min.y >= tag_clip.min.y
            && fill.rect.max.y <= tag_clip.max.y
        {
            return Some(fill.rect.min.y);
        }
        None
    });
    let first_tag_y = first_tag_y.expect("tag pill should paint in the tag field");
    let input_rect = frame.paint_plan.primitives.iter().find_map(|primitive| {
        if let PaintPrimitive::TextInput(input) = primitive {
            return Some(input.rect);
        }
        None
    });
    let input_rect = input_rect.expect("tag input should paint");

    assert!(input_rect.min.y > first_tag_y);
    assert!(input_rect.max.x <= tag_clip.max.x);
}

#[test]
fn folder_browser_metadata_tag_input_keeps_identity_when_wrapping_rows() {
    let browser = super::FolderBrowserState::load_default();
    let short_tags = vec![String::from("kick")];
    let crowded_tags = vec![
        String::from("test"),
        String::from("another"),
        String::from("cool-tag"),
    ];
    let short_frame = radiant::runtime::UiSurface::new(
        super::folder_browser::folder_browser_view(
            &browser,
            260.0,
            true,
            "wow",
            &[],
            None,
            "add tag",
            None,
            &[],
            &short_tags,
        )
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(260.0, 620.0)),
        &radiant::theme::ThemeTokens::default(),
    );
    let crowded_frame = radiant::runtime::UiSurface::new(
        super::folder_browser::folder_browser_view(
            &browser,
            260.0,
            true,
            "wow",
            &[],
            None,
            "add tag",
            None,
            &[],
            &crowded_tags,
        )
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(260.0, 620.0)),
        &radiant::theme::ThemeTokens::default(),
    );

    let short_input = text_input_widget_id(&short_frame).expect("short tag field input");
    let crowded_input = text_input_widget_id(&crowded_frame).expect("crowded tag field input");

    assert_eq!(short_input, crowded_input);
}

#[test]
fn folder_browser_metadata_tag_input_wraps_after_full_tag_row() {
    let browser = super::FolderBrowserState::load_default();
    let tags = vec![
        String::from("yay"),
        String::from("cool-tag"),
        String::from("thing"),
        String::from("potato"),
    ];
    let frame = radiant::runtime::UiSurface::new(
        super::folder_browser::folder_browser_view(
            &browser,
            450.0,
            true,
            "",
            &[],
            None,
            "add tag",
            None,
            &[],
            &tags,
        )
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(450.0, 620.0)),
        &radiant::theme::ThemeTokens::default(),
    );

    let tag_clip = frame.paint_plan.primitives.iter().find_map(|primitive| {
        if let PaintPrimitive::ClipStart(clip) = primitive
            && clip.rect.height() >= 45.0
            && clip.rect.height() <= 48.0
        {
            return Some(clip.rect);
        }
        None
    });
    let tag_clip = tag_clip.expect("tag field should grow to at least two rows");
    let first_tag_y = frame.paint_plan.primitives.iter().find_map(|primitive| {
        if let PaintPrimitive::FillRect(fill) = primitive
            && (fill.rect.height() - 18.0).abs() < 0.01
            && fill.rect.min.y >= tag_clip.min.y
            && fill.rect.max.y <= tag_clip.max.y
        {
            return Some(fill.rect.min.y);
        }
        None
    });
    let first_tag_y = first_tag_y.expect("tag pill should paint in the tag field");
    let input_rect = frame.paint_plan.primitives.iter().find_map(|primitive| {
        if let PaintPrimitive::TextInput(input) = primitive {
            return Some(input.rect);
        }
        None
    });
    let input_rect = input_rect.expect("tag input should paint");

    assert!(input_rect.min.y > first_tag_y);
    assert!(input_rect.max.x <= tag_clip.max.x);
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
fn folder_context_menu_paints_as_full_width_overlay_panel() {
    let menu = super::BrowserContextMenu {
        kind: super::BrowserContextTargetKind::Folder,
        path: PathBuf::from("Documents"),
        source_id: None,
        anchor: Point::new(72.0, 142.0),
        title: String::from("Documents"),
    };
    let frame = radiant::runtime::UiSurface::new(super::context_menu::overlay(&menu).into_node())
        .frame(
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 540.0)),
            &radiant::theme::ThemeTokens::default(),
        );

    let action_text_rect = frame
        .paint_plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::Text(text) if text.text.as_str() == "Open in Explorer" => {
                Some(text.rect)
            }
            _ => None,
        })
        .expect("folder context menu action text should render");

    assert!(action_text_rect.width() > 150.0, "{action_text_rect:?}");
    assert!(
        action_text_rect.min.x >= 80.0 && action_text_rect.min.x < 100.0,
        "{action_text_rect:?}"
    );
}

#[test]
fn folder_context_menu_outside_click_closes_menu() {
    let menu = super::BrowserContextMenu {
        kind: super::BrowserContextTargetKind::Folder,
        path: PathBuf::from("Documents"),
        source_id: None,
        anchor: Point::new(72.0, 142.0),
        title: String::from("Documents"),
    };
    let bridge = DeclarativeOwnedRuntimeBridge::new(
        true,
        move |open| {
            if *open {
                radiant::runtime::UiSurface::new(super::context_menu::overlay(&menu).into_node())
            } else {
                radiant::runtime::UiSurface::new(ui::text("").into_node())
            }
        },
        |open, message| {
            if matches!(message, super::GuiMessage::CloseContextMenu) {
                *open = false;
            }
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(960.0, 540.0));
    let outside_menu = Point::new(18.0, 18.0);

    runtime.dispatch_event(Event::PointerPress {
        position: outside_menu,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });
    runtime.dispatch_event(Event::PointerRelease {
        position: outside_menu,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });

    assert!(
        !*runtime.bridge().state(),
        "clicking outside the context menu should route to the dismiss layer"
    );
}

#[test]
fn source_context_menu_paints_remove_source_action_for_user_sources() {
    let menu = super::BrowserContextMenu {
        kind: super::BrowserContextTargetKind::Source,
        path: PathBuf::from("C:\\Samples"),
        source_id: Some(String::from("source_id::samples")),
        anchor: Point::new(72.0, 142.0),
        title: String::from("Samples"),
    };
    let frame = radiant::runtime::UiSurface::new(super::context_menu::overlay(&menu).into_node())
        .frame(
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 540.0)),
            &radiant::theme::ThemeTokens::default(),
        );

    assert!(frame_has_text(&frame, "Remove Source"));
}

#[test]
fn folder_context_menu_open_does_not_toggle_folder_expansion() {
    let root = std::env::temp_dir().join(format!(
        "wavecrate-context-menu-right-click-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let parent = root.join("drums");
    fs::create_dir_all(parent.join("kicks")).expect("create nested folder");

    let mut state = gui_state_for_span_tests();
    let request = state
        .folder_browser
        .begin_add_source_path(root.clone(), 100)
        .expect("new source should request scan");
    let result = super::folder_browser::scan_source_with_progress(request, |_| {}, |_| {});
    state.finish_folder_scan(result);
    let (folder_id, expanded_before) = state
        .folder_browser
        .first_visible_child_folder_expansion_for_tests()
        .expect("test source should contain a child folder");

    state.open_folder_context_menu(folder_id.clone(), Point::new(40.0, 120.0));

    let expanded_after = state
        .folder_browser
        .folder_expansion_for_tests(&folder_id)
        .expect("context-menu target should remain visible");
    assert_eq!(
        expanded_after, expanded_before,
        "right-click context menu should not expand or collapse folders"
    );
    let _ = fs::remove_dir_all(root);
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
