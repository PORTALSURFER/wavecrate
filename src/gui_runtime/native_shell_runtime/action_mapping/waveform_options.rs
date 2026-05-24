use super::super::*;

pub(super) fn generic_to_product(
    value: runtime_contract::UiAction,
) -> Result<UiAction, runtime_contract::UiAction> {
    match value {
        runtime_contract::UiAction::SetInputMonitoringEnabled { enabled } => {
            Ok(UiAction::SetInputMonitoringEnabled { enabled })
        }
        runtime_contract::UiAction::SetAdvanceAfterRatingEnabled { enabled } => {
            Ok(UiAction::SetAdvanceAfterRatingEnabled { enabled })
        }
        runtime_contract::UiAction::SetDestructiveYoloMode { enabled } => {
            Ok(UiAction::SetDestructiveYoloMode { enabled })
        }
        runtime_contract::UiAction::SetInvertWaveformScroll { enabled } => {
            Ok(UiAction::SetInvertWaveformScroll { enabled })
        }
        runtime_contract::UiAction::ToggleLoopPlayback => Ok(UiAction::ToggleLoopPlayback),
        runtime_contract::UiAction::ToggleLoopLock => Ok(UiAction::ToggleLoopLock),
        runtime_contract::UiAction::SetWaveformChannelView { stereo } => {
            Ok(UiAction::SetWaveformChannelView { stereo })
        }
        runtime_contract::UiAction::SetNormalizedAuditionEnabled { enabled } => {
            Ok(UiAction::SetNormalizedAuditionEnabled { enabled })
        }
        runtime_contract::UiAction::SetBpmSnapEnabled { enabled } => {
            Ok(UiAction::SetBpmSnapEnabled { enabled })
        }
        runtime_contract::UiAction::SetRelativeBpmGridEnabled { enabled } => {
            Ok(UiAction::SetRelativeBpmGridEnabled { enabled })
        }
        runtime_contract::UiAction::AdjustWaveformBpm { delta } => {
            Ok(UiAction::AdjustWaveformBpm { delta })
        }
        runtime_contract::UiAction::SetWaveformBpmValue { value_tenths } => {
            Ok(UiAction::SetWaveformBpmValue { value_tenths })
        }
        runtime_contract::UiAction::SetTransientSnapEnabled { enabled } => {
            Ok(UiAction::SetTransientSnapEnabled { enabled })
        }
        runtime_contract::UiAction::SetTransientMarkersEnabled { enabled } => {
            Ok(UiAction::SetTransientMarkersEnabled { enabled })
        }
        runtime_contract::UiAction::ToggleTransientMarkers => Ok(UiAction::ToggleTransientMarkers),
        runtime_contract::UiAction::ToggleBpmSnap => Ok(UiAction::ToggleBpmSnap),
        runtime_contract::UiAction::SetSliceModeEnabled { enabled } => {
            Ok(UiAction::SetSliceModeEnabled { enabled })
        }
        runtime_contract::UiAction::SetVolume { value_milli } => {
            Ok(UiAction::SetVolume { value_milli })
        }
        runtime_contract::UiAction::CommitVolumeSetting => Ok(UiAction::CommitVolumeSetting),
        other => Err(other),
    }
}

pub(super) fn product_to_generic(value: UiAction) -> Result<runtime_contract::UiAction, UiAction> {
    match value {
        UiAction::SetInputMonitoringEnabled { enabled } => {
            Ok(runtime_contract::UiAction::SetInputMonitoringEnabled { enabled })
        }
        UiAction::SetAdvanceAfterRatingEnabled { enabled } => {
            Ok(runtime_contract::UiAction::SetAdvanceAfterRatingEnabled { enabled })
        }
        UiAction::SetDestructiveYoloMode { enabled } => {
            Ok(runtime_contract::UiAction::SetDestructiveYoloMode { enabled })
        }
        UiAction::SetInvertWaveformScroll { enabled } => {
            Ok(runtime_contract::UiAction::SetInvertWaveformScroll { enabled })
        }
        UiAction::ToggleLoopPlayback => Ok(runtime_contract::UiAction::ToggleLoopPlayback),
        UiAction::ToggleLoopLock => Ok(runtime_contract::UiAction::ToggleLoopLock),
        UiAction::SetWaveformChannelView { stereo } => {
            Ok(runtime_contract::UiAction::SetWaveformChannelView { stereo })
        }
        UiAction::SetNormalizedAuditionEnabled { enabled } => {
            Ok(runtime_contract::UiAction::SetNormalizedAuditionEnabled { enabled })
        }
        UiAction::SetBpmSnapEnabled { enabled } => {
            Ok(runtime_contract::UiAction::SetBpmSnapEnabled { enabled })
        }
        UiAction::SetRelativeBpmGridEnabled { enabled } => {
            Ok(runtime_contract::UiAction::SetRelativeBpmGridEnabled { enabled })
        }
        UiAction::AdjustWaveformBpm { delta } => {
            Ok(runtime_contract::UiAction::AdjustWaveformBpm { delta })
        }
        UiAction::SetWaveformBpmValue { value_tenths } => {
            Ok(runtime_contract::UiAction::SetWaveformBpmValue { value_tenths })
        }
        UiAction::SetTransientSnapEnabled { enabled } => {
            Ok(runtime_contract::UiAction::SetTransientSnapEnabled { enabled })
        }
        UiAction::SetTransientMarkersEnabled { enabled } => {
            Ok(runtime_contract::UiAction::SetTransientMarkersEnabled { enabled })
        }
        UiAction::ToggleTransientMarkers => Ok(runtime_contract::UiAction::ToggleTransientMarkers),
        UiAction::ToggleBpmSnap => Ok(runtime_contract::UiAction::ToggleBpmSnap),
        UiAction::SetSliceModeEnabled { enabled } => {
            Ok(runtime_contract::UiAction::SetSliceModeEnabled { enabled })
        }
        UiAction::SetVolume { value_milli } => {
            Ok(runtime_contract::UiAction::SetVolume { value_milli })
        }
        UiAction::CommitVolumeSetting => Ok(runtime_contract::UiAction::CommitVolumeSetting),
        other => Err(other),
    }
}
