use super::super::types::{HotkeyAction, HotkeyGesture, HotkeyScope, KeyPress};
use crate::app_core::actions::{NativeBrowserTagTarget, NativeUiAction};
use radiant::gui::input::KeyCode as Key;

const GLOBAL: HotkeyScope = HotkeyScope::Global;

pub(super) const UNDO_CTRL_Z: HotkeyAction = HotkeyAction {
    id: "undo-ctrl-z",
    label: "Undo",
    gesture: HotkeyGesture::with_command(Key::Z),
    scope: GLOBAL,
    action: NativeUiAction::Undo,
};
pub(super) const UNDO_U: HotkeyAction = HotkeyAction {
    id: "undo-u",
    label: "Undo",
    gesture: HotkeyGesture::new(Key::U),
    scope: GLOBAL,
    action: NativeUiAction::Undo,
};
pub(super) const REDO_CTRL_Y: HotkeyAction = HotkeyAction {
    id: "redo-ctrl-y",
    label: "Redo",
    gesture: HotkeyGesture::with_command(Key::Y),
    scope: GLOBAL,
    action: NativeUiAction::Redo,
};
pub(super) const REDO_SHIFT_U: HotkeyAction = HotkeyAction {
    id: "redo-shift-u",
    label: "Redo",
    gesture: HotkeyGesture::with_shift(Key::U),
    scope: GLOBAL,
    action: NativeUiAction::Redo,
};
pub(super) const TOGGLE_OVERLAY: HotkeyAction = HotkeyAction {
    id: "show-hotkeys",
    label: "Show hotkeys",
    gesture: HotkeyGesture::with_command(Key::Slash),
    scope: GLOBAL,
    action: NativeUiAction::PromptsAndEdits(
        crate::app_core::actions::NativePromptEditAction::ToggleHotkeyOverlay,
    ),
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
    action: NativeUiAction::PromptsAndEdits(
        crate::app_core::actions::NativePromptEditAction::CopyStatusLog,
    ),
};
pub(super) const OPEN_FEEDBACK_ISSUE_PROMPT: HotkeyAction = HotkeyAction {
    id: "submit-github-issue",
    label: "Submit GitHub issue",
    gesture: HotkeyGesture::with_shift(Key::F1),
    scope: GLOBAL,
    action: NativeUiAction::PromptsAndEdits(
        crate::app_core::actions::NativePromptEditAction::OpenFeedbackIssuePrompt,
    ),
};
pub(super) const FOCUS_WAVEFORM: HotkeyAction = HotkeyAction {
    id: "focus-waveform",
    label: "Focus waveform",
    gesture: HotkeyGesture::with_chord(KeyPress::new(Key::G), KeyPress::new(Key::W)),
    scope: GLOBAL,
    action: NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::FocusWaveformPanel),
};
pub(super) const FOCUS_BROWSER_SAMPLES: HotkeyAction = HotkeyAction {
    id: "focus-browser",
    label: "Focus source samples",
    gesture: HotkeyGesture::with_chord(KeyPress::new(Key::G), KeyPress::new(Key::B)),
    scope: GLOBAL,
    action: NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::FocusBrowserPanel),
};
pub(super) const FOCUS_FOLDER_TREE: HotkeyAction = HotkeyAction {
    id: "focus-folder-tree",
    label: "Focus folder tree",
    gesture: HotkeyGesture::with_chord(KeyPress::new(Key::G), KeyPress::new(Key::T)),
    scope: GLOBAL,
    action: NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::FocusFolderPanel),
};
pub(super) const FOCUS_SOURCES_LIST: HotkeyAction = HotkeyAction {
    id: "focus-sources-list",
    label: "Focus sources list",
    gesture: HotkeyGesture::with_chord(KeyPress::new(Key::G), KeyPress::new(Key::S)),
    scope: GLOBAL,
    action: NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::FocusSourcesPanel),
};
pub(super) const PLAY_FROM_START: HotkeyAction = HotkeyAction {
    id: "play-from-start",
    label: "Play from start",
    gesture: HotkeyGesture::new(Key::Space),
    scope: GLOBAL,
    action: NativeUiAction::Transport(
        crate::app_core::actions::NativeTransportAction::PlayFromStart,
    ),
};
pub(super) const PLAY_COMPARE_ANCHOR: HotkeyAction = HotkeyAction {
    id: "play-compare-anchor",
    label: "Play compare anchor",
    gesture: HotkeyGesture::with_shift(Key::Space),
    scope: GLOBAL,
    action: NativeUiAction::Transport(
        crate::app_core::actions::NativeTransportAction::PlayCompareAnchor,
    ),
};
pub(super) const PLAY_FROM_CURRENT_PLAYHEAD: HotkeyAction = HotkeyAction {
    id: "play-from-current-playhead",
    label: "Play from current playhead",
    gesture: HotkeyGesture::with_command(Key::Space),
    scope: GLOBAL,
    action: NativeUiAction::Transport(
        crate::app_core::actions::NativeTransportAction::PlayFromCurrentPlayhead,
    ),
};
pub(super) const TOGGLE_LOOP: HotkeyAction = HotkeyAction {
    id: "toggle-loop",
    label: "Toggle loop",
    gesture: HotkeyGesture::new(Key::L),
    scope: GLOBAL,
    action: NativeUiAction::Options(
        crate::app_core::actions::NativeOptionsAction::ToggleLoopPlayback,
    ),
};
pub(super) const TOGGLE_LOOP_LOCK: HotkeyAction = HotkeyAction {
    id: "toggle-loop-lock",
    label: "Cycle locked loop",
    gesture: HotkeyGesture::with_shift(Key::L),
    scope: GLOBAL,
    action: NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::ToggleLoopLock),
};
pub(super) const DECREMENT_RATING_SELECTED: HotkeyAction = HotkeyAction {
    id: "rate-decrement",
    label: "Decrement rating",
    gesture: HotkeyGesture::new(Key::OpenBracket),
    scope: GLOBAL,
    action: NativeUiAction::Browser(
        crate::app_core::actions::NativeBrowserAction::AdjustSelectedBrowserRating { delta: -1 },
    ),
};
pub(super) const INCREMENT_RATING_SELECTED: HotkeyAction = HotkeyAction {
    id: "rate-increment",
    label: "Increment rating",
    gesture: HotkeyGesture::new(Key::CloseBracket),
    scope: GLOBAL,
    action: NativeUiAction::Browser(
        crate::app_core::actions::NativeBrowserAction::AdjustSelectedBrowserRating { delta: 1 },
    ),
};
pub(super) const TAG_NEUTRAL_SELECTED: HotkeyAction = HotkeyAction {
    id: "tag-neutral",
    label: "Neutral sample(s)",
    gesture: HotkeyGesture::new(Key::Quote),
    scope: GLOBAL,
    action: NativeUiAction::PromptsAndEdits(
        crate::app_core::actions::NativePromptEditAction::TagBrowserSelection {
            target: NativeBrowserTagTarget::Neutral,
        },
    ),
};
pub(super) const TAG_KEEP_SELECTED: HotkeyAction = HotkeyAction {
    id: "tag-keep",
    label: "Keep sample(s)",
    gesture: HotkeyGesture::new(Key::Num5),
    scope: GLOBAL,
    action: NativeUiAction::PromptsAndEdits(
        crate::app_core::actions::NativePromptEditAction::TagBrowserSelection {
            target: NativeBrowserTagTarget::Keep,
        },
    ),
};
pub(super) const TAG_TRASH_SELECTED: HotkeyAction = HotkeyAction {
    id: "tag-trash",
    label: "Trash sample(s)",
    gesture: HotkeyGesture::new(Key::Num1),
    scope: GLOBAL,
    action: NativeUiAction::PromptsAndEdits(
        crate::app_core::actions::NativePromptEditAction::TagBrowserSelection {
            target: NativeBrowserTagTarget::Trash,
        },
    ),
};
