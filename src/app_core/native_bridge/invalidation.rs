use crate::app_core::actions::NativeUiAction;
#[cfg(test)]
use crate::app_core::actions::{GuiActionKind, representative_action_for_kind};
use crate::app_core::app_api::controller_state::{DerivedNodeId, DirtyReason};

/// Return whether an action requires unconditional projection-cache invalidation.
pub(super) fn action_requires_projection_cache_invalidation(action: &NativeUiAction) -> bool {
    !matches!(
        action,
        NativeUiAction::SeekWaveformPrecise { .. }
            | NativeUiAction::SetWaveformCursorPrecise { .. }
            | NativeUiAction::SeekWaveform { .. }
            | NativeUiAction::SetWaveformCursor { .. }
            | NativeUiAction::BeginWaveformCircularSlide { .. }
            | NativeUiAction::UpdateWaveformCircularSlide { .. }
            | NativeUiAction::FinishWaveformCircularSlide
            | NativeUiAction::SetWaveformViewCenter { .. }
            | NativeUiAction::BeginWaveformSelectionAt { .. }
            | NativeUiAction::SetWaveformSelectionRange { .. }
            | NativeUiAction::SetWaveformSelectionRangeSmartScale { .. }
            | NativeUiAction::SetWaveformEditSelectionRange { .. }
            | NativeUiAction::SetWaveformEditFadeInEnd { .. }
            | NativeUiAction::SetWaveformEditFadeInMuteStart { .. }
            | NativeUiAction::SetWaveformEditFadeInCurve { .. }
            | NativeUiAction::SetWaveformEditFadeOutStart { .. }
            | NativeUiAction::SetWaveformEditFadeOutMuteEnd { .. }
            | NativeUiAction::SetWaveformEditFadeOutCurve { .. }
            | NativeUiAction::FinishWaveformEditFadeDrag
            | NativeUiAction::FinishWaveformSelectionRangeDrag
            | NativeUiAction::FinishWaveformEditSelectionDrag
            | NativeUiAction::ClearWaveformSelection
            | NativeUiAction::ClearWaveformEditSelection
            | NativeUiAction::ClearWaveformSelections
            | NativeUiAction::SetWaveformBpmValue { .. }
            | NativeUiAction::AdjustWaveformBpm { .. }
            | NativeUiAction::ZoomWaveform { .. }
            | NativeUiAction::ZoomWaveformToSelection
            | NativeUiAction::ZoomWaveformFull
            | NativeUiAction::SetVolume { .. }
            | NativeUiAction::CommitVolumeSetting
    )
}

/// Conservative source-node set used for broad invalidation actions.
pub(super) const BROAD_DIRTY_SOURCES: [DerivedNodeId; 4] = [
    DerivedNodeId::BrowserState,
    DerivedNodeId::MapState,
    DerivedNodeId::TransportState,
    DerivedNodeId::StatusState,
];

/// Return whether an action should stay on targeted dirty-source invalidation.
///
/// High-frequency browser navigation/search actions are intentionally excluded
/// from broad invalidation because broad invalidation fans out to unrelated
/// map/transport/status sources and increases projection-key churn.
pub(super) fn action_prefers_targeted_invalidation(action: &NativeUiAction) -> bool {
    matches!(
        action,
        NativeUiAction::MoveBrowserFocus { .. }
            | NativeUiAction::SetBrowserViewStart { .. }
            | NativeUiAction::FocusBrowserRow { .. }
            | NativeUiAction::ToggleBrowserRowSelection { .. }
            | NativeUiAction::StartBrowserSampleDrag { .. }
            | NativeUiAction::UpdateBrowserSampleDrag { .. }
            | NativeUiAction::FinishBrowserSampleDrag
            | NativeUiAction::ExtendBrowserSelectionToRow { .. }
            | NativeUiAction::AddRangeBrowserSelection { .. }
            | NativeUiAction::ExtendBrowserSelectionFromFocus { .. }
            | NativeUiAction::AddRangeBrowserSelectionFromFocus { .. }
            | NativeUiAction::ToggleFocusedBrowserRowSelection
            | NativeUiAction::SelectAllBrowserRows
            | NativeUiAction::SetBrowserSearch { .. }
            | NativeUiAction::ToggleBrowserRatingFilter { .. }
            | NativeUiAction::ToggleBrowserSampleMark
            | NativeUiAction::ToggleBrowserMarkedFilter
            | NativeUiAction::ToggleBrowserDuplicateCleanupMode
            | NativeUiAction::ToggleBrowserDuplicateCleanupKeep { .. }
            | NativeUiAction::FocusBrowserPanel
            | NativeUiAction::FocusBrowserSearch
            | NativeUiAction::BlurBrowserSearch
            | NativeUiAction::FocusLoadedSampleInBrowser
    )
}

