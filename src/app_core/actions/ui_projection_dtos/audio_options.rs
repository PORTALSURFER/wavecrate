//! Audio engine and options-panel projection DTOs.

use radiant::gui::feedback;
use radiant::gui::form;

use super::AppModel;

/// Health state of the compact audio-engine status chip.
pub type AudioEngineChipStateModel = feedback::HealthState;

/// One selectable item shown inside an audio picker.
pub type AudioOptionItemModel = form::OptionItem<AudioOptionValueModel>;

/// Overview row shown for one audio field inside the options panel.
pub type AudioFieldModel = form::SummaryField;

/// Generic preference/settings panel state used by UI overlay projections.
pub type PreferencePanelStateModel<const TOGGLES: usize> = form::PreferencePanelState<TOGGLES>;

impl AppModel {
    /// Audio device and engine panel state for the UI projection options view.
    pub fn paired_device_panel(&self) -> &AudioEngineModel {
        &self.audio_engine
    }
}

/// Audio field currently expanded into a picker inside the options panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioPickerTargetModel {
    /// Output host/backend picker.
    OutputHost,
    /// Output device picker.
    OutputDevice,
    /// Output sample-rate picker.
    OutputSampleRate,
    /// Input host/backend picker.
    InputHost,
    /// Input device picker.
    InputDevice,
    /// Input sample-rate picker.
    InputSampleRate,
}

/// Raw value carried by one audio picker option.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AudioOptionValueModel {
    /// Output host identifier, or `None` for the system default.
    OutputHost(Option<String>),
    /// Output device name, or `None` for the host default.
    OutputDevice(Option<String>),
    /// Output sample rate in Hz, or `None` for the device default.
    OutputSampleRate(Option<u32>),
    /// Input host identifier, or `None` for the system default.
    InputHost(Option<String>),
    /// Input device name, or `None` for the host default.
    InputDevice(Option<String>),
    /// Input sample rate in Hz, or `None` for the device default.
    InputSampleRate(Option<u32>),
}

/// Output/input audio engine state projected into the UI projection.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct AudioEngineModel {
    /// Compact chip health state.
    pub chip_state: AudioEngineChipStateModel,
    /// Compact chip label shown in the top-right chrome.
    pub chip_label: String,
    /// Optional detail or error text shown inside the options overview.
    pub detail_label: Option<String>,
    /// Output host summary row.
    pub output_host: AudioFieldModel,
    /// Output device summary row.
    pub output_device: AudioFieldModel,
    /// Output sample-rate summary row.
    pub output_sample_rate: AudioFieldModel,
    /// Input host summary row.
    pub input_host: AudioFieldModel,
    /// Input device summary row.
    pub input_device: AudioFieldModel,
    /// Input sample-rate summary row.
    pub input_sample_rate: AudioFieldModel,
    /// Currently expanded picker, or `None` for the overview.
    pub active_picker: Option<AudioPickerTargetModel>,
    /// Output host choices.
    pub output_host_options: Vec<AudioOptionItemModel>,
    /// Output device choices.
    pub output_device_options: Vec<AudioOptionItemModel>,
    /// Output sample-rate choices.
    pub output_sample_rate_options: Vec<AudioOptionItemModel>,
    /// Input host choices.
    pub input_host_options: Vec<AudioOptionItemModel>,
    /// Input device choices.
    pub input_device_options: Vec<AudioOptionItemModel>,
    /// Input sample-rate choices.
    pub input_sample_rate_options: Vec<AudioOptionItemModel>,
}

impl AudioEngineModel {
    /// Compact health state for the audio engine status chip.
    pub fn status_state(&self) -> AudioEngineChipStateModel {
        self.chip_state
    }

    /// Compact label for the audio engine status chip.
    pub fn status_label(&self) -> &str {
        &self.chip_label
    }

    /// Optional secondary audio-engine detail or error label.
    pub fn detail_label(&self) -> Option<&str> {
        self.detail_label.as_deref()
    }

    /// Primary picker group, currently mapped to output host.
    pub fn primary_group(&self) -> &AudioFieldModel {
        &self.output_host
    }

    /// Primary picker item, currently mapped to output device.
    pub fn primary_item(&self) -> &AudioFieldModel {
        &self.output_device
    }

    /// Primary picker number field, currently mapped to output sample rate.
    pub fn primary_number(&self) -> &AudioFieldModel {
        &self.output_sample_rate
    }

    /// Secondary picker group, currently mapped to input host.
    pub fn secondary_group(&self) -> &AudioFieldModel {
        &self.input_host
    }

    /// Secondary picker item, currently mapped to input device.
    pub fn secondary_item(&self) -> &AudioFieldModel {
        &self.input_device
    }

    /// Secondary picker number field, currently mapped to input sample rate.
    pub fn secondary_number(&self) -> &AudioFieldModel {
        &self.input_sample_rate
    }

    /// Currently active generic paired-picker target.
    pub fn active_picker(&self) -> Option<form::PairedPickerTarget> {
        self.active_picker.map(Into::into)
    }

    /// Option rows for the requested generic paired-picker target.
    pub fn options_for(&self, target: form::PairedPickerTarget) -> &[AudioOptionItemModel] {
        match target {
            form::PairedPickerTarget::PrimaryGroup => &self.output_host_options,
            form::PairedPickerTarget::PrimaryItem => &self.output_device_options,
            form::PairedPickerTarget::PrimaryNumber => &self.output_sample_rate_options,
            form::PairedPickerTarget::SecondaryGroup => &self.input_host_options,
            form::PairedPickerTarget::SecondaryItem => &self.input_device_options,
            form::PairedPickerTarget::SecondaryNumber => &self.input_sample_rate_options,
        }
    }
}

impl From<AudioPickerTargetModel> for form::PairedPickerTarget {
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

/// Options-panel state projected into the UI projection.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct OptionsPanelModel {
    /// Whether the panel is currently visible.
    pub visible: bool,
    /// Current default identifier used by auto rename.
    pub default_identifier: String,
    /// Whether input monitoring is enabled.
    pub input_monitoring_enabled: bool,
    /// Whether rating advances browser focus.
    pub advance_after_rating_enabled: bool,
    /// Whether destructive edits skip confirmation.
    pub destructive_yolo_mode_enabled: bool,
    /// Whether waveform scrolling is inverted.
    pub invert_waveform_scroll_enabled: bool,
    /// Short display label for the configured trash folder, when available.
    pub trash_folder_label: Option<String>,
    /// Short display label for the configured audio write format.
    pub audio_write_format_label: Option<String>,
}

impl OptionsPanelModel {
    /// Return this panel's generic preference/settings state.
    pub fn preference_state(&self) -> PreferencePanelStateModel<4> {
        PreferencePanelStateModel::new(
            self.visible,
            self.default_identifier.clone(),
            [
                self.input_monitoring_enabled,
                self.advance_after_rating_enabled,
                self.destructive_yolo_mode_enabled,
                self.invert_waveform_scroll_enabled,
            ],
            self.trash_folder_label.clone(),
        )
    }
}
