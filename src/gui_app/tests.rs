use super::waveform::{WaveformSelectionEdge, WaveformSelectionKind};
use super::waveform_panel::waveform_loading_visual;
use super::{
    DEFAULT_FOLDER_WIDTH, GuiAppState, MAX_FOLDER_WIDTH, MIN_FOLDER_WIDTH, WaveformInteraction,
};
use radiant::{
    gui::types::{Point, Rect, Rgba8, Vector2},
    prelude::{self as ui, IntoView},
    runtime::{NativeFileDrop, PaintPrimitive},
    widgets::{DragHandleMessage, PointerButton, PointerModifiers, Widget, WidgetInput},
};
use std::{fs, path::PathBuf, sync::mpsc};

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
    }
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
        assert!(radiant::gui::svg::SvgIcon::from_svg(icon.svg()).is_some());
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

#[test]
fn native_file_hover_over_waveform_tracks_supported_state() {
    let root = temp_gui_root("wavecrate-native-file-hover");
    let wav = root.join("kick.wav");
    let txt = root.join("note.txt");
    write_test_wav_i16(&wav, &[0, 100]);
    fs::write(&txt, "not audio").expect("write text");
    let mut state = gui_state_for_span_tests();
    state.folder_browser = super::FolderBrowserState::from_sample_sources(&[
        wavecrate::sample_sources::SampleSource::new(root.clone()),
    ]);
    let mut context = ui::UpdateContext::default();

    state.apply_native_file_drop(
        NativeFileDrop::hover(
            wav.clone(),
            Some(Point::new(8.0, 8.0)),
            Some(super::WAVEFORM_WIDGET_ID),
        ),
        &mut context,
    );
    assert_eq!(
        state.native_file_drop_hover,
        Some(super::NativeFileDropHover {
            path: wav.clone(),
            supported: true,
        })
    );

    state.apply_native_file_drop(
        NativeFileDrop::hover(
            txt.clone(),
            Some(Point::new(8.0, 8.0)),
            Some(super::WAVEFORM_WIDGET_ID),
        ),
        &mut context,
    );
    assert_eq!(
        state.native_file_drop_hover,
        Some(super::NativeFileDropHover {
            path: txt,
            supported: false,
        })
    );

    state.apply_native_file_drop(
        NativeFileDrop::cancel(Some(Point::new(8.0, 8.0)), Some(super::WAVEFORM_WIDGET_ID)),
        &mut context,
    );
    assert_eq!(state.native_file_drop_hover, None);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn native_file_hover_without_widget_target_still_shows_waveform_drop_feedback() {
    let root = temp_gui_root("wavecrate-native-file-hover-targetless");
    let wav = root.join("kick.wav");
    write_test_wav_i16(&wav, &[0, 100]);
    let mut state = gui_state_for_span_tests();
    let mut context = ui::UpdateContext::default();

    state.apply_native_file_drop(
        NativeFileDrop::hover(wav.clone(), Some(Point::new(8.0, 8.0)), None),
        &mut context,
    );

    assert_eq!(
        state.native_file_drop_hover,
        Some(super::NativeFileDropHover {
            path: wav,
            supported: true,
        })
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn native_file_drop_on_waveform_copies_into_selected_folder_and_queues_load() {
    let root = temp_gui_root("wavecrate-native-file-drop-root");
    let external_root = temp_gui_root("wavecrate-native-file-drop-external");
    let loops = root.join("loops");
    fs::create_dir_all(&loops).expect("create loops");
    let source = external_root.join("kick.wav");
    write_test_wav_i16(&source, &[0, 100, -100]);
    let mut state = gui_state_for_span_tests();
    state.folder_browser = super::FolderBrowserState::from_sample_sources(&[
        wavecrate::sample_sources::SampleSource::new(root.clone()),
    ]);
    state
        .folder_browser
        .apply_message(super::FolderBrowserMessage::ActivateFolder(
            loops.display().to_string(),
        ));
    let mut context = ui::UpdateContext::default();

    state.apply_native_file_drop(
        NativeFileDrop::dropped(
            source,
            Some(Point::new(8.0, 8.0)),
            Some(super::WAVEFORM_WIDGET_ID),
        ),
        &mut context,
    );

    let copied = loops.join("kick.wav");
    let copied_id = copied.display().to_string();
    assert!(copied.is_file());
    assert_eq!(
        state.folder_browser.selected_file_id(),
        Some(copied_id.as_str())
    );
    assert_eq!(state.waveform_loading_label.as_deref(), Some("kick.wav"));
    assert!(state.sample_load_task.active().is_some());
    let _ = fs::remove_dir_all(root);
    let _ = fs::remove_dir_all(external_root);
}

#[test]
fn native_file_drop_without_widget_target_imports_into_selected_folder() {
    let root = temp_gui_root("wavecrate-native-file-drop-targetless-root");
    let external_root = temp_gui_root("wavecrate-native-file-drop-targetless-external");
    let loops = root.join("loops");
    fs::create_dir_all(&loops).expect("create loops");
    let source = external_root.join("kick.wav");
    write_test_wav_i16(&source, &[0, 100, -100]);
    let mut state = gui_state_for_span_tests();
    state.folder_browser = super::FolderBrowserState::from_sample_sources(&[
        wavecrate::sample_sources::SampleSource::new(root.clone()),
    ]);
    state
        .folder_browser
        .apply_message(super::FolderBrowserMessage::ActivateFolder(
            loops.display().to_string(),
        ));
    let mut context = ui::UpdateContext::default();

    state.apply_native_file_drop(
        NativeFileDrop::dropped(source, Some(Point::new(8.0, 8.0)), None),
        &mut context,
    );

    let copied = loops.join("kick.wav");
    let copied_id = copied.display().to_string();
    assert!(copied.is_file());
    assert_eq!(
        state.folder_browser.selected_file_id(),
        Some(copied_id.as_str())
    );
    let _ = fs::remove_dir_all(root);
    let _ = fs::remove_dir_all(external_root);
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
fn sample_row_hit_target_survives_frame_refresh_between_press_and_release() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(160.0, 22.0));
    let mut hit_target = super::sample_browser_view::SampleFileHitTarget::new(false);

    assert_eq!(
        hit_target.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(24.0, 10.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        ),
        None
    );

    let mut refreshed_hit_target = super::sample_browser_view::SampleFileHitTarget::new(false);
    refreshed_hit_target.common_mut().state = hit_target.common().state;
    let output = refreshed_hit_target
        .handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: Point::new(24.0, 10.0),
                button: PointerButton::Primary,
                modifiers: PointerModifiers {
                    command: true,
                    shift: true,
                    ..Default::default()
                },
            },
        )
        .expect("sample row should activate after a frame refresh");

    assert_eq!(
        output.typed_ref::<super::sample_browser_view::SampleFileHitMessage>(),
        Some(&super::sample_browser_view::SampleFileHitMessage::Activate(
            PointerModifiers {
                command: true,
                shift: true,
                ..Default::default()
            }
        ))
    );
    assert!(!refreshed_hit_target.common().state.pressed);
}

