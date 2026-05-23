use super::*;

impl From<runtime_contract::PairedPickerTargetModel> for AudioPickerTargetModel {
    fn from(value: runtime_contract::PairedPickerTargetModel) -> Self {
        match value {
            runtime_contract::PairedPickerTargetModel::PrimaryGroup => Self::OutputHost,
            runtime_contract::PairedPickerTargetModel::PrimaryItem => Self::OutputDevice,
            runtime_contract::PairedPickerTargetModel::PrimaryNumber => Self::OutputSampleRate,
            runtime_contract::PairedPickerTargetModel::SecondaryGroup => Self::InputHost,
            runtime_contract::PairedPickerTargetModel::SecondaryItem => Self::InputDevice,
            runtime_contract::PairedPickerTargetModel::SecondaryNumber => Self::InputSampleRate,
        }
    }
}

impl From<AudioPickerTargetModel> for runtime_contract::PairedPickerTargetModel {
    fn from(value: AudioPickerTargetModel) -> Self {
        match value {
            AudioPickerTargetModel::OutputHost => Self::PrimaryGroup,
            AudioPickerTargetModel::OutputDevice => Self::PrimaryItem,
            AudioPickerTargetModel::OutputSampleRate => Self::PrimaryNumber,
            AudioPickerTargetModel::InputHost => Self::SecondaryGroup,
            AudioPickerTargetModel::InputDevice => Self::SecondaryItem,
            AudioPickerTargetModel::InputSampleRate => Self::SecondaryNumber,
        }
    }
}

impl From<runtime_contract::PairedPickerValueModel> for AudioOptionValueModel {
    fn from(value: runtime_contract::PairedPickerValueModel) -> Self {
        match value {
            runtime_contract::PairedPickerValueModel::PrimaryGroup(value) => {
                Self::OutputHost(value)
            }
            runtime_contract::PairedPickerValueModel::PrimaryItem(value) => {
                Self::OutputDevice(value)
            }
            runtime_contract::PairedPickerValueModel::PrimaryNumber(value) => {
                Self::OutputSampleRate(value)
            }
            runtime_contract::PairedPickerValueModel::SecondaryGroup(value) => {
                Self::InputHost(value)
            }
            runtime_contract::PairedPickerValueModel::SecondaryItem(value) => {
                Self::InputDevice(value)
            }
            runtime_contract::PairedPickerValueModel::SecondaryNumber(value) => {
                Self::InputSampleRate(value)
            }
        }
    }
}

impl From<AudioOptionValueModel> for runtime_contract::PairedPickerValueModel {
    fn from(value: AudioOptionValueModel) -> Self {
        match value {
            AudioOptionValueModel::OutputHost(value) => Self::PrimaryGroup(value),
            AudioOptionValueModel::OutputDevice(value) => Self::PrimaryItem(value),
            AudioOptionValueModel::OutputSampleRate(value) => Self::PrimaryNumber(value),
            AudioOptionValueModel::InputHost(value) => Self::SecondaryGroup(value),
            AudioOptionValueModel::InputDevice(value) => Self::SecondaryItem(value),
            AudioOptionValueModel::InputSampleRate(value) => Self::SecondaryNumber(value),
        }
    }
}

fn audio_option_item_from_compat(
    value: runtime_contract::PairedPickerOptionModel,
) -> AudioOptionItemModel {
    AudioOptionItemModel {
        label: value.label,
        selected: value.selected,
        value: value.value.into(),
    }
}

fn audio_option_item_to_compat(
    value: AudioOptionItemModel,
) -> runtime_contract::PairedPickerOptionModel {
    runtime_contract::PairedPickerOptionModel {
        label: value.label,
        selected: value.selected,
        value: value.value.into(),
    }
}

impl From<runtime_contract::PairedDevicePanelModel> for AudioEngineModel {
    fn from(value: runtime_contract::PairedDevicePanelModel) -> Self {
        Self {
            chip_state: value.status_state,
            chip_label: value.status_label,
            detail_label: value.detail_label,
            output_host: value.primary_group,
            output_device: value.primary_item,
            output_sample_rate: value.primary_number,
            input_host: value.secondary_group,
            input_device: value.secondary_item,
            input_sample_rate: value.secondary_number,
            active_picker: value.active_picker.map(Into::into),
            output_host_options: value
                .primary_group_options
                .into_iter()
                .map(audio_option_item_from_compat)
                .collect(),
            output_device_options: value
                .primary_item_options
                .into_iter()
                .map(audio_option_item_from_compat)
                .collect(),
            output_sample_rate_options: value
                .primary_number_options
                .into_iter()
                .map(audio_option_item_from_compat)
                .collect(),
            input_host_options: value
                .secondary_group_options
                .into_iter()
                .map(audio_option_item_from_compat)
                .collect(),
            input_device_options: value
                .secondary_item_options
                .into_iter()
                .map(audio_option_item_from_compat)
                .collect(),
            input_sample_rate_options: value
                .secondary_number_options
                .into_iter()
                .map(audio_option_item_from_compat)
                .collect(),
        }
    }
}

impl From<AudioEngineModel> for runtime_contract::PairedDevicePanelModel {
    fn from(value: AudioEngineModel) -> Self {
        Self {
            status_state: value.chip_state,
            status_label: value.chip_label,
            detail_label: value.detail_label,
            primary_group: value.output_host,
            primary_item: value.output_device,
            primary_number: value.output_sample_rate,
            secondary_group: value.input_host,
            secondary_item: value.input_device,
            secondary_number: value.input_sample_rate,
            active_picker: value.active_picker.map(Into::into),
            primary_group_options: value
                .output_host_options
                .into_iter()
                .map(audio_option_item_to_compat)
                .collect(),
            primary_item_options: value
                .output_device_options
                .into_iter()
                .map(audio_option_item_to_compat)
                .collect(),
            primary_number_options: value
                .output_sample_rate_options
                .into_iter()
                .map(audio_option_item_to_compat)
                .collect(),
            secondary_group_options: value
                .input_host_options
                .into_iter()
                .map(audio_option_item_to_compat)
                .collect(),
            secondary_item_options: value
                .input_device_options
                .into_iter()
                .map(audio_option_item_to_compat)
                .collect(),
            secondary_number_options: value
                .input_sample_rate_options
                .into_iter()
                .map(audio_option_item_to_compat)
                .collect(),
        }
    }
}

impl From<&AudioEngineModel> for runtime_contract::PairedDevicePanelModel {
    fn from(value: &AudioEngineModel) -> Self {
        value.clone().into()
    }
}

impl From<&OptionsPanelModel> for runtime_contract::OptionsPanelModel {
    fn from(value: &OptionsPanelModel) -> Self {
        value.clone()
    }
}
