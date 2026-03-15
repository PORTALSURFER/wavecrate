use super::super::types::{HotkeyAction, HotkeyCommand, HotkeyGesture, HotkeyScope};
use crate::app::state::FocusContext;
use crate::gui::input::KeyCode as Key;

const SAMPLE_BROWSER: HotkeyScope = HotkeyScope::Focus(FocusContext::SampleBrowser);

pub(super) const TOGGLE_SELECT: HotkeyAction = HotkeyAction {
    id: "toggle-select",
    label: "Toggle selection",
    gesture: HotkeyGesture::new(Key::X),
    scope: SAMPLE_BROWSER,
    command: HotkeyCommand::ToggleFocusedSelection,
};

pub(super) const FOCUS_HISTORY_PREVIOUS: HotkeyAction = HotkeyAction {
    id: "focus-history-previous",
    label: "Previous focused sample",
    gesture: HotkeyGesture::new(Key::ArrowLeft),
    scope: SAMPLE_BROWSER,
    command: HotkeyCommand::FocusHistoryPrevious,
};

pub(super) const FOCUS_HISTORY_NEXT: HotkeyAction = HotkeyAction {
    id: "focus-history-next",
    label: "Next focused sample",
    gesture: HotkeyGesture::new(Key::ArrowRight),
    scope: SAMPLE_BROWSER,
    command: HotkeyCommand::FocusHistoryNext,
};

pub(super) const RENAME_SAMPLE: HotkeyAction = HotkeyAction {
    id: "rename-sample",
    label: "Rename sample",
    gesture: HotkeyGesture::new(Key::R),
    scope: SAMPLE_BROWSER,
    command: HotkeyCommand::RenameFocusedSample,
};

pub(super) const SELECT_ALL: HotkeyAction = HotkeyAction {
    id: "select-all-browser",
    label: "Select all samples",
    gesture: HotkeyGesture::with_command(Key::A),
    scope: SAMPLE_BROWSER,
    command: HotkeyCommand::SelectAllBrowser,
};

pub(super) const NORMALIZE_SAMPLE: HotkeyAction = HotkeyAction {
    id: "normalize-browser",
    label: "Normalize sample",
    gesture: HotkeyGesture::new(Key::N),
    scope: SAMPLE_BROWSER,
    command: HotkeyCommand::NormalizeFocusedSample,
};

pub(super) const DELETE_SAMPLE: HotkeyAction = HotkeyAction {
    id: "delete-browser",
    label: "Delete sample",
    gesture: HotkeyGesture::new(Key::D),
    scope: SAMPLE_BROWSER,
    command: HotkeyCommand::DeleteFocusedSample,
};

pub(super) const REVERSE_SELECTION: HotkeyAction = HotkeyAction {
    id: "reverse-selection-browser",
    label: "Reverse selection",
    gesture: HotkeyGesture::with_shift(Key::R),
    scope: SAMPLE_BROWSER,
    command: HotkeyCommand::ReverseSelection,
};
