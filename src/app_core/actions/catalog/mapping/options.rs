use super::shared::{GuiActionKind, Kind, NativeOptionsAction};

pub(super) fn options_action_kind(action: &NativeOptionsAction) -> GuiActionKind {
    match action {
        NativeOptionsAction::OpenOptionsMenu => Kind::OpenOptionsMenu,
        NativeOptionsAction::CloseOptionsPanel => Kind::CloseOptionsPanel,
        NativeOptionsAction::PickTrashFolder => Kind::PickTrashFolder,
        NativeOptionsAction::OpenTrashFolder => Kind::OpenTrashFolder,
        NativeOptionsAction::EditDefaultIdentifier => Kind::EditDefaultIdentifier,
        NativeOptionsAction::ShowOptionsOverview => Kind::ShowOptionsOverview,
        NativeOptionsAction::OpenAudioOutputHostPicker => Kind::OpenAudioOutputHostPicker,
        NativeOptionsAction::OpenAudioOutputDevicePicker => Kind::OpenAudioOutputDevicePicker,
        NativeOptionsAction::OpenAudioOutputSampleRatePicker => {
            Kind::OpenAudioOutputSampleRatePicker
        }
        NativeOptionsAction::OpenAudioInputHostPicker => Kind::OpenAudioInputHostPicker,
        NativeOptionsAction::OpenAudioInputDevicePicker => Kind::OpenAudioInputDevicePicker,
        NativeOptionsAction::OpenAudioInputSampleRatePicker => Kind::OpenAudioInputSampleRatePicker,
        NativeOptionsAction::SetAudioOutputHost { .. } => Kind::SetAudioOutputHost,
        NativeOptionsAction::SetAudioOutputDevice { .. } => Kind::SetAudioOutputDevice,
        NativeOptionsAction::SetAudioOutputSampleRate { .. } => Kind::SetAudioOutputSampleRate,
        NativeOptionsAction::SetAudioInputHost { .. } => Kind::SetAudioInputHost,
        NativeOptionsAction::SetAudioInputDevice { .. } => Kind::SetAudioInputDevice,
        NativeOptionsAction::SetAudioInputSampleRate { .. } => Kind::SetAudioInputSampleRate,
        NativeOptionsAction::SetInputMonitoringEnabled { .. } => Kind::SetInputMonitoringEnabled,
        NativeOptionsAction::SetAdvanceAfterRatingEnabled { .. } => {
            Kind::SetAdvanceAfterRatingEnabled
        }
        NativeOptionsAction::SetDestructiveYoloMode { .. } => Kind::SetDestructiveYoloMode,
        NativeOptionsAction::SetInvertWaveformScroll { .. } => Kind::SetInvertWaveformScroll,
        NativeOptionsAction::ToggleLoopPlayback => Kind::ToggleLoopPlayback,
        NativeOptionsAction::ToggleLoopLock => Kind::ToggleLoopLock,
        NativeOptionsAction::SetWaveformChannelView { .. } => Kind::SetWaveformChannelView,
        NativeOptionsAction::SetNormalizedAuditionEnabled { .. } => {
            Kind::SetNormalizedAuditionEnabled
        }
        NativeOptionsAction::SetBpmSnapEnabled { .. } => Kind::SetBpmSnapEnabled,
        NativeOptionsAction::SetRelativeBpmGridEnabled { .. } => Kind::SetRelativeBpmGridEnabled,
        NativeOptionsAction::AdjustWaveformBpm { .. } => Kind::AdjustWaveformBpm,
        NativeOptionsAction::SetWaveformBpmValue { .. } => Kind::SetWaveformBpmValue,
        NativeOptionsAction::SetTransientSnapEnabled { .. } => Kind::SetTransientSnapEnabled,
        NativeOptionsAction::SetTransientMarkersEnabled { .. } => Kind::SetTransientMarkersEnabled,
        NativeOptionsAction::ToggleTransientMarkers => Kind::ToggleTransientMarkers,
        NativeOptionsAction::ToggleBpmSnap => Kind::ToggleBpmSnap,
        NativeOptionsAction::SetSliceModeEnabled { .. } => Kind::SetSliceModeEnabled,
        NativeOptionsAction::SetVolume { .. } => Kind::SetVolume,
        NativeOptionsAction::CommitVolumeSetting => Kind::CommitVolumeSetting,
    }
}
