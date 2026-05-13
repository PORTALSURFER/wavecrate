use super::super::types::{HotkeyAction, HotkeyGesture, HotkeyScope};
use crate::app::state::FocusContext;
use crate::app_core::actions::NativeUiAction;
use crate::gui::input::KeyCode as Key;

const SOURCE_FOLDERS: HotkeyScope = HotkeyScope::Focus(FocusContext::SourceFolders);

pub(super) const TOGGLE_SELECT: HotkeyAction = HotkeyAction {
    id: "toggle-folder-select",
    label: "Toggle folder selection",
    gesture: HotkeyGesture::new(Key::X),
    scope: SOURCE_FOLDERS,
    action: NativeUiAction::ToggleFocusedFolderSelection,
};
pub(super) const MOVE_FOLDER_FOCUS_UP: HotkeyAction = HotkeyAction {
    id: "move-folder-focus-up",
    label: "Move focus up",
    gesture: HotkeyGesture::new(Key::ArrowUp),
    scope: SOURCE_FOLDERS,
    action: NativeUiAction::MoveFolderFocus { delta: -1 },
};
pub(super) const MOVE_FOLDER_FOCUS_DOWN: HotkeyAction = HotkeyAction {
    id: "move-folder-focus-down",
    label: "Move focus down",
    gesture: HotkeyGesture::new(Key::ArrowDown),
    scope: SOURCE_FOLDERS,
    action: NativeUiAction::MoveFolderFocus { delta: 1 },
};
pub(super) const COLLAPSE_FOCUSED_FOLDER: HotkeyAction = HotkeyAction {
    id: "collapse-focused-folder",
    label: "Collapse folder",
    gesture: HotkeyGesture::new(Key::ArrowLeft),
    scope: SOURCE_FOLDERS,
    action: NativeUiAction::CollapseFocusedFolder,
};
pub(super) const EXPAND_FOCUSED_FOLDER: HotkeyAction = HotkeyAction {
    id: "expand-focused-folder",
    label: "Expand folder",
    gesture: HotkeyGesture::new(Key::ArrowRight),
    scope: SOURCE_FOLDERS,
    action: NativeUiAction::ExpandFocusedFolder,
};
pub(super) const DELETE_FOLDER: HotkeyAction = HotkeyAction {
    id: "delete-folder",
    label: "Delete folder",
    gesture: HotkeyGesture::new(Key::D),
    scope: SOURCE_FOLDERS,
    action: NativeUiAction::DeleteFocusedFolder,
};
pub(super) const RENAME_FOLDER: HotkeyAction = HotkeyAction {
    id: "rename-folder",
    label: "Rename folder",
    gesture: HotkeyGesture::new(Key::F2),
    scope: SOURCE_FOLDERS,
    action: NativeUiAction::StartFolderRename,
};
pub(super) const RENAME_FOLDER_LEGACY: HotkeyAction = HotkeyAction {
    id: "rename-folder-r",
    label: "Rename folder",
    gesture: HotkeyGesture::new(Key::R),
    scope: SOURCE_FOLDERS,
    action: NativeUiAction::StartFolderRename,
};
pub(super) const CREATE_FOLDER: HotkeyAction = HotkeyAction {
    id: "new-folder",
    label: "New folder",
    gesture: HotkeyGesture::new(Key::N),
    scope: SOURCE_FOLDERS,
    action: NativeUiAction::StartNewFolder,
};
pub(super) const FOCUS_SEARCH: HotkeyAction = HotkeyAction {
    id: "search-folders",
    label: "Search folders",
    gesture: HotkeyGesture::with_command(Key::F),
    scope: SOURCE_FOLDERS,
    action: NativeUiAction::FocusFolderSearch,
};
