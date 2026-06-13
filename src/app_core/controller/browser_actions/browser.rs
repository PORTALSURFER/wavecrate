//! Browser-list routing for UI browser actions.

#[path = "browser/drag.rs"]
mod drag;
#[path = "browser/edits.rs"]
mod edits;
#[path = "browser/filters.rs"]
mod filters;
#[path = "browser/tagging.rs"]
mod tagging;

use super::super::AppController;
use crate::app_core::actions::NativeUiAction;
use crate::app_core::app_api::state::FocusContext;
use crate::app_core::state::StatusTone;

/// Try to dispatch browser-list UI actions.
pub(super) fn apply_browser_list_ui_action(
    controller: &mut AppController,
    action: NativeUiAction,
) -> Result<(), NativeUiAction> {
    let action = match apply_focus_and_selection_action(controller, action) {
        Ok(()) => return Ok(()),
        Err(action) => action,
    };
    let action = match drag::apply_drag_action(controller, action) {
        Ok(()) => return Ok(()),
        Err(action) => action,
    };
    let action = match filters::apply_filter_action(controller, action) {
        Ok(()) => return Ok(()),
        Err(action) => action,
    };
    let action = match apply_history_and_cleanup_action(controller, action) {
        Ok(()) => return Ok(()),
        Err(action) => action,
    };
    let action = match tagging::apply_tagging_action(controller, action) {
        Ok(()) => return Ok(()),
        Err(action) => action,
    };
    edits::apply_edit_action(controller, action)
}

fn apply_focus_and_selection_action(
    controller: &mut AppController,
    action: NativeUiAction,
) -> Result<(), NativeUiAction> {
    match action {
        NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::FocusBrowserPanel) => {
            controller.focus_browser_list()
        }
        NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::FocusWaveformPanel) => {
            controller.focus_waveform()
        }
        NativeUiAction::Shell(
            crate::app_core::actions::NativeShellAction::FocusLoadedSampleInBrowser,
        ) => controller.focus_loaded_sample_in_browser(),
        NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::FocusBrowserSearch) => {
            controller.focus_browser_search()
        }
        NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::BlurBrowserSearch) => {
            controller.blur_browser_search()
        }
        NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::MoveBrowserFocus { delta },
        ) => controller.focus_browser_delta_action(delta),
        NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::SetBrowserViewStart { visible_row },
        ) => controller.set_browser_view_start_action(visible_row),
        NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::FocusBrowserRow { visible_row },
        ) => controller.focus_browser_row_from_pointer_action(visible_row),
        NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::SetCompareAnchorFromFocusedBrowserSample,
        ) => controller.set_compare_anchor_from_focused_browser_sample(),
        NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::CommitFocusedBrowserRow,
        ) => handle_commit_focused_browser_row(controller),
        NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::ExtendBrowserSelectionToRow {
                visible_row,
            },
        ) => controller.extend_browser_selection_to_row(visible_row),
        NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::AddRangeBrowserSelection { visible_row },
        ) => controller.add_range_browser_selection(visible_row),
        NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::ExtendBrowserSelectionFromFocus {
                delta,
            },
        ) => controller.extend_browser_selection_from_focus_action(delta),
        NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::AddRangeBrowserSelectionFromFocus {
                delta,
            },
        ) => controller.add_range_browser_selection_from_focus_action(delta),
        NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::ToggleBrowserRowSelection {
                visible_row,
            },
        ) => controller.toggle_browser_row_selection(visible_row),
        NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::ToggleFocusedBrowserRowSelection,
        ) => controller.toggle_focused_selection(),
        NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::SelectAllBrowserRows,
        ) => controller.select_all_browser_rows(),
        NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::SetBrowserSearch { query },
        ) => controller.set_browser_search(query),
        action => return Err(action),
    }
    Ok(())
}

fn apply_history_and_cleanup_action(
    controller: &mut AppController,
    action: NativeUiAction,
) -> Result<(), NativeUiAction> {
    match action {
        NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::ToggleRandomNavigationMode,
        ) => controller.toggle_random_navigation_mode(),
        NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::ToggleBrowserDuplicateCleanupMode,
        ) => controller.toggle_browser_duplicate_cleanup_mode(),
        NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::FocusPreviousBrowserHistory,
        ) => controller.focus_previous_sample_history(),
        NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::FocusNextBrowserHistory,
        ) => controller.focus_next_sample_history(),
        NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::ToggleFindSimilarFocusedSample,
        ) => controller.toggle_find_similar_focused_sample(),
        NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::ToggleBrowserDuplicateCleanupKeep {
                visible_row,
            },
        ) => {
            if let Err(err) =
                controller.toggle_browser_duplicate_cleanup_keep_for_visible_row(visible_row)
            {
                controller.set_status(
                    format!("Duplicate cleanup keep toggle failed: {err}"),
                    StatusTone::Warning,
                );
            }
        }
        NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::ConfirmBrowserDuplicateCleanup,
        ) => {
            if let Err(err) = controller.confirm_browser_duplicate_cleanup() {
                controller.set_status(
                    format!("Duplicate cleanup failed: {err}"),
                    StatusTone::Error,
                );
            }
        }
        NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::PlayRandomSample,
        ) => controller.play_random_visible_sample(),
        NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::PlayPreviousRandomSample,
        ) => controller.play_previous_random_sample(),
        NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::AdjustSelectedBrowserRating { delta },
        ) => controller.adjust_selected_rating(delta),
        action => return Err(action),
    }
    Ok(())
}

fn handle_commit_focused_browser_row(controller: &mut AppController) {
    if controller.ui.focus.context == FocusContext::SampleBrowser {
        controller.commit_focused_browser_row_action();
        return;
    }
    controller.toggle_play_pause();
}
