use super::super::*;

pub(super) fn action_slug(action: &UiAction) -> Option<&'static str> {
    Some(match action {
        UiAction::MoveBrowserFocus { .. } => "move_browser_focus",
        UiAction::SetBrowserViewStart { .. } => "set_browser_view_start",
        UiAction::FocusBrowserRow { .. } => "focus_browser_row",
        UiAction::AutoRenameBrowserSelection { .. } => "auto_rename_browser_selection",
        UiAction::SetCompareAnchorFromFocusedContent => "set_compare_anchor_from_focused_content",
        UiAction::CommitFocusedBrowserRow => "commit_focused_browser_row",
        UiAction::SaveWaveformSelectionToBrowser => "save_waveform_selection_to_browser",
        UiAction::SaveWaveformSelectionToBrowserWithKeep2 => {
            "save_waveform_selection_to_browser_with_keep2"
        }
        UiAction::CommitWaveformEditFades => "commit_waveform_edit_fades",
        UiAction::DetectWaveformSilenceSlices => "detect_waveform_silence_slices",
        UiAction::DetectWaveformExactDuplicateSlices => "detect_waveform_exact_duplicate_slices",
        UiAction::CleanWaveformExactDuplicateSlices => "clean_waveform_exact_duplicate_slices",
        UiAction::ToggleBrowserRowSelection { .. } => "toggle_browser_row_selection",
        UiAction::StartContentItemDrag { .. } => "start_content_item_drag",
        UiAction::UpdateContentItemDrag { .. } => "update_content_item_drag",
        UiAction::FinishContentItemDrag => "finish_content_item_drag",
        UiAction::ExtendBrowserSelectionToRow { .. } => "extend_browser_selection_to_row",
        UiAction::AddRangeBrowserSelection { .. } => "add_range_browser_selection",
        UiAction::ExtendBrowserSelectionFromFocus { .. } => "extend_browser_selection_from_focus",
        UiAction::AddRangeBrowserSelectionFromFocus { .. } => {
            "add_range_browser_selection_from_focus"
        }
        UiAction::ToggleFocusedBrowserRowSelection => "toggle_focused_browser_row_selection",
        UiAction::SelectAllBrowserRows => "select_all_browser_rows",
        UiAction::SetBrowserSearch { .. } => "set_browser_search",
        UiAction::ToggleBrowserRatingFilter { .. } => "toggle_browser_rating_filter",
        UiAction::ToggleBrowserPlaybackAgeFilter { .. } => "toggle_browser_playback_age_filter",
        UiAction::ToggleBrowserSidebarFilter { .. } => "toggle_browser_sidebar_filter",
        UiAction::ClearBrowserSidebarFilter { .. } => "clear_browser_sidebar_filter",
        UiAction::ToggleContentMark => "toggle_content_mark",
        UiAction::ToggleBrowserMarkedFilter => "toggle_browser_marked_filter",
        UiAction::ToggleBrowserDerivedLabelFilter { .. } => "toggle_browser_derived_label_filter",
        UiAction::ToggleRandomNavigationMode => "toggle_random_navigation_mode",
        UiAction::ToggleBrowserPillEditor => "toggle_browser_pill_editor",
        UiAction::ToggleBrowserPillEditorPrimaryAction => {
            "toggle_browser_pill_editor_primary_action"
        }
        UiAction::ToggleBrowserDuplicateCleanupMode => "toggle_browser_duplicate_cleanup_mode",
        UiAction::FocusPreviousBrowserHistory => "focus_previous_browser_history",
        UiAction::FocusNextBrowserHistory => "focus_next_browser_history",
        UiAction::ToggleFindSimilarFocusedContent => "toggle_find_similar_focused_content",
        UiAction::ToggleBrowserDuplicateCleanupKeep { .. } => {
            "toggle_browser_duplicate_cleanup_keep"
        }
        UiAction::ConfirmBrowserDuplicateCleanup => "confirm_browser_duplicate_cleanup",
        UiAction::PlayRandomContentItem => "play_random_content_item",
        UiAction::PlayPreviousRandomContentItem => "play_previous_random_content_item",
        UiAction::AdjustSelectedBrowserRating { .. } => "adjust_selected_browser_rating",
        UiAction::SetBrowserTab { .. } => "set_browser_tab",
        UiAction::FocusBrowserPillEditorInput => "focus_browser_pill_editor_input",
        UiAction::SetBrowserPillEditorInput { .. } => "set_browser_pill_editor_input",
        UiAction::CommitBrowserPillEditorInput => "commit_browser_pill_editor_input",
        UiAction::SetBrowserSidebarLooped { .. } => "set_browser_sidebar_looped",
        UiAction::ToggleBrowserPillOption { .. } => "toggle_browser_pill_option",
        UiAction::FocusSpatialContentItem { .. } => "focus_spatial_content_item",
        UiAction::SetPromptInput { .. } => "set_prompt_input",
        UiAction::StartBrowserRename => "start_browser_rename",
        UiAction::ConfirmBrowserRename => "confirm_browser_rename",
        UiAction::CancelBrowserRename => "cancel_browser_rename",
        UiAction::SetBrowserTriageMark { .. } => "set_browser_triage_mark",
        UiAction::DeleteBrowserSelection => "delete_browser_selection",
        _ => return None,
    })
}