/// Resolve the primary dirty source node and reason for one native action.
pub(super) fn classify_dirty_source(
    action: &NativeUiAction,
) -> Option<(DerivedNodeId, DirtyReason)> {
    match action {
        NativeUiAction::SeekWaveformPrecise { .. }
        | NativeUiAction::SetWaveformCursorPrecise { .. }
        | NativeUiAction::SeekWaveform { .. }
        | NativeUiAction::SetWaveformCursor { .. }
        | NativeUiAction::BeginWaveformSelectionAt { .. }
        | NativeUiAction::SetWaveformSelectionRange { .. }
        | NativeUiAction::SetWaveformEditSelectionRange { .. }
        | NativeUiAction::SetWaveformEditFadeInEnd { .. }
        | NativeUiAction::SetWaveformEditFadeInMuteStart { .. }
        | NativeUiAction::SetWaveformEditFadeInCurve { .. }
        | NativeUiAction::SetWaveformEditFadeOutStart { .. }
        | NativeUiAction::SetWaveformEditFadeOutMuteEnd { .. }
        | NativeUiAction::SetWaveformEditFadeOutCurve { .. }
        | NativeUiAction::FinishWaveformEditFadeDrag
        | NativeUiAction::FinishWaveformSelectionRangeDrag
        | NativeUiAction::FinishWaveformEditSelectionDrag
        | NativeUiAction::ClearWaveformSelection
        | NativeUiAction::ClearWaveformEditSelection => Some((
            DerivedNodeId::WaveformState,
            DirtyReason::WaveformOverlayAction,
        )),
        NativeUiAction::ClearWaveformSelections => Some((
            DerivedNodeId::WaveformState,
            DirtyReason::WaveformOverlayAction,
        )),
        NativeUiAction::ZoomWaveform { .. }
        | NativeUiAction::ZoomWaveformToSelection
        | NativeUiAction::ZoomWaveformFull
        | NativeUiAction::SetWaveformViewCenter { .. }
        | NativeUiAction::BeginWaveformCircularSlide { .. }
        | NativeUiAction::UpdateWaveformCircularSlide { .. }
        | NativeUiAction::FinishWaveformCircularSlide
        | NativeUiAction::SetWaveformBpmValue { .. }
        | NativeUiAction::AdjustWaveformBpm { .. }
        | NativeUiAction::SetWaveformSelectionRangeSmartScale { .. }
        | NativeUiAction::FinishWaveformSelectionSmartScaleDrag
        | NativeUiAction::CommitWaveformEditFades
        | NativeUiAction::CropWaveformSelection
        | NativeUiAction::CropWaveformSelectionToNewSample
        | NativeUiAction::TrimWaveformSelection => Some((
            DerivedNodeId::WaveformState,
            DirtyReason::WaveformViewAction,
        )),
        NativeUiAction::MoveBrowserFocus { .. }
        | NativeUiAction::SetBrowserViewStart { .. }
        | NativeUiAction::FocusBrowserRow { .. }
        | NativeUiAction::CommitFocusedBrowserRow
        | NativeUiAction::ToggleBrowserRowSelection { .. }
        | NativeUiAction::StartBrowserSampleDrag { .. }
        | NativeUiAction::UpdateBrowserSampleDrag { .. }
        | NativeUiAction::FinishBrowserSampleDrag
        | NativeUiAction::ExtendBrowserSelectionToRow { .. }
        | NativeUiAction::AddRangeBrowserSelection { .. }
        | NativeUiAction::ExtendBrowserSelectionFromFocus { .. }
        | NativeUiAction::AddRangeBrowserSelectionFromFocus { .. }
        | NativeUiAction::ToggleFocusedBrowserRowSelection
        | NativeUiAction::SelectAllBrowserRows
        | NativeUiAction::SetBrowserSearch { .. }
        | NativeUiAction::ToggleBrowserRatingFilter { .. }
        | NativeUiAction::ToggleBrowserSampleMark
        | NativeUiAction::ToggleBrowserMarkedFilter
        | NativeUiAction::FocusBrowserPanel
        | NativeUiAction::FocusBrowserSearch
        | NativeUiAction::BlurBrowserSearch
        | NativeUiAction::FocusLoadedSampleInBrowser
        | NativeUiAction::ToggleBrowserDuplicateCleanupMode
        | NativeUiAction::ToggleBrowserDuplicateCleanupKeep { .. }
        | NativeUiAction::ConfirmBrowserDuplicateCleanup
        | NativeUiAction::StartBrowserRename
        | NativeUiAction::ConfirmBrowserRename
        | NativeUiAction::CancelBrowserRename
        | NativeUiAction::TagBrowserSelection { .. }
        | NativeUiAction::DeleteBrowserSelection
        | NativeUiAction::SetBrowserTab { map: false } => {
            Some((DerivedNodeId::BrowserState, DirtyReason::BrowserAction))
        }
        NativeUiAction::SetBrowserTab { map: true } | NativeUiAction::FocusMapSample { .. } => {
            Some((DerivedNodeId::MapState, DirtyReason::MapAction))
        }
        NativeUiAction::PlayCompareAnchor
        | NativeUiAction::PlayFromStart
        | NativeUiAction::PlayFromCurrentPlayhead
        | NativeUiAction::PlayFromWaveformCursor
        | NativeUiAction::PlayWaveformAtPrecise { .. }
        | NativeUiAction::ToggleTransport
        | NativeUiAction::ToggleLoopPlayback
        | NativeUiAction::SetVolume { .. }
        | NativeUiAction::CommitVolumeSetting => {
            Some((DerivedNodeId::TransportState, DirtyReason::TransportAction))
        }
        NativeUiAction::SetCompareAnchorFromFocusedBrowserSample => {
            Some((DerivedNodeId::TransportState, DirtyReason::TransportAction))
        }
        NativeUiAction::CheckForUpdates
        | NativeUiAction::OpenUpdateLink
        | NativeUiAction::InstallUpdate
        | NativeUiAction::DismissUpdate
        | NativeUiAction::OpenOptionsMenu
        | NativeUiAction::CloseOptionsPanel
        | NativeUiAction::PickTrashFolder
        | NativeUiAction::OpenTrashFolder
        | NativeUiAction::SetInputMonitoringEnabled { .. }
        | NativeUiAction::SetAdvanceAfterRatingEnabled { .. }
        | NativeUiAction::SetDestructiveYoloMode { .. }
        | NativeUiAction::SetInvertWaveformScroll { .. }
        | NativeUiAction::ConfirmPrompt
        | NativeUiAction::CancelPrompt
        | NativeUiAction::CancelProgress
        | NativeUiAction::SetPromptInput { .. } => {
            Some((DerivedNodeId::StatusState, DirtyReason::StatusAction))
        }
        _ => None,
    }
}

/// Return whether dirty waveform render inputs require a full image refresh.
pub(super) fn waveform_render_inputs_require_refresh(reason: Option<DirtyReason>) -> bool {
    !matches!(reason, Some(DirtyReason::WaveformOverlayAction))
}

/// Resolve whether one catalog action kind prefers targeted invalidation.
#[cfg(test)]
pub(crate) fn catalog_prefers_targeted_invalidation(kind: GuiActionKind) -> bool {
    action_prefers_targeted_invalidation(&representative_action_for_kind(kind))
}

/// Resolve the dirty source metadata for one catalog action kind.
#[cfg(test)]
pub(crate) fn catalog_dirty_source(kind: GuiActionKind) -> Option<(DerivedNodeId, DirtyReason)> {
    classify_dirty_source(&representative_action_for_kind(kind))
}
