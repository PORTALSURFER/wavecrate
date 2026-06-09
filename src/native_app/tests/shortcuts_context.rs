use super::gui_state_for_span_tests;
use crate::native_app::test_support::{
    DEBUG_LAYOUT_ARG, DEBUG_LAYOUT_SHORT_ARG, NativeAppState, debug_layout_requested,
};
use radiant::{gui::types::Point, prelude as ui};
use std::ffi::OsString;

#[test]
fn canonical_debug_layout_arg_enables_default_gui_overlay() {
    assert!(debug_layout_requested([
        OsString::from("wavecrate"),
        OsString::from(DEBUG_LAYOUT_ARG),
    ]));
}

#[test]
fn short_debug_layout_arg_enables_default_gui_overlay() {
    assert!(debug_layout_requested([
        OsString::from("wavecrate"),
        OsString::from(DEBUG_LAYOUT_SHORT_ARG),
    ]));
}

#[test]
fn unrelated_args_leave_default_gui_overlay_disabled() {
    assert!(!debug_layout_requested([
        OsString::from("wavecrate"),
        OsString::from("--debug-log"),
    ]));
}

#[test]
fn escape_shortcut_routes_to_stop_playback() {
    let state = NativeAppState::load_default().expect("default state loads");
    let resolution = crate::native_app::test_support::default_gui_shortcuts(&state)
        .resolve(ui::KeyPress::new(ui::KeyCode::Escape));

    assert_eq!(
        resolution.action,
        Some(crate::native_app::test_support::GuiMessage::StopPlayback)
    );
    assert!(resolution.handled);
}

#[test]
fn escape_shortcut_cancels_rename_while_renaming() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    let sample_path = state
        .folder_browser
        .selected_audio_files()
        .first()
        .expect("default assets include an audio sample")
        .id
        .clone();
    state.folder_browser.select_file(sample_path);
    state
        .folder_browser
        .begin_rename_selected()
        .expect("begin rename should not fail");

    let resolution = crate::native_app::test_support::default_gui_shortcuts(&state)
        .resolve(ui::KeyPress::new(ui::KeyCode::Escape));

    assert_eq!(
        resolution.action,
        Some(crate::native_app::test_support::GuiMessage::FolderBrowser(
            crate::native_app::test_support::FolderBrowserMessage::CancelRename
        ))
    );
    assert!(resolution.handled);
}

#[test]
fn escape_shortcut_cancels_file_column_drag() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.folder_browser.apply_message(
        crate::native_app::test_support::FolderBrowserMessage::DragFileColumn(
            String::from("rating"),
            radiant::widgets::DragHandleMessage::started(Point::new(284.0, 0.0)),
        ),
    );
    state.folder_browser.apply_message(
        crate::native_app::test_support::FolderBrowserMessage::DragFileColumn(
            String::from("rating"),
            radiant::widgets::DragHandleMessage::moved(Point::new(560.0, 0.0)),
        ),
    );

    let resolution = crate::native_app::test_support::default_gui_shortcuts(&state)
        .resolve(ui::KeyPress::new(ui::KeyCode::Escape));

    assert_eq!(
        resolution.action,
        Some(crate::native_app::test_support::GuiMessage::FolderBrowser(
            crate::native_app::test_support::FolderBrowserMessage::CancelFileColumnDrag
        ))
    );
    assert!(resolution.handled);
}

#[test]
fn audio_settings_window_does_not_capture_main_escape_shortcut() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.settings_ui.audio_settings_open = true;

    let resolution = crate::native_app::test_support::default_gui_shortcuts(&state)
        .resolve(ui::KeyPress::new(ui::KeyCode::Escape));

    assert_eq!(
        resolution.action,
        Some(crate::native_app::test_support::GuiMessage::StopPlayback)
    );
    assert!(resolution.handled);
}

