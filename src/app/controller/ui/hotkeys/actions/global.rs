use super::super::types::{HotkeyAction, HotkeyCommand, HotkeyGesture, HotkeyScope, KeyPress};
use crate::gui::input::KeyCode as Key;

const GLOBAL: HotkeyScope = HotkeyScope::Global;

pub(super) const UNDO_CTRL_Z: HotkeyAction = HotkeyAction {
    id: "undo-ctrl-z",
    label: "Undo",
    gesture: HotkeyGesture::with_command(Key::Z),
    scope: GLOBAL,
    command: HotkeyCommand::Undo,
};

pub(super) const UNDO_U: HotkeyAction = HotkeyAction {
    id: "undo-u",
    label: "Undo",
    gesture: HotkeyGesture::new(Key::U),
    scope: GLOBAL,
    command: HotkeyCommand::Undo,
};

pub(super) const REDO_CTRL_Y: HotkeyAction = HotkeyAction {
    id: "redo-ctrl-y",
    label: "Redo",
    gesture: HotkeyGesture::with_command(Key::Y),
    scope: GLOBAL,
    command: HotkeyCommand::Redo,
};

pub(super) const REDO_SHIFT_U: HotkeyAction = HotkeyAction {
    id: "redo-shift-u",
    label: "Redo",
    gesture: HotkeyGesture::with_shift(Key::U),
    scope: GLOBAL,
    command: HotkeyCommand::Redo,
};

pub(super) const FOCUS_BROWSER_SEARCH: HotkeyAction = HotkeyAction {
    id: "search-browser",
    label: "Search samples",
    gesture: HotkeyGesture::with_command(Key::F),
    scope: GLOBAL,
    command: HotkeyCommand::FocusBrowserSearch,
};

pub(super) const FOCUS_LOADED_SAMPLE: HotkeyAction = HotkeyAction {
    id: "focus-loaded-sample",
    label: "Focus loaded sample",
    gesture: HotkeyGesture::new(Key::F),
    scope: GLOBAL,
    command: HotkeyCommand::FocusLoadedSample,
};

pub(super) const FIND_SIMILAR: HotkeyAction = HotkeyAction {
    id: "find-similar",
    label: "Toggle find similar",
    gesture: HotkeyGesture::with_shift(Key::F),
    scope: GLOBAL,
    command: HotkeyCommand::FindSimilarFocusedSample,
};

pub(super) const TOGGLE_OVERLAY: HotkeyAction = HotkeyAction {
    id: "show-hotkeys",
    label: "Show hotkeys",
    gesture: HotkeyGesture::with_command(Key::Slash),
    scope: GLOBAL,
    command: HotkeyCommand::ToggleOverlay,
};

pub(super) const COPY_STATUS_LOG: HotkeyAction = HotkeyAction {
    id: "copy-status-log",
    label: "Copy status log",
    gesture: HotkeyGesture {
        first: KeyPress {
            key: Key::L,
            command: true,
            shift: true,
            alt: false,
        },
        chord: None,
    },
    scope: GLOBAL,
    command: HotkeyCommand::CopyStatusLog,
};

pub(super) const OPEN_FEEDBACK_ISSUE_PROMPT: HotkeyAction = HotkeyAction {
    id: "submit-github-issue",
    label: "Submit GitHub issue",
    gesture: HotkeyGesture::with_shift(Key::F1),
    scope: GLOBAL,
    command: HotkeyCommand::OpenFeedbackIssuePrompt,
};

pub(super) const TOGGLE_LOOP: HotkeyAction = HotkeyAction {
    id: "toggle-loop",
    label: "Toggle loop",
    gesture: HotkeyGesture::new(Key::L),
    scope: GLOBAL,
    command: HotkeyCommand::ToggleLoop,
};

pub(super) const TOGGLE_LOOP_LOCK: HotkeyAction = HotkeyAction {
    id: "toggle-loop-lock",
    label: "Toggle loop lock",
    gesture: HotkeyGesture::with_shift(Key::L),
    scope: GLOBAL,
    command: HotkeyCommand::ToggleLoopLock,
};

pub(super) const FOCUS_WAVEFORM: HotkeyAction = HotkeyAction {
    id: "focus-waveform",
    label: "Focus waveform",
    gesture: HotkeyGesture::with_chord(KeyPress::new(Key::G), KeyPress::new(Key::W)),
    scope: GLOBAL,
    command: HotkeyCommand::FocusWaveform,
};

pub(super) const FOCUS_BROWSER_SAMPLES: HotkeyAction = HotkeyAction {
    id: "focus-browser",
    label: "Focus source samples",
    gesture: HotkeyGesture::with_chord(KeyPress::new(Key::G), KeyPress::new(Key::B)),
    scope: GLOBAL,
    command: HotkeyCommand::FocusBrowserSamples,
};

