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
    use crate::gui::input::KeyCode;

    fn map_focus_context(
        focus: radiant::compat::sempal_shell::FocusContextModel,
    ) -> FocusContext {
        match focus {
            radiant::compat::sempal_shell::FocusContextModel::None => FocusContext::None,
            radiant::compat::sempal_shell::FocusContextModel::Waveform => FocusContext::Waveform,
            radiant::compat::sempal_shell::FocusContextModel::SampleBrowser => {
                FocusContext::SampleBrowser
            }
            radiant::compat::sempal_shell::FocusContextModel::SourceFolders => {
                FocusContext::SourceFolders
            }
            radiant::compat::sempal_shell::FocusContextModel::SourcesList => {
                FocusContext::SourcesList
            }
        }
    }

    fn map_scope(scope: radiant::compat::sempal_shell::HotkeyScope) -> HotkeyScope {
        match scope {
            radiant::compat::sempal_shell::HotkeyScope::Global => HotkeyScope::Global,
            radiant::compat::sempal_shell::HotkeyScope::Focus(focus) => {
                HotkeyScope::Focus(map_focus_context(focus))
            }
        }
    }

    fn map_keypress(press: radiant::compat::sempal_shell::KeyPress) -> KeyPress {
        KeyPress {
            key: press.key,
            command: press.command,
            shift: press.shift,
            alt: press.alt,
        }
    }

    fn map_gesture(gesture: radiant::compat::sempal_shell::HotkeyGesture) -> HotkeyGesture {
        HotkeyGesture {
            first: map_keypress(gesture.first),
            chord: gesture.chord.map(map_keypress),
        }
    }

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
            crate::app_core::actions::NativeUiAction::FocusBrowserPanel
        )));

        let folder_focus = hotkeys::focused_actions(FocusContext::SourceFolders);
        assert!(!folder_focus.is_empty());
        assert!(folder_focus.iter().all(|action| matches!(
            action.scope,
            HotkeyScope::Focus(FocusContext::SourceFolders)
        )));
        assert!(folder_focus.iter().any(|action| matches!(
            action.action,
            crate::app_core::actions::NativeUiAction::FocusFolderSearch { .. }
        )));
        assert!(folder_focus.iter().all(|action| !matches!(
            action.action,
            crate::app_core::actions::NativeUiAction::FocusBrowserPanel
        )));
    }

    #[test]
    fn sempal_catalog_preserves_legacy_radiant_binding_contract() {
        let legacy: Vec<_> = radiant::compat::sempal_shell::iter_hotkey_bindings().collect();
        assert_eq!(HOTKEY_ACTIONS.len(), legacy.len());

        for (action, binding) in HOTKEY_ACTIONS.iter().zip(legacy) {
            assert_eq!(action.id, binding.id);
            assert_eq!(action.label, binding.label);
            assert_eq!(action.gesture, map_gesture(binding.gesture));
            assert_eq!(action.scope, map_scope(binding.scope));
            assert_eq!(action.action, binding.action.clone().into());
        }
    }

    #[test]
    fn source_hotkeys_are_owned_by_sempal_catalog() {
        let source_focus = hotkeys::focused_actions(FocusContext::SourcesList);
        assert!(source_focus.iter().any(|action| matches!(
            action.action,
            crate::app_core::actions::NativeUiAction::ReloadFocusedSourceRow
        )));
        assert!(source_focus.iter().any(|action| {
            action.gesture == HotkeyGesture::new(KeyCode::D)
                && matches!(
                    action.action,
                    crate::app_core::actions::NativeUiAction::RemoveFocusedSourceRow
                )
        }));
    }
}