#[test]
fn audio_settings_window_does_not_block_main_shortcuts() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.settings_ui.audio_settings_open = true;

    let resolution = crate::native_app::test_support::default_gui_shortcuts(&state)
        .resolve(ui::KeyPress::new(ui::KeyCode::N));

    assert!(matches!(
        resolution.action,
        Some(crate::native_app::test_support::GuiMessage::FolderBrowser(
            crate::native_app::test_support::FolderBrowserMessage::BeginCreateSubfolder
        ))
    ));
    assert!(resolution.handled);
}

#[test]
fn context_menu_escape_shortcut_closes_context_menu() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.browser_interaction.context_menu =
        Some(crate::native_app::test_support::BrowserContextMenu {
            kind: crate::native_app::test_support::BrowserContextTargetKind::Sample,
            path: std::path::PathBuf::from("C:\\samples\\kick.wav"),
            source_id: None,
            source_removable: false,
            metadata_tag: None,
            collection: None,
            anchor: Point::new(12.0, 24.0),
            title: String::from("kick.wav"),
        });

    let resolution = crate::native_app::test_support::default_gui_shortcuts(&state)
        .resolve(ui::KeyPress::new(ui::KeyCode::Escape));

    assert_eq!(
        resolution.action,
        Some(crate::native_app::test_support::GuiMessage::CloseContextMenu)
    );
    assert!(resolution.handled);
}

#[test]
fn metadata_tag_category_escape_shortcut_cancels_tag_entry() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.metadata.tag_input_mode =
        crate::native_app::test_support::MetadataTagInputMode::Category {
            pending_tag: String::from("deep-kick"),
        };

    let resolution = crate::native_app::test_support::default_gui_shortcuts(&state)
        .resolve(ui::KeyPress::new(ui::KeyCode::Escape));

    assert_eq!(
        resolution.action,
        Some(crate::native_app::test_support::GuiMessage::CancelMetadataTagEntry)
    );
    assert!(resolution.handled);
}

#[test]
fn audio_backend_dropdown_escape_shortcut_closes_dropdown() {
    let mut state = gui_state_for_span_tests();
    state
        .settings_ui
        .audio_settings_dropdown
        .open(crate::native_app::test_support::AudioSettingsDropdown::Backend);

    let resolution = crate::native_app::test_support::default_gui_shortcuts(&state)
        .resolve(ui::KeyPress::new(ui::KeyCode::Escape));

    assert_eq!(
        resolution.action,
        Some(crate::native_app::test_support::GuiMessage::CloseAudioSettingsDropdowns)
    );
    assert!(resolution.handled);
}

#[test]
fn format_copy_path_uses_forward_slashes_and_quotes_spaces() {
    assert_eq!(
        crate::native_app::test_support::format_copy_path(std::path::Path::new(
            "C:\\sample folder\\kick.wav"
        )),
        "\"C:/sample folder/kick.wav\""
    );
    assert_eq!(
        crate::native_app::test_support::format_copy_path(std::path::Path::new(
            "C:\\samples\\kick.wav"
        )),
        "C:/samples/kick.wav"
    );
}

