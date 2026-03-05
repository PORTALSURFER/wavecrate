use crate::app_core::actions::NativeUiAction;
use crate::app_core::app_api::controller_state::{DerivedNodeId, DirtyReason};

/// Return whether an action requires unconditional projection-cache invalidation.
pub(super) fn action_requires_projection_cache_invalidation(action: &NativeUiAction) -> bool {
    !matches!(
        action,
        NativeUiAction::SeekWaveform { .. }
            | NativeUiAction::SetWaveformCursor { .. }
            | NativeUiAction::SetWaveformSelectionRange { .. }
            | NativeUiAction::ClearWaveformSelection
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
            | NativeUiAction::FocusBrowserRow { .. }
            | NativeUiAction::ToggleBrowserRowSelection { .. }
            | NativeUiAction::ExtendBrowserSelectionToRow { .. }
            | NativeUiAction::AddRangeBrowserSelection { .. }
            | NativeUiAction::ExtendBrowserSelectionFromFocus { .. }
            | NativeUiAction::AddRangeBrowserSelectionFromFocus { .. }
            | NativeUiAction::ToggleFocusedBrowserRowSelection
            | NativeUiAction::SelectAllBrowserRows
            | NativeUiAction::SetBrowserSearch { .. }
            | NativeUiAction::FocusBrowserPanel
            | NativeUiAction::FocusBrowserSearch
            | NativeUiAction::FocusLoadedSampleInBrowser
    )
}

/// Resolve the primary dirty source node and reason for one native action.
pub(super) fn classify_dirty_source(
    action: &NativeUiAction,
) -> Option<(DerivedNodeId, DirtyReason)> {
    match action {
        NativeUiAction::SeekWaveform { .. }
        | NativeUiAction::SetWaveformCursor { .. }
        | NativeUiAction::SetWaveformSelectionRange { .. }
        | NativeUiAction::ClearWaveformSelection => Some((
            DerivedNodeId::WaveformState,
            DirtyReason::WaveformOverlayAction,
        )),
        NativeUiAction::ZoomWaveform { .. }
        | NativeUiAction::ZoomWaveformToSelection
        | NativeUiAction::ZoomWaveformFull
        | NativeUiAction::AdjustWaveformBpm { .. } => Some((
            DerivedNodeId::WaveformState,
            DirtyReason::WaveformViewAction,
        )),
        NativeUiAction::MoveBrowserFocus { .. }
        | NativeUiAction::FocusBrowserRow { .. }
        | NativeUiAction::CommitFocusedBrowserRow
        | NativeUiAction::ToggleBrowserRowSelection { .. }
        | NativeUiAction::ExtendBrowserSelectionToRow { .. }
        | NativeUiAction::AddRangeBrowserSelection { .. }
        | NativeUiAction::ExtendBrowserSelectionFromFocus { .. }
        | NativeUiAction::AddRangeBrowserSelectionFromFocus { .. }
        | NativeUiAction::ToggleFocusedBrowserRowSelection
        | NativeUiAction::SelectAllBrowserRows
        | NativeUiAction::SetBrowserSearch { .. }
        | NativeUiAction::FocusBrowserPanel
        | NativeUiAction::FocusBrowserSearch
        | NativeUiAction::FocusLoadedSampleInBrowser
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
        NativeUiAction::ToggleTransport
        | NativeUiAction::ToggleLoopPlayback
        | NativeUiAction::SetVolume { .. }
        | NativeUiAction::CommitVolumeSetting => {
            Some((DerivedNodeId::TransportState, DirtyReason::TransportAction))
        }
        NativeUiAction::CheckForUpdates
        | NativeUiAction::OpenUpdateLink
        | NativeUiAction::InstallUpdate
        | NativeUiAction::DismissUpdate
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