#[test]
fn top_status_bar_replaces_text_labels_with_volume_slider_and_audio_pill() {
    let mut state = GuiAppState::load_default().expect("default state loads");
    state.audio_output_resolved = Some(super::ResolvedOutput {
        host_id: String::from("wasapi"),
        device_name: String::from("Studio"),
        sample_rate: 48_000,
        buffer_size_frames: None,
        channel_count: 2,
        used_fallback: false,
    });
    let frame = radiant::runtime::UiSurface::new(super::top_status_bar(&state).into_node()).frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(320.0, 30.0)),
        &radiant::theme::ThemeTokens::default(),
    );
    let texts = frame
        .paint_plan
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::Text(text) => Some(text.text.as_str().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();
    let slider_fills = frame
        .paint_plan
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill)
                if fill.widget_id == super::VOLUME_SLIDER_ID
                    && fill.rect.width() > 0.0
                    && fill.rect.height() > 0.0 =>
            {
                Some(fill)
            }
            _ => None,
        })
        .count();

    assert!(!texts.iter().any(|text| text == "Wavecrate"));
    assert!(!texts.iter().any(|text| text == "Wavecrate GUI"));
    assert!(!texts.iter().any(|text| text == "ready"));
    assert!(texts.iter().any(|text| text == "Audio"), "{texts:?}");
    assert!(slider_fills >= 2, "expected track and fill rects");
}

