use super::test_support::{
    DEFAULT_FOLDER_WIDTH, MAX_FOLDER_WIDTH, MIN_FOLDER_WIDTH, NativeAppState, WaveformInteraction,
};
use super::waveform::{WaveformSelectionEdge, WaveformSelectionKind};
use radiant::{
    gui::types::{Point, Rect, Vector2},
    prelude::{self as ui, IntoView},
    runtime::{Event, PaintTextInput, TransientOverlayContext, UiSurface},
    widgets::{DragHandleMessage, PointerButton, PointerModifiers, WidgetInput, WidgetKey},
};
use std::{fs, path::PathBuf, time::Duration};

mod audio_settings_controls;
mod audio_settings_dropdowns;
mod browser_context_menu;
mod config_sources;
mod metadata_tag_tests;
mod native_file_drop;
mod sample_browser;
mod shortcuts_context;
mod status_bar;
mod toolbar_playback;
mod transactions;
mod waveform_playback;
mod window_chrome;

fn first_visible_asset_file_path(browser: &super::test_support::FolderBrowserState) -> String {
    browser
        .selected_audio_files()
        .first()
        .unwrap_or_else(|| panic!("expected at least one visible audio sample"))
        .id
        .clone()
}

fn gui_state_for_span_tests() -> NativeAppState {
    super::test_support::NativeAppStateFixture::default()
        .with_synthetic_waveform()
        .with_sample_status("")
        .build()
}

type NativeRuntimeForTests = ui::DeclarativeOwnedSurfaceRuntime<
    NativeAppState,
    super::test_support::GuiMessage,
    fn(&mut NativeAppState) -> UiSurface<super::test_support::GuiMessage>,
    fn(&mut NativeAppState, super::test_support::GuiMessage),
>;

fn native_runtime_for_tests(state: NativeAppState, viewport: Vector2) -> NativeRuntimeForTests {
    NativeRuntimeForTests::new_declarative_owned(
        state,
        viewport,
        project_gui_surface_for_tests,
        reduce_gui_message_for_tests,
    )
}

fn project_gui_surface_for_tests(
    state: &mut NativeAppState,
) -> UiSurface<super::test_support::GuiMessage> {
    super::test_support::view(state).into_surface()
}

fn reduce_gui_message_for_tests(
    state: &mut NativeAppState,
    message: super::test_support::GuiMessage,
) {
    state.apply_message(message, &mut ui::UpdateContext::default());
}

fn native_app_state_with_temp_sample(name: &str) -> (NativeAppState, tempfile::TempDir, String) {
    let mut state = gui_state_for_span_tests();
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join(name);
    fs::write(&sample_path, []).expect("sample file");
    state.folder_browser = super::test_support::FolderBrowserState::from_sample_sources(&[
        wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
    ]);
    let selected_file = sample_path.display().to_string();
    state.folder_browser.select_file(selected_file.clone());
    (state, source_root, selected_file)
}

#[test]
fn file_move_conflict_dialog_renders_resolution_choices() {
    let mut state = gui_state_for_span_tests();
    let source_root = tempfile::tempdir().expect("source root");
    let drums = source_root.path().join("drums");
    let loops = source_root.path().join("loops");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let source = drums.join("kick.wav");
    let destination = loops.join("kick.wav");
    fs::write(&source, b"source").expect("write source");
    fs::write(&destination, b"destination").expect("write destination");
    state.folder_browser = super::test_support::FolderBrowserState::from_sample_sources(&[
        wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
    ]);
    state
        .folder_browser
        .apply_message(super::test_support::FolderBrowserMessage::ActivateFolder(
            drums.display().to_string(),
        ));
    state
        .folder_browser
        .select_file(source.display().to_string());
    state
        .folder_browser
        .begin_file_drag(source.display().to_string(), Point::new(4.0, 8.0));
    state
        .folder_browser
        .drop_drag_on_folder(&loops.display().to_string())
        .expect("drop should park conflict");

    let frame = super::test_support::view(&mut state)
        .view_frame_at_size_with_default_theme(Vector2::new(900.0, 620.0));

    assert!(frame.paint_plan.contains_text("File Move Conflict"));
    assert!(frame.paint_plan.contains_text("Conflict 1 of 1"));
    assert!(frame.paint_plan.contains_text("kick.wav"));
    assert!(frame.paint_plan.contains_text("Overwrite"));
    assert!(frame.paint_plan.contains_text("Rename"));
    assert!(frame.paint_plan.contains_text("Skip"));
}

