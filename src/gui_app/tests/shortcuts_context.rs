use super::gui_state_for_span_tests;
use crate::gui_app::{
    DEBUG_LAYOUT_ARG, DEBUG_LAYOUT_SHORT_ARG, GuiAppState, debug_layout_requested,
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
    let state = GuiAppState::load_default().expect("default state loads");
    let resolution = crate::gui_app::default_gui_shortcut_resolution(
        &state,
        ui::KeyPress::new(ui::KeyCode::Escape),
    );

    assert_eq!(
        resolution.action,
        Some(crate::gui_app::GuiMessage::StopPlayback)
    );
    assert!(resolution.handled);
}

#[test]
fn escape_shortcut_cancels_rename_while_renaming() {
    let mut state = GuiAppState::load_default().expect("default state loads");
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

    let resolution = crate::gui_app::default_gui_shortcut_resolution(
        &state,
        ui::KeyPress::new(ui::KeyCode::Escape),
    );

    assert_eq!(
        resolution.action,
        Some(crate::gui_app::GuiMessage::FolderBrowser(
            crate::gui_app::FolderBrowserMessage::CancelRename
        ))
    );
    assert!(resolution.handled);
}

#[test]
fn audio_settings_window_does_not_capture_main_escape_shortcut() {
    let mut state = GuiAppState::load_default().expect("default state loads");
    state.audio_settings_open = true;

    let resolution = crate::gui_app::default_gui_shortcut_resolution(
        &state,
        ui::KeyPress::new(ui::KeyCode::Escape),
    );

    assert_eq!(
        resolution.action,
        Some(crate::gui_app::GuiMessage::StopPlayback)
    );
    assert!(resolution.handled);
}

#[test]
fn audio_settings_window_does_not_block_main_shortcuts() {
    let mut state = GuiAppState::load_default().expect("default state loads");
    state.audio_settings_open = true;

    let resolution =
        crate::gui_app::default_gui_shortcut_resolution(&state, ui::KeyPress::new(ui::KeyCode::N));

    assert!(matches!(
        resolution.action,
        Some(crate::gui_app::GuiMessage::FolderBrowser(
            crate::gui_app::FolderBrowserMessage::BeginCreateSubfolder
        ))
    ));
    assert!(resolution.handled);
}

#[test]
fn context_menu_escape_shortcut_closes_context_menu() {
    let mut state = GuiAppState::load_default().expect("default state loads");
    state.context_menu = Some(crate::gui_app::BrowserContextMenu {
        kind: crate::gui_app::BrowserContextTargetKind::Sample,
        path: std::path::PathBuf::from("C:\\samples\\kick.wav"),
        source_id: None,
        metadata_tag: None,
        anchor: Point::new(12.0, 24.0),
        title: String::from("kick.wav"),
    });

    let resolution = crate::gui_app::default_gui_shortcut_resolution(
        &state,
        ui::KeyPress::new(ui::KeyCode::Escape),
    );

    assert_eq!(
        resolution.action,
        Some(crate::gui_app::GuiMessage::CloseContextMenu)
    );
    assert!(resolution.handled);
}

#[test]
fn audio_backend_dropdown_escape_shortcut_closes_dropdown() {
    let mut state = gui_state_for_span_tests();
    state.audio_backend_dropdown_open = true;

    let resolution = crate::gui_app::default_gui_shortcut_resolution(
        &state,
        ui::KeyPress::new(ui::KeyCode::Escape),
    );

    assert_eq!(
        resolution.action,
        Some(crate::gui_app::GuiMessage::CloseAudioSettingsDropdowns)
    );
    assert!(resolution.handled);
}

