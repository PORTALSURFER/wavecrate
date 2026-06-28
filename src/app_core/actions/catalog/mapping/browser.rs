use super::shared::{GuiActionKind, Kind, NativeBrowserAction};

pub(super) fn browser_action_kind(action: &NativeBrowserAction) -> GuiActionKind {
    match action {
        NativeBrowserAction::MoveBrowserFocus { .. } => Kind::MoveBrowserFocus,
        NativeBrowserAction::SetBrowserViewStart { .. } => Kind::SetBrowserViewStart,
        NativeBrowserAction::FocusBrowserRow { .. } => Kind::FocusBrowserRow,
        NativeBrowserAction::SetCompareAnchorFromFocusedBrowserSample => {
            Kind::SetCompareAnchorFromFocusedBrowserSample
        }
        NativeBrowserAction::CommitFocusedBrowserRow => Kind::CommitFocusedBrowserRow,
        NativeBrowserAction::SaveWaveformSelectionToBrowser => Kind::SaveWaveformSelectionToBrowser,
        NativeBrowserAction::SaveWaveformSelectionToBrowserWithKeep2 => {
            Kind::SaveWaveformSelectionToBrowserWithKeep2
        }
        NativeBrowserAction::CommitWaveformEditFades => Kind::CommitWaveformEditFades,
        NativeBrowserAction::DetectWaveformSilenceSlices => Kind::DetectWaveformSilenceSlices,
        NativeBrowserAction::DetectWaveformExactDuplicateSlices => {
            Kind::DetectWaveformExactDuplicateSlices
        }
        NativeBrowserAction::CleanWaveformExactDuplicateSlices => {
            Kind::CleanWaveformExactDuplicateSlices
        }
        NativeBrowserAction::ToggleBrowserRowSelection { .. } => Kind::ToggleBrowserRowSelection,
        NativeBrowserAction::StartBrowserSampleDrag { .. } => Kind::StartBrowserSampleDrag,
        NativeBrowserAction::UpdateBrowserSampleDrag { .. } => Kind::UpdateBrowserSampleDrag,
        NativeBrowserAction::FinishBrowserSampleDrag => Kind::FinishBrowserSampleDrag,
        NativeBrowserAction::ExtendBrowserSelectionToRow { .. } => {
            Kind::ExtendBrowserSelectionToRow
        }
        NativeBrowserAction::AddRangeBrowserSelection { .. } => Kind::AddRangeBrowserSelection,
        NativeBrowserAction::ExtendBrowserSelectionFromFocus { .. } => {
            Kind::ExtendBrowserSelectionFromFocus
        }
        NativeBrowserAction::AddRangeBrowserSelectionFromFocus { .. } => {
            Kind::AddRangeBrowserSelectionFromFocus
        }
        NativeBrowserAction::ToggleFocusedBrowserRowSelection => {
            Kind::ToggleFocusedBrowserRowSelection
        }
        NativeBrowserAction::SelectAllBrowserRows => Kind::SelectAllBrowserRows,
        NativeBrowserAction::SetBrowserSearch { .. } => Kind::SetBrowserSearch,
        NativeBrowserAction::ToggleBrowserRatingFilter { .. } => Kind::ToggleBrowserRatingFilter,
        NativeBrowserAction::ToggleBrowserPlaybackAgeFilter { .. } => {
            Kind::ToggleBrowserPlaybackAgeFilter
        }
        NativeBrowserAction::ToggleBrowserSidebarFilter { .. } => Kind::ToggleBrowserSidebarFilter,
        NativeBrowserAction::ClearBrowserSidebarFilter { .. } => Kind::ClearBrowserSidebarFilter,
        NativeBrowserAction::ToggleBrowserTagNamedFilter { .. } => {
            Kind::ToggleBrowserTagNamedFilter
        }
        NativeBrowserAction::ToggleRandomNavigationMode => Kind::ToggleRandomNavigationMode,
        NativeBrowserAction::ToggleBrowserTagSidebar => Kind::ToggleBrowserTagSidebar,
        NativeBrowserAction::ToggleBrowserTagSidebarAutoRename => {
            Kind::ToggleBrowserTagSidebarAutoRename
        }
        NativeBrowserAction::ToggleBrowserDuplicateCleanupMode => {
            Kind::ToggleBrowserDuplicateCleanupMode
        }
        NativeBrowserAction::FocusPreviousBrowserHistory => Kind::FocusPreviousBrowserHistory,
        NativeBrowserAction::FocusNextBrowserHistory => Kind::FocusNextBrowserHistory,
        NativeBrowserAction::ToggleFindSimilarFocusedSample => Kind::ToggleFindSimilarFocusedSample,
        NativeBrowserAction::ToggleBrowserDuplicateCleanupKeep { .. } => {
            Kind::ToggleBrowserDuplicateCleanupKeep
        }
        NativeBrowserAction::ConfirmBrowserDuplicateCleanup => Kind::ConfirmBrowserDuplicateCleanup,
        NativeBrowserAction::PlayRandomSample => Kind::PlayRandomSample,
        NativeBrowserAction::PlayPreviousRandomSample => Kind::PlayPreviousRandomSample,
        NativeBrowserAction::AdjustSelectedBrowserRating { .. } => {
            Kind::AdjustSelectedBrowserRating
        }
        NativeBrowserAction::SetBrowserTab { .. } => Kind::SetBrowserTab,
        NativeBrowserAction::FocusBrowserTagSidebarInput => Kind::FocusBrowserTagSidebarInput,
        NativeBrowserAction::SetBrowserTagSidebarInput { .. } => Kind::SetBrowserTagSidebarInput,
        NativeBrowserAction::CommitBrowserTagSidebarInput => Kind::CommitBrowserTagSidebarInput,
        NativeBrowserAction::SetBrowserSidebarLooped { .. } => Kind::SetBrowserSidebarLooped,
        NativeBrowserAction::ToggleBrowserSidebarNormalTag { .. } => {
            Kind::ToggleBrowserSidebarNormalTag
        }
        NativeBrowserAction::FocusMapSample { .. } => Kind::FocusMapSample,
    }
}
