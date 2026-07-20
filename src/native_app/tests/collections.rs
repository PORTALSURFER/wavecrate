use super::{native_app_state_with_temp_sample, native_runtime_for_tests};
use crate::native_app::test_support::state::{FolderBrowserMessage, GuiMessage, NativeAppState};
use radiant::{gui::types::Point, prelude as ui};
use std::path::Path;

#[test]
fn collection_shortcut_toggles_selected_sample_membership() {
    let (mut state, source_root, selected_file) = native_app_state_with_temp_sample("toggle.wav");
    let collection = wavecrate::sample_sources::SampleCollection::new(0).expect("collection");
    let db = wavecrate::sample_sources::SourceDatabase::open_for_test_fixture_source_write(
        source_root.path(),
    )
    .expect("db");

    state.apply_message(
        GuiMessage::AssignSelectedCollection(collection),
        &mut ui::UiUpdateContext::default(),
    );

    assert_eq!(
        db.collections_for_path(std::path::Path::new("toggle.wav"))
            .expect("collections"),
        vec![collection]
    );
    assert!(
        state
            .library
            .folder_browser
            .selected_source_audio_files()
            .into_iter()
            .find(|file| file.id == selected_file)
            .expect("sample")
            .belongs_to_collection(collection)
    );

    state.apply_message(
        GuiMessage::AssignSelectedCollection(collection),
        &mut ui::UiUpdateContext::default(),
    );

    assert_eq!(
        db.collections_for_path(std::path::Path::new("toggle.wav"))
            .expect("collections"),
        Vec::<wavecrate::sample_sources::SampleCollection>::new()
    );
    assert!(
        !state
            .library
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
    let db = wavecrate::sample_sources::SourceDatabase::open_for_test_fixture_source_write(
        source_root.path(),
    )
    .expect("db");

    state.apply_message(
        GuiMessage::AssignSelectedCollection(collection),
        &mut ui::UiUpdateContext::default(),
    );
    assert_eq!(state.transactions.history.list_items().len(), 1);
    assert_eq!(
        db.collections_for_path(std::path::Path::new("undo-collection.wav"))
            .expect("collections"),
        vec![collection]
    );

    state.apply_message(
        GuiMessage::UndoTransaction,
        &mut ui::UiUpdateContext::default(),
    );
    assert_eq!(
        db.collections_for_path(std::path::Path::new("undo-collection.wav"))
            .expect("collections"),
        Vec::<wavecrate::sample_sources::SampleCollection>::new()
    );
    assert!(
        !state
            .library
            .folder_browser
            .selected_source_audio_files()
            .into_iter()
            .find(|file| file.id == selected_file)
            .expect("sample")
            .belongs_to_collection(collection)
    );

    state.apply_message(
        GuiMessage::RedoTransaction,
        &mut ui::UiUpdateContext::default(),
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
    let db = wavecrate::sample_sources::SourceDatabase::open_for_test_fixture_source_write(
        source_root.path(),
    )
    .expect("db");

    state.apply_message(
        GuiMessage::AssignSelectedCollection(collection),
        &mut ui::UiUpdateContext::default(),
    );
    state.apply_message(
        GuiMessage::FolderBrowser(FolderBrowserMessage::ActivateCollection(collection)),
        &mut ui::UiUpdateContext::default(),
    );
    state.open_sample_context_menu(selected_file, Point::new(12.0, 24.0));

    assert_eq!(
        state
            .ui
            .browser_interaction
            .context_menu
            .as_ref()
            .and_then(|menu| menu.collection),
        Some(collection)
    );

    state.apply_message(
        GuiMessage::RemoveContextSampleFromCollection,
        &mut ui::UiUpdateContext::default(),
    );

    assert_eq!(
        db.collections_for_path(std::path::Path::new("remove.wav"))
            .expect("collections"),
        Vec::<wavecrate::sample_sources::SampleCollection>::new()
    );
    assert!(
        state
            .library
            .folder_browser
            .selected_audio_files()
            .is_empty()
    );
    assert_eq!(state.ui.browser_interaction.context_menu, None);
    assert!(
        state
            .ui
            .status
            .sample
            .contains("Removed 1 sample from Collection 1")
    );
}

#[test]
fn sample_context_menu_cleans_missing_collection_member() {
    let source_root = tempfile::tempdir().expect("source root");
    let present = source_root.path().join("present.wav");
    std::fs::write(&present, []).expect("write present sample");
    let collection = wavecrate::sample_sources::SampleCollection::new(0).expect("collection");
    let db = wavecrate::sample_sources::SourceDatabase::open_for_test_fixture_source_write(
        source_root.path(),
    )
    .expect("db");
    seed_file_collections(&db, "missing/lost.wav", &[collection]);
    seed_file_collections(&db, "present.wav", &[collection]);
    let mut state = super::gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state.apply_message(
        GuiMessage::FolderBrowser(FolderBrowserMessage::ActivateCollection(collection)),
        &mut ui::UiUpdateContext::default(),
    );
    let missing_path = source_root.path().join("missing/lost.wav");
    state.open_sample_context_menu(
        missing_path.to_string_lossy().to_string(),
        Point::new(12.0, 24.0),
    );

    assert_eq!(
        state
            .ui
            .browser_interaction
            .context_menu
            .as_ref()
            .map(|menu| (menu.sample_missing, menu.collection)),
        Some((true, Some(collection)))
    );

    state.apply_message(
        GuiMessage::CleanMissingContextSampleFromCollection,
        &mut ui::UiUpdateContext::default(),
    );

    assert_eq!(
        db.collections_for_path(Path::new("missing/lost.wav"))
            .expect("missing collections"),
        Vec::<wavecrate::sample_sources::SampleCollection>::new()
    );
    assert_eq!(
        db.collections_for_path(Path::new("present.wav"))
            .expect("present collections"),
        vec![collection]
    );
    assert_eq!(
        state
            .library
            .folder_browser
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["present.wav"]
    );
    assert_eq!(state.ui.browser_interaction.context_menu, None);
    assert!(
        state
            .ui
            .status
            .sample
            .contains("Cleaned 1 missing sample from Collection 1")
    );
}

#[test]
fn sample_context_menu_cleans_all_missing_members_in_collection() {
    let source_root = tempfile::tempdir().expect("source root");
    let present = source_root.path().join("present.wav");
    std::fs::write(&present, []).expect("write present sample");
    let collection = wavecrate::sample_sources::SampleCollection::new(0).expect("collection");
    let db = wavecrate::sample_sources::SourceDatabase::open_for_test_fixture_source_write(
        source_root.path(),
    )
    .expect("db");
    seed_file_collections(&db, "missing/one.wav", &[collection]);
    seed_file_collections(&db, "missing/two.wav", &[collection]);
    seed_file_collections(&db, "present.wav", &[collection]);
    let mut state = super::gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state.apply_message(
        GuiMessage::FolderBrowser(FolderBrowserMessage::ActivateCollection(collection)),
        &mut ui::UiUpdateContext::default(),
    );
    state.open_sample_context_menu(
        source_root
            .path()
            .join("missing/one.wav")
            .to_string_lossy()
            .to_string(),
        Point::new(12.0, 24.0),
    );

    state.apply_message(
        GuiMessage::CleanMissingFilesFromActiveCollection,
        &mut ui::UiUpdateContext::default(),
    );

    assert_eq!(
        db.collections_for_path(Path::new("missing/one.wav"))
            .expect("first missing collections"),
        Vec::<wavecrate::sample_sources::SampleCollection>::new()
    );
    assert_eq!(
        db.collections_for_path(Path::new("missing/two.wav"))
            .expect("second missing collections"),
        Vec::<wavecrate::sample_sources::SampleCollection>::new()
    );
    assert_eq!(
        db.collections_for_path(Path::new("present.wav"))
            .expect("present collections"),
        vec![collection]
    );
    assert_eq!(
        state
            .library
            .folder_browser
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["present.wav"]
    );
    assert!(
        state
            .ui
            .status
            .sample
            .contains("Cleaned 2 missing samples from Collection 1")
    );
}

#[test]
fn collection_context_menu_cleans_all_missing_members_without_sample_context() {
    let source_root = tempfile::tempdir().expect("source root");
    let present = source_root.path().join("present.wav");
    std::fs::write(&present, []).expect("write present sample");
    let collection = wavecrate::sample_sources::SampleCollection::new(0).expect("collection");
    let db = wavecrate::sample_sources::SourceDatabase::open_for_test_fixture_source_write(
        source_root.path(),
    )
    .expect("db");
    seed_file_collections(&db, "missing/one.wav", &[collection]);
    seed_file_collections(&db, "missing/two.wav", &[collection]);
    seed_file_collections(&db, "present.wav", &[collection]);
    let mut state = super::gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);

    state.apply_message(
        GuiMessage::FolderBrowser(FolderBrowserMessage::OpenCollectionContextMenu(
            collection,
            Point::new(12.0, 24.0),
        )),
        &mut ui::UiUpdateContext::default(),
    );

    assert_eq!(
        state
            .ui
            .browser_interaction
            .context_menu
            .as_ref()
            .map(|menu| (menu.kind.clone(), menu.collection)),
        Some((
            crate::native_app::test_support::context_menu::BrowserContextTargetKind::Collection,
            Some(collection)
        ))
    );

    state.apply_message(
        GuiMessage::CleanMissingFilesFromActiveCollection,
        &mut ui::UiUpdateContext::default(),
    );

    assert_eq!(
        db.collections_for_path(Path::new("missing/one.wav"))
            .expect("first missing collections"),
        Vec::<wavecrate::sample_sources::SampleCollection>::new()
    );
    assert_eq!(
        db.collections_for_path(Path::new("missing/two.wav"))
            .expect("second missing collections"),
        Vec::<wavecrate::sample_sources::SampleCollection>::new()
    );
    assert_eq!(
        db.collections_for_path(Path::new("present.wav"))
            .expect("present collections"),
        vec![collection]
    );
    assert_eq!(state.ui.browser_interaction.context_menu, None);
    assert!(
        state
            .ui
            .status
            .sample
            .contains("Cleaned 2 missing samples from Collection 1")
    );
}

#[test]
fn collection_rename_input_selects_name_when_focused() {
    let collection = wavecrate::sample_sources::SampleCollection::new(0).expect("collection");
    let mut state = NativeAppState::load_default().expect("default state loads");
    let mut context = ui::UiUpdateContext::default();
    state.apply_message(
        GuiMessage::FolderBrowser(FolderBrowserMessage::RenameCollection(collection)),
        &mut context,
    );
    let rename = state
        .library
        .folder_browser
        .collection_rename_view(collection)
        .expect("collection rename view");
    let input_id = rename.input_id;

    let theme = radiant::theme::ThemeTokens::default();
    let mut runtime = native_runtime_for_tests(state, ui::Vector2::new(900.0, 620.0));
    runtime.frame(&theme);

    assert!(runtime.focus_widget(input_id));
    assert_eq!(
        runtime.focused_text_selection().as_deref(),
        Some("Collection 1")
    );
}

fn seed_file_collections(
    db: &wavecrate::sample_sources::SourceDatabase,
    relative_path: &str,
    collections: &[wavecrate::sample_sources::SampleCollection],
) {
    let path = Path::new(relative_path);
    db.upsert_file(path, 8, 1).expect("upsert source row");
    let mut batch = db.write_batch().expect("open write batch");
    for collection in collections {
        batch
            .add_collection(path, *collection)
            .expect("add collection membership");
    }
    batch.commit().expect("commit collection membership");
}

#[test]
fn collection_rename_persists_across_default_state_reload() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let collection = wavecrate::sample_sources::SampleCollection::new(0).expect("collection");
    let mut state = NativeAppState::load_default().expect("default state loads");
    let mut context = ui::UiUpdateContext::default();

    state.apply_message(
        GuiMessage::FolderBrowser(FolderBrowserMessage::RenameCollection(collection)),
        &mut context,
    );
    state.apply_message(
        GuiMessage::FolderBrowser(FolderBrowserMessage::RenameInput(
            radiant::widgets::TextInputMessage::Submitted {
                value: String::from("Drums"),
            },
        )),
        &mut context,
    );

    let config = wavecrate::sample_sources::config::load_or_default().expect("saved config");
    assert_eq!(
        config.core.collection_names.get("0").map(String::as_str),
        Some("Drums")
    );

    let reloaded = NativeAppState::load_default().expect("default state reloads");
    assert_eq!(
        reloaded
            .library
            .folder_browser
            .visible_collections()
            .into_iter()
            .find(|entry| entry.collection == collection)
            .map(|entry| entry.name),
        Some(String::from("Drums"))
    );
}