#[test]
fn volume_slider_drag_emits_normalized_volume() {
    assert_eq!(
        radiant::runtime::UiSurface::new(super::audio_settings::volume_slider(0.25).into_node(),)
            .dispatch_widget_output(
                super::VOLUME_SLIDER_ID,
                radiant::widgets::WidgetOutput::typed(
                    radiant::widgets::SliderMessage::ValueChanged { value: 0.75 },
                ),
            ),
        Some(super::GuiMessage::SetVolume(0.75))
    );
}

#[test]
fn default_gui_volume_state_clamps() {
    let mut state = GuiAppState::load_default().expect("default state loads");

    state.set_volume(1.5);
    assert_eq!(state.volume, 1.0);

    state.set_volume(-0.5);
    assert_eq!(state.volume, 0.0);
}

#[test]
fn audio_engine_pill_activates_settings_toggle() {
    let mut pill = super::audio_settings::AudioEnginePill::new(String::from("48 kHz"), false);
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(66.0, 18.0));
    assert!(
        pill.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(24.0, 8.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        )
        .is_none()
    );
    let output = pill
        .handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: Point::new(24.0, 8.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        )
        .expect("audio pill should activate");

    assert_eq!(
        output.typed_ref::<super::GuiMessage>(),
        Some(&super::GuiMessage::ToggleAudioSettings)
    );
}

#[test]
fn audio_settings_snapshot_uses_cached_device_options() {
    let mut state = gui_state_for_span_tests();
    state.audio_hosts = vec![super::AudioHostSummary {
        id: String::from("cached-host"),
        label: String::from("Cached Host"),
        is_default: true,
    }];

    let snapshot = super::audio_settings::AudioSettingsSnapshot::from_app_state(&state);

    assert_eq!(snapshot.audio_hosts.len(), 1);
    assert_eq!(snapshot.audio_hosts[0].id, "cached-host");
}

#[test]
fn audio_engine_detail_distinguishes_selected_host_from_runtime_fallback() {
    let mut state = gui_state_for_span_tests();
    state.audio_output_config.host = Some(String::from("asio"));
    state.audio_hosts = vec![
        super::AudioHostSummary {
            id: String::from("wasapi"),
            label: String::from("WASAPI"),
            is_default: true,
        },
        super::AudioHostSummary {
            id: String::from("asio"),
            label: String::from("ASIO"),
            is_default: false,
        },
    ];
    state.audio_output_resolved = Some(super::ResolvedOutput {
        host_id: String::from("wasapi"),
        device_name: String::from("Studio"),
        sample_rate: 48_000,
        buffer_size_frames: None,
        channel_count: 2,
        used_fallback: true,
    });

    assert_eq!(
        state.audio_engine_detail_label(),
        "ASIO selected | using WASAPI | Studio | 48 kHz"
    );
}

#[test]
fn audio_sample_rate_label_matches_status_chip_format() {
    assert_eq!(super::format_sample_rate_label(48_000), "48 kHz");
    assert_eq!(super::format_sample_rate_label(44_100), "44.1 kHz");
    assert_eq!(super::format_sample_rate_label(960), "960 Hz");
}

