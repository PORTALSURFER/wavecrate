use crate::app_core::actions::NativeUiAction;

/// Interaction classes tracked by native bridge profiling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum InteractionActionClass {
    /// Wheel-like browser row movement actions.
    Wheel,
    /// Map interaction actions flowing through the bridge.
    MapPanProxy,
    /// Waveform seek/cursor/selection/zoom actions.
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
        | NativeUiAction::ReplayFromLastStart
        | NativeUiAction::SetWaveformCursor { .. }
        | NativeUiAction::SetWaveformSelectionRange { .. }
        | NativeUiAction::SetWaveformEditSelectionRange { .. }
        | NativeUiAction::ClearWaveformSelection
        | NativeUiAction::ClearWaveformEditSelection
        | NativeUiAction::ZoomWaveform { .. }
        | NativeUiAction::ZoomWaveformToSelection
        | NativeUiAction::ZoomWaveformFull
        | NativeUiAction::SetWaveformChannelView { .. }
        | NativeUiAction::SetNormalizedAuditionEnabled { .. }
        | NativeUiAction::SetBpmSnapEnabled { .. }
        | NativeUiAction::SetTransientSnapEnabled { .. }
        | NativeUiAction::SetTransientMarkersEnabled { .. }
        | NativeUiAction::SetSliceModeEnabled { .. } => Some(InteractionActionClass::Waveform),
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
            | NativeUiAction::ClearWaveformSelection
            | NativeUiAction::ClearWaveformEditSelection
    )
}
