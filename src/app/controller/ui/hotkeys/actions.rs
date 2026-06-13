mod browser;
mod folders;
mod global;
mod sources;
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
    global::TOGGLE_OVERLAY,
    global::COPY_STATUS_LOG,
    global::OPEN_FEEDBACK_ISSUE_PROMPT,
    global::FOCUS_WAVEFORM,
    global::FOCUS_BROWSER_SAMPLES,
    global::FOCUS_FOLDER_TREE,
    global::FOCUS_SOURCES_LIST,
    global::PLAY_FROM_START,
    global::PLAY_COMPARE_ANCHOR,
    global::PLAY_FROM_CURRENT_PLAYHEAD,
    global::TOGGLE_LOOP,
    global::TOGGLE_LOOP_LOCK,
    global::DECREMENT_RATING_SELECTED,
    global::INCREMENT_RATING_SELECTED,
    global::TAG_NEUTRAL_SELECTED,
    global::TAG_KEEP_SELECTED,
    global::TAG_TRASH_SELECTED,
    browser::SEARCH_BROWSER,
    browser::FOCUS_LOADED_SAMPLE,
    browser::COPY_BROWSER_SELECTION,
    browser::SET_COMPARE_ANCHOR,
    browser::FIND_SIMILAR,
    browser::TOGGLE_RANDOM_NAVIGATION_MODE,
    browser::PLAY_RANDOM_SAMPLE,
    browser::PLAY_PREVIOUS_RANDOM_SAMPLE,
    browser::MOVE_TRASHED_TO_FOLDER,
    browser::MOVE_TRASHED_TO_FOLDER_SHIFT,
    browser::TOGGLE_SELECT,
    browser::TOGGLE_BROWSER_SAMPLE_MARK,
    browser::MOVE_BROWSER_FOCUS_UP,
    browser::MOVE_BROWSER_FOCUS_DOWN,
    browser::FOCUS_HISTORY_PREVIOUS,
    browser::FOCUS_HISTORY_NEXT,
    browser::RENAME_SAMPLE,
    browser::SELECT_ALL,
    browser::NORMALIZE_SAMPLE,
    browser::DELETE_SAMPLE,
    folders::TOGGLE_SELECT,
    folders::MOVE_FOLDER_FOCUS_UP,
    folders::MOVE_FOLDER_FOCUS_DOWN,
    folders::COLLAPSE_FOCUSED_FOLDER,
    folders::EXPAND_FOCUSED_FOLDER,
    folders::DELETE_FOLDER,
    folders::RENAME_FOLDER,
    folders::RENAME_FOLDER_LEGACY,
    folders::CREATE_FOLDER,
    folders::FOCUS_SEARCH,
    sources::MOVE_SOURCE_FOCUS_UP,
    sources::MOVE_SOURCE_FOCUS_DOWN,
    sources::RELOAD_FOCUSED_SOURCE,
    sources::HARD_SYNC_FOCUSED_SOURCE,
    sources::OPEN_FOCUSED_SOURCE_FOLDER,
    sources::REMOVE_FOCUSED_SOURCE,
    waveform::NORMALIZE_SELECTION,
    waveform::ALIGN_START_TO_MARKER,
    waveform::CROP_SELECTION,
    waveform::COPY_WAVEFORM_SELECTION,
    waveform::CROP_SELECTION_NEW_SAMPLE,
    waveform::SAVE_SELECTION_TO_BROWSER,
    waveform::SAVE_SELECTION_TO_BROWSER_KEEP2,
    waveform::COMMIT_WAVEFORM_EDIT_FADES,
    waveform::TOGGLE_FOCUSED_SLICE_EXPORT_MARK,
    waveform::TRIM_SELECTION,
    waveform::TOGGLE_BPM_SNAP,
    waveform::TOGGLE_TRANSIENT_MARKERS,
    waveform::REVERSE_SELECTION,
    waveform::FADE_SELECTION_LEFT_TO_RIGHT,
    waveform::FADE_SELECTION_RIGHT_TO_LEFT,
    waveform::DELETE_SLICE_MARKERS,
    waveform::DELETE_LOADED_SAMPLE,
    waveform::MUTE_SELECTION,
    waveform::ZOOM_IN_SELECTION,
    waveform::ZOOM_OUT_SELECTION,
    waveform::SLIDE_SELECTION_LEFT,
    waveform::SLIDE_SELECTION_RIGHT,
    waveform::MICRO_SLIDE_SELECTION_LEFT,
    waveform::MICRO_SLIDE_SELECTION_RIGHT,
    waveform::NUDGE_SELECTION_LEFT,
    waveform::NUDGE_SELECTION_RIGHT,
];