#[test]
fn audio_settings_popover_stays_output_only() {
    let mut state = GuiAppState::load_default().expect("default state loads");
    state.audio_settings_error = None;
    state.audio_hosts = vec![super::AudioHostSummary {
        id: String::from("asio"),
        label: String::from("ASIO"),
        is_default: false,
    }];
    state.audio_devices = vec![super::AudioDeviceSummary {
        host_id: String::from("asio"),
        name: String::from("Studio Out"),
        is_default: true,
    }];
    state.audio_sample_rates = vec![44_100, 48_000];
    let frame = radiant::runtime::UiSurface::new(super::audio_settings_popover(&state).into_node())
        .frame(
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(480.0, 360.0)),
            &radiant::theme::ThemeTokens::default(),
        );
    let texts = frame
        .paint_plan
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::Text(text) => Some(text.text.as_str().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert!(
        !texts.iter().any(|text| text == "Audio Engine"),
        "{texts:?}"
    );
    assert!(texts.iter().any(|text| text == "Backend"), "{texts:?}");
    assert!(texts.iter().any(|text| text == "Output"), "{texts:?}");
    assert!(texts.iter().any(|text| text == "Sample Rate"), "{texts:?}");
    assert!(
        texts.iter().any(|text| text == "Clear Rebuildable Caches"),
        "{texts:?}"
    );
    assert!(
        !texts.iter().any(|text| text.contains("Input")),
        "{texts:?}"
    );
}

#[test]
fn audio_backend_dropdown_renders_expanded_host_options() {
    let mut state = gui_state_for_span_tests();
    state.audio_settings_error = None;
    state.audio_backend_dropdown_open = true;
    state.audio_hosts = vec![
        super::AudioHostSummary {
            id: String::from("wasapi"),
            label: String::from("WASAPI"),
            is_default: true,
        },
        super::AudioHostSummary {
            id: String::from("asio"),
            label: String::from("ASIO"),
            is_default: false,
        },
    ];
    let frame = radiant::runtime::UiSurface::new(super::audio_settings_popover(&state).into_node())
        .frame(
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(480.0, 360.0)),
            &radiant::theme::ThemeTokens::default(),
        );
    let texts = frame
        .paint_plan
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::Text(text) => Some(text.text.as_str().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert!(
        texts.iter().any(|text| text == "System default"),
        "{texts:?}"
    );
    assert!(
        texts.iter().any(|text| text == "WASAPI (default)"),
        "{texts:?}"
    );
    assert!(texts.iter().any(|text| text == "ASIO"), "{texts:?}");
}

#[test]
fn audio_output_dropdown_renders_expanded_device_options() {
    let mut state = gui_state_for_span_tests();
    state.audio_settings_error = None;
    state.audio_output_dropdown_open = true;
    state.audio_devices = vec![super::AudioDeviceSummary {
        host_id: String::from("asio"),
        name: String::from("Studio Out"),
        is_default: true,
    }];
    let frame = radiant::runtime::UiSurface::new(super::audio_settings_popover(&state).into_node())
        .frame(
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(480.0, 360.0)),
            &radiant::theme::ThemeTokens::default(),
        );
    let texts = frame
        .paint_plan
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::Text(text) => Some(text.text.as_str().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert!(texts.iter().any(|text| text == "Host default"), "{texts:?}");
    assert!(
        texts.iter().any(|text| text == "Studio Out (default)"),
        "{texts:?}"
    );
}

#[test]
fn audio_sample_rate_dropdown_renders_expanded_rate_options() {
    let mut state = gui_state_for_span_tests();
    state.audio_settings_error = None;
    state.audio_sample_rate_dropdown_open = true;
    state.audio_sample_rates = vec![44_100, 48_000];
    let frame = radiant::runtime::UiSurface::new(super::audio_settings_popover(&state).into_node())
        .frame(
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(480.0, 360.0)),
            &radiant::theme::ThemeTokens::default(),
        );
    let texts = frame
        .paint_plan
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::Text(text) => Some(text.text.as_str().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert!(
        texts.iter().any(|text| text == "Device default"),
        "{texts:?}"
    );
    assert!(texts.iter().any(|text| text == "44.1 kHz"), "{texts:?}");
    assert!(texts.iter().any(|text| text == "48 kHz"), "{texts:?}");
}