#[test]
fn delete_selected_file_moves_it_to_configured_trash_folder() {
    let mut state = gui_state_for_span_tests();
    let source_root = tempfile::tempdir().expect("source root");
    let trash_root = tempfile::tempdir().expect("trash root");
    let keep = source_root.path().join("keep.wav");
    let delete = source_root.path().join("delete.wav");
    fs::write(&keep, []).expect("write keep wav");
    fs::write(&delete, []).expect("write delete wav");
    state.persisted_settings.trash_folder = Some(trash_root.path().to_path_buf());
    state.folder_browser = super::test_support::FolderBrowserState::from_sample_sources(&[
        wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
    ]);
    state
        .folder_browser
        .select_file(delete.display().to_string());

    state.delete_selected_item();

    assert!(!delete.exists());
    assert!(trash_root.path().join("delete.wav").exists());
    assert!(keep.exists());
    assert_eq!(state.folder_browser.selected_file_id(), None);
    assert!(
        state
            .folder_browser
            .selected_audio_files()
            .iter()
            .any(|file| file.name == "keep.wav")
    );
    assert!(
        !state
            .folder_browser
            .selected_audio_files()
            .iter()
            .any(|file| file.name == "delete.wav")
    );
    assert!(state.sample_status.contains("Moved 1 file to trash"));
}

#[test]
fn delete_selected_folder_moves_it_to_configured_trash_folder() {
    let mut state = gui_state_for_span_tests();
    let source_root = tempfile::tempdir().expect("source root");
    let trash_root = tempfile::tempdir().expect("trash root");
    let drums = source_root.path().join("drums");
    let loops = source_root.path().join("loops");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    fs::write(drums.join("kick.wav"), []).expect("write kick wav");
    fs::write(loops.join("loop.wav"), []).expect("write loop wav");
    state.persisted_settings.trash_folder = Some(trash_root.path().to_path_buf());
    state.folder_browser = super::test_support::FolderBrowserState::from_sample_sources(&[
        wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
    ]);
    state
        .folder_browser
        .apply_message(super::test_support::FolderBrowserMessage::ActivateFolder(
            drums.display().to_string(),
        ));

    state.delete_selected_item();

    assert!(!drums.exists());
    assert!(trash_root.path().join("drums").join("kick.wav").exists());
    assert!(loops.exists());
    assert_eq!(state.folder_browser.selected_file_id(), None);
    state
        .folder_browser
        .apply_message(super::test_support::FolderBrowserMessage::ActivateFolder(
            loops.display().to_string(),
        ));
    assert!(
        state
            .folder_browser
            .selected_audio_files()
            .iter()
            .any(|file| file.name == "loop.wav")
    );
    assert!(state.sample_status.contains("Moved drums to trash"));
}

#[test]
fn delete_selected_file_requires_configured_trash_folder() {
    let (mut state, _source_root, selected_file) = native_app_state_with_temp_sample("blocked.wav");

    state.delete_selected_item();

    assert!(std::path::Path::new(&selected_file).exists());
    assert_eq!(
        state.folder_browser.selected_file_id(),
        Some(selected_file.as_str())
    );
    assert!(
        state
            .sample_status
            .contains("Set a trash folder in Settings > General"),
        "{}",
        state.sample_status
    );
}

