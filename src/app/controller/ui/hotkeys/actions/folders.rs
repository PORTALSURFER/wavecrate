use super::super::types::{HotkeyAction, HotkeyCommand, HotkeyGesture, HotkeyScope};
use crate::app::state::FocusContext;
use crate::gui::input::KeyCode as Key;

const SOURCE_FOLDERS: HotkeyScope = HotkeyScope::Focus(FocusContext::SourceFolders);

pub(super) const TOGGLE_SELECT: HotkeyAction = HotkeyAction {
    id: "toggle-folder-select",
    label: "Toggle folder selection",
    gesture: HotkeyGesture::new(Key::X),
    scope: SOURCE_FOLDERS,
    command: HotkeyCommand::ToggleFolderSelection,
};

pub(super) const DELETE_FOLDER: HotkeyAction = HotkeyAction {
    id: "delete-folder",
    label: "Delete folder",
    gesture: HotkeyGesture::new(Key::D),
    scope: SOURCE_FOLDERS,
    command: HotkeyCommand::DeleteFocusedFolder,
};

pub(super) const RENAME_FOLDER: HotkeyAction = HotkeyAction {
    id: "rename-folder",
    label: "Rename folder",
    gesture: HotkeyGesture::new(Key::R),
    scope: SOURCE_FOLDERS,
    command: HotkeyCommand::RenameFocusedFolder,
};

pub(super) const CREATE_FOLDER: HotkeyAction = HotkeyAction {
    id: "new-folder",
    label: "New folder",
    gesture: HotkeyGesture::new(Key::N),
    scope: SOURCE_FOLDERS,
    command: HotkeyCommand::CreateFolder,
};

pub(super) const FOCUS_SEARCH: HotkeyAction = HotkeyAction {
    id: "search-folders",
    label: "Search folders",
    gesture: HotkeyGesture::with_command(Key::F),
    scope: SOURCE_FOLDERS,
    command: HotkeyCommand::FocusFolderSearch,
};
