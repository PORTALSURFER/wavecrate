use super::{InvalidationReason, InvalidationSource};
#[cfg(test)]
use crate::app_core::actions::{GuiActionKind, representative_action_for_kind};
use crate::app_core::actions::{NativeCompatibilityAction, NativeOptionsAction, NativeUiAction};

/// Return whether an action requires unconditional projection-cache invalidation.
pub(in crate::app_core::ui_bridge) fn action_requires_projection_cache_invalidation(
    action: &NativeUiAction,
) -> bool {
    !matches!(
        action,
        NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SeekWaveformPrecise { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformCursorPrecise { .. })
            | NativeUiAction::Compatibility(NativeCompatibilityAction::SeekWaveform { .. })
            | NativeUiAction::Compatibility(NativeCompatibilityAction::SetWaveformCursor { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::BeginWaveformCircularSlide { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::UpdateWaveformCircularSlide { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::FinishWaveformCircularSlide)
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformViewCenter { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::BeginWaveformSelectionAt { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::BeginWaveformSelectionAtPrecise { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformSelectionRange { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformSelectionRangePrecise { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformSelectionRangeSmartScale { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformSelectionRangeSmartScalePrecise { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformEditSelectionRange { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformEditSelectionRangePrecise { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformEditFadeInEnd { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformEditFadeInMuteStart { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformEditFadeInCurve { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformEditFadeOutStart { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformEditFadeOutMuteEnd { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformEditFadeOutCurve { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::FinishWaveformEditFadeDrag)
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::FinishWaveformSelectionRangeDrag)
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::FinishWaveformEditSelectionDrag)
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::ClearWaveformSelection)
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::ClearWaveformEditSelection)
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::ClearWaveformSelections)
            | NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::SetWaveformBpmValue { .. })
            | NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::AdjustWaveformBpm { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::ZoomWaveform { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::ZoomWaveformToSelection)
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::ZoomWaveformFull)
            | NativeUiAction::Options(NativeOptionsAction::SetVolume { .. })
            | NativeUiAction::Options(NativeOptionsAction::CommitVolumeSetting)
    )
}

/// Conservative source-node set used for broad invalidation actions.
pub(in crate::app_core::ui_bridge) const BROAD_DIRTY_SOURCES: [InvalidationSource; 4] = [
    InvalidationSource::Browser,
    InvalidationSource::Map,
    InvalidationSource::Transport,
    InvalidationSource::Status,
];

/// Return whether an action should stay on targeted dirty-source invalidation.
///
/// High-frequency browser navigation/search actions are intentionally excluded
/// from broad invalidation because broad invalidation fans out to unrelated
/// map/transport/status sources and increases projection-key churn.
pub(in crate::app_core::ui_bridge) fn action_prefers_targeted_invalidation(
    action: &NativeUiAction,
) -> bool {
    matches!(
        action,
        NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::MoveBrowserFocus { .. }
        ) | NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::SetBrowserViewStart { .. }
        ) | NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::FocusSourcesPanel)
            | NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::FocusFolderPanel)
            | NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::FocusFolderSearch)
            | NativeUiAction::Shell(
                crate::app_core::actions::NativeShellAction::SetFolderSearch { .. }
            )
            | NativeUiAction::Shell(
                crate::app_core::actions::NativeShellAction::ToggleShowAllFolders
            )
            | NativeUiAction::Shell(
                crate::app_core::actions::NativeShellAction::ToggleFolderFlattenedView
            )
            | NativeUiAction::SourcesAndFolders(
                crate::app_core::actions::NativeSourcesFoldersAction::FocusFolderRow { .. }
            )
            | NativeUiAction::SourcesAndFolders(
                crate::app_core::actions::NativeSourcesFoldersAction::ActivateFolderRow { .. }
            )
            | NativeUiAction::SourcesAndFolders(
                crate::app_core::actions::NativeSourcesFoldersAction::ToggleFolderRowExpanded { .. }
            )
            | NativeUiAction::SourcesAndFolders(
                crate::app_core::actions::NativeSourcesFoldersAction::ExpandFocusedFolder
            )
            | NativeUiAction::SourcesAndFolders(
                crate::app_core::actions::NativeSourcesFoldersAction::CollapseFocusedFolder
            )
            | NativeUiAction::SourcesAndFolders(
                crate::app_core::actions::NativeSourcesFoldersAction::ToggleFocusedFolderSelection
            )
            | NativeUiAction::SourcesAndFolders(
                crate::app_core::actions::NativeSourcesFoldersAction::MoveFolderFocus { .. }
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::FocusBrowserRow { .. }
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::AdjustSelectedBrowserRating { .. }
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::ToggleBrowserRowSelection { .. }
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::StartBrowserSampleDrag { .. }
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::UpdateBrowserSampleDrag { .. }
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::FinishBrowserSampleDrag
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::ExtendBrowserSelectionToRow { .. }
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::AddRangeBrowserSelection { .. }
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::ExtendBrowserSelectionFromFocus { .. }
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::AddRangeBrowserSelectionFromFocus { .. }
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::ToggleFocusedBrowserRowSelection
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::SelectAllBrowserRows
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::SetBrowserSearch { .. }
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::ToggleBrowserRatingFilter { .. }
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::ToggleBrowserPlaybackAgeFilter { .. }
            )
            | NativeUiAction::PromptsAndEdits(
                crate::app_core::actions::NativePromptEditAction::AutoRenameBrowserSelection { .. }
            )
            | NativeUiAction::PromptsAndEdits(
                crate::app_core::actions::NativePromptEditAction::TagBrowserSelection { .. }
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::ToggleBrowserSampleMark
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::ToggleBrowserMarkedFilter
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::ToggleBrowserTagNamedFilter { .. }
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::ToggleBrowserTagSidebar
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::ToggleBrowserTagSidebarAutoRename
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::ToggleBrowserDuplicateCleanupMode
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::ToggleBrowserDuplicateCleanupKeep { .. }
            )
            | NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::FocusBrowserPanel)
            | NativeUiAction::Shell(
                crate::app_core::actions::NativeShellAction::FocusBrowserSearch
            )
            | NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::BlurBrowserSearch)
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::FocusBrowserTagSidebarInput
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::SetBrowserTagSidebarInput { .. }
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::CommitBrowserTagSidebarInput
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::SetBrowserSidebarLooped { .. }
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::ToggleBrowserSidebarNormalTag { .. }
            )
            | NativeUiAction::Shell(
                crate::app_core::actions::NativeShellAction::FocusLoadedSampleInBrowser
            )
    )
}

/// Resolve the primary dirty source node and reason for one UI action.
pub(in crate::app_core::ui_bridge) fn classify_dirty_source(
    action: &NativeUiAction,
) -> Option<(InvalidationSource, InvalidationReason)> {
    match action {
        NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SeekWaveformPrecise { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformCursorPrecise { .. })
        | NativeUiAction::Compatibility(NativeCompatibilityAction::SeekWaveform { .. })
        | NativeUiAction::Compatibility(NativeCompatibilityAction::SetWaveformCursor { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::BeginWaveformSelectionAt { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::BeginWaveformSelectionAtPrecise { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformSelectionRange { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformSelectionRangePrecise { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformEditSelectionRange { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformEditSelectionRangePrecise { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformEditFadeInEnd { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformEditFadeInMuteStart { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformEditFadeInCurve { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformEditFadeOutStart { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformEditFadeOutMuteEnd { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformEditFadeOutCurve { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::FinishWaveformEditFadeDrag)
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::FinishWaveformSelectionRangeDrag)
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::FinishWaveformEditSelectionDrag)
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::ClearWaveformSelection)
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::ClearWaveformEditSelection) => Some((
            InvalidationSource::Waveform,
            InvalidationReason::WaveformOverlayAction,
        )),
        NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::ClearWaveformSelections) => Some((
            InvalidationSource::Waveform,
            InvalidationReason::WaveformOverlayAction,
        )),
        NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::ZoomWaveform { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::ZoomWaveformToSelection)
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::ZoomWaveformFull)
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformViewCenter { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::BeginWaveformCircularSlide { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::UpdateWaveformCircularSlide { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::FinishWaveformCircularSlide)
        | NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::SetWaveformBpmValue { .. })
        | NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::AdjustWaveformBpm { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformSelectionRangeSmartScale { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformSelectionRangeSmartScalePrecise { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::FinishWaveformSelectionSmartScaleDrag)
        | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::CommitWaveformEditFades)
        | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::CropWaveformSelection)
        | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::CropWaveformSelectionToNewSample)
        | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::TrimWaveformSelection) => Some((
            InvalidationSource::Waveform,
            InvalidationReason::WaveformViewAction,
        )),
        NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::MoveBrowserFocus { .. })
        | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::SetBrowserViewStart { .. })
        | NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::FocusSourcesPanel)
        | NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::FocusFolderPanel)
        | NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::FocusFolderSearch)
        | NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::SetFolderSearch { .. })
        | NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::ToggleShowAllFolders)
        | NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::ToggleFolderFlattenedView)
        | NativeUiAction::SourcesAndFolders(crate::app_core::actions::NativeSourcesFoldersAction::FocusFolderRow { .. })
        | NativeUiAction::SourcesAndFolders(crate::app_core::actions::NativeSourcesFoldersAction::ActivateFolderRow { .. })
        | NativeUiAction::SourcesAndFolders(crate::app_core::actions::NativeSourcesFoldersAction::ToggleFolderRowExpanded { .. })
        | NativeUiAction::SourcesAndFolders(crate::app_core::actions::NativeSourcesFoldersAction::ExpandFocusedFolder)
        | NativeUiAction::SourcesAndFolders(crate::app_core::actions::NativeSourcesFoldersAction::CollapseFocusedFolder)
        | NativeUiAction::SourcesAndFolders(crate::app_core::actions::NativeSourcesFoldersAction::ToggleFocusedFolderSelection)
        | NativeUiAction::SourcesAndFolders(crate::app_core::actions::NativeSourcesFoldersAction::MoveFolderFocus { .. })
        | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::FocusBrowserRow { .. })
        | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::AdjustSelectedBrowserRating { .. })
        | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::CommitFocusedBrowserRow)
        | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::ToggleBrowserRowSelection { .. })
        | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::StartBrowserSampleDrag { .. })
        | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::UpdateBrowserSampleDrag { .. })
        | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::FinishBrowserSampleDrag)
        | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::ExtendBrowserSelectionToRow { .. })
        | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::AddRangeBrowserSelection { .. })
        | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::ExtendBrowserSelectionFromFocus { .. })
        | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::AddRangeBrowserSelectionFromFocus { .. })
        | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::ToggleFocusedBrowserRowSelection)
        | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::SelectAllBrowserRows)
        | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::SetBrowserSearch { .. })
        | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::ToggleBrowserRatingFilter { .. })
        | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::ToggleBrowserPlaybackAgeFilter { .. })
        | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::ToggleBrowserSampleMark)
        | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::ToggleBrowserMarkedFilter)
        | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::ToggleBrowserTagNamedFilter { .. })
        | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::ToggleBrowserTagSidebar)
        | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::ToggleBrowserTagSidebarAutoRename)
        | NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::FocusBrowserPanel)
        | NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::FocusBrowserSearch)
        | NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::BlurBrowserSearch)
        | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::FocusBrowserTagSidebarInput)
        | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::SetBrowserTagSidebarInput { .. })
        | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::CommitBrowserTagSidebarInput)
        | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::SetBrowserSidebarLooped { .. })
        | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::ToggleBrowserSidebarNormalTag { .. })
        | NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::FocusLoadedSampleInBrowser)
        | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::ToggleBrowserDuplicateCleanupMode)
        | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::ToggleBrowserDuplicateCleanupKeep { .. })
        | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::ConfirmBrowserDuplicateCleanup)
        | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::StartBrowserRename)
        | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::ConfirmBrowserRename)
        | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::CancelBrowserRename)
        | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::AutoRenameBrowserSelection { .. })
        | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::TagBrowserSelection { .. })
        | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::DeleteBrowserSelection)
        | NativeUiAction::SourcesAndFolders(crate::app_core::actions::NativeSourcesFoldersAction::StartNewFolder)
        | NativeUiAction::SourcesAndFolders(crate::app_core::actions::NativeSourcesFoldersAction::StartNewFolderAtFolderRow { .. })
        | NativeUiAction::SourcesAndFolders(crate::app_core::actions::NativeSourcesFoldersAction::StartNewFolderAtRoot)
        | NativeUiAction::SourcesAndFolders(crate::app_core::actions::NativeSourcesFoldersAction::FocusFolderCreateInput)
        | NativeUiAction::SourcesAndFolders(crate::app_core::actions::NativeSourcesFoldersAction::SetFolderCreateInput { .. })
        | NativeUiAction::SourcesAndFolders(crate::app_core::actions::NativeSourcesFoldersAction::ConfirmFolderCreate)
        | NativeUiAction::SourcesAndFolders(crate::app_core::actions::NativeSourcesFoldersAction::CancelFolderCreate)
        | NativeUiAction::SourcesAndFolders(crate::app_core::actions::NativeSourcesFoldersAction::StartFolderRename)
        | NativeUiAction::SourcesAndFolders(crate::app_core::actions::NativeSourcesFoldersAction::DeleteFocusedFolder)
        | NativeUiAction::SourcesAndFolders(crate::app_core::actions::NativeSourcesFoldersAction::RestoreRetainedFolderDeletes)
        | NativeUiAction::SourcesAndFolders(crate::app_core::actions::NativeSourcesFoldersAction::PurgeRetainedFolderDeletes)
        | NativeUiAction::SourcesAndFolders(crate::app_core::actions::NativeSourcesFoldersAction::ClearFolderDeleteRecoveryLog)
        | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::SetBrowserTab { map: false }) => {
            Some((InvalidationSource::Browser, InvalidationReason::BrowserAction))
        }
        NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::SetBrowserTab { map: true }) | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::FocusMapSample { .. }) => {
            Some((InvalidationSource::Map, InvalidationReason::MapAction))
        }
        NativeUiAction::Transport(
            crate::app_core::actions::NativeTransportAction::PlayCompareAnchor,
        )
        | NativeUiAction::Transport(
            crate::app_core::actions::NativeTransportAction::PlayFromStart,
        )
        | NativeUiAction::Transport(
            crate::app_core::actions::NativeTransportAction::PlayFromCurrentPlayhead,
        )
        | NativeUiAction::Transport(
            crate::app_core::actions::NativeTransportAction::PlayFromWaveformCursor,
        )
        | NativeUiAction::Transport(
            crate::app_core::actions::NativeTransportAction::PlayWaveformAtPrecise { .. },
        )
        | NativeUiAction::Transport(
            crate::app_core::actions::NativeTransportAction::ToggleTransport,
        )
        | NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::ToggleLoopPlayback)
        | NativeUiAction::Options(NativeOptionsAction::SetVolume { .. })
        | NativeUiAction::Options(NativeOptionsAction::CommitVolumeSetting) => {
            Some((InvalidationSource::Transport, InvalidationReason::TransportAction))
        }
        NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::SetCompareAnchorFromFocusedBrowserSample) => {
            Some((InvalidationSource::Transport, InvalidationReason::TransportAction))
        }
        NativeUiAction::HistoryAndUpdate(
            crate::app_core::actions::NativeHistoryUpdateAction::CheckForUpdates
            | crate::app_core::actions::NativeHistoryUpdateAction::OpenUpdateLink
            | crate::app_core::actions::NativeHistoryUpdateAction::InstallUpdate
            | crate::app_core::actions::NativeHistoryUpdateAction::DismissUpdate,
        )
        | NativeUiAction::Compatibility(
            crate::app_core::actions::NativeCompatibilityAction::CheckForUpdates
            | crate::app_core::actions::NativeCompatibilityAction::OpenUpdateLink
            | crate::app_core::actions::NativeCompatibilityAction::InstallUpdate
            | crate::app_core::actions::NativeCompatibilityAction::DismissUpdate,
        )
        | NativeUiAction::Options(NativeOptionsAction::OpenOptionsMenu)
        | NativeUiAction::Options(NativeOptionsAction::CloseOptionsPanel)
        | NativeUiAction::Options(NativeOptionsAction::PickTrashFolder)
        | NativeUiAction::Options(NativeOptionsAction::OpenTrashFolder)
        | NativeUiAction::Options(NativeOptionsAction::SetInputMonitoringEnabled { .. })
        | NativeUiAction::Options(NativeOptionsAction::SetAdvanceAfterRatingEnabled { .. })
        | NativeUiAction::Options(NativeOptionsAction::SetDestructiveYoloMode { .. })
        | NativeUiAction::Options(NativeOptionsAction::SetInvertWaveformScroll { .. })
        | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::ConfirmPrompt)
        | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::CancelPrompt)
        | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::CancelProgress)
        | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::SetPromptInput { .. }) => {
            Some((InvalidationSource::Status, InvalidationReason::StatusAction))
        }
        _ => None,
    }
}

/// Return whether dirty waveform render inputs require a full image refresh.
pub(in crate::app_core::ui_bridge) fn waveform_render_inputs_require_refresh(
    reason: Option<InvalidationReason>,
) -> bool {
    !matches!(reason, Some(InvalidationReason::WaveformOverlayAction))
}

/// Resolve whether one catalog action kind prefers targeted invalidation.
#[cfg(test)]
pub(crate) fn catalog_prefers_targeted_invalidation(kind: GuiActionKind) -> bool {
    action_prefers_targeted_invalidation(&representative_action_for_kind(kind))
}

/// Resolve the dirty source metadata for one catalog action kind.
#[cfg(test)]
pub(crate) fn catalog_dirty_source(
    kind: GuiActionKind,
) -> Option<(InvalidationSource, InvalidationReason)> {
    classify_dirty_source(&representative_action_for_kind(kind))
}
