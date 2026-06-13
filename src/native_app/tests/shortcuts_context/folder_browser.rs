use crate::native_app::test_support::state::{
    FolderBrowserMessage, GuiMessage, NativeAppState, default_gui_shortcuts,
};
use radiant::{gui::types::Point, prelude as ui};

#[test]
fn escape_shortcut_cancels_rename_while_renaming() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    let sample_path = state
        .library
        .folder_browser
        .selected_audio_files()
        .first()
        .expect("default assets include an audio sample")
        .id
        .clone();
    state.library.folder_browser.select_file(sample_path);
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
