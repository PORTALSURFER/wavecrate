use super::super::*;

pub(super) fn generic_to_product(
    value: runtime_contract::UiAction,
) -> Result<UiAction, runtime_contract::UiAction> {
    match value {
        runtime_contract::UiAction::MoveBrowserFocus { delta } => {
            Ok(UiAction::MoveBrowserFocus { delta })
        }
        runtime_contract::UiAction::SetBrowserViewStart { visible_row } => {
            Ok(UiAction::SetBrowserViewStart { visible_row })
        }
        runtime_contract::UiAction::FocusBrowserRow { visible_row } => {
            Ok(UiAction::FocusBrowserRow { visible_row })
        }
        runtime_contract::UiAction::SetCompareAnchorFromFocusedContent => {
            Ok(UiAction::SetCompareAnchorFromFocusedBrowserSample)
        }
        runtime_contract::UiAction::CommitFocusedBrowserRow => {
            Ok(UiAction::CommitFocusedBrowserRow)
        }
        runtime_contract::UiAction::SaveWaveformSelectionToBrowser => {
            Ok(UiAction::SaveWaveformSelectionToBrowser)
        }
        runtime_contract::UiAction::SaveWaveformSelectionToBrowserWithKeep2 => {
            Ok(UiAction::SaveWaveformSelectionToBrowserWithKeep2)
        }
        runtime_contract::UiAction::CommitWaveformEditFades => {
            Ok(UiAction::CommitWaveformEditFades)
        }
        runtime_contract::UiAction::DetectWaveformSilenceSlices => {
            Ok(UiAction::DetectWaveformSilenceSlices)
        }
        runtime_contract::UiAction::DetectWaveformExactDuplicateSlices => {
            Ok(UiAction::DetectWaveformExactDuplicateSlices)
        }
        runtime_contract::UiAction::CleanWaveformExactDuplicateSlices => {
            Ok(UiAction::CleanWaveformExactDuplicateSlices)
        }
        runtime_contract::UiAction::ToggleBrowserRowSelection { visible_row } => {
            Ok(UiAction::ToggleBrowserRowSelection { visible_row })
        }
        runtime_contract::UiAction::StartContentItemDrag {
            visible_row,
            pointer_x,
            pointer_y,
        } => Ok(UiAction::StartBrowserSampleDrag {
            visible_row,
            pointer_x,
            pointer_y,
        }),
        runtime_contract::UiAction::UpdateContentItemDrag {
            pointer_x,
            pointer_y,
            hovered_folder_pane,
            hovered_folder_row,
            over_folder_panel,
            shift_down,
            alt_down,
        } => Ok(UiAction::UpdateBrowserSampleDrag {
            pointer_x,
            pointer_y,
            hovered_folder_pane,
            hovered_folder_row,
            over_folder_panel,
            shift_down,
            alt_down,
        }),
        runtime_contract::UiAction::FinishContentItemDrag => Ok(UiAction::FinishBrowserSampleDrag),
        runtime_contract::UiAction::ExtendBrowserSelectionToRow { visible_row } => {
            Ok(UiAction::ExtendBrowserSelectionToRow { visible_row })
        }
        runtime_contract::UiAction::AddRangeBrowserSelection { visible_row } => {
            Ok(UiAction::AddRangeBrowserSelection { visible_row })
        }
        runtime_contract::UiAction::ExtendBrowserSelectionFromFocus { delta } => {
            Ok(UiAction::ExtendBrowserSelectionFromFocus { delta })
        }
        runtime_contract::UiAction::AddRangeBrowserSelectionFromFocus { delta } => {
            Ok(UiAction::AddRangeBrowserSelectionFromFocus { delta })
        }
        runtime_contract::UiAction::ToggleFocusedBrowserRowSelection => {
            Ok(UiAction::ToggleFocusedBrowserRowSelection)
        }
        runtime_contract::UiAction::SelectAllBrowserRows => Ok(UiAction::SelectAllBrowserRows),
        runtime_contract::UiAction::SetBrowserSearch { query } => {
            Ok(UiAction::SetBrowserSearch { query })
        }
        runtime_contract::UiAction::ToggleBrowserRatingFilter { level, invert } => {
            Ok(UiAction::ToggleBrowserRatingFilter { level, invert })
        }
        runtime_contract::UiAction::ToggleBrowserPlaybackAgeFilter { bucket, invert } => {
            Ok(UiAction::ToggleBrowserPlaybackAgeFilter { bucket, invert })
        }
        runtime_contract::UiAction::ToggleBrowserSidebarFilter { option, additive } => {
            Ok(UiAction::ToggleBrowserSidebarFilter { option, additive })
        }
        runtime_contract::UiAction::ClearBrowserSidebarFilter { facet } => {
            Ok(UiAction::ClearBrowserSidebarFilter { facet })
        }
        runtime_contract::UiAction::ToggleContentMark => Ok(UiAction::ToggleBrowserSampleMark),
        runtime_contract::UiAction::ToggleBrowserMarkedFilter => {
            Ok(UiAction::ToggleBrowserMarkedFilter)
        }
        runtime_contract::UiAction::ToggleBrowserDerivedLabelFilter { invert } => {
            Ok(UiAction::ToggleBrowserTagNamedFilter { invert })
        }
        runtime_contract::UiAction::ToggleRandomNavigationMode => {
            Ok(UiAction::ToggleRandomNavigationMode)
        }
        runtime_contract::UiAction::ToggleBrowserPillEditor => {
            Ok(UiAction::ToggleBrowserTagSidebar)
        }
        runtime_contract::UiAction::ToggleBrowserPillEditorPrimaryAction => {
            Ok(UiAction::ToggleBrowserTagSidebarAutoRename)
        }
        runtime_contract::UiAction::ToggleBrowserDuplicateCleanupMode => {
            Ok(UiAction::ToggleBrowserDuplicateCleanupMode)
        }
        runtime_contract::UiAction::FocusPreviousBrowserHistory => {
            Ok(UiAction::FocusPreviousBrowserHistory)
        }
        runtime_contract::UiAction::FocusNextBrowserHistory => {
            Ok(UiAction::FocusNextBrowserHistory)
        }
        runtime_contract::UiAction::ToggleFindSimilarFocusedContent => {
            Ok(UiAction::ToggleFindSimilarFocusedSample)
        }
        runtime_contract::UiAction::ToggleBrowserDuplicateCleanupKeep { visible_row } => {
            Ok(UiAction::ToggleBrowserDuplicateCleanupKeep { visible_row })
        }
        runtime_contract::UiAction::ConfirmBrowserDuplicateCleanup => {
            Ok(UiAction::ConfirmBrowserDuplicateCleanup)
        }
        runtime_contract::UiAction::PlayRandomContentItem => Ok(UiAction::PlayRandomSample),
        runtime_contract::UiAction::PlayPreviousRandomContentItem => {
            Ok(UiAction::PlayPreviousRandomSample)
        }
        runtime_contract::UiAction::AdjustSelectedBrowserRating { delta } => {
            Ok(UiAction::AdjustSelectedBrowserRating { delta })
        }
        runtime_contract::UiAction::SetBrowserTab { map } => Ok(UiAction::SetBrowserTab { map }),
        runtime_contract::UiAction::FocusBrowserPillEditorInput => {
            Ok(UiAction::FocusBrowserTagSidebarInput)
        }
        runtime_contract::UiAction::SetBrowserPillEditorInput { value } => {
            Ok(UiAction::SetBrowserTagSidebarInput { value })
        }
        runtime_contract::UiAction::CommitBrowserPillEditorInput => {
            Ok(UiAction::CommitBrowserTagSidebarInput)
        }
        runtime_contract::UiAction::SetBrowserSidebarLooped { looped } => {
            Ok(UiAction::SetBrowserSidebarLooped { looped })
        }
        runtime_contract::UiAction::ToggleBrowserPillOption { label } => {
            Ok(UiAction::ToggleBrowserSidebarNormalTag { label })
        }
        runtime_contract::UiAction::FocusSpatialContentItem { content_id } => {
            Ok(UiAction::FocusMapSample {
                sample_id: content_id,
            })
        }
        runtime_contract::UiAction::SetPromptInput { value } => {
            Ok(UiAction::SetPromptInput { value })
        }
        runtime_contract::UiAction::StartBrowserRename => Ok(UiAction::StartBrowserRename),
        runtime_contract::UiAction::ConfirmBrowserRename => Ok(UiAction::ConfirmBrowserRename),
        runtime_contract::UiAction::CancelBrowserRename => Ok(UiAction::CancelBrowserRename),
        runtime_contract::UiAction::AutoRenameBrowserSelection { visible_row } => {
            Ok(UiAction::AutoRenameBrowserSelection { visible_row })
        }
        runtime_contract::UiAction::SetBrowserTriageMark { target } => {
            Ok(UiAction::TagBrowserSelection {
                target: target.into(),
            })
        }
        runtime_contract::UiAction::DeleteBrowserSelection => Ok(UiAction::DeleteBrowserSelection),
        other => Err(other),
    }
}

