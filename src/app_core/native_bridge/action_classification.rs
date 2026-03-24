use crate::app_core::actions::NativeUiAction;

/// Interaction classes tracked by native bridge profiling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum InteractionActionClass {
    /// Wheel-like browser row movement actions.
    Wheel,
    /// Map interaction actions flowing through the bridge.
    MapPanProxy,
    /// Waveform seek/cursor/selection/edit/fade/zoom actions.
    Waveform,
    /// Volume slider interaction actions.
    Volume,
}

/// Classify UI actions into focused interaction profile groups.
pub(super) fn classify_action_interaction(
    action: &NativeUiAction,
) -> Option<InteractionActionClass> {
    match action {
        NativeUiAction::MoveBrowserFocus { .. } | NativeUiAction::SetBrowserViewStart { .. } => {
            Some(InteractionActionClass::Wheel)
        }
        NativeUiAction::SetBrowserTab { map: true } | NativeUiAction::FocusMapSample { .. } => {
            Some(InteractionActionClass::MapPanProxy)
        }
        NativeUiAction::SeekWaveformPrecise { .. }
        | NativeUiAction::SetWaveformCursorPrecise { .. }
        | NativeUiAction::PlayFromWaveformCursor
        | NativeUiAction::PlayWaveformAtPrecise { .. }
        | NativeUiAction::SeekWaveform { .. }
        | NativeUiAction::SetWaveformCursor { .. }
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
        | NativeUiAction::StartWaveformSelectionDrag { .. }
        | NativeUiAction::UpdateWaveformSelectionDrag { .. }
        | NativeUiAction::FinishWaveformSelectionDrag
        | NativeUiAction::FinishWaveformSelectionRangeDrag
        | NativeUiAction::FinishWaveformSelectionSmartScaleDrag
        | NativeUiAction::FinishWaveformEditSelectionDrag
        | NativeUiAction::ClearWaveformEditSelection
        | NativeUiAction::ClearWaveformSelections
        | NativeUiAction::SetWaveformBpmValue { .. }
        | NativeUiAction::AdjustWaveformBpm { .. }
        | NativeUiAction::ClearWaveformSelection
        | NativeUiAction::CropWaveformSelection
        | NativeUiAction::CropWaveformSelectionToNewSample
        | NativeUiAction::TrimWaveformSelection
        | NativeUiAction::ReverseWaveformSelection
        | NativeUiAction::FadeWaveformSelectionLeftToRight
        | NativeUiAction::FadeWaveformSelectionRightToLeft
        | NativeUiAction::MuteWaveformSelection
        | NativeUiAction::DeleteSelectedSliceMarkers
        | NativeUiAction::DetectWaveformSilenceSlices
        | NativeUiAction::ToggleWaveformSliceSelection { .. }
        | NativeUiAction::AlignWaveformStartToMarker
        | NativeUiAction::DeleteLoadedWaveformSample
        | NativeUiAction::SlideWaveformSelection { .. }
        | NativeUiAction::ToggleTransientMarkers
        | NativeUiAction::ToggleBpmSnap
        | NativeUiAction::ZoomWaveform { .. }
        | NativeUiAction::ZoomWaveformToSelection
        | NativeUiAction::ZoomWaveformFull => Some(InteractionActionClass::Waveform),
        NativeUiAction::SetVolume { .. } | NativeUiAction::CommitVolumeSetting => {
            Some(InteractionActionClass::Volume)
        }
        _ => None,
    }
}

/// Return whether a waveform action should apply immediately for smooth preview.
///
/// These actions update overlay state frequently (cursor and selection edits) and
/// benefit from immediate feedback more than queue coalescing.
pub(super) fn is_immediate_waveform_preview_action(action: &NativeUiAction) -> bool {
    matches!(
        action,
        NativeUiAction::SetWaveformCursorPrecise { .. }
            | NativeUiAction::SetWaveformCursor { .. }
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
            | NativeUiAction::StartWaveformSelectionDrag { .. }
            | NativeUiAction::UpdateWaveformSelectionDrag { .. }
            | NativeUiAction::FinishWaveformSelectionDrag
            | NativeUiAction::FinishWaveformSelectionRangeDrag
            | NativeUiAction::FinishWaveformSelectionSmartScaleDrag
            | NativeUiAction::FinishWaveformEditSelectionDrag
            | NativeUiAction::ToggleWaveformSliceSelection { .. }
            | NativeUiAction::ClearWaveformSelection
            | NativeUiAction::ClearWaveformEditSelection
            | NativeUiAction::ClearWaveformSelections
    )
}

/// Return whether an action only mutates native-visible UI state and can use a
/// one-shot local model-pull fast path.
///
/// These actions already update controller UI state directly and do not rely on
/// derived recomputation, background maintenance, or transport ticking to make
/// the next projected frame correct.
pub(super) fn uses_local_model_pull_fast_path(action: &NativeUiAction) -> bool {
    matches!(
        action,
        NativeUiAction::MoveBrowserFocus { .. }
            | NativeUiAction::SetBrowserViewStart { .. }
            | NativeUiAction::FocusBrowserPanel
            | NativeUiAction::FocusSourcesPanel
            | NativeUiAction::FocusWaveformPanel
            | NativeUiAction::FocusFolderPanel
            | NativeUiAction::FocusBrowserSearch
            | NativeUiAction::BlurBrowserSearch
            | NativeUiAction::FocusFolderSearch
            | NativeUiAction::OpenOptionsMenu
            | NativeUiAction::CloseOptionsPanel
            | NativeUiAction::SetPromptInput { .. }
            | NativeUiAction::ToggleWaveformSliceSelection { .. }
    )
}
