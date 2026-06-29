use crate::native_app::test_support::state::{
    FolderBrowserMessage, FolderBrowserState, GuiMessage, NativeAppState, NativeAppStateFixture,
    default_gui_shortcuts,
};
use radiant::{gui::types::Point, prelude as ui};

#[test]
fn escape_shortcut_cancels_rename_while_renaming() {
    let (mut state, _source_root) = state_with_renamable_temp_sample("escape-rename.wav");
    state
        .library
        .folder_browser
        .begin_rename_selected()
        .expect("begin rename should not fail");

    let resolution = default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::Escape));

    assert_eq!(
        resolution.action,
        Some(GuiMessage::FolderBrowser(
            FolderBrowserMessage::CancelRename
        ))
    );
    assert!(resolution.handled);
}

fn state_with_renamable_temp_sample(name: &str) -> (NativeAppState, tempfile::TempDir) {
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join(name);
    std::fs::write(&sample_path, []).expect("sample file");
    let folder_browser =
        FolderBrowserState::from_sample_sources(&[wavecrate::sample_sources::SampleSource::new(
            source_root.path().to_path_buf(),
        )]);
    let mut state = NativeAppStateFixture::default()
        .with_folder_browser(folder_browser)
        .build();
    state
        .library
        .folder_browser
        .select_file(sample_path.display().to_string());
    (state, source_root)
}

#[test]
fn escape_shortcut_cancels_file_column_drag() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state
        .library
        .folder_browser
        .apply_message(FolderBrowserMessage::DragFileColumn(
            String::from("rating"),
            radiant::widgets::DragHandleMessage::started(Point::new(284.0, 0.0)),
        ));
    state
        .library
        .folder_browser
        .apply_message(FolderBrowserMessage::DragFileColumn(
            String::from("rating"),
            radiant::widgets::DragHandleMessage::moved(Point::new(560.0, 0.0)),
        ));

    let resolution = default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::Escape));

    assert_eq!(
        resolution.action,
        Some(GuiMessage::FolderBrowser(
            FolderBrowserMessage::CancelFileColumnDrag
        ))
    );
    assert!(resolution.handled);
}

#[test]
fn audio_settings_window_does_not_block_folder_creation_shortcut() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.ui.settings.ui.audio_settings_open = true;

    let resolution = default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::N));

    assert!(matches!(
        resolution.action,
        Some(GuiMessage::FolderBrowser(
            FolderBrowserMessage::BeginCreateSubfolder
        ))
    ));
    assert!(resolution.handled);
}

#[test]
fn command_arrow_navigation_preserves_folder_selection() {
    let state = NativeAppState::load_default().expect("default state loads");

    let resolution = default_gui_shortcuts(&state).resolve(ui::KeyPress {
        key: ui::KeyCode::ArrowDown,
        command: true,
        control: false,
        shift: false,
        alt: false,
    });

    assert_eq!(
        resolution.action,
        Some(GuiMessage::NavigateBrowser {
            delta: 1,
            extend: false,
            preserve_selection: true,
        })
    );
    assert!(resolution.handled);
}

#[test]
fn shift_arrow_navigation_extends_folder_selection() {
    let state = NativeAppState::load_default().expect("default state loads");

    let resolution =
        default_gui_shortcuts(&state).resolve(ui::KeyPress::with_shift(ui::KeyCode::ArrowDown));

    assert_eq!(
        resolution.action,
        Some(GuiMessage::NavigateBrowser {
            delta: 1,
            extend: true,
            preserve_selection: false,
        })
    );
    assert!(resolution.handled);
}

#[test]
fn f2_shortcut_renames_active_collection() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    let collection = wavecrate::sample_sources::SampleCollection::new(0).expect("collection");
    state
        .library
        .folder_browser
        .apply_message(FolderBrowserMessage::ActivateCollection(collection));

    let resolution = default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::F2));

    assert_eq!(
        resolution.action,
        Some(GuiMessage::FolderBrowser(
            FolderBrowserMessage::BeginRenameSelected
        ))
    );
    assert!(resolution.handled);

    state.apply_message(
        resolution.action.expect("F2 action"),
        &mut ui::UiUpdateContext::default(),
    );

    assert!(
        state
            .library
            .folder_browser
            .collection_rename_view(collection)
            .is_some()
    );
    assert_eq!(state.ui.status.sample, "Renaming selected collection");
}

#[test]
fn command_r_shortcut_renames_active_collection() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    let collection = wavecrate::sample_sources::SampleCollection::new(0).expect("collection");
    state
        .library
        .folder_browser
        .apply_message(FolderBrowserMessage::ActivateCollection(collection));

    let resolution =
        default_gui_shortcuts(&state).resolve(ui::KeyPress::with_command(ui::KeyCode::R));

    assert_eq!(
        resolution.action,
        Some(GuiMessage::FolderBrowser(
            FolderBrowserMessage::BeginRenameSelected
        ))
    );
    assert!(resolution.handled);

    state.apply_message(
        resolution.action.expect("Command-R action"),
        &mut ui::UiUpdateContext::default(),
    );

    assert!(
        state
            .library
            .folder_browser
            .collection_rename_view(collection)
            .is_some()
    );
    assert_eq!(state.ui.status.sample, "Renaming selected collection");
}

#[test]
fn arrow_down_shortcut_moves_active_collection_focus() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    let first = wavecrate::sample_sources::SampleCollection::new(0).expect("collection");
    let second = wavecrate::sample_sources::SampleCollection::new(1).expect("collection");
    state
        .library
        .folder_browser
        .apply_message(FolderBrowserMessage::ActivateCollection(first));

    let resolution =
        default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::ArrowDown));

    assert_eq!(
        resolution.action,
        Some(GuiMessage::NavigateBrowser {
            delta: 1,
            extend: false,
            preserve_selection: false,
        })
    );
    assert!(resolution.handled);

    state.apply_message(
        resolution.action.expect("ArrowDown action"),
        &mut ui::UiUpdateContext::default(),
    );

    let selected = state
        .library
        .folder_browser
        .visible_collections()
        .into_iter()
        .find(|collection| collection.selected)
        .map(|collection| collection.collection);
    assert_eq!(selected, Some(second));
    assert_eq!(state.library.folder_browser.selected_file_id(), None);
}
