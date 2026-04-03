//! Browser-list routing for native browser actions.

use super::super::AppController;
use crate::app_core::actions::NativeUiAction;
use crate::app_core::app_api::state::{
    DragSource, DragTarget, FocusContext, FolderBrowserUiState, FolderPaneId, UiPoint,
};
use crate::app_core::state::{PlaybackAgeFilterChip, StatusTone};
use radiant::app::FolderPaneIdModel;

/// Try to dispatch browser-list native actions.
pub(super) fn apply_browser_list_native_ui_action(
    controller: &mut AppController,
    action: NativeUiAction,
) -> Result<(), NativeUiAction> {
    match action {
        NativeUiAction::FocusBrowserPanel => controller.focus_browser_list(),
        NativeUiAction::FocusWaveformPanel => controller.focus_waveform(),
        NativeUiAction::FocusLoadedSampleInBrowser => controller.focus_loaded_sample_in_browser(),
        NativeUiAction::FocusBrowserSearch => controller.focus_browser_search(),
        NativeUiAction::BlurBrowserSearch => controller.blur_browser_search(),
        NativeUiAction::MoveBrowserFocus { delta } => controller.focus_browser_delta_action(delta),
        NativeUiAction::SetBrowserViewStart { visible_row } => {
            controller.set_browser_view_start_action(visible_row)
        }
        NativeUiAction::FocusBrowserRow { visible_row } => {
            controller.focus_browser_row_and_play_action(visible_row)
        }
        NativeUiAction::SetCompareAnchorFromFocusedBrowserSample => {
            controller.set_compare_anchor_from_focused_browser_sample()
        }
        NativeUiAction::CommitFocusedBrowserRow => handle_commit_focused_browser_row(controller),
        NativeUiAction::ToggleBrowserRowSelection { visible_row } => {
            controller.toggle_browser_row_selection(visible_row)
        }
        NativeUiAction::StartBrowserSampleDrag {
            visible_row,
            pointer_x,
            pointer_y,
        } => controller
            .start_browser_sample_drag_action(visible_row, native_drag_point(pointer_x, pointer_y)),
        NativeUiAction::UpdateBrowserSampleDrag {
            pointer_x,
            pointer_y,
            hovered_folder_pane,
            hovered_folder_row,
            over_folder_panel,
            shift_down,
            alt_down,
        } => {
            let target = folder_drag_target(
                controller,
                hovered_folder_pane,
                hovered_folder_row,
                over_folder_panel,
            );
            controller.update_active_drag(
                native_drag_point(pointer_x, pointer_y),
                DragSource::Browser,
                target,
                shift_down,
                alt_down,
            );
        }
        NativeUiAction::FinishBrowserSampleDrag => controller.finish_active_drag(),
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
        NativeUiAction::ToggleBrowserPlaybackAgeFilter { bucket, invert } => {
            controller.focus_browser_list();
            let chip = match bucket {
                radiant::app::PlaybackAgeFilterChip::NeverPlayed => {
                    PlaybackAgeFilterChip::NeverPlayed
                }
                radiant::app::PlaybackAgeFilterChip::OlderThanMonth => {
                    PlaybackAgeFilterChip::OlderThanMonth
                }
                radiant::app::PlaybackAgeFilterChip::OlderThanWeek => {
                    PlaybackAgeFilterChip::OlderThanWeek
                }
            };
            if invert {
                controller.invert_browser_playback_age_filter(chip);
            } else {
                controller.set_browser_playback_age_filter(chip, true);
            }
        }
        NativeUiAction::ToggleBrowserSampleMark => {
            controller.focus_browser_list();
            controller.toggle_browser_sample_mark();
        }
        NativeUiAction::ToggleBrowserMarkedFilter => {
            controller.focus_browser_list();
            controller.toggle_browser_marked_filter();
        }
        NativeUiAction::ToggleRandomNavigationMode => controller.toggle_random_navigation_mode(),
        NativeUiAction::ToggleBrowserDuplicateCleanupMode => {
            controller.toggle_browser_duplicate_cleanup_mode()
        }
        NativeUiAction::FocusPreviousBrowserHistory => controller.focus_previous_sample_history(),
        NativeUiAction::FocusNextBrowserHistory => controller.focus_next_sample_history(),
        NativeUiAction::ToggleFindSimilarFocusedSample => {
            controller.toggle_find_similar_focused_sample()
        }
        NativeUiAction::ToggleBrowserDuplicateCleanupKeep { visible_row } => {
            match controller.toggle_browser_duplicate_cleanup_keep_for_visible_row(visible_row) {
                Ok(_) => {}
                Err(err) => controller.set_status(
                    format!("Duplicate cleanup keep toggle failed: {err}"),
                    StatusTone::Warning,
                ),
            }
        }
        NativeUiAction::ConfirmBrowserDuplicateCleanup => {
            if let Err(err) = controller.confirm_browser_duplicate_cleanup() {
                controller.set_status(
                    format!("Duplicate cleanup failed: {err}"),
                    StatusTone::Error,
                );
            }
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

fn handle_commit_focused_browser_row(controller: &mut AppController) {
    if controller.ui.focus.context == FocusContext::SampleBrowser {
        controller.commit_focused_browser_row_action();
        return;
    }
    controller.toggle_play_pause();
}

fn folder_drag_target(
    controller: &AppController,
    hovered_folder_pane: Option<FolderPaneIdModel>,
    hovered_folder_row: Option<usize>,
    over_folder_panel: Option<FolderPaneIdModel>,
) -> DragTarget {
    if let Some(folder) = hovered_folder_pane
        .zip(hovered_folder_row)
        .and_then(|(pane, row)| folder_row_path(controller, pane, row))
    {
        return DragTarget::FolderPanel {
            pane: hovered_folder_pane
                .map(folder_pane_id_from_native)
                .unwrap_or_else(|| controller.active_folder_pane()),
            folder: Some(folder),
        };
    }
    if let Some(pane) = over_folder_panel.map(folder_pane_id_from_native) {
        DragTarget::FolderPanel { pane, folder: None }
    } else {
        DragTarget::None
    }
}

fn folder_row_path(
    controller: &AppController,
    pane: FolderPaneIdModel,
    folder_row: usize,
) -> Option<std::path::PathBuf> {
    folder_browser_for_pane(controller, pane)
        .rows
        .get(folder_row)
        .map(|row| row.path.clone())
}

/// Convert native action pointer coordinates into controller UI points.
fn native_drag_point(pointer_x: u16, pointer_y: u16) -> UiPoint {
    UiPoint::new(f32::from(pointer_x), f32::from(pointer_y))
}

fn folder_pane_id_from_native(pane: FolderPaneIdModel) -> FolderPaneId {
    match pane {
        FolderPaneIdModel::Upper => FolderPaneId::Upper,
        FolderPaneIdModel::Lower => FolderPaneId::Lower,
    }
}

fn folder_browser_for_pane(
    controller: &AppController,
    pane: FolderPaneIdModel,
) -> &FolderBrowserUiState {
    let pane = folder_pane_id_from_native(pane);
    if controller.active_folder_pane() == pane {
        &controller.ui.sources.folders
    } else {
        &controller.ui.sources.folder_pane(pane).browser
    }
}
