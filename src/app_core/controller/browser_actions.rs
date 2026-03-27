//! Browser, source, and folder native action dispatch helpers.
//!
//! This keeps the migration controller facade focused on top-level orchestration
//! while the branch-heavy browser/source action table lives in its own module.

use super::AppController;
use crate::app_core::actions::NativeUiAction;
use crate::app_core::state::StatusTone;

/// Try to dispatch browser-and-sources native actions.
pub(super) fn apply_browser_native_ui_action(
    controller: &mut AppController,
    action: NativeUiAction,
) -> Result<(), NativeUiAction> {
    match action {
        NativeUiAction::FocusBrowserPanel => controller.focus_browser_list(),
        NativeUiAction::FocusSourcesPanel => controller.focus_sources_list(),
        NativeUiAction::FocusWaveformPanel => controller.focus_waveform(),
        NativeUiAction::FocusFolderPanel => controller
            .focus_context_from_ui(crate::app_core::app_api::state::FocusContext::SourceFolders),
        NativeUiAction::FocusLoadedSampleInBrowser => controller.focus_loaded_sample_in_browser(),
        NativeUiAction::FocusBrowserSearch => controller.focus_browser_search(),
        NativeUiAction::BlurBrowserSearch => controller.blur_browser_search(),
        NativeUiAction::FocusFolderSearch => controller.focus_folder_search(),
        NativeUiAction::SetFolderSearch { query } => controller.set_folder_search(query),
        NativeUiAction::FocusSourceRow { index } => {
            controller.select_source_by_index(index);
            controller.focus_sources_context();
        }
        NativeUiAction::SelectSourceRow { index } => controller.select_source_by_index(index),
        NativeUiAction::MoveSourceFocus { delta } => {
            controller.nudge_source_selection(delta as isize)
        }
        NativeUiAction::ReloadFocusedSourceRow => {
            if controller.ui.sources.selected.is_some() {
                controller.request_quick_sync();
            }
        }
        NativeUiAction::HardSyncFocusedSourceRow => {
            if controller.ui.sources.selected.is_some() {
                controller.request_hard_sync();
            }
        }
        NativeUiAction::OpenFocusedSourceFolder => {
            if let Some(index) = controller.ui.sources.selected {
                controller.open_source_folder(index);
            }
        }
        NativeUiAction::RemoveFocusedSourceRow => {
            if let Some(index) = controller.ui.sources.selected {
                controller.remove_source(index);
            }
        }
        NativeUiAction::RemoveDeadLinksForFocusedSourceRow => {
            if let Some(index) = controller.ui.sources.selected {
                controller.remove_dead_links_for_source(index);
            }
        }
        NativeUiAction::FocusFolderRow { index } => controller.replace_folder_selection(index),
        NativeUiAction::ActivateFolderRow { index } => controller.activate_folder_row(index),
        NativeUiAction::ToggleFolderRowExpanded { index } => {
            controller.toggle_folder_expanded(index)
        }
        NativeUiAction::ExpandFocusedFolder => controller.expand_focused_folder(),
        NativeUiAction::CollapseFocusedFolder => controller.collapse_focused_folder(),
        NativeUiAction::ToggleFocusedFolderSelection => {
            controller.toggle_focused_folder_selection()
        }
        NativeUiAction::MoveFolderFocus { delta } => controller.nudge_folder_focus_action(delta),
        NativeUiAction::StartNewFolder => controller.start_new_folder(),
        NativeUiAction::StartNewFolderAtFolderRow { index } => {
            controller.start_new_folder_at_folder_row(index)
        }
        NativeUiAction::StartNewFolderAtRoot => {
            if controller.current_source().is_none() {
                controller.add_source_via_dialog();
            } else {
                controller.start_new_folder_at_root();
            }
        }
        NativeUiAction::FocusFolderCreateInput => controller.focus_new_folder_creation_input(),
        NativeUiAction::StartFolderRename => controller.start_folder_rename(),
        NativeUiAction::DeleteFocusedFolder => controller.delete_focused_folder(),
        NativeUiAction::RestoreRetainedFolderDeletes => {
            controller.start_restore_retained_folder_deletes()
        }
        NativeUiAction::PurgeRetainedFolderDeletes => {
            controller.start_purge_retained_folder_deletes()
        }
        NativeUiAction::ClearFolderDeleteRecoveryLog => {
            controller.clear_folder_delete_recovery_log()
        }
        NativeUiAction::ReloadSourceRow { index } => {
            controller.select_source_by_index(index);
            controller.request_quick_sync();
        }
        NativeUiAction::HardSyncSourceRow { index } => {
            controller.select_source_by_index(index);
            controller.request_hard_sync();
        }
        NativeUiAction::OpenSourceFolderRow { index } => controller.open_source_folder(index),
        NativeUiAction::RemoveSourceRow { index } => controller.remove_source(index),
        NativeUiAction::RemoveDeadLinksForSourceRow { index } => {
            controller.remove_dead_links_for_source(index)
        }
        NativeUiAction::OpenAddSourceDialog => controller.add_source_via_dialog(),
        NativeUiAction::OpenOptionsMenu => controller.open_options_panel(),
        NativeUiAction::CloseOptionsPanel => controller.close_options_panel(),
        NativeUiAction::PickTrashFolder => controller.pick_trash_folder(),
        NativeUiAction::OpenTrashFolder => controller.open_trash_folder(),
        NativeUiAction::SetInputMonitoringEnabled { enabled } => {
            controller.set_input_monitoring_enabled(enabled)
        }
        NativeUiAction::SetAdvanceAfterRatingEnabled { enabled } => {
            controller.set_advance_after_rating(enabled)
        }
        NativeUiAction::SetDestructiveYoloMode { enabled } => {
            controller.set_destructive_yolo_mode(enabled)
        }
        NativeUiAction::SetInvertWaveformScroll { enabled } => {
            controller.set_invert_waveform_scroll(enabled)
        }
        NativeUiAction::MoveBrowserFocus { delta } => controller.focus_browser_delta_action(delta),
        NativeUiAction::SetBrowserViewStart { visible_row } => {
            controller.set_browser_view_start_action(visible_row)
        }
        NativeUiAction::FocusBrowserRow { visible_row } => {
            controller.focus_browser_row_and_play_action(visible_row)
        }
        NativeUiAction::CommitFocusedBrowserRow => {
            if matches!(
                controller.ui.focus.context,
                crate::app_core::app_api::state::FocusContext::SampleBrowser
            ) && controller.commit_focused_browser_row_action()
            {
                return Ok(());
            }
            controller.toggle_play_pause();
        }
        NativeUiAction::ToggleBrowserRowSelection { visible_row } => {
            controller.toggle_browser_row_selection(visible_row)
        }
        NativeUiAction::ExtendBrowserSelectionToRow { visible_row } => {
            controller.extend_browser_selection_to_row(visible_row)
        }
        NativeUiAction::AddRangeBrowserSelection { visible_row } => {
            controller.add_range_browser_selection(visible_row)
        }
        NativeUiAction::ExtendBrowserSelectionFromFocus { delta } => {
            controller.extend_browser_selection_from_focus_action(delta)
        }
        NativeUiAction::AddRangeBrowserSelectionFromFocus { delta } => {
            controller.add_range_browser_selection_from_focus_action(delta)
        }
        NativeUiAction::ToggleFocusedBrowserRowSelection => controller.toggle_focused_selection(),
        NativeUiAction::SelectAllBrowserRows => controller.select_all_browser_rows(),
        NativeUiAction::SetBrowserSearch { query } => controller.set_browser_search(query),
        NativeUiAction::ToggleBrowserRatingFilter { level, invert } => {
            controller.focus_browser_list();
            if invert {
                controller.invert_browser_rating_filter(level);
            } else {
                controller.set_browser_rating_filter(level, true);
            }
        }
        NativeUiAction::ToggleRandomNavigationMode => controller.toggle_random_navigation_mode(),
        NativeUiAction::FocusPreviousBrowserHistory => controller.focus_previous_sample_history(),
        NativeUiAction::FocusNextBrowserHistory => controller.focus_next_sample_history(),
        NativeUiAction::ToggleFindSimilarFocusedSample => {
            toggle_find_similar_focused_sample(controller)
        }
        NativeUiAction::PlayRandomSample => controller.play_random_visible_sample(),
        NativeUiAction::PlayPreviousRandomSample => controller.play_previous_random_sample(),
        NativeUiAction::AdjustSelectedBrowserRating { delta } => {
            controller.adjust_selected_rating(delta)
        }
        NativeUiAction::StartBrowserRename => controller.start_browser_rename(),
        NativeUiAction::ConfirmBrowserRename => controller.apply_pending_browser_rename(),
        NativeUiAction::CancelBrowserRename => controller.cancel_browser_rename(),
        NativeUiAction::TagBrowserSelection { target } => {
            controller.tag_selected_browser_target(target.into())
        }
        NativeUiAction::NormalizeFocusedBrowserSample => {
            if let Some(row) = controller.focused_browser_row() {
                let _ = controller.normalize_browser_sample(row);
            } else {
                controller.set_status("Focus a sample to normalize it", StatusTone::Info);
            }
        }
        NativeUiAction::DeleteBrowserSelection => {
            controller.delete_active_browser_selection_action()
        }
        NativeUiAction::MoveTrashedSamplesToFolder => controller.move_all_trashed_to_folder(),
        action => return Err(action),
    }
    Ok(())
}

fn toggle_find_similar_focused_sample(controller: &mut AppController) {
    if matches!(
        controller.ui.browser.active_tab,
        crate::app_core::state::SampleBrowserTab::Map
    ) {
        controller.ui.browser.active_tab = crate::app_core::state::SampleBrowserTab::List;
    }
    let Some(row) = controller.focused_browser_row() else {
        controller.set_status("Focus a sample to find similar", StatusTone::Info);
        return;
    };
    let focused_sample_id = controller.sample_id_for_visible_row(row).ok();
    let query_matches_focus = controller
        .ui
        .browser
        .search
        .similar_query
        .as_ref()
        .zip(focused_sample_id.as_deref())
        .is_some_and(|(query, focused_sample_id)| query.sample_id == focused_sample_id);
    if query_matches_focus {
        controller.clear_similar_filter();
        return;
    }
    if let Err(err) = controller.find_similar_for_visible_row(row) {
        controller.set_status(format!("Find similar failed: {err}"), StatusTone::Error);
    }
}
