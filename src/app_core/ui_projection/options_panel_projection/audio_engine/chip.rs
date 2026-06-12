use crate::app_core::state::UiState;

use super::formatting::format_sample_rate_label;

/// Lightweight audio-chip summary used by the top bar and retained cache keys.
pub(crate) struct ProjectedAudioEngineChipModel {
    /// Health state rendered in the compact top-bar chip.
    pub(crate) chip_state: crate::app_core::actions::NativeAudioEngineChipStateModel,
    /// Label rendered in the compact top-bar chip.
    pub(crate) chip_label: String,
}

/// Project the compact audio-engine chip state without materializing picker options.
pub(crate) fn project_audio_engine_chip_model(ui: &UiState) -> ProjectedAudioEngineChipModel {
    let chip_error = ui.audio.output_runtime_error.is_some() || ui.audio.applied.is_none();
    ProjectedAudioEngineChipModel {
        chip_state: if chip_error {
            crate::app_core::actions::NativeAudioEngineChipStateModel::Error
        } else {
            crate::app_core::actions::NativeAudioEngineChipStateModel::Healthy
        },
        chip_label: if chip_error {
            String::from("Audio Err")
        } else {
            format_sample_rate_label(
                ui.audio
                    .applied
                    .as_ref()
                    .map(|output| output.sample_rate)
                    .unwrap_or(0),
            )
        },
    }
}
