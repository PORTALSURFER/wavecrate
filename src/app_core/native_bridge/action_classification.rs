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
        NativeUiAction::MoveBrowserFocus { .. } => Some(InteractionActionClass::Wheel),
        NativeUiAction::SetBrowserTab { map: true } | NativeUiAction::FocusMapSample { .. } => {
            Some(InteractionActionClass::MapPanProxy)
        }
        NativeUiAction::SeekWaveform { .. }
        | NativeUiAction::SetWaveformCursor { .. }
        | NativeUiAction::SetWaveformSelectionRange { .. }
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
        | NativeUiAction::ClearWaveformEditSelection
        | NativeUiAction::SetWaveformBpmValue { .. }
        | NativeUiAction::AdjustWaveformBpm { .. }
        | NativeUiAction::ClearWaveformSelection
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
        NativeUiAction::SetWaveformCursor { .. }
            | NativeUiAction::SetWaveformSelectionRange { .. }
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
            | NativeUiAction::ClearWaveformSelection
            | NativeUiAction::ClearWaveformEditSelection
    )
}