#[test]
fn audio_backend_dropdown_overlay_does_not_reflow_later_sections() {
    let mut state = gui_state_for_span_tests();
    state.audio_settings_error = None;
    state.audio_hosts = vec![
        super::AudioHostSummary {
            id: String::from("wasapi"),
            label: String::from("WASAPI"),
            is_default: true,
        },
        super::AudioHostSummary {
            id: String::from("asio"),
            label: String::from("ASIO"),
            is_default: false,
        },
    ];

    state.audio_backend_dropdown_open = false;
    let closed =
        radiant::runtime::UiSurface::new(super::audio_settings_popover(&state).into_node()).frame(
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(480.0, 360.0)),
            &radiant::theme::ThemeTokens::default(),
        );
    state.audio_backend_dropdown_open = true;
    let open = radiant::runtime::UiSurface::new(super::audio_settings_popover(&state).into_node())
        .frame(
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(480.0, 360.0)),
            &radiant::theme::ThemeTokens::default(),
        );

    let text_top = |frame: &radiant::runtime::SurfaceFrame, label: &str| {
        frame
            .paint_plan
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                PaintPrimitive::Text(text) if text.text.as_str() == label => Some(text.rect.min.y),
                _ => None,
            })
            .unwrap_or_else(|| panic!("expected text {label}"))
    };
    let text_index = |frame: &radiant::runtime::SurfaceFrame, label: &str| {
        frame
            .paint_plan
            .primitives
            .iter()
            .position(|primitive| match primitive {
                PaintPrimitive::Text(text) => text.text.as_str() == label,
                _ => false,
            })
            .unwrap_or_else(|| panic!("expected text {label}"))
    };

    assert_eq!(text_top(&closed, "Output"), text_top(&open, "Output"));
    assert_eq!(
        text_top(&closed, "Sample Rate"),
        text_top(&open, "Sample Rate")
    );
    assert!(text_top(&open, "WASAPI (default)") > text_top(&open, "Output"));
    assert!(text_index(&open, "WASAPI (default)") > text_index(&open, "Output"));
}

#[test]
fn audio_backend_dropdown_toggle_and_close_are_ui_only() {
    let mut state = gui_state_for_span_tests();

    state.apply_message(
        super::GuiMessage::ToggleAudioBackendDropdown,
        &mut ui::UpdateContext::default(),
    );
    assert!(state.audio_backend_dropdown_open);

    state.apply_message(
        super::GuiMessage::CloseAudioSettingsDropdowns,
        &mut ui::UpdateContext::default(),
    );
    assert!(!state.audio_backend_dropdown_open);

    state.apply_message(
        super::GuiMessage::ToggleAudioBackendDropdown,
        &mut ui::UpdateContext::default(),
    );
    assert!(state.audio_backend_dropdown_open);

    state.apply_message(
        super::GuiMessage::ToggleAudioOutputDropdown,
        &mut ui::UpdateContext::default(),
    );
    assert!(!state.audio_backend_dropdown_open);
    assert!(state.audio_output_dropdown_open);

    state.apply_message(
        super::GuiMessage::ToggleAudioSampleRateDropdown,
        &mut ui::UpdateContext::default(),
    );
    assert!(!state.audio_output_dropdown_open);
    assert!(state.audio_sample_rate_dropdown_open);

    state.apply_message(
        super::GuiMessage::CloseAudioSettingsDropdowns,
        &mut ui::UpdateContext::default(),
    );
    assert!(!state.audio_sample_rate_dropdown_open);

    state.apply_message(
        super::GuiMessage::CloseAudioSettings,
        &mut ui::UpdateContext::default(),
    );
    assert!(!state.audio_backend_dropdown_open);
    assert!(!state.audio_output_dropdown_open);
    assert!(!state.audio_sample_rate_dropdown_open);
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
fn sample_browser_frame_paints_column_and_file_text() {
    let mut state = GuiAppState::load_default().expect("default state loads");
    let surface = super::sample_browser(&mut state).into_node();
    let frame = radiant::runtime::UiSurface::new(surface).frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(720.0, 360.0)),
        &radiant::theme::ThemeTokens::default(),
    );
    let texts = frame
        .paint_plan
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::Text(text) => Some(text.text.as_str().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert!(
        texts.iter().any(|text| text.starts_with("Name")),
        "{texts:?}"
    );
    assert!(
        texts.iter().any(|text| text.starts_with("portal_SS_")),
        "{texts:?}"
    );
}

