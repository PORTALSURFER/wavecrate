use serde::{Deserialize, Serialize};

/// Options, audio-settings, and persistent interaction toggle actions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptionsAction {
    OpenOptionsMenu,
    CloseOptionsPanel,
    PickTrashFolder,
    OpenTrashFolder,
    EditDefaultIdentifier,
    ShowOptionsOverview,
    OpenAudioOutputHostPicker,
    OpenAudioOutputDevicePicker,
    OpenAudioOutputSampleRatePicker,
    OpenAudioInputHostPicker,
    OpenAudioInputDevicePicker,
    OpenAudioInputSampleRatePicker,
    SetAudioOutputHost { host_id: Option<String> },
    SetAudioOutputDevice { device_name: Option<String> },
    SetAudioOutputSampleRate { sample_rate: Option<u32> },
    SetAudioInputHost { host_id: Option<String> },
    SetAudioInputDevice { device_name: Option<String> },
    SetAudioInputSampleRate { sample_rate: Option<u32> },
    SetInputMonitoringEnabled { enabled: bool },
    SetAdvanceAfterRatingEnabled { enabled: bool },
    SetDestructiveYoloMode { enabled: bool },
    SetInvertWaveformScroll { enabled: bool },
    SetVolume { value_milli: u16 },
    CommitVolumeSetting,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_core::actions::NativeUiAction;

    #[test]
    fn options_action_preserves_flat_serialized_payloads() {
        let action = NativeUiAction::Options(OptionsAction::SetAudioOutputSampleRate {
            sample_rate: Some(48_000),
        });
        let json = serde_json::to_value(&action).expect("serialize action");
        assert_eq!(
            json,
            serde_json::json!({ "SetAudioOutputSampleRate": { "sample_rate": 48000 } })
        );

        let parsed: NativeUiAction = serde_json::from_value(json).expect("parse action");
        assert_eq!(parsed, action);
    }
}