pub(super) fn product_to_generic(value: UiAction) -> Result<runtime_contract::UiAction, UiAction> {
    match value {
        UiAction::MoveBrowserFocus { delta } => {
            Ok(runtime_contract::UiAction::MoveBrowserFocus { delta })
        }
        UiAction::SetBrowserViewStart { visible_row } => {
            Ok(runtime_contract::UiAction::SetBrowserViewStart { visible_row })
        }
        UiAction::FocusBrowserRow { visible_row } => {
            Ok(runtime_contract::UiAction::FocusBrowserRow { visible_row })
        }
        UiAction::SetCompareAnchorFromFocusedBrowserSample => {
            Ok(runtime_contract::UiAction::SetCompareAnchorFromFocusedContent)
        }
        UiAction::CommitFocusedBrowserRow => {
            Ok(runtime_contract::UiAction::CommitFocusedBrowserRow)
        }
        UiAction::SaveWaveformSelectionToBrowser => {
            Ok(runtime_contract::UiAction::SaveWaveformSelectionToBrowser)
        }
        UiAction::SaveWaveformSelectionToBrowserWithKeep2 => {
            Ok(runtime_contract::UiAction::SaveWaveformSelectionToBrowserWithKeep2)
        }
        UiAction::CommitWaveformEditFades => {
            Ok(runtime_contract::UiAction::CommitWaveformEditFades)
        }
        UiAction::DetectWaveformSilenceSlices => {
            Ok(runtime_contract::UiAction::DetectWaveformSilenceSlices)
        }
        UiAction::DetectWaveformExactDuplicateSlices => {
            Ok(runtime_contract::UiAction::DetectWaveformExactDuplicateSlices)
        }
        UiAction::CleanWaveformExactDuplicateSlices => {
            Ok(runtime_contract::UiAction::CleanWaveformExactDuplicateSlices)
        }
        UiAction::ToggleBrowserRowSelection { visible_row } => {
            Ok(runtime_contract::UiAction::ToggleBrowserRowSelection { visible_row })
        }
        UiAction::StartBrowserSampleDrag {
            visible_row,
            pointer_x,
            pointer_y,
        } => Ok(runtime_contract::UiAction::StartContentItemDrag {
            visible_row,
            pointer_x,
            pointer_y,
        }),
        UiAction::UpdateBrowserSampleDrag {
            pointer_x,
            pointer_y,
            hovered_folder_pane,
            hovered_folder_row,
            over_folder_panel,
            shift_down,
            alt_down,
        } => Ok(runtime_contract::UiAction::UpdateContentItemDrag {
            pointer_x,
            pointer_y,
            hovered_folder_pane,
            hovered_folder_row,
            over_folder_panel,
            shift_down,
            alt_down,
        }),
        UiAction::FinishBrowserSampleDrag => Ok(runtime_contract::UiAction::FinishContentItemDrag),
        UiAction::ExtendBrowserSelectionToRow { visible_row } => {
            Ok(runtime_contract::UiAction::ExtendBrowserSelectionToRow { visible_row })
        }
        UiAction::AddRangeBrowserSelection { visible_row } => {
            Ok(runtime_contract::UiAction::AddRangeBrowserSelection { visible_row })
        }
        UiAction::ExtendBrowserSelectionFromFocus { delta } => {
            Ok(runtime_contract::UiAction::ExtendBrowserSelectionFromFocus { delta })
        }
        UiAction::AddRangeBrowserSelectionFromFocus { delta } => {
            Ok(runtime_contract::UiAction::AddRangeBrowserSelectionFromFocus { delta })
        }
        UiAction::ToggleFocusedBrowserRowSelection => {
            Ok(runtime_contract::UiAction::ToggleFocusedBrowserRowSelection)
        }
        UiAction::SelectAllBrowserRows => Ok(runtime_contract::UiAction::SelectAllBrowserRows),
        UiAction::SetBrowserSearch { query } => {
            Ok(runtime_contract::UiAction::SetBrowserSearch { query })
        }
        UiAction::ToggleBrowserRatingFilter { level, invert } => {
            Ok(runtime_contract::UiAction::ToggleBrowserRatingFilter { level, invert })
        }
        UiAction::ToggleBrowserPlaybackAgeFilter { bucket, invert } => {
            Ok(runtime_contract::UiAction::ToggleBrowserPlaybackAgeFilter { bucket, invert })
        }
        UiAction::ToggleBrowserSidebarFilter { option, additive } => {
            Ok(runtime_contract::UiAction::ToggleBrowserSidebarFilter { option, additive })
        }
        UiAction::ClearBrowserSidebarFilter { facet } => {
            Ok(runtime_contract::UiAction::ClearBrowserSidebarFilter { facet })
        }
        UiAction::ToggleBrowserSampleMark => Ok(runtime_contract::UiAction::ToggleContentMark),
        UiAction::ToggleBrowserMarkedFilter => {
            Ok(runtime_contract::UiAction::ToggleBrowserMarkedFilter)
        }
        UiAction::ToggleBrowserTagNamedFilter { invert } => {
            Ok(runtime_contract::UiAction::ToggleBrowserDerivedLabelFilter { invert })
        }
        UiAction::ToggleRandomNavigationMode => {
            Ok(runtime_contract::UiAction::ToggleRandomNavigationMode)
        }
        UiAction::ToggleBrowserTagSidebar => {
            Ok(runtime_contract::UiAction::ToggleBrowserPillEditor)
        }
        UiAction::ToggleBrowserTagSidebarAutoRename => {
            Ok(runtime_contract::UiAction::ToggleBrowserPillEditorPrimaryAction)
        }
        UiAction::ToggleBrowserDuplicateCleanupMode => {
            Ok(runtime_contract::UiAction::ToggleBrowserDuplicateCleanupMode)
        }
        UiAction::FocusPreviousBrowserHistory => {
            Ok(runtime_contract::UiAction::FocusPreviousBrowserHistory)
        }
        UiAction::FocusNextBrowserHistory => {
            Ok(runtime_contract::UiAction::FocusNextBrowserHistory)
        }
        UiAction::ToggleFindSimilarFocusedSample => {
            Ok(runtime_contract::UiAction::ToggleFindSimilarFocusedContent)
        }
        UiAction::ToggleBrowserDuplicateCleanupKeep { visible_row } => {
            Ok(runtime_contract::UiAction::ToggleBrowserDuplicateCleanupKeep { visible_row })
        }
        UiAction::ConfirmBrowserDuplicateCleanup => {
            Ok(runtime_contract::UiAction::ConfirmBrowserDuplicateCleanup)
        }
        UiAction::PlayRandomSample => Ok(runtime_contract::UiAction::PlayRandomContentItem),
        UiAction::PlayPreviousRandomSample => {
            Ok(runtime_contract::UiAction::PlayPreviousRandomContentItem)
        }
        UiAction::AdjustSelectedBrowserRating { delta } => {
            Ok(runtime_contract::UiAction::AdjustSelectedBrowserRating { delta })
        }
        UiAction::SetBrowserTab { map } => Ok(runtime_contract::UiAction::SetBrowserTab { map }),
        UiAction::FocusBrowserTagSidebarInput => {
            Ok(runtime_contract::UiAction::FocusBrowserPillEditorInput)
        }
        UiAction::SetBrowserTagSidebarInput { value } => {
            Ok(runtime_contract::UiAction::SetBrowserPillEditorInput { value })
        }
        UiAction::CommitBrowserTagSidebarInput => {
            Ok(runtime_contract::UiAction::CommitBrowserPillEditorInput)
        }
        UiAction::SetBrowserSidebarLooped { looped } => {
            Ok(runtime_contract::UiAction::SetBrowserSidebarLooped { looped })
        }
        UiAction::ToggleBrowserSidebarNormalTag { label } => {
            Ok(runtime_contract::UiAction::ToggleBrowserPillOption { label })
        }
        UiAction::FocusMapSample { sample_id } => {
            Ok(runtime_contract::UiAction::FocusSpatialContentItem {
                content_id: sample_id,
            })
        }
        UiAction::SetPromptInput { value } => {
            Ok(runtime_contract::UiAction::SetPromptInput { value })
        }
        UiAction::StartBrowserRename => Ok(runtime_contract::UiAction::StartBrowserRename),
        UiAction::ConfirmBrowserRename => Ok(runtime_contract::UiAction::ConfirmBrowserRename),
        UiAction::CancelBrowserRename => Ok(runtime_contract::UiAction::CancelBrowserRename),
        UiAction::AutoRenameBrowserSelection { visible_row } => {
            Ok(runtime_contract::UiAction::AutoRenameBrowserSelection { visible_row })
        }
        UiAction::TagBrowserSelection { target } => {
            Ok(runtime_contract::UiAction::SetBrowserTriageMark {
                target: target.into(),
            })
        }
        UiAction::DeleteBrowserSelection => Ok(runtime_contract::UiAction::DeleteBrowserSelection),
        other => Err(other),
    }
}