#[test]
fn sample_browser_rows_match_keyboard_scroll_stride() {
    let mut state = GuiAppState::load_default().expect("default state loads");
    let surface = super::sample_browser(&mut state).into_node();
    let frame = radiant::runtime::UiSurface::new(surface).frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(720.0, 360.0)),
        &radiant::theme::ThemeTokens::default(),
    );
    let mut row_tops = frame
        .paint_plan
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::Text(text) if text.text.as_str().starts_with("portal_SS_") => {
                Some(text.rect.min.y)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    row_tops.sort_by(|a, b| a.total_cmp(b));
    row_tops.dedup_by(|a, b| (*a - *b).abs() < 0.5);

    assert!(row_tops.len() >= 2, "{row_tops:?}");
    assert!(
        row_tops
            .windows(2)
            .all(|pair| ((pair[1] - pair[0]) - super::SAMPLE_BROWSER_ROW_HEIGHT).abs() < 0.5),
        "{row_tops:?}"
    );
}

#[test]
fn sample_browser_keyboard_scroll_keeps_two_context_rows() {
    assert_eq!(super::SAMPLE_BROWSER_EDGE_CONTEXT_ROWS, 2);
    assert_eq!(super::SAMPLE_BROWSER_ROW_HEIGHT, 22.0);
}

#[test]
fn selected_sample_browser_row_paints_strong_fill_and_left_marker() {
    let widget = super::sample_browser_view::SampleFileHitTarget::new(true);
    let bounds = Rect::from_min_size(Point::new(12.0, 8.0), Vector2::new(240.0, 22.0));
    let mut primitives = Vec::new();
    widget.append_paint(
        &mut primitives,
        bounds,
        &Default::default(),
        &radiant::theme::ThemeTokens::default(),
    );
    let fills = primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) => Some(fill),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert!(fills.iter().any(|fill| fill.rect == bounds
        && fill.color
            == Rgba8 {
                r: 255,
                g: 82,
                b: 62,
                a: 120,
            }));
    assert!(fills.iter().any(|fill| {
        fill.color
            == Rgba8 {
                r: 255,
                g: 82,
                b: 62,
                a: 245,
            }
            && fill.rect.width() <= 3.5
    }));
}

#[test]
fn sample_browser_row_hover_paints_bright_background_without_marker() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(180.0, 22.0));
    let mut hit_target = super::sample_browser_view::SampleFileHitTarget::new(false);

    assert_eq!(
        hit_target.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(20.0, 10.0),
            },
        ),
        None
    );

    let mut primitives = Vec::new();
    hit_target.append_paint(
        &mut primitives,
        bounds,
        &Default::default(),
        &radiant::theme::ThemeTokens::default(),
    );
    let fills = primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) => Some(fill),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(fills.len(), 1, "{fills:?}");
    assert_eq!(fills[0].rect, bounds);
    assert_eq!(
        fills[0].color,
        Rgba8 {
            r: 255,
            g: 108,
            b: 88,
            a: 155,
        }
    );
}

#[test]
fn full_gui_frame_places_sample_browser_text_inside_visible_area() {
    let mut state = GuiAppState::load_default().expect("default state loads");
    let surface = super::view(&mut state).into_node();
    let frame = radiant::runtime::UiSurface::new(surface).frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(1517.0, 758.0)),
        &radiant::theme::ThemeTokens::default(),
    );
    let sample_texts = frame
        .paint_plan
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::Text(text)
                if text.text.as_str() == "Name" || text.text.as_str().starts_with("portal_SS_") =>
            {
                Some((text.text.as_str().to_string(), text.rect, text.baseline))
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    assert!(!sample_texts.is_empty(), "{sample_texts:?}");
    assert!(
        sample_texts.iter().any(|(_, rect, baseline)| {
            rect.width() > 20.0
                && rect.height() >= 10.0
                && rect.min.x >= 280.0
                && rect.min.y >= 320.0
                && rect.max.y <= 730.0
                && baseline.is_some()
        }),
        "{sample_texts:?}"
    );
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