#[test]
fn collection_shortcut_toggles_selected_sample_membership() {
    let (mut state, source_root, selected_file) = native_app_state_with_temp_sample("toggle.wav");
    let collection = wavecrate::sample_sources::SampleCollection::new(0).expect("collection");
    let db = wavecrate::sample_sources::SourceDatabase::open(source_root.path()).expect("db");

    state.apply_message(
        super::test_support::GuiMessage::AssignSelectedCollection(collection),
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
        super::test_support::GuiMessage::AssignSelectedCollection(collection),
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
    let (mut state, source_root, selected_file) =
        native_app_state_with_temp_sample("undo-collection.wav");
    let collection = wavecrate::sample_sources::SampleCollection::new(0).expect("collection");
    let db = wavecrate::sample_sources::SourceDatabase::open(source_root.path()).expect("db");

    state.apply_message(
        super::test_support::GuiMessage::AssignSelectedCollection(collection),
        &mut ui::UpdateContext::default(),
    );
    assert_eq!(state.transactions.history.list_items().len(), 1);
    assert_eq!(
        db.collections_for_path(std::path::Path::new("undo-collection.wav"))
            .expect("collections"),
        vec![collection]
    );

    state.apply_message(
        super::test_support::GuiMessage::UndoTransaction,
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
        super::test_support::GuiMessage::RedoTransaction,
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
    let (mut state, source_root, selected_file) = native_app_state_with_temp_sample("remove.wav");
    let collection = wavecrate::sample_sources::SampleCollection::new(0).expect("collection");
    let db = wavecrate::sample_sources::SourceDatabase::open(source_root.path()).expect("db");

    state.apply_message(
        super::test_support::GuiMessage::AssignSelectedCollection(collection),
        &mut ui::UpdateContext::default(),
    );
    state.apply_message(
        super::test_support::GuiMessage::FolderBrowser(
            super::test_support::FolderBrowserMessage::ActivateCollection(collection),
        ),
        &mut ui::UpdateContext::default(),
    );
    state.open_sample_context_menu(selected_file, Point::new(12.0, 24.0));

    assert_eq!(
        state
            .browser_interaction
            .context_menu
            .as_ref()
            .and_then(|menu| menu.collection),
        Some(collection)
    );

    state.apply_message(
        super::test_support::GuiMessage::RemoveContextSampleFromCollection,
        &mut ui::UpdateContext::default(),
    );

    assert_eq!(
        db.collections_for_path(std::path::Path::new("remove.wav"))
            .expect("collections"),
        Vec::<wavecrate::sample_sources::SampleCollection>::new()
    );
    assert!(state.folder_browser.selected_audio_files().is_empty());
    assert_eq!(state.browser_interaction.context_menu, None);
    assert!(
        state
            .sample_status
            .contains("Removed 1 sample from Collection 1")
    );
}

fn start_deferred_sample_load_for_tests(
    state: &mut NativeAppState,
    path: String,
    autoplay: bool,
    context: &mut ui::UpdateContext<super::test_support::GuiMessage>,
) {
    let ticket = state
        .background
        .deferred_sample_load_task
        .active()
        .expect("deferred sample load queued");
    state.apply_message(
        super::test_support::GuiMessage::DeferredSampleLoad {
            ticket,
            path,
            autoplay,
            check_cache: false,
            scheduled_at: std::time::Instant::now(),
        },
        context,
    );
}

#[test]
fn folder_browser_splitter_resizes_and_clamps_width() {
    let mut state = super::test_support::NativeAppStateFixture::default()
        .with_synthetic_waveform()
        .with_sample_status("")
        .build();
    state.resize_folder_browser(DragHandleMessage::started(Point::new(100.0, 0.0)));
    state.resize_folder_browser(DragHandleMessage::moved(Point::new(160.0, 0.0)));

    assert_eq!(
        state.chrome.folder_panel.size(),
        DEFAULT_FOLDER_WIDTH + 60.0
    );

    state.resize_folder_browser(DragHandleMessage::moved(Point::new(900.0, 0.0)));
    assert_eq!(state.chrome.folder_panel.size(), MAX_FOLDER_WIDTH);

    state.resize_folder_browser(DragHandleMessage::ended(Point::new(-900.0, 0.0)));
    assert_eq!(state.chrome.folder_panel.size(), MIN_FOLDER_WIDTH);
    assert!(!state.chrome.folder_panel.is_resizing());
}

#[test]
fn default_gui_starts_without_loading_a_sample() {
    let waveform =
        super::test_support::WaveformState::load_default().expect("default sample loads");
    assert!(!waveform.has_loaded_sample());
    assert_eq!(waveform.file_name(), "No sample loaded");
}

#[test]
fn collection_rename_input_selects_name_when_focused() {
    let collection = wavecrate::sample_sources::SampleCollection::new(0).expect("collection");
    let mut state = NativeAppState::load_default().expect("default state loads");
    let mut context = ui::UpdateContext::default();
    state.apply_message(
        super::test_support::GuiMessage::FolderBrowser(
            super::test_support::FolderBrowserMessage::RenameCollection(collection),
        ),
        &mut context,
    );
    let rename = state
        .folder_browser
        .collection_rename_view(collection)
        .expect("collection rename view");
    let input_id = rename.input_id;

    let theme = radiant::theme::ThemeTokens::default();
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
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
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.sample_status = String::from("ready");

    state.apply_message(
        super::test_support::GuiMessage::ClearRebuildableCaches,
        &mut ui::UpdateContext::default(),
    );

    assert!(!cache_payload.exists());
    assert!(handoff_payload.exists());
    assert_eq!(state.audio.settings_error, None);
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
    metadata_tag_text_input(frame).map(|input| input.widget_id)
}

fn metadata_tag_text_input(frame: &ui::SurfaceFrame) -> Option<&PaintTextInput> {
    frame.paint_plan.text_inputs().find(|input| {
        input.widget_id == crate::native_app::app_chrome::library_browser::folder_sidebar::METADATA_TAG_INPUT_ID
    })
}
