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
mod config_sources;
mod context_menu;
mod metadata_tag_tests;
mod native_file_drop;
mod sample_browser;
mod shortcuts_context;
mod status_bar;
mod toolbar_playback;
mod waveform_playback;

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
