use super::super::types::{HotkeyAction, HotkeyGesture, HotkeyScope};
use crate::app::state::FocusContext;
use crate::app_core::actions::NativeUiAction;
use radiant::gui::input::KeyCode as Key;

const SOURCES: HotkeyScope = HotkeyScope::Focus(FocusContext::SourcesList);

pub(super) const MOVE_SOURCE_FOCUS_UP: HotkeyAction = HotkeyAction {
    id: "move-source-focus-up",
    label: "Previous source",
    gesture: HotkeyGesture::new(Key::ArrowUp),
    scope: SOURCES,
    action: NativeUiAction::SourcesAndFolders(
        crate::app_core::actions::NativeSourcesFoldersAction::MoveSourceFocus { delta: -1 },
    ),
};
pub(super) const MOVE_SOURCE_FOCUS_DOWN: HotkeyAction = HotkeyAction {
    id: "move-source-focus-down",
    label: "Next source",
    gesture: HotkeyGesture::new(Key::ArrowDown),
    scope: SOURCES,
    action: NativeUiAction::SourcesAndFolders(
        crate::app_core::actions::NativeSourcesFoldersAction::MoveSourceFocus { delta: 1 },
    ),
};
pub(super) const RELOAD_FOCUSED_SOURCE: HotkeyAction = HotkeyAction {
    id: "reload-focused-source",
    label: "Reload source",
    gesture: HotkeyGesture::new(Key::R),
    scope: SOURCES,
    action: NativeUiAction::SourcesAndFolders(
        crate::app_core::actions::NativeSourcesFoldersAction::ReloadFocusedSourceRow,
    ),
};
pub(super) const HARD_SYNC_FOCUSED_SOURCE: HotkeyAction = HotkeyAction {
    id: "hard-sync-focused-source",
    label: "Hard sync source",
    gesture: HotkeyGesture::new(Key::H),
    scope: SOURCES,
    action: NativeUiAction::SourcesAndFolders(
        crate::app_core::actions::NativeSourcesFoldersAction::HardSyncFocusedSourceRow,
    ),
};
pub(super) const OPEN_FOCUSED_SOURCE_FOLDER: HotkeyAction = HotkeyAction {
    id: "open-focused-source-folder",
    label: "Open source folder",
    gesture: HotkeyGesture::new(Key::O),
    scope: SOURCES,
    action: NativeUiAction::SourcesAndFolders(
        crate::app_core::actions::NativeSourcesFoldersAction::OpenFocusedSourceFolder,
    ),
};
pub(super) const REMOVE_FOCUSED_SOURCE: HotkeyAction = HotkeyAction {
    id: "remove-focused-source",
    label: "Remove source",
    gesture: HotkeyGesture::new(Key::D),
    scope: SOURCES,
    action: NativeUiAction::SourcesAndFolders(
        crate::app_core::actions::NativeSourcesFoldersAction::RemoveFocusedSourceRow,
    ),
};
