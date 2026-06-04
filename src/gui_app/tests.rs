use super::waveform::{WaveformSelectionEdge, WaveformSelectionKind};
use super::{
    DEFAULT_FOLDER_WIDTH, GuiAppState, MAX_FOLDER_WIDTH, MIN_FOLDER_WIDTH, WaveformInteraction,
};
use radiant::{
    gui::types::{Point, Rect, Vector2},
    prelude::{self as ui, IntoView},
    runtime::{Event, TransientOverlayContext, UiSurface},
    widgets::{DragHandleMessage, PointerModifiers, WidgetInput, WidgetKey},
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
mod transactions;
mod waveform_playback;
mod window_chrome;

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
        folder_panel: ui::PanelResizeState::new(DEFAULT_FOLDER_WIDTH),
        folder_browser: super::FolderBrowserState::load_default(),
        waveform: super::WaveformState::synthetic_for_tests(),
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
        volume: super::DEFAULT_VOLUME,
        volume_persist_deadline: None,
        audio_output_config: super::AudioOutputConfig::default(),
        audio_output_resolved: None,
        audio_hosts: Vec::new(),
        audio_devices: Vec::new(),
        audio_sample_rates: Vec::new(),
        persisted_settings: super::AppSettingsCore::default(),
        audio_settings_open: false,
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
        sample_name_view_mode: super::SampleNameViewMode::DiskFilename,
        startup_source_scan_pending: false,
        startup_auto_load_pending: false,
        waveform_cache: HashMap::new(),
        waveform_cache_order: Default::default(),
        waveform_cache_bytes: 0,
        waveform_cache_warm_pending: Default::default(),
        waveform_cache_warm_task: ui::LatestTask::new(),
        waveform_cache_warm_results: Default::default(),
        cached_sample_paths: Default::default(),
    }
}

type GuiRuntimeForTests = ui::DeclarativeOwnedSurfaceRuntime<
    GuiAppState,
    super::GuiMessage,
    fn(&mut GuiAppState) -> UiSurface<super::GuiMessage>,
    fn(&mut GuiAppState, super::GuiMessage),
>;

fn gui_runtime_for_tests(state: GuiAppState, viewport: Vector2) -> GuiRuntimeForTests {
    GuiRuntimeForTests::new_declarative_owned(
        state,
        viewport,
        project_gui_surface_for_tests,
        reduce_gui_message_for_tests,
    )
}

fn project_gui_surface_for_tests(state: &mut GuiAppState) -> UiSurface<super::GuiMessage> {
    super::view(state).into_surface()
}