#[test]
fn context_menu_availability_requires_existing_target_kind() {
    let root = std::env::temp_dir().join(format!(
        "wavecrate-context-menu-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).expect("create temp root");
    let sample = root.join("kick.wav");
    std::fs::write(&sample, [0_u8; 8]).expect("write sample");

    assert!(
        crate::native_app::sample_library::context_menu_target::target_available(
            &crate::native_app::test_support::BrowserContextTargetKind::Source,
            &root
        )
    );
    assert!(
        crate::native_app::sample_library::context_menu_target::target_available(
            &crate::native_app::test_support::BrowserContextTargetKind::Folder,
            &root
        )
    );
    assert!(
        crate::native_app::sample_library::context_menu_target::target_available(
            &crate::native_app::test_support::BrowserContextTargetKind::Sample,
            &sample
        )
    );
    assert!(
        !crate::native_app::sample_library::context_menu_target::target_available(
            &crate::native_app::test_support::BrowserContextTargetKind::Sample,
            &root
        )
    );
    assert!(
        !crate::native_app::sample_library::context_menu_target::target_available(
            &crate::native_app::test_support::BrowserContextTargetKind::Folder,
            &sample
        )
    );

    std::fs::remove_file(&sample).expect("remove sample");
    assert!(
        !crate::native_app::sample_library::context_menu_target::target_available(
            &crate::native_app::test_support::BrowserContextTargetKind::Sample,
            &sample
        )
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn stale_context_menu_copy_path_refuses_missing_sample_file() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.browser_interaction.context_menu =
        Some(crate::native_app::test_support::BrowserContextMenu {
            kind: crate::native_app::test_support::BrowserContextTargetKind::Sample,
            path: std::env::temp_dir().join("wavecrate-missing-context-sample.wav"),
            source_id: None,
            source_removable: false,
            metadata_tag: None,
            collection: None,
            anchor: Point::new(12.0, 24.0),
            title: String::from("missing.wav"),
        });

    let mut context = ui::UpdateContext::default();
    state.copy_context_path(&mut context);

    assert_eq!(state.sample_status, "Sample file is missing");
    assert_eq!(state.browser_interaction.context_menu, None);
}

#[test]
fn context_path_copy_completion_updates_status() {
    let mut state = NativeAppState::load_default().expect("default state loads");

    state.finish_context_path_copy(
        crate::native_app::test_support::BrowserContextTargetKind::Sample,
        std::path::PathBuf::from("C:\\samples\\kick.wav"),
        Ok(ui::PlatformResponse::Completed),
    );
    assert_eq!(state.sample_status, "Copied path");

    state.finish_context_path_copy(
        crate::native_app::test_support::BrowserContextTargetKind::Sample,
        std::path::PathBuf::from("C:\\samples\\kick.wav"),
        Err(String::from("clipboard unavailable")),
    );
    assert_eq!(
        state.sample_status,
        "Copy path failed: clipboard unavailable"
    );
}

#[test]
fn context_target_open_completion_updates_status() {
    let mut state = NativeAppState::load_default().expect("default state loads");

    state.finish_context_target_open(
        crate::native_app::test_support::BrowserContextTargetKind::Sample,
        std::path::PathBuf::from("C:\\samples\\kick.wav"),
        Ok(ui::PlatformResponse::Completed),
    );
    assert_eq!(state.sample_status, "Revealed sample");

    state.finish_context_target_open(
        crate::native_app::test_support::BrowserContextTargetKind::Folder,
        std::path::PathBuf::from("C:\\samples"),
        Err(String::from("shell unavailable")),
    );
    assert_eq!(state.sample_status, "shell unavailable");
}

#[test]
fn copy_shortcut_routes_to_browser_file_handoff() {
    let state = NativeAppState::load_default().expect("default state loads");
    let resolution = crate::native_app::test_support::default_gui_shortcuts(&state)
        .resolve(ui::KeyPress::with_command(ui::KeyCode::C));

    assert_eq!(
        resolution.action,
        Some(crate::native_app::test_support::GuiMessage::CopySelectedFiles)
    );
    assert!(resolution.handled);
}

#[test]
fn backspace_shortcut_routes_to_delete_selected_item() {
    let state = NativeAppState::load_default().expect("default state loads");
    let resolution = crate::native_app::test_support::default_gui_shortcuts(&state)
        .resolve(ui::KeyPress::new(ui::KeyCode::Backspace));

    assert_eq!(
        resolution.action,
        Some(crate::native_app::test_support::GuiMessage::DeleteSelectedItem)
    );
    assert!(resolution.handled);
}

#[test]
fn delete_shortcut_removes_selected_metadata_tag_before_deleting_files() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.metadata.selected_tag = Some(String::from("bass"));

    let resolution = crate::native_app::test_support::default_gui_shortcuts(&state)
        .resolve(ui::KeyPress::new(ui::KeyCode::Delete));

    assert_eq!(
        resolution.action,
        Some(crate::native_app::test_support::GuiMessage::DeleteSelectedMetadataTag)
    );
    assert!(resolution.handled);
}

#[test]
fn loop_shortcut_routes_to_loop_toggle() {
    let state = NativeAppState::load_default().expect("default state loads");
    let resolution = crate::native_app::test_support::default_gui_shortcuts(&state)
        .resolve(ui::KeyPress::new(ui::KeyCode::L));

    assert_eq!(
        resolution.action,
        Some(crate::native_app::test_support::GuiMessage::ToggleLoopPlayback)
    );
    assert!(resolution.handled);
}

#[test]
fn x_shortcut_routes_to_toggle_selected_sample_and_advance() {
    let state = NativeAppState::load_default().expect("default state loads");
    let resolution = crate::native_app::test_support::default_gui_shortcuts(&state)
        .resolve(ui::KeyPress::new(ui::KeyCode::X));

    assert_eq!(
        resolution.action,
        Some(crate::native_app::test_support::GuiMessage::ToggleSelectedSampleAndAdvance)
    );
    assert!(resolution.handled);
}

#[test]
fn x_shortcut_is_consumed_while_renaming() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    let sample_path = state
        .folder_browser
        .selected_audio_files()
        .first()
        .expect("default assets include an audio sample")
        .id
        .clone();
    state.folder_browser.select_file(sample_path);
    state
        .folder_browser
        .begin_rename_selected()
        .expect("begin rename should not fail");

    let resolution = crate::native_app::test_support::default_gui_shortcuts(&state)
        .resolve(ui::KeyPress::new(ui::KeyCode::X));

    assert_eq!(resolution.action, None);
    assert!(resolution.handled);
}

#[test]
fn shift_u_shortcut_toggles_transaction_list() {
    let state = NativeAppState::load_default().expect("default state loads");
    let resolution = crate::native_app::test_support::default_gui_shortcuts(&state)
        .resolve(ui::KeyPress::with_shift(ui::KeyCode::U));

    assert_eq!(
        resolution.action,
        Some(crate::native_app::test_support::GuiMessage::ToggleTransactionList)
    );
    assert!(resolution.handled);
}

#[test]
fn command_undo_redo_shortcuts_route_to_transactions() {
    let state = NativeAppState::load_default().expect("default state loads");

    let undo = crate::native_app::test_support::default_gui_shortcuts(&state)
        .resolve(ui::KeyPress::with_command(ui::KeyCode::Z));
    let redo_shift_z =
        crate::native_app::test_support::default_gui_shortcuts(&state).resolve(ui::KeyPress {
            key: ui::KeyCode::Z,
            command: true,
            shift: true,
            alt: false,
        });
    let redo_y = crate::native_app::test_support::default_gui_shortcuts(&state)
        .resolve(ui::KeyPress::with_command(ui::KeyCode::Y));

    assert_eq!(
        undo.action,
        Some(crate::native_app::test_support::GuiMessage::UndoTransaction)
    );
    assert_eq!(
        redo_shift_z.action,
        Some(crate::native_app::test_support::GuiMessage::RedoTransaction)
    );
    assert_eq!(
        redo_y.action,
        Some(crate::native_app::test_support::GuiMessage::RedoTransaction)
    );
    assert!(undo.handled);
    assert!(redo_shift_z.handled);
    assert!(redo_y.handled);
}

#[test]
fn bracket_shortcuts_route_to_rating_adjustments() {
    let state = NativeAppState::load_default().expect("default state loads");

    let down = crate::native_app::test_support::default_gui_shortcuts(&state)
        .resolve(ui::KeyPress::new(ui::KeyCode::OpenBracket));
    let up = crate::native_app::test_support::default_gui_shortcuts(&state)
        .resolve(ui::KeyPress::new(ui::KeyCode::CloseBracket));

    assert_eq!(
        down.action,
        Some(crate::native_app::test_support::GuiMessage::AdjustSelectedRating(-1))
    );
    assert_eq!(
        up.action,
        Some(crate::native_app::test_support::GuiMessage::AdjustSelectedRating(1))
    );
    assert!(down.handled);
    assert!(up.handled);
}
