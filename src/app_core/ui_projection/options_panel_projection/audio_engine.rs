use crate::app_core::state::UiState;

#[path = "audio_engine/chip.rs"]
mod chip;
#[path = "audio_engine/detail.rs"]
mod detail;
#[path = "audio_engine/fields.rs"]
mod fields;
#[path = "audio_engine/formatting.rs"]
mod formatting;
#[path = "audio_engine/options.rs"]
mod options;
#[path = "audio_engine/picker.rs"]
mod picker;

#[cfg(test)]
#[path = "audio_engine_tests.rs"]
mod tests;

pub(crate) use chip::project_audio_engine_chip_model;

use detail::{audio_engine_detail_label, output_selection_mismatch};
use fields::{
    input_device_field, input_host_field, input_sample_rate_field, output_device_field,
    output_host_field, output_sample_rate_field,
};
use options::{
    input_device_options, input_host_options, input_sample_rate_options, output_device_options,
    output_host_options, output_sample_rate_options,
};
use picker::project_audio_picker_target;

/// Project the native audio-engine model from UI state.
pub(crate) fn project_audio_engine_model(
    ui: &UiState,
) -> crate::app_core::actions::NativeAudioEngineModel {
    let chip = project_audio_engine_chip_model(ui);
    let output_mismatch = output_selection_mismatch(ui);
    crate::app_core::actions::NativeAudioEngineModel {
        chip_state: chip.chip_state,
        chip_label: chip.chip_label,
        detail_label: audio_engine_detail_label(ui, output_mismatch),
        output_host: output_host_field(ui),
        output_device: output_device_field(ui),
        output_sample_rate: output_sample_rate_field(ui),
        input_host: input_host_field(ui),
        input_device: input_device_field(ui),
        input_sample_rate: input_sample_rate_field(ui),
        active_picker: ui
            .options_panel
            .active_audio_picker
            .map(project_audio_picker_target),
        output_host_options: output_host_options(ui),
        output_device_options: output_device_options(ui),
        output_sample_rate_options: output_sample_rate_options(ui),
        input_host_options: input_host_options(ui),
        input_device_options: input_device_options(ui),
        input_sample_rate_options: input_sample_rate_options(ui),
    }
}