fn reduce_gui_message_for_tests(state: &mut GuiAppState, message: super::GuiMessage) {
    state.apply_message(message, &mut ui::UpdateContext::default());
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
fn collection_shortcut_toggles_selected_sample_membership() {
    let (mut state, source_root, selected_file) = gui_state_with_temp_sample("toggle.wav");
    let collection = wavecrate::sample_sources::SampleCollection::new(0).expect("collection");
    let db = wavecrate::sample_sources::SourceDatabase::open(source_root.path()).expect("db");

    state.apply_message(
        super::GuiMessage::AssignSelectedCollection(collection),
        &mut ui::UpdateContext::default(),
    );

    assert_eq!(
        db.collections_for_path(std::path::Path::new("toggle.wav"))
            .expect("collections"),
        vec![collection]
    );
    assert!(
        state
            .folder_browser
            .selected_source_audio_files()
            .into_iter()
            .find(|file| file.id == selected_file)
            .expect("sample")
            .belongs_to_collection(collection)
    );

    state.apply_message(
        super::GuiMessage::AssignSelectedCollection(collection),
        &mut ui::UpdateContext::default(),
    );

    assert_eq!(
        db.collections_for_path(std::path::Path::new("toggle.wav"))
            .expect("collections"),
        Vec::<wavecrate::sample_sources::SampleCollection>::new()
    );
    assert!(
        !state
            .folder_browser
            .selected_source_audio_files()
            .into_iter()
            .find(|file| file.id == selected_file)
            .expect("sample")
            .belongs_to_collection(collection)
    );
}

#[test]
fn collection_assignment_transaction_undoes_and_redoes_membership() {
    let (mut state, source_root, selected_file) = gui_state_with_temp_sample("undo-collection.wav");
    let collection = wavecrate::sample_sources::SampleCollection::new(0).expect("collection");
    let db = wavecrate::sample_sources::SourceDatabase::open(source_root.path()).expect("db");

    state.apply_message(
        super::GuiMessage::AssignSelectedCollection(collection),
        &mut ui::UpdateContext::default(),
    );
    assert_eq!(state.transaction_history.list_items().len(), 1);
    assert_eq!(
        db.collections_for_path(std::path::Path::new("undo-collection.wav"))
            .expect("collections"),
        vec![collection]
    );

    state.apply_message(
        super::GuiMessage::UndoTransaction,
        &mut ui::UpdateContext::default(),
    );
    assert_eq!(
        db.collections_for_path(std::path::Path::new("undo-collection.wav"))
            .expect("collections"),
        Vec::<wavecrate::sample_sources::SampleCollection>::new()
    );
    assert!(
        !state
            .folder_browser
            .selected_source_audio_files()
            .into_iter()
            .find(|file| file.id == selected_file)
            .expect("sample")
            .belongs_to_collection(collection)
    );

    state.apply_message(
        super::GuiMessage::RedoTransaction,
        &mut ui::UpdateContext::default(),
    );
    assert_eq!(
        db.collections_for_path(std::path::Path::new("undo-collection.wav"))
            .expect("collections"),
        vec![collection]
    );
}

#[test]
fn sample_context_menu_removes_item_from_active_collection_view() {
    let (mut state, source_root, selected_file) = gui_state_with_temp_sample("remove.wav");
    let collection = wavecrate::sample_sources::SampleCollection::new(0).expect("collection");
    let db = wavecrate::sample_sources::SourceDatabase::open(source_root.path()).expect("db");

    state.apply_message(
        super::GuiMessage::AssignSelectedCollection(collection),
        &mut ui::UpdateContext::default(),
    );
    state.apply_message(
        super::GuiMessage::FolderBrowser(super::FolderBrowserMessage::ActivateCollection(
            collection,
        )),
        &mut ui::UpdateContext::default(),
    );
    state.open_sample_context_menu(selected_file, Point::new(12.0, 24.0));

    assert_eq!(
        state.context_menu.as_ref().and_then(|menu| menu.collection),
        Some(collection)
    );

    state.apply_message(
        super::GuiMessage::RemoveContextSampleFromCollection,
        &mut ui::UpdateContext::default(),
    );

    assert_eq!(
        db.collections_for_path(std::path::Path::new("remove.wav"))
            .expect("collections"),
        Vec::<wavecrate::sample_sources::SampleCollection>::new()
    );
    assert!(state.folder_browser.selected_audio_files().is_empty());
    assert_eq!(state.context_menu, None);
    assert!(
        state
            .sample_status
            .contains("Removed 1 sample from Collection 1")
    );
}

fn start_deferred_sample_load_for_tests(
    state: &mut GuiAppState,
    path: String,
    autoplay: bool,
    context: &mut ui::UpdateContext<super::GuiMessage>,
) {
    let ticket = state
        .deferred_sample_load_task
        .active()
        .expect("deferred sample load queued");
    state.apply_message(
        super::GuiMessage::DeferredSampleLoad {
            ticket,
            path,
            autoplay,
            check_cache: false,
        },
        context,
    );
}

#[test]
fn folder_browser_splitter_resizes_and_clamps_width() {
    let mut state = GuiAppState {
        folder_panel: ui::PanelResizeState::new(DEFAULT_FOLDER_WIDTH),
        folder_browser: super::FolderBrowserState::load_default(),
        waveform: super::WaveformState::synthetic_for_tests(),
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
        volume: super::DEFAULT_VOLUME,
        volume_persist_deadline: None,
        audio_output_config: super::AudioOutputConfig::default(),
        audio_output_resolved: None,
        audio_hosts: Vec::new(),
        audio_devices: Vec::new(),
        audio_sample_rates: Vec::new(),
        persisted_settings: super::AppSettingsCore::default(),
        audio_settings_open: false,
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
        sample_name_view_mode: super::SampleNameViewMode::DiskFilename,
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
    state.resize_folder_browser(DragHandleMessage::Started {
        position: Point::new(100.0, 0.0),
    });
    state.resize_folder_browser(DragHandleMessage::Moved {
        position: Point::new(160.0, 0.0),
    });

    assert_eq!(state.folder_panel.size(), DEFAULT_FOLDER_WIDTH + 60.0);

    state.resize_folder_browser(DragHandleMessage::Moved {
        position: Point::new(900.0, 0.0),
    });
    assert_eq!(state.folder_panel.size(), MAX_FOLDER_WIDTH);

    state.resize_folder_browser(DragHandleMessage::Ended {
        position: Point::new(-900.0, 0.0),
    });
    assert_eq!(state.folder_panel.size(), MIN_FOLDER_WIDTH);
    assert!(!state.folder_panel.is_resizing());
}

#[test]
fn default_gui_starts_without_loading_a_sample() {
    let waveform = super::WaveformState::load_default().expect("default sample loads");
    assert!(!waveform.has_loaded_sample());
    assert_eq!(waveform.file_name(), "No sample loaded");
}

#[test]
fn collection_rename_input_selects_name_when_focused() {
    let collection = wavecrate::sample_sources::SampleCollection::new(0).expect("collection");
    let mut state = GuiAppState::load_default().expect("default state loads");
    let mut context = ui::UpdateContext::default();
    state.apply_message(
        super::GuiMessage::FolderBrowser(super::FolderBrowserMessage::RenameCollection(collection)),
        &mut context,
    );
    let rename = state
        .folder_browser
        .collection_rename_view(collection)
        .expect("collection rename view");
    let input_id = rename.input_id;

    let theme = radiant::theme::ThemeTokens::default();
    let mut runtime = gui_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    runtime.frame(&theme);

    assert!(runtime.focus_widget(input_id));
    assert_eq!(
        runtime.focused_text_selection().as_deref(),
        Some("Collection 1")
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

fn frame_has_clip_height(frame: &ui::SurfaceFrame, expected: f32) -> bool {
    frame
        .paint_plan
        .clip_starts()
        .any(|clip| (clip.rect.height() - expected).abs() < 0.01)
}

fn text_input_widget_id(frame: &ui::SurfaceFrame) -> Option<u64> {
    frame
        .paint_plan
        .first_text_input()
        .map(|input| input.widget_id)
}