#[cfg(test)]
mod tests {
    use super::super::types::{HotkeyGesture, HotkeyScope, KeyPress};
    use super::*;
    use crate::app::controller::ui::hotkeys;
    use crate::app::state::FocusContext;
    use crate::app_core::actions::NativeUiAction;
    use radiant::gui::input::KeyCode;

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
        assert!(global.iter().any(|action| matches!(
            action.action,
            crate::app_core::actions::NativeUiAction::Shell(
                crate::app_core::actions::NativeShellAction::FocusBrowserPanel
            )
        )));

        let folder_focus = hotkeys::focused_actions(FocusContext::SourceFolders);
        assert!(!folder_focus.is_empty());
        assert!(folder_focus.iter().all(|action| matches!(
            action.scope,
            HotkeyScope::Focus(FocusContext::SourceFolders)
        )));
        assert!(folder_focus.iter().any(|action| matches!(
            action.action,
            crate::app_core::actions::NativeUiAction::Shell(
                crate::app_core::actions::NativeShellAction::FocusFolderSearch
            )
        )));
        assert!(folder_focus.iter().all(|action| !matches!(
            action.action,
            crate::app_core::actions::NativeUiAction::Shell(
                crate::app_core::actions::NativeShellAction::FocusBrowserPanel
            )
        )));
    }

    #[test]
    fn wavecrate_catalog_resolves_chords_and_contextual_actions() {
        let first = hotkeys::resolve_hotkey_press(
            None,
            KeyPress::new(KeyCode::G),
            FocusContext::SampleBrowser,
        );
        assert_eq!(first.pending_chord, Some(KeyPress::new(KeyCode::G)));
        assert!(first.handled);

        let second = hotkeys::resolve_hotkey_press(
            first.pending_chord,
            KeyPress::new(KeyCode::W),
            FocusContext::SampleBrowser,
        );
        assert_eq!(
            second.action,
            Some(NativeUiAction::Shell(
                crate::app_core::actions::NativeShellAction::FocusWaveformPanel
            ))
        );
        assert!(second.handled);
        assert_eq!(second.pending_chord, None);

        let browser_copy = hotkeys::resolve_hotkey_press(
            None,
            KeyPress::with_command(KeyCode::C),
            FocusContext::SampleBrowser,
        );
        assert_eq!(
            browser_copy.action,
            Some(NativeUiAction::PromptsAndEdits(
                crate::app_core::actions::NativePromptEditAction::CopySelectionToClipboard
            ))
        );

        let waveform_search =
            hotkeys::resolve_hotkey_press(None, KeyPress::new(KeyCode::F), FocusContext::Waveform);
        assert_eq!(waveform_search.action, None);
        assert!(!waveform_search.handled);
    }

    #[test]
    fn source_hotkeys_are_owned_by_wavecrate_catalog() {
        let source_focus = hotkeys::focused_actions(FocusContext::SourcesList);
        assert!(source_focus.iter().any(|action| matches!(
            action.action,
            crate::app_core::actions::NativeUiAction::SourcesAndFolders(
                crate::app_core::actions::NativeSourcesFoldersAction::ReloadFocusedSourceRow
            )
        )));
        assert!(source_focus.iter().any(|action| {
            action.gesture == HotkeyGesture::new(KeyCode::D)
                && matches!(
                    action.action,
                    crate::app_core::actions::NativeUiAction::SourcesAndFolders(crate::app_core::actions::NativeSourcesFoldersAction::RemoveFocusedSourceRow)
                )
        }));
    }
}
