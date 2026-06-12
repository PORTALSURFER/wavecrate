use crate::app_core::state::AudioPickerTarget;

pub(super) fn project_audio_picker_target(
    target: AudioPickerTarget,
) -> crate::app_core::actions::NativeAudioPickerTargetModel {
    match target {
        AudioPickerTarget::OutputHost => {
            crate::app_core::actions::NativeAudioPickerTargetModel::OutputHost
        }
        AudioPickerTarget::OutputDevice => {
            crate::app_core::actions::NativeAudioPickerTargetModel::OutputDevice
        }
        AudioPickerTarget::OutputSampleRate => {
            crate::app_core::actions::NativeAudioPickerTargetModel::OutputSampleRate
        }
        AudioPickerTarget::InputHost => {
            crate::app_core::actions::NativeAudioPickerTargetModel::InputHost
        }
        AudioPickerTarget::InputDevice => {
            crate::app_core::actions::NativeAudioPickerTargetModel::InputDevice
        }
        AudioPickerTarget::InputSampleRate => {
            crate::app_core::actions::NativeAudioPickerTargetModel::InputSampleRate
        }
    }
}
