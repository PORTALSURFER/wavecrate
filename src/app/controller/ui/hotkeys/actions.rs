mod browser;
mod folders;
mod global;
mod waveform;

use super::types::HotkeyAction;

/// Combined hotkey registry export kept in the existing order so lookup and
/// UI presentation behavior stay stable while scope-owned definitions live in
/// smaller modules.
pub(crate) const HOTKEY_ACTIONS: &[HotkeyAction] = &[
    global::UNDO_CTRL_Z,
    global::UNDO_U,
    global::REDO_CTRL_Y,
    global::REDO_SHIFT_U,
    browser::TOGGLE_SELECT,
    browser::FOCUS_HISTORY_PREVIOUS,
    browser::FOCUS_HISTORY_NEXT,
    folders::TOGGLE_SELECT,
    folders::DELETE_FOLDER,
    folders::RENAME_FOLDER,
    browser::RENAME_SAMPLE,
    folders::CREATE_FOLDER,
    folders::FOCUS_SEARCH,
    global::FOCUS_BROWSER_SEARCH,
    global::FOCUS_LOADED_SAMPLE,
    global::FIND_SIMILAR,
    browser::SELECT_ALL,
    browser::NORMALIZE_SAMPLE,
    waveform::NORMALIZE_SELECTION,
    waveform::ALIGN_START_TO_MARKER,
    browser::DELETE_SAMPLE,
    waveform::CROP_SELECTION,
    waveform::CROP_SELECTION_NEW_SAMPLE,
    waveform::SAVE_SELECTION_TO_BROWSER,
    global::TOGGLE_OVERLAY,
    global::COPY_STATUS_LOG,
    global::OPEN_FEEDBACK_ISSUE_PROMPT,
    global::TOGGLE_LOOP,
    global::TOGGLE_LOOP_LOCK,
    global::FOCUS_WAVEFORM,
    global::FOCUS_BROWSER_SAMPLES,
    global::FOCUS_FOLDER_TREE,
    global::FOCUS_SOURCES_LIST,
    global::PLAY_FROM_START,
    global::PLAY_FROM_CURRENT_PLAYHEAD,
    global::PLAY_RANDOM_SAMPLE,
    global::TOGGLE_RANDOM_NAVIGATION_MODE,
    global::PLAY_PREVIOUS_RANDOM_SAMPLE,
    global::MOVE_TRASHED_TO_FOLDER,
    global::MOVE_TRASHED_TO_FOLDER_SHIFT,
    global::DECREMENT_RATING_SELECTED,
    global::INCREMENT_RATING_SELECTED,
    global::TAG_NEUTRAL_SELECTED,
    global::TAG_KEEP_SELECTED,
    global::TAG_TRASH_SELECTED,
    waveform::TRIM_SELECTION,
    waveform::TOGGLE_BPM_SNAP,
    waveform::TOGGLE_TRANSIENT_MARKERS,
    waveform::REVERSE_SELECTION,
    browser::REVERSE_SELECTION,
    waveform::FADE_SELECTION_LEFT_TO_RIGHT,
    waveform::FADE_SELECTION_RIGHT_TO_LEFT,
    waveform::DELETE_SLICE_MARKERS,
    waveform::DELETE_LOADED_SAMPLE,
    waveform::MUTE_SELECTION,
    waveform::ZOOM_IN_SELECTION,
    waveform::ZOOM_OUT_SELECTION,
    waveform::SLIDE_SELECTION_LEFT,
    waveform::SLIDE_SELECTION_RIGHT,
    waveform::NUDGE_SELECTION_LEFT,
    waveform::NUDGE_SELECTION_RIGHT,
];

#[cfg(test)]
mod tests {
    use super::super::types::{HotkeyCommand, HotkeyScope};
    use super::*;
    use crate::app::controller::ui::hotkeys;
    use crate::app::state::FocusContext;

    #[test]
    fn hotkey_registry_ids_are_unique() {
        for (index, action) in HOTKEY_ACTIONS.iter().enumerate() {
            let duplicate = HOTKEY_ACTIONS
                .iter()
                .skip(index + 1)
                .find(|candidate| candidate.id == action.id);
            assert!(duplicate.is_none(), "duplicate hotkey id: {}", action.id);
        }
    }

    #[test]
    fn hotkey_registry_gestures_are_unique_within_scope() {
        for (index, action) in HOTKEY_ACTIONS.iter().enumerate() {
            let duplicate = HOTKEY_ACTIONS.iter().skip(index + 1).find(|candidate| {
                candidate.scope == action.scope && candidate.gesture == action.gesture
            });
            assert!(
                duplicate.is_none(),
                "duplicate scoped hotkey gesture for {:?}: {:?}",
                action.scope,
                action.gesture
            );
        }
    }

    #[test]
    fn hotkey_helper_views_keep_global_and_focus_actions_separate() {
        let global = hotkeys::global_actions();
        assert!(!global.is_empty());
        assert!(global.iter().all(HotkeyAction::is_global));
        assert!(
            global
                .iter()
                .any(|action| action.command() == HotkeyCommand::FocusBrowserSearch)
        );

        let folder_focus = hotkeys::focused_actions(FocusContext::SourceFolders);
        assert!(!folder_focus.is_empty());
        assert!(folder_focus.iter().all(|action| matches!(
            action.scope,
            HotkeyScope::Focus(FocusContext::SourceFolders)
        )));
        assert!(
            folder_focus
                .iter()
                .any(|action| action.command() == HotkeyCommand::FocusFolderSearch)
        );
        assert!(
            folder_focus
                .iter()
                .all(|action| action.command() != HotkeyCommand::FocusBrowserSearch)
        );
    }
}