pub(super) const FOCUS_FOLDER_TREE: HotkeyAction = HotkeyAction {
    id: "focus-folder-tree",
    label: "Focus folder tree",
    gesture: HotkeyGesture::with_chord(KeyPress::new(Key::G), KeyPress::new(Key::T)),
    scope: GLOBAL,
    command: HotkeyCommand::FocusFolderTree,
};

pub(super) const FOCUS_SOURCES_LIST: HotkeyAction = HotkeyAction {
    id: "focus-sources-list",
    label: "Focus sources list",
    gesture: HotkeyGesture::with_chord(KeyPress::new(Key::G), KeyPress::new(Key::S)),
    scope: GLOBAL,
    command: HotkeyCommand::FocusSourcesList,
};

pub(super) const PLAY_FROM_START: HotkeyAction = HotkeyAction {
    id: "play-from-start",
    label: "Play from start",
    gesture: HotkeyGesture::new(Key::Space),
    scope: GLOBAL,
    command: HotkeyCommand::PlayFromStart,
};

pub(super) const PLAY_FROM_CURRENT_PLAYHEAD: HotkeyAction = HotkeyAction {
    id: "play-from-current-playhead",
    label: "Play from current playhead",
    gesture: HotkeyGesture::with_command(Key::Space),
    scope: GLOBAL,
    command: HotkeyCommand::PlayFromCurrentPlayhead,
};

pub(super) const PLAY_RANDOM_SAMPLE: HotkeyAction = HotkeyAction {
    id: "play-random-sample",
    label: "Play random sample",
    gesture: HotkeyGesture {
        first: KeyPress::with_shift(Key::R),
        chord: None,
    },
    scope: GLOBAL,
    command: HotkeyCommand::PlayRandomSample,
};

pub(super) const TOGGLE_RANDOM_NAVIGATION_MODE: HotkeyAction = HotkeyAction {
    id: "toggle-random-navigation-mode",
    label: "Toggle random navigation mode",
    gesture: HotkeyGesture {
        first: KeyPress::with_alt(Key::R),
        chord: None,
    },
    scope: GLOBAL,
    command: HotkeyCommand::ToggleRandomNavigationMode,
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
    scope: GLOBAL,
    command: HotkeyCommand::PlayPreviousRandomSample,
};

pub(super) const MOVE_TRASHED_TO_FOLDER: HotkeyAction = HotkeyAction {
    id: "move-trashed-to-folder",
    label: "Move trashed samples to folder",
    gesture: HotkeyGesture::new(Key::P),
    scope: GLOBAL,
    command: HotkeyCommand::MoveTrashedToFolder,
};

pub(super) const MOVE_TRASHED_TO_FOLDER_SHIFT: HotkeyAction = HotkeyAction {
    id: "move-trashed-to-folder-shift",
    label: "Move trashed samples to folder",
    gesture: HotkeyGesture::with_shift(Key::P),
    scope: GLOBAL,
    command: HotkeyCommand::MoveTrashedToFolder,
};

pub(super) const DECREMENT_RATING_SELECTED: HotkeyAction = HotkeyAction {
    id: "rate-decrement",
    label: "Decrement rating",
    gesture: HotkeyGesture::new(Key::OpenBracket),
    scope: GLOBAL,
    command: HotkeyCommand::DecrementRatingSelected,
};

pub(super) const INCREMENT_RATING_SELECTED: HotkeyAction = HotkeyAction {
    id: "rate-increment",
    label: "Increment rating",
    gesture: HotkeyGesture::new(Key::CloseBracket),
    scope: GLOBAL,
    command: HotkeyCommand::IncrementRatingSelected,
};

pub(super) const TAG_NEUTRAL_SELECTED: HotkeyAction = HotkeyAction {
    id: "tag-neutral",
    label: "Neutral sample(s)",
    gesture: HotkeyGesture::new(Key::Quote),
    scope: GLOBAL,
    command: HotkeyCommand::TagNeutralSelected,
};

pub(super) const TAG_KEEP_SELECTED: HotkeyAction = HotkeyAction {
    id: "tag-keep",
    label: "Keep sample(s)",
    gesture: HotkeyGesture::new(Key::Num5),
    scope: GLOBAL,
    command: HotkeyCommand::TagKeepSelected,
};

pub(super) const TAG_TRASH_SELECTED: HotkeyAction = HotkeyAction {
    id: "tag-trash",
    label: "Trash sample(s)",
    gesture: HotkeyGesture::new(Key::Num1),
    scope: GLOBAL,
    command: HotkeyCommand::TagTrashSelected,
};
