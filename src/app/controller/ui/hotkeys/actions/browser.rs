use super::super::types::{HotkeyAction, HotkeyGesture, HotkeyScope, KeyPress};
use crate::app::state::FocusContext;
use crate::app_core::actions::NativeUiAction;
use crate::gui::input::KeyCode as Key;

const SAMPLE_BROWSER: HotkeyScope = HotkeyScope::Focus(FocusContext::SampleBrowser);

pub(super) const SEARCH_BROWSER: HotkeyAction = HotkeyAction {
    id: "search-browser",
    label: "Search samples",
    gesture: HotkeyGesture::with_command(Key::F),
    scope: SAMPLE_BROWSER,
    action: NativeUiAction::FocusBrowserSearch,
};
pub(super) const FOCUS_LOADED_SAMPLE: HotkeyAction = HotkeyAction {
    id: "focus-loaded-sample",
    label: "Focus loaded sample",
    gesture: HotkeyGesture::new(Key::F),
    scope: SAMPLE_BROWSER,
    action: NativeUiAction::FocusLoadedSampleInBrowser,
};
pub(super) const COPY_BROWSER_SELECTION: HotkeyAction = HotkeyAction {
    id: "copy-browser-selection",
    label: "Copy sample file(s)",
    gesture: HotkeyGesture::with_command(Key::C),
    scope: SAMPLE_BROWSER,
    action: NativeUiAction::CopySelectionToClipboard,
};
pub(super) const SET_COMPARE_ANCHOR: HotkeyAction = HotkeyAction {
    id: "set-compare-anchor",
    label: "Set compare anchor",
    gesture: HotkeyGesture::new(Key::C),
    scope: SAMPLE_BROWSER,
    action: NativeUiAction::SetCompareAnchorFromFocusedBrowserSample,
};
pub(super) const FIND_SIMILAR: HotkeyAction = HotkeyAction {
    id: "find-similar",
    label: "Toggle find similar",
    gesture: HotkeyGesture::new(Key::S),
    scope: SAMPLE_BROWSER,
    action: NativeUiAction::ToggleFindSimilarFocusedSample,
};
pub(super) const TOGGLE_RANDOM_NAVIGATION_MODE: HotkeyAction = HotkeyAction {
    id: "toggle-random-navigation-mode",
    label: "Toggle random navigation mode",
    gesture: HotkeyGesture::with_alt(Key::R),
    scope: SAMPLE_BROWSER,
    action: NativeUiAction::ToggleRandomNavigationMode,
};
pub(super) const PLAY_RANDOM_SAMPLE: HotkeyAction = HotkeyAction {
    id: "play-random-sample",
    label: "Play random sample",
    gesture: HotkeyGesture::with_shift(Key::R),
    scope: SAMPLE_BROWSER,
    action: NativeUiAction::PlayRandomSample,
};
pub(super) const PLAY_PREVIOUS_RANDOM_SAMPLE: HotkeyAction = HotkeyAction {
    id: "play-previous-random-sample",
    label: "Play previous random sample",
    gesture: HotkeyGesture {
        first: KeyPress {
            key: Key::R,
            command: true,
            shift: true,
            alt: false,
        },
        chord: None,
    },
    scope: SAMPLE_BROWSER,
    action: NativeUiAction::PlayPreviousRandomSample,
};
pub(super) const MOVE_TRASHED_TO_FOLDER: HotkeyAction = HotkeyAction {
    id: "move-trashed-to-folder",
    label: "Move trashed samples to folder",
    gesture: HotkeyGesture::new(Key::P),
    scope: SAMPLE_BROWSER,
    action: NativeUiAction::MoveTrashedSamplesToFolder,
};
pub(super) const MOVE_TRASHED_TO_FOLDER_SHIFT: HotkeyAction = HotkeyAction {
    id: "move-trashed-to-folder-shift",
    label: "Move trashed samples to folder",
    gesture: HotkeyGesture::with_shift(Key::P),
    scope: SAMPLE_BROWSER,
    action: NativeUiAction::MoveTrashedSamplesToFolder,
};
pub(super) const TOGGLE_SELECT: HotkeyAction = HotkeyAction {
    id: "toggle-select",
    label: "Toggle selection",
    gesture: HotkeyGesture::new(Key::X),
    scope: SAMPLE_BROWSER,
    action: NativeUiAction::ToggleFocusedBrowserRowSelection,
};
pub(super) const TOGGLE_BROWSER_SAMPLE_MARK: HotkeyAction = HotkeyAction {
    id: "toggle-browser-sample-mark",
    label: "Toggle sample mark",
    gesture: HotkeyGesture::new(Key::Semicolon),
    scope: SAMPLE_BROWSER,
    action: NativeUiAction::ToggleBrowserSampleMark,
};
pub(super) const MOVE_BROWSER_FOCUS_UP: HotkeyAction = HotkeyAction {
    id: "move-browser-focus-up",
    label: "Move focus up",
    gesture: HotkeyGesture::new(Key::ArrowUp),
    scope: SAMPLE_BROWSER,
    action: NativeUiAction::MoveBrowserFocus { delta: -1 },
};
pub(super) const MOVE_BROWSER_FOCUS_DOWN: HotkeyAction = HotkeyAction {
    id: "move-browser-focus-down",
    label: "Move focus down",
    gesture: HotkeyGesture::new(Key::ArrowDown),
    scope: SAMPLE_BROWSER,
    action: NativeUiAction::MoveBrowserFocus { delta: 1 },
};
pub(super) const FOCUS_HISTORY_PREVIOUS: HotkeyAction = HotkeyAction {
    id: "focus-history-previous",
    label: "Previous focused sample",
    gesture: HotkeyGesture::new(Key::ArrowLeft),
    scope: SAMPLE_BROWSER,
    action: NativeUiAction::FocusPreviousBrowserHistory,
};
pub(super) const FOCUS_HISTORY_NEXT: HotkeyAction = HotkeyAction {
    id: "focus-history-next",
    label: "Next focused sample",
    gesture: HotkeyGesture::new(Key::ArrowRight),
    scope: SAMPLE_BROWSER,
    action: NativeUiAction::FocusNextBrowserHistory,
};
pub(super) const RENAME_SAMPLE: HotkeyAction = HotkeyAction {
    id: "rename-sample",
    label: "Rename sample",
    gesture: HotkeyGesture::new(Key::R),
    scope: SAMPLE_BROWSER,
    action: NativeUiAction::StartBrowserRename,
};
pub(super) const SELECT_ALL: HotkeyAction = HotkeyAction {
    id: "select-all-browser",
    label: "Select all samples",
    gesture: HotkeyGesture::with_command(Key::A),
    scope: SAMPLE_BROWSER,
    action: NativeUiAction::SelectAllBrowserRows,
};
pub(super) const NORMALIZE_SAMPLE: HotkeyAction = HotkeyAction {
    id: "normalize-browser",
    label: "Normalize sample",
    gesture: HotkeyGesture::new(Key::N),
    scope: SAMPLE_BROWSER,
    action: NativeUiAction::NormalizeFocusedBrowserSample,
};
pub(super) const DELETE_SAMPLE: HotkeyAction = HotkeyAction {
    id: "delete-browser",
    label: "Delete sample",
    gesture: HotkeyGesture::new(Key::D),
    scope: SAMPLE_BROWSER,
    action: NativeUiAction::DeleteBrowserSelection,
};