#[test]
fn format_copy_path_uses_forward_slashes_and_quotes_spaces() {
    assert_eq!(
        crate::gui_app::format_copy_path(std::path::Path::new("C:\\sample folder\\kick.wav")),
        "\"C:/sample folder/kick.wav\""
    );
    assert_eq!(
        crate::gui_app::format_copy_path(std::path::Path::new("C:\\samples\\kick.wav")),
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

    assert!(crate::gui_app::context_menu::target_available(
        &crate::gui_app::BrowserContextTargetKind::Source,
        &root
    ));
    assert!(crate::gui_app::context_menu::target_available(
        &crate::gui_app::BrowserContextTargetKind::Folder,
        &root
    ));
    assert!(crate::gui_app::context_menu::target_available(
        &crate::gui_app::BrowserContextTargetKind::Sample,
        &sample
    ));
    assert!(!crate::gui_app::context_menu::target_available(
        &crate::gui_app::BrowserContextTargetKind::Sample,
        &root
    ));
    assert!(!crate::gui_app::context_menu::target_available(
        &crate::gui_app::BrowserContextTargetKind::Folder,
        &sample
    ));

    std::fs::remove_file(&sample).expect("remove sample");
    assert!(!crate::gui_app::context_menu::target_available(
        &crate::gui_app::BrowserContextTargetKind::Sample,
        &sample
    ));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn stale_context_menu_copy_path_refuses_missing_sample_file() {
    let mut state = GuiAppState::load_default().expect("default state loads");
    state.context_menu = Some(crate::gui_app::BrowserContextMenu {
        kind: crate::gui_app::BrowserContextTargetKind::Sample,
        path: std::env::temp_dir().join("wavecrate-missing-context-sample.wav"),
        source_id: None,
        metadata_tag: None,
        anchor: Point::new(12.0, 24.0),
        title: String::from("missing.wav"),
    });

    state.copy_context_path();

    assert_eq!(state.sample_status, "Sample file is missing");
    assert_eq!(state.context_menu, None);
}

#[test]
fn copy_shortcut_routes_to_browser_file_handoff() {
    let state = GuiAppState::load_default().expect("default state loads");
    let resolution = crate::gui_app::default_gui_shortcut_resolution(
        &state,
        ui::KeyPress::with_command(ui::KeyCode::C),
    );

    assert_eq!(
        resolution.action,
        Some(crate::gui_app::GuiMessage::CopySelectedFiles)
    );
    assert!(resolution.handled);
}

#[test]
fn backspace_shortcut_routes_to_delete_selected_item() {
    let state = GuiAppState::load_default().expect("default state loads");
    let resolution = crate::gui_app::default_gui_shortcut_resolution(
        &state,
        ui::KeyPress::new(ui::KeyCode::Backspace),
    );

    assert_eq!(
        resolution.action,
        Some(crate::gui_app::GuiMessage::DeleteSelectedItem)
    );
    assert!(resolution.handled);
}

#[test]
fn delete_shortcut_removes_selected_metadata_tag_before_deleting_files() {
    let mut state = GuiAppState::load_default().expect("default state loads");
    state.selected_metadata_tag = Some(String::from("bass"));

    let resolution = crate::gui_app::default_gui_shortcut_resolution(
        &state,
        ui::KeyPress::new(ui::KeyCode::Delete),
    );

    assert_eq!(
        resolution.action,
        Some(crate::gui_app::GuiMessage::DeleteSelectedMetadataTag)
    );
    assert!(resolution.handled);
}

#[test]
fn loop_shortcut_routes_to_loop_toggle() {
    let state = GuiAppState::load_default().expect("default state loads");
    let resolution =
        crate::gui_app::default_gui_shortcut_resolution(&state, ui::KeyPress::new(ui::KeyCode::L));

    assert_eq!(
        resolution.action,
        Some(crate::gui_app::GuiMessage::ToggleLoopPlayback)
    );
    assert!(resolution.handled);
}

#[test]
fn bracket_shortcuts_route_to_rating_adjustments() {
    let state = GuiAppState::load_default().expect("default state loads");

    let down = crate::gui_app::default_gui_shortcut_resolution(
        &state,
        ui::KeyPress::new(ui::KeyCode::OpenBracket),
    );
    let up = crate::gui_app::default_gui_shortcut_resolution(
        &state,
        ui::KeyPress::new(ui::KeyCode::CloseBracket),
    );

    assert_eq!(
        down.action,
        Some(crate::gui_app::GuiMessage::AdjustSelectedRating(-1))
    );
    assert_eq!(
        up.action,
        Some(crate::gui_app::GuiMessage::AdjustSelectedRating(1))
    );
    assert!(down.handled);
    assert!(up.handled);
}
